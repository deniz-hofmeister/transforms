use crate::error::TransformError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("Max age in seconds of {0} must be > 0 and <= {1}")]
    MaxAgeInvalid(f64, f64),

    #[error("No transforms available matching your criteria")]
    NoTransformAvailable,

    #[error("Transform error: {0}")]
    TransformError(#[from] TransformError),
}
