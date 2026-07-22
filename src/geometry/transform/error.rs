use alloc::{boxed::Box, string::String};

use thiserror::Error;

use crate::errors::{BufferError, QuaternionError, TimeError};

/// Error type for transform lookup, composition, and application.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TransformError {
    /// The rotation is not a unit quaternion within the accepted tolerance.
    #[error("rotation is not a unit quaternion (norm: {0})")]
    NonUnitRotation(f64),

    /// The transform contains non-finite (NaN or infinite) components.
    #[error("transform contains non-finite values")]
    NonFiniteValues,

    /// Two timestamps that must agree do not (given in seconds): composed
    /// transforms with differing timestamps, swapped interpolation endpoints,
    /// or applying a transform to a value from another time.
    #[error("transform timestamps do not match (lhs: {lhs}, rhs: {rhs})")]
    TimestampMismatch {
        /// The left-hand timestamp, in seconds.
        lhs: f64,
        /// The right-hand timestamp, in seconds.
        rhs: f64,
    },

    /// The requested timestamp lies outside the covered time range (all
    /// values in seconds). There is no extrapolation. `requested > end`
    /// means the request is merely too new (latency); `requested < start`
    /// means the data is stale or missing.
    #[error("requested timestamp {requested} is outside the covered range [{start}, {end}]")]
    TimestampOutOfRange {
        /// The requested timestamp, in seconds.
        requested: f64,
        /// The start of the covered range, in seconds.
        start: f64,
        /// The end of the covered range, in seconds.
        end: f64,
    },

    /// Both transforms describe the same child frame.
    #[error("cannot multiply transforms that both describe child frame {frame}")]
    SameFrameMultiplication {
        /// The child frame described by both operands.
        frame: String,
    },

    /// The frames do not form a valid parent-child composition.
    #[error("frames do not have a parent-child relationship (expected {expected}, found {found})")]
    IncompatibleFrames {
        /// The frame (or frame pair) the operation required.
        expected: String,
        /// The frame (or frame pair) actually found.
        found: String,
    },

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

    /// The lookup stopped at a frame whose buffer holds data but could not
    /// serve the requested time — typically a transient gap: the request
    /// is outside the frame's covered time range. `frame` names where the
    /// chain walk stopped and `source` carries the buffer's error,
    /// including the covered range.
    ///
    /// Receiving this variant does not guarantee the frames are connectable:
    /// when a data gap and a topological disconnection coexist, the recorded
    /// walk failure takes precedence over the [`Disconnected`](Self::Disconnected)
    /// diagnosis.
    #[error(
        "transform from {source_frame} into {target_frame} not found (frame {frame}: {source})"
    )]
    NotFoundAt {
        /// The `target` argument of the failed lookup — the frame the data
        /// would have been expressed in.
        target_frame: String,
        /// The `source` argument of the failed lookup — the frame the data
        /// would have come from.
        source_frame: String,
        /// The frame whose buffer could not serve the requested time.
        frame: String,
        /// The buffer error that stopped the chain walk.
        source: Box<BufferError>,
    },

    /// A timestamp operation failed.
    #[error("timestamp error: {0}")]
    TimestampError(#[from] TimeError),

    /// A quaternion operation failed.
    #[error("quaternion error: {0}")]
    QuaternionError(#[from] QuaternionError),
}
