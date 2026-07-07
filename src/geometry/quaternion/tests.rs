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
        let _ = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    }

    #[test]
    fn identity_quaternion() {
        let identity = Quaternion::identity();
        assert_relative_eq!(identity.w, 1.0, epsilon = f64::EPSILON);
        assert_relative_eq!(identity.x, 0.0, epsilon = f64::EPSILON);
        assert_relative_eq!(identity.y, 0.0, epsilon = f64::EPSILON);
        assert_relative_eq!(identity.z, 0.0, epsilon = f64::EPSILON);

        let v = Vector3::new(1.0, 2.0, 3.0);
        let rotated = identity.rotate_vector(v);
        assert_relative_eq!(rotated.x, v.x, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.y, v.y, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.z, v.z, epsilon = f64::EPSILON);
    }

    #[test]
    fn conjugate() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let expected = Quaternion::new(1.0, -2.0, -3.0, -4.0);
        assert_eq!(q.conjugate(), expected);
    }

    #[test]
    fn normalize() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let result = q.normalize();
        assert!(
            result.is_ok(),
            "Normalization of {q:?} failed with error {result:?}"
        );
        let normalized = result.unwrap();
        let norm = normalized.norm();
        assert!(
            (norm - 1.0).abs() < f64::EPSILON,
            "Normalized quaternion {normalized:?} does not have norm 1. Got: {norm}"
        );
    }

    #[test]
    fn normalize_zero_length() {
        let q = Quaternion::new(0.0, 0.0, 0.0, 0.0);
        let result = q.normalize();
        assert!(
            matches!(result, Err(QuaternionError::ZeroLengthNormalization)),
            "Expected ZeroLengthNormalization error for {q:?}. Got: {result:?}"
        );
    }

    #[test]
    fn norm() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let expected: f64 = (1.0_f64 + 4.0 + 9.0 + 16.0).sqrt();

        assert_relative_eq!(q.norm(), expected, epsilon = f64::EPSILON);
    }

    #[test]
    fn norm_squared() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let expected = 1.0_f64 + 4.0 + 9.0 + 16.0;
        assert_relative_eq!(q.norm_squared(), expected, epsilon = f64::EPSILON);
    }

    #[test]
    fn scale() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let factor = 2.0;
        let expected = Quaternion::new(2.0, 4.0, 6.0, 8.0);
        assert_eq!(q.scale(factor), expected);
    }

    #[test]
    fn rotate_vector() {
        let q = Quaternion::new(
            (f64::consts::PI / 4.0).cos(),
            0.0,
            0.0,
            (f64::consts::PI / 4.0).sin(),
        );
        let v = Vector3::new(1.0, 0.0, 0.0);
        let rotated = q.rotate_vector(v);
        let expected = Vector3::new(0.0, 1.0, 0.0);

        assert_relative_eq!(rotated.x, expected.x, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.y, expected.y, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.z, expected.z, epsilon = f64::EPSILON);
    }

    #[test]
    fn rotate_vector_multiple_axes() {
        let q_z = Quaternion::new(
            (f64::consts::PI / 4.0).cos(),
            0.0,
            0.0,
            (f64::consts::PI / 4.0).sin(),
        );

        let q_x = Quaternion::new(
            (f64::consts::PI / 4.0).cos(),
            (f64::consts::PI / 4.0).sin(),
            0.0,
            0.0,
        );

        let q_combined = q_x * q_z;
        let v = Vector3::new(1.0, 0.0, 0.0);
        let rotated = q_combined.rotate_vector(v);
        let expected = Vector3::new(0.0, 0.0, 1.0);

        assert_relative_eq!(rotated.x, expected.x, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.y, expected.y, epsilon = f64::EPSILON);
        assert_relative_eq!(rotated.z, expected.z, epsilon = f64::EPSILON);
    }

    #[test]
    fn quaternion_multiplication_properties() {
        let q1 = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);

        let q1_times_q2 = q1 * q2;
        let q2_times_q1 = q2 * q1;

        assert_ne!(
            q1_times_q2, q2_times_q1,
            "Quaternion multiplication should not be commutative"
        );
    }

    #[test]
    fn add() {
        let q1 = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quaternion::new(5.0, 6.0, 7.0, 8.0);
        let expected = Quaternion::new(6.0, 8.0, 10.0, 12.0);
        assert_eq!(q1 + q2, expected);
    }

    #[test]
    fn sub() {
        let q1 = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quaternion::new(5.0, 6.0, 7.0, 8.0);
        let expected = Quaternion::new(-4.0, -4.0, -4.0, -4.0);
        assert_eq!(q1 - q2, expected);
    }

    #[test]
    fn mul() {
        let q1 = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quaternion::new(5.0, 6.0, 7.0, 8.0);
        let expected = Quaternion::new(-60.0, 12.0, 30.0, 24.0);
        assert_eq!(q1 * q2, expected);
    }

    #[test]
    fn div() {
        let q1 = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quaternion::new(5.0, 6.0, 7.0, 8.0);
        let result = q1 / q2;
        assert!(
            result.is_ok(),
            "Division of {q1:?} by {q2:?} failed with error {result:?}"
        );
    }

    #[test]
    fn div_by_zero() {
        let q1 = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quaternion::new(0.0, 0.0, 0.0, 0.0);
        let result = q1 / q2;
        assert!(
            matches!(result, Err(QuaternionError::DivisionByZero)),
            "Expected DivisionByZero error for {q1:?} / {q2:?}"
        );
    }

    #[test]
    fn slerp() {
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        let t = 0.5;
        let result = q1.slerp(q2, t);
        let expected = Quaternion::new((0.5_f64).sqrt(), (0.5_f64).sqrt(), 0.0, 0.0);

        assert_relative_eq!(result.w, expected.w, epsilon = f64::EPSILON);
        assert_relative_eq!(result.x, expected.x, epsilon = f64::EPSILON);
        assert_relative_eq!(result.y, expected.y, epsilon = f64::EPSILON);
        assert_relative_eq!(result.z, expected.z, epsilon = f64::EPSILON);
    }

    #[test]
    fn slerp_edge_cases() {
        let q1 = Quaternion::new(0.5, 0.5, 0.5, 0.5).normalize().unwrap();
        let q2 = Quaternion::identity();

        let result = q1.slerp(q2, 0.0);
        assert_relative_eq!(result.w, q1.w, epsilon = f64::EPSILON);
        assert_relative_eq!(result.x, q1.x, epsilon = f64::EPSILON);
        assert_relative_eq!(result.y, q1.y, epsilon = f64::EPSILON);
        assert_relative_eq!(result.z, q1.z, epsilon = f64::EPSILON);

        let result = q1.slerp(q2, 1.0);
        assert_relative_eq!(result.w, q2.w, epsilon = f64::EPSILON);
        assert_relative_eq!(result.x, q2.x, epsilon = f64::EPSILON);
        assert_relative_eq!(result.y, q2.y, epsilon = f64::EPSILON);
        assert_relative_eq!(result.z, q2.z, epsilon = f64::EPSILON);

        let q1 = Quaternion::new(0.9999, 0.0001, 0.0, 0.0)
            .normalize()
            .unwrap();
        let q2 = Quaternion::new(0.9998, 0.0002, 0.0, 0.0)
            .normalize()
            .unwrap();

        let result = q1.slerp(q2, 0.5);
        assert!(
            (result.norm() - 1.0).abs() < f64::EPSILON,
            "Slerp result should be normalized"
        );
    }

    #[test]
    fn slerp_uses_shortest_path_for_antipodal_quaternions() {
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(-1.0, 0.0, 0.0, 0.0);

        let result = q1.slerp(q2, 0.5);

        assert_relative_eq!(result.w, 1.0, epsilon = f64::EPSILON);
        assert_relative_eq!(result.x, 0.0, epsilon = f64::EPSILON);
        assert_relative_eq!(result.y, 0.0, epsilon = f64::EPSILON);
        assert_relative_eq!(result.z, 0.0, epsilon = f64::EPSILON);
        assert_relative_eq!(result.norm(), 1.0, epsilon = f64::EPSILON);
    }
}
