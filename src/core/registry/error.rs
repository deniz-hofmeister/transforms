use alloc::string::String;
use core::fmt;

use thiserror::Error;

use crate::{
    core::buffer::InsertError,
    errors::TransformError,
    time::{TimePoint, Timestamp},
};

/// Errors from fallible [`Registry`](crate::Registry) operations.
///
/// Lookup payloads retain the registry's time type `T`; conversion to seconds
/// occurs only in `Display`. Numeric validation failures use the flat
/// [`NonUnitRotation`](Self::NonUnitRotation) and
/// [`NonFiniteValues`](Self::NonFiniteValues) variants, including on lookup
/// return paths. Other geometry and time errors use
/// [`TransformError`](Self::TransformError).
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RegistryError<T = Timestamp>
where
    T: TimePoint,
{
    /// The transform's rotation is not a unit quaternion within
    /// [`UNIT_NORM_TOLERANCE`](crate::geometry::UNIT_NORM_TOLERANCE),
    /// carrying the offending norm. A transform straight from a constructor
    /// cannot fail this; one derived by `*`, interpolation, inversion or a
    /// lookup was never re-validated, and this is where re-publishing it is
    /// caught.
    #[error("rotation is not a unit quaternion (norm: {0})")]
    NonUnitRotation(f64),

    /// The transform contains non-finite (NaN or infinite) components,
    /// caught on the same boundary as [`NonUnitRotation`](Self::NonUnitRotation)
    /// — and, unlike that one, also reachable from a lookup: finite hops can
    /// compose to an infinite translation, which the inversion of the target
    /// half rejects. A lookup that inverts nothing — the documented
    /// ancestor-ward direction — does not check, and returns that
    /// translation as `Ok`; see
    /// [`Registry::get_transform`](crate::Registry::get_transform).
    #[error("transform contains non-finite values")]
    NonFiniteValues,

    /// The transform's parent and child are the same frame.
    #[error("a frame cannot be its own parent")]
    SelfReferentialFrame,

    /// The child frame already has a different parent, given here.
    /// [`Registry::add_transform`](crate::Registry::add_transform) never
    /// changes an existing pin — a publisher with a stale frame layout
    /// must not silently rewire the tree — so re-parenting takes a
    /// deliberate call:
    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame)
    /// moves the frame under the transform's parent, at the price of the
    /// frame's stored history. To move the frame *and* keep its history,
    /// or to change its static-or-dynamic kind, remove the frame
    /// ([`Registry::remove_frame`](crate::Registry::remove_frame)) and
    /// re-add its history under the new parent.
    ///
    /// Do not resolve this error mechanically: two publishers that
    /// disagree on the frame's parent would then take turns wiping each
    /// other's history. Re-parent when the *decision* to re-parent was
    /// made, not whenever an insert fails.
    ///
    /// (The variant name predates `reparent_frame` and is kept for
    /// compatibility; renaming it would be a breaking change, deferred to
    /// a 3.0.)
    #[error("add_transform cannot change the child frame's parent ({current_parent})")]
    ReparentingNotSupported {
        /// The parent frame pinned by the child frame's first insert.
        current_parent: String,
    },

    /// Inserting the transform would create a cycle in the frame tree.
    #[error("inserting the transform would create a cycle in the frame tree")]
    CycleDetected,

    /// The transform's kind (static or dynamic) does not match the kind the
    /// child frame was fixed to by its first insert — `Stamp::Static` makes
    /// the frame static, `Stamp::At` makes it dynamic — and a child frame is
    /// one or the other, never both. Fires even after the frame has been
    /// drained of every sample: the kind is a property of the frame, not of
    /// what it currently stores.
    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame)
    /// deliberately preserves it — a seed transform of the opposite kind is
    /// rejected with this same variant: the move drops the history either
    /// way, but a frame flipped to static would answer every instant where
    /// a dynamic frame fails loudly once its stream stops.
    /// [`Registry::remove_frame`](crate::Registry::remove_frame) is the only
    /// way to change it — remove the frame, then re-add it with the other
    /// kind.
    #[error("cannot mix static and dynamic transforms for the same child frame")]
    StaticDynamicConflict,

    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame) was
    /// asked to move a frame that has no parent to replace: the frame is a
    /// root, existing only as other frames' parent. Giving a root a parent
    /// is an ordinary first insert —
    /// [`Registry::add_transform`](crate::Registry::add_transform) — not a
    /// re-parent. The one arrangement neither call reaches directly is
    /// reversing an existing edge (making a frame the parent of its own
    /// current parent, which the cycle check rejects from the other side):
    /// first delete the edge —
    /// [`Registry::remove_frame`](crate::Registry::remove_frame) on its
    /// child frame — then attach the old parent under the old child:
    /// [`Registry::add_transform`](crate::Registry::add_transform) if the
    /// old parent was a root,
    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame) if it
    /// hangs mid-tree with a pin of its own. The mid-tree route replaces
    /// the old parent's own edge to *its* parent, splitting the reversed
    /// pair and its descendants off from the tree above; re-attach the
    /// pair's new root afterwards if the split is not wanted.
    /// (Defensively, the variant is also returned for a registered frame
    /// with no pinned parent — a state `Registry` cannot produce.)
    #[error("frame {0} has no parent to replace")]
    NoParentToReplace(String),

    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame) was
    /// asked to "move" a frame to the parent it already has. This is an
    /// error rather than an upsert into the existing history because
    /// re-parenting drops the frame's stored history, and such a move
    /// would drop that history for nothing. Publishing samples on an
    /// existing edge is
    /// [`Registry::add_transform`](crate::Registry::add_transform)'s job.
    #[error("frame {0} already has the requested parent")]
    ParentUnchanged(String),

    /// The requested frame exists nowhere in the transform tree, neither
    /// as a child nor as a parent frame. Usually a typo or a frame that
    /// has not been published yet.
    #[error("frame {0} does not exist in the transform tree")]
    UnknownFrame(String),

    /// Both frames exist, but no chain of transforms connects them: they
    /// live in different trees. This reflects the tree topology at the
    /// time of the lookup, not a transient data gap — gaps are reported as
    /// [`NotFoundAt`](Self::NotFoundAt).
    #[error("no transform chain connects {target_frame} and {source_frame}")]
    Disconnected {
        /// The `target` argument of the failed lookup.
        ///
        /// (Suffixed `_frame` because `source` is reserved by the error
        /// trait's source-chaining convention.)
        target_frame: String,
        /// The `source` argument of the failed lookup.
        source_frame: String,
    },

    /// A sampled edge could not serve the requested time.
    ///
    /// `covered: Some((start, end))` means the request falls outside that
    /// edge's retained range; `None` means it has no samples. Changing the
    /// requested time cannot help an empty edge until new data is inserted.
    ///
    /// A recorded sampling failure takes precedence over `Disconnected`, so
    /// this error does not establish that the endpoints are connected.
    /// Use [`Registry::latest_common_time`](crate::Registry::latest_common_time)
    /// to find their newest commonly covered instant.
    #[error(
        "transform from {source_frame} into {target_frame} at {} not found ({frame} {})",
        .requested.as_seconds_lossy(),
        Coverage(.covered)
    )]
    NotFoundAt {
        /// The `target` argument of the failed lookup — the frame the data
        /// would have been expressed in.
        target_frame: String,
        /// The `source` argument of the failed lookup — the frame the data
        /// would have come from.
        source_frame: String,
        /// The frame whose stored transforms could not serve the requested
        /// time.
        frame: String,
        /// The timestamp the lookup asked for.
        requested: T,
        /// The time range `frame` covers, or `None` when it holds nothing.
        covered: Option<(T, T)>,
    },

    /// No instant is covered by every connecting edge.
    ///
    /// `covered: Some(range)` names an edge whose range begins after the
    /// newest instant the rest of the chain covers. `None` names an empty
    /// edge. Recovery requires inserting data so the ranges overlap, or
    /// changing the topology. Backfill is subject to the retention policy.
    #[error(
        "no instant is covered by every hop between {target_frame} and {source_frame} ({frame} {})",
        Coverage(.covered)
    )]
    NoCommonTime {
        /// The `target` argument of the failed query.
        target_frame: String,
        /// The `source` argument of the failed query.
        source_frame: String,
        /// The frame whose coverage rules out a common instant.
        frame: String,
        /// That frame's covered range, or `None` when it holds nothing.
        covered: Option<(T, T)>,
    },

    /// An operation on the resolved chain failed: composing, inverting or
    /// interpolating the transforms the walk collected. Unlike the variants
    /// above this one does not name a frame — it reports the geometry or
    /// time failure itself.
    #[error("transform error: {0}")]
    TransformError(#[source] TransformError),
}

impl<T> From<TransformError> for RegistryError<T>
where
    T: TimePoint,
{
    /// Canonicalizes the two validation failures into their flat variants,
    /// so a caller matching [`RegistryError::NonUnitRotation`] or
    /// [`RegistryError::NonFiniteValues`] cannot miss a wrapped copy of the
    /// same condition arriving from another code path.
    fn from(error: TransformError) -> Self {
        match error {
            TransformError::NonUnitRotation(norm) => Self::NonUnitRotation(norm),
            TransformError::NonFiniteValues => Self::NonFiniteValues,
            other => Self::TransformError(other),
        }
    }
}

impl<T> From<InsertError> for RegistryError<T>
where
    T: TimePoint,
{
    fn from(error: InsertError) -> Self {
        match error {
            InsertError::Invalid(error) => error.into(),
            InsertError::StaticDynamicConflict => Self::StaticDynamicConflict,
            InsertError::SelfReferentialFrame => Self::SelfReferentialFrame,
            InsertError::ReparentingNotSupported(current_parent) => {
                Self::ReparentingNotSupported { current_parent }
            }
            // Buffers are keyed by child frame, so the buffer an insert
            // reaches always stores that transform's own child and this
            // arm is unreachable through `Registry`. `Buffer` keeps the
            // check — it is the only place that can make the pin true —
            // and its violation is reported as the frame incompatibility it
            // is, rather than mapped onto a registry cause it is not.
            InsertError::ChildFrameMismatch { pinned, found } => {
                Self::TransformError(TransformError::IncompatibleFrames {
                    expected: pinned,
                    found,
                })
            }
        }
    }
}

/// Renders a frame's coverage for [`RegistryError::NotFoundAt`] and
/// [`RegistryError::NoCommonTime`]: the range the frame holds — each
/// variant's own doc says what lying outside it means there — or that it
/// holds nothing at all.
///
/// A separate `Display` rather than a formatted `String` because an error
/// message that can fail to be built — here, on a failing allocation — is
/// worse than the error it reports.
struct Coverage<'a, T>(&'a Option<(T, T)>);

impl<T> fmt::Display for Coverage<'_, T>
where
    T: TimePoint,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self.0 {
            Some((start, end)) => write!(
                f,
                "covers [{}, {}]",
                start.as_seconds_lossy(),
                end.as_seconds_lossy()
            ),
            None => f.write_str("holds no transforms"),
        }
    }
}
