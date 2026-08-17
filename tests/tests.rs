use std::time::Duration;
use transforms::{
    Registry,
    errors::{BufferError, TransformError},
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

#[test]
fn test_matching_tree() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(t),
    )
    .unwrap();

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform::new(
        "a",
        "b",
        Vector3::new(2.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_secs(1)).unwrap()),
    )
    .unwrap();
    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 1.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_millis(500)).unwrap()),
    )
    .unwrap();

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 2.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_millis(1500)).unwrap()),
    )
    .unwrap();

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    let middle_timestamp = (t + Duration::from_millis(750)).unwrap();
    let t_a_c = Transform::new(
        "a",
        "c",
        Vector3::new(1.75, 1.25, 0.0),
        Quaternion::identity(),
        Stamp::At(middle_timestamp),
    )
    .unwrap();

    let r = registry.get_transform("a", "c", middle_timestamp);

    assert!(r.is_ok(), "expected Ok, got {r:?}");
    assert_eq!(r.unwrap(), t_a_c);
}

#[test]
fn test_non_matching_tree() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(t),
    )
    .unwrap();

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform::new(
        "a",
        "b",
        Vector3::new(2.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_secs(1)).unwrap()),
    )
    .unwrap();

    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 1.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_secs(2)).unwrap()),
    )
    .unwrap();

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 2.0, 0.0),
        Quaternion::identity(),
        Stamp::At((t + Duration::from_secs(3)).unwrap()),
    )
    .unwrap();

    registry.add_transform(t_a_b_0.clone()).unwrap();
    registry.add_transform(t_a_b_1.clone()).unwrap();
    registry.add_transform(t_b_c_0.clone()).unwrap();
    registry.add_transform(t_b_c_1.clone()).unwrap();

    // The b->c buffer covers [t+2s, t+3s]; querying at t stops the walk at
    // frame "c" with the exact covered range in the payload.
    let r = registry.get_transform("a", "c", t);

    match r {
        Err(TransformError::NotFoundAt {
            target_frame,
            source_frame,
            frame,
            source,
        }) => {
            assert_eq!(target_frame, "a");
            assert_eq!(source_frame, "c");
            assert_eq!(frame, "c");
            match *source {
                BufferError::TransformError(TransformError::TimestampOutOfRange {
                    requested,
                    start,
                    end,
                }) => {
                    assert_eq!(requested, 1.0);
                    assert_eq!(start, 3.0);
                    assert_eq!(end, 4.0);
                }
                other => panic!("expected TimestampOutOfRange, got {other:?}"),
            }
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }
}
