//! Rigid-body coordinate transforms inspired by ROS2 tf2.
//!
//! This crate shares tf2's frame-tree, buffering, and interpolation concepts.
//! It is a Rust implementation with no ROS2 dependency or tf2 API-compatibility
//! requirement. ROS2 communication belongs in an application or adapter.
//!
//! [`Registry`] stores a frame tree and resolves transforms between frames,
//! interpolating translation linearly and rotation with SLERP. A
//! [`Transform`] maps source-frame coordinates into its target frame:
//! `lookup("map", "sensor", t)` expresses sensor data in map coordinates.
//!
//! A relationship valid at all times carries [`Stamp::Static`]; a sample of
//! a moving one carries [`Stamp::At`]. No timestamp value is reserved.
//! [`Registry::with_max_age`] keeps a time window per frame and rejects
//! samples older than it, while [`Registry::unbounded`] retains every
//! sample until it is removed. [`Registry::insert_all`] stores a batch all
//! or nothing, and [`Registry::reparent`] moves a frame under a new parent,
//! dropping that frame's stored history. [`Registry::latest_common_time`]
//! answers "what is the newest instant I can look up?" between two frames,
//! and [`Registry::lookup_across`] relates two frames at two instants.
//!
//! Every [`Rotation`] is a unit quaternion and every [`Transform`] is
//! finite, whether it was built, decoded, composed, inverted or looked up:
//! an operation that cannot produce a valid value returns an error. The
//! crate does not apply a transform to a point; the [`Transform`]
//! documentation shows how.
//!
//! # Features
//!
//! - `serde`: serialization and deserialization for [`Vector3`],
//!   [`Rotation`], [`Timestamp`], [`Stamp`] and [`Transform`]. Off by
//!   default.
//!
//! The crate is `no_std` and needs a heap allocator. All coordinates are
//! `f64`, and `libm` supplies the square root and the trigonometry on every
//! target. Tests pin selected results on the test host; cross-device bitwise
//! replay is not guaranteed.
//!
//! # Reliability
//!
//! This crate forbids unsafe code; dependencies must still be sound. It has
//! no documented panic. Allocation failure follows the application's
//! allocation error handler.
//!
//! [`Registry`] is `Send` and `Sync`. Readers may hold `RwLock` read guards
//! together; readers can wait for writers. `HashDoS` resistance and
//! deterministic hash ordering are not guarantees.
//!
//! Geometry equality is exact. See [`Rotation`] for rotation conventions.
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
//! use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
//!
//! let mut registry = Registry::unbounded();
//! registry
//!     .insert(
//!         &Transform::new(
//!             "base",
//!             "sensor",
//!             Vector3::new(1.0, 0.0, 0.0),
//!             Rotation::IDENTITY,
//!             Stamp::Static,
//!         )
//!         .unwrap(),
//!     )
//!     .unwrap();
//!
//! let transform = registry
//!     .lookup("base", "sensor", Timestamp::from_nanos(0))
//!     .unwrap();
//! assert_eq!(transform.target(), "base");
//! assert_eq!(transform.source(), "sensor");
//! ```
#![no_std]
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
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, doc(auto_cfg))]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod geometry;
mod registry;
mod time;

pub use geometry::{CrossTimeTransform, Rotation, Transform, TransformError, Vector3};
pub use registry::{BatchError, InsertError, LookupError, Registry};
pub use time::{Stamp, Timestamp};

// Compile the README examples in both feature modes.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme {}
