//! A module for managing a buffer of transforms with timestamps.
//!
//! This module provides the `Buffer` struct, which is designed to store and manage
//! a collection of transforms, each associated with a timestamp. The buffer uses
//! an ordered map (B-tree) to efficiently store and retrieve transforms based on their timestamps.
//!
//! `Buffer` is internal to the crate: [`Registry`](crate::Registry) owns one
//! per child frame and is the only way to reach it. The invariants that make a
//! frame tree well-formed are split between the two, and most of them live
//! here: [`Buffer::insert`] is the sole enforcement site for the single-parent
//! pin, the child pin, the static-xor-dynamic kind, and the numeric validity
//! of what is stored. `Registry` adds only the check that needs a view of the
//! whole tree — the cycle check — and runs it solely for a child frame it has
//! not seen before, precisely because this module's pin makes an existing
//! buffer's parent immutable. Any rework of the storage below must keep those
//! pins: without them a re-parenting insert reaches no check at all, and every
//! later lookup through the frame returns a pose expressed relative to the
//! wrong parent.
//!
//! The numeric check is deliberately *not* redundant with the one the
//! constructors run. A [`Transform`] is validated where it is built, but `*`,
//! [`Transform::interpolate`], [`Transform::inverse`] and every registry
//! lookup derive transforms without re-validating them — by design, because
//! rotation norms drift across a long chain. A caller who flattens a chain
//! and re-publishes the result therefore hands storage a value nothing has
//! checked, and a rotation that has left
//! [`UNIT_NORM_TOLERANCE`](crate::geometry::UNIT_NORM_TOLERANCE) silently
//! scales every vector every later lookup rotates. This is the last boundary
//! before a transform starts answering lookups, and the check is O(1) per
//! insert.
//!
//! # Features
//!
//! - **Store Transforms with Timestamps**: The `Buffer` allows you to store multiple transforms,
//!   each associated with a unique timestamp. This is useful for applications that require
//!   time-based transformations, such as robotics, animation, and simulations.
//!
//! - **Retrieve Transforms with Interpolation**: You can retrieve transforms at specific timestamps.
//!   If an exact match is not found, the buffer can interpolate between the nearest transforms to
//!   provide an estimated transform at the requested timestamp.
//!
//! - **Static Buffers**: A buffer is either static or dynamic — a property declared at
//!   construction ([`Buffer::static_edge`] vs. [`Buffer::dynamic`]) and fixed for the buffer's
//!   lifetime. A static buffer holds one transform carrying `Stamp::Static` and returns it for
//!   any requested timestamp; a dynamic buffer holds a time series of `Stamp::At` samples.
//!   Inserting the opposite kind is rejected with `InsertError::StaticDynamicConflict`.
//!
//! - **Automatic Expiration of Transforms**:
//!   - Buffers created with `Buffer::dynamic_with_max_age` remove entries older than `max_age`
//!     relative to the latest inserted timestamp on every insert.
//!   - This ensures that the buffer does not grow indefinitely and only retains relevant
//!     transforms within the specified duration.
//!   - Buffers created with `Buffer::dynamic` never expire entries; use the `remove_before`
//!     method for manual cleanup. Static transforms never expire and survive manual
//!     cleanup.

use crate::{
    geometry::Transform,
    time::{Stamp, TimePoint, Timestamp},
};
use alloc::{collections::BTreeMap, string::String};
use core::{fmt, time::Duration};
pub(crate) use error::{GetError, InsertError};
mod error;

type NearestTransforms<'a, T> = (
    Option<(&'a T, &'a Transform<T>)>,
    Option<(&'a T, &'a Transform<T>)>,
);

/// A buffer that stores transforms ordered by timestamps.
///
/// The `Buffer` struct is designed to manage a collection of transforms,
/// each associated with a timestamp. It uses an ordered map (B-tree) to efficiently
/// store and retrieve transforms based on their timestamps.
///
/// A buffer is either static or dynamic, declared at construction and fixed
/// for the buffer's lifetime: [`Buffer::static_edge`] builds a buffer that
/// holds one transform carrying `Stamp::Static` and serves it for any
/// requested time; [`Buffer::dynamic`] and [`Buffer::dynamic_with_max_age`]
/// build buffers that hold a time series of `Stamp::At` samples. Inserts of
/// the opposite kind are rejected with `InsertError::StaticDynamicConflict`.
///
/// The first insert pins the buffer's parent and child frames: every later
/// insert must carry the same pair, so a buffer stores the history of
/// exactly one parent-child relationship. Re-parenting is rejected with
/// `InsertError::ReparentingNotSupported`, and a transform for a different
/// child frame with `InsertError::ChildFrameMismatch`.
///
/// When constructed with [`Buffer::dynamic_with_max_age`], entries older
/// than `max_age` relative to the latest inserted timestamp are removed
/// automatically on insert. A buffer created with [`Buffer::dynamic`] never
/// expires entries; use [`Buffer::remove_before`] for manual cleanup.
pub(crate) struct Buffer<T = Timestamp>
where
    T: TimePoint,
{
    parent: Option<String>,
    child: Option<String>,
    kind: Kind<T>,
}

/// Summarizes the buffer instead of dumping it: frames, kind, sample count
/// and covered range. A dynamic frame can hold thousands of samples, and a
/// derived `Debug` — reachable through `Registry`'s — printed every one of
/// them, which made the output useless at exactly the buffer sizes worth
/// debugging.
impl<T> fmt::Debug for Buffer<T>
where
    T: TimePoint,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let mut summary = f.debug_struct("Buffer");
        summary
            .field("parent", &self.parent)
            .field("child", &self.child);
        match &self.kind {
            // A static buffer holds at most one transform, and that
            // transform — a mount's pose — is what gets debugged: print it.
            Kind::Static(slot) => summary.field("kind", &"static").field("transform", slot),
            Kind::Dynamic {
                data,
                latest_timestamp,
                max_age,
            } => summary
                .field("kind", &"dynamic")
                .field("samples", &data.len())
                .field(
                    "covered",
                    &data
                        .first_key_value()
                        .zip(data.last_key_value())
                        .map(|((start, _), (end, _))| (start, end)),
                )
                .field("latest", latest_timestamp)
                .field("max_age", max_age),
        };
        summary.finish()
    }
}

/// The span of instants a buffer can serve, for chain-level reasoning such
/// as [`Registry::latest_common_time`](crate::Registry::latest_common_time).
pub(crate) enum Coverage<T>
where
    T: TimePoint,
{
    /// A static buffer holding its transform: every instant.
    AllTime,
    /// A dynamic buffer's stored span, both endpoints inclusive.
    Range {
        /// The oldest stored instant.
        start: T,
        /// The newest stored instant.
        end: T,
    },
    /// No stored transforms: no instant can be served.
    Empty,
}

/// The buffer's storage, decided at construction: one static transform, or
/// a time series of dynamic samples. Keeping the kind structural — instead
/// of a flag re-derived from the stored data — makes it impossible for a
/// buffer to change kind when it is emptied and refilled.
enum Kind<T>
where
    T: TimePoint,
{
    /// One transform valid for all time; `None` until the first insert.
    Static(Option<Transform<T>>),
    /// A time series of samples, keyed by their instant.
    Dynamic {
        data: BTreeMap<T, Transform<T>>,
        latest_timestamp: Option<T>,
        max_age: Option<Duration>,
    },
}

impl<T> Buffer<T>
where
    T: TimePoint,
{
    /// Creates a new dynamic `Buffer` without automatic expiry.
    ///
    /// Entries are kept until removed manually with
    /// [`Buffer::remove_before`].
    #[must_use]
    pub fn dynamic() -> Self {
        Self {
            parent: None,
            child: None,
            kind: Kind::Dynamic {
                data: BTreeMap::new(),
                latest_timestamp: None,
                max_age: None,
            },
        }
    }

    /// Creates a new dynamic `Buffer` with automatic expiry after `max_age`.
    ///
    /// Entries older than `max_age` relative to the latest inserted timestamp
    /// are removed automatically whenever a transform is inserted.
    /// `Duration::ZERO` therefore retains only the newest sample.
    #[must_use]
    pub fn dynamic_with_max_age(max_age: Duration) -> Self {
        Self {
            parent: None,
            child: None,
            kind: Kind::Dynamic {
                data: BTreeMap::new(),
                latest_timestamp: None,
                max_age: Some(max_age),
            },
        }
    }

    /// Creates a new static `Buffer`: one transform, valid for all time.
    ///
    /// The buffer accepts only transforms carrying `Stamp::Static`; a later
    /// static insert replaces the stored transform. Static buffers never
    /// expire and survive [`Buffer::remove_before`].
    #[must_use]
    pub fn static_edge() -> Self {
        Self {
            parent: None,
            child: None,
            kind: Kind::Static(None),
        }
    }

    /// Returns the buffer's parent frame, pinned by the first insert.
    ///
    /// `None` for a buffer that has never held a transform. The parent stays
    /// pinned even if all entries are removed; drop the whole buffer
    /// (`Registry::remove_frame`) to release it.
    #[must_use]
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    /// The span of instants this buffer can serve: every instant for a
    /// static buffer holding its transform, the stored range (inclusive)
    /// for a dynamic buffer, and [`Coverage::Empty`] for a buffer holding
    /// nothing — a drained dynamic frame, or a static buffer before its
    /// first insert (unreachable through `Registry`, which registers a
    /// buffer only after its first insert succeeds).
    #[must_use]
    pub fn coverage(&self) -> Coverage<T> {
        match &self.kind {
            Kind::Static(Some(_)) => Coverage::AllTime,
            Kind::Static(None) => Coverage::Empty,
            Kind::Dynamic { data, .. } => match (data.first_key_value(), data.last_key_value()) {
                (Some((&start, _)), Some((&end, _))) => Coverage::Range { start, end },
                _ => Coverage::Empty,
            },
        }
    }

    /// Adds a transform to the buffer.
    ///
    /// The transform is validated first: it must have finite components and a
    /// unit rotation (see [`Transform::validate`]). The constructors ran that
    /// check already, but a transform derived from valid ones — composed,
    /// interpolated, inverted, or read back out of a lookup — was not
    /// re-checked on the way here, so this is where such a value is caught.
    /// Its stamp must match the buffer's kind, declared at construction: a
    /// static buffer accepts only `Stamp::Static`, a dynamic buffer only
    /// `Stamp::At`.
    ///
    /// # Errors
    ///
    /// Returns `InsertError::Invalid` wrapping
    /// `TransformError::NonUnitRotation` or `TransformError::NonFiniteValues`
    /// if the transform fails validation — storing such a transform would
    /// make later lookups return silently wrong results.
    ///
    /// Returns `InsertError::StaticDynamicConflict` if the transform's kind
    /// (static or dynamic) does not match the buffer's declared kind. Mixing
    /// the two would silently corrupt interpolation.
    ///
    /// Returns `InsertError::SelfReferentialFrame` if the transform's parent
    /// and child are the same frame,
    /// `InsertError::ReparentingNotSupported` if the buffer's parent frame
    /// (pinned by the first insert) differs from the transform's parent, and
    /// `InsertError::ChildFrameMismatch` if the buffer's child frame (pinned
    /// the same way) differs from the transform's child — accepting a second
    /// child frame would silently overwrite a static transform or corrupt
    /// interpolation between dynamic ones.
    ///
    /// Inserting at a timestamp that is already stored replaces the stored
    /// transform, as does inserting into a static buffer that already holds
    /// one: last write wins. Re-publishing a sample is an upsert, not an
    /// error.
    pub fn insert(
        &mut self,
        transform: Transform<T>,
    ) -> Result<(), InsertError> {
        transform.validate().map_err(InsertError::Invalid)?;

        if transform.parent() == transform.child() {
            return Err(InsertError::SelfReferentialFrame);
        }
        if let Some(parent) = &self.parent {
            if parent != transform.parent() {
                return Err(InsertError::ReparentingNotSupported(parent.clone()));
            }
        }
        if let Some(child) = &self.child {
            if child != transform.child() {
                return Err(InsertError::ChildFrameMismatch {
                    pinned: child.clone(),
                    found: transform.child().into(),
                });
            }
        }

        // Captured before the transform is moved into storage; applied only
        // after the insert is accepted, so a rejected transform cannot pin
        // frames for a buffer that never stored it.
        let pin = self
            .parent
            .is_none()
            .then(|| (transform.parent().into(), transform.child().into()));

        match (&mut self.kind, transform.timestamp()) {
            (Kind::Static(slot), Stamp::Static) => {
                *slot = Some(transform);
            }
            (
                Kind::Dynamic {
                    data,
                    latest_timestamp,
                    max_age,
                },
                Stamp::At(timestamp),
            ) => {
                *latest_timestamp = Some(match *latest_timestamp {
                    Some(current_latest) if current_latest > timestamp => current_latest,
                    _ => timestamp,
                });
                data.insert(timestamp, transform);
                remove_expired(data, *latest_timestamp, *max_age);
            }
            _ => return Err(InsertError::StaticDynamicConflict),
        }

        if let Some((parent, child)) = pin {
            self.parent = Some(parent);
            self.child = Some(child);
        }

        Ok(())
    }

    /// Retrieves a transform from the buffer at the specified timestamp.
    ///
    /// # Errors
    ///
    /// Returns `GetError::NoTransformAvailable` if the buffer holds no
    /// transforms at all.
    ///
    /// Returns `GetError::OutOfRange`, carrying both endpoints of the
    /// covered range in the buffer's own timestamp type, if the buffer holds
    /// transforms but the requested timestamp lies outside their range.
    /// There is no extrapolation; a timestamp between two stored samples
    /// always has neighbors to interpolate between, so an out-of-range
    /// request is the only way a lookup on a non-empty dynamic buffer can
    /// fail to find data. Static buffers serve any requested timestamp.
    ///
    /// Returns `GetError::Interpolation` if interpolating between the two
    /// neighboring samples fails. With both frames pinned at insertion and
    /// every stored sample keyed by its own `Stamp::At` instant, this is
    /// only reachable through timestamp arithmetic: a span between the
    /// neighboring samples too large to represent as a `Duration`
    /// (`TimeError::DurationOverflow`).
    pub fn get(
        &self,
        timestamp: T,
    ) -> Result<Transform<T>, GetError<T>> {
        let data = match &self.kind {
            // A static transform is valid for all time: the requested
            // timestamp is deliberately ignored.
            Kind::Static(Some(transform)) => return Ok(transform.clone()),
            Kind::Static(None) => return Err(GetError::NoTransformAvailable),
            Kind::Dynamic { data, .. } => data,
        };

        let (before, after) = self.get_nearest(&timestamp);

        match (before, after) {
            (Some(before), Some(after)) => Transform::interpolate(before.1, after.1, timestamp)
                .map_err(GetError::Interpolation),
            _ => match (data.first_key_value(), data.last_key_value()) {
                (Some((first, _)), Some((last, _))) => Err(GetError::OutOfRange {
                    start: *first,
                    end: *last,
                }),
                _ => Err(GetError::NoTransformAvailable),
            },
        }
    }

    /// Retrieves the nearest transforms before and after the given timestamp.
    ///
    /// Returns a tuple containing the nearest transform before and the
    /// nearest transform after the specified timestamp. If the exact
    /// timestamp exists, both elements of the tuple will be the same. A
    /// static buffer stores no time series, so both elements are `None`.
    fn get_nearest(
        &self,
        timestamp: &T,
    ) -> NearestTransforms<'_, T> {
        let Kind::Dynamic { data, .. } = &self.kind else {
            return (None, None);
        };

        let before = data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = data.range(timestamp..).next();
        (before, after)
    }

    /// Removes dynamic transforms older than the given timestamp.
    ///
    /// This function removes all transforms from the buffer that have a
    /// timestamp lower than the given timestamp. Static buffers are left
    /// untouched: a static transform is valid for all time, so cleaning it up
    /// by timestamp would silently destroy it.
    pub fn remove_before(
        &mut self,
        timestamp: T,
    ) {
        if let Kind::Dynamic {
            data,
            latest_timestamp,
            ..
        } = &mut self.kind
        {
            // Everything at or after the cutoff survives. split_off finds
            // the boundary in O(log n); freeing the evicted prefix is
            // O(evicted) on top — the same O(log n + evicted) as expiry,
            // each entry freed exactly once.
            let kept = data.split_off(&timestamp);
            *data = kept;
            // The expiry reference must not outlive the samples it was
            // derived from: a stale value would make `max_age` eviction
            // measure a restarted stream against the wiped one, silently
            // evicting every new sample on the insert that added it.
            *latest_timestamp = data.last_key_value().map(|(&k, _)| k);
        }
    }
}

/// Removes expired transforms based on `max_age`: everything older than
/// `(latest inserted timestamp - max_age)`. Buffers without a configured
/// `max_age` never expire entries.
///
/// Runs on every dynamic insert, so it evicts in order from the front of
/// the map — O(log n + evicted) — instead of scanning the whole buffer.
///
/// When `latest - max_age` underflows the timestamp type, no sample can be
/// older than the threshold, so skipping the sweep entirely is the correct
/// behavior — the `checked_sub` failure is deliberately not an error.
fn remove_expired<T>(
    data: &mut BTreeMap<T, Transform<T>>,
    latest_timestamp: Option<T>,
    max_age: Option<Duration>,
) where
    T: TimePoint,
{
    if let (Some(max_age), Some(latest_timestamp)) = (max_age, latest_timestamp) {
        if let Ok(threshold) = latest_timestamp.checked_sub(max_age) {
            while let Some((&oldest, _)) = data.first_key_value() {
                if oldest >= threshold {
                    break;
                }
                data.pop_first();
            }
        }
    }
}

#[cfg(test)]
mod tests;
