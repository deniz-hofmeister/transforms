use crate::types::{Duration, Timestamp, Transform};
use core::f64;
use std::collections::BTreeMap;
mod error;
pub use error::BufferError;

type NearestTransforms<'a> = (
    Option<(&'a Timestamp, &'a Transform)>,
    Option<(&'a Timestamp, &'a Transform)>,
);
pub struct Buffer {
    data: BTreeMap<Timestamp, Transform>,
    max_age: u128,
    is_static: bool,
}

impl Buffer {
    pub fn new(max_age: f64) -> Result<Self, BufferError> {
        if max_age <= 0.0 {
            return Err(BufferError::MaxAgeInvalid(max_age, f64::INFINITY));
        }

        if max_age == f64::INFINITY {
            return Ok(Self {
                data: BTreeMap::new(),
                max_age: u128::MAX,
                is_static: false,
            });
        }

        Ok(Self {
            data: BTreeMap::new(),
            max_age: (max_age * 1e9) as u128,
            is_static: false,
        })
    }

    pub fn insert(
        &mut self,
        transform: Transform,
    ) {
        self.is_static = transform.timestamp.nanoseconds == 0;
        self.data.insert(transform.timestamp, transform);
    }

    pub fn get(
        &mut self,
        timestamp: &Timestamp,
    ) -> Result<Transform, BufferError> {
        if self.is_static {
            match self.data.get(&Timestamp { nanoseconds: 0 }) {
                Some(tf) => return Ok(tf.clone()),
                None => return Err(BufferError::NoTransformAvailable),
            }
        };

        self.delete_expired();
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
                nanoseconds: self.max_age,
            };
        if let Ok(t) = timestamp_threshold {
            self.delete_before(t)
        }
    }

    fn delete_before(
        &mut self,
        timestamp: Timestamp,
    ) {
        self.data.retain(|&k, _| k >= timestamp);
    }
}

#[cfg(test)]
mod tests;
