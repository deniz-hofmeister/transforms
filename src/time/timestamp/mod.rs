use core::{
    cmp::Ordering,
    ops::{Add, Sub},
    time::Duration,
};

pub mod error;
pub use error::TimestampError;

/// A `Timestamp` represents a point in time as nanoseconds since the UNIX epoch.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Timestamp {
    pub time: u128,
}

impl Timestamp {
    /// Returns the current time as a `Timestamp`.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let now = Timestamp::now();
    /// ```
    pub fn set(time: u128) -> Self {
        Timestamp { time }
    }

    /// Returns a `Timestamp` representing the UNIX epoch (0 nanoseconds).
    /// This functionality is especially useful for static transforms.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let zero = Timestamp::zero();
    /// assert_eq!(zero.nanoseconds, 0);
    /// ```
    pub fn zero() -> Self {
        Timestamp { time: 0 }
    }

    /// Converts the `Timestamp` to seconds as a floating-point number.
    ///
    /// Returns an error if the conversion results in accuracy loss.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp {
    ///     nanoseconds: 1_000_000_000,
    /// };
    /// let result = timestamp.as_seconds();
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), 1.0);
    ///
    /// let timestamp = Timestamp {
    ///     nanoseconds: 1_000_000_000_000_000_001,
    /// };
    /// let result = timestamp.as_seconds();
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `TimestampError::AccuracyLoss` if the conversion is not exact.
    pub fn as_seconds(&self) -> Result<f64, TimestampError> {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        let seconds = self.time as f64 / NANOSECONDS_PER_SECOND;

        if (seconds * NANOSECONDS_PER_SECOND) as u128 != self.time {
            Err(TimestampError::AccuracyLoss)
        } else {
            Ok(seconds)
        }
    }

    /// Converts the `Timestamp` to seconds as a floating-point number without checking for accuracy.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp {
    ///     nanoseconds: 1_000_000_000_000_000_001,
    /// };
    /// let seconds = timestamp.as_seconds_unchecked();
    /// assert_eq!(seconds, 1_000_000_000.0);
    /// ```
    pub fn as_seconds_unchecked(&self) -> f64 {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        self.time as f64 / NANOSECONDS_PER_SECOND
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Result<Duration, TimestampError>;

    fn sub(
        self,
        other: Timestamp,
    ) -> Self::Output {
        match self.time.cmp(&other.time) {
            Ordering::Less => Err(TimestampError::DurationUnderflow),
            Ordering::Equal => Ok(Duration::from_secs(0)),
            Ordering::Greater => {
                let diff = self.time - other.time;
                let seconds = diff / 1_000_000_000;
                let nanos = (diff % 1_000_000_000) as u32;

                if seconds > u64::MAX as u128 {
                    return Err(TimestampError::DurationOverflow);
                }

                Ok(Duration::new(seconds as u64, nanos))
            }
        }
    }
}

impl Add<Duration> for Timestamp {
    type Output = Result<Timestamp, TimestampError>;

    fn add(
        self,
        rhs: Duration,
    ) -> Self::Output {
        (rhs.as_secs() as u128)
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(rhs.subsec_nanos() as u128))
            .and_then(|total_duration_nanos| self.time.checked_add(total_duration_nanos))
            .map(|final_nanos| Timestamp { time: final_nanos })
            .ok_or(TimestampError::DurationOverflow)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Result<Timestamp, TimestampError>;

    fn sub(
        self,
        rhs: Duration,
    ) -> Self::Output {
        (rhs.as_secs() as u128)
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(rhs.subsec_nanos() as u128))
            .and_then(|total_duration_nanos| self.time.checked_sub(total_duration_nanos))
            .map(|final_nanos| Timestamp { time: final_nanos })
            .ok_or(TimestampError::DurationUnderflow)
    }
}

#[cfg(test)]
mod tests;
