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
        to: &'a str,
        timestamp: Timestamp,
    ) -> Option<Transform> {
        let (before, after) = self.data.get(from)?.get_nearest(&timestamp);
        if before.is_none() || after.is_none() {
            return None;
        } else {
            Some(Transform::interpolate(
                before.unwrap().1.clone(),
                after.unwrap().1.clone(),
                timestamp,
            ))
        }
    }
}
