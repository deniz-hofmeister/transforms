#[cfg(test)]
mod quaternion_tests {
    use crate::{
        errors::QuaternionError,
        geometry::{Quaternion, Vector3},
    };
    use approx::assert_relative_eq;
    use core::f64;

    #[test]
    fn quaternion_creation() {
        let _ = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }

    #[test]
    fn conjugate() {
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
    fn normalize() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let result = q.normalize();
        assert!(
            result.is_ok(),
            "Normalization of {q:?} failed with error {result:?}"
        );
        let normalized = result.unwrap();
        assert!(
            (normalized.norm() - 1.0).abs() < f64::EPSILON,
            "Normalized quaternion {:?} does not have norm 1. Got: {}",
            normalized,
            normalized.norm()
        );
    }

    #[test]
    fn normalize_zero_length() {
        let q = Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let result = q.normalize();
        assert!(
            matches!(result, Err(QuaternionError::ZeroLengthNormalization)),
            "Expected ZeroLengthNormalization error for {q:?}. Got: {result:?}"
        );
    }

    #[test]
    fn norm() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let expected: f64 = (1.0_f64 + 4.0 + 9.0 + 16.0).sqrt();

        assert_relative_eq!(q.norm(), expected, epsilon = f64::EPSILON);
    }

    #[test]
    fn norm_squared() {
        let q = Quaternion {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let expected = 1.0_f64 + 4.0 + 9.0 + 16.0;
        assert_relative_eq!(q.norm_squared(), expected, epsilon = f64::EPSILON);
    }

    #[test]
    fn scale() {
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
    fn rotate_vector() {
        let q = Quaternion {
            w: (f64::consts::PI / 4.0).cos(),
            x: 0.0,
            y: 0.0,
            z: (f64::consts::PI / 4.0).sin(),
        };
        let v = Vector3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let rotated = q.rotate_vector(v);
        let expected = Vector3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };

        assert_relative_eq!(rotated.z, expected.x, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.y, expected.y, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.z, expected.z, epsilon = f64::EPSILON);
    }

    #[test]
    fn add() {
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
    fn sub() {
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
    fn mul() {
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
    fn div() {
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
    fn div_by_zero() {
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
            "Expected DivisionByZero error for {q1:?} / {q2:?}"
        );
    }

    #[test]
    fn slerp() {
        let q1 = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let q2 = Quaternion {
            w: 0.0,
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let t = 0.5;
        let result = q1.slerp(q2, t);
        let expected = Quaternion {
            w: (0.5_f64).sqrt(),
            x: (0.5_f64).sqrt(),
            y: 0.0,
            z: 0.0,
        };

        assert_relative_eq!(result.w, expected.w, epsilon = f64::EPSILON);
        assert_relative_eq!(result.x, expected.x, epsilon = f64::EPSILON);
        assert_relative_eq!(result.y, expected.y, epsilon = f64::EPSILON);
        assert_relative_eq!(result.z, expected.z, epsilon = f64::EPSILON);
    }
}
