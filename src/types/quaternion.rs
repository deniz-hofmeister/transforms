use std::ops::{Add, Div, Mul, Sub};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Error, Debug)]
pub enum QuaternionError {
    #[error("Division by zero quaternion")]
    DivisionByZero,
    #[error("Cannot normalize a zero-length quaternion")]
    ZeroLengthNormalization,
}

impl Quaternion {
    const EPSILON: f64 = 1e-9;
    const PI: f64 = 3.14159265358979311599796346854;

    pub fn conjugate(self) -> Quaternion {
        Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    pub fn normalize(self) -> Result<Quaternion, QuaternionError> {
        let norm = self.norm();
        if norm < Self::EPSILON {
            return Err(QuaternionError::ZeroLengthNormalization);
        }
        Ok(self.scale(1.0 / norm))
    }

    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn norm_squared(self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

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

    pub fn rotate_vector(
        self,
        v: (f64, f64, f64),
    ) -> (f64, f64, f64) {
        let q_vec = Quaternion {
            w: 0.0,
            x: v.0,
            y: v.1,
            z: v.2,
        };
        let q_res = self.mul(q_vec).mul(self.conjugate());
        (q_res.x, q_res.y, q_res.z)
    }
}

impl Add for Quaternion {
    type Output = Quaternion;

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

    fn div(
        self,
        other: Quaternion,
    ) -> Result<Quaternion, QuaternionError> {
        let norm_sq = other.norm_squared();
        if norm_sq < Quaternion::EPSILON {
            return Err(QuaternionError::DivisionByZero);
        }
        Ok(self.mul(other.conjugate()).scale(1.0 / norm_sq))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use std::format;
    #[test]
    fn quaternion_creation() {
        let _q = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }
    #[test]
    fn test_conjugate() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let expected = Quaternion {
            w: 1.0,
            x: -2.0,
            y: -3.0,
            z: -4.0,
        };
        assert_eq!(
            q.conjugate(),
            expected,
            "Conjugate of {:?} was incorrect. Expected: {:?}, Got: {:?}",
            q,
            expected,
            q.conjugate()
        );
    }

    #[test]
    fn test_normalize() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let result = q.normalize();
        assert!(
            result.is_ok(),
            "Normalization of {:?} failed with error {:?}",
            q,
            result
        );
        let normalized = result.unwrap();
        assert!(
            (normalized.norm() - 1.0).abs() < Quaternion::EPSILON,
            "Normalized quaternion {:?} does not have norm 1. Got: {}",
            normalized,
            normalized.norm()
        );
    }

    #[test]
    fn test_normalize_zero_length() {
        let q = Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let result = q.normalize();
        assert!(
            matches!(result, Err(QuaternionError::ZeroLengthNormalization)),
            "Expected ZeroLengthNormalization error for {:?}. Got: {:?}",
            q,
            result
        );
    }

    #[test]
    fn test_norm() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let expected = (1.0_f64 + 4.0 + 9.0 + 16.0).sqrt();
        assert_eq!(
            q.norm(),
            expected,
            "Norm of {:?} was incorrect. Expected: {}, Got: {}",
            q,
            expected,
            q.norm()
        );
    }

    #[test]
    fn test_norm_squared() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let expected = 1.0_f64 + 4.0 + 9.0 + 16.0;
        assert_eq!(
            q.norm_squared(),
            expected,
            "Norm squared of {:?} was incorrect. Expected: {}, Got: {}",
            q,
            expected,
            q.norm_squared()
        );
    }

    #[test]
    fn test_scale() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let factor = 2.0;
        let expected = Quaternion {
            w: 2.0,
            x: 4.0,
            y: 6.0,
            z: 8.0,
        };
        assert_eq!(
            q.scale(factor),
            expected,
            "Scaling {:?} by {} was incorrect. Expected: {:?}, Got: {:?}",
            q,
            factor,
            expected,
            q.scale(factor)
        );
    }

    #[test]
    fn test_rotate_vector() {
        let q = Quaternion {
            w: (Quaternion::PI / 4.0).cos(),
            x: 0.0,
            y: 0.0,
            z: (Quaternion::PI / 4.0).sin(),
        };
        let v = (1.0, 0.0, 0.0);
        let rotated = q.rotate_vector(v);
        let expected = (0.0, 1.0, 0.0);

        assert_relative_eq!(rotated.0, expected.0, epsilon = Quaternion::EPSILON);
        assert_relative_eq!(rotated.1, expected.1, epsilon = Quaternion::EPSILON);
        assert_relative_eq!(rotated.2, expected.2, epsilon = Quaternion::EPSILON);
    }

    #[test]
    fn test_add() {
        let q1 = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let q2 = Quaternion {
            w: 5.0,
            x: 6.0,
            y: 7.0,
            z: 8.0,
        };
        let expected = Quaternion {
            w: 6.0,
            x: 8.0,
            y: 10.0,
            z: 12.0,
        };
        assert_eq!(
            q1 + q2,
            expected,
            "Addition of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            q1,
            q2,
            expected,
            q1 + q2
        );
    }

    #[test]
    fn test_sub() {
        let q1 = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let q2 = Quaternion {
            w: 5.0,
            x: 6.0,
            y: 7.0,
            z: 8.0,
        };
        let expected = Quaternion {
            w: -4.0,
            x: -4.0,
            y: -4.0,
            z: -4.0,
        };
        assert_eq!(
            q1 - q2,
            expected,
            "Subtraction of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            q1,
            q2,
            expected,
            q1 - q2
        );
    }

    #[test]
    fn test_mul() {
        let q1 = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let q2 = Quaternion {
            w: 5.0,
            x: 6.0,
            y: 7.0,
            z: 8.0,
        };
        let expected = Quaternion {
            w: -60.0,
            x: 12.0,
            y: 30.0,
            z: 24.0,
        };
        assert_eq!(
            q1 * q2,
            expected,
            "Multiplication of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            q1,
            q2,
            expected,
            q1 * q2
        );
    }

    #[test]
    fn test_div() {
        let q1 = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let q2 = Quaternion {
            w: 5.0,
            x: 6.0,
            y: 7.0,
            z: 8.0,
        };
        let result = q1 / q2;
        assert!(
            result.is_ok(),
            "Division of {:?} by {:?} failed with error {:?}",
            q1,
            q2,
            result.unwrap_err()
        );
    }

    #[test]
    fn test_div_by_zero() {
        let q1 = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let q2 = Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let result = q1 / q2;
        assert!(
            matches!(result, Err(QuaternionError::DivisionByZero)),
            "Expected DivisionByZero error for {:?} / {:?}",
            q1,
            q2
        );
    }
}
