use crate::types::{Buffer, Transform};
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

    pub fn add(
        &mut self,
        t: Transform,
    ) {
        let key = format!("{}_{}", t.parent, t.child);
        self.data
            .entry(key)
            .or_insert_with(|| Buffer::new(self.max_age))
            .insert(t.transform);
    }
}
