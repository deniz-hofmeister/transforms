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

    pub fn get_transform<'a>(
        &'a self,
        source: &'a str,
        target: &'a str,
        timestamp: Timestamp,
    ) -> Option<Transform> {
        let key = format!("{}_{}", source, target);
        let r = match self
            .data
            .get(&key)
        {
            Some(v) => v.get(&timestamp),
            None => return None,
        };

        Some(Transform {
            parent: source,
            child: target,
            transform: *r.unwrap(),
        })
    }
}
