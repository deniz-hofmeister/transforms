use crate::types::Timestamp;
use core::{
    f64,
    ops::{Add, Div, Sub},
};
mod error;
pub use error::DurationError;

use crate::errors::TimestampError;

/// Represents a duration of time in nanoseconds.
///
/// The `Duration` struct encapsulates a time span measured in nanoseconds.
/// It provides a conversion from the standard library's `std::time::Duration` type.
///
/// # Examples
///
/// ```
/// use transforms::types::Duration;
///
/// // Create a standard duration of 2 seconds
/// let std_duration = std::time::Duration::new(2, 0);
///
/// // Convert it to our custom Duration type
/// let custom_duration: Duration = std_duration.into();
///
/// assert_eq!(custom_duration.nanoseconds, 2_000_000_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Default)]
pub struct Duration {
    pub nanoseconds: u128,
}

impl From<std::time::Duration> for Duration {
    fn from(d: std::time::Duration) -> Self {
        Self {
            nanoseconds: d.as_nanos(),
        }
    }
}

impl TryFrom<f64> for Duration {
    type Error = DurationError;

    fn try_from(seconds: f64) -> Result<Self, Self::Error> {
        if !seconds.is_finite() {
            return Err(DurationError::InvalidInput(
                "Duration must be finite".into(),
            ));
        }

        if seconds < 0.0 {
            return Err(DurationError::InvalidInput(
                "Duration cannot be negative".into(),
            ));
        }

        let nanos = seconds * 1e9;
        if nanos > u128::MAX as f64 {
            return Err(DurationError::DurationOverflow);
        }

        // Check for accuracy loss
        // This will likely trigger when Duration is greater than
        // 2^53 nanoseconds ~ 104 days.
        //
        // If such a duration would be necessary, consider using
        // static transforms or avoid using f64.
        let nanos_u128 = nanos as u128;
        let back_to_seconds = nanos_u128 as f64 / 1e9;
        if (back_to_seconds - seconds).abs() > f64::EPSILON {
            return Err(DurationError::AccuracyLoss);
        }

        Ok(Self {
            nanoseconds: nanos_u128,
        })
    }
}

impl Duration {
    /// Converts the `Duration` to seconds as a floating-point number.
    ///
    /// Returns an error if the conversion results in accuracy loss.
    ///
    /// # Examples
    ///
    /// ```
    /// # use transforms::types::Duration;
    ///
    /// let duration = Duration { nanoseconds: 1_000_000_000 };
    /// let result = duration.as_seconds();
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), 1.0);
    ///
    /// let duration = Duration { nanoseconds: 1_000_000_000_000_000_001 };
    /// let result = duration.as_seconds();
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DurationError::AccuracyLoss` if the conversion is not exact.
    pub fn as_seconds(&self) -> Result<f64, DurationError> {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        let seconds = self.nanoseconds as f64 / NANOSECONDS_PER_SECOND;

        if (seconds * NANOSECONDS_PER_SECOND) as u128 != self.nanoseconds {
            Err(DurationError::AccuracyLoss)
        } else {
            Ok(seconds)
        }
    }

    /// Converts the `Duration` to seconds as a floating-point number without checking for accuracy.
    ///
    /// # Examples
    ///
    /// ```
    /// # use transforms::types::Duration;
    ///
    /// let duration = Duration { nanoseconds: 1_000_000_000_000_000_001 };
    /// let seconds = duration.as_seconds_unchecked();
    /// assert_eq!(seconds, 1_000_000_000.0);
    /// ```
    pub fn as_seconds_unchecked(&self) -> f64 {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        self.nanoseconds as f64 / NANOSECONDS_PER_SECOND
    }
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

#[cfg(test)]
mod tests;
