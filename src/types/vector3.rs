use std::ops::{Add, Div, Mul, Sub};

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
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = Vector3 {
            x: 5.0,
            y: 7.0,
            z: 9.0,
        };
        assert_eq!(
            v1 + v2,
            expected,
            "Addition of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1 + v2
        );
    }

    #[test]
    fn test_sub() {
        let v1 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let v2 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let expected = Vector3 {
            x: 3.0,
            y: 3.0,
            z: 3.0,
        };
        assert_eq!(
            v1 - v2,
            expected,
            "Subtraction of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1 - v2
        );
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let scalar = 2.0;
        let expected = Vector3 {
            x: 2.0,
            y: 4.0,
            z: 6.0,
        };
        assert_eq!(
            v * scalar,
            expected,
            "Multiplication of {:?} by scalar {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v,
            scalar,
            expected,
            v * scalar
        );
    }

    #[test]
    fn test_div_scalar() {
        let v = Vector3 {
            x: 2.0,
            y: 4.0,
            z: 6.0,
        };
        let scalar = 2.0;
        let expected = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        assert_eq!(
            v / scalar,
            expected,
            "Division of {:?} by scalar {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v,
            scalar,
            expected,
            v / scalar
        );
    }

    #[test]
    fn test_dot_product() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = 32.0;
        assert_eq!(
            v1.dot(v2),
            expected,
            "Dot product of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1.dot(v2)
        );
    }

    #[test]
    fn test_cross_product() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = Vector3 {
            x: -3.0,
            y: 6.0,
            z: -3.0,
        };
        assert_eq!(
            v1.cross(v2),
            expected,
            "Cross product of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1.cross(v2)
        );
    }
}
