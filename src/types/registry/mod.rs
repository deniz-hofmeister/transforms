use crate::types::{Buffer, Timestamp, Transform};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
mod error;
use crate::error::{BufferError, TransformError};

pub struct Registry {
    pub data: HashMap<String, Buffer>,
    pub max_age: f64,
}

impl Registry {
    pub fn new(max_age: f64) -> Self {
        Self {
            data: HashMap::new(),
            max_age,
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
                let buffer = Buffer::new(self.max_age)?;
                let buffer = entry.insert(buffer);
                buffer.insert(t);
            }
        }
        Ok(())
    }

    pub fn get_transform(
        &self,
        from: &str,
        to: &str,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        let from_chain = self.get_transform_chain(from, to, timestamp);
        let to_chain = self.get_transform_chain(to, from, timestamp);

        match (from_chain, to_chain) {
            (Ok(mut from_chain), Ok(mut to_chain)) => {
                Self::find_common_parent(&mut from_chain, &mut to_chain);
                Self::combine_transforms(from_chain, to_chain)
            }
            (Ok(from_chain), Err(_)) => Self::combine_transforms(from_chain, VecDeque::new()),
            (Err(_), Ok(to_chain)) => Self::combine_transforms(VecDeque::new(), to_chain),
            (Err(_), Err(_)) => Err(TransformError::NotFound(from.to_string(), to.to_string())),
        }
    }

    fn get_transform_chain(
        &self,
        from: &str,
        to: &str,
        timestamp: Timestamp,
    ) -> Result<VecDeque<Transform>, TransformError> {
        let mut transforms = VecDeque::new();
        let mut current_frame = from.to_string();

        while let Some(frame_buffer) = self.data.get(&current_frame) {
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

    fn find_common_parent(
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
        let mut final_transform = if let Some(transform) = from_chain.pop_front() {
            transform
        } else if let Some(transform) = to_chain.pop_back() {
            transform.inverse()?
        } else {
            return Err(TransformError::NotFound(
                "from".to_string(),
                "to".to_string(),
            ));
        };

        for transform in to_chain.into_iter() {
            final_transform = (final_transform * transform.inverse()?)?;
        }

        for transform in from_chain.into_iter().rev() {
            final_transform = (final_transform * transform)?;
        }

        Ok(final_transform.inverse()?)
    }
}

#[cfg(test)]
mod tests;
