#[cfg(test)]
mod tests {
    use env_logger;
    use log::debug;
    use transforms::types::{Point, Quaternion, Registry, Timestamp, Transform, Vector3};

    #[test]
    fn test_transforms() {
        env_logger::init();
        let mut registry = Registry::new(u128::MAX);

        // Child frame B at x=1m without rotation
        let t_a_b_0 = Transform {
            transform: Point {
                position: Vector3 {
                    x: 1.,
                    y: 0.,
                    z: 0.,
                },
                orientation: Quaternion {
                    w: 1.,
                    x: 0.,
                    y: 0.,
                    z: 0.,
                },
                timestamp: Timestamp { nanoseconds: 0 },
            },
            parent: "a",
            child: "b",
        };

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = std::f64::consts::PI / 2.0;
        let t_a_b_1 = Transform {
            transform: Point {
                position: Vector3 {
                    x: 0.,
                    y: 1.,
                    z: 0.,
                },
                orientation: Quaternion {
                    w: (theta / 2.0).cos(),
                    x: 0.,
                    y: 0.,
                    z: (theta / 2.0).sin(),
                },
                timestamp: Timestamp {
                    nanoseconds: 1_000_000,
                },
            },
            parent: "a",
            child: "b",
        };

        registry.add_transform(t_a_b_0);
        registry.add_transform(t_a_b_1);

        let middle_timestamp = Timestamp {
            nanoseconds: (t_a_b_0.transform.timestamp.nanoseconds
                + t_a_b_1.transform.timestamp.nanoseconds)
                / 2,
        };

        let t_a_b_2 = Transform {
            transform: Point {
                position: (t_a_b_0.transform.position + t_a_b_1.transform.position) / 2.0,
                orientation: (t_a_b_0
                    .transform
                    .orientation
                    .slerp(t_a_b_1.transform.orientation, 0.5)),
                timestamp: middle_timestamp,
            },
            parent: "a",
            child: "b",
        };

        let r = registry.get_transform("a", "b", middle_timestamp);

        debug!("{:?}", r);

        assert!(r.is_some(), "Registry returned None, expected Some");
        assert_eq!(
            r.unwrap(),
            t_a_b_2,
            "Registry returned a transform that is different"
        );
    }
}
