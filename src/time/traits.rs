use core::time::Duration;

use crate::time::timestamp::TimestampError;

/// Trait describing time-point behavior required by the transform core.
///
/// In plain terms, this is the "time contract" for the library.
///
/// - [`TimePoint`] is the interface: it defines what a time type must do.
/// - [`crate::time::Timestamp`] is the default concrete type that implements this interface.
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
/// use transforms::{errors::TimestampError, time::TimePoint};
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
    /// Returns `TimestampError::DurationUnderflow` if `earlier` is later than `self`.
    /// Implementations may return another `TimestampError` variant if conversion to
    /// `Duration` is not possible.
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimestampError>;

    /// Adds duration to timestamp using checked arithmetic.
    ///
    /// # Errors
    ///
    /// Returns `TimestampError::DurationOverflow` if the addition exceeds the
    /// representable range of the timestamp type.
    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError>;

    /// Subtracts duration from timestamp using checked arithmetic.
    ///
    /// # Errors
    ///
    /// Returns `TimestampError::DurationUnderflow` if subtraction would produce
    /// a value smaller than the representable range of the timestamp type.
    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError>;

    /// Returns timestamp represented in seconds.
    ///
    /// # Errors
    ///
    /// Returns a `TimestampError` if the conversion cannot be represented according
    /// to the implementation's precision and range guarantees.
    fn as_seconds(self) -> Result<f64, TimestampError>;
}
