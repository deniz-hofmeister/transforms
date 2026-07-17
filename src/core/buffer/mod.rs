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
//!   - Buffers created with `Buffer::with_max_age` remove entries older than `max_age`
//!     relative to the latest inserted timestamp on every insert of a dynamic transform.
//!   - This ensures that the buffer does not grow indefinitely and only retains relevant
//!     transforms within the specified duration.
//!   - Buffers created with `Buffer::new` never expire entries; use the `delete_before`
//!     method for manual cleanup. Static transforms never expire and survive manual
//!     cleanup.
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
//! let mut buffer = Buffer::with_max_age(max_age);
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
use alloc::{collections::BTreeMap, string::String};
use core::time::Duration;
pub use error::BufferError;
mod error;

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
/// The first insert also pins the buffer's parent and child frames: every
/// later insert must carry the same pair, so a buffer stores the history of
/// exactly one parent-child relationship. Re-parenting is rejected with
/// `BufferError::ReparentingNotSupported`, and a transform for a different
/// child frame with `BufferError::ChildFrameMismatch`.
///
/// When constructed with [`Buffer::with_max_age`], entries older than
/// `max_age` relative to the latest inserted timestamp are removed
/// automatically on insert. A buffer created with [`Buffer::new`] never
/// expires entries; use [`Buffer::delete_before`] for manual cleanup.
#[derive(Debug)]
pub struct Buffer<T = Timestamp>
where
    T: TimePoint,
{
    data: BTreeMap<T, Transform<T>>,
    max_age: Option<Duration>,
    latest_timestamp: Option<T>,
    is_static: bool,
    parent: Option<String>,
    child: Option<String>,
}

impl<T> Buffer<T>
where
    T: TimePoint,
{
    /// Creates a new `Buffer` without automatic expiry.
    ///
    /// Entries are kept until removed manually with
    /// [`Buffer::delete_before`].
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::core::Buffer;
    /// let buffer: Buffer = Buffer::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            max_age: None,
            latest_timestamp: None,
            is_static: false,
            parent: None,
            child: None,
        }
    }

    /// Creates a new `Buffer` with automatic expiry after `max_age`.
    ///
    /// Entries older than `max_age` relative to the latest inserted timestamp
    /// are removed automatically whenever a dynamic transform is inserted.
    /// `Duration::ZERO` therefore retains only the newest sample.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::core::Buffer;
    ///
    /// let max_age = Duration::from_secs(10);
    /// let buffer: Buffer = Buffer::with_max_age(max_age);
    /// ```
    #[must_use]
    pub fn with_max_age(max_age: Duration) -> Self {
        Self {
            data: BTreeMap::new(),
            max_age: Some(max_age),
            latest_timestamp: None,
            is_static: false,
            parent: None,
            child: None,
        }
    }

    /// Returns the buffer's parent frame, pinned by the first insert.
    ///
    /// `None` for a buffer that has never held a transform. The parent stays
    /// pinned even if all entries are removed; drop the whole buffer
    /// (`Registry::remove_frame`) to release it.
    #[must_use]
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    /// Returns `true` if the buffer holds no transforms.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Adds a transform to the buffer.
    ///
    /// The transform is validated first: it must have finite components and
    /// a unit rotation (see [`Transform::validate`]). The first transform
    /// inserted into an empty buffer determines whether the buffer is static
    /// (timestamp equal to `T::static_timestamp()`) or dynamic. Subsequent
    /// inserts must be of the same kind.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::TransformError` wrapping
    /// `TransformError::NonUnitRotation` or `TransformError::NonFiniteValues`
    /// if the transform fails validation — storing such a transform would
    /// make later lookups return silently wrong results.
    ///
    /// Returns `BufferError::StaticDynamicConflict` if the transform's kind
    /// (static or dynamic) does not match the transforms already stored in
    /// this buffer. Mixing the two would silently corrupt interpolation, as
    /// the static timestamp would be treated as a regular data point.
    ///
    /// Returns `BufferError::SelfReferentialFrame` if the transform's parent
    /// and child are the same frame,
    /// `BufferError::ReparentingNotSupported` if the buffer's parent frame
    /// (pinned by the first insert) differs from the transform's parent, and
    /// `BufferError::ChildFrameMismatch` if the buffer's child frame (pinned
    /// the same way) differs from the transform's child — accepting a second
    /// child frame would silently overwrite a static transform or corrupt
    /// interpolation between dynamic ones.
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
    /// let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
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
        transform.validate()?;

        if transform.parent == transform.child {
            return Err(BufferError::SelfReferentialFrame);
        }
        if let Some(parent) = &self.parent {
            if *parent != transform.parent {
                return Err(BufferError::ReparentingNotSupported(parent.clone()));
            }
        } else {
            self.parent = Some(transform.parent.clone());
        }
        if let Some(child) = &self.child {
            if *child != transform.child {
                return Err(BufferError::ChildFrameMismatch(child.clone()));
            }
        } else {
            self.child = Some(transform.child.clone());
        }

        let timestamp = transform.timestamp;
        let is_static = timestamp.is_static();

        if self.data.is_empty() {
            self.is_static = is_static;
        } else if self.is_static != is_static {
            return Err(BufferError::StaticDynamicConflict);
        }

        self.data.insert(timestamp, transform);

        if !self.is_static {
            self.latest_timestamp = Some(match self.latest_timestamp {
                Some(current_latest) if current_latest > timestamp => current_latest,
                _ => timestamp,
            });
            self.delete_expired();
        }

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
    /// Returns `BufferError::TransformError` if interpolating between the two
    /// neighboring samples fails. With both frames pinned at insertion, this
    /// is only reachable through timestamp arithmetic: a span between the
    /// neighboring samples too large to represent as a `Duration`
    /// (`TimeError::DurationOverflow`).
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
    /// # let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
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

    /// Removes dynamic transforms older than the given timestamp.
    ///
    /// This function deletes all transforms from the buffer that have a
    /// timestamp lower than the given timestamp. Static buffers are left
    /// untouched: a static transform is valid for all time, so cleaning it up
    /// by timestamp would silently destroy it.
    pub fn delete_before(
        &mut self,
        timestamp: T,
    ) {
        if self.is_static {
            return;
        }
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
    /// timestamp older than `(latest inserted timestamp - max_age)`. Buffers
    /// without a configured `max_age` never expire entries.
    fn delete_expired(&mut self) {
        if let (Some(max_age), Some(latest_timestamp)) = (self.max_age, self.latest_timestamp) {
            if let Ok(threshold) = latest_timestamp.checked_sub(max_age) {
                self.data.retain(|&k, _| k >= threshold);
            }
        }
    }
}

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
