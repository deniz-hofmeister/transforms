//! Instants: signed nanoseconds since a caller-chosen epoch.

use core::{fmt, time::Duration};

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// An instant: signed nanoseconds since an epoch the caller chooses.
///
/// The type does not imply Unix time. A boot-relative clock, a simulation
/// clock and a wall clock all fit, as long as one registry sees one of
/// them. The range is about 292 years either side of the epoch at 1 ns
/// resolution, and every instant in it is ordinary: zero and negative
/// values are not reserved.
///
/// `Display` prints sign-magnitude decimal seconds with nine fractional
/// digits, `-0.500000000` for half a second before the epoch. That is not
/// the floored `(sec, nanosec)` pair of the conversion below, which writes
/// the same instant as `(-1, 500_000_000)`. Width, fill and precision flags
/// are ignored.
///
/// With `serde`, a timestamp encodes as the bare nanosecond count.
///
/// # Arithmetic
///
/// Use [`checked_add`](Self::checked_add),
/// [`checked_sub`](Self::checked_sub) and
/// [`checked_duration_since`](Self::checked_duration_since) rather than
/// arithmetic on [`as_nanos`](Self::as_nanos). The hand-written forms give
/// wrong answers at ordinary timestamps, without an overflow:
///
/// - `(now.as_nanos() - stamp.as_nanos()) as u64` reads a stamp 2 ms ahead
///   of the clock as an age of 584 years;
/// - `now.as_nanos().abs_diff(stamp.as_nanos())` reads a stamp 3 s in the
///   future as an age of 3 s;
/// - `now.as_nanos() + timeout.as_nanos() as i64` with `Duration::MAX` is a
///   deadline 1 ns in the past.
///
/// # Converting a `(seconds, nanoseconds)` stamp
///
/// Middleware stamps such as ROS 2's
/// `builtin_interfaces/Time { sec: i32, nanosec: u32 }` convert at the
/// boundary. The split back must floor: `/` and `%` truncate toward zero,
/// so the naive split of -0.5 s followed by a cast gives
/// `(0, 3794967296)`.
///
/// ```
/// use transforms::Timestamp;
///
/// const NANOS_PER_SEC: i64 = 1_000_000_000;
///
/// /// `builtin_interfaces/Time { sec: i32, nanosec: u32 }` to a `Timestamp`.
/// fn from_ros(
///     sec: i32,
///     nanosec: u32,
/// ) -> Option<Timestamp> {
///     let nanosec = i64::from(nanosec);
///     if nanosec >= NANOS_PER_SEC {
///         return None; // denormalized stamp
///     }
///     // Cannot overflow: |sec| * 1e9 + nanosec < 2.2e18 < i64::MAX.
///     Some(Timestamp::from_nanos(
///         i64::from(sec) * NANOS_PER_SEC + nanosec,
///     ))
/// }
///
/// /// A `Timestamp` to `(sec, nanosec)`. Euclidean division floors, so the
/// /// nanosecond part is always in `0..1e9`: -1 ns is `(-1, 999_999_999)`.
/// fn to_ros(t: Timestamp) -> Option<(i32, u32)> {
///     let nanos = t.as_nanos();
///     let sec = i32::try_from(nanos.div_euclid(NANOS_PER_SEC)).ok()?;
///     let nanosec = u32::try_from(nanos.rem_euclid(NANOS_PER_SEC)).ok()?;
///     Some((sec, nanosec))
/// }
///
/// assert_eq!(to_ros(Timestamp::from_nanos(-1)), Some((-1, 999_999_999)));
/// assert_eq!(
///     from_ros(-1, 500_000_000),
///     Some(Timestamp::from_nanos(-500_000_000))
/// );
/// assert_eq!(
///     to_ros(Timestamp::from_nanos(-500_000_000)),
///     Some((-1, 500_000_000))
/// );
/// assert_eq!(from_ros(0, 1_000_000_000), None);
/// assert_eq!(to_ros(Timestamp::MAX), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Timestamp(i64);

impl Timestamp {
    /// The latest representable instant.
    ///
    /// It exists as a cutoff:
    /// [`Registry::remove_samples_through`](crate::Registry::remove_samples_through)
    /// with `MAX` clears all dynamic history. It is not meant as a stamp or
    /// as a lookup instant, although nothing reserves it. A dynamic sample
    /// inserted at `MAX` becomes its frame's newest, and under
    /// [`Registry::with_max_age`](crate::Registry::with_max_age) the
    /// frame's later samples older than `MAX - max_age` are then rejected
    /// until that history is removed.
    pub const MAX: Self = Self(i64::MAX);

    /// Creates an instant from nanoseconds since the epoch.
    #[must_use]
    pub const fn from_nanos(nanos: i64) -> Self {
        Self(nanos)
    }

    /// Nanoseconds since the epoch.
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.0
    }

    /// The instant `duration` after this one, or `None` if it is not
    /// representable.
    ///
    /// Exact over the whole signed range and for every `Duration`,
    /// including one longer than `i64::MAX` nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::Timestamp;
    ///
    /// let now = Timestamp::from_nanos(1_700_000_000_123_456_789);
    /// assert_eq!(
    ///     now.checked_add(Duration::from_millis(5)),
    ///     Some(Timestamp::from_nanos(1_700_000_000_128_456_789))
    /// );
    /// // "No timeout" is not a deadline in the past.
    /// assert_eq!(now.checked_add(Duration::MAX), None);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn checked_add(
        self,
        duration: Duration,
    ) -> Option<Self> {
        // A `Duration` holds fewer than 2^95 nanoseconds, so the sum of
        // the two widened operands cannot overflow an `i128`.
        let duration = i128::try_from(duration.as_nanos()).ok()?;
        let nanos = i128::from(self.0).checked_add(duration)?;
        i64::try_from(nanos).ok().map(Self)
    }

    /// The instant `duration` before this one, or `None` if it is not
    /// representable.
    ///
    /// Exact over the whole signed range and for every `Duration`,
    /// including one longer than `i64::MAX` nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::Timestamp;
    ///
    /// let t = Timestamp::from_nanos(1_000);
    /// assert_eq!(
    ///     t.checked_sub(Duration::from_nanos(1_500)),
    ///     Some(Timestamp::from_nanos(-500))
    /// );
    /// assert_eq!(t.checked_sub(Duration::MAX), None);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn checked_sub(
        self,
        duration: Duration,
    ) -> Option<Self> {
        let duration = i128::try_from(duration.as_nanos()).ok()?;
        let nanos = i128::from(self.0).checked_sub(duration)?;
        i64::try_from(nanos).ok().map(Self)
    }

    /// The time elapsed from `earlier` to this instant, or `None` if
    /// `earlier` is later than this instant.
    ///
    /// The direction is part of the answer: a stamp ahead of the clock has
    /// no age, and this returns `None` for it rather than a magnitude.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::Timestamp;
    ///
    /// let stamp = Timestamp::from_nanos(-250);
    /// let now = Timestamp::from_nanos(750);
    /// assert_eq!(
    ///     now.checked_duration_since(stamp),
    ///     Some(Duration::from_nanos(1_000))
    /// );
    /// assert_eq!(stamp.checked_duration_since(now), None);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn checked_duration_since(
        self,
        earlier: Self,
    ) -> Option<Duration> {
        (earlier <= self).then(|| Duration::from_nanos(self.0.abs_diff(earlier.0)))
    }

    /// How far this instant lies between `from` and `to`, as the fraction
    /// an interpolation blends by. The caller has checked
    /// `from < self < to`.
    ///
    /// Both spans are integer nanosecond differences. Below 2^53 ns (about
    /// 104 days) each converts to `f64` exactly, and the division is then
    /// correctly rounded.
    pub(crate) fn ratio_between(
        self,
        from: Self,
        to: Self,
    ) -> f64 {
        nanos_to_f64(self.0.abs_diff(from.0)) / nanos_to_f64(to.0.abs_diff(from.0))
    }
}

/// `nanos` as the nearest `f64`.
///
/// `u64` has no lossless conversion into `f64`, so the count is split:
/// both 32-bit halves convert exactly, scaling the high half by 2^32 is
/// exact, and the one addition rounds once, to the value nearest the
/// count.
fn nanos_to_f64(nanos: u64) -> f64 {
    const TWO_POW_32: f64 = 4_294_967_296.0;
    let [b0, b1, b2, b3, b4, b5, b6, b7] = nanos.to_be_bytes();
    let high = u32::from_be_bytes([b0, b1, b2, b3]);
    let low = u32::from_be_bytes([b4, b5, b6, b7]);
    f64::from(high) * TWO_POW_32 + f64::from(low)
}

impl fmt::Display for Timestamp {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // Computed on the magnitude, so `i64::MIN` needs no negation.
        let magnitude = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(
            f,
            "{sign}{}.{:09}",
            magnitude / NANOS_PER_SEC,
            magnitude % NANOS_PER_SEC
        )
    }
}

#[cfg(test)]
mod tests;
