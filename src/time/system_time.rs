use crate::time::{TimeError, TimePoint};
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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
}
