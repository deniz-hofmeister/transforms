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

    pub fn get_transform<'a>(
        &mut self,
        from: &'a str,
        to: &'a str,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        let mut from_transforms_vec = VecDeque::<Transform>::new();
        let mut frame = from.to_string();
        loop {
            if let Some(frame_buffer) = self.data.get(&frame) {
                if let Ok(tf) = frame_buffer.get(&timestamp) {
                    from_transforms_vec.push_back(tf.clone());
                    frame = tf.parent.clone();
                    if frame == to {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let mut to_transforms_vec = VecDeque::<Transform>::new();
        let mut frame = to.to_string();
        loop {
            if let Some(frame_buffer) = self.data.get(&frame) {
                if let Ok(tf) = frame_buffer.get(&timestamp) {
                    to_transforms_vec.push_back(tf.clone());
                    frame = tf.parent.clone();
                    if frame == from {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(index) = from_transforms_vec.iter().position(|tf| {
            to_transforms_vec
                .iter()
                .any(|to_tf| to_tf.parent == tf.parent)
        }) {
            from_transforms_vec.truncate(index + 1);
        }

        if let Some(index) = to_transforms_vec.iter().position(|tf| {
            from_transforms_vec
                .iter()
                .any(|to_tf| to_tf.parent == tf.parent)
        }) {
            to_transforms_vec.truncate(index + 1);
        }

        let mut final_transform = Transform::identity();
        for transform in from_transforms_vec.iter() {
            final_transform = (final_transform * transform.clone())?;
        }

        for transform in to_transforms_vec.iter().rev() {
            final_transform = (final_transform * transform.inverse()?)?;
        }

        Ok(final_transform)
    }
}

#[cfg(test)]
mod tests;
