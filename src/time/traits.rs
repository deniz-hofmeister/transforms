use core::time::Duration;

use crate::time::TimeError;

/// Trait describing time-point behavior required by the transform core.
///
/// Implementing this trait allows using custom time types with
/// `Transform`, `Buffer`, and `Registry`.
///
/// The trait requires `Copy` because transform lookups and composition are hot
/// paths where timestamps are passed around frequently, and `Debug` because
/// every type that carries a timestamp derives `Debug`: without the bound a
/// clock type without its own derive would make `Transform<YourClock>`
/// silently unprintable, exactly where a diagnosis is needed.
///
/// The three methods are the whole time algebra the crate uses; nothing is
/// required that the core does not call.
///
/// Implementations must keep `Ord` total and consistent with
/// [`TimePoint::duration_since`] and [`TimePoint::checked_sub`]: if `a < b`,
/// then `b.duration_since(a)` is the `Ok` span between them. Buffer
/// ordering, interpolation, and eviction all rest on that consistency.
///
/// No timestamp value is reserved: staticness is expressed by
/// [`Stamp::Static`](crate::time::Stamp), not by a sentinel instant, so
/// every value the clock can produce — including `t = 0` on boot-relative
/// clocks — is ordinary dynamic data.
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
///     fn as_seconds_lossy(self) -> f64 {
///         self.0 as f64 / 1_000_000_000.0
///     }
/// }
/// ```
pub trait TimePoint: Copy + Ord + core::fmt::Debug {
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

    /// Returns the timestamp in seconds for diagnostics, accepting precision
    /// loss.
    ///
    /// Error messages are formatted with this method, and it is infallible by
    /// contract: a conversion error can never mask the error actually being
    /// reported. Implement it as a best-effort conversion — for a clock whose
    /// range or epoch makes some values unconvertible, return `f64::NAN` for
    /// those rather than a plausible-looking wrong number.
    fn as_seconds_lossy(self) -> f64;
}
