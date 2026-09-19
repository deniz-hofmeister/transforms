//! Errors from registry, geometry, and time operations.
//!
//! Fallible [`Registry`](crate::Registry) operations return [`RegistryError`].
//! Geometry operations return [`TransformError`], which can wrap
//! [`QuaternionError`] or [`TimeError`].
//!
//! Match variants and payloads, not `Display` or `Debug` text. All public error
//! enums are non-exhaustive; message wording may change in minor releases.

pub use crate::{
    core::RegistryError,
    geometry::{quaternion::QuaternionError, transform::TransformError},
    time::TimeError,
};
