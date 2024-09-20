use crate::types::{Buffer, Timestamp, Transform};
use std::collections::HashMap;

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
        _to: &'a str,
        timestamp: Timestamp,
    ) -> Option<Transform> {
        self.data.get(from)?.get(&timestamp)
    }
}
