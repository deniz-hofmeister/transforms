//! Unit quaternions.

use super::{NumericError, TransformError, Vector3};

/// How far the squared norm of a [`Rotation`] may sit from 1.
///
/// Wide enough that rebuilding a value from its own components never
/// changes a bit, and that a product of two rotations almost always stays
/// inside it; narrow enough that the remaining error is rounding scale.
const BAND: f64 = 8.0 * f64::EPSILON;

/// How far the norm of the components given to [`Rotation::from_wxyz`] may
/// sit from 1.
///
/// Loose enough to accept a unit quaternion that was stored or transmitted
/// as `f32`, tight enough to reject a rotation that was never normalized.
const INPUT_TOLERANCE: f64 = 1e-6;

/// Four raw quaternion components, scalar first, with no invariant: the
/// working form of the arithmetic below.
#[derive(Clone, Copy, PartialEq)]
struct Quaternion {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

impl Quaternion {
    /// The squared norm, summed in component order. Every norm in this
    /// module is measured by this expression, so a value that passed the
    /// band once passes it again.
    #[inline]
    fn norm_squared(self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// The Hamilton product: `self * other` applies `other` first.
    #[inline]
    fn product(
        self,
        other: Self,
    ) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    #[inline]
    fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    #[inline]
    fn scale(
        self,
        factor: f64,
    ) -> Self {
        Self {
            w: self.w * factor,
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    /// The blend `a * wa + b * wb`, component-wise.
    #[inline]
    fn weighted_sum(
        a: Self,
        wa: f64,
        b: Self,
        wb: f64,
    ) -> Self {
        Self {
            w: a.w * wa + b.w * wb,
            x: a.x * wa + b.x * wb,
            y: a.y * wa + b.y * wb,
            z: a.z * wa + b.z * wb,
        }
    }

    /// The one rule that turns raw components into a [`Rotation`].
    ///
    /// Components whose squared norm is inside the band are kept bit for
    /// bit. Anything else is scaled by `1 / sqrt(n²)` and must then be
    /// finite and inside the band.
    #[inline]
    fn into_rotation(self) -> Result<Rotation, NumericError> {
        if (self.norm_squared() - 1.0).abs() <= BAND {
            return Ok(Rotation(self));
        }
        let scaled = self.scale(1.0 / libm::sqrt(self.norm_squared()));
        let norm_squared = scaled.norm_squared();
        if !norm_squared.is_finite() {
            return Err(NumericError::NonFinite);
        }
        if (norm_squared - 1.0).abs() > BAND {
            return Err(NumericError::NonUnitRotation {
                norm: libm::sqrt(norm_squared),
            });
        }
        Ok(Rotation(scaled))
    }
}

/// A rotation in 3D space: a unit quaternion in scalar-first
/// `(w, x, y, z)` order.
///
/// Every value is finite and has a squared norm within `8 * f64::EPSILON`
/// of 1, measured as `w*w + x*x + y*y + z*z`. [`Rotation::from_wxyz`]
/// establishes that, and every operation of this crate that produces a
/// rotation re-establishes it, so no rotation needs to be checked or
/// normalized by its user.
///
/// With right-handed axes, positive rotations follow the right-hand rule.
/// `q` and `-q` describe the same rotation and do not compare equal:
/// equality is exact component-wise comparison.
///
/// With `serde`, a rotation encodes as its four components and decodes
/// through [`Rotation::from_wxyz`], with the same checks.
///
/// # Examples
///
/// ```
/// use transforms::{Rotation, Vector3};
///
/// // A quarter turn about z.
/// let half = core::f64::consts::FRAC_1_SQRT_2;
/// let quarter_turn = Rotation::from_wxyz(half, 0.0, 0.0, half).unwrap();
///
/// let v = quarter_turn.rotate(Vector3::new(1.0, 0.0, 0.0));
/// assert!(v.x.abs() < 1e-15);
/// assert!((v.y - 1.0).abs() < 1e-15);
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct Rotation(Quaternion);

// Prints the four components under the public name, not the private
// working type that holds them.
impl core::fmt::Debug for Rotation {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        f.debug_struct("Rotation")
            .field("w", &self.0.w)
            .field("x", &self.0.x)
            .field("y", &self.0.y)
            .field("z", &self.0.z)
            .finish()
    }
}

impl Rotation {
    /// No rotation: `(1, 0, 0, 0)`.
    pub const IDENTITY: Self = Self(Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });

    /// Builds a rotation from quaternion components, scalar first.
    ///
    /// The norm of the components must be within `1e-6` of 1, which admits
    /// a unit quaternion that travelled as `f32`. Components whose squared
    /// norm is within `8 * f64::EPSILON` of 1 are stored unchanged; any
    /// other accepted input is scaled to unit length, so the stored
    /// components can differ from the given ones in their last digits.
    ///
    /// The order is positional and not checked: the scalar-last components
    /// of a unit quaternion are a unit quaternion too, and generally
    /// describe a different rotation.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::NonFinite`] if a component is NaN or
    /// infinite, and [`TransformError::NonUnitRotation`] if the norm is
    /// further than `1e-6` from 1.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Rotation, TransformError};
    ///
    /// let identity = Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0).unwrap();
    /// assert_eq!(identity, Rotation::IDENTITY);
    ///
    /// // A rotation that travelled as `f32` is brought to unit length.
    /// let widened = f64::from(core::f32::consts::FRAC_1_SQRT_2);
    /// let rotation = Rotation::from_wxyz(widened, widened, 0.0, 0.0).unwrap();
    /// assert_ne!(rotation.w(), widened);
    ///
    /// assert!(matches!(
    ///     Rotation::from_wxyz(1.01, 0.0, 0.0, 0.0),
    ///     Err(TransformError::NonUnitRotation { .. })
    /// ));
    /// ```
    pub fn from_wxyz(
        w: f64,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<Self, TransformError> {
        if !(w.is_finite() && x.is_finite() && y.is_finite() && z.is_finite()) {
            return Err(TransformError::NonFinite);
        }
        let components = Quaternion { w, x, y, z };
        let norm = libm::sqrt(components.norm_squared());
        if (norm - 1.0).abs() > INPUT_TOLERANCE {
            return Err(TransformError::NonUnitRotation { norm });
        }
        Ok(components.into_rotation()?)
    }

    /// The scalar (real) component.
    #[must_use]
    pub fn w(self) -> f64 {
        self.0.w
    }

    /// The `x` component of the vector part.
    #[must_use]
    pub fn x(self) -> f64 {
        self.0.x
    }

    /// The `y` component of the vector part.
    #[must_use]
    pub fn y(self) -> f64 {
        self.0.y
    }

    /// The `z` component of the vector part.
    #[must_use]
    pub fn z(self) -> f64 {
        self.0.z
    }

    /// Rotates a vector: `q * (0, v) * conjugate(q)`.
    ///
    /// This is raw arithmetic. It knows no frame, so it cannot tell a
    /// vector that belongs to this rotation's source frame from any other,
    /// and for components near the limits of `f64` the result can be
    /// infinite or NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Rotation, Vector3};
    ///
    /// // A half turn about x.
    /// let half_turn = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();
    /// assert_eq!(
    ///     half_turn.rotate(Vector3::new(1.0, 2.0, 3.0)),
    ///     Vector3::new(1.0, -2.0, -3.0)
    /// );
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn rotate(
        self,
        v: Vector3,
    ) -> Vector3 {
        let vector = Quaternion {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };
        let rotated = self.0.product(vector).product(self.0.conjugate());
        Vector3::new(rotated.x, rotated.y, rotated.z)
    }

    /// The squared norm, as the band measures it.
    #[inline]
    pub(crate) fn norm_squared(self) -> f64 {
        self.0.norm_squared()
    }

    /// The opposite rotation. Negation is exact and leaves every square
    /// unchanged, so the conjugate has the squared norm of the original and
    /// needs no correction.
    #[inline]
    pub(crate) fn conjugate(self) -> Self {
        Self(self.0.conjugate())
    }

    /// The rotation that applies `other` first, then `self`.
    ///
    /// # Errors
    ///
    /// The `NumericError` of a product that could not be brought back to
    /// unit length. Two rotations inside the band cannot overflow, and
    /// their product's norm stays within rounding of 1, so no input is
    /// known to reach it; the check is what guarantees the invariant.
    #[inline]
    pub(crate) fn product(
        self,
        other: Self,
    ) -> Result<Self, NumericError> {
        self.0.product(other.0).into_rotation()
    }

    /// Spherical linear interpolation from `self` to `other`.
    ///
    /// `t` is clamped to `[0.0, 1.0]`: there is no extrapolation. The
    /// shorter arc is taken. The trigonometry runs through `libm`.
    ///
    /// # Errors
    ///
    /// The `NumericError` of a blend that could not be brought back to
    /// unit length, or of a NaN factor.
    #[inline]
    pub(crate) fn slerp(
        self,
        other: Self,
        t: f64,
    ) -> Result<Self, NumericError> {
        let t = t.clamp(0.0, 1.0);

        let from = self.0;
        let mut to = other.0;
        let mut dot = from.w * to.w + from.x * to.x + from.y * to.y + from.z * to.z;

        if dot < 0.0 {
            to = to.scale(-1.0);
            dot = -dot;
        }

        let dot = dot.clamp(-1.0, 1.0);

        // Nearly parallel: the arc is too short for a stable division by
        // its sine, and a linear blend is exact to rounding. It is brought
        // to unit length by the one rule every rotation goes through, so a
        // blend already inside the band keeps its bits.
        if dot > 1.0 - f64::EPSILON {
            return Quaternion::weighted_sum(from, 1.0 - t, to, t).into_rotation();
        }

        let theta = libm::acos(dot);

        let sin_theta = libm::sin(theta);
        let scale_from = libm::sin((1.0 - t) * theta) / sin_theta;
        let scale_to = libm::sin(t * theta) / sin_theta;

        Quaternion::weighted_sum(from, scale_from, to, scale_to).into_rotation()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Rotation {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut rotation = serializer.serialize_struct("Rotation", 4)?;
        rotation.serialize_field("w", &self.0.w)?;
        rotation.serialize_field("x", &self.0.x)?;
        rotation.serialize_field("y", &self.0.y)?;
        rotation.serialize_field("z", &self.0.z)?;
        rotation.end()
    }
}

/// The record serde reads a [`Rotation`] from: the four components, in the
/// order they are written, before [`Rotation::from_wxyz`] has checked them.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(rename = "Rotation", deny_unknown_fields)]
struct RotationRecord {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Rotation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let RotationRecord { w, x, y, z } = RotationRecord::deserialize(deserializer)?;
        Self::from_wxyz(w, x, y, z).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
