//! Re-exports of all error types in this crate.
//!
//! [`RegistryError`] is what every [`Registry`](crate::Registry) call
//! reports — insertion and lookup alike — and it is flat: one `match`
//! reaches every cause. [`TransformError`] is the geometry side, reported by
//! the [`Transform`](crate::Transform) constructors and by composition,
//! inversion, interpolation and [`Transformable`](crate::Transformable).
//! [`QuaternionError`] and [`TimeError`] are the leaf types
//! [`TransformError`] wraps.
//!
//! # Display messages are not a stability surface
//!
//! Match on error variants and their payloads, never on `Display` text:
//! message wording may improve in minor releases without notice. The
//! variants and their fields are the stable contract.

pub use crate::{
    core::RegistryError,
    geometry::{quaternion::QuaternionError, transform::TransformError},
    time::TimeError,
};
