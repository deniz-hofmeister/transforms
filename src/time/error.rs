use thiserror::Error;

/// Error type for timestamp and time-point operations.
#[derive(Error, Debug)]
pub enum TimeError {
    /// Subtracting would produce a time before the representable range.
    #[error("duration underflow")]
    DurationUnderflow,
    /// Adding would produce a time beyond the representable range.
    #[error("duration overflow")]
    DurationOverflow,
    /// Converting to seconds could not be done exactly.
    #[error("conversion to seconds lost accuracy")]
    AccuracyLoss,
}
