#[cfg(test)]
mod registry_tests {
    use crate::types::{Duration, Quaternion, Registry, Timestamp, Transform, Vector3};
    use log::debug;

    #[test]
    fn basic_chain_linear() {
        let _ = env_logger::try_init();
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());
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
            parent: "a".to_string(),
            child: "b".to_string(),
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
            parent: "b".to_string(),
            child: "c".to_string(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();

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
            parent: "a".to_string(),
            child: "c".to_string(),
        };

        let r = registry.get_transform("a", "c", t_a_b.timestamp);

        debug!("Result: {:?}", r);
        debug!("Desired: {:?}", t_a_c);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_c,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_chain_rotation() {
        let _ = env_logger::try_init();
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());
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
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        // Child frame C at +90 degrees
        let theta = std::f64::consts::PI / 2.0;
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
            parent: "b".to_string(),
            child: "c".to_string(),
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
            parent: "c".to_string(),
            child: "d".to_string(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();
        registry.add_transform(t_c_d.clone()).unwrap();

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
            parent: "a".to_string(),
            child: "d".to_string(),
        };
        let r = registry.get_transform("a", "d", t_a_b.timestamp);

        debug!("Result: {:?}", r);
        debug!("Desired: {:?}", t_a_d);

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
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());

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
            timestamp: Timestamp::now(),
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = std::f64::consts::PI / 2.0;
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
            timestamp: Timestamp::now(),
            parent: "a".to_string(),
            child: "c".to_string(),
        };

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_a_c.clone()).unwrap();

        let r = registry.get_transform("a", "b", t_a_b.timestamp);

        debug!("{:?}", r);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_b,
            "Registry returned a transform that is different"
        );

        let r = registry.get_transform("a", "c", t_a_c.timestamp);

        debug!("{:?}", r);

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
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());
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
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        // Child frame B at y=1m with 90 degrees rotation around +Z
        let theta = std::f64::consts::PI / 2.0;
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
            timestamp: (t + Duration::try_from(1.0).unwrap()).unwrap(),
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();

        let middle_timestamp = Timestamp {
            nanoseconds: (t_a_b_0.timestamp.nanoseconds + t_a_b_1.timestamp.nanoseconds) / 2,
        };

        let t_a_b_2 = Transform {
            translation: (t_a_b_0.translation + t_a_b_1.translation) / 2.0,
            rotation: (t_a_b_0.rotation.slerp(t_a_b_1.rotation, 0.5)),
            timestamp: middle_timestamp,
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        let r = registry.get_transform("a", "b", middle_timestamp);

        debug!("Result: {:?}", r);
        debug!("Expected: {:?}", t_a_b_2);

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
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());
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
            parent: "a".to_string(),
            child: "b".to_string(),
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
            timestamp: (t + Duration::try_from(1.0).unwrap()).unwrap(),
            parent: "a".to_string(),
            child: "b".to_string(),
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
            parent: "b".to_string(),
            child: "c".to_string(),
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
            timestamp: (t + Duration::try_from(1.0).unwrap()).unwrap(),
            parent: "b".to_string(),
            child: "c".to_string(),
        };

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();
        registry.add_transform(t_b_c_0.clone()).unwrap();
        registry.add_transform(t_b_c_1.clone()).unwrap();

        let middle_timestamp = Timestamp {
            nanoseconds: (t_a_b_0.timestamp.nanoseconds + t_a_b_1.timestamp.nanoseconds) / 2,
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
            parent: "a".to_string(),
            child: "c".to_string(),
        };

        let r = registry.get_transform("a", "c", middle_timestamp);

        debug!("Result: {:?}", r);
        debug!("Expected: {:?}", t_a_c);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_eq!(
            r.unwrap(),
            t_a_c,
            "Registry returned a transform that is different"
        );
    }

    #[test]
    fn basic_common_parent_elimination() {
        let _ = env_logger::try_init();
        let mut registry = Registry::new(Duration::try_from(1.0).unwrap());
        let t = Timestamp::now();

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
            parent: "b".to_string(),
            child: "c".to_string(),
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
            parent: "b".to_string(),
            child: "d".to_string(),
        };

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
            parent: "a".to_string(),
            child: "b".to_string(),
        };

        registry.add_transform(t_a_b).unwrap();
        registry.add_transform(t_b_c).unwrap();
        registry.add_transform(t_b_d).unwrap();

        let from_chain = registry.get_transform_chain("c", "d", t);
        let mut to_chain = registry.get_transform_chain("d", "c", t);

        if let Ok(chain) = to_chain.as_mut() {
            Registry::reverse_and_invert_transforms(chain).unwrap();
        }

        assert!(from_chain.is_ok());
        assert!(to_chain.is_ok());

        let mut from = from_chain.unwrap();
        let mut to = to_chain.unwrap();

        Registry::truncate_at_common_parent(&mut from, &mut to);
        let result = Registry::combine_transforms(from, to);

        debug!("{:?}", result);
    }
}
