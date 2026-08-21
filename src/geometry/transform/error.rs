use alloc::string::String;

use thiserror::Error;

use crate::errors::{QuaternionError, TimeError};

/// Error type for building, composing, interpolating, and applying
/// transforms.
///
/// Pure geometry and time: a failure of a [`Registry`](crate::Registry) call
/// is a [`RegistryError`](crate::errors::RegistryError) instead, which flattens
/// the two validation causes below into variants of its own and wraps the
/// rest.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TransformError {
    /// The rotation is not a unit quaternion within the accepted tolerance.
    #[error("rotation is not a unit quaternion (norm: {0})")]
    NonUnitRotation(f64),

    /// The transform contains non-finite (NaN or infinite) components.
    #[error("transform contains non-finite values")]
    NonFiniteValues,

    /// Two timestamps that must agree do not (given in seconds): composed
    /// transforms with differing timestamps, swapped interpolation endpoints,
    /// or applying a transform to a value from another time.
    #[error("transform timestamps do not match (lhs: {lhs}, rhs: {rhs})")]
    TimestampMismatch {
        /// The left-hand timestamp, in seconds.
        lhs: f64,
        /// The right-hand timestamp, in seconds.
        rhs: f64,
    },

    /// A static transform was used as an interpolation endpoint. A static
    /// transform is valid for all time — there is nothing to interpolate.
    #[error("static transforms cannot be interpolation endpoints")]
    StaticInterpolation,

    /// The requested timestamp lies outside the covered time range (all
    /// values in seconds). There is no extrapolation. `requested > end`
    /// means the request is merely too new (latency); `requested < start`
    /// means the data is stale or missing.
    #[error("requested timestamp {requested} is outside the covered range [{start}, {end}]")]
    TimestampOutOfRange {
        /// The requested timestamp, in seconds.
        requested: f64,
        /// The start of the covered range, in seconds.
        start: f64,
        /// The end of the covered range, in seconds.
        end: f64,
    },

    /// Both transforms describe the same child frame.
    #[error("cannot multiply transforms that both describe child frame {frame}")]
    SameFrameMultiplication {
        /// The child frame described by both operands.
        frame: String,
    },

    /// The frames do not match the pairing the operation requires: a
    /// composition whose left-hand child is not the right-hand parent,
    /// interpolation endpoints describing different frame pairs, or a value
    /// whose frame is not the transform's child.
    #[error("frames do not have a parent-child relationship (expected {expected}, found {found})")]
    IncompatibleFrames {
        /// The frame (or frame pair) the operation required.
        expected: String,
        /// The frame (or frame pair) actually found.
        found: String,
    },

    /// A timestamp operation failed.
    #[error("timestamp error: {0}")]
    TimestampError(#[from] TimeError),

    /// A quaternion operation failed.
    #[error("quaternion error: {0}")]
    QuaternionError(#[from] QuaternionError),
}
