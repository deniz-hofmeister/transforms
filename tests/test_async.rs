#[cfg(all(test, feature = "async"))]
use log::debug;
#[cfg(all(test, feature = "async"))]
use transforms::types::{Duration, Quaternion, Registry, Timestamp, Transform, Vector3};

#[cfg(all(test, feature = "async"))]
#[tokio::test]
async fn test_async_matching_tree() {
    let _ = env_logger::try_init();
    let registry = Registry::new(Duration::try_from(10.0).unwrap());
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
        timestamp: (t + Duration::try_from(0.5).unwrap()).unwrap(),
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
        timestamp: (t + Duration::try_from(1.5).unwrap()).unwrap(),
        parent: "b".to_string(),
        child: "c".to_string(),
    };

    registry.add_transform(t_a_b_0.clone()).await.unwrap();
    registry.add_transform(t_a_b_1.clone()).await.unwrap();
    registry.add_transform(t_b_c_0.clone()).await.unwrap();
    registry.add_transform(t_b_c_1.clone()).await.unwrap();

    let middle_timestamp = (t + Duration::try_from(0.75).unwrap()).unwrap();
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
    let registry = Registry::new(Duration::try_from(1.0).unwrap());
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
        timestamp: (t + Duration::try_from(2.0).unwrap()).unwrap(),
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
        timestamp: (t + Duration::try_from(3.0).unwrap()).unwrap(),
        parent: "b".to_string(),
        child: "c".to_string(),
    };

    registry.add_transform(t_a_b_0.clone()).await.unwrap();
    registry.add_transform(t_a_b_1.clone()).await.unwrap();
    registry.add_transform(t_b_c_0.clone()).await.unwrap();
    registry.add_transform(t_b_c_1.clone()).await.unwrap();

    let r = registry.get_transform("a", "c", t).await;

    debug!("Result: {:?}", r);

    assert!(r.is_err(), "Registry returned Ok, expected Err");
}
