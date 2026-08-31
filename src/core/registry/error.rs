use alloc::string::String;
use core::fmt;

use thiserror::Error;

use crate::{
    core::buffer::InsertError,
    errors::TransformError,
    time::{TimePoint, Timestamp},
};

/// Error type for every [`Registry`](crate::Registry) call: insertion,
/// lookup, and the [`latest_common_time`](crate::Registry::latest_common_time)
/// coverage query.
///
/// One flat enum for both: every cause a registry call can report is a
/// variant of this type, so a caller diagnoses a failure with a single
/// `match` instead of unwrapping nested error types. The lookup payloads
/// carry timestamps in the registry's own time type `T` rather than
/// pre-formatted seconds, so a caller can compare them against the clock it
/// asked with.
///
/// The first six variants are reported by
/// [`add_transform`](crate::Registry::add_transform);
/// [`reparent_frame`](crate::Registry::reparent_frame) shares five of
/// them — all but [`ReparentingNotSupported`](Self::ReparentingNotSupported),
/// since the seed transform crosses the same checks an insert does — and adds
/// [`NoParentToReplace`](Self::NoParentToReplace) and
/// [`ParentUnchanged`](Self::ParentUnchanged), plus
/// [`UnknownFrame`](Self::UnknownFrame) for a frame that exists nowhere.
/// The three variants after those are reported by
/// the lookups, and [`NoCommonTime`](Self::NoCommonTime) by
/// [`latest_common_time`](crate::Registry::latest_common_time) — which
/// shares [`UnknownFrame`](Self::UnknownFrame) and
/// [`Disconnected`](Self::Disconnected) with the lookups, so the same
/// match arms diagnose both. One crossover: where a lookup inverts a half-chain that
/// composed to an infinite translation, it reports the same flat
/// [`NonFiniteValues`](Self::NonFiniteValues) an insert would — one spelling
/// per condition, on every path that reports it at all.
///
/// [`TransformError`](Self::TransformError) is the geometry or time failure
/// of an operation on the resolved chain and is the one arm that wraps
/// another error type. It never carries
/// `TransformError::NonUnitRotation` or `TransformError::NonFiniteValues` —
/// those are canonically the flat [`NonUnitRotation`](Self::NonUnitRotation)
/// and [`NonFiniteValues`](Self::NonFiniteValues) variants, so neither
/// condition has two spellings to match on.
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
    /// compatibility; renaming it is logged for 3.0.)
    #[error("add_transform cannot change the child frame's parent ({current_parent}); re-parenting takes reparent_frame")]
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
    /// rejected with this same variant, because a move must not quietly
    /// turn a time series into an eternal pose (or the reverse).
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
    /// re-parent. The one arrangement neither call reaches is reversing an
    /// existing edge (making a frame the parent of its own current parent,
    /// which the cycle check rejects from the other side): rebuild that
    /// edge explicitly —
    /// [`Registry::remove_frame`](crate::Registry::remove_frame) on its
    /// child frame, then re-add both frames in the new arrangement.
    #[error("frame {0} has no parent to replace")]
    NoParentToReplace(String),

    /// [`Registry::reparent_frame`](crate::Registry::reparent_frame) was
    /// asked to "move" a frame to the parent it already has. This is an
    /// error rather than an upsert into the existing history because
    /// re-parenting drops the frame's stored history: a caller resolving
    /// every failed insert into a re-parent would wipe the buffer once and
    /// look correct forever after. Publishing samples on an existing edge
    /// is [`Registry::add_transform`](crate::Registry::add_transform)'s
    /// job.
    #[error("frame {0} already has this parent")]
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

    /// The lookup stopped at a frame that exists in the tree but could not
    /// serve the requested time. `frame` names where the chain walk stopped
    /// and `covered` says which of two cases it is: `Some(range)` when the
    /// request falls outside data the frame does hold — typically a
    /// transient gap, and `requested > end` means merely too new — or
    /// `None` when the frame holds no data at all. Only the first case is a
    /// timing question. A frame drained by
    /// [`Registry::remove_transforms_before`](crate::Registry::remove_transforms_before)
    /// keeps its entry and reports `None` for as long as nothing is
    /// inserted into it, so waiting or widening the requested time window
    /// will not make it answer.
    ///
    /// Receiving this variant does not guarantee the frames are connectable:
    /// when a data gap and a topological disconnection coexist, the recorded
    /// walk failure takes precedence over the [`Disconnected`](Self::Disconnected)
    /// diagnosis.
    ///
    /// The `Some` case's `requested > end` reading — merely too new —
    /// raises the question "what is the newest instant this chain *can*
    /// serve?". That is a first-class query:
    /// [`Registry::latest_common_time`](crate::Registry::latest_common_time)
    /// answers it exactly, also for mid-tree targets, without retrying
    /// lookups against this variant's payloads.
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

    /// [`Registry::latest_common_time`](crate::Registry::latest_common_time)
    /// found no instant that every hop of the resolved chain can serve —
    /// so a lookup between these frames would fail at *any* requested
    /// time. `frame` names the hop that rules it out and `covered` says
    /// how: `Some(range)` when that frame's stored range begins after the
    /// newest instant the rest of the chain still covers — the chain's
    /// covered ranges are disjoint, typically one hop lagging far behind
    /// another under `max_age` eviction — or `None` when the frame holds
    /// no data at all, the state
    /// [`Registry::remove_transforms_before`](crate::Registry::remove_transforms_before)
    /// leaves a drained frame in. Either way, only inserts make the
    /// chain answer again: data on the named frame for the `None` case;
    /// for the `Some` case, ranges that come to overlap — the named
    /// frame's range reaching back via a backfill at a past instant, or
    /// every hop still ending before that range catching up to it.
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
