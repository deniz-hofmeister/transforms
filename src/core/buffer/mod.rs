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
//! - **Automatic Expiration of Transforms**: The buffer can automatically remove expired transforms
//!   based on a specified max_age. This ensures that the buffer does not grow indefinitely
//!   and only retains relevant transforms within the specified duration.
//!
//! - **Retrieve Transforms with Interpolation**: You can retrieve transforms at specific timestamps.
//!   If an exact match is not found, the buffer can interpolate between the nearest transforms to
//!   provide an estimated transform at the requested timestamp.
//!
//! - **Static Lookup Mode**: The buffer supports a static lookup mode. When a timestamp with
//!   nanoseconds set to zero is supplied, the buffer will return a static transform if available.
//!   This is useful for scenarios where a constant transform is needed regardless of the timestamp.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use transforms::{
//!     core::Buffer,
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::Timestamp,
//! };
//!
//! let max_age = Duration::from_secs(10);
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
//! let timestamp = Timestamp { time: 0 };
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
use core::time::Duration;
pub use error::BufferError;
mod error;

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
/// - `max_age`: A `Duration` that defines the max_age for each entry, determining how long
///   entries remain valid.
/// - `is_static`: A boolean flag that, when set to true, converts the buffer to a static
///   lookup if a timestamp with nanoseconds set to zero is supplied. Any
pub struct Buffer {
    data: BTreeMap<Timestamp, Transform>,
    max_age: Duration,
    is_static: bool,
}

impl Buffer {
    /// Creates a new buffer with the specified max_age.
    /// Entries older than the max_age will automatically be removed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use transforms::core::Buffer;
    /// use std::time::Duration;
    ///
    /// let max_age = Duration::from_secs(10);
    /// let mut buffer = Buffer::new(max_age);
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
    /// use std::time::Duration;
    /// # use transforms::{
    /// #     core::Buffer,
    /// #     geometry::{Quaternion, Transform, Vector3},
    /// #     time::Timestamp,
    /// # };
    ///
    /// let max_age = Duration::from_secs(10);
    /// let mut buffer = Buffer::new(max_age);
    ///
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
    /// # let timestamp = Timestamp::now();
    /// # let parent = "a".into();
    /// # let child = "b".into();
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

        if !self.is_static {
            self.delete_expired();
        };
    }

    /// Retrieves a transform from the buffer at the specified timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use transforms::{
    ///     core::Buffer,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let max_age = Duration::from_secs(10);
    /// let mut buffer = Buffer::new(max_age);
    ///
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
    /// # let timestamp = Timestamp::now();
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
            (Some(before), Some(after)) => Ok(Transform::interpolate(
                before.1.clone(),
                after.1.clone(),
                *timestamp,
            )?),
            _ => Err(BufferError::NoTransformAvailable),
        }
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

    /// Removes expired transforms from the buffer based on the max_age.
    ///
    /// This function deletes all transforms from the buffer that have a
    /// timestamp older than the current time minus the max_age.
    ///
    /// Only available when the `std` feature is enabled.
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
