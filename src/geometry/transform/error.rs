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
    #[error("cannot multiply transforms with the same frame")]
    SameFrameMultiplication,

    /// The frames do not form a valid parent-child composition.
    #[error("frames do not have a parent-child relationship")]
    IncompatibleFrames,

    /// No transform chain connects the two frames at the requested time.
    #[error("transform not found from {0} to {1}")]
    NotFound(String, String),

    /// The lookup stopped at a frame whose buffer holds data but could not
    /// serve the requested time. Unlike [`NotFound`](Self::NotFound), the
    /// failure has a concrete cause: `frame` names where the chain walk
    /// stopped and `source` carries the buffer's error.
    #[error("transform not found from {from} to {to} (frame {frame}: {source})")]
    NotFoundAt {
        /// The requested source frame.
        from: String,
        /// The requested target frame.
        to: String,
        /// The frame whose buffer could not serve the requested time.
        frame: String,
        /// The buffer error that stopped the chain walk.
        source: Box<BufferError>,
    },

    /// The transform chain was empty after processing.
    #[error("transform tree is empty")]
    TransformTreeEmpty,

    /// A timestamp operation failed.
    #[error("timestamp error: {0}")]
    TimestampError(#[from] TimeError),

    /// A quaternion operation failed.
    #[error("quaternion error: {0}")]
    QuaternionError(#[from] QuaternionError),
}
