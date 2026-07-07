use std::time::Duration;
use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
};

#[test]
fn test_matching_tree() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(1)).unwrap(),
        parent: "a".into(),
        child: "b".into(),
    };
    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_millis(500)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform {
        translation: Vector3::new(0.0, 2.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_millis(1500)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    let middle_timestamp = (t + Duration::from_millis(750)).unwrap();
    let t_a_c = Transform {
        translation: Vector3::new(1.75, 1.25, 0.0),
        rotation: Quaternion::identity(),
        timestamp: middle_timestamp,
        parent: "a".into(),
        child: "c".into(),
    };

    let r = registry.get_transform("a", "c", middle_timestamp);

    assert!(r.is_ok(), "expected Ok, got {r:?}");
    assert_eq!(r.unwrap(), t_a_c);
}

#[test]
fn test_non_matching_tree() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(1)).unwrap(),
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(2)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform {
        translation: Vector3::new(0.0, 2.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(3)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    let r = registry.get_transform("a", "c", t);

    assert!(r.is_err(), "expected Err, got {r:?}");
}
