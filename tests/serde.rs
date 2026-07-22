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

#[cfg(feature = "std")]
#[test]
fn transform_over_system_time_roundtrips() {
    use core::time::Duration;
    use std::time::UNIX_EPOCH;

    let transform: Transform<std::time::SystemTime> = Transform {
        translation: Vector3::new(1.5, -2.25, 3.125),
        rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        timestamp: UNIX_EPOCH
            .checked_add(Duration::from_secs(1_753_142_400))
            .unwrap(),
        parent: "map".into(),
        child: "base_link".into(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let back: Transform<std::time::SystemTime> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, transform);
}

/// Golden-bytes pin for non-self-describing formats: in postcard (and
/// bincode) the struct field ORDER is the wire contract. Reordering any
/// serde-derived field of Transform, Vector3, Quaternion, or Timestamp
/// compiles fine and passes every JSON test, but silently corrupts every
/// postcard/bincode stream — this test is what catches it.
#[test]
fn transform_postcard_bytes_are_frozen() {
    let transform: Transform = Transform {
        translation: Vector3::new(1.5, -2.25, 3.125),
        rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        timestamp: Timestamp::from_nanos(1_753_142_400_000_000_000),
        parent: "map".into(),
        child: "base_link".into(),
    };

    let bytes = postcard::to_allocvec(&transform).unwrap();

    // translation.x/y/z and rotation.w/x/y/z as fixed 8-byte LE f64, the
    // u128 timestamp as a LEB128 varint, then length-prefixed frame names.
    let expected: &[u8] = &[
        0, 0, 0, 0, 0, 0, 248, 63, // 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        128, 128, 180, 197, 150, 183, 154, 170, 24, // timestamp varint
        3, 109, 97, 112, // "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
    ];
    assert_eq!(bytes, expected, "postcard wire format changed");

    let back: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, transform);
}
