#[cfg(test)]
mod tests {
    use env_logger;
    use log::debug;
    use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};

    #[test]
    fn test_transforms() {
        env_logger::init();
        let mut registry = Registry::new(u128::MAX);

        // Child frame B at x=1m without rotation
        let t_a_b_0 = Transform::new(
            Vector3 {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            Timestamp { nanoseconds: 0 },
            "a",
            "b",
        );

        // Child frame C at y=1m with 90 degrees rotation around +Z
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
            timestamp: Timestamp {
                nanoseconds: 1_000_000,
            },
            frame: "a".to_string(),
            parent: "b".to_string(),
        };

        registry.add_transform(t_a_b_0.clone());
        registry.add_transform(t_a_b_1.clone());

        let middle_timestamp = Timestamp {
            nanoseconds: (t_a_b_0.timestamp.nanoseconds + t_a_b_1.timestamp.nanoseconds) / 2,
        };

        let t_a_b_2 = Transform {
            translation: (t_a_b_0.translation + t_a_b_1.translation) / 2.0,
            rotation: (t_a_b_0.rotation.slerp(t_a_b_1.rotation, 0.5)),
            timestamp: middle_timestamp,
            frame: "a".to_string(),
            parent: "b".to_string(),
        };

        let r = registry.get_transform("a", "b", middle_timestamp);

        debug!("{:?}", r);

        assert!(r.is_some(), "Registry returned None, expected Some");
        assert_eq!(
            r.unwrap(),
            &t_a_b_2,
            "Registry returned a transform that is different"
        );
    }
}
