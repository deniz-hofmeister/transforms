use thiserror::Error;

/// Error type for timestamp and time-point operations.
#[derive(Error, Debug)]
pub enum TimeError {
    #[error("Duration underflow")]
    DurationUnderflow,
    #[error("Duration overflow")]
    DurationOverflow,
    #[error("Conversion to seconds lost accuracy")]
    AccuracyLoss,
}

#[deprecated(since = "1.2.0", note = "use time::TimeError instead")]
pub type TimestampError = TimeError;
