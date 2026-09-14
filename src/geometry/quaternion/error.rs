use thiserror::Error;

/// Error type for quaternion operations.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QuaternionError {
    /// The quaternion has (near-)zero norm and cannot be normalized.
    #[error("cannot normalize a zero-length quaternion")]
    ZeroLengthNormalization,
    /// The norm is non-finite, from non-finite components or overflow.
    #[error("quaternion has non-finite components")]
    NonFinite,
}
