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
/// The static-transform sentinel is [`Timestamp::STATIC`] (`u128::MAX`
/// nanoseconds, ~10²² years) — a value no real clock produces, so every
/// ordinary instant including `t = 0` is a valid dynamic timestamp. This
/// makes `Timestamp` safe for boot-relative clocks whose first reading is
/// zero.
///
/// For custom clocks, implement `crate::time::TimePoint` on your own type and
/// use it with `Registry<T>`.
///
/// With the optional `serde` feature, this type implements `Serialize` and
/// `Deserialize` (the docs.rs listing cannot banner derive-generated impls).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp {
    /// Nanoseconds since the epoch of the chosen clock.
    t: u128,
}

impl Timestamp {
    /// The static-transform sentinel: a transform carrying this timestamp
    /// is valid for all time.
    ///
    /// `u128::MAX` nanoseconds lies ~10²² years in the future — no wall
    /// clock or boot-relative clock ever produces it organically, so no
    /// real instant is sacrificed to the reservation. Prefer
    /// [`Transform::static_between`](crate::geometry::Transform::static_between)
    /// over spelling the sentinel out.
    pub const STATIC: Timestamp = Timestamp { t: u128::MAX };

    /// Returns a `Timestamp` initialized to the current time.
    ///
    /// This functionality is useful for dynamic transforms.
    ///
    /// # Panics
    ///
    /// Panics if the system time is earlier than `UNIX_EPOCH` (January 1,
    /// 1970). Use [`Timestamp::try_now`] for the panic-free variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let now = Timestamp::now();
    /// assert!(now.as_nanos() > 0);
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[must_use]
    #[allow(
        clippy::expect_used,
        reason = "the pre-epoch panic is documented above; no meaningful recovery exists"
    )]
    pub fn now() -> Self {
        Self::try_now().expect("time went backwards")
    }

    /// Returns a `Timestamp` initialized to the current time, or an error
    /// if the system clock reports a time before `UNIX_EPOCH` (January 1,
    /// 1970).
    ///
    /// The panic-free counterpart of [`Timestamp::now`].
    ///
    /// # Errors
    ///
    /// Returns `TimeError::DurationUnderflow` if the system clock is set
    /// before the Unix epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let now = Timestamp::try_now().unwrap();
    /// assert!(now.as_nanos() > 0);
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn try_now() -> Result<Self, TimeError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration_since_epoch| Timestamp {
                t: duration_since_epoch.as_nanos(),
            })
            .map_err(|_| TimeError::DurationUnderflow)
    }

    /// Returns a `Timestamp` initialized at zero.
    ///
    /// Zero is an ordinary dynamic instant — the epoch of the chosen
    /// clock. The static-transform sentinel is [`Timestamp::STATIC`], not
    /// zero, so a boot-relative clock's first reading needs no special
    /// handling.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let zero = Timestamp::zero();
    /// assert_eq!(zero.as_nanos(), 0);
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
    /// `f64` has a 53-bit mantissa, so timestamps up to 2^53 nanoseconds
    /// (about 104 days) convert with sub-nanosecond accuracy; beyond that the
    /// conversion silently loses precision, which this method refuses to do.
    /// Use [`Timestamp::as_seconds_lossy`] (or
    /// [`TimePoint::as_seconds_lossy`]) for a best-effort conversion of
    /// larger values, such as wall-clock times.
    ///
    /// # Errors
    ///
    /// Returns `TimeError::AccuracyLoss` if the timestamp exceeds 2^53
    /// nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    /// assert_eq!(timestamp.as_seconds().unwrap(), 1.0);
    ///
    /// // Beyond 2^53 ns, sub-nanosecond accuracy is unrepresentable.
    /// let timestamp = Timestamp::from_nanos(1_000_000_000_000_000_001);
    /// assert!(timestamp.as_seconds().is_err());
    /// ```
    pub fn as_seconds(&self) -> Result<f64, TimeError> {
        const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
        /// 2^53: the largest range in which `f64` represents every integer
        /// nanosecond count exactly.
        const MAX_ACCURATE_NANOS: u128 = 1 << 53;

        if self.t > MAX_ACCURATE_NANOS {
            return Err(TimeError::AccuracyLoss);
        }
        #[allow(clippy::cast_precision_loss)]
        Ok(self.t as f64 / NANOSECONDS_PER_SECOND)
    }

    /// Converts the `Timestamp` to seconds as a floating-point number,
    /// accepting precision loss beyond 2^53 nanoseconds.
    ///
    /// Inherent counterpart of [`TimePoint::as_seconds_lossy`], callable
    /// without importing the trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::time::Timestamp;
    ///
    /// let timestamp = Timestamp::from_nanos(1_000_000_000_000_000_001);
    /// let seconds = timestamp.as_seconds_lossy();
    /// assert_eq!(seconds, 1_000_000_000.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_seconds_lossy(&self) -> f64 {
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
        Timestamp::STATIC
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
        Timestamp::as_seconds_lossy(&self)
    }
}

#[cfg(test)]
mod tests;
