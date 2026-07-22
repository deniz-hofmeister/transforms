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
    /// This value is a reserved sentinel: a transform carrying it is
    /// treated as static (valid for all time), so dynamic samples must
    /// never be recorded at exactly this value. The default `Timestamp`
    /// uses `t=0`; a type whose clock legitimately produces `t=0` as a
    /// dynamic instant (e.g. a boot-relative clock whose first sample can
    /// be zero) should pick a different sentinel, such as the maximum
    /// representable value.
    ///
    /// Implementations must also keep `Ord` total and consistent with
    /// [`TimePoint::duration_since`] and the checked arithmetic: if
    /// `a < b`, then `b.duration_since(a)` is the `Ok` span between them.
    /// Buffer ordering, interpolation, and eviction all rest on that
    /// consistency.
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

    /// Returns the timestamp in seconds for diagnostics, accepting precision
    /// loss.
    ///
    /// Error messages are formatted with this method: unlike
    /// [`TimePoint::as_seconds`] it cannot fail, so a conversion error can
    /// never mask the error actually being reported. The default
    /// implementation falls back to NaN when `as_seconds` fails; implementors
    /// should override it with a lossy conversion.
    fn as_seconds_lossy(self) -> f64 {
        self.as_seconds().unwrap_or(f64::NAN)
    }
}
