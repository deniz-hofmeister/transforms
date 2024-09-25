use thiserror::Error;

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("Max age value {0} must be > 0 and <= {1}")]
    MaxAgeInvalid(f64, i128),
}
