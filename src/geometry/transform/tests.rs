#[cfg(test)]
mod transform_tests {
    use crate::{
        errors::TransformError,
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
    };

    #[test]
    fn transform_creation() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let rotation = Quaternion::identity();
        let timestamp = Timestamp::zero();
        let parent = "map".into();
        let child = "base".into();

        let _ = Transform {
            translation,
            rotation,
            timestamp,
            parent,
            child,
        };
    }

    #[test]
    fn mul_translation() {
        let t = Timestamp::zero();

        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3::new(0.0, 2.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).unwrap();

        assert_eq!(result.translation, Vector3::new(1.0, 2.0, 0.0));
        assert_eq!(result.parent, "a");
        assert_eq!(result.child, "c");
    }

    #[test]
    fn mul_with_rotation() {
        let t = Timestamp::zero();
        let theta = core::f64::consts::PI / 2.0;

        let t_a_b = Transform {
            translation: Vector3::zero(),
            rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).unwrap();

        assert!((result.translation.x - 0.0).abs() < 1e-10);
        assert!((result.translation.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn inverse() {
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_a = t_a_b.inverse().unwrap();

        assert_eq!(t_b_a.translation, Vector3::new(-1.0, -2.0, -3.0));
        assert_eq!(t_b_a.parent, "b");
        assert_eq!(t_b_a.child, "a");
    }

    #[test]
    fn mul_inverse_identity() {
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::new(0.707, 0.707, 0.0, 0.0).normalize().unwrap(),
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_a = t_a_b.clone().inverse().unwrap();
        let result = (t_a_b * t_b_a).unwrap();

        let identity = Transform::<Timestamp>::identity();
        assert!(result.translation.x.abs() < 1e-10);
        assert!(result.translation.y.abs() < 1e-10);
        assert!(result.translation.z.abs() < 1e-10);
        assert!((result.rotation.w - identity.rotation.w).abs() < 1e-10);
    }

    #[test]
    fn mul_static_to_timestamped() {
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::STATIC,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_now = Timestamp::from_nanos(1_000_000_000);

        let t_b_c = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t_now,
            parent: "b".into(),
            child: "c".into(),
        };

        let t_a_c_expected = Transform {
            translation: Vector3::new(1.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t_now,
            parent: "a".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).expect("multiplication should succeed");

        assert_eq!(
            result, t_a_c_expected,
            "Static * Timestamped should produce timestamped result"
        );
    }

    #[test]
    fn mul_timestamped_to_static() {
        let t_now = Timestamp::from_nanos(1_000_000_000);

        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t_now,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::STATIC,
            parent: "b".into(),
            child: "c".into(),
        };

        let t_a_c_expected = Transform {
            translation: Vector3::new(1.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t_now,
            parent: "a".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).expect("multiplication should succeed");

        assert_eq!(
            result, t_a_c_expected,
            "Timestamped * Static should produce timestamped result"
        );
    }

    fn transform_at(
        parent: &str,
        child: &str,
        t: Timestamp,
    ) -> Transform {
        Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: parent.into(),
            child: child.into(),
        }
    }

    #[test]
    fn mul_rejects_reversed_composition() {
        // Only `t_a_b * t_b_c` (lhs child == rhs parent) is a valid
        // composition. The reversed order composes the underlying math in the
        // wrong order and must be rejected.
        let t = Timestamp::from_nanos(1_000_000_000);
        let t_a_b = transform_at("a", "b", t);
        let t_b_c = transform_at("b", "c", t);

        let result = t_b_c * t_a_b;
        assert!(
            matches!(result, Err(TransformError::IncompatibleFrames { .. })),
            "reversed composition must be rejected, got {result:?}"
        );
    }

    #[test]
    fn mul_rejects_unrelated_frames() {
        let t = Timestamp::from_nanos(1_000_000_000);
        let t_a_b = transform_at("a", "b", t);
        let t_c_d = transform_at("c", "d", t);

        let result = t_a_b * t_c_d;
        assert!(
            matches!(result, Err(TransformError::IncompatibleFrames { .. })),
            "unrelated frames must be rejected, got {result:?}"
        );
    }

    #[test]
    fn mul_rejects_mismatched_timestamps() {
        let t_a_b = transform_at("a", "b", Timestamp::from_nanos(1_000_000_000));
        let t_b_c = transform_at("b", "c", Timestamp::from_nanos(2_000_000_000));

        let result = t_a_b * t_b_c;
        assert!(
            matches!(result, Err(TransformError::TimestampMismatch { .. })),
            "dynamic transforms with different timestamps must be rejected, got {result:?}"
        );
    }

    #[test]
    fn interpolate_rejects_out_of_range_timestamps() {
        let from = transform_at("a", "b", Timestamp::from_nanos(1_000_000_000));
        let mut to = transform_at("a", "b", Timestamp::from_nanos(2_000_000_000));
        to.translation = Vector3::new(2.0, 0.0, 0.0);

        // Before the covered range: extrapolation must be rejected.
        let result = Transform::interpolate(&from, &to, Timestamp::from_nanos(500_000_000));
        assert!(
            matches!(result, Err(TransformError::TimestampOutOfRange { .. })),
            "interpolation before the range must fail, got {result:?}"
        );

        // After the covered range: extrapolation must be rejected.
        let result = Transform::interpolate(&from, &to, Timestamp::from_nanos(3_000_000_000));
        assert!(
            matches!(result, Err(TransformError::TimestampOutOfRange { .. })),
            "interpolation after the range must fail, got {result:?}"
        );

        // Swapped endpoints must be rejected.
        let result = Transform::interpolate(&to, &from, Timestamp::from_nanos(1_500_000_000));
        assert!(
            matches!(result, Err(TransformError::TimestampMismatch { .. })),
            "swapped endpoints must fail, got {result:?}"
        );
    }

    #[test]
    fn errors_survive_wall_clock_timestamps() {
        // Realistic wall-clock nanosecond values cannot be converted to
        // seconds exactly; error reporting previously failed with
        // AccuracyLoss instead of diagnosing the actual problem.
        let t1 = Timestamp::from_nanos(1_783_400_000_123_456_789);
        let t2 = Timestamp::from_nanos(1_783_400_001_123_456_789);

        let t_a_b = transform_at("a", "b", t1);
        let t_b_c = transform_at("b", "c", t2);
        let result = t_a_b * t_b_c;
        assert!(
            matches!(result, Err(TransformError::TimestampMismatch { .. })),
            "expected TimestampMismatch, got {result:?}"
        );

        let from = transform_at("a", "b", t1);
        let to = transform_at("a", "b", t2);
        let result =
            Transform::interpolate(&from, &to, Timestamp::from_nanos(1_783_400_002_000_000_000));
        assert!(
            matches!(result, Err(TransformError::TimestampOutOfRange { .. })),
            "expected TimestampOutOfRange, got {result:?}"
        );
    }

    #[test]
    fn interpolate_rejects_mismatched_frames() {
        let from = transform_at("a", "b", Timestamp::from_nanos(1_000_000_000));
        let to = transform_at("a", "c", Timestamp::from_nanos(2_000_000_000));

        let result = Transform::interpolate(&from, &to, Timestamp::from_nanos(1_500_000_000));
        assert!(
            matches!(result, Err(TransformError::IncompatibleFrames { .. })),
            "interpolating between different frame pairs must fail, got {result:?}"
        );
    }

    #[test]
    fn validate_enforces_finite_unit_rotations() {
        let t = Timestamp::from_nanos(1_000_000_000);

        assert!(transform_at("a", "b", t).validate().is_ok());

        // f32-grade precision loss on a unit rotation is accepted.
        let mut f32_grade = transform_at("a", "b", t);
        f32_grade.rotation = Quaternion::new(1.0 + 1e-8, 0.0, 0.0, 0.0);
        assert!(f32_grade.validate().is_ok());

        // A genuinely denormalized rotation is rejected with its norm.
        let mut denormalized = transform_at("a", "b", t);
        denormalized.rotation = Quaternion::new(1.001, 0.0, 0.0, 0.0);
        assert!(matches!(
            denormalized.validate(),
            Err(TransformError::NonUnitRotation(_))
        ));

        let mut non_finite = transform_at("a", "b", t);
        non_finite.rotation = Quaternion::new(f64::NAN, 0.0, 0.0, 0.0);
        assert!(matches!(
            non_finite.validate(),
            Err(TransformError::NonFiniteValues)
        ));

        let mut inf_translation = transform_at("a", "b", t);
        inf_translation.translation = Vector3::new(f64::INFINITY, 0.0, 0.0);
        assert!(matches!(
            inf_translation.validate(),
            Err(TransformError::NonFiniteValues)
        ));
    }

    #[test]
    fn same_child_multiplication_is_rejected_with_the_frame_named() {
        let t = Timestamp::from_nanos(1_000_000_000);
        let t_a_b = transform_at("a", "b", t);
        let t_c_b = transform_at("c", "b", t);

        // Same child frame on both sides. This check runs BEFORE the
        // parent/child pairing check, so it wins over IncompatibleFrames.
        match t_a_b * t_c_b {
            Err(TransformError::SameFrameMultiplication { frame }) => {
                assert_eq!(frame, "b");
            }
            other => panic!("expected SameFrameMultiplication, got {other:?}"),
        }
    }

    #[test]
    fn self_multiplication_is_same_frame_multiplication() {
        let t = Timestamp::from_nanos(1_000_000_000);
        let t_a_b = transform_at("a", "b", t);
        assert!(matches!(
            t_a_b.clone() * t_a_b,
            Err(TransformError::SameFrameMultiplication { .. })
        ));
    }

    #[test]
    fn unrelated_frames_multiplication_names_both_sides() {
        // Control for the check ordering: distinct children with no
        // parent/child match is IncompatibleFrames, carrying what the
        // composition required and what it found.
        let t = Timestamp::from_nanos(1_000_000_000);
        let t_a_b = transform_at("a", "b", t);
        let t_c_d = transform_at("c", "d", t);

        match t_a_b * t_c_d {
            Err(TransformError::IncompatibleFrames { expected, found }) => {
                assert_eq!(expected, "b");
                assert_eq!(found, "c");
            }
            other => panic!("expected IncompatibleFrames, got {other:?}"),
        }
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn timestamp_mismatch_payload_carries_both_times_in_seconds() {
        let t_a_b = transform_at("a", "b", Timestamp::from_nanos(1_000_000_000));
        let t_b_c = transform_at("b", "c", Timestamp::from_nanos(2_000_000_000));

        match t_a_b * t_b_c {
            Err(TransformError::TimestampMismatch { lhs, rhs }) => {
                assert_eq!(lhs, 1.0);
                assert_eq!(rhs, 2.0);
            }
            other => panic!("expected TimestampMismatch, got {other:?}"),
        }
    }
}
