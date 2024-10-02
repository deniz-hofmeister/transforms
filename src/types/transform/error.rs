use crate::error::{DurationError, TimestampError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Transform timestamps do not match (difference: {0} seconds)")]
    TimestampMismatch(f64),

    #[error("Cannot multiply transforms with the same frame")]
    SameFrameMultiplication,

    #[error("Frames do not have a parent-child relationship")]
    IncompatibleFrames,

    #[error("Duration error: {0}")]
    DurationError(#[from] DurationError),

    #[error("Timestamp error: {0}")]
    TimestampError(#[from] TimestampError),
}
