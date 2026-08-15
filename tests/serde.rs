#![cfg(feature = "serde")]
//! JSON roundtrip tests for the optional serde support.

use transforms::{
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
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
        timestamp: Stamp::At(Timestamp::from_nanos(1_000_000_000)),
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
    // `Stamp` serializes as an optional timestamp: `Stamp::At(t)` is `t`
    // itself, so a dynamic transform's wire shape is identical to a bare
    // timestamp field.
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
        timestamp: Stamp::At(Timestamp::from_nanos(1_000_000_000)),
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

#[test]
fn static_transform_serializes_timestamp_as_null() {
    // `Stamp::Static` is `null` on the wire — self-describing, with no
    // reserved magic timestamp value.
    let json = r#"{
        "translation": { "x": 0.1, "y": 0.0, "z": 0.5 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "timestamp": null,
        "parent": "base",
        "child": "camera"
    }"#;

    let deserialized: Transform<Timestamp> = serde_json::from_str(json).unwrap();

    let expected = Transform::static_between(
        "base",
        "camera",
        Vector3::new(0.1, 0.0, 0.5),
        Quaternion::identity(),
    );
    assert_eq!(deserialized, expected);

    let value: serde_json::Value = serde_json::to_value(&expected).unwrap();
    assert!(
        value.as_object().unwrap()["timestamp"].is_null(),
        "Stamp::Static must serialize as null"
    );
}

#[test]
fn transform_missing_timestamp_field_is_an_error() {
    // `Stamp` uses an optional encoding, whose serde missing-field fallback
    // would silently yield `Stamp::Static`. The `deserialize_with` detour
    // on `Transform.timestamp` makes an absent field a hard error instead:
    // a producer that drops the timestamp key must not mint an eternal
    // static transform.
    let json = r#"{
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "parent": "map",
        "child": "base"
    }"#;

    let result = serde_json::from_str::<Transform<Timestamp>>(json);
    assert!(
        result.is_err(),
        "a missing timestamp field must be rejected, got {result:?}"
    );
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains("timestamp"),
        "the error must name the missing field, got: {message}"
    );
}

#[cfg(feature = "std")]
#[test]
fn transform_over_system_time_roundtrips() {
    use core::time::Duration;
    use std::time::UNIX_EPOCH;

    let transform: Transform<std::time::SystemTime> = Transform {
        translation: Vector3::new(1.5, -2.25, 3.125),
        rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        timestamp: Stamp::At(
            UNIX_EPOCH
                .checked_add(Duration::from_secs(1_753_142_400))
                .unwrap(),
        ),
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
        timestamp: Stamp::At(Timestamp::from_nanos(1_753_142_400_000_000_000)),
        parent: "map".into(),
        child: "base_link".into(),
    };

    let bytes = postcard::to_allocvec(&transform).unwrap();

    // translation.x/y/z and rotation.w/x/y/z as fixed 8-byte LE f64, the
    // stamp as an Option (1-byte Some tag, then the u64 timestamp as a
    // LEB128 varint), then length-prefixed frame names.
    let expected: &[u8] = &[
        0, 0, 0, 0, 0, 0, 248, 63, // 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        1, // Stamp::At = Some
        128, 128, 180, 197, 150, 183, 154, 170, 24, // timestamp varint
        3, 109, 97, 112, // "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
    ];
    assert_eq!(bytes, expected, "postcard wire format changed");

    let back: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, transform);
}

/// The same field-order pin for `Point`: position, orientation, the bare
/// timestamp varint (no `Option` tag — `Point.timestamp` is a plain
/// `Timestamp`), then the frame name.
#[test]
fn point_postcard_bytes_are_frozen() {
    let point: Point = Point {
        position: Vector3::new(1.5, -2.25, 3.125),
        orientation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        timestamp: Timestamp::from_nanos(1_753_142_400_000_000_000),
        frame: "camera".into(),
    };

    let bytes = postcard::to_allocvec(&point).unwrap();

    let expected: &[u8] = &[
        0, 0, 0, 0, 0, 0, 248, 63, // 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        128, 128, 180, 197, 150, 183, 154, 170, 24, // timestamp varint
        6, 99, 97, 109, 101, 114, 97, // "camera"
    ];
    assert_eq!(bytes, expected, "postcard wire format changed");

    let back: Point = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, point);
}

/// The static arm of the same pin: `Stamp::Static` is the 1-byte `None`
/// tag — no reserved timestamp value appears on the wire.
#[test]
fn static_transform_postcard_bytes_are_frozen() {
    let transform: Transform = Transform::static_between(
        "map",
        "base_link",
        Vector3::new(1.5, -2.25, 3.125),
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
    );

    let bytes = postcard::to_allocvec(&transform).unwrap();

    let expected: &[u8] = &[
        0, 0, 0, 0, 0, 0, 248, 63, // 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // 0.0
        0, // Stamp::Static = None
        3, 109, 97, 112, // "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
    ];
    assert_eq!(bytes, expected, "postcard wire format changed");

    let back: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, transform);
}
