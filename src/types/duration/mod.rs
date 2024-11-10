use crate::types::Timestamp;
use core::ops::{Add, Div, Sub};
mod error;
pub use error::DurationError;

use crate::errors::TimestampError;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Default)]
pub struct Duration {
    pub nanoseconds: u128,
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
    type Output = Result<Duration, DurationError>;

    fn div(
        self,
        rhs: f64,
    ) -> Self::Output {
        if rhs == 0.0 {
            return Err(DurationError::DivisionByZero);
        }

        let result = self.nanoseconds as f64 / rhs;

        Ok(Duration {
            nanoseconds: result as u128,
        })
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
