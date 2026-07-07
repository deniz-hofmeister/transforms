//! # Registry Module
//!
//! The `registry` module provides the core functionality for managing transforms between different coordinate frames. It maintains a collection of transforms and offers methods to add, retrieve, and chain these transforms.
//!
//! ## Features
//!
//! - **Static Transforms**: The registry can handle static transforms by using the static timestamp value (`t=0` by default).
//! - **Dynamic Transforms**: Supports dynamic transforms with timestamps to handle time-varying transformations.
//! - **Interpolation**: Interpolates between transforms if a requested timestamp lies between two known transforms.
//! - **Automatic Buffer Cleanup**: A registry built with `Registry::with_max_age`
//!   automatically cleans up old dynamic transforms on insert; one built with
//!   `Registry::new` keeps them until `delete_transforms_before` is called.
//!
//! ## Usage
//!
//! The `Registry` struct is the main entry point for interacting with the registry.
//!
//! ## Time type selection
//!
//! `Registry` defaults to `Timestamp`, so `Registry::new()` is equivalent to
//! `Registry::<Timestamp>::new()`.
//!
//! You can use custom timestamps by implementing `time::TimePoint` and then
//! constructing `Registry::<CustomTimestamp>::new(...)`.
//!
//! With the `std` feature enabled, `std::time::SystemTime` already implements
//! `TimePoint`, so `Registry::<SystemTime>::with_max_age(Duration::from_secs(...))`
//! works out of the box.
//!
//! # Examples
//!
//! ```rust
//! # {
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
//! let t1 = Timestamp::now();
//!
//! # #[cfg(not(feature = "std"))]
//! let mut registry = Registry::new();
//! # #[cfg(not(feature = "std"))]
//! let t1 = Timestamp::zero();
//!
//! let t2 = t1;
//!
//! // Define a transform from frame "a" to frame "b"
//! let t_a_b_1 = Transform {
//!     translation: Vector3::new(1.0, 0.0, 0.0),
//!     rotation: Quaternion::identity(),
//!     timestamp: t1,
//!     parent: "a".into(),
//!     child: "b".into(),
//! };
//!
//! // For validation
//! let t_a_b_2 = t_a_b_1.clone();
//!
//! // Add the transform to the registry
//! registry.add_transform(t_a_b_1).unwrap();
//!
//! // Retrieve the transform from "a" to "b"
//! let result = registry.get_transform("a", "b", t2);
//! assert!(result.is_ok());
//! assert_eq!(result.unwrap(), t_a_b_2);
//! # }
//! ```

use crate::{
    core::Buffer,
    errors::{BufferError, TransformError},
    geometry::{Localized, Quaternion, Transform, Vector3},
    time::{TimePoint, Timestamp},
};
use alloc::{
    collections::{BTreeSet, VecDeque},
    string::String,
};
use hashbrown::HashMap;

use core::time::Duration;

/// A registry for managing transforms between different frames. It can
/// traverse the parent-child tree and calculate the final transform.
/// It will interpolate between two entries if a time is requested that
/// lies in between.
///
/// The `Registry` struct provides methods to add and retrieve transforms
/// between frames.
///
/// # Examples
///
/// ```
/// use transforms::{
///     Registry,
///     geometry::{Quaternion, Transform, Vector3},
///     time::Timestamp,
/// };
///
/// # #[cfg(feature = "std")]
/// use core::time::Duration;
/// # #[cfg(feature = "std")]
/// let mut registry = Registry::with_max_age(Duration::from_secs(60));
/// # #[cfg(feature = "std")]
/// let t1 = Timestamp::now();
///
/// # #[cfg(not(feature = "std"))]
/// let mut registry = Registry::new();
/// # #[cfg(not(feature = "std"))]
/// let t1 = Timestamp::zero();
///
/// let t2 = t1;
///
/// // Define a transform from frame "a" to frame "b"
/// let t_a_b_1 = Transform {
///     translation: Vector3::new(1.0, 0.0, 0.0),
///     rotation: Quaternion::identity(),
///     timestamp: t1,
///     parent: "a".into(),
///     child: "b".into(),
/// };
///
/// // For validation
/// let t_a_b_2 = t_a_b_1.clone();
///
/// // Add the transform to the registry
/// registry.add_transform(t_a_b_1).unwrap();
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
    /// Maps a child frame name to the buffer of transforms into that frame.
    data: HashMap<String, Buffer<T>>,
    max_age: Option<Duration>,
}

impl<T> Registry<T>
where
    T: TimePoint,
{
    /// Creates a new `Registry` without automatic cleanup.
    ///
    /// Transforms are kept until removed manually with
    /// [`Registry::delete_transforms_before`]. Use
    /// [`Registry::with_max_age`] for automatic cleanup.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, time::Timestamp};
    ///
    /// let registry = Registry::<Timestamp>::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_age: None,
        }
    }

    /// Creates a new `Registry` with automatic cleanup after `max_age`.
    ///
    /// Dynamic transforms older than `max_age` relative to the latest
    /// inserted timestamp of their child frame are removed automatically on
    /// insert. Static transforms never expire.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::{Registry, time::Timestamp};
    ///
    /// let mut registry = Registry::<Timestamp>::with_max_age(Duration::from_secs(60));
    /// ```
    #[must_use]
    pub fn with_max_age(max_age: Duration) -> Self {
        Self {
            data: HashMap::new(),
            max_age: Some(max_age),
        }
    }

    /// Adds a transform to the registry.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::StaticDynamicConflict` if the transform's child
    /// frame already holds transforms of the opposite kind: a child frame is
    /// either static (timestamp equal to the static timestamp value, `t=0` by
    /// default) or dynamic, never both.
    ///
    /// Returns `BufferError::TransformError` if the transform fails
    /// validation (non-finite values or a non-unit rotation),
    /// `BufferError::SelfReferentialFrame` if its parent and child are the
    /// same frame, `BufferError::ReparentingNotSupported` if the child frame
    /// already has a different parent (remove the frame first with
    /// [`Registry::remove_frame`]), and `BufferError::CycleDetected` if the
    /// new relationship would create a cycle in the frame tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let mut registry = Registry::<Timestamp>::new();
    /// let transform = Transform {
    ///     translation: Vector3::new(1.0, 0.0, 0.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: Timestamp::zero(),
    ///     parent: "base".into(),
    ///     child: "sensor".into(),
    /// };
    ///
    /// registry.add_transform(transform).unwrap();
    /// ```
    pub fn add_transform(
        &mut self,
        t: Transform<T>,
    ) -> Result<(), BufferError> {
        Self::process_add_transform(t, &mut self.data, self.max_age)
    }

    /// Retrieves the transform from the `from` frame to the `to` frame at
    /// the requested timestamp.
    ///
    /// The returned transform always carries the requested timestamp, also
    /// when the chain consists of static transforms. Requesting a frame
    /// relative to itself returns the identity transform.
    ///
    /// # Errors
    ///
    /// Returns a `TransformError` if the transform cannot be found.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::with_max_age(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// let t1 = Timestamp::zero();
    ///
    /// let t2 = t1;
    ///
    /// // Define a transform from frame "a" to frame "b"
    /// let t_a_b_1 = Transform {
    ///     translation: Vector3::new(1.0, 0.0, 0.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: t1,
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// // For validation
    /// let t_a_b_2 = t_a_b_1.clone();
    ///
    /// registry.add_transform(t_a_b_1).unwrap();
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
    /// If the value is already in `target_frame`, this returns an identity
    /// transform with `parent == child == target_frame` and the value's
    /// timestamp (via `get_transform`'s same-frame identity).
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
    /// 1. Computing the transform that expresses `source_frame` in `fixed_frame` at `source_time`
    /// 2. Computing the transform that expresses `target_frame` in `fixed_frame` at `target_time`
    /// 3. Combining the two into the requested transform
    ///
    /// `fixed_frame` is a frame that does not change over time, used as an
    /// intermediate reference point (typically a world or map frame).
    ///
    /// # Choosing the fixed frame
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
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::with_max_age(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    /// # #[cfg(feature = "std")]
    /// let t2 = (t1 + Duration::from_secs(1)).unwrap();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// let t1 = Timestamp::from_nanos(1_000_000_000);
    /// # #[cfg(not(feature = "std"))]
    /// let t2 = Timestamp::from_nanos(2_000_000_000);
    ///
    /// // Tree: fixed -> a -> b
    ///
    /// // fixed -> a at t1: a is at x=1
    /// registry
    ///     .add_transform(Transform {
    ///         translation: Vector3::new(1.0, 0.0, 0.0),
    ///         rotation: Quaternion::identity(),
    ///         timestamp: t1,
    ///         parent: "fixed".into(),
    ///         child: "a".into(),
    ///     })
    ///     .unwrap();
    ///
    /// // fixed -> a at t2: a has moved to x=2
    /// registry
    ///     .add_transform(Transform {
    ///         translation: Vector3::new(2.0, 0.0, 0.0),
    ///         rotation: Quaternion::identity(),
    ///         timestamp: t2,
    ///         parent: "fixed".into(),
    ///         child: "a".into(),
    ///     })
    ///     .unwrap();
    ///
    /// // a -> b at t1: b is at y=1 relative to a
    /// registry
    ///     .add_transform(Transform {
    ///         translation: Vector3::new(0.0, 1.0, 0.0),
    ///         rotation: Quaternion::identity(),
    ///         timestamp: t1,
    ///         parent: "a".into(),
    ///         child: "b".into(),
    ///     })
    ///     .unwrap();
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

    /// Removes dynamic transforms older than the given threshold.
    ///
    /// Iterates over all buffers and deletes their dynamic entries with a
    /// timestamp lower than the input argument. Static transforms are
    /// preserved: they are valid for all time, so cleaning them up by
    /// timestamp would silently destroy them.
    ///
    /// Frames left without any transforms are removed entirely, so the
    /// registry does not grow without bound as frames come and go.
    pub fn delete_transforms_before(
        &mut self,
        timestamp: T,
    ) {
        for buffer in self.data.values_mut() {
            buffer.delete_before(timestamp);
        }
        self.data.retain(|_, buffer| !buffer.is_empty());
    }

    /// Removes a child frame and all of its transforms from the registry.
    ///
    /// Returns `true` if the frame existed. This is also the escape hatch
    /// for re-parenting, which `add_transform` rejects: remove the frame,
    /// then re-add it under its new parent.
    pub fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.data.remove(child).is_some()
    }

    /// Adds a transform to the data buffer.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::StaticDynamicConflict` if the child frame's buffer
    /// already holds transforms of the opposite kind (static vs. dynamic).
    fn process_add_transform(
        t: Transform<T>,
        data: &mut HashMap<String, Buffer<T>>,
        max_age: Option<Duration>,
    ) -> Result<(), BufferError> {
        // A new child->parent relationship changes the tree topology; reject
        // it if it would close a cycle. (Existing buffers have their parent
        // pinned, so occupied inserts cannot.)
        if !data.contains_key(&t.child) && Self::creates_cycle(&t.child, &t.parent, data) {
            return Err(BufferError::CycleDetected);
        }

        if let Some(buffer) = data.get_mut(&t.child) {
            return buffer.insert(t);
        }

        // New frame: fill the buffer BEFORE registering it in the map, so a
        // failed insert cannot leave an empty, parentless frame behind —
        // which would bypass the cycle check on a later insert of the same
        // child frame.
        let mut buffer = match max_age {
            Some(max_age) => Buffer::with_max_age(max_age),
            None => Buffer::new(),
        };
        let child = t.child.clone();
        buffer.insert(t)?;
        data.insert(child, buffer);
        Ok(())
    }

    /// Returns `true` if adding the relationship `child -> parent` would
    /// create a cycle in the frame tree.
    ///
    /// Walks upward from `parent` through the pinned buffer parents. The
    /// existing tree is acyclic (every insert passes this check), so the walk
    /// terminates at a root; the visited set is a defensive bound only.
    fn creates_cycle(
        child: &str,
        parent: &str,
        data: &HashMap<String, Buffer<T>>,
    ) -> bool {
        let mut visited = BTreeSet::new();
        let mut current = parent;
        while let Some(buffer) = data.get(current) {
            if !visited.insert(current) {
                return true;
            }
            match buffer.parent() {
                Some(next) => {
                    if next == child {
                        return true;
                    }
                    current = next;
                }
                None => return false,
            }
        }
        false
    }

    /// Retrieves and computes the transform between two frames at a specific timestamp.
    ///
    /// # Errors
    ///
    /// * `TransformError::NotFound` - If no valid transform chain is found between the specified frames
    /// * Other variants of `TransformError` resulting from transform operations
    fn process_get_transform(
        from: &str,
        to: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<Transform<T>, TransformError> {
        // A frame relative to itself is the identity, regardless of whether
        // the frame is known: the answer holds either way, and it keeps
        // same-frame queries consistent with `get_transform_for`.
        if from == to {
            return Ok(Transform {
                translation: Vector3::zero(),
                rotation: Quaternion::identity(),
                timestamp,
                parent: from.into(),
                child: to.into(),
            });
        }

        let reached = |chain: &VecDeque<Transform<T>>, target: &str| {
            chain.back().is_some_and(|tf| tf.parent == target)
        };

        let from_chain = Self::get_transform_chain(from, to, timestamp, data);

        let result = match from_chain {
            // `to` is an ancestor of `from`: the from-side chain spans the
            // whole path, no to-side walk is needed.
            Ok(from_chain) if reached(&from_chain, to) => {
                Self::combine_transforms(from_chain, VecDeque::new())
            }
            from_chain => match (
                from_chain,
                Self::get_transform_chain(to, from, timestamp, data),
            ) {
                // `from` is an ancestor of `to`: the to-side chain spans the
                // whole path by itself.
                (_, Ok(mut to_chain)) if reached(&to_chain, from) => {
                    Self::reverse_and_invert_transforms(&mut to_chain)?;
                    Self::combine_transforms(VecDeque::new(), to_chain)
                }
                // Both chains ran to the root: drop the shared suffix above
                // the common parent and combine the remainders.
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
            },
        }?;

        // A chain can resolve without ever reaching the requested frame, for
        // example when `to` does not exist in the tree and the walk stopped at
        // the root instead. Verify the combined transform answers the exact
        // question asked; otherwise report it as not found.
        if result.parent != from || result.child != to {
            return Err(TransformError::NotFound(from.into(), to.into()));
        }

        // The result answers "where is `to` relative to `from` at the
        // requested time", so it carries the requested timestamp — also for
        // chains of static transforms, whose own timestamps are the static
        // sentinel.
        let mut result = result;
        result.timestamp = timestamp;
        Ok(result)
    }

    /// Retrieves a transform between two frames at different timestamps using a fixed frame.
    ///
    /// This implements "time travel" by:
    /// 1. Getting the transform that expresses `source_frame` in `fixed_frame` at `source_time`
    /// 2. Getting the transform that expresses `target_frame` in `fixed_frame` at `target_time`
    /// 3. Combining the two into the requested transform
    ///
    /// `fixed_frame` must be a frame that doesn't change over time (e.g., "world").
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
                translation: Vector3::zero(),
                rotation: Quaternion::identity(),
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
                translation: Vector3::zero(),
                rotation: Quaternion::identity(),
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

        // The frame tree is acyclic by construction (cycles are rejected at
        // insertion), so the walk terminates at a root; the depth bound is a
        // defensive backstop only.
        let mut remaining = data.len();
        while let Some(frame_buffer) = data.get(&current_frame) {
            if remaining == 0 {
                return Err(TransformError::NotFound(from.into(), to.into()));
            }
            remaining -= 1;

            match frame_buffer.get(&timestamp) {
                Ok(tf) => {
                    current_frame.clone_from(&tf.parent);
                    transforms.push_back(tf);
                }
                Err(_) => break,
            }

            // Reaching `to` completes the chain; walking on to the root would
            // only add work that truncate_at_common_parent discards again.
            if current_frame == to {
                break;
            }
        }

        if transforms.is_empty() {
            Err(TransformError::NotFound(from.into(), to.into()))
        } else {
            Ok(transforms)
        }
    }

    /// Truncates two transform chains at their common parent frame to optimize the transformation computation.
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
    /// `from_chain` runs from the source frame toward the common ancestor;
    /// `to_chain` must already be reversed and inverted (from the target frame
    /// toward the common ancestor).
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

impl<T> Default for Registry<T>
where
    T: TimePoint,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
