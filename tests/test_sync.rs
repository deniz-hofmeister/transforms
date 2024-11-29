#[cfg(not(feature = "async"))]
use {
    log::debug,
    std::time::Duration,
    transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3},
};

#[cfg(not(feature = "async"))]
#[test]
fn test_sync_matching_tree() {
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
        timestamp: (t + Duration::from_millis(1000)).unwrap(),
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
        timestamp: (t + Duration::from_millis(500)).unwrap(),
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
        timestamp: (t + Duration::from_millis(1500)).unwrap(),
        parent: "b".to_string(),
        child: "c".to_string(),
    };

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    let middle_timestamp = (t + Duration::from_millis(750)).unwrap();
    let t_a_c = Transform {
        translation: Vector3 {
            x: 1.75,
            y: 1.25,
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

#[cfg(not(feature = "async"))]
#[test]
fn test_sync_non_matching_tree() {
    let _ = env_logger::try_init();
    let mut registry = Registry::new(Duration::from_secs(1));
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
        timestamp: (t + Duration::from_secs(1)).unwrap(),
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
        timestamp: (t + Duration::from_secs(2)).unwrap(),
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
        timestamp: (t + Duration::from_secs(3)).unwrap(),
        parent: "b".to_string(),
        child: "c".to_string(),
    };

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    let r = registry.get_transform("a", "c", t);

    debug!("Result: {:?}", r);

    assert!(r.is_err(), "Registry returned Ok, expected Err");
}
