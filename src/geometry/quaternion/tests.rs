#[cfg(test)]
mod quaternion_tests {
    use crate::{
        errors::QuaternionError,
        geometry::{Quaternion, Vector3},
    };
    use approx::{assert_abs_diff_eq, assert_relative_eq};

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
            (core::f64::consts::PI / 4.0).cos(),
            0.0,
            0.0,
            (core::f64::consts::PI / 4.0).sin(),
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
            (core::f64::consts::PI / 4.0).cos(),
            0.0,
            0.0,
            (core::f64::consts::PI / 4.0).sin(),
        );

        let q_x = Quaternion::new(
            (core::f64::consts::PI / 4.0).cos(),
            (core::f64::consts::PI / 4.0).sin(),
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

    #[test]
    fn normalize_rejects_non_finite_quaternions() {
        let nan = Quaternion::new(f64::NAN, 0.0, 0.0, 0.0);
        assert!(matches!(nan.normalize(), Err(QuaternionError::NonFinite)));

        let inf = Quaternion::new(f64::INFINITY, 0.0, 0.0, 0.0);
        assert!(matches!(inf.normalize(), Err(QuaternionError::NonFinite)));
    }

    #[test]
    fn slerp_clamps_t_to_the_unit_interval() {
        let theta = core::f64::consts::PI / 2.0;
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin());

        // No extrapolation: out-of-range factors saturate at the endpoints.
        assert_abs_diff_eq!(q1.slerp(q2, 2.0), q1.slerp(q2, 1.0));
        assert_abs_diff_eq!(q1.slerp(q2, -0.5), q1.slerp(q2, 0.0));
    }

    /// A rotation of `angle` radians about the z-axis.
    fn rotation_about_z(angle: f64) -> Quaternion {
        Quaternion::new((angle / 2.0).cos(), 0.0, 0.0, (angle / 2.0).sin())
    }

    /// Rotational agreement up to sign (q and -q are the same rotation).
    fn same_rotation_up_to_sign(
        a: Quaternion,
        b: Quaternion,
    ) -> bool {
        let dot = a.w * b.w + a.x * b.x + a.y * b.y + a.z * b.z;
        (dot.abs() - 1.0).abs() < 1e-12
    }

    #[test]
    fn slerp_interior_points_follow_the_shortest_geodesic() {
        // Negating the second operand forces the shortest-path flip, so
        // the interior points run through the sin-weighted branch on the
        // effective arc 0.1 -> 3.0 radians about z.
        let q1 = rotation_about_z(0.1);
        let q2 = rotation_about_z(3.0).scale(-1.0);

        for t in [0.25, 0.5, 0.7] {
            let s = q1.slerp(q2, t);
            assert_abs_diff_eq!(s.norm(), 1.0, epsilon = 1e-12);
            let expected = rotation_about_z(0.1 + 2.9 * t);
            assert!(
                same_rotation_up_to_sign(s, expected),
                "slerp at t={t} left the geodesic: {s:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn slerp_near_antipodal_pair_stays_unit_and_on_the_small_arc() {
        // dot = -cos(0.01) ~ -0.99995: the numerically riskiest region.
        // After the shortest-path flip the effective arc is 0.02 radians,
        // so the midpoint is the 0.01-radian rotation, up to sign.
        let q1 = Quaternion::identity();
        let q2 = rotation_about_z(0.02).scale(-1.0);

        let s = q1.slerp(q2, 0.5);
        assert_abs_diff_eq!(s.norm(), 1.0, epsilon = 1e-12);
        assert!(
            same_rotation_up_to_sign(s, rotation_about_z(0.01)),
            "near-antipodal midpoint off the small arc: {s:?}"
        );
    }

    /// The four components as raw bit patterns, for exact comparison.
    ///
    /// The three tests below pin `slerp` bit for bit instead of comparing it
    /// within a tolerance. `sqrt`, `sin`, and `acos` are `libm`'s in every
    /// feature mode, so the same bits must come out under `cargo test` and
    /// under `cargo test --no-default-features`: a platform intrinsic put
    /// back into the math path moves the last bits in one mode only — by one
    /// ulp per component in the interior case below, on x86-64 glibc — and
    /// that is precisely what a tolerant comparison cannot see. The operands
    /// are decimal literals rather than computed rotations so that no
    /// platform trig function reaches the inputs either. A `libm` upgrade
    /// that moves a last bit fails these too; re-derive the constants then,
    /// deliberately.
    fn bits(q: Quaternion) -> [u64; 4] {
        [q.w.to_bits(), q.x.to_bits(), q.y.to_bits(), q.z.to_bits()]
    }

    #[test]
    fn slerp_interior_point_is_bit_pinned() {
        // Halfway along a 1.22-radian arc about x (operands: cos and sin of
        // 0.61), so the trig branch runs on a dot of 0.82. The exact answer
        // is (cos(0.305), sin(0.305)), whose correctly rounded doubles are
        // 0x3FEE_85EA_0B55_5D66 and 0x3FD3_3800_DDAF_16F5 — one ulp below
        // each pinned value.
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(0.819_648_017_845_479_5, 0.572_867_460_100_481_3, 0.0, 0.0);

        assert_eq!(
            bits(q1.slerp(q2, 0.5)),
            [0x3FEE_85EA_0B55_5D67, 0x3FD3_3800_DDAF_16F6, 0, 0]
        );
    }

    #[test]
    fn slerp_near_antipodal_point_is_bit_pinned() {
        // The negated 0.02-radian rotation about z: dot is -cos(0.01), so
        // the shortest-path flip fires and the trig branch runs on the
        // 0.01-radian remainder, where acos gives up most of its input
        // precision. Here the pinned values are the correctly rounded
        // cos(0.005) and sin(0.005) exactly.
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(
            -0.999_950_000_416_665_3,
            0.0,
            0.0,
            -0.009_999_833_334_166_664,
        );

        assert_eq!(
            bits(q1.slerp(q2, 0.5)),
            [0x3FEF_FFE5_C920_A926, 0, 0, 0x3F74_7ADB_B006_A986]
        );
    }

    #[test]
    fn slerp_near_identity_point_is_bit_pinned() {
        // Two rotations a ten-thousandth of a radian apart, well inside
        // UNIT_NORM_TOLERANCE of unit length: dot rounds to exactly 1.0, so
        // the switchover takes the normalized-lerp branch and this pins
        // `libm::sqrt` where the two above pin the trig path.
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(1.0, 0.0001, 0.0, 0.0);

        assert_eq!(
            bits(q1.slerp(q2, 0.25)),
            [0x3FEF_FFFF_FFD5_0CE2, 0x3EFA_36E2_EAF9_13FA, 0, 0]
        );
    }
}
