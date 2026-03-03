#[cfg(test)]
mod registry_tests {
    use crate::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry,
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
    fn time_travel_conveyor_belt() {
        // This test simulates a robot arm on a moving base:
        // - The arm detects an object at t1 (with the base at one position)
        // - We want to transform that detection into the map frame at t2 (when the base has moved)
        // - The fixed frame is "map" (stationary reference)
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t1 = Timestamp::zero();
        #[cfg(not(feature = "std"))]
        let t2 = Timestamp { t: 1_000_000_000 }; // 1 second later

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t1 = Timestamp::now();
        #[cfg(feature = "std")]
        let t2 = (t1 + Duration::from_secs(1)).unwrap();

        // Map -> Base at t1: robot base is at x=1
        let map_to_base_t1 = Transform {
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
            parent: "map".into(),
            child: "base".into(),
        };

        // Map -> Base at t2: robot has moved to x=2
        let map_to_base_t2 = Transform {
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
            parent: "map".into(),
            child: "base".into(),
        };

        // Base -> Camera at t1: camera is at y=0.5 relative to base
        let base_to_camera_t1 = Transform {
            translation: Vector3 {
                x: 0.,
                y: 0.5,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t1,
            parent: "base".into(),
            child: "camera".into(),
        };

        // Base -> Camera at t2: camera still at same position relative to base
        let base_to_camera_t2 = Transform {
            translation: Vector3 {
                x: 0.,
                y: 0.5,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t2,
            parent: "base".into(),
            child: "camera".into(),
        };

        registry.add_transform(map_to_base_t1);
        registry.add_transform(map_to_base_t2);
        registry.add_transform(base_to_camera_t1);
        registry.add_transform(base_to_camera_t2);

        // At t1, camera should be at map position (1, 0.5, 0)
        let map_to_camera_t1 = registry.get_transform("map", "camera", t1);
        assert!(map_to_camera_t1.is_ok());
        let tf = map_to_camera_t1.unwrap();
        debug!("Camera at t1: {:?}", tf);
        assert!(
            (tf.translation.x - 1.0).abs() < f64::EPSILON,
            "Expected x=1.0, got {}",
            tf.translation.x
        );
        assert!(
            (tf.translation.y - 0.5).abs() < f64::EPSILON,
            "Expected y=0.5, got {}",
            tf.translation.y
        );

        // At t2, camera should be at map position (2, 0.5, 0) - robot moved
        let map_to_camera_t2 = registry.get_transform("map", "camera", t2);
        assert!(map_to_camera_t2.is_ok());
        let tf = map_to_camera_t2.unwrap();
        debug!("Camera at t2: {:?}", tf);
        assert!(
            (tf.translation.x - 2.0).abs() < f64::EPSILON,
            "Expected x=2.0, got {}",
            tf.translation.x
        );

        // Time travel: express camera frame at t1 in the map frame at t2
        // Since "map" is fixed and doesn't move between t1 and t2,
        // the camera's position in map at t1 (which was 1, 0.5, 0) remains (1, 0.5, 0)
        // when expressed in map at t2.
        let result = registry.get_transform_at_times(
            "map",    // target_frame
            t2,       // target_time
            "camera", // source_frame
            t1,       // source_time
            "map",    // fixed_frame
        );

        assert!(
            result.is_ok(),
            "get_transform_at_times failed: {:?}",
            result
        );
        let tf = result.unwrap();

        debug!("Time travel result: {:?}", tf);

        // The camera was at (1, 0.5, 0) in map at t1.
        // Map doesn't move, so expressing "camera at t1" in "map at t2" gives (1, 0.5, 0)
        assert!(
            (tf.translation.x - 1.0).abs() < f64::EPSILON,
            "Expected x=1.0, got {}",
            tf.translation.x
        );
        assert!(
            (tf.translation.y - 0.5).abs() < f64::EPSILON,
            "Expected y=0.5, got {}",
            tf.translation.y
        );
    }

    #[test]
    fn time_travel_same_time() {
        // When source_time == target_time, should behave like get_transform
        let _ = env_logger::try_init();

        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(not(feature = "std"))]
        let t = Timestamp::zero();

        #[cfg(feature = "std")]
        let mut registry = Registry::new(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = Timestamp::now();

        let world_to_robot = Transform {
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
            timestamp: t,
            parent: "world".into(),
            child: "robot".into(),
        };

        registry.add_transform(world_to_robot.clone());

        // Time travel with same time should give same result as regular get_transform
        let regular = registry.get_transform("world", "robot", t);
        let time_travel = registry.get_transform_at_times("world", t, "robot", t, "world");

        assert!(regular.is_ok());
        assert!(time_travel.is_ok());

        let regular_tf = regular.unwrap();
        let time_travel_tf = time_travel.unwrap();

        assert_eq!(regular_tf.translation, time_travel_tf.translation);
        assert_eq!(regular_tf.rotation, time_travel_tf.rotation);
    }

    #[test]
    fn time_travel_with_rotation() {
        // Test time travel when frames have different rotations
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

        // World -> Sensor at t1: sensor is at x=1, no rotation
        let world_to_sensor_t1 = Transform {
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
            parent: "world".into(),
            child: "sensor".into(),
        };

        // World -> Sensor at t2: sensor has moved and rotated 90 degrees around z
        let theta = core::f64::consts::PI / 2.0;
        let world_to_sensor_t2 = Transform {
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
            timestamp: t2,
            parent: "world".into(),
            child: "sensor".into(),
        };

        registry.add_transform(world_to_sensor_t1);
        registry.add_transform(world_to_sensor_t2);

        // Time travel: sensor position at t1 relative to world at t2
        let result = registry.get_transform_at_times(
            "world",  // target_frame
            t2,       // target_time
            "sensor", // source_frame
            t1,       // source_time
            "world",  // fixed_frame
        );

        assert!(
            result.is_ok(),
            "Time travel with rotation failed: {:?}",
            result
        );
        let tf = result.unwrap();

        debug!("Time travel with rotation result: {:?}", tf);

        // The sensor was at (1, 0, 0) at t1
        // At t2, the world frame hasn't moved (it's fixed), so the result
        // should still be (1, 0, 0) when expressed in world frame
        assert!(
            (tf.translation.x - 1.0).abs() < 1e-10,
            "Expected x=1.0, got {}",
            tf.translation.x
        );
        assert!(
            tf.translation.y.abs() < 1e-10,
            "Expected y=0.0, got {}",
            tf.translation.y
        );
    }

}
