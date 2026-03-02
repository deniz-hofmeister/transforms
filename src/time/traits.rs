use core::time::Duration;

use crate::time::TimeError;

/// Trait describing time-point behavior required by the transform core.
///
/// Implementing this trait allows using custom time types with
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
/// use transforms::{errors::TimeError, time::TimePoint};
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
/// impl TimePoint for CoreTime {
///     fn static_timestamp() -> Self {
///         Self(0)
///     }
///
///     fn duration_since(
///         self,
///         earlier: Self,
///     ) -> Result<Duration, TimeError> {
///         self.0
///             .checked_sub(earlier.0)
///             .map(Duration::from_nanos)
///             .ok_or(TimeError::DurationUnderflow)
///     }
///
///     fn checked_add(
///         self,
///         rhs: Duration,
///     ) -> Result<Self, TimeError> {
///         let rhs_ns: u64 = rhs
///             .as_nanos()
///             .try_into()
///             .map_err(|_| TimeError::DurationOverflow)?;
///
///         self.0
///             .checked_add(rhs_ns)
///             .map(Self)
///             .ok_or(TimeError::DurationOverflow)
///     }
///
///     fn checked_sub(
///         self,
///         rhs: Duration,
///     ) -> Result<Self, TimeError> {
///         let rhs_ns: u64 = rhs
///             .as_nanos()
///             .try_into()
///             .map_err(|_| TimeError::DurationOverflow)?;
///
///         self.0
///             .checked_sub(rhs_ns)
///             .map(Self)
///             .ok_or(TimeError::DurationUnderflow)
///     }
///
///     fn as_seconds(self) -> Result<f64, TimeError> {
///         Ok(self.0 as f64 / 1_000_000_000.0)
///     }
/// }
/// ```
pub trait TimePoint: Copy + Ord {
    /// Returns the static timestamp value.
    ///
    /// By default this is usually `t=0`.
    fn static_timestamp() -> Self;

    /// Returns `true` if this timestamp is the static value.
    fn is_static(self) -> bool {
        self == Self::static_timestamp()
    }

    /// Returns elapsed time between two timestamps.
    ///
    /// # Errors
    ///
    /// Returns `TimeError::DurationUnderflow` if `earlier` is later than `self`.
    /// Implementations may return another `TimeError` variant if conversion to
    /// `Duration` is not possible.
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError>;

    /// Adds duration to timestamp using checked arithmetic.
    ///
    /// # Errors
    ///
    /// Returns `TimeError::DurationOverflow` if the addition exceeds the
    /// representable range of the timestamp type.
    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError>;

    /// Subtracts duration from timestamp using checked arithmetic.
    ///
    /// # Errors
    ///
    /// Returns `TimeError::DurationUnderflow` if subtraction would produce
    /// a value smaller than the representable range of the timestamp type.
    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError>;

    /// Returns timestamp represented in seconds.
    ///
    /// # Errors
    ///
    /// Returns a `TimeError` if the conversion cannot be represented according
    /// to the implementation's precision and range guarantees.
    fn as_seconds(self) -> Result<f64, TimeError>;
}
