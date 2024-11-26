use crate::types::{Duration, Timestamp, Transform};
use std::collections::BTreeMap;
mod error;
pub use error::BufferError;

type NearestTransforms<'a> = (
    Option<(&'a Timestamp, &'a Transform)>,
    Option<(&'a Timestamp, &'a Transform)>,
);

pub struct Buffer {
    data: BTreeMap<Timestamp, Transform>,
    ttl: u128,
    is_static: bool,
}

impl Buffer {
    pub fn new(ttl: Duration) -> Self {
        Self {
            data: BTreeMap::new(),
            ttl: ttl.nanoseconds,
            is_static: false,
        }
    }

    pub fn insert(
        &mut self,
        transform: Transform,
    ) {
        self.is_static = transform.timestamp.nanoseconds == 0;
        self.data.insert(transform.timestamp, transform);

        if !self.is_static {
            self.delete_expired();
        };
    }

    pub fn get(
        &self,
        timestamp: &Timestamp,
    ) -> Result<Transform, BufferError> {
        if self.is_static {
            match self.data.get(&Timestamp { nanoseconds: 0 }) {
                Some(tf) => return Ok(tf.clone()),
                None => return Err(BufferError::NoTransformAvailable),
            }
        };

        let (before, after) = self.get_nearest(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => Ok(Transform::interpolate(
                before.1.clone(),
                after.1.clone(),
                *timestamp,
            )?),
            _ => Err(BufferError::NoTransformAvailable),
        }
    }

    fn get_nearest(
        &self,
        timestamp: &Timestamp,
    ) -> NearestTransforms {
        let before = self.data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self.data.range(timestamp..).next();
        (before, after)
    }

    fn delete_expired(&mut self) {
        let timestamp_threshold = Timestamp::now()
            - Duration {
                nanoseconds: self.ttl,
            };
        if let Ok(t) = timestamp_threshold {
            self.data.retain(|&k, _| k >= t);
        }
    }
}

#[cfg(test)]
mod tests;
