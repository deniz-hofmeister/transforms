use crate::types::Timestamp;
use core::ops::{Add, Sub};

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

#[cfg(test)]
mod tests;
