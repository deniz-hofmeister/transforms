use crate::types::{Duration, Timestamp, Transform};
use std::collections::BTreeMap;
mod error;
pub use error::BufferError;

pub struct Buffer {
    data: BTreeMap<Timestamp, Transform>,
    max_age: f64,
}

impl Buffer {
    pub fn new(max_age: f64) -> Result<Self, BufferError> {
        if max_age <= 0.0 || max_age > f64::MAX {
            return Err(BufferError::MaxAgeInvalid(max_age, f64::MAX));
        }

        Ok(Self {
            data: BTreeMap::new(),
            max_age,
        })
    }

    pub fn insert(
        &mut self,
        transform: Transform,
    ) {
        self.data.insert(transform.timestamp, transform);
    }

    pub fn get(
        &self,
        timestamp: &Timestamp,
    ) -> Result<Transform, BufferError> {
        let (before, after) = self.get_nearest(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => Ok(Transform::interpolate(
                before.1.clone(),
                after.1.clone(),
                timestamp.clone(),
            )?),
            _ => Err(BufferError::NoTransformAvailable),
        }
    }

    pub fn get_exact(
        &self,
        timestamp: &Timestamp,
    ) -> Option<&Transform> {
        self.data.get(timestamp)
    }

    pub fn get_nearest(
        &self,
        timestamp: &Timestamp,
    ) -> (
        Option<(&Timestamp, &Transform)>,
        Option<(&Timestamp, &Transform)>,
    ) {
        let before = self.data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self.data.range(timestamp..).next();
        (before, after)
    }

    pub fn delete_expired(&mut self) {
        let timestamp_threshold = Timestamp::now()
            - Duration {
                nanoseconds: (self.max_age * 1e9) as i128,
            };
        self.delete_before(timestamp_threshold.unwrap());
    }

    pub fn delete_before(
        &mut self,
        timestamp: Timestamp,
    ) {
        self.data.retain(|&k, _| k >= timestamp);
    }
}

#[cfg(test)]
mod tests;
