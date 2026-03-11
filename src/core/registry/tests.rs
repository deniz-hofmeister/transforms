#[cfg(test)]
mod registry_tests {
    use crate::{
        errors::TransformError,
        geometry::{Point, Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry, Transformable,
    };
    use core::time::Duration;
    use log::debug;

    #[test]
    fn basic_chain_linear() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at x=1m without rotation
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

        // Child frame C at y=1m
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
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone());
        registry.add_transform(t_b_c.clone());

        let t_a_c = Transform {
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
            timestamp: t,
            parent: "a".into(),
            child: "c".into(),
        };

        let r = registry.get_transform("a", "c", t_a_b.timestamp);

        debug!("Result: {r:?}");
        debug!("Desired: {t_a_c:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_c,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_chain_linear_reverse() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at x=1m without rotation
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

        // Child frame C at y=1m
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
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone());
        registry.add_transform(t_b_c.clone());

        let t_c_a = Transform {
            translation: Vector3 {
                x: -1.,
                y: -1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "c".into(),
            child: "a".into(),
        };

        let r = registry.get_transform("c", "a", t_a_b.timestamp);

        debug!("Result: {r:?}");
        debug!("Desired: {t_c_a:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_c_a,
            "Registry returned a transform that is different"
        );
    }
    #[test]
    fn basic_chain_rotation() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at x=1m without rotation
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

        // Child frame C at +90 degrees
        let theta = core::f64::consts::PI / 2.0;
        let t_b_c = Transform {
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
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame D at x=1m
        let t_c_d = Transform {
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
            parent: "c".into(),
            child: "d".into(),
        };

        registry.add_transform(t_a_b.clone());
        registry.add_transform(t_b_c.clone());
        registry.add_transform(t_c_d.clone());

        let t_a_d = Transform {
            translation: Vector3 {
                x: 1.,
                y: 1.,
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
            child: "d".into(),
        };
        let r = registry.get_transform("a", "d", t_a_b.timestamp);

        debug!("Result: {r:?}");
        debug!("Desired: {t_a_d:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_d,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_exact_match() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at x=1m without rotation
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

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_c = Transform {
            translation: Vector3 {
                x: 0.,
                y: 1.,
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
            child: "c".into(),
        };

        registry.add_transform(t_a_b.clone());
        registry.add_transform(t_a_c.clone());

        let r = registry.get_transform("a", "b", t_a_b.timestamp);

        debug!("{r:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_b,
            "Registry returned a transform that is different"
        );

        let r = registry.get_transform("a", "c", t_a_c.timestamp);

        debug!("{r:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_c,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_interpolation() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at x=1m without rotation
        let t_a_b_0 = Transform {
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

        // Child frame B at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_b_1 = Transform {
            translation: Vector3 {
                x: 0.,
                y: 1.,
                z: 0.,
            },
            rotation: Quaternion {
                w: (theta / 2.0).cos(),
                x: 0.,
                y: 0.,
                z: (theta / 2.0).sin(),
            },
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "a".into(),
            child: "b".into(),
        };

        registry.add_transform(t_a_b_0.clone());
        registry.add_transform(t_a_b_1.clone());

        let middle_timestamp = Timestamp {
            t: u128::midpoint(t_a_b_0.timestamp.t, t_a_b_1.timestamp.t),
        };

        let t_a_b_2 = Transform {
            translation: (t_a_b_0.translation + t_a_b_1.translation) / 2.0,
            rotation: (t_a_b_0.rotation.slerp(t_a_b_1.rotation, 0.5)),
            timestamp: middle_timestamp,
            parent: "a".into(),
            child: "b".into(),
        };

        let r = registry.get_transform("a", "b", middle_timestamp);

        debug!("Result: {r:?}");
        debug!("Expected: {t_a_b_2:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_b_2,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_chained_interpolation() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at t=0, x=1m without rotation
        let t_a_b_0 = Transform {
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

        // Child frame B at t=1, x=2m without rotation
        let t_a_b_1 = Transform {
            translation: Vector3 {
                x: 2.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "a".into(),
            child: "b".into(),
        };
        // Child frame C at t=0, y=1m without rotation
        let t_b_c_0 = Transform {
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
            timestamp: t,
            parent: "b".into(),
            child: "c".into(),
        };

        // Child frame B at t=1, y=2m without rotation
        let t_b_c_1 = Transform {
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
            timestamp: (t + Duration::from_secs(1)).unwrap(),
            parent: "b".into(),
            child: "c".into(),
        };

        registry.add_transform(t_a_b_0.clone());
        registry.add_transform(t_a_b_1.clone());
        registry.add_transform(t_b_c_0.clone());
        registry.add_transform(t_b_c_1.clone());

        let middle_timestamp = Timestamp {
            t: u128::midpoint(t_a_b_0.timestamp.t, t_a_b_1.timestamp.t),
        };

        let t_a_c = Transform {
            translation: Vector3 {
                x: 1.5,
                y: 1.5,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: middle_timestamp,
            parent: "a".into(),
            child: "c".into(),
        };

        let r = registry.get_transform("a", "c", middle_timestamp);

        debug!("Result: {r:?}");
        debug!("Expected: {t_a_c:?}");

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_c,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_branch_navigation() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform {
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
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at t=0, x=1m without rotation
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

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform {
            translation: Vector3 {
                x: 2.,
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
            child: "d".into(),
        };

        registry.add_transform(t_a_b);
        registry.add_transform(t_b_c);
        registry.add_transform(t_b_d);

        let result = registry.get_transform("c", "d", t);

        assert!(result.is_ok());

        let t_c_d = result.unwrap();
        let t_c_d_expected = Transform {
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
            parent: "c".into(),
            child: "d".into(),
        };

        assert_eq!(t_c_d, t_c_d_expected);

        debug!("{t_c_d:?}");
    }

    #[test]
    fn basic_common_parent_elimination() {
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform {
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
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        };

        // Child frame C at t=0, x=1m without rotation
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

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform {
            translation: Vector3 {
                x: 2.,
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
            child: "d".into(),
        };

        registry.add_transform(t_a_b);
        registry.add_transform(t_b_c);
        registry.add_transform(t_b_d);

        let from_chain = Registry::get_transform_chain("d", "a", t, &registry.data);
        let mut to_chain = Registry::get_transform_chain("c", "a", t, &registry.data);

        if let Ok(chain) = to_chain.as_mut() {
            Registry::reverse_and_invert_transforms(chain).expect("Failed to reverse and invert");
        }

        assert!(from_chain.is_ok());
        assert!(to_chain.is_ok());

        let mut from = from_chain.unwrap();
        let mut to = to_chain.unwrap();

        Registry::truncate_at_common_parent(&mut from, &mut to);
        let result = Registry::combine_transforms(from, to);

        debug!("{result:?}");
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
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t1 = Timestamp::zero();
        #[cfg(not(feature = "std"))]
        let t2 = Timestamp { t: 1_000_000_000 };

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t1 = Timestamp::now();
        #[cfg(feature = "std")]
        let t2 = (t1 + Duration::from_secs(1)).unwrap();

        // fixed -> a at t1: a is at x=1
        registry.add_transform(Transform {
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
            timestamp: t1,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // fixed -> a at t2: a has moved to x=2
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 2.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t2,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // a -> b at t1: b is at y=1 relative to a
        registry.add_transform(Transform {
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
            timestamp: t1,
            parent: "a".into(),
            child: "b".into(),
        });

        // a -> b at t2: b is at y=2 relative to a
        registry.add_transform(Transform {
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
            timestamp: t2,
            parent: "a".into(),
            child: "b".into(),
        });

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        debug!("Time travel result: {tf:?}");

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
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        registry.add_transform(Transform {
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
            parent: "fixed".into(),
            child: "a".into(),
        });

        registry.add_transform(Transform {
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
            timestamp: t,
            parent: "a".into(),
            child: "b".into(),
        });

        let regular = registry.get_transform("a", "b", t);
        let time_travel = registry.get_transform_at("a", t, "b", t, "fixed");

        assert!(regular.is_ok());
        assert!(time_travel.is_ok());

        let regular_tf = regular.unwrap();
        let time_travel_tf = time_travel.unwrap();

        assert_eq!(regular_tf.translation, time_travel_tf.translation);
        assert_eq!(regular_tf.rotation, time_travel_tf.rotation);
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
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t1 = Timestamp::zero();
        #[cfg(not(feature = "std"))]
        let t2 = Timestamp { t: 1_000_000_000 };

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t1 = Timestamp::now();
        #[cfg(feature = "std")]
        let t2 = (t1 + Duration::from_secs(1)).unwrap();

        let theta = core::f64::consts::PI / 2.0;

        // fixed -> a at t1: at (1,0,0), no rotation
        registry.add_transform(Transform {
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
            timestamp: t1,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // fixed -> a at t2: at origin, rotated 90° CCW around z
        registry.add_transform(Transform {
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
            timestamp: t2,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // a -> b at t1: b is at (0.5, 0, 0) relative to a
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 0.5,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t1,
            parent: "a".into(),
            child: "b".into(),
        });

        // a -> b at t2: b still at (0.5, 0, 0) relative to a
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 0.5,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t2,
            parent: "a".into(),
            child: "b".into(),
        });

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

        debug!("Time travel with rotation result: {tf:?}");

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
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t1 = Timestamp::zero();
        #[cfg(not(feature = "std"))]
        let t2 = Timestamp { t: 1_000_000_000 };

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t1 = Timestamp::now();
        #[cfg(feature = "std")]
        let t2 = (t1 + Duration::from_secs(1)).unwrap();

        // fixed -> a at t1
        registry.add_transform(Transform {
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
            timestamp: t1,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // fixed -> a at t2
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 2.,
                y: 0.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t2,
            parent: "fixed".into(),
            child: "a".into(),
        });

        // fixed -> b at t1
        registry.add_transform(Transform {
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
            timestamp: t1,
            parent: "fixed".into(),
            child: "b".into(),
        });

        // fixed -> b at t2
        registry.add_transform(Transform {
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
            timestamp: t2,
            parent: "fixed".into(),
            child: "b".into(),
        });

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

        debug!("Time travel branching tree result: {tf:?}");

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
    fn get_transform_for_success_with_point() {
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        registry.add_transform(Transform {
            translation: Vector3::new(2.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: t,
            parent: "map".into(),
            child: "camera".into(),
        });

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
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

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
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        let point = Point {
            position: Vector3::new(0.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: t,
            frame: "camera".into(),
        };

        let result = registry.get_transform_for(&point, "map");

        assert!(matches!(result, Err(TransformError::NotFound(_, _))));
    }
}
