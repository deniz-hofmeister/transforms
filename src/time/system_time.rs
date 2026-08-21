//! `TimePoint` implementation for `std::time::SystemTime`.

use crate::time::{TimeError, TimePoint};
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

/// Note that `SystemTime` is wall-clock time and not monotonic — clock
/// adjustments (NTP steps, manual changes) can move it backwards; prefer a
/// monotonic custom `TimePoint` where that matters. No value is reserved:
/// a transform stamped exactly at `UNIX_EPOCH` is an ordinary dynamic
/// sample, and staticness is expressed by `Stamp::Static`.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl TimePoint for SystemTime {
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        SystemTime::duration_since(&self, earlier).map_err(|_| TimeError::DurationUnderflow)
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        SystemTime::checked_sub(&self, rhs).ok_or(TimeError::DurationUnderflow)
    }

    /// A time before `UNIX_EPOCH` has no representation as seconds since the
    /// epoch, so it yields NaN rather than a plausible-looking number.
    fn as_seconds_lossy(self) -> f64 {
        SystemTime::duration_since(&self, UNIX_EPOCH)
            .map_or(f64::NAN, |duration| duration.as_secs_f64())
    }
}
