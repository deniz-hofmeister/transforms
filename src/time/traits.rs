use core::time::Duration;

use crate::time::timestamp::TimestampError;

/// Trait describing timestamp behavior required by the transform core.
///
/// Implementing this trait allows using custom timestamp types with
/// `Transform`, `Buffer`, and `Registry`.
///
/// The trait requires `Copy` because transform lookups and composition are hot
/// paths where timestamps are passed around frequently.
///
/// # Adapter example
///
/// If your external timestamp type does not fit this trait directly, you can
/// create a small `Copy` adapter and convert at your application boundary.
///
/// ```
/// use core::time::Duration;
/// use transforms::{errors::TimestampError, time::TimestampLike};
///
/// #[derive(Debug, Clone)]
/// struct ExternalTime {
///     nanos_since_epoch: u64,
/// }
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// struct CoreTime(u64);
///
/// impl From<ExternalTime> for CoreTime {
///     fn from(value: ExternalTime) -> Self {
///         Self(value.nanos_since_epoch)
///     }
/// }
///
/// impl From<CoreTime> for ExternalTime {
///     fn from(value: CoreTime) -> Self {
///         Self {
///             nanos_since_epoch: value.0,
///         }
///     }
/// }
///
/// impl TimestampLike for CoreTime {
///     fn static_timestamp() -> Self {
///         Self(0)
///     }
///
///     fn duration_since(self, earlier: Self) -> Result<Duration, TimestampError> {
///         self.0
///             .checked_sub(earlier.0)
///             .map(Duration::from_nanos)
///             .ok_or(TimestampError::DurationUnderflow)
///     }
///
///     fn checked_add(self, rhs: Duration) -> Result<Self, TimestampError> {
///         let rhs_ns: u64 = rhs
///             .as_nanos()
///             .try_into()
///             .map_err(|_| TimestampError::DurationOverflow)?;
///
///         self.0
///             .checked_add(rhs_ns)
///             .map(Self)
///             .ok_or(TimestampError::DurationOverflow)
///     }
///
///     fn checked_sub(self, rhs: Duration) -> Result<Self, TimestampError> {
///         let rhs_ns: u64 = rhs
///             .as_nanos()
///             .try_into()
///             .map_err(|_| TimestampError::DurationOverflow)?;
///
///         self.0
///             .checked_sub(rhs_ns)
///             .map(Self)
///             .ok_or(TimestampError::DurationUnderflow)
///     }
///
///     fn as_seconds(self) -> Result<f64, TimestampError> {
///         Ok(self.0 as f64 / 1_000_000_000.0)
///     }
/// }
/// ```
pub trait TimestampLike: Copy + Ord {
    /// Returns the static timestamp value.
    ///
    /// By default this is usually `t=0`.
    fn static_timestamp() -> Self;

    /// Returns `true` if this timestamp is the static value.
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
