#[cfg(test)]
mod tests {
    use log::debug;
    use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};

    #[test]
    fn test_basic_chain_linear() {
        let _ = env_logger::try_init();
        let mut registry = Registry::new(f64::MAX);
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
}
