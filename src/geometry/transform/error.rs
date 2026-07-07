use alloc::string::String;

use thiserror::Error;

use crate::errors::{QuaternionError, TimeError};

/// Error type for transform lookup, composition, and application.
#[derive(Error, Debug)]
pub enum TransformError {
    /// Two timestamps that must agree do not (given in seconds): composed
    /// transforms with differing timestamps, an interpolation request outside
    /// the covered range, or applying a transform to a value from another time.
    #[error("transform timestamps do not match (lhs: {0}, rhs: {1})")]
    TimestampMismatch(f64, f64),

    /// Both transforms describe the same child frame.
    #[error("cannot multiply transforms with the same frame")]
    SameFrameMultiplication,

    /// The frames do not form a valid parent-child composition.
    #[error("frames do not have a parent-child relationship")]
    IncompatibleFrames,

    /// No transform chain connects the two frames at the requested time.
    #[error("transform not found from {0} to {1}")]
    NotFound(String, String),

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
