use thiserror::Error;

use crate::errors::TransformError;

/// Error type for buffer insertion and retrieval.
#[derive(Error, Debug)]
pub enum BufferError {
    /// No stored transforms match the requested timestamp.
    #[error("no transforms available matching your criteria")]
    NoTransformAvailable,

    /// The buffer already holds transforms of the other kind; a child frame
    /// is either static or dynamic, never both.
    #[error("cannot mix static and dynamic transforms for the same child frame")]
    StaticDynamicConflict,

    /// A transform operation failed during retrieval.
    #[error("transform error: {0}")]
    TransformError(#[from] TransformError),
}
