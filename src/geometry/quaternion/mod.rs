//! Quaternions for representing rotations in 3D space.

use crate::geometry::Vector3;
use approx::AbsDiffEq;
use core::ops::{Add, Div, Mul, Sub};
pub use error::QuaternionError;

mod error;

/// Float math that works with and without `std`.
///
/// `f64::sqrt`, `sin`, and `acos` are `std` methods rather than `core`
/// intrinsics; without `std` the equivalent `libm` implementations are used.
mod math {
    #[inline]
    pub fn sqrt(x: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            x.sqrt()
        }
        #[cfg(not(feature = "std"))]
        {
            libm::sqrt(x)
        }
    }

    #[inline]
    pub fn sin(x: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            x.sin()
        }
        #[cfg(not(feature = "std"))]
        {
            libm::sin(x)
        }
    }

    #[inline]
    pub fn acos(x: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            x.acos()
        }
        #[cfg(not(feature = "std"))]
        {
            libm::acos(x)
        }
    }
}

/// A quaternion representing a rotation in 3D space.
#[derive(Debug, Clone, Copy, PartialOrd)]
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
    /// Creates a new quaternion from its `w`, `x`, `y`, and `z` components.
    ///
    /// The scalar part `w` comes first, matching the field order of this type.
    /// No normalization is performed; rotations are expected to be unit
    /// quaternions, so call [`Quaternion::normalize`] if the components do not
    /// already form one.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    ///
    /// let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    /// assert_eq!(q, Quaternion::identity());
    /// ```
    #[must_use]
    pub const fn new(
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
    /// let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(q.conjugate(), Quaternion::new(1.0, -2.0, -3.0, -4.0));
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
    /// # Errors
    ///
    /// Returns `QuaternionError::ZeroLengthNormalization` if the quaternion is
    /// zero-length, and `QuaternionError::NonFinite` if any component is NaN
    /// or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{errors::QuaternionError, geometry::Quaternion};
    ///
    /// let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    /// let normalized = q.normalize().unwrap();
    /// assert!((normalized.norm() - 1.0).abs() < f64::EPSILON);
    ///
    /// let zero_q = Quaternion::new(0.0, 0.0, 0.0, 0.0);
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
    /// let q = Quaternion::new(1.0, 1.0, 1.0, 1.0);
    /// assert_eq!(q.norm(), 2.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn norm(self) -> f64 {
        math::sqrt(self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z)
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
    /// let q = Quaternion::new(1.0, 2.0, 2.0, 2.0);
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
    /// let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(q.scale(2.0), Quaternion::new(2.0, 4.0, 6.0, 8.0));
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
    /// let q = Quaternion::new(
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
    /// Interpolates between `self` and `other` by the factor `t`.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Quaternion;
    /// # use approx::assert_relative_eq;
    ///
    /// let q1 = Quaternion::identity();
    /// let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);
    /// let result = q1.slerp(q2, 0.5);
    /// let expected = Quaternion::new((0.5_f64).sqrt(), (0.5_f64).sqrt(), 0.0, 0.0);
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

        let theta = math::acos(dot);

        let sin_theta = math::sin(theta);
        let scale_self = math::sin((1.0 - t) * theta) / sin_theta;
        let scale_other = math::sin(t * theta) / sin_theta;

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

impl PartialEq for Quaternion {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.abs_diff_eq(other, f64::EPSILON)
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

#[cfg(test)]
mod tests;
