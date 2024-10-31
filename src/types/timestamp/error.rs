use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimestampError {
    #[error("Duration underflow")]
    DurationUnderflow,
    #[error("Duration overflow")]
    DurationOverflow,
    #[error("Conversion to seconds lost accuracy")]
    AccuracyLoss,
}
