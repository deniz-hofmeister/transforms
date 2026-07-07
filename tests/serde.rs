#![cfg(feature = "serde")]
//! JSON roundtrip tests for the optional serde support.

use transforms::{
    geometry::{Point, Quaternion, Transform, Vector3},
    time::Timestamp,
};

#[test]
fn vector3_json_roundtrip_is_exact() {
    let vector = Vector3::new(1.5, -2.25, 3.125);

    let json = serde_json::to_string(&vector).unwrap();
    let deserialized: Vector3 = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, vector);
}

#[test]
fn quaternion_json_roundtrip_is_exact() {
    let quaternion = Quaternion::new(0.5, 0.5, -0.5, 0.5);

    let json = serde_json::to_string(&quaternion).unwrap();
    let deserialized: Quaternion = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, quaternion);
}

#[test]
fn timestamp_json_roundtrip_is_exact() {
    let timestamp = Timestamp::from_nanos(1_234_567_890_123_456_789);

    let json = serde_json::to_string(&timestamp).unwrap();
    let deserialized: Timestamp = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, timestamp);
}

#[test]
fn transform_json_roundtrip_is_exact() {
    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: Timestamp::from_nanos(1_000_000_000),
        parent: "map".into(),
        child: "base".into(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: Transform<Timestamp> = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, transform);
}

#[test]
fn point_json_roundtrip_is_exact() {
    let point = Point {
        position: Vector3::new(-1.0, 0.5, 2.0),
        orientation: Quaternion::identity(),
        timestamp: Timestamp::from_nanos(2_000_000_000),
        frame: "camera".into(),
    };

    let json = serde_json::to_string(&point).unwrap();
    let deserialized: Point<Timestamp> = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, point);
}

#[test]
fn transform_deserializes_from_handwritten_json_with_struct_field_names() {
    let json = r#"{
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "timestamp": { "t": 1000000000 },
        "parent": "map",
        "child": "base"
    }"#;

    let deserialized: Transform<Timestamp> = serde_json::from_str(json).unwrap();

    let expected = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: Timestamp::from_nanos(1_000_000_000),
        parent: "map".into(),
        child: "base".into(),
    };
    assert_eq!(deserialized, expected);

    // Field names in the serialized form are the struct field names.
    let value: serde_json::Value = serde_json::to_value(&expected).unwrap();
    let object = value.as_object().unwrap();
    for field in ["translation", "rotation", "timestamp", "parent", "child"] {
        assert!(object.contains_key(field), "missing field {field}");
    }
}
