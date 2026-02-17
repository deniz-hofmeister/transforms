use core::time::Duration;

use crate::time::timestamp::TimestampError;

/// Trait describing timestamp behavior required by the transform core.
///
/// Implementing this trait allows using custom timestamp types with
/// `Transform`, `Buffer`, and `Registry`.
pub trait TimestampLike: Copy + Ord {
    /// Returns the sentinel timestamp used for static transforms.
    fn static_timestamp() -> Self;

    /// Returns `true` if this timestamp is the static sentinel.
    fn is_static(self) -> bool {
        self == Self::static_timestamp()
    }

    /// Returns elapsed time between two timestamps.
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimestampError>;

    /// Adds duration to timestamp using checked arithmetic.
    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError>;

    /// Subtracts duration from timestamp using checked arithmetic.
    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError>;

    /// Returns timestamp represented in seconds.
    fn as_seconds(self) -> Result<f64, TimestampError>;
}
