//! When a transform holds: for all time, or at one instant.

use super::Timestamp;

/// When a transform holds: for all time, or at one instant.
///
/// A fixed relationship such as a sensor mount carries `Stamp::Static`; a
/// sample of a moving one carries `Stamp::At(t)`. Staticness is a variant
/// of its own rather than a reserved timestamp, so every instant a clock
/// can produce, zero and negative ones included, is ordinary dynamic data.
///
/// `Stamp` is deliberately not ordered. `Static` denotes all time and has
/// no position on a time axis: any ordering would rank an eternal transform
/// against real instants, and `max_by_key(|tf| tf.stamp())` would silently
/// pick the wrong sample. Match on the stamp, order the [`Timestamp`]s, and
/// decide what static means for the comparison at hand.
///
/// With `serde`, a stamp is an explicitly tagged enum: `"Static"` and
/// `{"At": t}` in JSON, a variant index followed by the payload in formats
/// that are not self-describing. A missing or `null` stamp is a decode
/// error, never a static transform.
///
/// # Examples
///
/// ```
/// use transforms::{Stamp, Timestamp};
///
/// let stamp = Stamp::At(Timestamp::from_nanos(5));
/// let nanos = match stamp {
///     Stamp::At(t) => Some(t.as_nanos()),
///     Stamp::Static => None,
/// };
/// assert_eq!(nanos, Some(5));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stamp {
    /// Valid for all time.
    Static,
    /// Valid at one instant.
    At(Timestamp),
}

#[cfg(test)]
mod tests;
