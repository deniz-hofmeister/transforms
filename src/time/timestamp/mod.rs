use core::{
    cmp::Ordering,
    ops::{Add, Sub},
    time::Duration,
};

#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

pub mod error;
pub use error::TimestampError;

/// A `Timestamp` represents a point in time. It is assumed that the time is measured in
/// nanoseconds when using feature = "std". The definition of the timestamp in a ```no_std``` environment
/// is free to be chosen by the user.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Timestamp {
    pub t: u128,
}

impl Timestamp {
    #[cfg(feature = "std")]
    #[must_use = "this returns the result of the operation"]
    /// Returns a `Timestamp` initialized to the current time.
    /// This functionality is useful for dynamic transforms.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let now = Timestamp::now();
    /// assert!(now.t > 0);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the system time is earlier than `UNIX_EPOCH` (January 1, 1970).
    pub fn now() -> Self {
        let duration_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        Timestamp {
            t: duration_since_epoch.as_nanos(),
        }
    }

    #[must_use = "this returns the result of the operation"]
    /// Returns a `Timestamp` initialized at zero.
    /// This functionality is especially useful for static transforms.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let zero = Timestamp::zero();
    /// assert_eq!(zero.t, 0);
    /// ```
    pub fn zero() -> Self {
        Timestamp { t: 0 }
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
    /// let timestamp = Timestamp { t: 1_000_000_000 };
    /// let result = timestamp.as_seconds();
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), 1.0);
    ///
    /// let timestamp = Timestamp {
    ///     t: 1_000_000_000_000_000_001,
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
        #[allow(clippy::cast_precision_loss)]
        let seconds = self.t as f64 / NANOSECONDS_PER_SECOND;

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        if (seconds * NANOSECONDS_PER_SECOND) as u128 == self.t {
            Ok(seconds)
        } else {
            Err(TimestampError::AccuracyLoss)
        }
    }

    #[must_use = "this returns the result of the operation"]
    /// Converts the `Timestamp` to seconds as a floating-point number without checking for accuracy.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp {
    ///     t: 1_000_000_000_000_000_001,
    /// };
    /// let seconds = timestamp.as_seconds_unchecked();
    /// assert_eq!(seconds, 1_000_000_000.0);
    /// ```
    #[allow(clippy::cast_precision_loss)]
    pub fn as_seconds_unchecked(&self) -> f64 {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        self.t as f64 / NANOSECONDS_PER_SECOND
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Result<Duration, TimestampError>;

    fn sub(
        self,
        other: Timestamp,
    ) -> Self::Output {
        match self.t.cmp(&other.t) {
            Ordering::Less => Err(TimestampError::DurationUnderflow),
            Ordering::Equal => Ok(Duration::from_secs(0)),
            Ordering::Greater => {
                let diff = self.t - other.t;
                let seconds = diff / 1_000_000_000;
                let nanos = (diff % 1_000_000_000) as u32;

                if seconds > u128::from(u64::MAX) {
                    return Err(TimestampError::DurationOverflow);
                }

                #[allow(clippy::cast_possible_truncation)]
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
        (u128::from(rhs.as_secs()))
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(u128::from(rhs.subsec_nanos())))
            .and_then(|total_duration_nanos| self.t.checked_add(total_duration_nanos))
            .map(|final_nanos| Timestamp { t: final_nanos })
            .ok_or(TimestampError::DurationOverflow)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Result<Timestamp, TimestampError>;

    fn sub(
        self,
        rhs: Duration,
    ) -> Self::Output {
        u128::from(rhs.as_secs())
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(u128::from(rhs.subsec_nanos())))
            .and_then(|total_duration_nanos| self.t.checked_sub(total_duration_nanos))
            .map(|final_nanos| Timestamp { t: final_nanos })
            .ok_or(TimestampError::DurationUnderflow)
    }
}

#[cfg(test)]
mod tests;
