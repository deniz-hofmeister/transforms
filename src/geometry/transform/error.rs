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
    #[error("transform timestamps do not match (lhs: {0}, rhs: {1})")]
    TimestampMismatch(f64, f64),

    /// The requested timestamp lies outside the covered time range (all
    /// values in seconds: requested, range start, range end). There is no
    /// extrapolation.
    #[error("requested timestamp {0} is outside the covered range [{1}, {2}]")]
    TimestampOutOfRange(f64, f64, f64),

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
    #[error("no transform chain connects {0} and {1}")]
    Disconnected(String, String),

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
    #[error("transform not found from {from} to {to} (frame {frame}: {source})")]
    NotFoundAt {
        /// The `target` argument of the failed lookup — the frame the data
        /// would have been expressed in.
        from: String,
        /// The `source` argument of the failed lookup — the frame the data
        /// would have come from.
        to: String,
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
