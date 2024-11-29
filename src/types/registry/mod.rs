use crate::types::{Buffer, Timestamp, Transform};
use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::Duration;
mod error;
use crate::errors::{BufferError, TransformError};

#[cfg(feature = "async")]
pub use async_impl::Registry;

#[cfg(not(feature = "async"))]
pub use sync_impl::Registry;

#[cfg(feature = "async")]
pub mod async_impl {
    use super::*;
    use tokio::sync::{Mutex, Notify};

    /// A registry for managing transforms between different frames.
    ///
    /// The `Registry` struct provides methods to add and retrieve transforms
    /// between frames, supporting both synchronous and asynchronous operations
    /// depending on the feature flags.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
    /// use std::time::Duration;
    /// # use tokio_test::block_on;
    ///
    /// # block_on(async {
    /// // Create a new registry with a time-to-live duration
    /// let mut registry = Registry::new(Duration::from_secs(60));
    /// let t1 = Timestamp::now();
    /// let t2 = t1.clone();
    ///
    /// // Define a transform from frame "a" to frame "b"
    /// let t_a_b_1 = Transform {
    ///     translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 },
    ///     rotation: Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
    ///     timestamp: t1,
    ///     parent: "a".to_string(),
    ///     child: "b".to_string(),
    /// };
    ///
    /// // For validation
    /// let t_a_b_2 = t_a_b_1.clone();
    ///
    /// // Add the transform to the registry
    /// let result = registry.add_transform(t_a_b_1).await;
    /// assert!(result.is_ok());
    ///
    /// // Retrieve the transform from "a" to "b"
    /// let result = registry.get_transform("a", "b", t2).await;
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), t_a_b_2);
    /// # });
    /// ```
    pub struct Registry {
        pub data: Mutex<HashMap<String, Buffer>>,
        max_age: Duration,
        notify: Notify,
    }

    impl Registry {
        /// Creates a new `Registry` with the specified time-to-live duration.
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
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// use std::time::Duration;
        ///
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// ```
        pub fn new(max_age: std::time::Duration) -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                max_age: max_age.into(),
                notify: Notify::new(),
            }
        }

        /// Adds a transform to the registry asynchronously.
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
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// # use tokio_test::block_on;
        /// use std::time::Duration;
        ///
        /// # block_on(async {
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// let transform = Transform::identity();
        ///
        /// let result = registry.add_transform(transform).await;
        /// assert!(result.is_ok());
        /// # });
        /// ```
        pub async fn add_transform(
            &self,
            t: Transform,
        ) -> Result<(), BufferError> {
            {
                let mut data = self.data.lock().await;
                Self::process_add_transform(t, &mut data, self.max_age)?;
            }
            self.notify.notify_waiters();
            Ok(())
        }

        /// Awaits for a transform to become available in the registry.
        ///
        /// This method will (indefinitely) wait until the requested transform becomes available.
        ///
        /// # Arguments
        ///
        /// * `from` - The source frame.
        /// * `to` - The destination frame.
        /// * `timestamp` - The timestamp for which the transform is requested.
        ///
        /// # Returns
        ///
        /// A `Result` containing the `Transform` if found, or an error if not found.
        ///
        /// # Examples
        ///
        /// ```
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// # use tokio_test::block_on;
        /// use std::time::Duration;
        ///
        /// # block_on(async {
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// let t1 = Timestamp::zero();
        /// let t2 = t1.clone();
        ///
        /// // Define a transform from frame "a" to frame "b"
        /// let t_a_b_1 = Transform {
        ///     translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 },
        ///     rotation: Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
        ///     timestamp: t1,
        ///     parent: "a".to_string(),
        ///     child: "b".to_string(),
        /// };
        /// // For validation
        /// let t_a_b_2 = t_a_b_1.clone();
        ///
        /// let result = registry.add_transform(t_a_b_1).await;
        /// assert!(result.is_ok());
        ///
        /// let result = registry.await_transform("a", "b", t2).await;
        /// assert!(result.is_ok());
        /// assert_eq!(result.unwrap(), t_a_b_2);
        /// # });
        /// ```
        pub async fn await_transform(
            &self,
            from: &str,
            to: &str,
            timestamp: Timestamp,
        ) -> Result<Transform, TransformError> {
            loop {
                if let Ok(transform) = self.get_transform(from, to, timestamp).await {
                    return Ok(transform);
                }
                self.notify.notified().await;
            }
        }

        /// Retrieves a transform from the registry asynchronously.
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
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// # use tokio_test::block_on;
        /// use std::time::Duration;
        ///
        /// # block_on(async {
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// let t1 = Timestamp::zero();
        /// let t2 = t1.clone();
        ///
        /// // Define a transform from frame "a" to frame "b"
        /// let t_a_b_1 = Transform {
        ///     translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 },
        ///     rotation: Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
        ///     timestamp: t1,
        ///     parent: "a".to_string(),
        ///     child: "b".to_string(),
        /// };
        /// // For validation
        /// let t_a_b_2 = t_a_b_1.clone();
        ///
        /// let result = registry.add_transform(t_a_b_1).await;
        /// assert!(result.is_ok());
        ///
        /// let result = registry.get_transform("a", "b", t2).await;
        /// assert!(result.is_ok());
        /// assert_eq!(result.unwrap(), t_a_b_2);
        /// # });
        /// ```
        pub async fn get_transform(
            &self,
            from: &str,
            to: &str,
            timestamp: Timestamp,
        ) -> Result<Transform, TransformError> {
            let mut d = self.data.lock().await;
            Self::process_get_transform(from, to, timestamp, &mut d)
        }
    }
}

#[cfg(not(feature = "async"))]
pub mod sync_impl {
    use super::*;

    /// A registry for managing transforms between different frames. It can
    /// traverse the parent-child tree and calculate the final transform.
    /// It will interpolate between two entries if a time is requested that
    /// lies in between.
    ///
    /// The `Registry` struct provides methods to add and retrieve transforms
    /// between frames, supporting both synchronous and asynchronous operations
    /// depending on the feature flags.
    ///
    /// # Examples
    ///
    /// ```
    /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
    /// use std::time::Duration;
    ///
    /// // Create a new registry with a time-to-live duration
    /// let mut registry = Registry::new(Duration::from_secs(60));
    /// let t1 = Timestamp::now();
    /// let t2 = t1.clone();
    ///
    /// // Define a transform from frame "a" to frame "b"
    /// let t_a_b_1 = Transform {
    ///     translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 },
    ///     rotation: Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
    ///     timestamp: t1,
    ///     parent: "a".to_string(),
    ///     child: "b".to_string(),
    /// };
    ///
    /// // For validation
    /// let t_a_b_2 = t_a_b_1.clone();
    ///
    /// // Add the transform to the registry
    /// let result = registry.add_transform(t_a_b_1);
    /// assert!(result.is_ok());
    ///
    /// // Retrieve the transform from "a" to "b"
    /// let result = registry.get_transform("a", "b", t2);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), t_a_b_2);
    /// ```
    pub struct Registry {
        pub data: HashMap<String, Buffer>,
        max_age: Duration,
    }

    impl Registry {
        /// Creates a new `Registry` with the specified time-to-live duration.
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
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// use std::time::Duration;
        ///
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// ```
        pub fn new(max_age: std::time::Duration) -> Self {
            Self {
                data: HashMap::new(),
                max_age: max_age.into(),
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
        /// # use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// use std::time::Duration;
        ///
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// let transform = Transform::identity();
        ///
        /// let result = registry.add_transform(transform);
        /// assert!(result.is_ok());
        /// ```
        pub fn add_transform(
            &mut self,
            t: Transform,
        ) -> Result<(), BufferError> {
            Self::process_add_transform(t, &mut self.data, self.max_age)
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
        /// use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};
        /// use std::time::Duration;
        ///
        /// let mut registry = Registry::new(Duration::from_secs(60));
        /// let t1 = Timestamp::zero();
        /// let t2 = t1.clone();
        ///
        /// // Define a transform from frame "a" to frame "b"
        /// let t_a_b_1 = Transform {
        ///     translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 },
        ///     rotation: Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
        ///     timestamp: t1,
        ///     parent: "a".to_string(),
        ///     child: "b".to_string(),
        /// };
        /// // For validation
        /// let t_a_b_2 = t_a_b_1.clone();
        ///
        /// let result = registry.add_transform(t_a_b_1);
        /// assert!(result.is_ok());
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
    }
}

impl Registry {
    fn process_add_transform(
        t: Transform,
        data: &mut HashMap<String, Buffer>,
        max_age: Duration,
    ) -> Result<(), BufferError> {
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
        Ok(())
    }

    fn process_get_transform(
        from: &str,
        to: &str,
        timestamp: Timestamp,
        data: &mut HashMap<String, Buffer>,
    ) -> Result<Transform, TransformError> {
        let from_chain = Self::get_transform_chain(from, to, timestamp, data);
        let mut to_chain = Self::get_transform_chain(to, from, timestamp, data);

        if let Ok(chain) = to_chain.as_mut() {
            Self::reverse_and_invert_transforms(chain)?;
        }

        match (from_chain, to_chain) {
            (Ok(mut from_chain), Ok(mut to_chain)) => {
                Self::truncate_at_common_parent(&mut from_chain, &mut to_chain);
                Self::combine_transforms(from_chain, to_chain)
            }
            (Ok(from_chain), Err(_)) => Self::combine_transforms(from_chain, VecDeque::new()),
            (Err(_), Ok(to_chain)) => Self::combine_transforms(VecDeque::new(), to_chain),
            (Err(_), Err(_)) => Err(TransformError::NotFound(from.to_string(), to.to_string())),
        }
    }

    fn get_transform_chain(
        from: &str,
        to: &str,
        timestamp: Timestamp,
        data: &HashMap<String, Buffer>,
    ) -> Result<VecDeque<Transform>, TransformError> {
        let mut transforms = VecDeque::new();
        let mut current_frame = from.to_string();

        while let Some(frame_buffer) = data.get(&current_frame) {
            match frame_buffer.get(&timestamp) {
                Ok(tf) => {
                    transforms.push_back(tf.clone());
                    current_frame = tf.parent.clone();
                    if current_frame == to {
                        return Ok(transforms);
                    }
                }
                Err(_) => break,
            }
        }

        if transforms.is_empty() {
            Err(TransformError::NotFound(from.to_string(), to.to_string()))
        } else {
            Ok(transforms)
        }
    }

    fn truncate_at_common_parent(
        from_chain: &mut VecDeque<Transform>,
        to_chain: &mut VecDeque<Transform>,
    ) {
        if let Some(index) = from_chain
            .iter()
            .position(|tf| to_chain.iter().any(|to_tf| to_tf.parent == tf.parent))
        {
            from_chain.truncate(index + 1);
        }

        if let Some(index) = to_chain
            .iter()
            .position(|tf| from_chain.iter().any(|from_tf| from_tf.parent == tf.parent))
        {
            to_chain.truncate(index + 1);
        }
    }

    fn combine_transforms(
        mut from_chain: VecDeque<Transform>,
        mut to_chain: VecDeque<Transform>,
    ) -> Result<Transform, TransformError> {
        from_chain.append(&mut to_chain);

        let mut iter = from_chain.into_iter();

        let mut final_transform = match iter.next() {
            Some(transform) => transform,
            None => {
                return Err(TransformError::TransformTreeEmpty);
            }
        };

        for transform in iter {
            final_transform = (transform * final_transform)?;
        }

        final_transform.inverse()
    }

    fn reverse_and_invert_transforms(
        chain: &mut VecDeque<Transform>
    ) -> Result<(), TransformError> {
        let reversed_and_inverted = chain
            .iter()
            .rev()
            .map(|item| item.inverse())
            .collect::<Result<VecDeque<Transform>, TransformError>>()?;

        *chain = reversed_and_inverted;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
