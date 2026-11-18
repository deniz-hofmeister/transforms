//! Errors of changing a registry and of querying it.

use crate::{
    geometry::NumericError,
    time::{Stamp, Timestamp},
};
use alloc::string::String;
use thiserror::Error;

/// Why a transform was not stored.
///
/// Returned by [`Registry::insert`](crate::Registry::insert) and
/// [`Registry::reparent`](crate::Registry::reparent), and inside a
/// [`BatchError`] by [`Registry::insert_all`](crate::Registry::insert_all).
/// Every `frame` field is the child frame of the rejected transform, its
/// [`source`](crate::Transform::source). Each method documents the subset
/// it can return.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum InsertError {
    /// The transform names one frame as both parent and child.
    #[error("frame {frame} cannot be its own parent")]
    #[non_exhaustive]
    SelfEdge {
        /// The frame named twice.
        frame: String,
    },
    /// The child frame already has a different parent.
    ///
    /// A frame's first insert pins its parent.
    /// [`Registry::reparent`](crate::Registry::reparent) moves a frame
    /// deliberately.
    #[error("frame {frame} has parent {pinned}, not {offered}")]
    #[non_exhaustive]
    ParentMismatch {
        /// The child frame.
        frame: String,
        /// The parent the frame is pinned to.
        pinned: String,
        /// The parent the rejected transform named.
        offered: String,
    },
    /// The transform is static and the frame is dynamic, or the reverse.
    ///
    /// A frame's first insert fixes its kind; mixing the two would let a
    /// stopped stream answer at every instant.
    #[error(
        "frame {frame} is {} and cannot take {} transform",
        stored_kind(*.incoming),
        incoming_kind(*.incoming)
    )]
    #[non_exhaustive]
    KindMismatch {
        /// The child frame.
        frame: String,
        /// The stamp of the rejected transform. The frame is of the other
        /// kind.
        incoming: Stamp,
    },
    /// The new edge would close a cycle: `parent` is already below `child`.
    #[error("placing {child} under {parent} would close a cycle")]
    #[non_exhaustive]
    Cycle {
        /// The child frame of the rejected edge.
        child: String,
        /// The parent frame of the rejected edge.
        parent: String,
    },
    /// The sample is older than its frame's retention window, so storing it
    /// would evict it in the same call. Nothing is stored.
    ///
    /// Reported only by a registry built with
    /// [`Registry::with_max_age`](crate::Registry::with_max_age), only for
    /// a dynamic sample, and only when `stamp` is strictly older than
    /// `newest` minus `max_age`. Compare the two stamps with `Ord`; do not
    /// subtract them by hand.
    ///
    /// The error does not say which stamp is wrong. A sample that arrives
    /// late and a frame whose newest stamp is ahead of its stream report it
    /// the same way, and the sample that moved `newest` ahead was accepted.
    ///
    /// To recover, [`Registry::remove_frame`](crate::Registry::remove_frame)
    /// discards the frame's samples together with `newest`, and releases
    /// its parent and kind until its next insert.
    /// [`Registry::remove_samples_through`](crate::Registry::remove_samples_through)
    /// with `newest` as the cutoff drains the frame and keeps its parent
    /// and kind, and also removes every dynamic sample at or before
    /// `newest` from every other frame. From
    /// [`Registry::insert_all`](crate::Registry::insert_all), `newest` can
    /// be the stamp of an earlier member of the same batch, which the
    /// registry does not hold after the error.
    #[error("sample for {frame} at {stamp} is older than the retention window behind {newest}")]
    #[non_exhaustive]
    Expired {
        /// The child frame of the rejected sample.
        frame: String,
        /// The rejected sample's instant.
        stamp: Timestamp,
        /// The frame's newest instant when the sample was checked.
        newest: Timestamp,
    },
    /// [`Registry::reparent`](crate::Registry::reparent) only: the frame to
    /// move is not known to the registry.
    #[error("frame {frame} is not known")]
    #[non_exhaustive]
    UnknownFrame {
        /// The frame that was to be moved.
        frame: String,
    },
    /// [`Registry::reparent`](crate::Registry::reparent) only: the frame to
    /// move is a root. It has no parent to replace;
    /// [`Registry::insert`](crate::Registry::insert) gives it one.
    #[error("frame {frame} has no parent to replace")]
    #[non_exhaustive]
    NoParent {
        /// The root frame.
        frame: String,
    },
    /// [`Registry::reparent`](crate::Registry::reparent) only: the frame is
    /// already under that parent. Re-parenting drops history, so it is not
    /// an upsert.
    #[error("frame {frame} is already under {parent}")]
    #[non_exhaustive]
    ParentUnchanged {
        /// The frame that was to be moved.
        frame: String,
        /// Its current parent, which the seed named again.
        parent: String,
    },
}

/// The kind of the frame that rejected a transform stamped `incoming`.
fn stored_kind(incoming: Stamp) -> &'static str {
    match incoming {
        Stamp::Static => "dynamic",
        Stamp::At(_) => "static",
    }
}

/// The kind of a transform stamped `incoming`, with its article.
fn incoming_kind(incoming: Stamp) -> &'static str {
    match incoming {
        Stamp::Static => "a static",
        Stamp::At(_) => "a dynamic",
    }
}

/// Why [`Registry::insert_all`](crate::Registry::insert_all) stored
/// nothing: the member that was rejected, and why.
///
/// [`source`](core::error::Error::source) returns the [`InsertError`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error("batch member {index} was rejected")]
pub struct BatchError {
    /// The zero-based index, in the slice, of the first member the insert
    /// loop would have rejected.
    pub index: usize,
    /// Why that member was rejected.
    #[source]
    pub error: InsertError,
}

/// Why a query could not be answered.
///
/// Returned by [`Registry::lookup`](crate::Registry::lookup),
/// [`Registry::lookup_across`](crate::Registry::lookup_across) and
/// [`Registry::latest_common_time`](crate::Registry::latest_common_time).
/// Causes are diagnosed in the order of the variants up to
/// [`OutOfRange`](Self::OutOfRange): an unknown name, then disconnected
/// frames, then missing coverage, and only then a numerical failure. Each
/// method documents the subset it can return.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum LookupError {
    /// A name in the query is not known to the registry: no stored edge
    /// names it as child or as parent.
    ///
    /// When several names are unknown, the first in the order target,
    /// source, fixed is reported. The empty name is never known.
    #[error("frame {frame} is not known")]
    #[non_exhaustive]
    UnknownFrame {
        /// The unknown name.
        frame: String,
    },
    /// Both frames are known and no chain of edges connects them.
    ///
    /// In a cross-time lookup, `a` is the endpoint of the leg that does not
    /// connect and `b` is the fixed frame; otherwise `a` is the target and
    /// `b` the source.
    #[error("frames {a} and {b} are not connected")]
    #[non_exhaustive]
    Disconnected {
        /// One frame of the disconnected pair.
        a: String,
        /// The other frame of the disconnected pair.
        b: String,
    },
    /// An edge on the connecting chain holds no samples at all. This is not
    /// a timing problem: no instant can be served until the frame is
    /// published again.
    #[error("frame {frame} holds no samples")]
    #[non_exhaustive]
    NoData {
        /// The child frame of the empty edge.
        frame: String,
    },
    /// An edge on the connecting chain holds samples, and the requested
    /// instant lies outside them. There is no extrapolation.
    #[error("frame {frame} covers {earliest} to {latest} and not {requested}")]
    #[non_exhaustive]
    OutOfRange {
        /// The child frame of the edge.
        frame: String,
        /// The instant the lookup asked for.
        requested: Timestamp,
        /// The edge's oldest sample.
        earliest: Timestamp,
        /// The edge's newest sample.
        latest: Timestamp,
    },
    /// [`Registry::latest_common_time`](crate::Registry::latest_common_time)
    /// only: every edge on the chain holds samples, and no instant is
    /// covered by all of them.
    #[error(
        "frame {frame} covers {earliest} to {latest} and the rest of the chain ends at {latest_possible}"
    )]
    #[non_exhaustive]
    NoCommonTime {
        /// The child frame of the edge whose coverage starts too late.
        frame: String,
        /// That edge's oldest sample.
        earliest: Timestamp,
        /// That edge's newest sample.
        latest: Timestamp,
        /// The oldest of the newest samples of the chain's dynamic edges:
        /// no later instant can be common.
        latest_possible: Timestamp,
    },
    /// Combining the chain overflowed a translation.
    #[error("lookup produced a non-finite value")]
    NonFinite,
    /// A rotation computed along the chain could not be brought back to
    /// unit length. No input is known to reach this.
    #[error("lookup produced a rotation with norm {norm} that is not a unit quaternion")]
    #[non_exhaustive]
    NonUnitRotation {
        /// The norm that was measured.
        norm: f64,
    },
}

impl From<NumericError> for LookupError {
    fn from(error: NumericError) -> Self {
        match error {
            NumericError::NonFinite => Self::NonFinite,
            NumericError::NonUnitRotation { norm } => Self::NonUnitRotation { norm },
        }
    }
}
