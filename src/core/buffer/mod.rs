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
//! - **Static Buffers**: A buffer is either static or dynamic, decided by the first transform
//!   inserted: a transform carrying the static timestamp value (`t=0` by default) makes the buffer
//!   static, and a static buffer returns its single transform for any requested timestamp.
//!   In downstream crates, you can customize what counts as the static timestamp by implementing
//!   `TimePoint::static_timestamp()` for your timestamp type, in case the `t=0` definition causes
//!   conflicts. A sensible alternative is handling `t=u64::MAX` as a static timestamp.
//!
//! - **Automatic Expiration of Transforms**:
//!   - This feature is available only when the `std` feature is enabled.
//!   - On every insert of a dynamic transform, entries older than `max_age` relative to the
//!     latest inserted timestamp are removed automatically.
//!   - This ensures that the buffer does not grow indefinitely and only retains relevant transforms
//!     within the specified duration.
//!   - The `no_std` variant requires manual cleanup through the `delete_before` method.
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
//! let translation = Vector3::new(1.0, 2.0, 3.0);
//! let rotation = Quaternion::identity();
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
//! buffer.insert(transform).unwrap();
//!
//! let result = buffer.get(&timestamp);
//! match result {
//!     Ok(transform) => println!("Transform found: {transform:?}"),
//!     Err(_) => println!("No transform available"),
//! }
//! ```

use crate::{
    geometry::Transform,
    time::{TimePoint, Timestamp},
};
use alloc::collections::BTreeMap;
pub use error::BufferError;
mod error;

#[cfg(feature = "std")]
use core::time::Duration;

type NearestTransforms<'a, T> = (
    Option<(&'a T, &'a Transform<T>)>,
    Option<(&'a T, &'a Transform<T>)>,
);

/// A buffer that stores transforms ordered by timestamps.
///
/// The `Buffer` struct is designed to manage a collection of transforms,
/// each associated with a timestamp. It uses a binary tree to efficiently
/// store and retrieve transforms based on their timestamps.
///
/// A buffer is either static or dynamic, determined by the first transform
/// inserted into an empty buffer: a transform carrying the static timestamp
/// value (`t=0` by default) makes the buffer static. Later inserts of the
/// opposite kind are rejected with `BufferError::StaticDynamicConflict`.
///
/// When the `std` feature is enabled, entries older than `max_age` relative
/// to the latest inserted timestamp are removed automatically on insert.
pub struct Buffer<T = Timestamp>
where
    T: TimePoint,
{
    data: BTreeMap<T, Transform<T>>,
    #[cfg(feature = "std")]
    max_age: Duration,
    #[cfg(feature = "std")]
    latest_timestamp: Option<T>,
    is_static: bool,
}

impl<T> Buffer<T>
where
    T: TimePoint,
{
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
    /// let buffer: Buffer = Buffer::new();
    /// ```
    #[cfg(not(feature = "std"))]
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            is_static: false,
        }
    }

    /// Creates a new `Buffer` in a `std` environment with a specified `max_age`.
    ///
    /// Entries older than `max_age` relative to the latest inserted timestamp
    /// are removed automatically whenever a dynamic transform is inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::core::Buffer;
    ///
    /// let max_age = Duration::from_secs(10);
    /// let buffer: Buffer = Buffer::new(max_age);
    /// ```
    #[cfg(feature = "std")]
    #[must_use]
    pub fn new(max_age: Duration) -> Self {
        Self {
            data: BTreeMap::new(),
            max_age,
            latest_timestamp: None,
            is_static: false,
        }
    }

    /// Adds a transform to the buffer.
    ///
    /// The first transform inserted into an empty buffer determines whether
    /// the buffer is static (timestamp equal to `T::static_timestamp()`) or
    /// dynamic. Subsequent inserts must be of the same kind.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::StaticDynamicConflict` if the transform's kind
    /// (static or dynamic) does not match the transforms already stored in
    /// this buffer. Mixing the two would silently corrupt interpolation, as
    /// the static timestamp would be treated as a regular data point.
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
    /// let translation = Vector3::new(1.0, 2.0, 3.0);
    /// let rotation = Quaternion::identity();
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
    /// buffer.insert(transform).unwrap();
    /// ```
    pub fn insert(
        &mut self,
        transform: Transform<T>,
    ) -> Result<(), BufferError> {
        let timestamp = transform.timestamp;
        let is_static = timestamp.is_static();

        if self.data.is_empty() {
            self.is_static = is_static;
        } else if self.is_static != is_static {
            return Err(BufferError::StaticDynamicConflict);
        }

        self.data.insert(timestamp, transform);

        #[cfg(feature = "std")]
        if !self.is_static {
            self.latest_timestamp = Some(match self.latest_timestamp {
                Some(current_latest) if current_latest > timestamp => current_latest,
                _ => timestamp,
            });
            self.delete_expired();
        };

        Ok(())
    }

    /// Retrieves a transform from the buffer at the specified timestamp.
    ///
    /// # Errors
    ///
    /// This function returns a `BufferError::NoTransformAvailable` if:
    /// - The buffer is static and no transform is available at the static timestamp value.
    /// - There are no transforms available to interpolate between for the given timestamp.
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
    /// # let translation = Vector3::new(1.0, 2.0, 3.0);
    /// # let rotation = Quaternion::identity();
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
    /// buffer.insert(transform).unwrap();
    ///
    /// let result = buffer.get(&timestamp);
    /// match result {
    ///     Ok(transform) => println!("Transform found: {transform:?}"),
    ///     Err(_) => println!("No transform available"),
    /// }
    /// ```
    pub fn get(
        &self,
        timestamp: &T,
    ) -> Result<Transform<T>, BufferError> {
        if self.is_static {
            match self.data.get(&T::static_timestamp()) {
                Some(tf) => return Ok(tf.clone()),
                None => return Err(BufferError::NoTransformAvailable),
            }
        }

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
    /// timestamp lower than the given timestamp.
    pub fn delete_before(
        &mut self,
        timestamp: T,
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
        timestamp: &T,
    ) -> NearestTransforms<'_, T> {
        let before = self.data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self.data.range(timestamp..).next();
        (before, after)
    }

    /// Removes expired transforms from the buffer based on the `max_age`.
    ///
    /// This function deletes all transforms from the buffer that have a
    /// timestamp older than `(latest inserted timestamp - max_age)`.
    #[cfg(feature = "std")]
    fn delete_expired(&mut self) {
        if let Some(latest_timestamp) = self.latest_timestamp {
            let timestamp_threshold = latest_timestamp.checked_sub(self.max_age);
            if let Ok(threshold) = timestamp_threshold {
                self.data.retain(|&k, _| k >= threshold);
            }
        }
    }
}

#[cfg(not(feature = "std"))]
impl<T> Default for Buffer<T>
where
    T: TimePoint,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
