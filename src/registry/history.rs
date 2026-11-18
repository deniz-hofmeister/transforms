//! What one edge of the frame tree stores.

use crate::{
    geometry::{Isometry, NumericError},
    time::{Stamp, Timestamp},
};
use alloc::collections::BTreeMap;
use core::{fmt, time::Duration};

/// The stored geometry of one edge: a single pose that holds at every
/// instant, or samples of a moving one.
///
/// The kind is structural, decided by the edge's first transform and fixed
/// for as long as the edge exists: an emptied dynamic history is still
/// dynamic, and no stored data can turn one kind into the other.
pub(super) enum History {
    /// One isometry, valid for all time. It sits outside the time path: a
    /// lookup reads it without consulting the requested instant.
    Static(Isometry),
    /// Samples keyed by their instant. The newest stored instant is the
    /// map's last key and is never remembered beside the map, so a drained
    /// history has no newest instant left to expire a restarted stream
    /// against.
    Dynamic(BTreeMap<Timestamp, Isometry>),
}

/// Why an edge cannot serve an instant, as far as its coverage decides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Gap {
    /// The edge holds no samples.
    Empty,
    /// The edge holds samples and the instant lies outside them.
    Outside {
        /// The oldest stored instant.
        earliest: Timestamp,
        /// The newest stored instant.
        latest: Timestamp,
    },
}

/// Why sampling an edge failed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum SampleError {
    /// The edge does not cover the instant.
    Gap(Gap),
    /// The edge covers the instant and interpolating there failed.
    Numeric(NumericError),
}

impl History {
    /// A history holding its first transform, of the kind `stamp` declares.
    pub(super) fn seeded(
        isometry: Isometry,
        stamp: Stamp,
    ) -> Self {
        match stamp {
            Stamp::Static => Self::Static(isometry),
            Stamp::At(at) => Self::Dynamic(BTreeMap::from([(at, isometry)])),
        }
    }

    /// Whether this is a static edge.
    pub(super) fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }

    /// The oldest and newest stored instants of a dynamic history that
    /// holds samples; `None` for an empty one and for a static edge.
    pub(super) fn range(&self) -> Option<(Timestamp, Timestamp)> {
        let Self::Dynamic(samples) = self else {
            return None;
        };
        let (&earliest, _) = samples.first_key_value()?;
        let (&latest, _) = samples.last_key_value()?;
        Some((earliest, latest))
    }

    /// Stores a transform's geometry, replacing whatever the same stamp
    /// held: a static pose is replaced, a dynamic sample is an upsert.
    ///
    /// Under `max_age`, samples older than the newest stored instant minus
    /// `max_age` are then evicted; a sample exactly on that boundary stays.
    /// Where the subtraction is not representable nothing can be older, and
    /// nothing is evicted.
    ///
    /// Returns `false`, storing nothing, if the stamp is of the other kind.
    /// The insert checks refuse that first; it is repeated here rather than
    /// assumed, so a broken check cannot store a pose under the wrong kind.
    pub(super) fn store(
        &mut self,
        isometry: Isometry,
        stamp: Stamp,
        max_age: Option<Duration>,
    ) -> bool {
        match (self, stamp) {
            (Self::Static(pose), Stamp::Static) => {
                *pose = isometry;
                true
            }
            (Self::Dynamic(samples), Stamp::At(at)) => {
                samples.insert(at, isometry);
                let cutoff = max_age
                    .zip(samples.last_key_value())
                    .and_then(|(max_age, (newest, _))| newest.checked_sub(max_age));
                if let Some(cutoff) = cutoff {
                    // Eviction runs on every insert, so it pops from the
                    // front: O(log n + evicted), not a scan.
                    while samples
                        .first_key_value()
                        .is_some_and(|(&oldest, _)| oldest < cutoff)
                    {
                        samples.pop_first();
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Removes every dynamic sample at or before `cutoff`. A static pose
    /// holds for all time and is never removed by a cutoff.
    pub(super) fn remove_through(
        &mut self,
        cutoff: Timestamp,
    ) {
        let Self::Dynamic(samples) = self else {
            return;
        };
        // The first instant that survives. Past the latest representable
        // instant there is none, and everything goes.
        match cutoff.checked_add(Duration::from_nanos(1)) {
            Some(first_kept) => *samples = samples.split_off(&first_kept),
            None => samples.clear(),
        }
    }

    /// Why this edge cannot serve `at`, judged on coverage alone, or `None`
    /// if it can. No geometry is computed.
    pub(super) fn gap(
        &self,
        at: Timestamp,
    ) -> Option<Gap> {
        if self.is_static() {
            return None;
        }
        // A history is a sorted map, so an instant inside its range has a
        // stored neighbour on each side: outside the range is the only way
        // a non-empty history fails to serve an instant.
        match self.range() {
            Some((earliest, latest)) => {
                (at < earliest || at > latest).then_some(Gap::Outside { earliest, latest })
            }
            None => Some(Gap::Empty),
        }
    }

    /// The edge's geometry at `at`: the static pose, a stored sample bit
    /// for bit, or the interpolation between the two samples that bracket
    /// the instant. There is no extrapolation.
    ///
    /// # Errors
    ///
    /// Returns `SampleError::Gap` if the edge does not cover `at`, and
    /// `SampleError::Numeric` if interpolating there failed.
    #[inline]
    pub(super) fn sample(
        &self,
        at: Timestamp,
    ) -> Result<Isometry, SampleError> {
        let samples = match self {
            Self::Static(pose) => return Ok(*pose),
            Self::Dynamic(samples) => samples,
        };

        let before = samples.range(..=at).next_back();
        if let Some((&stored, isometry)) = before {
            if stored == at {
                return Ok(*isometry);
            }
        }
        // `at` is not stored, so this is the first sample after it.
        let after = samples.range(at..).next();

        match (before, after) {
            (Some((&from, start)), Some((&to, end))) => start
                .interpolate(*end, at.ratio_between(from, to))
                .map_err(SampleError::Numeric),
            _ => Err(SampleError::Gap(match self.range() {
                Some((earliest, latest)) => Gap::Outside { earliest, latest },
                None => Gap::Empty,
            })),
        }
    }
}

/// Summarizes the history instead of dumping it: a dynamic edge can hold
/// thousands of samples, and printing each one makes the output useless at
/// exactly the sizes worth debugging.
impl fmt::Debug for History {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Static(pose) => f.debug_tuple("Static").field(pose).finish(),
            Self::Dynamic(samples) => f
                .debug_struct("Dynamic")
                .field("samples", &samples.len())
                .field("covered", &self.range())
                .finish(),
        }
    }
}
