use thiserror::Error;

/// Error type for quaternion operations.
#[derive(Error, Debug)]
pub enum QuaternionError {
    /// The divisor quaternion has (near-)zero norm.
    #[error("division by zero quaternion")]
    DivisionByZero,
    /// The quaternion has (near-)zero norm and cannot be normalized.
    #[error("cannot normalize a zero-length quaternion")]
    ZeroLengthNormalization,
    /// The quaternion has non-finite (NaN or infinite) components.
    #[error("quaternion has non-finite components")]
    NonFinite,
}
