//! A 3D vector type with basic arithmetic, dot, and cross products.

use core::ops::{Add, Div, Mul, Sub};

use approx::{AbsDiffEq, RelativeEq};

/// A 3D vector with `x`, `y`, and `z` components.
///
/// The `Vector3` struct represents a point or direction in 3D space.
///
/// # Examples
///
/// ```
/// use transforms::geometry::Vector3;
///
/// let vector = Vector3::new(1.0, 2.0, 3.0);
///
/// assert_eq!(vector.x, 1.0);
/// assert_eq!(vector.y, 2.0);
/// assert_eq!(vector.z, 3.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vector3 {
    /// The x component.
    pub x: f64,
    /// The y component.
    pub y: f64,
    /// The z component.
    pub z: f64,
}

impl Vector3 {
    /// Creates a new `Vector3` with the given x, y, z coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Vector3;
    /// let v = Vector3::new(1.0, 2.0, 3.0);
    /// assert_eq!(v.x, 1.0);
    /// assert_eq!(v.y, 2.0);
    /// assert_eq!(v.z, 3.0);
    /// ```
    #[must_use]
    pub const fn new(
        x: f64,
        y: f64,
        z: f64,
    ) -> Self {
        Self { x, y, z }
    }

    /// Returns the zero vector `(0.0, 0.0, 0.0)`.
    #[must_use]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Returns the unit vector along the x-axis `(1.0, 0.0, 0.0)`.
    #[must_use]
    pub const fn unit_x() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// Returns the unit vector along the y-axis `(0.0, 1.0, 0.0)`.
    #[must_use]
    pub const fn unit_y() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    /// Returns the unit vector along the z-axis `(0.0, 0.0, 1.0)`.
    #[must_use]
    pub const fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// Computes the dot product of two vectors, the sum of the products of their components.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Vector3;
    ///
    /// let a = Vector3::new(1.0, 2.0, 3.0);
    /// let b = Vector3::new(4.0, 5.0, 6.0);
    /// assert_eq!(a.dot(b), 32.0);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn dot(
        self,
        other: Self,
    ) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Computes the cross product of two vectors, a vector perpendicular to both operands.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::geometry::Vector3;
    ///
    /// let a = Vector3::new(1.0, 0.0, 0.0);
    /// let b = Vector3::new(0.0, 1.0, 0.0);
    /// assert_eq!(a.cross(b), Vector3::new(0.0, 0.0, 1.0));
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn cross(
        self,
        other: Self,
    ) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl Add for Vector3 {
    type Output = Self;

    #[inline]
    fn add(
        self,
        other: Self,
    ) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vector3 {
    type Output = Self;

    #[inline]
    fn sub(
        self,
        other: Self,
    ) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f64> for Vector3 {
    type Output = Self;

    #[inline]
    fn mul(
        self,
        scalar: f64,
    ) -> Self::Output {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul<Vector3> for f64 {
    type Output = Vector3;

    #[inline]
    fn mul(
        self,
        rhs: Vector3,
    ) -> Self::Output {
        Vector3 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
        }
    }
}

impl Div<f64> for Vector3 {
    type Output = Self;

    #[inline]
    fn div(
        self,
        scalar: f64,
    ) -> Self::Output {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl AbsDiffEq for Vector3 {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::EPSILON
    }

    fn abs_diff_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
    ) -> bool {
        f64::abs_diff_eq(&self.x, &other.x, epsilon)
            && f64::abs_diff_eq(&self.y, &other.y, epsilon)
            && f64::abs_diff_eq(&self.z, &other.z, epsilon)
    }
}

impl RelativeEq for Vector3 {
    fn default_max_relative() -> Self::Epsilon {
        f64::EPSILON
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        f64::relative_eq(&self.x, &other.x, epsilon, max_relative)
            && f64::relative_eq(&self.y, &other.y, epsilon, max_relative)
            && f64::relative_eq(&self.z, &other.z, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests;
