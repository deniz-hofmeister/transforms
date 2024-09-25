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
        match self.data.entry(t.frame.clone()) {
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
        let mut from_iterator = Some(from.clone());
        let mut to_iterator = Some(to.clone());
        let mut from_transforms_vec = VecDeque::<Transform>::new();
        let mut to_transforms_vec = VecDeque::<Transform>::new();

        loop {
            let frame_buffer = self.data.get(from);
            if frame_buffer.is_none() {
                break;
            };

            let tf = frame_buffer.unwrap().get(&timestamp);
            if tf.is_none() {
                break;
            }

            from_transforms_vec.push_back(tf.unwrap());
        }

        None
    }
}

#[cfg(test)]
mod tests;
