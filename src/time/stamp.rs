//! The validity stamp carried by every transform: one instant, or all time.

use crate::time::{TimePoint, Timestamp};

/// When a transform is valid: at one instant, or for all time.
///
/// `Stamp` is what [`Transform::timestamp`](crate::Transform::timestamp)
/// returns and what its constructors take:
/// a dynamic sample carries `Stamp::At(t)`, a fixed relationship such as a
/// sensor mount carries `Stamp::Static`. Staticness is a separate variant
/// rather than a reserved timestamp value, so every instant the clock can
/// produce — including `t = 0` on boot-relative clocks — is ordinary
/// dynamic data.
///
/// # Examples
///
/// ```
/// use transforms::time::{Stamp, Timestamp};
///
/// let dynamic: Stamp = Stamp::At(Timestamp::zero());
/// let fixed: Stamp = Stamp::Static;
///
/// assert!(!dynamic.is_static());
/// assert!(fixed.is_static());
/// assert_eq!(dynamic.at(), Some(Timestamp::zero()));
/// assert_eq!(fixed.at(), None);
/// ```
///
/// `Stamp` is deliberately not ordered. `Static` denotes all time rather
/// than an instant, so it has no position on a time axis; a derived ordering
/// would place it below every real instant and make the natural
/// freshest-sample idiom (`max_by_key(|tf| tf.timestamp)`) rank an eternal
/// transform as older than the epoch. Order the instants themselves — via
/// [`Stamp::at`] — and decide what static means for the comparison at hand.
///
/// With the optional `serde` feature, `Stamp` serializes as an explicitly
/// tagged enum: `Stamp::At(t)` as `{"At": t}` and `Stamp::Static` as
/// `"Static"` in JSON, and as a variant index followed by the payload in
/// non-self-describing formats. No timestamp value is reserved, and neither
/// an absent nor a `null` `timestamp` field decodes — a message that lost
/// its stamp is an error, never an eternal static transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stamp<T = Timestamp>
where
    T: TimePoint,
{
    /// Valid for all time: a fixed frame relationship that never expires.
    Static,
    /// Valid at one instant.
    At(T),
}

impl<T> Stamp<T>
where
    T: TimePoint,
{
    /// Returns `true` for `Stamp::Static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        matches!(self, Stamp::Static)
    }

    /// Returns the instant for `Stamp::At`, or `None` for `Stamp::Static`.
    #[must_use]
    pub const fn at(self) -> Option<T> {
        match self {
            Stamp::Static => None,
            Stamp::At(t) => Some(t),
        }
    }
}
