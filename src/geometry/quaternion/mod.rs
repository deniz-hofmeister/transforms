use crate::geometry::Vector3;
use core::ops::{Add, Div, Mul, Sub};
pub mod error;
use approx::AbsDiffEq;
pub use error::QuaternionError;

/// A quaternion representing a rotation in 3D space.
#[derive(Debug, Clone, Copy, PartialOrd)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Optional: Implement Default trait which returns identity
impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl Quaternion {
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
    pub fn identity() -> Self {
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
    /// let q = Quaternion {
    ///     w: 1.0,
    ///     x: 2.0,
    ///     y: 3.0,
    ///     z: 4.0,
    /// };
    /// assert_eq!(
    ///     q.conjugate(),
    ///     Quaternion {
    ///         w: 1.0,
    ///         x: -2.0,
    ///         y: -3.0,
    ///         z: -4.0
    ///     }
    /// );
    /// ```
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
    /// Returns `QuaternionError::ZeroLengthNormalization` if the quaternion is zero-length.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{errors::QuaternionError, geometry::Quaternion};
    ///
    /// let q = Quaternion {
    ///     w: 1.0,
    ///     x: 2.0,
    ///     y: 3.0,
    ///     z: 4.0,
    /// };
    /// let normalized = q.normalize().unwrap();
    /// assert!((normalized.norm() - 1.0).abs() < f64::EPSILON);
    ///
    /// let zero_q = Quaternion {
    ///     w: 0.0,
    ///     x: 0.0,
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// assert!(matches!(
    ///     zero_q.normalize(),
    ///     Err(QuaternionError::ZeroLengthNormalization)
    /// ));
    /// ```
    #[inline]
    pub fn normalize(self) -> Result<Quaternion, QuaternionError> {
        let norm = self.norm();
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
    /// let q = Quaternion {
    ///     w: 1.0,
    ///     x: 1.0,
    ///     y: 1.0,
    ///     z: 1.0,
    /// };
    /// assert_eq!(q.norm(), 2.0);
    /// ```
    #[inline]
    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
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
    /// let q = Quaternion {
    ///     w: 1.0,
    ///     x: 2.0,
    ///     y: 2.0,
    ///     z: 2.0,
    /// };
    /// assert_eq!(q.norm_squared(), 13.0);
    /// ```
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
    /// let q = Quaternion { w: 1.0, x: 2.0, y: 3.0, z: 4.0 };
    /// assert_eq!(q.scale(2.0), Quaternion { w: 2.0, x: 4.0, y: 6.0, z: 8.0 });
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
    /// let q = Quaternion {
    ///     w: (core::f64::consts::PI / 4.0).cos(),
    ///     x: 0.0,
    ///     y: 0.0,
    ///     z: (core::f64::consts::PI / 4.0).sin(),
    /// };
    /// let v = Vector3 {
    ///     x: 1.0,
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// assert_relative_eq!(
    ///     q.rotate_vector(v),
    ///     Vector3 {
    ///         x: 0.0,
    ///         y: 1.0,
    ///         z: 0.0
    ///     }
    /// );
    /// ```
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
    /// let q1 = Quaternion {
    ///     w: 1.0,
    ///     x: 0.0,
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// let q2 = Quaternion {
    ///     w: 0.0,
    ///     x: 1.0,
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// let result = q1.slerp(q2, 0.5);
    /// let expected = Quaternion {
    ///     w: (0.5_f64).sqrt(),
    ///     x: (0.5_f64).sqrt(),
    ///     y: 0.0,
    ///     z: 0.0,
    /// };
    /// assert_relative_eq!(result.w, expected.w, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.x, expected.x, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.y, expected.y, epsilon = f64::EPSILON);
    /// assert_relative_eq!(result.z, expected.z, epsilon = f64::EPSILON);
    /// ```
    #[inline]
    pub fn slerp(
        self,
        other: Quaternion,
        t: f64,
    ) -> Quaternion {
        let dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        let dot = dot.clamp(-1.0, 1.0);
        let theta = dot.acos();

        if theta.abs() < f64::EPSILON {
            return self.scale(1.0 - t) + other.scale(t);
        }

        let sin_theta = theta.sin();
        let scale_self = ((1.0 - t) * theta).sin() / sin_theta;
        let scale_other = (t * theta).sin() / sin_theta;

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
