use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuaternionError {
    #[error("Division by zero quaternion")]
    DivisionByZero,
    #[error("Cannot normalize a zero-length quaternion")]
    ZeroLengthNormalization,
}
