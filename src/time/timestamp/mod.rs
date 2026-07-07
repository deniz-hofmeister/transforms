//! The default nanosecond-resolution timestamp type.

use core::{
    cmp::Ordering,
    ops::{Add, Sub},
    time::Duration,
};

use crate::time::{TimeError, TimePoint};

#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

/// Default concrete time type used by this crate.
///
/// `Timestamp` stores a time value in `u128` nanoseconds.
///
/// For custom clocks, implement `crate::time::TimePoint` on your own type and
/// use it with `Registry<T>`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Timestamp {
    /// Nanoseconds since the epoch of the chosen clock.
    pub t: u128,
}

impl Timestamp {
    /// Returns a `Timestamp` initialized to the current time.
    ///
    /// This functionality is useful for dynamic transforms.
    ///
    /// # Panics
    ///
    /// Panics if the system time is earlier than `UNIX_EPOCH` (January 1, 1970).
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let now = Timestamp::now();
    /// assert!(now.t > 0);
    /// ```
    #[cfg(feature = "std")]
    #[must_use]
    #[allow(
        clippy::expect_used,
        reason = "the pre-epoch panic is documented above; no meaningful recovery exists"
    )]
    pub fn now() -> Self {
        let duration_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");

        Timestamp {
            t: duration_since_epoch.as_nanos(),
        }
    }

    /// Returns a `Timestamp` initialized at zero.
    ///
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
    #[must_use]
    pub const fn zero() -> Self {
        Timestamp { t: 0 }
    }

    /// Creates a `Timestamp` from a number of nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    /// assert_eq!(timestamp.as_seconds().unwrap(), 1.0);
    /// ```
    #[must_use]
    pub const fn from_nanos(nanos: u128) -> Self {
        Timestamp { t: nanos }
    }

    /// Returns the timestamp as nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    /// assert_eq!(timestamp.as_nanos(), 1_000_000_000);
    /// ```
    #[must_use]
    pub const fn as_nanos(&self) -> u128 {
        self.t
    }

    /// Converts the `Timestamp` to seconds as a floating-point number.
    ///
    /// # Errors
    ///
    /// Returns `TimeError::AccuracyLoss` if the conversion is not exact.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    /// let result = timestamp.as_seconds();
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), 1.0);
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000_000_000_001);
    /// let result = timestamp.as_seconds();
    /// assert!(result.is_err());
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn as_seconds(&self) -> Result<f64, TimeError> {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let seconds = self.t as f64 / NANOSECONDS_PER_SECOND;

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        if (seconds * NANOSECONDS_PER_SECOND) as u128 == self.t {
            Ok(seconds)
        } else {
            Err(TimeError::AccuracyLoss)
        }
    }

    /// Converts the `Timestamp` to seconds as a floating-point number without checking for accuracy.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000_000_000_001);
    /// let seconds = timestamp.as_seconds_unchecked();
    /// assert_eq!(seconds, 1_000_000_000.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_seconds_unchecked(&self) -> f64 {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        self.t as f64 / NANOSECONDS_PER_SECOND
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Result<Duration, TimeError>;

    fn sub(
        self,
        other: Timestamp,
    ) -> Self::Output {
        match self.t.cmp(&other.t) {
            Ordering::Less => Err(TimeError::DurationUnderflow),
            Ordering::Equal => Ok(Duration::from_secs(0)),
            Ordering::Greater => {
                let diff = self.t - other.t;
                let seconds = diff / 1_000_000_000;
                let nanos = (diff % 1_000_000_000) as u32;

                if seconds > u128::from(u64::MAX) {
                    return Err(TimeError::DurationOverflow);
                }

                #[allow(clippy::cast_possible_truncation)]
                Ok(Duration::new(seconds as u64, nanos))
            }
        }
    }
}

impl Add<Duration> for Timestamp {
    type Output = Result<Timestamp, TimeError>;

    fn add(
        self,
        rhs: Duration,
    ) -> Self::Output {
        (u128::from(rhs.as_secs()))
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(u128::from(rhs.subsec_nanos())))
            .and_then(|total_duration_nanos| self.t.checked_add(total_duration_nanos))
            .map(|final_nanos| Timestamp { t: final_nanos })
            .ok_or(TimeError::DurationOverflow)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Result<Timestamp, TimeError>;

    fn sub(
        self,
        rhs: Duration,
    ) -> Self::Output {
        u128::from(rhs.as_secs())
            .checked_mul(1_000_000_000)
            .and_then(|seconds| seconds.checked_add(u128::from(rhs.subsec_nanos())))
            .and_then(|total_duration_nanos| self.t.checked_sub(total_duration_nanos))
            .map(|final_nanos| Timestamp { t: final_nanos })
            .ok_or(TimeError::DurationUnderflow)
    }
}

impl TimePoint for Timestamp {
    fn static_timestamp() -> Self {
        Timestamp::zero()
    }

    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        self - earlier
    }

    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        self + rhs
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        self - rhs
    }

    fn as_seconds(self) -> Result<f64, TimeError> {
        Timestamp::as_seconds(&self)
    }

    fn as_seconds_lossy(self) -> f64 {
        self.as_seconds_unchecked()
    }
}

#[cfg(test)]
mod tests;
