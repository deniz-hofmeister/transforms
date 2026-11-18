//! The result of a lookup between two instants.

use super::{Isometry, Rotation, Vector3};
use crate::time::Timestamp;
use alloc::string::String;

/// Maps coordinates in the source frame at one instant into the target
/// frame at another.
///
/// Returned by [`Registry::lookup_across`](crate::Registry::lookup_across)
/// and built nowhere else. It is deliberately not a
/// [`Transform`](crate::Transform): a transform holds at one stamp, and
/// this value holds between two. It cannot be composed, inserted into a
/// registry or serialized, each of which would lose one of the instants.
///
/// The fixed frame the lookup went through is not recorded. It is an
/// assumption of the query, which the result cannot verify.
///
/// Applying it to a point is the recipe shown on
/// [`Transform`](crate::Transform), with a point measured in
/// [`source`](Self::source) at [`source_time`](Self::source_time):
/// the result is in [`target`](Self::target) at
/// [`target_time`](Self::target_time).
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTimeTransform {
    target: String,
    target_time: Timestamp,
    source: String,
    source_time: Timestamp,
    isometry: Isometry,
}

impl CrossTimeTransform {
    /// Labels a resolved isometry with the frames and instants of the query
    /// that produced it.
    pub(crate) fn new(
        target: String,
        target_time: Timestamp,
        source: String,
        source_time: Timestamp,
        isometry: Isometry,
    ) -> Self {
        Self {
            target,
            target_time,
            source,
            source_time,
            isometry,
        }
    }

    /// The frame the result maps coordinates into.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The instant at which the target frame is taken.
    #[must_use]
    pub fn target_time(&self) -> Timestamp {
        self.target_time
    }

    /// The frame the result maps coordinates from.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The instant at which the source frame is taken.
    #[must_use]
    pub fn source_time(&self) -> Timestamp {
        self.source_time
    }

    /// Where the source frame's origin at the source time sits in the
    /// target frame at the target time.
    #[must_use]
    pub fn translation(&self) -> Vector3 {
        self.isometry.translation()
    }

    /// How the source frame at the source time is oriented in the target
    /// frame at the target time.
    #[must_use]
    pub fn rotation(&self) -> Rotation {
        self.isometry.rotation()
    }
}

#[cfg(test)]
mod tests;
