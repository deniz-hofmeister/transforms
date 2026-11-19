//! The numbers of a rigid-body transform, without frames or a stamp.

use super::{NumericError, Rotation, Vector3};

/// A rotation followed by a translation: what a
/// [`Transform`](super::Transform) computes with, stripped of the frames
/// and stamp it checks.
///
/// All transform arithmetic lives here, once. `Transform` checks frames and
/// stamps, then delegates the numbers; the registry stores histories of
/// isometries and composes them along a chain, materializing one labelled
/// result for the answer.
///
/// Every isometry is valid: its translation is finite and its rotation is a
/// [`Rotation`]. Each operation below re-establishes that or fails, so no
/// caller re-validates one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Isometry {
    translation: Vector3,
    rotation: Rotation,
}

impl Isometry {
    /// Maps every point onto itself.
    pub(crate) const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        rotation: Rotation::IDENTITY,
    };

    /// Pairs a translation with a rotation.
    ///
    /// # Errors
    ///
    /// Returns `NumericError::NonFinite` if a translation component is NaN
    /// or infinite.
    pub(crate) fn new(
        translation: Vector3,
        rotation: Rotation,
    ) -> Result<Self, NumericError> {
        if !translation.is_finite() {
            return Err(NumericError::NonFinite);
        }
        Ok(Self {
            translation,
            rotation,
        })
    }

    /// Where the source frame's origin sits in the target frame.
    pub(crate) fn translation(self) -> Vector3 {
        self.translation
    }

    /// How the source frame is oriented in the target frame.
    pub(crate) fn rotation(self) -> Rotation {
        self.rotation
    }

    /// Composes two isometries: with `self` mapping `b` into `a` and `rhs`
    /// mapping `c` into `b`, the result maps `c` into `a`.
    ///
    /// # Errors
    ///
    /// Returns `NumericError::NonFinite` if the composed translation
    /// overflows, and the `NumericError` of [`Rotation::product`].
    #[inline]
    pub(crate) fn compose(
        self,
        rhs: Self,
    ) -> Result<Self, NumericError> {
        let rotated = self.rotation.rotate(rhs.translation);
        let translation = Vector3::new(
            rotated.x + self.translation.x,
            rotated.y + self.translation.y,
            rotated.z + self.translation.z,
        );
        Self::new(translation, self.rotation.product(rhs.rotation)?)
    }

    /// The same relationship read the other way round.
    ///
    /// The rotation is the conjugate, which is exact. The translation is
    /// the rotated original, negated and divided by the rotation's squared
    /// norm: that divides out the scale the rotation applies while its norm
    /// sits a few ulps from 1, without a square root.
    ///
    /// # Errors
    ///
    /// Returns `NumericError::NonFinite` if the inverted translation
    /// overflows, which translations near `f64::MAX` reach.
    #[inline]
    pub(crate) fn inverse(self) -> Result<Self, NumericError> {
        let rotation = self.rotation.conjugate();
        let rotated = rotation.rotate(self.translation);
        let scale = -1.0 / self.rotation.norm_squared();
        Self::new(
            Vector3::new(scale * rotated.x, scale * rotated.y, scale * rotated.z),
            rotation,
        )
    }

    /// The isometry `ratio` of the way from `self` to `to`: linear in
    /// translation, slerp in rotation. `ratio` is clamped to `[0.0, 1.0]`
    /// by the slerp; callers pass a ratio inside it.
    ///
    /// # Errors
    ///
    /// Returns `NumericError::NonFinite` if the blended translation
    /// overflows, and the `NumericError` of [`Rotation::slerp`].
    #[inline]
    pub(crate) fn interpolate(
        self,
        to: Self,
        ratio: f64,
    ) -> Result<Self, NumericError> {
        let keep = 1.0 - ratio;
        let translation = Vector3::new(
            keep * self.translation.x + ratio * to.translation.x,
            keep * self.translation.y + ratio * to.translation.y,
            keep * self.translation.z + ratio * to.translation.z,
        );
        Self::new(translation, self.rotation.slerp(to.rotation, ratio)?)
    }
}

#[cfg(test)]
mod tests;
