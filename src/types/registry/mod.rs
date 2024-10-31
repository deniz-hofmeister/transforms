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
        &mut self,
        from: &str,
        to: &str,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        let from_chain = self.get_transform_chain(from, to, timestamp);
        let mut to_chain = self.get_transform_chain(to, from, timestamp);

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
        &mut self,
        from: &str,
        to: &str,
        timestamp: Timestamp,
    ) -> Result<VecDeque<Transform>, TransformError> {
        let mut transforms = VecDeque::new();
        let mut current_frame = from.to_string();

        while let Some(frame_buffer) = self.data.get_mut(&current_frame) {
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

        if from_chain.is_empty() {
            return Err(TransformError::NotFound(
                "from".to_string(),
                "to".to_string(),
            ));
        }

        let mut final_transform = from_chain.pop_front().unwrap();

        for transform in from_chain.into_iter() {
            final_transform = (transform * final_transform)?;
        }

        Ok(final_transform.inverse()?)
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
