use crate::types::Timestamp;
use core::ops::{Add, Div, Sub};
mod error;
pub use error::DurationError;

use crate::error::TimestampError;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Default)]
pub struct Duration {
    pub nanoseconds: i128,
}

impl Add<Timestamp> for Duration {
    type Output = Result<Timestamp, TimestampError>;

    fn add(
        self,
        rhs: Timestamp,
    ) -> Self::Output {
        rhs + self
    }
}

impl Sub<Timestamp> for Duration {
    type Output = Result<Timestamp, TimestampError>;

    fn sub(
        self,
        rhs: Timestamp,
    ) -> Self::Output {
        rhs - self
    }
}

impl Div<f64> for Duration {
    type Output = Duration;

    fn div(
        self,
        rhs: f64,
    ) -> Self::Output {
        Duration {
            nanoseconds: (self.nanoseconds as f64 / rhs).round() as i128,
        }
    }
}

impl Duration {
    pub fn as_seconds(&self) -> Result<f64, DurationError> {
        let seconds = self.nanoseconds as f64 / 1_000_000_000.0;
        let rounded = (seconds * 1_000_000_000.0).round() / 1_000_000_000.0;

        if (seconds - rounded).abs() > f64::EPSILON {
            Err(DurationError::AccuracyLoss)
        } else {
            Ok(seconds)
        }
    }
}

#[cfg(test)]
mod tests;
