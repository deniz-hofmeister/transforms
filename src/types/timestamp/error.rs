use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimestampError {
    #[error("Negative duration")]
    NegativeDuration,
    #[error("Duration overflow")]
    DurationOverflow,
    #[error("Conversion to seconds lost accuracy")]
    AccuracyLoss,
}
