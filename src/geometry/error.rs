//! Errors of building and combining transforms.

use crate::time::Timestamp;
use alloc::string::String;
use thiserror::Error;

/// Why a [`Rotation`](crate::Rotation) or [`Transform`](crate::Transform)
/// could not be built, composed or inverted.
///
/// Each method documents the subset it can return.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum TransformError {
    /// A component is NaN or infinite, or an operation produced one.
    #[error("transform contains a non-finite value")]
    NonFinite,
    /// The rotation is not a unit quaternion.
    ///
    /// From [`Rotation::from_wxyz`](crate::Rotation::from_wxyz), the norm of
    /// the given components is further than `1e-6` from 1. From
    /// [`Transform::compose`](crate::Transform::compose), the product could
    /// not be brought back to unit length.
    #[error("rotation with norm {norm} is not a unit quaternion")]
    #[non_exhaustive]
    NonUnitRotation {
        /// The norm that was measured.
        norm: f64,
    },
    /// A frame name is empty: a missing name, which every producer that
    /// left it unset would share as one frame.
    #[error("frame name is empty")]
    EmptyFrameName,
    /// Two transforms do not chain: the left-hand source frame is not the
    /// right-hand target frame.
    #[error("frame {found} does not match expected frame {expected}")]
    #[non_exhaustive]
    FrameMismatch {
        /// The left-hand operand's source frame.
        expected: String,
        /// The right-hand operand's target frame.
        found: String,
    },
    /// Two dynamic transforms hold at different instants.
    #[error("stamps {lhs} and {rhs} differ")]
    #[non_exhaustive]
    StampMismatch {
        /// The left-hand operand's instant.
        lhs: Timestamp,
        /// The right-hand operand's instant.
        rhs: Timestamp,
    },
}

/// A numerical failure of the private arithmetic, before it is reported in
/// the error type of the public operation that met it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NumericError {
    /// An operation produced a NaN or an infinity.
    NonFinite,
    /// A finite rotation could not be brought back to unit length.
    NonUnitRotation {
        /// The norm measured after the attempt.
        norm: f64,
    },
}

impl From<NumericError> for TransformError {
    fn from(error: NumericError) -> Self {
        match error {
            NumericError::NonFinite => Self::NonFinite,
            NumericError::NonUnitRotation { norm } => Self::NonUnitRotation { norm },
        }
    }
}
