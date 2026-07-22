//! A fast, middleware-independent coordinate transform library for robotics and computer vision applications.
//!
//! This library provides functionality for managing coordinate transformations between different frames
//! of reference.
//!
//! # Architecture
//!
//! The library is organized around three main components:
//!
//! - **Registry**: The main interface for managing transforms
//! - **Buffer**: The time-indexed store for one parent-child frame pair,
//!   also usable standalone
//! - **Transform**: The core data structure representing spatial transformations
//!
//! # Features
//!
//! - **Transform Interpolation**: Smooth interpolation between transforms at different timestamps
//! - **Transform Chaining**: Automatic computation of transforms between indirectly connected frames
//! - **Static Transforms**: Transforms carrying the static timestamp sentinel
//!   (`Timestamp::STATIC`, i.e. `u128::MAX` nanoseconds by default) are valid for
//!   all time; build them with `Transform::static_between`.
//! - **Custom Timestamp Types**: You can use your own `Copy` timestamp type by implementing `time::TimePoint`.
//! - **Time-based Buffer Management**: `Registry::with_max_age` cleans up old transforms
//!   automatically on insert; `Registry::new` keeps them until `delete_transforms_before`
//!   is called. Both work with and without `std`.
//! - **Serde**: optional serialization for the geometry and time types behind the `serde` feature.
//!
//! # Non-Goals
//!
//! This library intentionally limits its scope to rigid body transformations (translation and rotation)
//! commonly used in robotics and computer vision. The following transformations are explicitly not
//! supported and will not be considered for future implementation:
//!
//! - Scaling transformations
//! - Skew transformations
//! - Perspective transformations
//! - Non-rigid transformations
//! - Affine transformations beyond rigid body motion
//! - API parity with ROS2 tf2
//! - Non-linear interpolation
//! - Extrapolation
//!
//! This decision helps maintain the library's focus on its core purpose: providing fast and efficient
//! rigid body transformations for robotics applications. For more general transformation needs,
//! consider using a computer graphics or linear algebra library instead.
//!
//! # Examples
//!
//! ```rust
//! use transforms::{
//!     Registry,
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//! };
//!
//! # #[cfg(feature = "std")]
//! use core::time::Duration;
//! # #[cfg(feature = "std")]
//! let mut registry = Registry::with_max_age(Duration::from_secs(60));
//! # #[cfg(feature = "std")]
//! let timestamp = Timestamp::now();
//!
//! # #[cfg(not(feature = "std"))]
//! # let mut registry = Registry::new();
//! # #[cfg(not(feature = "std"))]
//! # let timestamp = Timestamp::zero();
//!
//! // Create a transform from frame "base" to frame "sensor"
//! let transform = Transform {
//!     translation: Vector3::new(1.0, 0.0, 0.0),
//!     rotation: Quaternion::identity(),
//!     timestamp,
//!     parent: "base".into(),
//!     child: "sensor".into(),
//! };
//!
//! // Add the transform to the registry
//! registry.add_transform(transform).unwrap();
//!
//! // Retrieve the transform
//! let result = registry.get_transform("base", "sensor", timestamp).unwrap();
//!
//! # #[cfg(not(feature = "std"))]
//! # // Delete old transforms
//! # #[cfg(not(feature = "std"))]
//! # registry.delete_transforms_before(timestamp);
//! ```
//!
//! # Transform and Data Transformation
//!
//! The library provides a `Transform` type that represents spatial transformations between different
//! coordinate frames. Transforms follow the common robotics convention where transformations are
//! considered from child to parent frame (e.g., from sensor frame to base frame, or from base frame
//! to map frame).
//!
//! To make your data transformable between different coordinate frames, implement the `Transformable`
//! trait. This allows you to easily transform your data using the transforms stored in the registry.
//! ```rust
//! use transforms::{
//!     Transformable,
//!     geometry::{Point, Quaternion, Transform, Vector3},
//!     time::Timestamp,
//! };
//!
//! // Create a point in the camera frame
//! let mut point = Point {
//!     position: Vector3::new(1.0, 0.0, 0.0),
//!     orientation: Quaternion::identity(),
//! # #[cfg(not(feature = "std"))]
//! # timestamp: Timestamp::zero(),
//! # #[cfg(feature = "std")]
//!     timestamp: Timestamp::now(),
//!     frame: "camera".into(),
//! };
//!
//! // Define transform from camera to base frame
//! let transform = Transform {
//!     translation: Vector3::new(0.0, 1.0, 0.0),
//!     rotation: Quaternion::identity(),
//!     timestamp: point.timestamp,
//!     parent: "base".into(),
//!     child: "camera".into(),
//! };
//!
//! // Transform the point from camera frame to base frame
//! point.transform(&transform).unwrap();
//! assert_eq!(point.position.x, 1.0);
//! assert_eq!(point.position.y, 1.0);
//! ```
//!
//! The transform convention follows the common robotics practice where data typically needs to be
//! transformed from specific sensor reference frames "up" to more general frames like the robot's
//! base frame or a global map frame.
//!
//! # Relationship with ROS2's tf2
//!
//! This library draws inspiration from ROS2's tf2 (Transform Framework 2), a widely-used
//! transform library in the robotics community. While this crate aims to solve the same
//! fundamental problem of transformation tracking, it does so in its own way.
//!
//! ## Similarities with tf2
//!
//! - Maintains relationships between coordinate frames in a tree structure
//! - Buffers transforms over time
//! - Supports transform lookups between arbitrary frames
//! - Handles interpolation between transforms
//!
//! ## Key Differences
//!
//! This library:
//! - Is a pure Rust implementation, not a wrapper around tf2
//! - Makes no attempt to perfectly match the ROS2/tf2 API
//! - Focuses on providing an ergonomic Rust-first experience
//! - Is independent of ROS2's middleware and communication system
//!
//! While the core concepts and functionality align with tf2, this library prioritizes
//! optimal usage for rust software over maintaining API compatibility with ROS2's tf2. Users
//! familiar with tf2 will find the concepts familiar, but the implementation details
//! and API design follow Rust idioms and best practices as best as it can.
//!
//! # `TimePoint` vs `Timestamp`
//!
//! `time::TimePoint` defines the required behavior for timestamp types.
//! `time::Timestamp` is the default implementation.
//! `Registry::new(...)` therefore uses `Timestamp` by default.
//! If you need a custom clock, implement `TimePoint` and use
//! `Registry::<CustomTimestamp>::new(...)`.
//! With `std`, `std::time::SystemTime` is already supported via an existing
//! `TimePoint` implementation.
//! See `time` module docs for custom time-type guidance.
//!
//! # Performance Considerations
//!
//! - Transform lookups are O(log n) in the stored samples per frame;
//!   multi-hop lookups additionally scale linearly with chain depth, and a
//!   failed lookup runs an O(frames) diagnosis scan to name the cause
//! - Automatic cleanup of old transforms prevents unbounded memory growth
//!   (eviction on insert is O(log n + evicted)); the number of *frames* is
//!   unbounded — long-running processes that mint transient frame names
//!   should call `Registry::remove_frame` when a frame retires
//!
//! # External Crates
//!
//! If you are looking for a version of this crate that is directly compatible with ROS1 & ROS2 consider
//! [roslibrust_transforms](https://docs.rs/roslibrust_transforms/latest/roslibrust_transforms/) that wraps
//! this crate for pure-Rust ROS clients.
//!
//! # Reliability
//!
//! - **Memory safety**: `#![forbid(unsafe_code)]` — pure Rust throughout.
//! - **Panic policy**: library code does not panic on reachable paths; the
//!   single documented exception is `Timestamp::now()` on a system clock
//!   before the Unix epoch (`Timestamp::try_now` is the panic-free
//!   variant). This is enforced with clippy's `unwrap_used`,
//!   `expect_used`, `panic`, and `indexing_slicing` restriction lints.
//!   In `no_std` builds, allocation failure aborts via the global
//!   allocation error handler, as with any `alloc`-based crate: size the
//!   heap for `max_age` times the insert rate, or bound growth with
//!   `Registry::delete_transforms_before`.
//! - **Checked arithmetic**: all time arithmetic is checked; overflow and
//!   underflow surface as errors, never as wraparound.
//! - **Validated inputs**: transforms are validated at the registry boundary
//!   (finite values, unit rotations, an acyclic single-parent frame tree);
//!   invalid data is rejected with an error rather than corrupting lookups.
//! - **Thread safety**: all types are `Send + Sync`; wrap the `Registry` in
//!   your preferred lock for concurrent use (see the README for an example).
//! - **Deterministic hashing**: the frame map uses hashbrown's default
//!   hasher with a fixed seed on targets without entropy sources, giving
//!   deterministic behavior on MCUs. `HashDoS` resistance is deliberately
//!   not a goal — frame names come from the application, not the network.
//!
//! # Stability Commitments
//!
//! The `approx` traits (`AbsDiffEq`/`RelativeEq`) implemented on the
//! geometry types make `approx` 0.5 part of this crate's public API: a
//! future `approx` 0.6 requires a semver-major release of this crate. This
//! is deliberate — tolerant comparison is the documented alternative to the
//! exact `==`.
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
pub mod core;
pub mod errors;
pub mod geometry;
pub mod time;
pub use core::Registry;
pub use geometry::{Localized, Transform, Transformable};
