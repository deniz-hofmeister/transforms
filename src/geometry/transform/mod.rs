use crate::{
    geometry::{Quaternion, Vector3},
    time::Timestamp,
};
use alloc::string::String;
use approx::AbsDiffEq;
use core::{cmp::Ordering, ops::Mul, time::Duration};
pub use error::TransformError;
pub use traits::Transformable;

mod error;
mod traits;

/// Represents a 3D transformation with translation, rotation, and timestamp.
///
/// The `Transform` struct is used to represent a transformation in 3D space,
/// including translation, rotation, and associated metadata such as timestamps
/// and frame identifiers.
///
/// # Examples
///
/// ```
/// use transforms::geometry::{Quaternion, Transform, Vector3};
///
/// // Create an identity transform
/// let identity = Transform::identity();
///
/// assert_eq!(
///     identity.translation,
///     Vector3 {
///         x: 0.0,
///         y: 0.0,
///         z: 0.0
///     }
/// );
///
/// assert_eq!(
///     identity.rotation,
///     Quaternion {
///         w: 1.0,
///         x: 0.0,
///         y: 0.0,
///         z: 0.0
///     }
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub timestamp: Timestamp,
    pub parent: String,
    pub child: String,
}

impl Transform {
    /// Interpolates between two transforms at a given timestamp.
    ///
    /// Returns a new `Transform` that is the interpolation between `from` and `to`
    /// at the specified `timestamp`.
    ///
    /// # Errors
    ///
    /// Returns `TransformError::TimestampMismatch` if the timestamp is outside the range
    /// of `from` and `to`. Returns `TransformError::IncompatibleFrames` if the frames
    /// do not match.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let from = Transform {
    ///     translation: Vector3 {
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: Timestamp { t: 0 },
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let to = Transform {
    ///     translation: Vector3 {
    ///         x: 2.0,
    ///         y: 2.0,
    ///         z: 2.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: Timestamp { t: 2_000_000_000 },
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let result = Transform {
    ///     translation: Vector3 {
    ///         x: 1.0,
    ///         y: 1.0,
    ///         z: 1.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: Timestamp { t: 1_000_000_000 },
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let timestamp = Timestamp { t: 1_000_000_000 };
    ///
    /// let interpolated = Transform::interpolate(&from, &to, timestamp).unwrap();
    /// assert_eq!(result, interpolated);
    /// ```
    pub fn interpolate(
        from: &Transform,
        to: &Transform,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        if from.timestamp > to.timestamp || timestamp < from.timestamp || timestamp > to.timestamp {
            return Err(TransformError::TimestampMismatch(
                to.timestamp.as_seconds()?,
                from.timestamp.as_seconds()?,
            ));
        }
        if from.child != to.child || from.parent != to.parent {
            return Err(TransformError::IncompatibleFrames);
        }

        let range = to.timestamp.t - from.timestamp.t;
        if range == 0 {
            return Ok(from.clone());
        }

        let diff = timestamp.t - from.timestamp.t;
        #[allow(clippy::cast_precision_loss)]
        let ratio = diff as f64 / range as f64;

        Ok(Transform {
            translation: (1.0 - ratio) * from.translation + ratio * to.translation,
            rotation: from.rotation.slerp(to.rotation, ratio),
            timestamp,
            child: from.child.clone(),
            parent: from.parent.clone(),
        })
    }

    #[must_use = "Returns a new transform"]
    /// Returns the identity transform.
    ///
    /// The identity transform has no translation or rotation and is often used
    /// as a neutral element in transformations.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let identity = Transform::identity();
    /// let transform = Transform {
    ///     translation: Vector3 {
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 1.0,
    ///         x: 0.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     },
    ///     timestamp: Timestamp { t: 0 },
    ///     parent: "".into(),
    ///     child: "".into(),
    /// };
    ///
    /// assert_eq!(identity, transform);
    /// ```
    pub fn identity() -> Self {
        Transform {
            translation: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            rotation: Quaternion {
                w: 1.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            timestamp: Timestamp::zero(),
            parent: String::new(),
            child: String::new(),
        }
    }

    /// Computes the inverse of the transform.
    ///
    /// Returns a new `Transform` that is the inverse of the current transform.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// // Create a transform with specific translation and rotation
    /// let transform = Transform {
    ///     translation: Vector3 {
    ///         x: 1.0,
    ///         y: 2.0,
    ///         z: 3.0,
    ///     },
    ///     rotation: Quaternion {
    ///         w: 0.0,
    ///         x: 1.0,
    ///         y: 0.0,
    ///         z: 0.0,
    ///     }
    ///     .normalize()
    ///     .unwrap(),
    ///     timestamp: Timestamp::zero(),
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    ///
    /// // Compute its inverse
    /// let inverse = transform.inverse().unwrap();
    ///
    /// // Verify that the inverse has swapped frames
    /// assert_eq!(inverse.parent, "b");
    /// assert_eq!(inverse.child, "a");
    ///
    /// // Verify that applying the inverse transformation results in the identity
    /// let identity = Transform::identity();
    /// let result = (transform * inverse).unwrap();
    /// assert_eq!(result.translation, identity.translation);
    /// assert_eq!(result.rotation, identity.rotation);
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns a `TransformError` if:
    /// - The quaternion normalization fails, resulting in a `QuaternionError`.
    pub fn inverse(&self) -> Result<Self, TransformError> {
        let q = self.rotation.normalize()?;
        let inverse_rotation = q.conjugate();
        let inverse_translation = -1.0 * (inverse_rotation.rotate_vector(self.translation));

        Ok(Transform {
            translation: inverse_translation,
            rotation: inverse_rotation,
            timestamp: self.timestamp,
            parent: self.child.clone(),
            child: self.parent.clone(),
        })
    }
}

impl Mul for Transform {
    type Output = Result<Transform, TransformError>;

    #[inline]
    fn mul(
        self,
        rhs: Transform,
    ) -> Self::Output {
        let is_self_static = self.timestamp.t == 0;
        let is_rhs_static = rhs.timestamp.t == 0;

        let duration = if !is_self_static && !is_rhs_static {
            let d = match self.timestamp.cmp(&rhs.timestamp) {
                Ordering::Equal => Ok(Duration::from_secs(0)),
                Ordering::Greater => self.timestamp - rhs.timestamp,
                Ordering::Less => rhs.timestamp - self.timestamp,
            }?;

            if d.as_secs_f64() > 2.0 * f64::EPSILON {
                return Err(TransformError::TimestampMismatch(
                    self.timestamp.as_seconds()?,
                    rhs.timestamp.as_seconds()?,
                ));
            }
            d
        } else {
            Duration::from_secs(0)
        };

        if self.child == rhs.child {
            return Err(TransformError::SameFrameMultiplication);
        }

        if self.child != rhs.parent && self.parent != rhs.child {
            return Err(TransformError::IncompatibleFrames);
        }

        let r = self.rotation * rhs.rotation;
        let t = self.rotation.rotate_vector(rhs.translation) + self.translation;

        Ok(Transform {
            translation: t,
            rotation: r,
            timestamp: if is_self_static {
                rhs.timestamp
            } else if is_rhs_static {
                self.timestamp
            } else {
                (self.timestamp + (duration.div_f64(2.0)))?
            },
            parent: self.parent,
            child: rhs.child,
        })
    }
}

impl PartialEq for Transform {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.translation
            .abs_diff_eq(&other.translation, f64::EPSILON)
            && self.rotation.abs_diff_eq(&other.rotation, f64::EPSILON)
            && self.timestamp == other.timestamp
            && self.parent == other.parent
            && self.child == other.child
    }
}

impl Eq for Transform {}

#[cfg(test)]
mod tests;
