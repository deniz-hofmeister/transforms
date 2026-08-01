//! The validity stamp carried by every transform: one instant, or all time.

use crate::time::{TimePoint, Timestamp};

/// When a transform is valid: at one instant, or for all time.
///
/// `Stamp` is the type of [`Transform::timestamp`](crate::Transform):
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
/// With the optional `serde` feature, `Stamp` serializes as an optional
/// timestamp: `Stamp::At(t)` as `t` itself and `Stamp::Static` as `null`
/// (in self-describing formats), keeping the wire format free of reserved
/// magic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Stamp<T>
where
    T: TimePoint + serde::Serialize,
{
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Stamp::Static => serializer.serialize_none(),
            Stamp::At(t) => serializer.serialize_some(t),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Stamp<T>
where
    T: TimePoint + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|instant| match instant {
            Some(t) => Stamp::At(t),
            None => Stamp::Static,
        })
    }
}
