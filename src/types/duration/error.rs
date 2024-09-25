use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimestampError {
    #[error("Duration overflow")]
    DurationOverflow,
}
