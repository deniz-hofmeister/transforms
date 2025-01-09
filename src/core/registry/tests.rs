#[cfg(not(feature = "async"))]
#[cfg(test)]
mod registry_tests {
    use crate::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry,
    };
    use log::debug;
    use std::time::Duration;

    mod sync_tests {
        use super::*;

        #[test]
        fn basic_chain_linear() {
            let _ = env_logger::try_init();
            let mut registry = Registry::new(Duration::from_secs(10));
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
                parent: "a".into(),
                child: "c".into(),
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
        fn basic_chain_linear_reverse() {
            let _ = env_logger::try_init();
            let mut registry = Registry::new(Duration::from_secs(10));
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

            registry.add_transform(t_a_b.clone()).unwrap();
            registry.add_transform(t_b_c.clone()).unwrap();

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

            debug!("Result: {:?}", r);
            debug!("Desired: {:?}", t_c_a);

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
            let mut registry = Registry::new(Duration::from_secs(10));
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
                parent: "a".into(),
                child: "d".into(),
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
            let mut registry = Registry::new(Duration::from_secs(10));

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
                timestamp: Timestamp::now(),
                parent: "a".into(),
                child: "c".into(),
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
            let mut registry = Registry::new(Duration::from_secs(10));
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

            registry.add_transform(t_a_b_0.clone()).unwrap();
            registry.add_transform(t_a_b_1.clone()).unwrap();

            let middle_timestamp = Timestamp {
                t: (t_a_b_0.timestamp.t + t_a_b_1.timestamp.t) / 2,
            };

            let t_a_b_2 = Transform {
                translation: (t_a_b_0.translation + t_a_b_1.translation) / 2.0,
                rotation: (t_a_b_0.rotation.slerp(t_a_b_1.rotation, 0.5)),
                timestamp: middle_timestamp,
                parent: "a".into(),
                child: "b".into(),
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
            let mut registry = Registry::new(Duration::from_secs(10));
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

            registry.add_transform(t_a_b_0.clone()).unwrap();
            registry.add_transform(t_a_b_1.clone()).unwrap();
            registry.add_transform(t_b_c_0.clone()).unwrap();
            registry.add_transform(t_b_c_1.clone()).unwrap();

            let middle_timestamp = Timestamp {
                t: (t_a_b_0.timestamp.t + t_a_b_1.timestamp.t) / 2,
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
        fn basic_branch_navigation() {
            let _ = env_logger::try_init();
            let mut registry = Registry::new(Duration::from_secs(10));
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

            registry.add_transform(t_a_b).unwrap();
            registry.add_transform(t_b_c).unwrap();
            registry.add_transform(t_b_d).unwrap();

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

            debug!("{:?}", t_c_d);
        }

        #[test]
        fn basic_common_parent_elimination() {
            let _ = env_logger::try_init();
            let mut registry = Registry::new(Duration::from_secs(10));
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

            registry.add_transform(t_a_b).unwrap();
            registry.add_transform(t_b_c).unwrap();
            registry.add_transform(t_b_d).unwrap();

            let from_chain = Registry::get_transform_chain("d", "a", t, &registry.data);
            let mut to_chain = Registry::get_transform_chain("c", "a", t, &registry.data);

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
}
