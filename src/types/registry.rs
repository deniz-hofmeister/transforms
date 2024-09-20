use crate::types::{Buffer, Timestamp, Transform};
use std::collections::HashMap;
use std::rc::Rc;

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
        source: &'a str,
        target: &'a str,
        timestamp: Timestamp,
    ) -> Option<&Rc<Transform>> {
        let key = format!("{}_{}", source, target);
        self.data.get(&key)?.get(&timestamp)
    }
}
