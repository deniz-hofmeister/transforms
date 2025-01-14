//! A blazingly fast and efficient coordinate transform library for robotics and computer vision applications.
//!
//! This library provides functionality for managing coordinate transformations between different frames
//! of reference.
//!
//! # Architecture
//!
//! The library is organized around three main components:
//!
//! - **Registry**: The main interface for managing transforms
//! - **Buffer**: Internal storage for transforms between specific frames
//! - **Transform**: The core data structure representing spatial transformations
//!
//! # Features
//!
//! - **Transform Interpolation**: Smooth interpolation between transforms at different timestamps
//! - **Transform Chaining**: Automatic computation of transforms between indirectly connected frames
//! - **Thread-safe Operations**: Safe concurrent access to the transform registry
//! - **Time-based Buffer Management**: Automatic cleanup of old transforms
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
//!
//! This decision helps maintain the library's focus on its core purpose: providing fast and efficient
//! rigid body transformations for robotics applications. For more general transformation needs,
//! consider using a computer graphics or linear algebra library instead.
//!
//! # Examples
//!
//! ```rust
//! use transforms::{
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//!     Registry,
//! };
//!
//! # #[cfg(feature = "std")]
//! use core::time::Duration;
//! # #[cfg(feature = "std")]
//! let mut registry = Registry::new(Duration::from_secs(60));
//! # #[cfg(feature = "std")]
//! let timestamp = Timestamp::now();
//!
//! # #[cfg(not(feature = "std"))]
//! let mut registry = Registry::new();
//! # #[cfg(not(feature = "std"))]
//! let timestamp = Timestamp::zero();
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
//!     geometry::{Point, Quaternion, Transform, Vector3},
//!     time::Timestamp,
//!     Transformable,
//! };
//!
//! // Create a point in the camera frame
//! let mut point = Point {
//!     position: Vector3::new(1.0, 0.0, 0.0),
//!     orientation: Quaternion::identity(),
//! # #[cfg(not(feature = "std"))]
//!     timestamp: Timestamp::zero(),
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
//! # Performance Considerations
//!
//! - Transform lookups are optimized for O(log n) time complexity
//! - Automatic cleanup of old transforms prevents unbounded memory growth
//!
//! # Safety
//!
//! This crate uses `#![forbid(unsafe_code)]` to ensure memory safety through pure Rust implementations.
#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
pub mod core;
pub mod errors;
pub mod geometry;
pub mod time;
pub use core::Registry;
pub use geometry::{Transform, Transformable};
