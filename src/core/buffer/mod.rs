//! A module for managing a buffer of transforms with timestamps.
//!
//! This module provides the `Buffer` struct, which is designed to store and manage
//! a collection of transforms, each associated with a timestamp. The buffer uses
//! a binary tree to efficiently store and retrieve transforms based on their timestamps.
//!
//! # Features
//!
//! - **Store Transforms with Timestamps**: The `Buffer` allows you to store multiple transforms,
//!   each associated with a unique timestamp. This is useful for applications that require
//!   time-based transformations, such as robotics, animation, and simulations.
//!
//! - **Retrieve Transforms with Interpolation**: You can retrieve transforms at specific timestamps.
//!   If an exact match is not found, the buffer can interpolate between the nearest transforms to
//!   provide an estimated transform at the requested timestamp.
//!
//! - **Static Lookup Mode**: The buffer supports a static lookup mode. When a timestamp set to zero
//!   is supplied, the buffer will return a static transform if available. This is useful for
//!   scenarios where a constant transform is needed regardless of the timestamp.
//!
//! - **Automatic Expiration of Transforms**:
//!   - This feature is available only when the `std` feature is enabled.
//!   - The buffer can automatically remove expired transforms based on a specified ``max_age``.
//!   - This ensures that the buffer does not grow indefinitely and only retains relevant transforms
//!     within the specified duration.
//!   - the ``no_std`` variant requires manual cleanup through the `delete_before` method.
//!
//! # Examples
//!
//! ```
//! # #[cfg(feature = "std")]
//! use core::time::Duration;
//! use transforms::{
//!     core::Buffer,
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//! };
//!
//! # #[cfg(not(feature = "std"))]
//! let mut buffer = Buffer::new();
//!
//! # #[cfg(feature = "std")]
//! let max_age = Duration::from_secs(10);
//! # #[cfg(feature = "std")]
//! let mut buffer = Buffer::new(max_age);
//!
//! let translation = Vector3 {
//!     x: 1.0,
//!     y: 2.0,
//!     z: 3.0,
//! };
//! let rotation = Quaternion {
//!     w: 1.0,
//!     x: 0.0,
//!     y: 0.0,
//!     z: 0.0,
//! };
//!
//! # #[cfg(not(feature = "std"))]
//! let timestamp = Timestamp::zero();
//! # #[cfg(feature = "std")]
//! let timestamp = Timestamp::now();
//! let parent = "a".into();
//! let child = "b".into();
//!
//! let transform = Transform {
//!     translation,
//!     rotation,
//!     timestamp,
//!     parent,
//!     child,
//! };
//!
//! buffer.insert(transform);
//!
//! let result = buffer.get(&timestamp);
//! match result {
//!     Ok(transform) => println!("Transform found: {:?}", transform),
//!     Err(_) => println!("No transform available"),
//! }
//! ```
//!
//! # Modules
//!
//! - `error`: Contains the `BufferError` type for error handling.
//!
//! # Structs
//!
//! - `Buffer`: The main struct for managing the buffer of transforms.
//!
//! # Types
//!
//! - `NearestTransforms`: A type alias for a tuple containing the nearest transforms before and after a given timestamp.

use crate::{geometry::Transform, time::Timestamp};
use alloc::collections::BTreeMap;
pub use error::BufferError;
mod error;

#[cfg(feature = "std")]
use core::time::Duration;

type NearestTransforms<'a> = (
    Option<(&'a Timestamp, &'a Transform)>,
    Option<(&'a Timestamp, &'a Transform)>,
);

/// A buffer that stores transforms ordered by timestamps.
///
/// The `Buffer` struct is designed to manage a collection of transforms,
/// each associated with a timestamp. It uses a binary tree to efficiently
/// store and retrieve transforms based on their timestamps.
///
/// # Fields
///
/// - `data`: A `BTreeMap` where each key is a `Timestamp` and each value is a `Transform`.
/// - `max_age`: This feature is available only when the `std` feature is enabled. A `Duration` that
///   defines the ``max_age`` for each entry, determining how long entries remain valid.
/// - `is_static`: A boolean flag that determines if the buffer is a static. It can be set to
///   static by supplying a timestamp set to zero.
pub struct Buffer {
    data: BTreeMap<Timestamp, Transform>,
    #[cfg(feature = "std")]
    max_age: Duration,
    is_static: bool,
}

impl Buffer {
    #[cfg(not(feature = "std"))]
    #[allow(clippy::new_without_default)]
    /// Creates a new `Buffer` in a `no_std` environment.
    ///
    /// This variant does **not** track or remove entries based on their age,
    /// because the `std` feature is disabled and we do not have access to
    /// standard library functionality.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::core::Buffer;
    /// let buffer = Buffer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            is_static: false,
        }
    }

    #[cfg(feature = "std")]
    #[allow(clippy::new_without_default)]
    #[must_use = "The Buffer should be used to store transforms."]
    /// Creates a new `Buffer` in a `std` environment with a specified `max_age`.
    ///
    /// Entries older than `max_age` can be removed automatically, depending on
    /// how you implement your cleanup logic.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::core::Buffer;
    ///
    /// let max_age = Duration::from_secs(10);
    /// let buffer = Buffer::new(max_age);
    /// ```
    pub fn new(max_age: Duration) -> Self {
        Self {
            data: BTreeMap::new(),
            max_age,
            is_static: false,
        }
    }

    /// Adds a transform to the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     core::Buffer,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut buffer = Buffer::new(Duration::from_secs(10));
    /// # #[cfg(feature = "std")]
    /// let timestamp = Timestamp::now();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut buffer = Buffer::new();
    /// # #[cfg(not(feature = "std"))]
    /// let timestamp = Timestamp::zero();
    ///
    /// let translation = Vector3 {
    ///     x: 1.0,
    ///     y: 2.0,
    ///     z: 3.0,
    /// };
    /// let rotation = Quaternion {
    ///     w: 1.0,
    ///     x: 0.0,
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// let parent = "a".into();
    /// let child = "b".into();
    ///
    /// let transform = Transform {
    ///     translation,
    ///     rotation,
    ///     timestamp,
    ///     parent,
    ///     child,
    /// };
    ///
    /// buffer.insert(transform);
    /// ```
    pub fn insert(
        &mut self,
        transform: Transform,
    ) {
        self.is_static = transform.timestamp.t == 0;
        self.data.insert(transform.timestamp, transform);

        #[cfg(feature = "std")]
        if !self.is_static {
            self.delete_expired();
        };
    }

    /// Retrieves a transform from the buffer at the specified timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     core::Buffer,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// # let mut buffer = Buffer::new(Duration::from_secs(10));
    /// # #[cfg(feature = "std")]
    /// # let timestamp = Timestamp::now();
    /// # #[cfg(not(feature = "std"))]
    /// # let mut buffer = Buffer::new();
    /// # #[cfg(not(feature = "std"))]
    /// # let timestamp = Timestamp::zero();
    /// #
    /// # let translation = Vector3 {
    /// #       x: 1.0,
    /// #       y: 2.0,
    /// #       z: 3.0,
    /// #   };
    /// # let rotation = Quaternion {
    /// #       w: 1.0,
    /// #       x: 0.0,
    /// #       y: 0.0,
    /// #       z: 0.0,
    /// #   };
    /// # let parent = "a".into();
    /// # let child = "b".into();
    /// #
    /// let transform = Transform {
    ///     translation,
    ///     rotation,
    ///     timestamp,
    ///     parent,
    ///     child,
    /// };
    ///
    /// buffer.insert(transform);
    ///
    /// let result = buffer.get(&timestamp);
    /// match result {
    ///     Ok(transform) => println!("Transform found: {:?}", transform),
    ///     Err(_) => println!("No transform available"),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns a `BufferError::NoTransformAvailable` if:
    /// - The buffer is static and no transform is available at timestamp zero.
    /// - There are no transforms available to interpolate between for the given timestamp.
    pub fn get(
        &self,
        timestamp: &Timestamp,
    ) -> Result<Transform, BufferError> {
        if self.is_static {
            match self.data.get(&Timestamp { t: 0 }) {
                Some(tf) => return Ok(tf.clone()),
                None => return Err(BufferError::NoTransformAvailable),
            }
        };

        let (before, after) = self.get_nearest(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => {
                Ok(Transform::interpolate(before.1, after.1, *timestamp)?)
            }
            _ => Err(BufferError::NoTransformAvailable),
        }
    }

    /// Removes transforms from the buffer based on the given threshold.
    ///
    /// This function deletes all transforms from the buffer that have a
    /// timestamp lower than the input argument.
    ///
    /// # Fields
    ///
    /// - `timestamp`: the time to compare all entries in the buffer with.
    pub fn delete_before(
        &mut self,
        timestamp: Timestamp,
    ) {
        self.data.retain(|&k, _| k >= timestamp);
    }

    /// Retrieves the nearest transforms before and after the given timestamp.
    ///
    /// This function returns a tuple containing the nearest transform before
    /// and the nearest transform after the specified timestamp. If the exact
    /// timestamp exists, both elements of the tuple will be the same.
    fn get_nearest(
        &self,
        timestamp: &Timestamp,
    ) -> NearestTransforms {
        let before = self.data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self.data.range(timestamp..).next();
        (before, after)
    }

    /// Removes expired transforms from the buffer based on the ``max_age``.
    ///
    /// This function deletes all transforms from the buffer that have a
    /// timestamp older than the current time minus the ``max_age``.
    #[cfg(feature = "std")]
    fn delete_expired(&mut self) {
        let timestamp_threshold = Timestamp::now() - self.max_age;
        if let Ok(t) = timestamp_threshold {
            self.data.retain(|&k, _| k >= t);
        }
    }
}

#[cfg(test)]
mod tests;
