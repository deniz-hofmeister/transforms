//! `TimePoint` implementation for `std::time::SystemTime`.

use crate::time::{TimeError, TimePoint};
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

/// `UNIX_EPOCH` is the static sentinel: a transform stamped exactly at the
/// epoch is treated as static. Note that `SystemTime` is wall-clock time and
/// not monotonic — clock adjustments (NTP steps, manual changes) can move it
/// backwards; prefer a monotonic custom `TimePoint` where that matters.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl TimePoint for SystemTime {
    fn static_timestamp() -> Self {
        UNIX_EPOCH
    }

    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        SystemTime::duration_since(&self, earlier).map_err(|_| TimeError::DurationUnderflow)
    }

    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        SystemTime::checked_add(&self, rhs).ok_or(TimeError::DurationOverflow)
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        SystemTime::checked_sub(&self, rhs).ok_or(TimeError::DurationUnderflow)
    }

    fn as_seconds(self) -> Result<f64, TimeError> {
        SystemTime::duration_since(&self, UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .map_err(|_| TimeError::DurationUnderflow)
    }

    fn as_seconds_lossy(self) -> f64 {
        SystemTime::duration_since(&self, UNIX_EPOCH)
            .map_or(f64::NAN, |duration| duration.as_secs_f64())
    }
}
