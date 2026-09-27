//! Rigid-body transforms between coordinate frames, with composition, inversion, and interpolation.

use crate::{
    geometry::{Quaternion, Vector3},
    time::{Stamp, TimePoint, Timestamp},
};
use alloc::string::String;
use approx::{AbsDiffEq, RelativeEq};
use core::ops::Mul;
pub use error::TransformError;
pub(crate) use isometry::Isometry;
pub use traits::{Localized, Transformable};

mod error;
mod isometry;
mod traits;

/// The accepted deviation of a rotation's norm from 1, applied by
/// [`Transform::validate`].
///
/// That is the check behind [`Transform::new`],
/// [`Transform::static_between`], the `Deserialize` impl, and every
/// [`Registry::add_transform`](crate::Registry::add_transform), so the
/// tolerance gates every rotation that reaches storage.
///
/// Loose enough to accept unit quaternions that were stored or
/// transmitted as `f32` and widened to `f64`, tight enough to reject
/// genuinely denormalized rotations, which would otherwise corrupt every
/// lookup they take part in without any error.
pub const UNIT_NORM_TOLERANCE: f64 = 1e-6;

/// Maps child-frame coordinates into a parent frame at a given stamp.
///
/// Positions are rotated first, then translated. [`Transform::new`] and
/// [`Transform::static_between`] reject non-finite components and rotations
/// outside [`UNIT_NORM_TOLERANCE`]. Fields are private; use the accessors to
/// read components and a constructor to change them.
///
/// Derived transforms (composition, interpolation, inversion, and registry
/// lookups) are not re-validated. Rotation norms can drift beyond the tolerance
/// and translations can overflow. Use [`validate`](Self::validate) when needed;
/// [`Registry::add_transform`](crate::Registry::add_transform) always validates
/// before storage.
///
/// With `serde`, this type implements `Serialize` and `Deserialize`.
/// Deserialization validates; serialization writes the components unchanged.
/// Validate a derived result before publishing it to avoid a decode failure.
///
/// [`Registry::get_transform_at`](crate::Registry::get_transform_at) returns
/// cross-time geometry with only the target stamp. Retain the source instant
/// and follow that method's application restrictions; numeric validation does
/// not check temporal provenance.
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Quaternion, Transform, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// let t_map_base: Transform = Transform::new(
///     "map",
///     "base",
///     Vector3::new(1.0, 0.0, 0.0),
///     Quaternion::identity(),
///     Stamp::At(Timestamp::zero()),
/// )
/// .unwrap();
///
/// assert_eq!(t_map_base.parent(), "map");
/// assert_eq!(t_map_base.translation(), Vector3::new(1.0, 0.0, 0.0));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        try_from = "TransformRepr<T>",
        bound(deserialize = "T: TimePoint + serde::Deserialize<'de>")
    )
)]
#[non_exhaustive]
pub struct Transform<T = Timestamp>
where
    T: TimePoint,
{
    translation: Vector3,
    rotation: Quaternion,
    timestamp: Stamp<T>,
    parent: String,
    child: String,
}

impl<T> Transform<T>
where
    T: TimePoint,
{
    /// Builds a validated transform mapping `child`-frame coordinates into
    /// the `parent` frame.
    ///
    /// `timestamp` says when it holds: `Stamp::At(t)` for a dynamic sample,
    /// `Stamp::Static` for a fixed relationship (see
    /// [`Transform::static_between`], which is this constructor with
    /// `Stamp::Static`).
    ///
    /// # Errors
    ///
    /// Returns `TransformError::NonFiniteValues` if any component is NaN or
    /// infinite, and `TransformError::NonUnitRotation` if the rotation's norm
    /// deviates from 1 by more than [`UNIT_NORM_TOLERANCE`].
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     errors::TransformError,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let stamp = Stamp::At(Timestamp::zero());
    /// let valid: Transform = Transform::new(
    ///     "map",
    ///     "base",
    ///     Vector3::zero(),
    ///     Quaternion::identity(),
    ///     stamp,
    /// )
    /// .unwrap();
    /// assert_eq!(valid.child(), "base");
    ///
    /// let denormalized = Transform::new(
    ///     "map",
    ///     "base",
    ///     Vector3::zero(),
    ///     Quaternion::from_wxyz(1.01, 0.0, 0.0, 0.0),
    ///     stamp,
    /// );
    /// assert!(matches!(
    ///     denormalized,
    ///     Err(TransformError::NonUnitRotation(_))
    /// ));
    /// ```
    pub fn new(
        parent: &str,
        child: &str,
        translation: Vector3,
        rotation: Quaternion,
        timestamp: Stamp<T>,
    ) -> Result<Self, TransformError> {
        let transform = Self::unvalidated(
            parent.into(),
            child.into(),
            translation,
            rotation,
            timestamp,
        );
        transform.validate()?;
        Ok(transform)
    }

    /// Builds a validated static transform between two frames: valid for all
    /// time.
    ///
    /// The transform carries `Stamp::Static`, so the registry serves it for
    /// any requested time and never expires it. Use this for fixed
    /// relationships like sensor mounts.
    ///
    /// # Errors
    ///
    /// The same as [`Transform::new`], of which this is the `Stamp::Static`
    /// case: `TransformError::NonFiniteValues` for a non-finite component and
    /// `TransformError::NonUnitRotation` for a rotation outside
    /// [`UNIT_NORM_TOLERANCE`].
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::{Quaternion, Transform, Vector3};
    ///
    /// let mount: Transform = Transform::static_between(
    ///     "base",
    ///     "camera",
    ///     Vector3::new(0.1, 0.0, 0.5),
    ///     Quaternion::identity(),
    /// )
    /// .unwrap();
    /// assert!(mount.timestamp().is_static());
    /// ```
    pub fn static_between(
        parent: &str,
        child: &str,
        translation: Vector3,
        rotation: Quaternion,
    ) -> Result<Self, TransformError> {
        Self::new(parent, child, translation, rotation, Stamp::Static)
    }

    /// Assembles components without validation.
    ///
    /// For the registry's identity and for values derived from already
    /// validated transforms — interpolation, inversion, composition — where
    /// re-validating would reject legitimate norm drift. Every caller must
    /// be able to name the validated transform its inputs came from.
    pub(crate) fn unvalidated(
        parent: String,
        child: String,
        translation: Vector3,
        rotation: Quaternion,
        timestamp: Stamp<T>,
    ) -> Self {
        Self {
            translation,
            rotation,
            timestamp,
            parent,
            child,
        }
    }

    /// The translational component: where the child frame's origin sits in
    /// the parent frame.
    #[must_use]
    pub fn translation(&self) -> Vector3 {
        self.translation
    }

    /// The rotational component: how the child frame is oriented in the
    /// parent frame.
    #[must_use]
    pub fn rotation(&self) -> Quaternion {
        self.rotation
    }

    /// When the transform is valid: at one instant (`Stamp::At`) or for all
    /// time (`Stamp::Static`).
    #[must_use]
    pub fn timestamp(&self) -> Stamp<T> {
        self.timestamp
    }

    /// The target frame; the transform maps child-frame coordinates into
    /// this frame.
    #[must_use]
    pub fn parent(&self) -> &str {
        &self.parent
    }

    /// The source frame whose coordinates are mapped into the parent frame.
    #[must_use]
    pub fn child(&self) -> &str {
        &self.child
    }

    /// The numbers without the frames and stamp, for the arithmetic that
    /// [`Isometry`] carries out on the transform's behalf.
    pub(crate) fn isometry(&self) -> Isometry {
        Isometry {
            translation: self.translation,
            rotation: self.rotation,
        }
    }

    /// Checks that the transform is usable for composition and lookup.
    ///
    /// A valid transform has finite translation and rotation components and a
    /// rotation whose norm is within [`UNIT_NORM_TOLERANCE`] of `1.0`. The
    /// constructors run this, so a transform built through them passes;
    /// results of `*`, [`inverse`](Self::inverse) and
    /// [`interpolate`](Self::interpolate) are not re-checked, and neither is
    /// a transform a third-party [`Transformable`] implementation receives
    /// from elsewhere — call this when that provenance matters.
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
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let transform: Transform = Transform::new(
    ///     "a",
    ///     "b",
    ///     Vector3::new(1.0, 2.0, 3.0),
    ///     Quaternion::identity(),
    ///     Stamp::At(Timestamp::zero()),
    /// )
    /// .unwrap();
    ///
    /// assert!(transform.validate().is_ok());
    /// assert!(transform.inverse().unwrap().validate().is_ok());
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
        if (norm - 1.0).abs() > UNIT_NORM_TOLERANCE {
            return Err(TransformError::NonUnitRotation(norm));
        }

        Ok(())
    }

    /// Interpolates between two transforms at a given timestamp.
    ///
    /// Returns a new `Transform` that is the interpolation between `from` and `to`
    /// at the specified `timestamp`. If both endpoints share a timestamp, a
    /// clone of `from` is returned.
    ///
    /// # Errors
    ///
    /// Returns `TransformError::StaticInterpolation` if either endpoint is
    /// `Stamp::Static` — a static transform is valid for all time, so it is
    /// never an interpolation endpoint.
    ///
    /// Returns `TransformError::TimestampOutOfRange` if the timestamp is
    /// outside the range of `from` and `to` (there is no extrapolation),
    /// `TransformError::TimestampMismatch` if the endpoints are swapped, and
    /// `TransformError::IncompatibleFrames` if the frames do not match.
    ///
    /// Returns `TransformError::TimestampError` if a time span needed for
    /// the interpolation — between the endpoints, or from `from` to the
    /// requested timestamp — is too large to represent as a `Duration`.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let from: Transform = Transform::new(
    ///     "a",
    ///     "b",
    ///     Vector3::zero(),
    ///     Quaternion::identity(),
    ///     Stamp::At(Timestamp::zero()),
    /// )
    /// .unwrap();
    /// let to: Transform = Transform::new(
    ///     "a",
    ///     "b",
    ///     Vector3::new(2.0, 2.0, 2.0),
    ///     Quaternion::identity(),
    ///     Stamp::At(Timestamp::from_nanos(2_000_000_000)),
    /// )
    /// .unwrap();
    /// let timestamp = Timestamp::from_nanos(1_000_000_000);
    ///
    /// let interpolated = Transform::interpolate(&from, &to, timestamp).unwrap();
    ///
    /// assert_eq!(interpolated.translation(), Vector3::new(1.0, 1.0, 1.0));
    /// assert_eq!(interpolated.timestamp(), Stamp::At(timestamp));
    /// ```
    pub fn interpolate(
        from: &Transform<T>,
        to: &Transform<T>,
        timestamp: T,
    ) -> Result<Transform<T>, TransformError> {
        let (Stamp::At(from_time), Stamp::At(to_time)) = (from.timestamp, to.timestamp) else {
            return Err(TransformError::StaticInterpolation);
        };
        if from_time > to_time {
            return Err(TransformError::TimestampMismatch {
                lhs: from_time.as_seconds_lossy(),
                rhs: to_time.as_seconds_lossy(),
            });
        }
        if timestamp < from_time || timestamp > to_time {
            return Err(TransformError::TimestampOutOfRange {
                requested: timestamp.as_seconds_lossy(),
                start: from_time.as_seconds_lossy(),
                end: to_time.as_seconds_lossy(),
            });
        }
        if from.child != to.child || from.parent != to.parent {
            return Err(TransformError::IncompatibleFrames {
                expected: alloc::format!("{} -> {}", from.parent, from.child),
                found: alloc::format!("{} -> {}", to.parent, to.child),
            });
        }

        let isometry = from
            .isometry()
            .interpolate(to.isometry(), from_time, to_time, timestamp)?;
        Ok(Self::unvalidated(
            from.parent.clone(),
            from.child.clone(),
            isometry.translation,
            isometry.rotation,
            Stamp::At(timestamp),
        ))
    }

    /// Computes the inverse of the transform: the same relationship read the
    /// other way round, with the frames swapped.
    ///
    /// The rotation is normalized first — inverting a rotation that has
    /// drifted off the unit sphere would scale every value the result is
    /// applied to.
    ///
    /// # Errors
    ///
    /// Returns `TransformError::QuaternionError` if the rotation cannot be
    /// normalized, which a transform straight from a constructor cannot reach:
    /// its norm was checked there. `TransformError::NonFiniteValues` is
    /// returned if the inverted translation is not finite, and that one is
    /// reachable from a constructor-built transform too — rotating a
    /// translation whose components sit near `f64::MAX` overflows it — as well
    /// as from one composed out of extreme-magnitude operands, which `*` does
    /// not re-check.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let transform: Transform = Transform::new(
    ///     "a",
    ///     "b",
    ///     Vector3::new(1.0, 2.0, 3.0),
    ///     Quaternion::from_wxyz(0.0, 1.0, 0.0, 0.0),
    ///     Stamp::At(Timestamp::zero()),
    /// )
    /// .unwrap();
    ///
    /// let inverse = transform.clone().inverse().unwrap();
    ///
    /// // The inverse has the frames swapped ...
    /// assert_eq!(inverse.parent(), "b");
    /// assert_eq!(inverse.child(), "a");
    ///
    /// // ... and composing the two yields the identity.
    /// let result = (transform * inverse).unwrap();
    /// assert_eq!(result.translation(), Vector3::zero());
    /// assert_eq!(result.rotation(), Quaternion::identity());
    /// ```
    pub fn inverse(&self) -> Result<Self, TransformError> {
        let isometry = self.isometry().inverse()?;
        Ok(Self::unvalidated(
            self.child.clone(),
            self.parent.clone(),
            isometry.translation,
            isometry.rotation,
            self.timestamp,
        ))
    }
}

/// The frame rule of composition, shared by `Transform`'s `*` and the
/// registry's lookup: `t_a_b * t_b_c` composes only when the left-hand child
/// is the right-hand parent, and never onto the right-hand child itself.
///
/// # Errors
///
/// Returns [`TransformError::SameFrameMultiplication`] when the child frames
/// match, and [`TransformError::IncompatibleFrames`] unless
/// `lhs_child == rhs_parent`.
pub(crate) fn ensure_composable(
    lhs_child: &str,
    rhs_parent: &str,
    rhs_child: &str,
) -> Result<(), TransformError> {
    if lhs_child == rhs_child {
        return Err(TransformError::SameFrameMultiplication {
            frame: rhs_child.into(),
        });
    }

    if lhs_child != rhs_parent {
        return Err(TransformError::IncompatibleFrames {
            expected: lhs_child.into(),
            found: rhs_parent.into(),
        });
    }

    Ok(())
}

impl<T> Mul for Transform<T>
where
    T: TimePoint,
{
    type Output = Result<Transform<T>, TransformError>;

    /// Composes two transforms: `t_a_b * t_b_c` yields `t_a_c`.
    ///
    /// The result is not re-validated; see [`Transform::validate`].
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::TimestampMismatch`] for unequal timestamps,
    /// unless either operand is static; [`TransformError::IncompatibleFrames`]
    /// unless `self.child() == rhs.parent()`; and
    /// [`TransformError::SameFrameMultiplication`] when the child frames match.
    /// The last restriction also rejects a right-hand identity transform.
    #[inline]
    fn mul(
        self,
        rhs: Transform<T>,
    ) -> Self::Output {
        let timestamp = match (self.timestamp, rhs.timestamp) {
            (Stamp::Static, rhs_stamp) => rhs_stamp,
            (self_stamp, Stamp::Static) => self_stamp,
            (Stamp::At(lhs), Stamp::At(rhs_time)) => {
                if lhs != rhs_time {
                    return Err(TransformError::TimestampMismatch {
                        lhs: lhs.as_seconds_lossy(),
                        rhs: rhs_time.as_seconds_lossy(),
                    });
                }
                Stamp::At(lhs)
            }
        };

        ensure_composable(&self.child, &rhs.parent, &rhs.child)?;

        let isometry = self.isometry() * rhs.isometry();
        Ok(Self::unvalidated(
            self.parent,
            rhs.child,
            isometry.translation,
            isometry.rotation,
            timestamp,
        ))
    }
}

/// The field-for-field record serde reads a [`Transform`] from, converted
/// through [`TryFrom`] so that a deserialized transform runs the same
/// validation as [`Transform::new`]. Renamed to `Transform` so the wire
/// format is unchanged.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(rename = "Transform")]
#[serde(bound(deserialize = "T: TimePoint + serde::Deserialize<'de>"))]
struct TransformRepr<T>
where
    T: TimePoint,
{
    translation: Vector3,
    rotation: Quaternion,
    timestamp: Stamp<T>,
    parent: String,
    child: String,
}

#[cfg(feature = "serde")]
impl<T> TryFrom<TransformRepr<T>> for Transform<T>
where
    T: TimePoint,
{
    type Error = TransformError;

    fn try_from(repr: TransformRepr<T>) -> Result<Self, Self::Error> {
        // The body of `Transform::new`, moving the decoded frame names instead
        // of re-allocating them: the assembled value is validated before it
        // escapes, so `unvalidated` never hands out an unchecked transform.
        let transform = Self::unvalidated(
            repr.parent,
            repr.child,
            repr.translation,
            repr.rotation,
            repr.timestamp,
        );
        transform.validate()?;
        Ok(transform)
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
    /// transforms — `==` is exact IEEE 754 equality with no tolerance
    /// (`NaN` components never compare equal, and `0.0 == -0.0`), not a
    /// bit-level comparison.
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

impl<T> RelativeEq for Transform<T>
where
    T: TimePoint,
{
    fn default_max_relative() -> Self::Epsilon {
        f64::EPSILON
    }

    /// Compares translation and rotation with relative tolerance; frames and
    /// timestamps must match exactly.
    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.translation
            .relative_eq(&other.translation, epsilon, max_relative)
            && self
                .rotation
                .relative_eq(&other.rotation, epsilon, max_relative)
            && self.timestamp == other.timestamp
            && self.parent == other.parent
            && self.child == other.child
    }
}

#[cfg(test)]
mod tests;
