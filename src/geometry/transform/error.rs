use crate::errors::{QuaternionError, TimestampError};
use alloc::string::String;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Transform timestamps do not match (lhs: {0}, rhs: {1})")]
    TimestampMismatch(f64, f64),

    #[error("Cannot multiply transforms with the same frame")]
    SameFrameMultiplication,

    #[error("Frames do not have a parent-child relationship")]
    IncompatibleFrames,

    #[error("Transform not found from {0} to {1}")]
    NotFound(String, String),

    #[error("Transform tree is empty")]
    TransformTreeEmpty,

    #[error("Timestamp error: {0}")]
    TimestampError(#[from] TimestampError),

    #[error("Quaternion error: {0}")]
    QuaternionError(#[from] QuaternionError),
}
