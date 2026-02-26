//! # Registry Module
//!
//! The `registry` module provides the core functionality for managing transforms between different coordinate frames. It maintains a collection of transforms and offers methods to add, retrieve, and chain these transforms.
//!
//! ## Features
//!
//! - **Static Transforms**: The registry can handle static transforms by using a timestamp set to zero.
//! - **Dynamic Transforms**: Supports dynamic transforms with timestamps to handle time-varying transformations.
//! - **Interpolation**: Interpolates between transforms if a requested timestamp lies between two known transforms.
//! - **Automatic Buffer Cleanup**: Automatically cleans up old transforms based on the `max_age` parameter when the `std` feature is enabled.
//!
//! ## Usage
//!
//! The `Registry` struct is the main entry point for interacting with the registry.
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
//! - `add_transform(&self, t: Transform) -> Result<(), BufferError>`
//!   - Adds a transform to the registry.
//!   - **Arguments**
//!     - `t`: The transform to add.
//!   - **Errors**
//!     - Returns a `BufferError` if the transform cannot be added.
//!
//! - `get_transform(&self, from: &str, to: &str, timestamp: Timestamp) -> Result<Transform, TransformError>`
//!   - Retrieves a transform from the registry.
//!   - **Arguments**
//!     - `from`: The source frame.
//!     - `to`: The destination frame.
//!     - `timestamp`: The timestamp for which the transform is requested.
//!   - **Errors**
//!     - Returns a `TransformError` if the transform cannot be found.

use crate::{core::Buffer, errors::TransformError, geometry::Transform, time::Timestamp};
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
pub struct Registry {
    pub data: HashMap<String, Buffer>,
    #[cfg(feature = "std")]
    max_age: Duration,
}

impl Registry {
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
    /// use transforms::Registry;
    ///
    /// let mut registry = Registry::new(Duration::from_secs(60));
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
    /// use transforms::Registry;
    ///
    /// let registry = Registry::new();
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
    /// # Errors
    ///
    /// Returns a `BufferError` if the transform cannot be added.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{geometry::Transform, Registry};
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::new(Duration::from_secs(60));
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::new();
    ///
    /// let transform = Transform::identity();
    ///
    /// registry.add_transform(transform);
    /// ```
    pub fn add_transform(
        &mut self,
        t: Transform,
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
        &mut self,
        from: &str,
        to: &str,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        Self::process_get_transform(from, to, timestamp, &mut self.data)
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
    ///                   reference point (typically a world or map frame)
    ///
    /// # Errors
    ///
    /// Returns a `TransformError` if:
    /// - The fixed frame is not reachable from both source and target frames
    /// - Any of the required transforms cannot be found at the specified times
    /// - The fixed frame is not stationary between the two timestamps
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
    /// // World frame is static
    /// let world_to_conveyor_t1 = Transform {
    ///     translation: Vector3::new(1.0, 0.0, 0.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: t1,
    ///     parent: "world".into(),
    ///     child: "conveyor".into(),
    /// };
    ///
    /// // Conveyor has moved by t2
    /// let world_to_conveyor_t2 = Transform {
    ///     translation: Vector3::new(2.0, 0.0, 0.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: t2,
    ///     parent: "world".into(),
    ///     child: "conveyor".into(),
    /// };
    ///
    /// // Object is at a fixed position relative to conveyor
    /// let conveyor_to_object_t1 = Transform {
    ///     translation: Vector3::new(0.0, 0.5, 0.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: t1,
    ///     parent: "conveyor".into(),
    ///     child: "object".into(),
    /// };
    ///
    /// registry.add_transform(world_to_conveyor_t1);
    /// registry.add_transform(world_to_conveyor_t2);
    /// registry.add_transform(conveyor_to_object_t1);
    ///
    /// // Get the position of the object (detected at t1) in the world frame at t2
    /// // This answers: "Where is the object now, given it was detected at t1?"
    /// let result = registry.get_transform_at_times(
    ///     "world",  // target_frame
    ///     t2,       // target_time (current time)
    ///     "object", // source_frame
    ///     t1,       // source_time (when object was detected)
    ///     "world",  // fixed_frame
    /// );
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn get_transform_at_times(
        &mut self,
        target_frame: &str,
        target_time: Timestamp,
        source_frame: &str,
        source_time: Timestamp,
        fixed_frame: &str,
    ) -> Result<Transform, TransformError> {
        Self::process_get_transform_at_times(
            target_frame,
            target_time,
            source_frame,
            source_time,
            fixed_frame,
            &mut self.data,
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
        timestamp: Timestamp,
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
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if there is an issue adding the transform to the buffer
    fn process_add_transform(
        t: Transform,
        data: &mut HashMap<String, Buffer>,
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
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if there is an issue adding the transform to the buffer
    fn process_add_transform(
        t: Transform,
        data: &mut HashMap<String, Buffer>,
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
    /// * `data` - Mutable reference to the data buffer containing transforms
    ///
    /// # Errors
    ///
    /// * `TransformError::NotFound` - If no valid transform chain is found between the specified frames
    /// * `TransformError::TransformTreeEmpty` - If the combined transform chain is empty after processing
    /// * Other variants of `TransformError` resulting from transform operations
    fn process_get_transform(
        from: &str,
        to: &str,
        timestamp: Timestamp,
        data: &mut HashMap<String, Buffer>,
    ) -> Result<Transform, TransformError> {
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
    /// 1. Getting the transform from source_frame to fixed_frame at source_time
    /// 2. Getting the transform from fixed_frame to target_frame at target_time
    /// 3. Chaining these together
    ///
    /// # Arguments
    ///
    /// * `target_frame` - The destination frame
    /// * `target_time` - The time at which to evaluate the target frame
    /// * `source_frame` - The source frame
    /// * `source_time` - The time at which to evaluate the source frame
    /// * `fixed_frame` - A frame that doesn't change over time (e.g., "world")
    /// * `data` - The transform data buffer
    ///
    /// # Errors
    ///
    /// Returns `TransformError::FixedFrameNotInChain` if the fixed frame is not reachable
    /// from both source and target frames
    fn process_get_transform_at_times(
        target_frame: &str,
        target_time: Timestamp,
        source_frame: &str,
        source_time: Timestamp,
        fixed_frame: &str,
        data: &mut HashMap<String, Buffer>,
    ) -> Result<Transform, TransformError> {
        // Following tf2's algorithm:
        // 1. Get transform expressing source_frame in fixed_frame at source_time
        // 2. Get transform expressing target_frame in fixed_frame at target_time
        // 3. Compute: T_target_to_fixed.inverse() * T_source_to_fixed

        // Validate that the fixed frame is actually stationary between the two timestamps.
        // A frame is considered fixed if:
        // - It has no parent (is root of transform tree), OR
        // - Its transform to its parent is static (timestamp = 0), OR
        // - Its transform to its parent doesn't change between source_time and target_time
        // Note: If this transform is not static we COULD add it to the transform chain and get
        // a somewhat valid transform, but that API is so difficult to use correctly that it's
        // better to just fail.
        Self::validate_fixed_frame(fixed_frame, source_time, target_time, data)?;

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
        // with their timestamps set to zero.
        source_to_fixed.timestamp = Timestamp::zero();
        target_to_fixed.timestamp = Timestamp::zero();

        let mut result = (target_to_fixed.inverse()? * source_to_fixed)?;
        // We set the final timestamp to the target_time as per the API contract.
        result.timestamp = target_time;

        Ok(result)
    }

    /// Validates that the fixed frame is stationary between two timestamps.
    ///
    /// A frame is considered stationary if:
    /// - It has no parent (is root of transform tree), OR
    /// - Its transform to its parent is static (timestamp = 0), OR
    /// - Its transform to its parent doesn't change between the two timestamps
    ///
    /// # Arguments
    ///
    /// * `fixed_frame` - The frame to validate as stationary
    /// * `source_time` - The first timestamp
    /// * `target_time` - The second timestamp
    /// * `data` - Reference to the data buffer containing transforms
    ///
    /// # Errors
    ///
    /// Returns `TransformError::MovingFixedFrame` if the fixed frame is not stationary
    fn validate_fixed_frame(
        fixed_frame: &str,
        source_time: Timestamp,
        target_time: Timestamp,
        data: &HashMap<String, Buffer>,
    ) -> Result<(), TransformError> {
        // If timestamps are the same, no validation needed
        if source_time == target_time {
            return Ok(());
        }

        // Check if the fixed frame has a parent (i.e., is stored in the data map)
        let Some(frame_buffer) = data.get(fixed_frame) else {
            // No buffer for this frame means it's a root frame - it's fixed by definition
            return Ok(());
        };

        // Try to get the transform at both timestamps
        let tf_at_source = frame_buffer.get(&source_time);
        let tf_at_target = frame_buffer.get(&target_time);

        match (tf_at_source, tf_at_target) {
            (Ok(tf1), Ok(tf2)) => {
                // Both transforms exist - check if they're the same
                // Static transforms (timestamp = 0) are always considered fixed
                if tf1.timestamp == Timestamp::zero() && tf2.timestamp == Timestamp::zero() {
                    return Ok(());
                }

                // Check for equality using PartialEq which internally implements an epsilon tolerance
                // This is really to cover the case of users constantly publishing the same transform as pseudo-static
                if tf1 != tf2 {
                    return Err(TransformError::MovingFixedFrame(fixed_frame.into()));
                }

                Ok(())
            }
            // If we can't get a transform at one or both times, the frame might be
            // partially defined - we allow this case (the actual lookup will fail later
            // if the transform truly doesn't exist)
            _ => Ok(()),
        }
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
        timestamp: Timestamp,
        data: &HashMap<String, Buffer>,
    ) -> Result<VecDeque<Transform>, TransformError> {
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
        from_chain: &mut VecDeque<Transform>,
        to_chain: &mut VecDeque<Transform>,
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
        mut from_chain: VecDeque<Transform>,
        mut to_chain: VecDeque<Transform>,
    ) -> Result<Transform, TransformError> {
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
        chain: &mut VecDeque<Transform>
    ) -> Result<(), TransformError> {
        let reversed_and_inverted = chain
            .iter()
            .rev()
            .map(Transform::inverse)
            .collect::<Result<VecDeque<Transform>, TransformError>>()?;

        *chain = reversed_and_inverted;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
