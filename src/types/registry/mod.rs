use crate::types::{Buffer, Duration, Timestamp, Transform};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
mod error;
use crate::errors::{BufferError, TransformError};
use log::debug;

#[cfg(feature = "async")]
pub use async_impl::Registry;

#[cfg(not(feature = "async"))]
pub use sync_impl::Registry;

#[cfg(feature = "async")]
mod async_impl {
    use super::*;
    use tokio::sync::{Mutex, Notify};

    pub struct Registry {
        pub data: Mutex<HashMap<String, Buffer>>,
        ttl: Duration,
        notify: Notify,
    }

    impl Registry {
        pub fn new(ttl: Duration) -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                ttl,
                notify: Notify::new(),
            }
        }

        pub async fn add_transform(
            &self,
            t: Transform,
        ) -> Result<(), BufferError> {
            {
                let mut data = self.data.lock().await;
                match data.entry(t.child.clone()) {
                    Entry::Occupied(mut entry) => {
                        debug!("Buffer found, adding transform.");
                        entry.get_mut().insert(t);
                    }
                    Entry::Vacant(entry) => {
                        debug!("No buffer found for this parent-child, creating new buffer.");
                        let buffer = Buffer::new(self.ttl);
                        let buffer = entry.insert(buffer);
                        buffer.insert(t);
                    }
                }
            }
            self.notify.notify_waiters();
            Ok(())
        }

        pub async fn await_transform(
            &self,
            from: &str,
            to: &str,
            timestamp: Timestamp,
        ) -> Result<Transform, TransformError> {
            loop {
                if let Ok(transform) = self.get_transform(from, to, timestamp).await {
                    return Ok(transform);
                }
                self.notify.notified().await;
            }
        }

        pub async fn get_transform(
            &self,
            from: &str,
            to: &str,
            timestamp: Timestamp,
        ) -> Result<Transform, TransformError> {
            let mut d = self.data.lock().await;
            Self::process_transform(from, to, timestamp, &mut d)
        }
    }
}

#[cfg(not(feature = "async"))]
mod sync_impl {
    use super::*;
    pub struct Registry {
        pub data: HashMap<String, Buffer>,
        ttl: Duration,
    }

    impl Registry {
        pub fn new(d: Duration) -> Self {
            Self {
                data: HashMap::new(),
                ttl: d,
            }
        }

        pub fn add_transform(
            &mut self,
            t: Transform,
        ) -> Result<(), BufferError> {
            match self.data.entry(t.child.clone()) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(t);
                }
                Entry::Vacant(entry) => {
                    let buffer = Buffer::new(self.ttl);
                    let buffer = entry.insert(buffer);
                    buffer.insert(t);
                }
            }
            Ok(())
        }

        pub fn get_transform(
            &mut self,
            from: &str,
            to: &str,
            timestamp: Timestamp,
        ) -> Result<Transform, TransformError> {
            Self::process_transform(from, to, timestamp, &mut self.data)
        }
    }
}

impl Registry {
    pub fn process_transform(
        from: &str,
        to: &str,
        timestamp: Timestamp,
        data: &mut HashMap<String, Buffer>,
    ) -> Result<Transform, TransformError> {
        let from_chain = Self::get_transform_chain(from, to, timestamp, data);
        let mut to_chain = Self::get_transform_chain(to, from, timestamp, data);

        if let Ok(chain) = to_chain.as_mut() {
            Self::reverse_and_invert_transforms(chain)?;
        }

        match (from_chain, to_chain) {
            (Ok(mut from_chain), Ok(mut to_chain)) => {
                Self::truncate_at_common_parent(&mut from_chain, &mut to_chain);
                Self::combine_transforms(from_chain, to_chain)
            }
            (Ok(from_chain), Err(_)) => Self::combine_transforms(from_chain, VecDeque::new()),
            (Err(_), Ok(to_chain)) => Self::combine_transforms(VecDeque::new(), to_chain),
            (Err(_), Err(_)) => Err(TransformError::NotFound(from.to_string(), to.to_string())),
        }
    }

    fn get_transform_chain(
        from: &str,
        to: &str,
        timestamp: Timestamp,
        data: &HashMap<String, Buffer>, // Use an immutable reference
    ) -> Result<VecDeque<Transform>, TransformError> {
        let mut transforms = VecDeque::new();
        let mut current_frame = from.to_string();

        while let Some(frame_buffer) = data.get(&current_frame) {
            match frame_buffer.get(&timestamp) {
                Ok(tf) => {
                    transforms.push_back(tf.clone());
                    current_frame = tf.parent.clone();
                    if current_frame == to {
                        return Ok(transforms);
                    }
                }
                Err(_) => break,
            }
        }

        if transforms.is_empty() {
            Err(TransformError::NotFound(from.to_string(), to.to_string()))
        } else {
            Ok(transforms)
        }
    }
    fn truncate_at_common_parent(
        from_chain: &mut VecDeque<Transform>,
        to_chain: &mut VecDeque<Transform>,
    ) {
        if let Some(index) = from_chain
            .iter()
            .position(|tf| to_chain.iter().any(|to_tf| to_tf.parent == tf.parent))
        {
            from_chain.truncate(index + 1);
        }

        if let Some(index) = to_chain
            .iter()
            .position(|tf| from_chain.iter().any(|from_tf| from_tf.parent == tf.parent))
        {
            to_chain.truncate(index + 1);
        }
    }

    fn combine_transforms(
        mut from_chain: VecDeque<Transform>,
        mut to_chain: VecDeque<Transform>,
    ) -> Result<Transform, TransformError> {
        from_chain.append(&mut to_chain);

        let mut iter = from_chain.into_iter();

        let mut final_transform = match iter.next() {
            Some(transform) => transform,
            None => {
                return Err(TransformError::NotFound(
                    "from".to_string(),
                    "to".to_string(),
                ));
            }
        };

        for transform in iter {
            final_transform = (transform * final_transform)?;
        }

        final_transform.inverse()
    }

    fn reverse_and_invert_transforms(
        chain: &mut VecDeque<Transform>
    ) -> Result<(), TransformError> {
        let reversed_and_inverted = chain
            .iter()
            .rev()
            .map(|item| item.inverse())
            .collect::<Result<VecDeque<Transform>, TransformError>>()?;

        *chain = reversed_and_inverted;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
