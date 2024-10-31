use thiserror::Error;

#[derive(Error, Debug)]
pub enum DurationError {
    #[error("Duration overflow")]
    DurationOverflow,

    #[error("Conversion to seconds lost accuracy")]
    AccuracyLoss,

    #[error("Division by zero")]
    DivisionByZero,
}
