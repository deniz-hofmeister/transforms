use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Transform timestamps do not match (difference: {0} seconds)")]
    TimestampMismatch(f64),

    #[error("Cannot multiply transforms with the same frame")]
    SameFrameMultiplication,

    #[error("Frames do not have a parent-child relationship")]
    IncompatibleFrames,
}
