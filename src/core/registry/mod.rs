//! # Registry Module
//!
//! The `registry` module provides the core functionality for managing transforms between different coordinate frames. It maintains a collection of transforms and offers methods to add, retrieve, and chain these transforms.
//!
//! ## Features
//!
//! - **Static Transforms**: The registry can handle static transforms by using the static timestamp value (`t=0` by default).
//! - **Dynamic Transforms**: Supports dynamic transforms with timestamps to handle time-varying transformations.
//! - **Interpolation**: Interpolates between transforms if a requested timestamp lies between two known transforms.
//! - **Automatic Buffer Cleanup**: Automatically cleans up old transforms based on the `max_age` parameter when the `std` feature is enabled.
//!
//! ## Usage
//!
//! The `Registry` struct is the main entry point for interacting with the registry.
//!
//! ## Time type selection
//!
//! `Registry` defaults to `Timestamp`, so `Registry::new(...)` is equivalent to
//! `Registry::<Timestamp>::new(...)`.
//!
//! You can use custom timestamps by implementing `time::TimePoint` and then
//! constructing `Registry::<CustomTimestamp>::new(...)`.
//!
//! With the `std` feature enabled, `std::time::SystemTime` already implements
//! `TimePoint`, so `Registry::<SystemTime>::new(Duration::from_secs(...))`
//! works out of the box.
//!
//! # Examples
//!
//! ```rust
//! # {
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
//! let t1 = Timestamp::now();
//!
//! # #[cfg(not(feature = "std"))]
//! let mut registry = Registry::new();
//! # #[cfg(not(feature = "std"))]
//! let t1 = Timestamp::zero();
//!
//! let t2 = t1.clone();
//!
//! // Define a transform from frame "a" to frame "b"
//! let t_a_b_1 = Transform {
//!     translation: Vector3 {
//!         x: 1.0,
//!         y: 0.0,
//!         z: 0.0,
//!     },
//!     rotation: Quaternion {
//!         w: 1.0,
//!         x: 0.0,
//!         y: 0.0,
//!         z: 0.0,
//!     },
//!     timestamp: t1,
//!     parent: "a".into(),
//!     child: "b".into(),
//! };
//!
//! // For validation
//! let t_a_b_2 = t_a_b_1.clone();
//!
//! // Add the transform to the registry
//! registry.add_transform(t_a_b_1);
//!
//! // Retrieve the transform from "a" to "b"
//! let result = registry.get_transform("a", "b", t2);
//! assert!(result.is_ok());
//! assert_eq!(result.unwrap(), t_a_b_2);
//! # }
//! ```
//!
//! ## Structs
//!
//! ### `Registry`
//!
//! The `Registry` struct provides methods to add and retrieve transforms between frames.
//!
//! #### Methods
//!
//! - `new(max_age: Duration) -> Self`
//!   - Creates a new `Registry` with the specified ``max_age`` duration.
//!   - **Arguments**
//!     - `max_age`: The duration for which transforms are considered valid.
//!   - **Returns**
//!     - A new instance of `Registry`.
//!
//! - `add_transform(&mut self, t: Transform<T>)`
//!   - Adds a transform to the registry.
//!   - **Arguments**
//!     - `t`: The transform to add.
//!
//! - `get_transform(&self, from: &str, to: &str, timestamp: T) -> Result<Transform<T>, TransformError>`
//!   - Retrieves a transform from the registry.
//!   - **Arguments**
//!     - `from`: The source frame.
//!     - `to`: The destination frame.
//!     - `timestamp`: The timestamp for which the transform is requested.
//!   - **Errors**
//!     - Returns a `TransformError` if the transform cannot be found.

use crate::{
    core::Buffer,
    errors::TransformError,
    geometry::{Localized, Quaternion, Transform, Vector3},
    time::{TimePoint, Timestamp},
};
use alloc::{collections::VecDeque, string::String};
use hashbrown::{hash_map::Entry, HashMap};

mod error;

#[cfg(feature = "std")]
use core::time::Duration;

/// A registry for managing transforms between different frames. It can
/// traverse the parent-child tree and calculate the final transform.
/// It will interpolate between two entries if a time is requested that
/// lies in between.
///
/// The `Registry` struct provides methods to add and retrieve transforms
/// between frames
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Quaternion, Transform, Vector3},
///     time::Timestamp,
///     Registry,
/// };
///
/// # #[cfg(feature = "std")]
/// use core::time::Duration;
/// # #[cfg(feature = "std")]
/// let mut registry = Registry::new(Duration::from_secs(60));
/// # #[cfg(feature = "std")]
/// let t1 = Timestamp::now();
///
/// # #[cfg(not(feature = "std"))]
/// let mut registry = Registry::new();
/// # #[cfg(not(feature = "std"))]
/// let t1 = Timestamp::zero();
///
/// let t2 = t1.clone();
///
/// // Define a transform from frame "a" to frame "b"
/// let t_a_b_1 = Transform {
///     translation: Vector3 {
///         x: 1.0,
///         y: 0.0,
///         z: 0.0,
///     },
///     rotation: Quaternion {
///         w: 1.0,
///         x: 0.0,
///         y: 0.0,
///         z: 0.0,
///     },
///     timestamp: t1,
///     parent: "a".into(),
///     child: "b".into(),
/// };
///
/// // For validation
/// let t_a_b_2 = t_a_b_1.clone();
///
/// // Add the transform to the registry
/// registry.add_transform(t_a_b_1);
///
/// // Retrieve the transform from "a" to "b"
/// let result = registry.get_transform("a", "b", t2);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), t_a_b_2);
/// ```
pub struct Registry<T = Timestamp>
where
    T: TimePoint,
{
    pub data: HashMap<String, Buffer<T>>,
    #[cfg(feature = "std")]
    max_age: Duration,
}

impl<T> Registry<T>
where
    T: TimePoint,
{
    #[cfg(feature = "std")]
    #[allow(clippy::new_without_default)]
    #[must_use = "The Registry should be used to manage transforms."]
    /// Creates a new `Registry` with the specified ``max_age`` duration.
    ///
    /// # Arguments
    ///
    /// * `max_age` - The duration for which transforms are considered valid.
    ///
    /// # Returns
    ///
    /// A new instance of `Registry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::{time::Timestamp, Registry};
    ///
    /// let mut registry = Registry::<Timestamp>::new(Duration::from_secs(60));
    /// ```
    pub fn new(max_age: Duration) -> Self {
        Self {
            data: HashMap::new(),
            max_age,
        }
    }
    #[allow(clippy::new_without_default)]
    #[cfg(not(feature = "std"))]
    #[must_use = "The Registry should be used to manage transforms."]
    /// Creates a new `Registry`.
    ///
    /// # Returns
    ///
    /// A new instance of `Registry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{time::Timestamp, Registry};
    ///
    /// let registry = Registry::<Timestamp>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Adds a transform to the registry.
    ///
    /// # Arguments
    ///
    /// * `t` - The transform to add.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{geometry::Transform, time::Timestamp, Registry};
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::<Timestamp>::new(Duration::from_secs(60));
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::<Timestamp>::new();
    ///
    /// let transform = Transform::identity();
    ///
    /// registry.add_transform(transform);
    /// ```
    pub fn add_transform(
        &mut self,
        t: Transform<T>,
    ) {
        #[cfg(not(feature = "std"))]
        Self::process_add_transform(t, &mut self.data);
        #[cfg(feature = "std")]
        Self::process_add_transform(t, &mut self.data, self.max_age);
    }

    /// Retrieves a transform from the registry.
    ///
    /// # Arguments
    ///
    /// * `from` - The source frame.
    /// * `to` - The destination frame.
    /// * `timestamp` - The timestamp for which the transform is requested.
    ///
    /// # Errors
    ///
    /// Returns a `TransformError` if the transform cannot be found.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    ///     Registry,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::new(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// let t1 = Timestamp::zero();
    ///
    /// let t2 = t1.clone();
    ///
    /// // Define a transform from frame "a" to frame "b"
    /// let t_a_b_1 = Transform {
    ///     translation: Vector3 {
    ///         x: 1.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: t1,
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// // For validation
    /// let t_a_b_2 = t_a_b_1.clone();
    ///
    /// registry.add_transform(t_a_b_1);
    ///
    /// let result = registry.get_transform("a", "b", t2);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), t_a_b_2);
    /// ```
    pub fn get_transform(
        &self,
        from: &str,
        to: &str,
        timestamp: T,
    ) -> Result<Transform<T>, TransformError> {
        Self::process_get_transform(from, to, timestamp, &self.data)
    }

    /// Retrieves a transform for a specific value into `target_frame`.
    ///
    /// The source frame and timestamp are taken from the value.
    ///
    /// If the value is already in `target_frame`, this returns an identity transform
    /// with `parent == child == target_frame` and the value's timestamp.
    ///
    /// # Errors
    ///
    /// Returns a `TransformError` if a transform cannot be resolved.
    pub fn get_transform_for<U>(
        &self,
        value: &U,
        target_frame: &str,
    ) -> Result<Transform<T>, TransformError>
    where
        U: Localized<T>,
    {
        if value.frame() == target_frame {
            return Ok(Transform {
                translation: Vector3::new(0.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: value.timestamp(),
                parent: target_frame.into(),
                child: target_frame.into(),
            });
        }

        self.get_transform(target_frame, value.frame(), value.timestamp())
    }

    /// Retrieves a transform between two frames at different timestamps using a fixed frame.
    ///
    /// This is the "time travel" API that allows you to get the transform from a source frame
    /// at one time to a target frame at a different time. This is useful for scenarios like
    /// tracking an object that was detected on a moving platform (e.g., a conveyor belt) and
    /// getting its current position in a static world frame.
    ///
    /// The algorithm works by:
    /// 1. Computing the transform from `source_frame` to `fixed_frame` at `source_time`
    /// 2. Computing the transform from `fixed_frame` to `target_frame` at `target_time`
    /// 3. Chaining these transforms together
    ///
    /// # Arguments
    ///
    /// * `target_frame` - The destination frame for the transform
    /// * `target_time` - The time at which to evaluate the target frame's position
    /// * `source_frame` - The source frame for the transform
    /// * `source_time` - The time at which to evaluate the source frame's position
    /// * `fixed_frame` - A frame that does not change over time, used as an intermediate
    ///   reference point (typically a world or map frame)
    ///
    /// # Safety
    ///
    /// **The caller is responsible for ensuring that `fixed_frame` is actually stationary
    /// between `source_time` and `target_time`.** Passing a frame that moves between the
    /// two timestamps will produce a mathematically meaningless result without any error.
    /// Root frames (e.g., `"world"`, `"map"`) that have no parent are always safe choices.
    ///
    /// # Errors
    ///
    /// Returns a `TransformError` if any of the required transforms cannot be found
    /// at the specified times.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    ///     Registry,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::new(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    /// # #[cfg(feature = "std")]
    /// let t2 = (t1 + Duration::from_secs(1)).unwrap();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// let t1 = Timestamp::zero();
    /// # #[cfg(not(feature = "std"))]
    /// let t2 = Timestamp { t: 1_000_000_000 };
    ///
    /// // Tree: fixed -> a -> b
    ///
    /// // fixed -> a at t1: a is at x=1
    /// registry.add_transform(Transform {
    ///     translation: Vector3 {
    ///         x: 1.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: t1,
    ///     parent: "fixed".into(),
    ///     child: "a".into(),
    /// });
    ///
    /// // fixed -> a at t2: a has moved to x=2
    /// registry.add_transform(Transform {
    ///     translation: Vector3 {
    ///         x: 2.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: t2,
    ///     parent: "fixed".into(),
    ///     child: "a".into(),
    /// });
    ///
    /// // a -> b at t1: b is at y=1 relative to a
    /// registry.add_transform(Transform {
    ///     translation: Vector3 {
    ///         x: 0.0,
    ///         y: 1.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: t1,
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// });
    ///
    /// // Express b-at-t1 in a-at-t2, using "fixed" as the stationary reference
    /// let result = registry.get_transform_at(
    ///     "a",     // target_frame
    ///     t2,      // target_time
    ///     "b",     // source_frame
    ///     t1,      // source_time
    ///     "fixed", // fixed_frame
    /// );
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn get_transform_at(
        &self,
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
    ) -> Result<Transform<T>, TransformError> {
        Self::process_get_transform_at(
            target_frame,
            target_time,
            source_frame,
            source_time,
            fixed_frame,
            &self.data,
        )
    }

    /// Removes transforms from every buffer based on the given threshold.
    ///
    /// Iterates over all buffers and deletes all entries with a
    /// timestamp lower than the input argument.
    ///
    /// # Fields
    ///
    /// - `timestamp`: the time to compare all entries in the buffer with.
    pub fn delete_transforms_before(
        &mut self,
        timestamp: T,
    ) {
        for buffer in self.data.values_mut() {
            buffer.delete_before(timestamp);
        }
    }

    #[cfg(not(feature = "std"))]
    /// Adds a transform to the data buffer.
    ///
    /// # Arguments
    ///
    /// * `t` - The transform to be added to the registry
    /// * `data` - Mutable reference to the data buffer where transforms are stored
    fn process_add_transform(
        t: Transform<T>,
        data: &mut HashMap<String, Buffer<T>>,
    ) {
        match data.entry(t.child.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert(t);
            }
            Entry::Vacant(entry) => {
                let buffer = Buffer::new();
                let buffer = entry.insert(buffer);
                buffer.insert(t);
            }
        }
    }

    #[cfg(feature = "std")]
    /// Adds a transform to the data buffer.
    ///
    /// # Arguments
    ///
    /// * `t` - The transform to be added to the registry
    /// * `data` - Mutable reference to the data buffer where transforms are stored
    /// * `max_age` - The maximum duration for which transforms are considered valid
    fn process_add_transform(
        t: Transform<T>,
        data: &mut HashMap<String, Buffer<T>>,
        max_age: Duration,
    ) {
        match data.entry(t.child.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert(t);
            }
            Entry::Vacant(entry) => {
                let buffer = Buffer::new(max_age);
                let buffer = entry.insert(buffer);
                buffer.insert(t);
            }
        }
    }

    /// Retrieves and computes the transform between two frames at a specific timestamp.
    ///
    /// # Arguments
    ///
    /// * `from` - The source frame identifier
    /// * `to` - The target frame identifier
    /// * `timestamp` - The time for which the transform is requested
    /// * `data` - Reference to the data buffer containing transforms
    ///
    /// # Errors
    ///
    /// * `TransformError::NotFound` - If no valid transform chain is found between the specified frames
    /// * `TransformError::TransformTreeEmpty` - If the combined transform chain is empty after processing
    /// * Other variants of `TransformError` resulting from transform operations
    fn process_get_transform(
        from: &str,
        to: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<Transform<T>, TransformError> {
        let from_chain = Self::get_transform_chain(from, to, timestamp, data);
        let to_chain = Self::get_transform_chain(to, from, timestamp, data);

        match (from_chain, to_chain) {
            (Ok(mut from_chain), Ok(mut to_chain)) => {
                Self::truncate_at_common_parent(&mut from_chain, &mut to_chain);
                Self::reverse_and_invert_transforms(&mut to_chain)?;
                Self::combine_transforms(from_chain, to_chain)
            }
            (Ok(from_chain), Err(_)) => Self::combine_transforms(from_chain, VecDeque::new()),
            (Err(_), Ok(mut to_chain)) => {
                Self::reverse_and_invert_transforms(&mut to_chain)?;
                Self::combine_transforms(VecDeque::new(), to_chain)
            }
            (Err(_), Err(_)) => Err(TransformError::NotFound(from.into(), to.into())),
        }
    }

    /// Retrieves a transform between two frames at different timestamps using a fixed frame.
    ///
    /// This implements "time travel" by:
    /// 1. Getting the transform from `source_frame` to `fixed_frame` at `source_time`
    /// 2. Getting the transform from `fixed_frame` to `target_frame` at `target_time`
    /// 3. Chaining these together
    ///
    /// # Arguments
    ///
    /// * `target_frame` - The destination frame
    /// * `target_time` - The time at which to evaluate the target frame
    /// * `source_frame` - The source frame
    /// * `source_time` - The time at which to evaluate the source frame
    /// * `fixed_frame` - A frame that doesn't change over time (e.g., "world")
    /// * `data` - Reference to the transform data buffer
    ///
    /// # Errors
    ///
    /// * `TransformError::NotFound` - If no valid transform chain is found between a frame and the fixed frame
    /// * Other variants of `TransformError` resulting from transform operations
    fn process_get_transform_at(
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<Transform<T>, TransformError> {
        // Following tf2's algorithm:
        // 1. Get transform expressing source_frame in fixed_frame at source_time
        // 2. Get transform expressing target_frame in fixed_frame at target_time
        // 3. Compute: T_target_to_fixed.inverse() * T_source_to_fixed

        // Step 1: Get transform expressing source_frame in fixed_frame at source_time
        // process_get_transform(parent, child) returns "child expressed in parent"
        // So process_get_transform(fixed, source) returns "source expressed in fixed"
        let mut source_to_fixed = if source_frame == fixed_frame {
            // Identity transform if source_frame is the same as fixed_frame
            Transform {
                translation: crate::geometry::Vector3::new(0.0, 0.0, 0.0),
                rotation: crate::geometry::Quaternion::identity(),
                timestamp: source_time,
                parent: fixed_frame.into(),
                child: source_frame.into(),
            }
        } else {
            Self::process_get_transform(fixed_frame, source_frame, source_time, data)?
        };

        // Step 2: Get transform expressing target_frame in fixed_frame at target_time
        // process_get_transform(fixed, target) returns "target expressed in fixed"
        let mut target_to_fixed = if target_frame == fixed_frame {
            // Identity transform if target_frame is the same as fixed_frame
            Transform {
                translation: crate::geometry::Vector3::new(0.0, 0.0, 0.0),
                rotation: crate::geometry::Quaternion::identity(),
                timestamp: target_time,
                parent: fixed_frame.into(),
                child: target_frame.into(),
            }
        } else {
            Self::process_get_transform(fixed_frame, target_frame, target_time, data)?
        };

        // Since both transforms are expressed relative to a fixed frame, we can simply multiply them
        // with their timestamps set to the static value.
        source_to_fixed.timestamp = T::static_timestamp();
        target_to_fixed.timestamp = T::static_timestamp();

        let mut result = (target_to_fixed.inverse()? * source_to_fixed)?;
        // We set the final timestamp to the target_time as per the API contract.
        result.timestamp = target_time;

        Ok(result)
    }

    /// Constructs a chain of transforms from a starting frame to a target frame at a given timestamp.
    ///
    /// # Arguments
    ///
    /// * `from` - The starting frame identifier
    /// * `to` - The target frame identifier
    /// * `timestamp` - The time for which the transforms are requested
    /// * `data` - Reference to the data buffer containing transforms
    ///
    /// # Errors
    ///
    /// Returns `TransformError::NotFound` if no transform chain can be found from the starting frame to the target frame
    fn get_transform_chain(
        from: &str,
        to: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<VecDeque<Transform<T>>, TransformError> {
        let mut transforms = VecDeque::new();
        let mut current_frame: String = from.into();

        while let Some(frame_buffer) = data.get(&current_frame) {
            match frame_buffer.get(&timestamp) {
                Ok(tf) => {
                    transforms.push_back(tf.clone());
                    current_frame.clone_from(&tf.parent);
                }
                Err(_) => break,
            }
        }

        if transforms.is_empty() {
            Err(TransformError::NotFound(from.into(), to.into()))
        } else {
            Ok(transforms)
        }
    }

    /// Truncates two transform chains at their common parent frame to optimize the transformation computation.
    ///
    /// # Arguments
    ///
    /// * `from_chain` - Mutable reference to the transform chain originating from the source frame
    /// * `to_chain` - Mutable reference to the transform chain originating from the target frame
    fn truncate_at_common_parent(
        from_chain: &mut VecDeque<Transform<T>>,
        to_chain: &mut VecDeque<Transform<T>>,
    ) {
        let mut start_idx = 0;
        for (i, j) in from_chain.iter().rev().zip(to_chain.iter().rev()) {
            if i == j {
                start_idx += 1;
            } else {
                break;
            }
        }

        // Truncate the chains at the common parent frame
        from_chain.truncate(from_chain.len() - start_idx);
        to_chain.truncate(to_chain.len() - start_idx);
    }

    /// Combines two transform chains into a single transform representing the transformation from the source frame to the target frame.
    ///
    /// # Arguments
    ///
    /// * `from_chain` - The transform chain from the source frame toward the common ancestor
    /// * `to_chain` - The inverted and reversed transform chain from the target frame toward the common ancestor
    ///
    /// # Errors
    ///
    /// * `TransformError::TransformTreeEmpty` - If the combined transform chain is empty
    /// * Other variants of `TransformError` resulting from invalid transform operations
    fn combine_transforms(
        mut from_chain: VecDeque<Transform<T>>,
        mut to_chain: VecDeque<Transform<T>>,
    ) -> Result<Transform<T>, TransformError> {
        from_chain.append(&mut to_chain);

        let mut iter = from_chain.into_iter();

        let Some(mut final_transform) = iter.next() else {
            return Err(TransformError::TransformTreeEmpty);
        };

        for transform in iter {
            final_transform = (transform * final_transform)?;
        }

        final_transform.inverse()
    }

    /// Reverses a transform chain and inverts each transform within it.
    ///
    /// # Arguments
    ///
    /// * `chain` - Mutable reference to the transform chain to be reversed and inverted
    ///
    /// # Errors
    ///
    /// Returns `TransformError` if any transform in the chain cannot be inverted
    fn reverse_and_invert_transforms(
        chain: &mut VecDeque<Transform<T>>
    ) -> Result<(), TransformError> {
        let reversed_and_inverted = chain
            .iter()
            .rev()
            .map(Transform::inverse)
            .collect::<Result<VecDeque<Transform<T>>, TransformError>>()?;

        *chain = reversed_and_inverted;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
