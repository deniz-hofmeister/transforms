use crate::types::{Buffer, Timestamp, Transform};
use std::collections::{HashMap, VecDeque};

pub struct Registry {
    pub data: HashMap<String, Buffer>,
    pub max_age: u128,
}

impl Registry {
    pub fn new(max_age: u128) -> Self {
        Self {
            data: HashMap::new(),
            max_age,
        }
    }

    pub fn add_transform(
        &mut self,
        t: Transform,
    ) {
        self.data
            .entry(t.frame.clone())
            .or_insert_with(|| Buffer::new(self.max_age))
            .insert(t.into());
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
            let buffer = self.data.get(from);
            if buffer.is_none() {
                break;
            };

            let transf = buffer.unwrap().get(&timestamp);
            if transf.is_none() {
                break;
            }

            from_transforms_vec.push_back(transf.unwrap());
        }

        None
    }
}
