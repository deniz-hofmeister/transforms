#[cfg(test)]
mod registry_tests {
    use crate::{
        Registry, Transformable,
        errors::{BufferError, TransformError},
        geometry::{Point, Quaternion, Transform, Vector3},
        time::Timestamp,
    };
    use approx::assert_abs_diff_eq;
    use core::time::Duration;

    #[test]
    fn basic_chain_linear() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at y=1m
        let t_b_c = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();

        let t_a_c = Transform {
            translation: Vector3::new(1.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "c".into(),
        };

        let r = registry.get_transform("a", "c", t_a_b.timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_chain_linear_reverse() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at y=1m
        let t_b_c = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();

        let t_c_a = Transform {
            translation: Vector3::new(-1.0, -1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "c".into(),
            child: "a".into(),
        };

        let r = registry.get_transform("c", "a", t_a_b.timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_c_a);
    }

    #[test]
    fn basic_chain_rotation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at +90 degrees
        let theta = core::f64::consts::PI / 2.0;
        let t_b_c = Transform {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame D at x=1m
        let t_c_d = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "c".into(),
            child: "d".into(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();
        registry.add_transform(t_c_d.clone()).unwrap();

        let t_a_d = Transform {
            translation: Vector3::new(1.0, 1.0, 0.0),
            rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            timestamp: t,
            parent: "a".into(),
            child: "d".into(),
        };
        let r = registry.get_transform("a", "d", t_a_b.timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_d);
    }

    #[test]
    fn basic_exact_match() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_c = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            timestamp: t,
            parent: "a".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_a_c.clone()).unwrap();

        let r = registry.get_transform("a", "b", t_a_b.timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_b);

        let r = registry.get_transform("a", "c", t_a_c.timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_interpolation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b_0 = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame B at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_b_1 = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "a".into(),
            child: "b".into(),
        };

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();

        let middle_timestamp = Timestamp::from_nanos(u128::midpoint(
            t_a_b_0.timestamp.as_nanos(),
            t_a_b_1.timestamp.as_nanos(),
        ));

        let t_a_b_2 = Transform {
            translation: (t_a_b_0.translation + t_a_b_1.translation) / 2.0,
            rotation: (t_a_b_0.rotation.slerp(t_a_b_1.rotation, 0.5)),
            timestamp: middle_timestamp,
            parent: "a".into(),
            child: "b".into(),
        };

        let r = registry.get_transform("a", "b", middle_timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_b_2);
    }

    #[test]
    fn basic_chained_interpolation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, x=1m without rotation
        let t_a_b_0 = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame B at t=1, x=2m without rotation
        let t_a_b_1 = Transform {
            translation: Vector3::new(2.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "a".into(),
            child: "b".into(),
        };
        // Child frame C at t=0, y=1m without rotation
        let t_b_c_0 = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame C at t=1, y=2m without rotation
        let t_b_c_1 = Transform {
            translation: Vector3::new(0.0, 2.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();
        registry.add_transform(t_b_c_0.clone()).unwrap();
        registry.add_transform(t_b_c_1.clone()).unwrap();

        let middle_timestamp = Timestamp::from_nanos(u128::midpoint(
            t_a_b_0.timestamp.as_nanos(),
            t_a_b_1.timestamp.as_nanos(),
        ));

        let t_a_c = Transform {
            translation: Vector3::new(1.5, 1.5, 0.0),
            rotation: Quaternion::identity(),
            timestamp: middle_timestamp,
            parent: "a".into(),
            child: "c".into(),
        };

        let r = registry.get_transform("a", "c", middle_timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_branch_navigation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at t=0, x=1m without rotation
        let t_b_c = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform {
            translation: Vector3::new(2.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "d".into(),
        };

        registry.add_transform(t_a_b).unwrap();
        registry.add_transform(t_b_c).unwrap();
        registry.add_transform(t_b_d).unwrap();

        let result = registry.get_transform("c", "d", t);

        assert!(result.is_ok());

        let t_c_d = result.unwrap();
        let t_c_d_expected = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "c".into(),
            child: "d".into(),
        };

        assert_abs_diff_eq!(t_c_d, t_c_d_expected);
    }

    #[test]
    fn basic_common_parent_elimination() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at t=0, x=1m without rotation
        let t_b_c = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform {
            translation: Vector3::new(2.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "d".into(),
        };

        registry.add_transform(t_a_b).unwrap();
        registry.add_transform(t_b_c).unwrap();
        registry.add_transform(t_b_d).unwrap();

        let mut walk_failure = None;
        let from_chain =
            Registry::get_transform_chain("d", "a", t, &registry.data, &mut walk_failure);
        let mut to_chain =
            Registry::get_transform_chain("c", "a", t, &registry.data, &mut walk_failure);

        if let Some(chain) = to_chain.as_mut() {
            Registry::reverse_and_invert_transforms(chain).expect("failed to reverse and invert");
        }

        assert!(from_chain.is_some());
        assert!(to_chain.is_some());

        let mut from = from_chain.unwrap();
        let mut to = to_chain.unwrap();

        Registry::truncate_at_common_parent(&mut from, &mut to);
        let result = Registry::combine_transforms(from, to).expect("chains are non-empty");

        assert!(result.is_ok(), "combine_transforms failed: {result:?}");
    }

    #[test]
    fn time_travel_different_frames() {
        // All three frames (fixed, target, source) are different, so both
        // process_get_transform lookups are non-trivial (no identity shortcut).
        //
        // Tree: fixed -> a -> b
        // At t1: a at x=1 in fixed, b at y=1 in a  → b in fixed = (1,1,0)
        // At t2: a at x=2 in fixed, b at y=2 in a  → a in fixed = (2,0,0)
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   = (-2,0,0) + (1,1,0) = (-1, 1, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1
        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // fixed -> a at t2: a has moved to x=2
        registry
            .add_transform(Transform {
                translation: Vector3::new(2.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // a -> b at t1: b is at y=1 relative to a
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // a -> b at t2: b is at y=2 relative to a
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 2.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        assert!(
            (tf.translation.x - (-1.0)).abs() < f64::EPSILON,
            "Expected x=-1.0, got {}",
            tf.translation.x
        );
        assert!(
            (tf.translation.y - 1.0).abs() < f64::EPSILON,
            "Expected y=1.0, got {}",
            tf.translation.y
        );
        assert!(
            tf.translation.z.abs() < f64::EPSILON,
            "Expected z=0.0, got {}",
            tf.translation.z
        );
    }

    #[test]
    fn time_travel_same_time() {
        // When source_time == target_time, time travel should match get_transform.
        // Uses target != fixed so both lookups are non-trivial.
        //
        // Tree: fixed -> a -> b, all at time t
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        let regular = registry.get_transform("a", "b", t);
        let time_travel = registry.get_transform_at("a", t, "b", t, "fixed");

        assert!(regular.is_ok());
        assert!(time_travel.is_ok());

        let regular_tf = regular.unwrap();
        let time_travel_tf = time_travel.unwrap();

        assert_abs_diff_eq!(regular_tf.translation, time_travel_tf.translation);
        assert_abs_diff_eq!(regular_tf.rotation, time_travel_tf.rotation);
    }

    #[test]
    fn time_travel_with_rotation() {
        // All three frames different, with rotation on the target frame.
        //
        // Tree: fixed -> a -> b
        // At t1: a at (1,0,0) no rotation, b at (0.5,0,0) in a
        //   → b in fixed at t1 = (1.5, 0, 0)
        // At t2: a at origin rotated 90° CCW around z, b at (0.5,0,0) in a
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   a-in-fixed@t2 = {t: (0,0,0), R: 90°}  → inverse = {t: (0,0,0), R: -90°}
        //   R(-90°) * (1.5, 0, 0) = (0, -1.5, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        let theta = core::f64::consts::PI / 2.0;

        // fixed -> a at t1: at (1,0,0), no rotation
        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // fixed -> a at t2: at origin, rotated 90° CCW around z
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 0.0, 0.0),
                rotation: Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
                timestamp: t2,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // a -> b at t1: b is at (0.5, 0, 0) relative to a
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.5, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // a -> b at t2: b still at (0.5, 0, 0) relative to a
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.5, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(
            result.is_ok(),
            "Time travel with rotation failed: {result:?}"
        );
        let tf = result.unwrap();

        // b was at (1.5, 0, 0) in fixed at t1.
        // a is at origin rotated 90° CCW at t2.
        // In a's frame at t2: R(-90°) * (1.5, 0, 0) = (0, -1.5, 0)
        assert!(
            tf.translation.x.abs() < 1e-10,
            "Expected x=0.0, got {}",
            tf.translation.x
        );
        assert!(
            (tf.translation.y - (-1.5)).abs() < 1e-10,
            "Expected y=-1.5, got {}",
            tf.translation.y
        );
        assert!(
            tf.translation.z.abs() < 1e-10,
            "Expected z=0.0, got {}",
            tf.translation.z
        );
    }

    #[test]
    fn time_travel_branching_tree() {
        // Tree is a <- fixed -> b (source and target on separate branches).
        //
        // At t1: fixed->a is (1,0,0), fixed->b is (0,1,0)
        //   → b in fixed at t1 = (0,1,0)
        // At t2: fixed->a is (2,0,0), fixed->b is (0,2,0)
        //   → a in fixed at t2 = (2,0,0)
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   = (-2,0,0) + (0,1,0) = (-2, 1, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1
        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // fixed -> a at t2
        registry
            .add_transform(Transform {
                translation: Vector3::new(2.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        // fixed -> b at t1
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "fixed".into(),
                child: "b".into(),
            })
            .unwrap();

        // fixed -> b at t2
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 2.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "fixed".into(),
                child: "b".into(),
            })
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(
            result.is_ok(),
            "Time travel with branching tree failed: {result:?}"
        );
        let tf = result.unwrap();

        assert!(
            (tf.translation.x - (-2.0)).abs() < f64::EPSILON,
            "Expected x=-2.0, got {}",
            tf.translation.x
        );
        assert!(
            (tf.translation.y - 1.0).abs() < f64::EPSILON,
            "Expected y=1.0, got {}",
            tf.translation.y
        );
        assert!(
            tf.translation.z.abs() < f64::EPSILON,
            "Expected z=0.0, got {}",
            tf.translation.z
        );
    }

    #[test]
    fn time_travel_source_equals_fixed_returns_inverted_target_leg() {
        // "Where is the fixed/world origin relative to my platform now" —
        // source_frame == fixed_frame, a routine time-travel query that must
        // resolve to the inverse of the target leg, not error with
        // SameFrameMultiplication.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1; at t2: a has moved to x=2.
        for (t, x) in [(t1, 1.0), (t2, 2.0)] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(x, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "fixed".into(),
                    child: "a".into(),
                })
                .unwrap();
        }

        let result = registry.get_transform_at("a", t2, "fixed", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        // Inverse of fixed -> a at t2 (x=2): the origin sits at x=-2 in "a".
        assert_eq!(tf.parent, "a");
        assert_eq!(tf.child, "fixed");
        assert_eq!(tf.timestamp, t2);
        assert!(
            (tf.translation.x - (-2.0)).abs() < f64::EPSILON,
            "Expected x=-2.0, got {}",
            tf.translation.x
        );
    }

    #[test]
    fn time_travel_target_equals_fixed_returns_source_leg() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1; at t2: a has moved to x=2.
        for (t, x) in [(t1, 1.0), (t2, 2.0)] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(x, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "fixed".into(),
                    child: "a".into(),
                })
                .unwrap();
        }

        let result = registry.get_transform_at("fixed", t2, "a", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        // The source leg alone: a at t1 (x=1), stamped with target_time.
        assert_eq!(tf.parent, "fixed");
        assert_eq!(tf.child, "a");
        assert_eq!(tf.timestamp, t2);
        assert!(
            (tf.translation.x - 1.0).abs() < f64::EPSILON,
            "Expected x=1.0, got {}",
            tf.translation.x
        );
    }

    #[test]
    fn time_travel_all_frames_equal_returns_identity() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // The registry content is irrelevant for the degenerate query, but
        // keep it non-empty to mirror real use.
        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "fixed".into(),
                child: "a".into(),
            })
            .unwrap();

        let result = registry.get_transform_at("fixed", t2, "fixed", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        assert_eq!(tf.parent, "fixed");
        assert_eq!(tf.child, "fixed");
        assert_eq!(tf.timestamp, t2);
        assert_eq!(tf.translation, Vector3::zero());
        assert_eq!(tf.rotation, Quaternion::identity());
    }

    #[test]
    fn get_transform_at_unknown_fixed_frame_returns_not_found() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // Tree: fixed -> a -> b, known at both times.
        for &t in &[t1, t2] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "fixed".into(),
                    child: "a".into(),
                })
                .unwrap();
            registry
                .add_transform(Transform {
                    translation: Vector3::new(0.0, 1.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "a".into(),
                    child: "b".into(),
                })
                .unwrap();
        }

        // The fixed frame is not part of the tree: neither leg of the time
        // travel can resolve, so the whole query must fail loudly instead of
        // silently picking another reference — naming the unknown frame.
        let result = registry.get_transform_at("a", t2, "b", t1, "nowhere");
        assert!(
            matches!(&result, Err(TransformError::UnknownFrame(frame)) if frame == "nowhere"),
            "expected UnknownFrame for unknown fixed frame, got {result:?}"
        );
    }

    #[test]
    fn get_transform_at_missing_data_at_requested_times_returns_error() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        // fixed -> a is known at t1 and t2; a -> b only at t1.
        for &t in &[t1, t2] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "fixed".into(),
                    child: "a".into(),
                })
                .unwrap();
        }
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // The source frame has no data at the requested source time: the
        // b -> fixed leg cannot resolve at t2, and the error names "b" as
        // the frame that could not serve the time.
        let result = registry.get_transform_at("a", t1, "b", t2, "fixed");
        assert!(
            matches!(&result, Err(TransformError::NotFoundAt { frame, .. }) if frame == "b"),
            "expected NotFoundAt naming frame b for missing source data, got {result:?}"
        );

        // The target frame has no data at the requested target time: the
        // a -> fixed leg cannot resolve at t3 (no extrapolation).
        let result = registry.get_transform_at("a", t3, "b", t1, "fixed");
        assert!(
            matches!(&result, Err(TransformError::NotFoundAt { frame, .. }) if frame == "a"),
            "expected NotFoundAt naming frame a for missing target data, got {result:?}"
        );
    }

    #[test]
    fn get_transform_for_success_with_point() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(2.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "map".into(),
                child: "camera".into(),
            })
            .unwrap();

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: t,
            frame: "camera".into(),
        };

        let transform = registry.get_transform_for(&point, "map");

        assert!(transform.is_ok(), "get_transform_for failed: {transform:?}");
        let transform = transform.unwrap();
        assert_eq!(transform.parent, "map");
        assert_eq!(transform.child, "camera");
        assert_eq!(transform.timestamp, t);

        let result = point.transform(&transform);
        assert!(result.is_ok(), "transform apply failed: {result:?}");
        assert_eq!(point.frame, "map");
        assert_eq!(point.timestamp, t);
        assert_eq!(point.position, Vector3::new(3.0, 0.0, 0.0));
    }

    #[test]
    fn get_transform_for_same_frame_returns_identity_on_empty_registry() {
        let registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        let mut point = Point {
            position: Vector3::new(1.0, 2.0, 3.0),
            orientation: Quaternion::identity(),
            timestamp: t,
            frame: "camera".into(),
        };

        let transform = registry.get_transform_for(&point, "camera");

        assert!(
            transform.is_ok(),
            "same-frame get_transform_for should be Ok: {transform:?}"
        );
        let transform = transform.unwrap();
        assert_eq!(transform.parent, "camera");
        assert_eq!(transform.child, "camera");
        assert_eq!(transform.timestamp, t);
        assert_eq!(transform.translation, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.rotation, Quaternion::identity());

        let result = point.transform(&transform);
        assert!(result.is_ok(), "identity apply failed: {result:?}");
        assert_eq!(point.frame, "camera");
        assert_eq!(point.position, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn get_transform_for_propagates_lookup_error() {
        let registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        let point = Point {
            position: Vector3::new(0.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: t,
            frame: "camera".into(),
        };

        let result = registry.get_transform_for(&point, "map");

        assert!(
            matches!(&result, Err(TransformError::UnknownFrame(frame)) if frame == "map"),
            "expected UnknownFrame on an empty registry, got {result:?}"
        );
    }

    #[test]
    fn add_transform_rejects_cycles() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // Two-frame cycle: a -> b already exists, so b -> a must be rejected.
        let result = registry.add_transform(Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "a".into(),
        });
        assert!(matches!(result, Err(BufferError::CycleDetected)));

        // The direct lookup keeps working; the poisoning path is gone.
        assert!(registry.get_transform("a", "b", t).is_ok());

        // Three-frame cycle: extend the chain, then try to close it.
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "b".into(),
                child: "c".into(),
            })
            .unwrap();
        let result = registry.add_transform(Transform {
            translation: Vector3::new(0.0, 0.0, 1.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "c".into(),
            child: "a".into(),
        });
        assert!(matches!(result, Err(BufferError::CycleDetected)));
    }

    #[test]
    fn add_transform_rejects_self_referential_frames() {
        let mut registry = Registry::new();

        let result = registry.add_transform(Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::from_nanos(1_000_000_000),
            parent: "a".into(),
            child: "a".into(),
        });
        assert!(matches!(result, Err(BufferError::SelfReferentialFrame)));
    }

    #[test]
    fn add_transform_rejects_reparenting() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "world".into(),
                child: "object".into(),
            })
            .unwrap();

        // The object is "picked up": its parent changes. Not supported;
        // the frame must be removed first.
        let reparented = Transform {
            translation: Vector3::new(0.0, 0.5, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t2,
            parent: "gripper".into(),
            child: "object".into(),
        };
        let result = registry.add_transform(reparented.clone());
        assert!(matches!(
            result,
            Err(BufferError::ReparentingNotSupported(parent)) if parent == "world"
        ));

        // remove_frame is the escape hatch: after removal the new parent is
        // accepted.
        assert!(registry.remove_frame("object"));
        assert!(!registry.remove_frame("object"));
        registry.add_transform(reparented).unwrap();
        assert!(registry.get_transform("gripper", "object", t2).is_ok());
        assert!(registry.get_transform("world", "object", t1).is_err());
    }

    #[test]
    fn delete_transforms_before_prunes_empty_frames() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "world".into(),
                child: "object".into(),
            })
            .unwrap();

        // Wiping every transform of a frame releases the frame itself, so
        // the registry does not accumulate dead frames — and the frame can
        // come back under a new parent.
        registry.delete_transforms_before(t2);
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 0.5, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t2,
                parent: "gripper".into(),
                child: "object".into(),
            })
            .unwrap();
        assert!(registry.get_transform("gripper", "object", t2).is_ok());
    }

    #[test]
    fn get_transform_unknown_frame_returns_not_found() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // The requested frame does not exist. The walk from "b" still resolves
        // up to the root "a", but that partial answer must not be returned as
        // if it were the requested transform; the error names the unknown
        // frame.
        let result = registry.get_transform("b", "does_not_exist", t);
        assert!(
            matches!(&result, Err(TransformError::UnknownFrame(frame)) if frame == "does_not_exist"),
            "expected UnknownFrame for unknown target frame, got {result:?}"
        );

        let result = registry.get_transform("does_not_exist", "b", t);
        assert!(
            matches!(&result, Err(TransformError::UnknownFrame(frame)) if frame == "does_not_exist"),
            "expected UnknownFrame for unknown source frame, got {result:?}"
        );
    }

    #[test]
    // The compared seconds are exactly representable; the assertion is on
    // the reported values, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn get_transform_partial_chain_reports_failing_frame() {
        let mut registry = Registry::new();
        let t0 = Timestamp::from_nanos(1_000_000_000);

        let t1 = (t0 + Duration::from_secs(1)).unwrap();

        // a -> b is only known at t0; b -> c is known at t0 and t1.
        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t0,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();
        for &t in &[t0, t1] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(0.0, 1.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "b".into(),
                    child: "c".into(),
                })
                .unwrap();
        }

        // At t1 only the c -> b hop can be resolved; the chain to "a" is
        // incomplete and must not be returned as a c -> a transform. The
        // error pinpoints "b" as the frame that could not serve t1 and
        // carries the covered range as the cause: b's data ends at t0
        // (1.0s), one second before the requested t1 (2.0s).
        let result = registry.get_transform("c", "a", t1);
        assert!(
            matches!(
                &result,
                Err(TransformError::NotFoundAt { frame, source, .. })
                    if frame == "b"
                        && matches!(
                            source.as_ref(),
                            BufferError::TransformError(TransformError::TimestampOutOfRange { requested, start, end }) if *requested == 2.0 && *start == 1.0 && *end == 1.0
                        )
            ),
            "expected NotFoundAt naming frame b with the covered range, got {result:?}"
        );
    }

    #[test]
    fn get_transform_mid_chain_gap_reports_gap_frame() {
        // Tree: r -> a -> b -> c. The a -> b hop is only known at t1; the
        // others are known at t1 and t3. A query at t2 hits a timestamp gap
        // in the MIDDLE of the chain, so both partial walks stop in
        // different subtrees. That is a transient data gap and must be
        // reported as NotFoundAt naming the gap frame — not
        // IncompatibleFrames, whose "frames do not have a parent-child
        // relationship" message is false here.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        for &t in &[t1, t3] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "r".into(),
                    child: "a".into(),
                })
                .unwrap();
            registry
                .add_transform(Transform {
                    translation: Vector3::new(0.0, 0.0, 1.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "b".into(),
                    child: "c".into(),
                })
                .unwrap();
        }
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // With all hops resolvable (t1) the chain works: the topology is
        // intact and only the data gap at t2 must trip the lookup.
        let result = registry.get_transform("a", "c", t1);
        assert!(
            result.is_ok(),
            "expected chain at t1 to resolve: {result:?}"
        );

        let result = registry.get_transform("a", "c", t2);
        assert!(
            matches!(&result, Err(TransformError::NotFoundAt { frame, .. }) if frame == "b"),
            "expected NotFoundAt naming the gap frame b, got {result:?}"
        );
    }

    #[test]
    fn get_transform_disconnected_trees_returns_disconnected() {
        // Two disjoint trees: r1 -> a and r2 -> b. There is no path between
        // "a" and "b", which must be reported as Disconnected — not as a
        // failed composition of the two unrelated root transforms, and not
        // as a data gap: both frames exist and both walks complete cleanly,
        // so the disconnection is a statement about the current topology.
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "r1".into(),
                child: "a".into(),
            })
            .unwrap();
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "r2".into(),
                child: "b".into(),
            })
            .unwrap();

        let result = registry.get_transform("a", "b", t);
        assert!(
            matches!(
                &result,
                Err(TransformError::Disconnected { target_frame, source_frame })
                    if target_frame == "a" && source_frame == "b"
            ),
            "expected Disconnected for frames in disconnected trees, got {result:?}"
        );
    }

    #[test]
    fn get_transform_unknown_frame_takes_precedence_over_data_gap() {
        // a -> b holds data at t1 only. Querying b -> "nope" at t2 records
        // a data gap during the walk AND asks for a frame that does not
        // exist. The unknown frame is the more fundamental error — no
        // amount of waiting for data can make the lookup succeed — so it
        // must win the diagnosis.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t1,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        let result = registry.get_transform("b", "nope", t2);
        assert!(
            matches!(&result, Err(TransformError::UnknownFrame(frame)) if frame == "nope"),
            "expected UnknownFrame to take precedence over the data gap, got {result:?}"
        );
    }

    #[test]
    fn add_transform_rejects_static_dynamic_mixing() {
        let t_dynamic = Timestamp::from_nanos(1_000_000_000);

        let static_tf = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::STATIC,
            parent: "a".into(),
            child: "b".into(),
        };
        let dynamic_tf = Transform {
            translation: Vector3::new(2.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t_dynamic,
            parent: "a".into(),
            child: "b".into(),
        };

        // Static first, then dynamic.
        let mut registry = Registry::new();

        registry.add_transform(static_tf.clone()).unwrap();
        assert!(
            matches!(
                registry.add_transform(dynamic_tf.clone()),
                Err(BufferError::StaticDynamicConflict)
            ),
            "dynamic insert into a static child frame must be rejected"
        );

        // Dynamic first, then static.
        let mut registry = Registry::new();

        registry.add_transform(dynamic_tf.clone()).unwrap();
        assert!(
            matches!(
                registry.add_transform(static_tf),
                Err(BufferError::StaticDynamicConflict)
            ),
            "static insert into a dynamic child frame must be rejected"
        );

        // The registry state is untouched by the rejected insert.
        let result = registry.get_transform("a", "b", t_dynamic);
        assert_eq!(result.unwrap(), dynamic_tf);
    }

    #[test]
    fn delete_transforms_before_removes_old_dynamic_transforms() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        for &t in &[t1, t2] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "a".into(),
                    child: "b".into(),
                })
                .unwrap();
        }

        registry.delete_transforms_before(Timestamp::from_nanos(2_000_000_000));

        assert!(
            registry.get_transform("a", "b", t1).is_err(),
            "transforms before the cutoff must be deleted"
        );
        assert!(registry.get_transform("a", "b", t2).is_ok());
    }

    #[test]
    fn delete_transforms_before_preserves_static_transforms() {
        let mut registry = Registry::new();

        let static_tf = Transform {
            translation: Vector3::new(0.5, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::STATIC,
            parent: "base".into(),
            child: "lidar".into(),
        };
        registry.add_transform(static_tf.clone()).unwrap();

        // The documented manual-cleanup workflow must not destroy static
        // transforms: they are valid for all time.
        registry.delete_transforms_before(Timestamp::from_nanos(5_000_000_000));

        let query = Timestamp::from_nanos(9_000_000_000);
        let result = registry.get_transform("base", "lidar", query).unwrap();
        assert_eq!(
            result.translation, static_tf.translation,
            "static transforms must survive manual cleanup"
        );
        // Lookup results carry the requested timestamp, not the static
        // sentinel, so they compose with timestamped data.
        assert_eq!(result.timestamp, query);
    }

    #[test]
    fn mixed_static_dynamic_chain_resolves_and_interpolates() {
        let mut registry = Registry::new();

        // Static sensor mount: lidar sits 0.5 m ahead of base.
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.5, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: Timestamp::STATIC,
                parent: "base".into(),
                child: "lidar".into(),
            })
            .unwrap();

        // Dynamic robot pose: base moves from x=1 to x=3 between t1 and t2.
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);
        for (t, x) in [(t1, 1.0), (t2, 3.0)] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(x, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "map".into(),
                    child: "base".into(),
                })
                .unwrap();
        }

        // Query mid-way: the dynamic hop interpolates to x=2, the static hop
        // contributes its fixed 0.5 offset, and the result carries the query
        // timestamp.
        let mid = Timestamp::from_nanos(2_000_000_000);
        let result = registry.get_transform("map", "lidar", mid).unwrap();

        assert_eq!(result.parent, "map");
        assert_eq!(result.child, "lidar");
        assert_eq!(result.timestamp, mid);
        assert_abs_diff_eq!(result.translation, Vector3::new(2.5, 0.0, 0.0));
    }

    #[test]
    fn add_transform_rejects_invalid_rotations() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Rotation-equivalent to identity but non-unit: if accepted, it
        // would silently scale every lookup.
        let result = registry.add_transform(Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::new(2.0, 0.0, 0.0, 0.0),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        });
        assert!(matches!(
            result,
            Err(BufferError::TransformError(
                TransformError::NonUnitRotation(_)
            ))
        ));

        let result = registry.add_transform(Transform {
            translation: Vector3::new(f64::NAN, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        });
        assert!(matches!(
            result,
            Err(BufferError::TransformError(TransformError::NonFiniteValues))
        ));

        // Nothing was stored by the rejected inserts.
        assert!(registry.get_transform("a", "b", t).is_err());
    }

    #[test]
    fn with_max_age_expires_old_transforms_on_insert() {
        let mut registry = Registry::with_max_age(Duration::from_secs(1));

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(6_000_000_000);
        for &t in &[t1, t2] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "a".into(),
                    child: "b".into(),
                })
                .unwrap();
        }

        assert!(
            registry.get_transform("a", "b", t1).is_err(),
            "with_max_age registries must expire entries older than max_age"
        );
        assert!(registry.get_transform("a", "b", t2).is_ok());

        // A registry without max_age keeps everything.
        let mut registry = Registry::new();
        for &t in &[t1, t2] {
            registry
                .add_transform(Transform {
                    translation: Vector3::new(1.0, 0.0, 0.0),
                    rotation: Quaternion::identity(),
                    timestamp: t,
                    parent: "a".into(),
                    child: "b".into(),
                })
                .unwrap();
        }
        assert!(registry.get_transform("a", "b", t1).is_ok());
        assert!(registry.get_transform("a", "b", t2).is_ok());
    }

    #[test]
    fn failed_insert_does_not_bypass_cycle_detection() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // A rejected insert must not leave an empty frame behind in the
        // registry map...
        let invalid = Transform {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::new(2.0, 0.0, 0.0, 0.0),
            timestamp: t,
            parent: "b".into(),
            child: "a".into(),
        };
        assert!(registry.add_transform(invalid).is_err());

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // ...otherwise this valid insert would close the cycle a <-> b
        // without ever hitting the cycle check.
        let result = registry.add_transform(Transform {
            translation: Vector3::new(-1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "b".into(),
            child: "a".into(),
        });
        assert!(matches!(result, Err(BufferError::CycleDetected)));

        // The stored transform still resolves, unpoisoned.
        let result = registry.get_transform("a", "b", t).unwrap();
        assert_eq!(result.translation, Vector3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn same_frame_lookup_returns_identity() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(Transform {
                translation: Vector3::new(1.0, 0.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: t,
                parent: "a".into(),
                child: "b".into(),
            })
            .unwrap();

        // Identity for a known child frame, a root frame, and an unknown
        // frame alike: a frame relative to itself is always the identity.
        for frame in ["b", "a", "unknown"] {
            let result = registry.get_transform(frame, frame, t).unwrap();
            assert_eq!(result.parent, frame);
            assert_eq!(result.child, frame);
            assert_eq!(result.timestamp, t);
            assert_eq!(result.translation, Vector3::zero());
            assert_eq!(result.rotation, Quaternion::identity());
        }
    }

    #[test]
    fn static_chain_composes_with_timestamped_data() {
        let mut registry = Registry::new();

        // Purely static chain: base -> camera mount.
        registry
            .add_transform(Transform {
                translation: Vector3::new(0.0, 1.0, 0.0),
                rotation: Quaternion::identity(),
                timestamp: Timestamp::STATIC,
                parent: "base".into(),
                child: "camera".into(),
            })
            .unwrap();

        // A detection stamped at observation time, in the camera frame.
        let t = Timestamp::from_nanos(5_000_000_000);
        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: t,
            frame: "camera".into(),
        };

        // The flagship static-mount workflow: resolve and apply. The lookup
        // result carries the query time, so the application succeeds.
        let tf = registry.get_transform_for(&point, "base").unwrap();
        assert_eq!(tf.timestamp, t);
        point.transform(&tf).unwrap();
        assert_eq!(point.frame, "base");
        assert_eq!(point.position, Vector3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn static_transform_applies_directly_to_any_timestamp() {
        // A hand-built static transform (static sentinel timestamp) is valid
        // for all time when applied through Transformable.
        let static_tf = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::STATIC,
            parent: "base".into(),
            child: "camera".into(),
        };

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::from_nanos(5_000_000_000),
            frame: "camera".into(),
        };

        point.transform(&static_tf).unwrap();
        assert_eq!(point.frame, "base");
        assert_eq!(point.position, Vector3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn public_types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Registry>();
        assert_send_sync::<crate::core::Buffer>();
        assert_send_sync::<Transform>();
        assert_send_sync::<Point>();
        assert_send_sync::<Vector3>();
        assert_send_sync::<Quaternion>();
        assert_send_sync::<Timestamp>();
    }

    /// A transform translated by `x` along the x-axis.
    fn translated(
        parent: &str,
        child: &str,
        timestamp: Timestamp,
        x: f64,
    ) -> Transform {
        Transform {
            translation: Vector3::new(x, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp,
            parent: parent.into(),
            child: child.into(),
        }
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn duplicate_timestamp_add_is_a_last_write_wins_upsert() {
        let t = Timestamp::from_nanos(5_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", t, 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", t, 2.0))
            .unwrap();
        assert_eq!(
            registry.get_transform("a", "b", t).unwrap().translation.x,
            2.0
        );
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn duplicate_static_add_is_a_last_write_wins_upsert() {
        // The static sentinel is just another key: re-publishing the static
        // transform replaces it the same way.
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Timestamp::STATIC, 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Timestamp::STATIC, 7.0))
            .unwrap();
        let got = registry
            .get_transform("a", "b", Timestamp::from_nanos(3_000_000_000))
            .unwrap();
        assert_eq!(got.translation.x, 7.0);
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn zero_max_age_keeps_only_the_newest_sample() {
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);
        let mut registry = Registry::with_max_age(Duration::ZERO);
        registry
            .add_transform(translated("a", "b", t1, 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", t2, 2.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", t3, 3.0))
            .unwrap();

        // Exact hit on the newest sample still works.
        assert_eq!(
            registry.get_transform("a", "b", t3).unwrap().translation.x,
            3.0
        );

        // Older samples are gone: the covered range collapsed to [t3, t3].
        match registry.get_transform("a", "b", t2) {
            Err(TransformError::NotFoundAt { frame, source, .. }) => {
                assert_eq!(frame, "b");
                match *source {
                    BufferError::TransformError(TransformError::TimestampOutOfRange {
                        requested,
                        start,
                        end,
                    }) => {
                        assert_eq!(requested, 2.0);
                        assert_eq!(start, 3.0);
                        assert_eq!(end, 3.0);
                    }
                    other => panic!("unexpected buffer error: {other:?}"),
                }
            }
            other => panic!("expected NotFoundAt, got {other:?}"),
        }
    }

    #[test]
    fn remove_frame_mid_tree_strands_descendants() {
        // map -> odom -> base_link; removing odom strands base_link, whose
        // buffer keeps its pin to the removed parent. The subsequent lookup
        // is diagnosed relative to the remaining tree: "map" now exists
        // nowhere (it was only ever odom's parent), so the error names it —
        // the documented, deliberately-pinned behavior.
        let t = Timestamp::from_nanos(1_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("map", "odom", t, 1.0))
            .unwrap();
        registry
            .add_transform(translated("odom", "base_link", t, 1.0))
            .unwrap();

        assert!(registry.remove_frame("odom"));

        match registry.get_transform("map", "base_link", t) {
            Err(TransformError::UnknownFrame(frame)) => assert_eq!(frame, "map"),
            other => panic!("expected UnknownFrame, got {other:?}"),
        }
    }
}
