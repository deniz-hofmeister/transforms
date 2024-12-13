//! A blazingly fast and efficient coordinate transform library for robotics and computer vision applications.
//!
//! This library provides functionality for managing coordinate transformations between different frames
//! of reference. It supports both synchronous and asynchronous operations through feature flags, making
//! it suitable for both real-time and event-driven applications.
//!
//! # Features
//!
//! - **Synchronous and Asynchronous APIs**: Choose between sync and async implementations via the `async` feature flag
//! - **Transform Interpolation**: Smooth interpolation between transforms at different timestamps
//! - **Transform Chaining**: Automatic computation of transforms between indirectly connected frames
//! - **Thread-safe Operations**: Safe concurrent access to the transform registry
//! - **Time-based Buffer Management**: Automatic cleanup of old transforms
//!
//! # Examples
//!
//! ## Synchronous Usage
//!
//! ```rust
//! # #[cfg(not(feature = "async"))]
//! # async fn example() {
//! use std::time::Duration;
//! use transforms::{
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//!     Registry,
//! };
//!
//! let mut registry = Registry::new(Duration::from_secs(60));
//! let timestamp = Timestamp::now();
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
//! # }
//! ```
//!
//! ## Asynchronous Usage
//!
//! ```rust
//! # #[cfg(feature = "async")]
//! # async fn example() {
//! use std::time::Duration;
//! use transforms::{
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//!     Registry,
//! };
//!
//! let registry = Registry::new(Duration::from_secs(60));
//! let timestamp = Timestamp::now();
//!
//! let transform = Transform {
//!     translation: Vector3::new(1.0, 0.0, 0.0),
//!     rotation: Quaternion::identity(),
//!     timestamp,
//!     parent: "base".into(),
//!     child: "sensor".into(),
//! };
//!
//! registry.add_transform(transform).await.unwrap();
//!
//! // Wait for transform to become available
//! let result = registry
//!     .await_transform("base", "sensor", timestamp)
//!     .await
//!     .unwrap();
//! # }
//! ```
//!
//! # Architecture
//!
//! The library is organized around three main components:
//!
//! - **Registry**: The main interface for managing transforms
//! - **Buffer**: Internal storage for transforms between specific frames
//! - **Transform**: The core data structure representing spatial transformations
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
//!
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
//! point
//!     .transform(&transform)
//!     .expect("Failed to transform point");
//! ```
//!
//! The transform convention follows the common robotics practice where data typically needs to be
//! transformed from specific sensor reference frames "up" to more general frames like the robot's
//! base frame or a global map frame.
//!
//! # Feature Flags
//!
//! - `async`: Enables async support using tokio (disabled by default)
//!
//! # Performance Considerations
//!
//! - Transform lookups are optimized for O(log n) time complexity
//! - Automatic cleanup of old transforms prevents unbounded memory growth
//! - Lock-free data structures are used where possible in the async implementation
//!
//! # Safety
//!
//! This crate uses `#![forbid(unsafe_code)]` to ensure memory safety through pure Rust implementations.
#![forbid(unsafe_code)]

pub mod core;
pub mod errors;
pub mod geometry;
pub mod time;

pub use core::Registry;
pub use geometry::{Transform, Transformable};
