//! The numbers of a rigid-body transform, without frames or a stamp.

use super::TransformError;
use crate::{
    geometry::{Quaternion, Vector3},
    time::{TimeError, TimePoint},
};
use core::ops::Mul;

/// A rotation followed by a translation: what a [`Transform`](super::Transform)
/// computes with, stripped of the frames and stamp it checks.
///
/// All transform arithmetic lives here, once. `Transform` checks frames and
/// stamps, then delegates the numbers. The buffer stores dynamic history as
/// isometries under timestamp keys, and a registry lookup composes them
/// along its walk with frame names borrowed from the buffers, so that it
/// allocates names once, for the answer, rather than once per hop.
///
/// Like a derived `Transform`, an isometry is never re-validated: each one
/// is copied from a validated transform or computed from such copies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Isometry {
    pub(crate) translation: Vector3,
    pub(crate) rotation: Quaternion,
}

impl Isometry {
    /// Maps every point onto itself.
    pub(crate) fn identity() -> Self {
        Self {
            translation: Vector3::zero(),
            rotation: Quaternion::identity(),
        }
    }

    /// The same relationship read the other way round.
    ///
    /// The rotation is normalized first: inverting a rotation that has
    /// drifted off the unit sphere would scale every value the result is
    /// applied to.
    ///
    /// # Errors
    ///
    /// Returns `TransformError::QuaternionError` if the rotation cannot be
    /// normalized, and `TransformError::NonFiniteValues` if the inverted
    /// translation is not finite.
    pub(crate) fn inverse(self) -> Result<Self, TransformError> {
        let rotation = self.rotation.normalize()?.conjugate();
        let translation = -1.0 * (rotation.rotate_vector(self.translation));

        if !translation.x.is_finite() || !translation.y.is_finite() || !translation.z.is_finite() {
            return Err(TransformError::NonFiniteValues);
        }

        Ok(Self {
            translation,
            rotation,
        })
    }

    /// Interpolates at `timestamp` between `self`, holding at `from_time`,
    /// and `to`, holding at `to_time`: linearly in translation, by slerp in
    /// rotation. A zero span returns `self` unchanged.
    ///
    /// The caller has checked `from_time <= timestamp <= to_time`; there is
    /// no extrapolation.
    ///
    /// # Errors
    ///
    /// Returns the `TimeError` of a span — between the endpoints, or from
    /// `from_time` to `timestamp` — too large to represent as a `Duration`.
    pub(crate) fn interpolate<T>(
        self,
        to: Self,
        from_time: T,
        to_time: T,
        timestamp: T,
    ) -> Result<Self, TimeError>
    where
        T: TimePoint,
    {
        let range = to_time.duration_since(from_time)?;
        if range.is_zero() {
            return Ok(self);
        }

        let diff = timestamp.duration_since(from_time)?;
        let ratio = diff.as_secs_f64() / range.as_secs_f64();

        Ok(Self {
            translation: (1.0 - ratio) * self.translation + ratio * to.translation,
            rotation: self.rotation.slerp(to.rotation, ratio),
        })
    }
}

impl Mul for Isometry {
    type Output = Isometry;

    /// Composes two isometries: `t_a_b * t_b_c` yields `t_a_c`.
    #[inline]
    fn mul(
        self,
        rhs: Isometry,
    ) -> Isometry {
        Isometry {
            translation: self.rotation.rotate_vector(rhs.translation) + self.translation,
            rotation: self.rotation * rhs.rotation,
        }
    }
}
