//! Quaternions for representing rotations in 3D space.

use crate::geometry::Vector3;
use approx::{AbsDiffEq, RelativeEq};
use core::ops::{Add, Div, Mul, Sub};
pub use error::QuaternionError;

mod error;

// The `sqrt`, `sin`, and `acos` below are `libm`'s in every feature mode,
// never `std`'s: a desktop replay and the MCU it replays must agree bit for
// bit, and `std`'s implementations are the platform's, which do not.

/// A quaternion representing a rotation in 3D space.
///
/// With the optional `serde` feature, this type implements `Serialize` and
/// `Deserialize` (the docs.rs listing cannot banner derive-generated impls).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quaternion {
    /// The scalar (real) part of the quaternion.
    pub w: f64,
    /// The `x` component of the vector part.
    pub x: f64,
    /// The `y` component of the vector part.
    pub y: f64,
    /// The `z` component of the vector part.
    pub z: f64,
}

impl Default for Quaternion {
    /// Returns the identity quaternion.
    fn default() -> Self {
        Self::identity()
    }
}

impl Quaternion {
    /// Creates a quaternion from its `w`, `x`, `y`, and `z` components.
    ///
    /// The scalar part `w` comes first — the name spells the order out,
    /// because the other common convention puts it last and a silently
    /// swapped `w` is a valid quaternion describing a different rotation.
    /// No normalization is performed; rotations are expected to be unit
    /// quaternions, so call [`Quaternion::normalize`] if the components do not
    /// already form one.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::from_wxyz(1.0, 0.0, 0.0, 0.0);
    /// assert_eq!(q, Quaternion::identity());
    /// ```
    #[must_use]
    pub const fn from_wxyz(
        w: f64,
        x: f64,
        y: f64,
        z: f64,
    ) -> Self {
        Self { w, x, y, z }
    }

    /// Creates an identity quaternion representing no rotation.
    ///
    /// Returns a quaternion with w=1 and x=y=z=0, which represents the identity rotation
    /// (i.e., no rotation at all).
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::identity();
    /// assert_eq!(q.w, 1.0);
    /// assert_eq!(q.x, 0.0);
    /// assert_eq!(q.y, 0.0);
    /// assert_eq!(q.z, 0.0);
    /// ```
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Returns the conjugate of the quaternion.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::from_wxyz(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(q.conjugate(), Quaternion::from_wxyz(1.0, -2.0, -3.0, -4.0));
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn conjugate(self) -> Quaternion {
        Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Normalizes the quaternion to unit length.
    ///
    /// The intermediate norm — the square root of the sum of squares — sets
    /// both limits. A component beyond roughly `1.3e154`, the square root of
    /// `f64::MAX`, squares to infinity (reported as `NonFinite`), and a norm
    /// below `f64::EPSILON` (about `2.2e-16`) counts as zero (reported as
    /// `ZeroLengthNormalization`). That threshold is exclusive: a norm of
    /// exactly `f64::EPSILON` still normalizes.
    ///
    /// # Errors
    ///
    /// Returns `QuaternionError::ZeroLengthNormalization` if the norm is
    /// below `f64::EPSILON`, and `QuaternionError::NonFinite` if any
    /// component is NaN or the sum of squares overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{errors::QuaternionError, geometry::Quaternion};
    ///
    /// let q = Quaternion::from_wxyz(1.0, 2.0, 3.0, 4.0);
    /// let normalized = q.normalize().unwrap();
    /// assert!((normalized.norm() - 1.0).abs() < f64::EPSILON);
    ///
    /// // The zero threshold is `f64::EPSILON` on the norm, not a
    /// // rotation-scale epsilon: this one is far below rotation scale and
    /// // still normalizes.
    /// let at_threshold = Quaternion::from_wxyz(f64::EPSILON, 0.0, 0.0, 0.0);
    /// assert_eq!(at_threshold.normalize().unwrap(), Quaternion::identity());
    ///
    /// let below_threshold = Quaternion::from_wxyz(f64::EPSILON / 2.0, 0.0, 0.0, 0.0);
    /// assert!(matches!(
    ///     below_threshold.normalize(),
    ///     Err(QuaternionError::ZeroLengthNormalization)
    /// ));
    ///
    /// let zero_q = Quaternion::from_wxyz(0.0, 0.0, 0.0, 0.0);
    /// assert!(matches!(
    ///     zero_q.normalize(),
    ///     Err(QuaternionError::ZeroLengthNormalization)
    /// ));
    /// ```
    #[inline]
    pub fn normalize(self) -> Result<Quaternion, QuaternionError> {
        let norm = self.norm();
        if !norm.is_finite() {
            return Err(QuaternionError::NonFinite);
        }
        if norm < f64::EPSILON {
            return Err(QuaternionError::ZeroLengthNormalization);
        }
        Ok(self.scale(1.0 / norm))
    }

    /// Computes the norm (magnitude) of the quaternion.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::from_wxyz(1.0, 1.0, 1.0, 1.0);
    /// assert_eq!(q.norm(), 2.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn norm(self) -> f64 {
        libm::sqrt(self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z)
    }

    /// Computes the squared norm of the quaternion.
    ///
    /// This is the sum of the squares of the components.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::from_wxyz(1.0, 2.0, 2.0, 2.0);
    /// assert_eq!(q.norm_squared(), 13.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn norm_squared(self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Scales the quaternion by a given factor.
    ///
    /// Multiplies each component of the quaternion by the factor.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::from_wxyz(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(q.scale(2.0), Quaternion::from_wxyz(2.0, 4.0, 6.0, 8.0));
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn scale(
        self,
        factor: f64,
    ) -> Quaternion {
        Quaternion {
            w: self.w * factor,
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    /// Rotates a vector by the quaternion.
    ///
    /// The vector is treated as a pure quaternion with a real part of zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::{Quaternion, Vector3};
    /// # use approx::assert_relative_eq;
    ///
    /// let q = Quaternion::from_wxyz(
    ///     (core::f64::consts::PI / 4.0).cos(),
    ///     0.0,
    ///     0.0,
    ///     (core::f64::consts::PI / 4.0).sin(),
    /// );
    /// let v = Vector3::new(1.0, 0.0, 0.0);
    /// assert_relative_eq!(q.rotate_vector(v), Vector3::new(0.0, 1.0, 0.0));
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn rotate_vector(
        self,
        v: Vector3,
    ) -> Vector3 {
        let q_vec = Quaternion {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };
        let q_res = self.mul(q_vec).mul(self.conjugate());
        Vector3 {
            x: q_res.x,
            y: q_res.y,
            z: q_res.z,
        }
    }

    /// Performs spherical linear interpolation (slerp) between two quaternions.
    ///
    /// Interpolates between `self` and `other` by the factor `t`, which is
    /// clamped to `[0.0, 1.0]` — there is no extrapolation, matching the
    /// crate-wide policy. Infinite factors saturate to the corresponding
    /// endpoint; a NaN factor yields a NaN result.
    ///
    /// The trigonometry runs through `libm` whether or not `std` is enabled,
    /// so the same operands yield bit-identical results in both feature
    /// modes.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    /// # use approx::assert_relative_eq;
    ///
    /// let q1 = Quaternion::identity();
    /// let q2 = Quaternion::from_wxyz(0.0, 1.0, 0.0, 0.0);
    /// let result = q1.slerp(q2, 0.5);
    /// let expected = Quaternion::from_wxyz((0.5_f64).sqrt(), (0.5_f64).sqrt(), 0.0, 0.0);
    /// assert_relative_eq!(result.w, expected.w, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.x, expected.x, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.y, expected.y, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.z, expected.z, epsilon = f64::EPSILON);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn slerp(
        self,
        other: Quaternion,
        t: f64,
    ) -> Quaternion {
        let t = t.clamp(0.0, 1.0);

        let mut other = other;
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        if dot < 0.0 {
            other = other.scale(-1.0);
            dot = -dot;
        }

        let dot = dot.clamp(-1.0, 1.0);

        if dot > 1.0 - f64::EPSILON {
            let blended = self.scale(1.0 - t) + other.scale(t);
            let norm = blended.norm();
            return if norm < f64::EPSILON {
                blended
            } else {
                blended.scale(1.0 / norm)
            };
        }

        let theta = libm::acos(dot);

        let sin_theta = libm::sin(theta);
        let scale_self = libm::sin((1.0 - t) * theta) / sin_theta;
        let scale_other = libm::sin(t * theta) / sin_theta;

        self.scale(scale_self) + other.scale(scale_other)
    }
}

impl Add for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn add(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w + other.w,
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn sub(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w - other.w,
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn mul(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }
}

impl Div for Quaternion {
    type Output = Result<Quaternion, QuaternionError>;

    /// Divides by `other` via multiplication with its inverse.
    ///
    /// Returns `QuaternionError::DivisionByZero` if `other`'s squared norm
    /// is below `f64::EPSILON`: divisors with a norm under roughly `1.5e-8`
    /// are rejected as numerically zero — a deliberately stricter threshold
    /// than [`Quaternion::normalize`]'s, since dividing by a near-zero
    /// quaternion amplifies error quadratically.
    #[inline]
    fn div(
        self,
        other: Quaternion,
    ) -> Result<Quaternion, QuaternionError> {
        let norm_sq = other.norm_squared();
        if norm_sq < f64::EPSILON {
            return Err(QuaternionError::DivisionByZero);
        }
        Ok(self.mul(other.conjugate()).scale(1.0 / norm_sq))
    }
}

impl AbsDiffEq for Quaternion {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::EPSILON
    }

    fn abs_diff_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
    ) -> bool {
        f64::abs_diff_eq(&self.w, &other.w, epsilon)
            && f64::abs_diff_eq(&self.x, &other.x, epsilon)
            && f64::abs_diff_eq(&self.y, &other.y, epsilon)
            && f64::abs_diff_eq(&self.z, &other.z, epsilon)
    }
}

impl RelativeEq for Quaternion {
    fn default_max_relative() -> Self::Epsilon {
        f64::EPSILON
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        f64::relative_eq(&self.w, &other.w, epsilon, max_relative)
            && f64::relative_eq(&self.x, &other.x, epsilon, max_relative)
            && f64::relative_eq(&self.y, &other.y, epsilon, max_relative)
            && f64::relative_eq(&self.z, &other.z, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests;
