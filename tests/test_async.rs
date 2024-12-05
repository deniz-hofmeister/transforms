#[cfg(all(test, feature = "async"))]
use {
    log::debug,
    std::time::Duration,
    transforms::geometry::{Quaternion, Transform, Vector3},
    transforms::time::Timestamp,
    transforms::Registry,
};

#[cfg(all(test, feature = "async"))]
#[tokio::test]
async fn test_async_matching_tree() {
    let _ = env_logger::try_init();
    let registry = Registry::new(Duration::from_secs(60));
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
        timestamp: (t + Duration::from_millis(1000)).unwrap(),
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
        timestamp: (t + Duration::from_millis(500)).unwrap(),
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
        timestamp: (t + Duration::from_millis(1500)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone()).await.unwrap();
    registry.add_transform(t_a_b_1.clone()).await.unwrap();
    registry.add_transform(t_b_c_0.clone()).await.unwrap();
    registry.add_transform(t_b_c_1.clone()).await.unwrap();

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
        parent: "a".into(),
        child: "c".into(),
    };

    let r = registry.get_transform("a", "c", middle_timestamp).await;

    debug!("Result: {:?}", r);
    debug!("Expected: {:?}", t_a_c);

    assert!(r.is_ok(), "Registry returned Error, expected Ok");
    assert_eq!(
        r.unwrap(),
        t_a_c,
        "Registry returned a transform that is different"
    );
}

#[cfg(all(test, feature = "async"))]
#[tokio::test]
async fn test_async_non_matching_tree() {
    let _ = env_logger::try_init();
    let registry = Registry::new(std::time::Duration::from_secs(60));
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
        timestamp: (t + Duration::from_secs(2)).unwrap(),
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
        timestamp: (t + Duration::from_secs(3)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone()).await.unwrap();
    registry.add_transform(t_a_b_1.clone()).await.unwrap();
    registry.add_transform(t_b_c_0.clone()).await.unwrap();
    registry.add_transform(t_b_c_1.clone()).await.unwrap();

    let r = registry.get_transform("a", "c", t).await;

    debug!("Result: {:?}", r);

    assert!(r.is_err(), "Registry returned Ok, expected Err");
}
