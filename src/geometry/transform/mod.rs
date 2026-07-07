//! Rigid-body transforms between coordinate frames, with composition, inversion, and interpolation.

use crate::{
    geometry::{Quaternion, Vector3},
    time::{TimePoint, Timestamp},
};
use alloc::string::String;
use approx::AbsDiffEq;
use core::ops::Mul;
pub use error::TransformError;
pub use traits::{Localized, Transformable};

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
/// use transforms::{
///     geometry::{Quaternion, Transform, Vector3},
///     time::Timestamp,
/// };
///
/// // Create an identity transform
/// let identity = Transform::<Timestamp>::identity();
///
/// assert_eq!(identity.translation, Vector3::zero());
/// assert_eq!(identity.rotation, Quaternion::identity());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Transform<T = Timestamp>
where
    T: TimePoint,
{
    /// The translational component of the transform.
    pub translation: Vector3,
    /// The rotational component of the transform.
    pub rotation: Quaternion,
    /// The time at which the transform is valid.
    pub timestamp: T,
    /// The target frame; the transform maps child-frame coordinates into this frame.
    pub parent: String,
    /// The source frame whose coordinates are mapped into the parent frame.
    pub child: String,
}

impl<T> Transform<T>
where
    T: TimePoint,
{
    /// The accepted deviation of a rotation's norm from 1 in
    /// [`Transform::validate`].
    ///
    /// Loose enough to accept unit quaternions that were stored or
    /// transmitted as `f32` and widened to `f64`, tight enough to reject
    /// genuinely denormalized rotations, which would otherwise corrupt every
    /// lookup they take part in without any error.
    pub const UNIT_NORM_TOLERANCE: f64 = 1e-6;

    /// Checks that the transform is usable for composition and lookup.
    ///
    /// A valid transform has finite translation and rotation components and a
    /// rotation whose norm is within [`Transform::UNIT_NORM_TOLERANCE`] of
    /// `1.0`. The registry enforces this on insertion; call it directly when
    /// composing hand-built transforms with `*` or applying them via
    /// `Transformable`, which do not validate.
    ///
    /// # Errors
    ///
    /// Returns `TransformError::NonFiniteValues` if any component is NaN or
    /// infinite, and `TransformError::NonUnitRotation` if the rotation is not
    /// a unit quaternion within the tolerance.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     errors::TransformError,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let mut transform = Transform::<Timestamp>::identity();
    /// assert!(transform.validate().is_ok());
    ///
    /// transform.rotation = Quaternion::new(2.0, 0.0, 0.0, 0.0);
    /// assert!(matches!(
    ///     transform.validate(),
    ///     Err(TransformError::NonUnitRotation(_))
    /// ));
    /// ```
    pub fn validate(&self) -> Result<(), TransformError> {
        let t = self.translation;
        let q = self.rotation;

        let finite = t.x.is_finite()
            && t.y.is_finite()
            && t.z.is_finite()
            && q.w.is_finite()
            && q.x.is_finite()
            && q.y.is_finite()
            && q.z.is_finite();
        if !finite {
            return Err(TransformError::NonFiniteValues);
        }

        let norm = q.norm();
        if (norm - 1.0).abs() > Self::UNIT_NORM_TOLERANCE {
            return Err(TransformError::NonUnitRotation(norm));
        }

        Ok(())
    }

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
    ///     translation: Vector3::zero(),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: Timestamp::zero(),
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let to = Transform {
    ///     translation: Vector3::new(2.0, 2.0, 2.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: Timestamp::from_nanos(2_000_000_000),
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let result = Transform {
    ///     translation: Vector3::new(1.0, 1.0, 1.0),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: Timestamp::from_nanos(1_000_000_000),
    ///     parent: "a".into(),
    ///     child: "b".into(),
    /// };
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    ///
    /// let interpolated = Transform::interpolate(&from, &to, timestamp).unwrap();
    /// assert_eq!(result, interpolated);
    /// ```
    pub fn interpolate(
        from: &Transform<T>,
        to: &Transform<T>,
        timestamp: T,
    ) -> Result<Transform<T>, TransformError> {
        if from.timestamp > to.timestamp || timestamp < from.timestamp || timestamp > to.timestamp {
            return Err(TransformError::TimestampMismatch(
                to.timestamp.as_seconds()?,
                from.timestamp.as_seconds()?,
            ));
        }
        if from.child != to.child || from.parent != to.parent {
            return Err(TransformError::IncompatibleFrames);
        }

        let range = to.timestamp.duration_since(from.timestamp)?;
        if range.is_zero() {
            return Ok(from.clone());
        }

        let diff = timestamp.duration_since(from.timestamp)?;
        let ratio = diff.as_secs_f64() / range.as_secs_f64();

        Ok(Transform {
            translation: (1.0 - ratio) * from.translation + ratio * to.translation,
            rotation: from.rotation.slerp(to.rotation, ratio),
            timestamp,
            child: from.child.clone(),
            parent: from.parent.clone(),
        })
    }

    /// Returns the identity transform.
    ///
    /// The identity transform has no translation or rotation and is often used
    /// as a neutral element in transformations.
    ///
    /// The timestamp is set to the static timestamp value for the active
    /// timestamp type (`Timestamp::zero()` by default).
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let identity = Transform::<Timestamp>::identity();
    /// let transform = Transform {
    ///     translation: Vector3::zero(),
    ///     rotation: Quaternion::identity(),
    ///     timestamp: Timestamp::zero(),
    ///     parent: "".into(),
    ///     child: "".into(),
    /// };
    ///
    /// assert_eq!(identity, transform);
    /// ```
    #[must_use]
    pub fn identity() -> Self {
        Transform {
            translation: Vector3::zero(),
            rotation: Quaternion::identity(),
            timestamp: T::static_timestamp(),
            parent: String::new(),
            child: String::new(),
        }
    }

    /// Computes the inverse of the transform.
    ///
    /// Returns a new `Transform` that is the inverse of the current transform.
    ///
    /// # Errors
    ///
    /// This function returns a `TransformError` if:
    /// - The quaternion normalization fails, resulting in a `QuaternionError`.
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
    ///     translation: Vector3::new(1.0, 2.0, 3.0),
    ///     rotation: Quaternion::new(0.0, 1.0, 0.0, 0.0).normalize().unwrap(),
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
    /// let identity = Transform::<Timestamp>::identity();
    /// let result = (transform * inverse).unwrap();
    /// assert_eq!(result.translation, identity.translation);
    /// assert_eq!(result.rotation, identity.rotation);
    /// ```
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

impl<T> Mul for Transform<T>
where
    T: TimePoint,
{
    type Output = Result<Transform<T>, TransformError>;

    /// Composes two transforms: `t_a_b * t_b_c` yields `t_a_c`.
    ///
    /// The left-hand side's child frame must equal the right-hand side's
    /// parent frame; any other pairing is not a valid composition and
    /// returns an error. Unless one operand is static, both timestamps
    /// must be equal.
    #[inline]
    fn mul(
        self,
        rhs: Transform<T>,
    ) -> Self::Output {
        let is_self_static = self.timestamp.is_static();
        let is_rhs_static = rhs.timestamp.is_static();

        if !is_self_static && !is_rhs_static && self.timestamp != rhs.timestamp {
            return Err(TransformError::TimestampMismatch(
                self.timestamp.as_seconds()?,
                rhs.timestamp.as_seconds()?,
            ));
        }

        if self.child == rhs.child {
            return Err(TransformError::SameFrameMultiplication);
        }

        if self.child != rhs.parent {
            return Err(TransformError::IncompatibleFrames);
        }

        let r = self.rotation * rhs.rotation;
        let t = self.rotation.rotate_vector(rhs.translation) + self.translation;

        Ok(Transform {
            translation: t,
            rotation: r,
            timestamp: if is_self_static {
                rhs.timestamp
            } else {
                self.timestamp
            },
            parent: self.parent,
            child: rhs.child,
        })
    }
}

impl<T> AbsDiffEq for Transform<T>
where
    T: TimePoint,
{
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::EPSILON
    }

    /// Compares translation and rotation within `epsilon`; frames and
    /// timestamps must match exactly. Use this (via
    /// `approx::assert_abs_diff_eq!`) for tolerant comparison of computed
    /// transforms — `==` is exact, bitwise equality.
    fn abs_diff_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
    ) -> bool {
        self.translation.abs_diff_eq(&other.translation, epsilon)
            && self.rotation.abs_diff_eq(&other.rotation, epsilon)
            && self.timestamp == other.timestamp
            && self.parent == other.parent
            && self.child == other.child
    }
}

#[cfg(test)]
mod tests;
