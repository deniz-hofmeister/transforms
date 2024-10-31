use crate::types::Duration;
use core::ops::{Add, Sub};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod error;
pub use error::TimestampError;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Timestamp {
    pub nanoseconds: u128,
}

impl Timestamp {
    pub fn now() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Timestamp {
            nanoseconds: now.as_nanos(),
        }
    }

    pub fn as_seconds(&self) -> Result<f64, TimestampError> {
        let seconds = self.nanoseconds as f64 / 1_000_000_000.0;
        let rounded = (seconds * 1_000_000_000.0).round() / 1_000_000_000.0;

        if (seconds - rounded).abs() > f64::EPSILON {
            Err(TimestampError::AccuracyLoss)
        } else {
            Ok(seconds)
        }
    }

    pub fn as_milliseconds(&self) -> u128 {
        self.nanoseconds / 1_000_000
    }

    pub fn as_nanoseconds(&self) -> u128 {
        self.nanoseconds
    }

    pub fn from_seconds(seconds: f64) -> Self {
        let nanoseconds = (seconds * 1_000_000_000.0) as u128;
        Timestamp { nanoseconds }
    }

    pub fn from_milliseconds(milliseconds: u64) -> Self {
        Timestamp {
            nanoseconds: u128::from(milliseconds) * 1_000_000,
        }
    }

    pub fn from_nanoseconds(nanoseconds: u128) -> Self {
        Timestamp { nanoseconds }
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Result<Duration, TimestampError>;

    fn sub(
        self,
        other: Timestamp,
    ) -> Self::Output {
        match self.nanoseconds.cmp(&other.nanoseconds) {
            Ordering::Less => Err(TimestampError::DurationUnderflow),
            Ordering::Equal => Ok(Duration { nanoseconds: 0 }),
            Ordering::Greater => Ok(Duration {
                nanoseconds: self.nanoseconds - other.nanoseconds,
            }),
        }
    }
}

impl Add<Duration> for Timestamp {
    type Output = Result<Timestamp, TimestampError>;

    fn add(
        self,
        rhs: Duration,
    ) -> Self::Output {
        self.nanoseconds
            .checked_add(rhs.nanoseconds)
            .map(|result| Timestamp {
                nanoseconds: result,
            })
            .ok_or(TimestampError::DurationOverflow)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Result<Timestamp, TimestampError>;

    fn sub(
        self,
        rhs: Duration,
    ) -> Self::Output {
        self.nanoseconds
            .checked_sub(rhs.nanoseconds)
            .map(|result| Timestamp {
                nanoseconds: result,
            })
            .ok_or(TimestampError::DurationUnderflow)
    }
}

#[cfg(test)]
mod tests;
