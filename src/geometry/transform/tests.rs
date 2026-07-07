#[cfg(test)]
mod transform_tests {
    use crate::{
        errors::TransformError,
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
    };

    #[test]
    fn transform_creation() {
        let translation = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let rotation = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let timestamp = Timestamp { t: 0 };
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
            translation: Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3 {
                x: 0.,
                y: 2.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).unwrap();

        assert_eq!(
            result.translation,
            Vector3 {
                x: 1.,
                y: 2.,
                z: 0.
            }
        );
        assert_eq!(result.parent, "a");
        assert_eq!(result.child, "c");
    }

    #[test]
    fn mul_with_rotation() {
        let t = Timestamp::zero();
        let theta = core::f64::consts::PI / 2.0;

        let t_a_b = Transform {
            translation: Vector3 {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: (theta / 2.0).cos(),
                x: 0.,
                y: 0.,
                z: (theta / 2.0).sin(),
            },
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        let result = (t_a_b * t_b_c).unwrap();

        assert!((result.translation.x - 0.).abs() < 1e-10);
        assert!((result.translation.y - 1.).abs() < 1e-10);
    }

    #[test]
    fn inverse() {
        let t_a_b = Transform {
            translation: Vector3 {
                x: 1.,
                y: 2.,
                z: 3.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_a = t_a_b.inverse().unwrap();

        assert_eq!(
            t_b_a.translation,
            Vector3 {
                x: -1.,
                y: -2.,
                z: -3.
            }
        );
        assert_eq!(t_b_a.parent, "b");
        assert_eq!(t_b_a.child, "a");
    }

    #[test]
    fn mul_inverse_identity() {
        let t_a_b = Transform {
            translation: Vector3 {
                x: 1.,
                y: 2.,
                z: 3.,
            },
            rotation: Quaternion {
                w: 0.707,
                x: 0.707,
                y: 0.,
                z: 0.,
            }
            .normalize()
            .unwrap(),
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
            translation: Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        let t_now = Timestamp { t: 1_000_000_000 };

        let t_b_c = Transform {
            translation: Vector3 {
                x: 0.,
                y: 1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t_now,
            parent: "b".into(),
            child: "c".into(),
        };

        let t_a_c_expected = Transform {
            translation: Vector3 {
                x: 1.,
                y: 1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
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
        let t_now = Timestamp { t: 1_000_000_000 };

        let t_a_b = Transform {
            translation: Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t_now,
            parent: "a".into(),
            child: "b".into(),
        };

        let t_b_c = Transform {
            translation: Vector3 {
                x: 0.,
                y: 1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: Timestamp::zero(),
            parent: "b".into(),
            child: "c".into(),
        };

        let t_a_c_expected = Transform {
            translation: Vector3 {
                x: 1.,
                y: 1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
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
            translation: Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
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
        let t = Timestamp { t: 1_000_000_000 };
        let t_a_b = transform_at("a", "b", t);
        let t_b_c = transform_at("b", "c", t);

        let result = t_b_c * t_a_b;
        assert!(
            matches!(result, Err(TransformError::IncompatibleFrames)),
            "reversed composition must be rejected, got {result:?}"
        );
    }

    #[test]
    fn mul_rejects_unrelated_frames() {
        let t = Timestamp { t: 1_000_000_000 };
        let t_a_b = transform_at("a", "b", t);
        let t_c_d = transform_at("c", "d", t);

        let result = t_a_b * t_c_d;
        assert!(
            matches!(result, Err(TransformError::IncompatibleFrames)),
            "unrelated frames must be rejected, got {result:?}"
        );
    }

    #[test]
    fn mul_rejects_mismatched_timestamps() {
        let t_a_b = transform_at("a", "b", Timestamp { t: 1_000_000_000 });
        let t_b_c = transform_at("b", "c", Timestamp { t: 2_000_000_000 });

        let result = t_a_b * t_b_c;
        assert!(
            matches!(result, Err(TransformError::TimestampMismatch(_, _))),
            "dynamic transforms with different timestamps must be rejected, got {result:?}"
        );
    }
}
