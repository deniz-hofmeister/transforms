use crate::types::{Buffer, Timestamp, Transform};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
mod error;
use crate::error::BufferError;

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
    ) -> Option<Transform> {
        //TODO: Finish this.
        // 1. Iterate upwards the transform tree until parents don't exist
        // 2. find out if both the to-tree and the from-tree mention the same parents anywhere in
        //    the list.
        // 2b. if not, find out if the final parent, is the same parent, if so, create parent
        // 2c. if not, exit
        // 3. Assemble transforms

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

        // TODO: Implement the rest of the function
        None
    }
}

#[cfg(test)]
mod tests;
