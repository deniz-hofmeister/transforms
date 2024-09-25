use core::ops::{Add, Div, Mul, Sub};
mod error;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
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

impl Vector3 {
    #[inline]
    pub fn dot(
        self,
        other: Self,
    ) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

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

#[cfg(test)]
mod tests;
