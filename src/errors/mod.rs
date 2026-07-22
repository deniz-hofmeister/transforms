//! Re-exports of all error types in this crate.
//!
//! # Display messages are not a stability surface
//!
//! Match on error variants and their payloads, never on `Display` text:
//! message wording may improve in minor releases without notice. The
//! variants and their fields are the stable contract.

pub use crate::{
    core::buffer::BufferError,
    geometry::{quaternion::QuaternionError, transform::TransformError},
    time::TimeError,
};
