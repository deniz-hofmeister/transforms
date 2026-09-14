//! Rigid-body coordinate transforms inspired by ROS2 tf2.
//!
//! This crate shares tf2's frame-tree, buffering, and interpolation concepts.
//! It is a Rust implementation with no ROS2 dependency or tf2 API-compatibility
//! requirement. ROS2 communication belongs in an application or adapter.
//!
//! [`Registry`] stores a frame tree and resolves transforms between frames,
//! interpolating translation linearly and rotation with SLERP. A
//! [`Transform`] maps child-frame coordinates into its parent frame:
//! `get_transform("map", "sensor", t)` expresses sensor data in map coordinates.
//!
//! Use [`Transform::static_between`] for a relationship valid at all times.
//! Dynamic samples carry [`time::Stamp::At`]; no timestamp value is reserved.
//! [`Registry::with_max_age`] evicts old samples on insertion, while
//! [`Registry::new`] retains them until manual removal.
//!
//! Constructors and deserialization validate transforms. Derived results can
//! accumulate rotation drift or overflow; see [`Transform::validate`] and
//! [`Transformable`] before applying or publishing them. Cross-time results
//! have additional restrictions documented on [`Registry::get_transform_at`].
//!
//! # Features
//!
//! - `std` (default): `Timestamp::now()`, `Timestamp::try_now()`,
//!   and `std::time::SystemTime` support.
//! - `serde`: serialization and deserialization for geometry and time types.
//!
//! Disabling `std` requires a heap allocator. All coordinates are `f64`, and
//! `libm` supplies the same arithmetic in both feature modes. Tests pin selected
//! results on the test host; cross-device bitwise replay is not guaranteed.
//!
//! # Reliability
//!
//! This crate forbids unsafe code; dependencies must still be sound. Its only
//! documented panic is `Timestamp::now()` on an unrepresentable system
//! clock; use `Timestamp::try_now()` to handle that error. Allocation
//! failure follows the application's allocation error handler.
//!
//! [`Registry`] inherits `Send` and `Sync` from its timestamp type. Readers may
//! hold `RwLock` read guards together; readers can wait for writers.
//! `HashDoS` resistance and deterministic hash ordering are not guarantees.
//!
//! Geometry equality is exact. The `approx` 0.5 comparison traits are part of
//! the public API and compare numeric components, with exact frame and time
//! metadata. See [`geometry::Quaternion`] for rotation conventions.
//!
//! # Non-Goals
//!
//! The following are outside this crate's scope:
//!
//! - Scaling transformations
//! - Skew transformations
//! - Perspective transformations
//! - Non-rigid transformations
//! - Affine transformations beyond rigid body motion
//! - API parity with ROS2 tf2
//! - Non-linear interpolation
//! - Extrapolation
//! - f32 or mixed-precision arithmetic (every coordinate and rotation is f64)
//!
//! # Examples
//!
//! ```
//! use transforms::{
//!     Registry,
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//! };
//!
//! let mut registry: Registry = Registry::new();
//! registry
//!     .add_transform(
//!         Transform::static_between(
//!             "base",
//!             "sensor",
//!             Vector3::new(1.0, 0.0, 0.0),
//!             Quaternion::identity(),
//!         )
//!         .unwrap(),
//!     )
//!     .unwrap();
//!
//! let transform = registry
//!     .get_transform("base", "sensor", Timestamp::zero())
//!     .unwrap();
//! assert_eq!(transform.parent(), "base");
//! assert_eq!(transform.child(), "sensor");
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::alloc_instead_of_core)]
#![warn(clippy::std_instead_of_core)]
#![cfg_attr(
    not(test),
    warn(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]
#![cfg_attr(test, allow(clippy::similar_names))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, doc(auto_cfg))]

extern crate alloc;
mod core;
pub mod errors;
pub mod geometry;
pub mod time;
pub use core::Registry;
pub use geometry::{Localized, Transform, Transformable};

// Compile the README examples in every feature combination.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme {}
