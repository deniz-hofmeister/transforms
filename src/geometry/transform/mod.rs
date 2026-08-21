//! Rigid-body transforms between coordinate frames, with composition, inversion, and interpolation.

use crate::{
    geometry::{Quaternion, Vector3},
    time::{Stamp, TimePoint, Timestamp},
};
use alloc::string::String;
use approx::{AbsDiffEq, RelativeEq};
use core::ops::Mul;
pub use error::TransformError;
pub use traits::{Localized, Transformable};

mod error;
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

/// Where a child frame sits inside its parent frame, and when that holds.
///
/// A transform with frames `(parent, child)` maps child-frame coordinates
/// into the parent frame. It carries a translation, a rotation, and a
/// [`Stamp`]: one instant, or all time.
///
/// [`Transform::new`] and [`Transform::static_between`] build one from
/// components; both reject non-finite components and rotations whose norm
/// deviates from 1 by more than [`UNIT_NORM_TOLERANCE`], and the fields are
/// private so a built transform cannot be edited back into an invalid state.
/// Read the components with [`translation`](Self::translation),
/// [`rotation`](Self::rotation), [`timestamp`](Self::timestamp),
/// [`parent`](Self::parent) and [`child`](Self::child); to change one, build
/// a new transform.
///
/// Transforms *derived* from validated ones — [`inverse`](Self::inverse),
/// [`interpolate`](Self::interpolate), `*` composition, and every registry
/// lookup — are deliberately not re-validated: rotation norms drift by a few
/// ulps per composition, so re-checking a long chain would reject legitimate
/// results. A derived transform is therefore *usually* valid but not
/// guaranteed to be: composing operands that each sit at the edge of the
/// tolerance walks past it, and extreme magnitudes overflow a translation to
/// infinity. [`validate`](Self::validate) is there for exactly that — a
/// transform whose provenance a caller does not control — and
/// `Registry::add_transform` runs it, so a derived transform cannot re-enter
/// storage unchecked.
///
/// With the optional `serde` feature, this type implements `Serialize` and
/// `Deserialize` (the docs.rs listing cannot banner derive-generated impls).
/// Deserialization runs the same validation as the constructors, so a
/// transform read off the wire is valid too. Serialization does not: it writes
/// the fields as they stand, so a *derived* transform that drifted past the
/// tolerance encodes without complaint and fails on the consumer's decode.
/// Call [`validate`](Self::validate) before persisting or publishing a derived
/// transform.
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
    /// deviates from 1 by more than [`UNIT_NORM_TOLERANCE`]. Both would
    /// otherwise corrupt every lookup the transform takes part in without any
    /// error.
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

    /// Assembles a transform without validating it.
    ///
    /// For values derived from already-validated transforms — interpolation,
    /// inversion, composition, the registry's synthesized identity — where
    /// re-validating would reject legitimate results whose rotation norm has
    /// drifted within tolerance across a long chain. Every input reaching
    /// this constructor must come from a transform that was validated once.
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

        let range = to_time.duration_since(from_time)?;
        if range.is_zero() {
            return Ok(from.clone());
        }

        let diff = timestamp.duration_since(from_time)?;
        let ratio = diff.as_secs_f64() / range.as_secs_f64();

        Ok(Self::unvalidated(
            from.parent.clone(),
            from.child.clone(),
            (1.0 - ratio) * from.translation + ratio * to.translation,
            from.rotation.slerp(to.rotation, ratio),
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
        let q = self.rotation.normalize()?;
        let inverse_rotation = q.conjugate();
        let inverse_translation = -1.0 * (inverse_rotation.rotate_vector(self.translation));

        if !inverse_translation.x.is_finite()
            || !inverse_translation.y.is_finite()
            || !inverse_translation.z.is_finite()
        {
            return Err(TransformError::NonFiniteValues);
        }

        Ok(Self::unvalidated(
            self.child.clone(),
            self.parent.clone(),
            inverse_translation,
            inverse_rotation,
            self.timestamp,
        ))
    }

    /// Replaces the stamp, for the registry's re-stamping of a resolved
    /// chain: a lookup answers for the *requested* instant, whatever mix of
    /// static and dynamic edges produced the answer.
    #[must_use]
    pub(crate) fn restamped(
        mut self,
        timestamp: Stamp<T>,
    ) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Composes without the timestamp-agreement check, for callers that
    /// deliberately combine transforms resolved at different times (the
    /// time-travel lookup). Frame compatibility is still enforced. The
    /// result carries `self`'s timestamp; the caller re-stamps it.
    pub(crate) fn compose_ignoring_time(
        self,
        rhs: Transform<T>,
    ) -> Result<Transform<T>, TransformError> {
        if self.child == rhs.child {
            return Err(TransformError::SameFrameMultiplication { frame: rhs.child });
        }

        if self.child != rhs.parent {
            return Err(TransformError::IncompatibleFrames {
                expected: self.child,
                found: rhs.parent,
            });
        }

        let rotation = self.rotation * rhs.rotation;
        let translation = self.rotation.rotate_vector(rhs.translation) + self.translation;

        Ok(Self::unvalidated(
            self.parent,
            rhs.child,
            translation,
            rotation,
            self.timestamp,
        ))
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
    ///
    /// The result is not re-validated: rotation norms drift by a few ulps per
    /// composition, and rejecting that drift would fail legitimate long
    /// chains. Composing operands of extreme magnitude can therefore overflow
    /// the translation to infinity — [`Transform::validate`] catches it.
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

        let mut result = self.compose_ignoring_time(rhs)?;
        result.timestamp = timestamp;
        Ok(result)
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
