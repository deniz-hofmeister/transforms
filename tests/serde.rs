#![cfg(feature = "serde")]
//! The frozen wire schema of the optional serde support, in JSON and in
//! postcard, and the pin that the frozen 2.4 fixtures do not decode under
//! it. That pin is a regression test for those payloads, not a schema
//! detector: a 2.4 payload whose bytes form a valid transform decodes.
//!
//! `CrossTimeTransform` implements neither `Serialize` nor `Deserialize`,
//! so there is nothing to test for it at runtime.

use core::fmt::Debug;
use serde::{Serialize, de::DeserializeOwned};
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

/// 2025-07-22T00:00:00Z in nanoseconds, the instant of the 2.4 fixtures.
const NANOS: i64 = 1_753_142_400_000_000_000;

/// (10, 1, 2, 4) / 11 is a unit quaternion: 100 + 1 + 4 + 16 = 121. All four
/// components differ and none is zero, so any reordering changes the bytes.
/// Its squared norm is inside the stored band, so the components are kept
/// bit for bit.
fn general_rotation() -> Rotation {
    Rotation::from_wxyz(10.0 / 11.0, 1.0 / 11.0, 2.0 / 11.0, 4.0 / 11.0).unwrap()
}

/// A unit quaternion whose components are short decimals: 0.64 + 0.04 +
/// 0.16 + 0.16 = 1. The exact-string JSON tests use it because every JSON
/// parser reads such literals to the nearest `f64`.
fn short_rotation() -> Rotation {
    Rotation::from_wxyz(0.8, 0.2, -0.4, 0.4).unwrap()
}

fn transform(
    target: &str,
    source: &str,
    translation: Vector3,
    rotation: Rotation,
    stamp: Stamp,
) -> Transform {
    Transform::new(target, source, translation, rotation, stamp).unwrap()
}

/// The bit patterns of the seven numbers. `==` on `f64` equates `0.0` with
/// `-0.0`; this does not.
fn bits(transform: &Transform) -> [u64; 7] {
    let translation = transform.translation();
    let rotation = transform.rotation();
    [
        translation.x,
        translation.y,
        translation.z,
        rotation.w(),
        rotation.x(),
        rotation.y(),
        rotation.z(),
    ]
    .map(f64::to_bits)
}

/// Asserts the value encodes to exactly `json` and that `json` decodes back
/// to the value.
fn assert_json<T>(
    value: &T,
    json: &str,
) where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(serde_json::to_string(value).unwrap(), json);
    assert_eq!(&serde_json::from_str::<T>(json).unwrap(), value);
}

/// Asserts the value encodes to exactly `bytes`, that `bytes` decode back
/// to the value, and that the decoded value encodes to `bytes` again.
fn assert_frozen<T>(
    value: &T,
    bytes: &[u8],
) where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(
        postcard::to_allocvec(value).unwrap(),
        bytes,
        "postcard wire format changed"
    );
    let decoded: T = postcard::from_bytes(bytes).unwrap();
    assert_eq!(&decoded, value);
    assert_eq!(postcard::to_allocvec(&decoded).unwrap(), bytes);
}

/// Asserts `decode(encode(t)) == t`, down to the bits of every component,
/// in postcard, in JSON text, and through `serde_json::Value`.
///
/// The JSON text leg holds only with a parser that rounds every literal to
/// the nearest `f64`. serde_json does that under its `float_roundtrip`
/// feature; its default parser can be one ulp off on a 17-digit literal,
/// and a transform that carries such a number then decodes to a neighbour
/// of itself. The `Value` leg carries each `f64` as it is, so it isolates
/// the crate's own encode and decode from the text parser.
fn assert_fixed_point(original: &Transform) {
    let bytes = postcard::to_allocvec(original).unwrap();
    let from_postcard: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(&from_postcard, original);
    assert_eq!(bits(&from_postcard), bits(original));

    let value = serde_json::to_value(original).unwrap();
    let from_value: Transform = serde_json::from_value(value).unwrap();
    assert_eq!(&from_value, original);
    assert_eq!(bits(&from_value), bits(original));

    let json = serde_json::to_string(original).unwrap();
    let from_json: Transform = serde_json::from_str(&json).unwrap();
    assert_eq!(
        &from_json, original,
        "JSON text (needs serde_json/float_roundtrip): {json}"
    );
    assert_eq!(bits(&from_json), bits(original), "JSON text: {json}");
}

// ---------------------------------------------------------------------
// JSON shapes
// ---------------------------------------------------------------------

#[test]
fn vector3_json_is_an_object_of_x_y_z() {
    assert_json(
        &Vector3::new(1.5, -2.25, 3.125),
        r#"{"x":1.5,"y":-2.25,"z":3.125}"#,
    );
}

#[test]
fn rotation_json_is_an_object_of_w_x_y_z() {
    assert_json(&short_rotation(), r#"{"w":0.8,"x":0.2,"y":-0.4,"z":0.4}"#);
    assert_json(&Rotation::IDENTITY, r#"{"w":1.0,"x":0.0,"y":0.0,"z":0.0}"#);
}

#[test]
fn timestamp_json_is_a_bare_integer() {
    assert_json(&Timestamp::from_nanos(NANOS), "1753142400000000000");
    assert_json(&Timestamp::from_nanos(0), "0");
    assert_json(&Timestamp::from_nanos(-1), "-1");
    assert_json(&Timestamp::from_nanos(i64::MIN), "-9223372036854775808");
    assert_json(&Timestamp::MAX, "9223372036854775807");
}

#[test]
fn stamp_json_is_an_explicitly_tagged_enum() {
    assert_json(&Stamp::Static, r#""Static""#);
    assert_json(
        &Stamp::At(Timestamp::from_nanos(1_000_000_000)),
        r#"{"At":1000000000}"#,
    );
    assert_json(&Stamp::At(Timestamp::from_nanos(-5)), r#"{"At":-5}"#);
}

#[test]
fn transform_json_keys_are_target_source_translation_rotation_stamp_in_that_order() {
    assert_json(
        &transform(
            "map",
            "base_link",
            Vector3::new(1.5, -2.25, 3.125),
            short_rotation(),
            Stamp::At(Timestamp::from_nanos(NANOS)),
        ),
        concat!(
            r#"{"target":"map","source":"base_link","#,
            r#""translation":{"x":1.5,"y":-2.25,"z":3.125},"#,
            r#""rotation":{"w":0.8,"x":0.2,"y":-0.4,"z":0.4},"#,
            r#""stamp":{"At":1753142400000000000}}"#,
        ),
    );
}

#[test]
fn static_transform_json_carries_the_static_tag() {
    assert_json(
        &transform(
            "base",
            "camera",
            Vector3::new(0.5, 0.0, -0.25),
            Rotation::IDENTITY,
            Stamp::Static,
        ),
        concat!(
            r#"{"target":"base","source":"camera","#,
            r#""translation":{"x":0.5,"y":0.0,"z":-0.25},"#,
            r#""rotation":{"w":1.0,"x":0.0,"y":0.0,"z":0.0},"#,
            r#""stamp":"Static"}"#,
        ),
    );
}

#[test]
fn transform_json_decodes_with_its_keys_in_any_order() {
    // JSON is self-describing: the key order is fixed on output only.
    let json = r#"{
        "stamp": { "At": 1000000000 },
        "rotation": { "z": 0.0, "y": 0.0, "x": 0.0, "w": 1.0 },
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "source": "base",
        "target": "map"
    }"#;

    let decoded: Transform = serde_json::from_str(json).unwrap();

    assert_eq!(
        decoded,
        transform(
            "map",
            "base",
            Vector3::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(Timestamp::from_nanos(1_000_000_000)),
        )
    );
}

// ---------------------------------------------------------------------
// Frozen postcard bytes. In a format that is not self-describing the field
// order is the wire contract: a reordering compiles, passes every JSON
// test and corrupts every stored stream.
// ---------------------------------------------------------------------

#[test]
fn vector3_postcard_bytes_are_frozen() {
    // x, y, z, each a fixed 8-byte little-endian f64.
    let bytes: &[u8] = &[
        0, 0, 0, 0, 0, 0, 248, 63, // x = 1.5 = 0x3FF8_0000_0000_0000
        0, 0, 0, 0, 0, 0, 2, 192, // y = -2.25 = 0xC002_0000_0000_0000
        0, 0, 0, 0, 0, 0, 9, 64, // z = 3.125 = 0x4009_0000_0000_0000
    ];

    assert_frozen(&Vector3::new(1.5, -2.25, 3.125), bytes);
}

#[test]
fn rotation_postcard_bytes_are_frozen() {
    // w, x, y, z, each a fixed 8-byte little-endian f64. x, y and z share a
    // mantissa and differ in the exponent byte: 1/11, 2/11, 4/11.
    let bytes: &[u8] = &[
        23, 93, 116, 209, 69, 23, 237, 63, // w = 10/11
        70, 23, 93, 116, 209, 69, 183, 63, // x = 1/11
        70, 23, 93, 116, 209, 69, 199, 63, // y = 2/11
        70, 23, 93, 116, 209, 69, 215, 63, // z = 4/11
    ];

    assert_frozen(&general_rotation(), bytes);
}

#[test]
fn timestamp_postcard_bytes_are_a_zigzag_varint() {
    // A signed integer is zigzag-mapped (n >= 0 to 2n, n < 0 to -2n - 1)
    // and written as a LEB128 varint, low seven bits first.
    assert_frozen(&Timestamp::from_nanos(0), &[0]);
    assert_frozen(&Timestamp::from_nanos(1), &[2]);
    assert_frozen(&Timestamp::from_nanos(-1), &[1]);
    assert_frozen(&Timestamp::from_nanos(-2), &[3]);

    // 2 * 1_753_142_400_000_000_000 in seven-bit groups, low first:
    // 0, 0, 104, 10, 45, 110, 52, 84, 48; every group but the last has the
    // continuation bit (128) set.
    assert_frozen(
        &Timestamp::from_nanos(NANOS),
        &[128, 128, 232, 138, 173, 238, 180, 212, 48],
    );
    // The negative instant maps to one less, 2 * 1_753_142_400_000_000_000
    // - 1, which borrows through the two zero groups: 127, 127, 103, then
    // the same.
    assert_frozen(
        &Timestamp::from_nanos(-NANOS),
        &[255, 255, 231, 138, 173, 238, 180, 212, 48],
    );
}

#[test]
fn stamp_postcard_bytes_are_frozen() {
    // The variant index as a varint, then the payload. Reordering the
    // variants would reinterpret every stored stamp.
    assert_frozen(&Stamp::Static, &[0]);
    assert_frozen(
        &Stamp::At(Timestamp::from_nanos(NANOS)),
        &[
            1, // Stamp::At = variant 1
            128, 128, 232, 138, 173, 238, 180, 212, 48, // zigzag varint, as above
        ],
    );
    assert_frozen(&Stamp::At(Timestamp::from_nanos(-1)), &[1, 1]);
}

#[test]
fn static_transform_postcard_bytes_are_frozen() {
    // Length-prefixed target name, length-prefixed source name, the
    // translation as three little-endian f64, the rotation as four (w, x,
    // y, z), then the stamp: the variant index alone for a static one. No
    // reserved timestamp value appears on the wire.
    let bytes: &[u8] = &[
        3, 109, 97, 112, // target: length 3, "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // source: length 9, "base_link"
        0, 0, 0, 0, 0, 0, 248, 63, // translation.x = 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // translation.y = -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // translation.z = 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // rotation.w = 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.x = 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.y = 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.z = 0.0
        0, // Stamp::Static = variant 0
    ];

    assert_frozen(
        &transform(
            "map",
            "base_link",
            Vector3::new(1.5, -2.25, 3.125),
            Rotation::IDENTITY,
            Stamp::Static,
        ),
        bytes,
    );
}

#[test]
fn dynamic_transform_postcard_bytes_are_frozen() {
    // The same layout with a dynamic stamp: variant index 1, then the
    // timestamp as a zigzag varint.
    let bytes: &[u8] = &[
        3, 109, 97, 112, // target: length 3, "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // source: length 9, "base_link"
        0, 0, 0, 0, 0, 0, 248, 63, // translation.x = 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // translation.y = -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // translation.z = 3.125
        0, 0, 0, 0, 0, 0, 240, 63, // rotation.w = 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.x = 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.y = 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // rotation.z = 0.0
        1, // Stamp::At = variant 1
        128, 128, 232, 138, 173, 238, 180, 212, 48, // 1_753_142_400_000_000_000, zigzag
    ];

    assert_frozen(
        &transform(
            "map",
            "base_link",
            Vector3::new(1.5, -2.25, 3.125),
            Rotation::IDENTITY,
            Stamp::At(Timestamp::from_nanos(NANOS)),
        ),
        bytes,
    );
}

#[test]
fn transform_with_a_general_rotation_postcard_bytes_are_frozen() {
    // The identity's x, y and z are interchangeable zeros, so the two cases
    // above cannot tell them apart. Here all four components differ. The
    // stamp is negative: -1_000_000_000 maps to 1_999_999_999 =
    // 0x7735_93FF, in seven-bit groups low first 127, 39, 86, 57, 7.
    let bytes: &[u8] = &[
        3, 109, 97, 112, // target: length 3, "map"
        9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // source: length 9, "base_link"
        0, 0, 0, 0, 0, 0, 248, 63, // translation.x = 1.5
        0, 0, 0, 0, 0, 0, 2, 192, // translation.y = -2.25
        0, 0, 0, 0, 0, 0, 9, 64, // translation.z = 3.125
        23, 93, 116, 209, 69, 23, 237, 63, // rotation.w = 10/11
        70, 23, 93, 116, 209, 69, 183, 63, // rotation.x = 1/11
        70, 23, 93, 116, 209, 69, 199, 63, // rotation.y = 2/11
        70, 23, 93, 116, 209, 69, 215, 63, // rotation.z = 4/11
        1,  // Stamp::At = variant 1
        255, 167, 214, 185, 7, // -1_000_000_000, zigzag
    ];

    assert_frozen(
        &transform(
            "map",
            "base_link",
            Vector3::new(1.5, -2.25, 3.125),
            general_rotation(),
            Stamp::At(Timestamp::from_nanos(-1_000_000_000)),
        ),
        bytes,
    );
}

// ---------------------------------------------------------------------
// The pin against 2.4 data. The arrays below are the frozen postcard
// fixtures of the 2.4 `tests/serde.rs`, byte for byte. 2.4 wrote the
// translation first, so each begins with the low bytes of a float.
// ---------------------------------------------------------------------

/// 2.4 `transform_postcard_bytes_are_frozen`.
const V2_DYNAMIC_TRANSFORM: &[u8] = &[
    0, 0, 0, 0, 0, 0, 248, 63, // 1.5
    0, 0, 0, 0, 0, 0, 2, 192, // -2.25
    0, 0, 0, 0, 0, 0, 9, 64, // 3.125
    0, 0, 0, 0, 0, 0, 240, 63, // 1.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    1, // Stamp::At = variant 1
    128, 128, 180, 197, 150, 183, 154, 170, 24, // timestamp varint
    3, 109, 97, 112, // "map"
    9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
];

/// 2.4 `static_transform_postcard_bytes_are_frozen`.
const V2_STATIC_TRANSFORM: &[u8] = &[
    0, 0, 0, 0, 0, 0, 248, 63, // 1.5
    0, 0, 0, 0, 0, 0, 2, 192, // -2.25
    0, 0, 0, 0, 0, 0, 9, 64, // 3.125
    0, 0, 0, 0, 0, 0, 240, 63, // 1.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    0, 0, 0, 0, 0, 0, 0, 0, // 0.0
    0, // Stamp::Static = variant 0
    3, 109, 97, 112, // "map"
    9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
];

/// 2.4 `transform_with_a_general_rotation_postcard_bytes_are_frozen`.
const V2_GENERAL_ROTATION_TRANSFORM: &[u8] = &[
    0, 0, 0, 0, 0, 0, 248, 63, // 1.5
    0, 0, 0, 0, 0, 0, 2, 192, // -2.25
    0, 0, 0, 0, 0, 0, 9, 64, // 3.125
    23, 93, 116, 209, 69, 23, 237, 63, // w = 10/11
    70, 23, 93, 116, 209, 69, 183, 63, // x = 1/11
    70, 23, 93, 116, 209, 69, 199, 63, // y = 2/11
    70, 23, 93, 116, 209, 69, 215, 63, // z = 4/11
    1,  // Stamp::At = variant 1
    128, 128, 180, 197, 150, 183, 154, 170, 24, // timestamp varint
    3, 109, 97, 112, // "map"
    9, 98, 97, 115, 101, 95, 108, 105, 110, 107, // "base_link"
];

/// 2.4 `point_postcard_bytes_are_frozen`. `Point` no longer exists; its
/// stored bytes must not decode as a transform either.
const V2_POINT: &[u8] = &[
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

const V2_FIXTURES: [(&str, &[u8]); 4] = [
    ("dynamic transform", V2_DYNAMIC_TRANSFORM),
    ("static transform", V2_STATIC_TRANSFORM),
    ("general-rotation transform", V2_GENERAL_ROTATION_TRANSFORM),
    ("point", V2_POINT),
];

/// Asserts the bytes do not decode as a v3 `Transform`, and that the
/// decoder did not merely run out of input: a rejection by length would
/// turn into an acceptance as soon as more bytes follow in the stream.
fn assert_rejected_before_the_end(
    bytes: &[u8],
    case: &str,
) {
    match postcard::from_bytes::<Transform>(bytes) {
        Ok(decoded) => panic!("2.4 bytes decoded as a v3 transform ({case}): {decoded:?}"),
        Err(error) => assert_ne!(
            error,
            postcard::Error::DeserializeUnexpectedEnd,
            "rejected only by running out of input ({case})"
        ),
    }
}

#[test]
fn frozen_2_4_postcard_fixtures_fail_to_decode_alone() {
    for (name, fixture) in V2_FIXTURES {
        assert_rejected_before_the_end(fixture, name);
    }
}

#[test]
fn frozen_2_4_postcard_fixtures_fail_to_decode_when_followed_by_another_fixture() {
    for (name, fixture) in V2_FIXTURES {
        for (next_name, next) in V2_FIXTURES {
            let stream = [fixture, next].concat();

            assert_rejected_before_the_end(&stream, &format!("{name}, then {next_name}"));
        }
    }
}

#[test]
fn frozen_2_4_postcard_fixtures_fail_to_decode_when_followed_by_padding() {
    for (name, fixture) in V2_FIXTURES {
        for padding in [0_u8, 1, 127, 128, 255] {
            for length in [1, 2, 7, 8, 9, 64, 256] {
                let mut stream = fixture.to_vec();
                stream.resize(fixture.len() + length, padding);

                assert_rejected_before_the_end(
                    &stream,
                    &format!("{name}, then {length} bytes of {padding}"),
                );
            }
        }
    }
}

#[test]
fn frozen_2_4_postcard_fixtures_are_rejected_by_validation() {
    // The leading zero bytes of 2.4's first float read as two empty frame
    // names, and every later float is read two bytes out of step. The
    // misaligned rotation is far from unit length, so decoding stops at
    // the constructor's check, which postcard reports as a custom error.
    for (name, fixture) in V2_FIXTURES {
        assert_eq!(
            postcard::from_bytes::<Transform>(fixture).unwrap_err(),
            postcard::Error::SerdeDeCustom,
            "{name}"
        );
    }
}

/// This pins a documented unprotected case, not a guarantee. A standalone
/// `Timestamp` or `Stamp` carries nothing a decoder could check, so 2.x
/// postcard data decodes without error at a wrong instant: 2.x wrote an
/// unsigned varint, v3 reads the same bytes as a zigzag varint. Persisted
/// 2.x data is migrated with the 2.x decoder, and the schema version
/// travels outside the payload.
#[test]
fn documented_unprotected_case_a_standalone_2_x_postcard_timestamp_decodes_at_a_wrong_instant() {
    // 2.x `Timestamp::from_nanos(1)`: the u64 1 as a varint.
    let v2_timestamp: &[u8] = &[1];
    assert_eq!(
        postcard::from_bytes::<Timestamp>(v2_timestamp).unwrap(),
        Timestamp::from_nanos(-1)
    );

    // 2.x `Stamp::At(Timestamp::from_nanos(1))`: variant 1, then the same.
    let v2_stamp: &[u8] = &[1, 1];
    assert_eq!(
        postcard::from_bytes::<Stamp>(v2_stamp).unwrap(),
        Stamp::At(Timestamp::from_nanos(-1))
    );

    // The instant of the 2.4 fixtures reads as half its value.
    let v2_fixture_timestamp: &[u8] = &[128, 128, 180, 197, 150, 183, 154, 170, 24];
    assert_eq!(
        postcard::from_bytes::<Timestamp>(v2_fixture_timestamp).unwrap(),
        Timestamp::from_nanos(NANOS / 2)
    );
}

// ---------------------------------------------------------------------
// JSON documents that must not decode
// ---------------------------------------------------------------------

fn json_error(json: &str) -> String {
    match serde_json::from_str::<Transform>(json) {
        Ok(decoded) => panic!("decoded {decoded:?} from {json}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn a_2_x_transform_document_is_an_error() {
    let dynamic = r#"{
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "timestamp": { "At": 1000000000 },
        "parent": "map",
        "child": "base"
    }"#;
    let fixed = r#"{
        "translation": { "x": 0.1, "y": 0.0, "z": 0.5 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "timestamp": "Static",
        "parent": "base",
        "child": "camera"
    }"#;

    // The first key v3 does not know is reported before the missing ones.
    for json in [dynamic, fixed] {
        let message = json_error(json);
        assert!(message.contains("unknown field `timestamp`"), "{message}");
    }
}

#[test]
fn a_2_x_transform_document_without_its_unknown_keys_is_missing_target() {
    let json = r#"{
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }
    }"#;

    let message = json_error(json);

    assert!(message.contains("missing field `target`"), "{message}");
}

#[test]
fn a_half_migrated_document_with_a_leftover_2_x_key_is_an_error() {
    for (leftover, value) in [
        ("timestamp", r#"{ "At": 1000000000 }"#),
        ("timestamp", r#""Static""#),
        ("parent", r#""map""#),
        ("child", r#""base""#),
    ] {
        let json = format!(
            r#"{{
                "target": "map",
                "source": "base",
                "translation": {{ "x": 1.0, "y": 0.0, "z": 0.0 }},
                "rotation": {{ "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }},
                "stamp": {{ "At": 1000000000 }},
                "{leftover}": {value}
            }}"#
        );

        let message = json_error(&json);

        assert!(
            message.contains(&format!("unknown field `{leftover}`")),
            "{message}"
        );
    }
}

#[test]
fn an_unknown_key_inside_rotation_is_an_error() {
    let json = r#"{
        "target": "map",
        "source": "base",
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0, "angle": 0.0 },
        "stamp": "Static"
    }"#;

    let message = json_error(json);

    assert!(message.contains("unknown field `angle`"), "{message}");
    assert!(
        serde_json::from_str::<Rotation>(r#"{"w":1.0,"x":0.0,"y":0.0,"z":0.0,"angle":0.0}"#)
            .is_err()
    );
}

#[test]
fn an_unknown_key_inside_translation_is_an_error() {
    let json = r#"{
        "target": "map",
        "source": "base",
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0, "w": 1.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "stamp": "Static"
    }"#;

    let message = json_error(json);

    assert!(message.contains("unknown field `w`"), "{message}");
    assert!(serde_json::from_str::<Vector3>(r#"{"x":1.0,"y":0.0,"z":0.0,"w":1.0}"#).is_err());
}

#[test]
fn a_missing_stamp_is_an_error() {
    // A producer that drops the stamp must not mint a static transform,
    // which a registry would serve at every instant.
    let json = r#"{
        "target": "map",
        "source": "base",
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }
    }"#;

    let message = json_error(json);

    assert!(message.contains("missing field `stamp`"), "{message}");
}

#[test]
fn a_null_stamp_is_an_error() {
    // `null` is not a spelling of `Stamp::Static`.
    let json = r#"{
        "target": "map",
        "source": "base",
        "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
        "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
        "stamp": null
    }"#;

    json_error(json);
    assert!(serde_json::from_str::<Stamp>("null").is_err());
}

#[test]
fn an_unknown_stamp_variant_is_an_error() {
    for stamp in [
        r#""Dynamic""#,
        r#""static""#,
        r#"{ "Latest": 5 }"#,
        r#"{ "at": 5 }"#,
        "5",
    ] {
        let json = format!(
            r#"{{
                "target": "map",
                "source": "base",
                "translation": {{ "x": 1.0, "y": 0.0, "z": 0.0 }},
                "rotation": {{ "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }},
                "stamp": {stamp}
            }}"#
        );

        json_error(&json);
        assert!(serde_json::from_str::<Stamp>(stamp).is_err(), "{stamp}");
    }

    // In postcard the variant is an index; 2 names nothing.
    assert!(postcard::from_bytes::<Stamp>(&[2, 0]).is_err());
}

// ---------------------------------------------------------------------
// Decoding validates through the constructors
// ---------------------------------------------------------------------

/// "map" from "base" at instant 0, in postcard. The names take bytes 0..9,
/// so `translation.x` starts at byte 9 and `rotation.w` at byte 33.
fn valid_postcard_transform() -> Vec<u8> {
    postcard::to_allocvec(&transform(
        "map",
        "base",
        Vector3::new(1.0, 2.0, 3.0),
        Rotation::IDENTITY,
        Stamp::At(Timestamp::from_nanos(0)),
    ))
    .unwrap()
}

const TRANSLATION_OFFSETS: [usize; 3] = [9, 17, 25];
const ROTATION_OFFSETS: [usize; 4] = [33, 41, 49, 57];

/// The four components as postcard writes a `Rotation`.
fn rotation_bytes(wxyz: [f64; 4]) -> Vec<u8> {
    wxyz.iter().flat_map(|c| c.to_le_bytes()).collect()
}

#[test]
fn decoding_an_empty_frame_name_is_an_error() {
    for (target, source) in [("", "base"), ("map", ""), ("", "")] {
        let json = format!(
            r#"{{
                "target": "{target}",
                "source": "{source}",
                "translation": {{ "x": 1.0, "y": 0.0, "z": 0.0 }},
                "rotation": {{ "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }},
                "stamp": "Static"
            }}"#
        );

        let message = json_error(&json);

        assert!(message.contains("frame name is empty"), "{message}");
    }

    // The same in postcard: an empty target, then an empty source.
    let valid = valid_postcard_transform();
    let empty_target = [&[0], &valid[4..]].concat();
    let empty_source = [&valid[..4], &[0], &valid[9..]].concat();
    for bytes in [empty_target, empty_source] {
        assert_eq!(
            postcard::from_bytes::<Transform>(&bytes).unwrap_err(),
            postcard::Error::SerdeDeCustom
        );
    }
}

#[test]
fn decoding_a_non_finite_translation_component_is_an_error() {
    // JSON cannot spell NaN or infinity; a binary format can.
    for offset in TRANSLATION_OFFSETS {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut bytes = valid_postcard_transform();
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());

            assert_eq!(
                postcard::from_bytes::<Transform>(&bytes).unwrap_err(),
                postcard::Error::SerdeDeCustom,
                "{value} at byte {offset}"
            );
        }
    }
}

#[test]
fn decoding_a_non_finite_rotation_component_is_an_error() {
    for offset in ROTATION_OFFSETS {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut bytes = valid_postcard_transform();
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());

            assert_eq!(
                postcard::from_bytes::<Transform>(&bytes).unwrap_err(),
                postcard::Error::SerdeDeCustom,
                "{value} at byte {offset}"
            );
        }
    }

    assert_eq!(
        postcard::from_bytes::<Rotation>(&rotation_bytes([f64::NAN, 0.0, 0.0, 0.0])).unwrap_err(),
        postcard::Error::SerdeDeCustom
    );
    // An overflowing literal is how JSON reaches a non-finite value, and
    // serde_json refuses it before the constructor sees it.
    assert!(serde_json::from_str::<Rotation>(r#"{"w":1e999,"x":0.0,"y":0.0,"z":0.0}"#).is_err());
}

#[test]
fn decoding_a_rotation_further_than_1e_6_from_unit_norm_is_an_error() {
    // A norm of 1.01 scales everything it is applied to by 2 %. 1.000002
    // and 0.999998 sit just outside the tolerance.
    for w in ["1.01", "2.0", "0.0", "1.000002", "0.999998"] {
        let json = format!(
            r#"{{
                "target": "map",
                "source": "base",
                "translation": {{ "x": 1.0, "y": 0.0, "z": 0.0 }},
                "rotation": {{ "w": {w}, "x": 0.0, "y": 0.0, "z": 0.0 }},
                "stamp": {{ "At": 1000000000 }}
            }}"#
        );

        let message = json_error(&json);

        assert!(message.contains("not a unit quaternion"), "{message}");

        let standalone = format!(r#"{{"w":{w},"x":0.0,"y":0.0,"z":0.0}}"#);
        assert!(
            serde_json::from_str::<Rotation>(&standalone).is_err(),
            "{standalone}"
        );
    }

    let mut bytes = valid_postcard_transform();
    bytes[ROTATION_OFFSETS[0]..ROTATION_OFFSETS[0] + 8].copy_from_slice(&1.01_f64.to_le_bytes());
    assert_eq!(
        postcard::from_bytes::<Transform>(&bytes).unwrap_err(),
        postcard::Error::SerdeDeCustom
    );
}

#[test]
fn decoding_a_rotation_inside_the_tolerance_normalizes_it() {
    // A unit quaternion that travelled as f32: about 1e-8 from unit norm,
    // inside the 1e-6 input tolerance and outside the band of 8 EPSILON
    // that stored rotations keep.
    let [w, x, y, z] = [10.0_f32 / 11.0, 1.0 / 11.0, 2.0 / 11.0, 4.0 / 11.0].map(f64::from);
    let squared_norm = w * w + x * x + y * y + z * z;
    assert!((squared_norm - 1.0).abs() > 8.0 * f64::EPSILON);
    assert!((squared_norm.sqrt() - 1.0).abs() < 1e-6);
    let expected = Rotation::from_wxyz(w, x, y, z).unwrap();

    let standalone: Rotation = postcard::from_bytes(&rotation_bytes([w, x, y, z])).unwrap();
    assert_eq!(standalone, expected);
    assert_ne!(standalone.w().to_bits(), w.to_bits());
    let stored_squared_norm = standalone.w() * standalone.w()
        + standalone.x() * standalone.x()
        + standalone.y() * standalone.y()
        + standalone.z() * standalone.z();
    assert!((stored_squared_norm - 1.0).abs() <= 8.0 * f64::EPSILON);

    let mut bytes = valid_postcard_transform();
    bytes[ROTATION_OFFSETS[0]..ROTATION_OFFSETS[3] + 8]
        .copy_from_slice(&rotation_bytes([w, x, y, z]));
    let inside_a_transform: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(inside_a_transform.rotation(), expected);

    // The normalized value is a fixed point: it encodes and decodes
    // unchanged from here on.
    let reencoded = postcard::to_allocvec(&standalone).unwrap();
    assert_eq!(
        postcard::from_bytes::<Rotation>(&reencoded).unwrap(),
        standalone
    );
}

#[test]
fn a_standalone_vector3_decodes_unchecked() {
    // Finiteness is enforced when a vector enters a `Transform`, not here.
    let bytes: Vec<u8> = [f64::NAN, f64::INFINITY, -0.0]
        .iter()
        .flat_map(|c| c.to_le_bytes())
        .collect();

    let decoded: Vector3 = postcard::from_bytes(&bytes).unwrap();

    assert!(decoded.x.is_nan());
    assert_eq!(decoded.y.to_bits(), f64::INFINITY.to_bits());
    assert_eq!(decoded.z.to_bits(), (-0.0_f64).to_bits());
    assert!(
        Transform::new("map", "base", decoded, Rotation::IDENTITY, Stamp::Static).is_err(),
        "the constructor is where the vector is checked"
    );
}

// ---------------------------------------------------------------------
// Fixed points: what the crate hands out decodes to itself
// ---------------------------------------------------------------------

#[test]
fn constructor_built_transforms_are_decode_fixed_points() {
    let half = core::f64::consts::FRAC_1_SQRT_2;
    for built in [
        transform(
            "map",
            "base",
            Vector3::new(1.0, 2.0, 3.0),
            Rotation::IDENTITY,
            Stamp::Static,
        ),
        transform(
            "map",
            "base_link",
            Vector3::new(1.5, -2.25, 3.125),
            general_rotation(),
            Stamp::At(Timestamp::from_nanos(NANOS)),
        ),
        transform(
            "odom",
            "base",
            Vector3::new(0.1, -0.2, 1e-300),
            Rotation::from_wxyz(half, 0.0, 0.0, half).unwrap(),
            Stamp::At(Timestamp::from_nanos(i64::MIN)),
        ),
        transform(
            "a frame with spaces, \"quotes\" and \u{e9}",
            "b",
            Vector3::new(-0.0, f64::MAX, f64::MIN_POSITIVE),
            Rotation::from_wxyz(0.1, 0.3, 0.5, 0.806_225_774_829_855).unwrap(),
            Stamp::At(Timestamp::MAX),
        ),
        // Normalized on construction from f32-widened components.
        transform(
            "map",
            "imu",
            Vector3::new(0.0, 0.0, 0.0),
            Rotation::from_wxyz(
                f64::from(0.1_f32),
                f64::from(0.3_f32),
                f64::from(0.5_f32),
                f64::from(0.806_225_8_f32),
            )
            .unwrap(),
            Stamp::Static,
        ),
    ] {
        assert_fixed_point(&built);
    }
}

#[test]
fn derived_transforms_are_decode_fixed_points() {
    let stamp = Stamp::At(Timestamp::from_nanos(NANOS));
    let map_from_odom = transform(
        "map",
        "odom",
        Vector3::new(1.5, -2.25, 3.125),
        general_rotation(),
        stamp,
    );
    let odom_from_base = transform(
        "odom",
        "base",
        Vector3::new(0.1, 0.2, -0.3),
        Rotation::from_wxyz(0.1, 0.3, 0.5, 0.806_225_774_829_855).unwrap(),
        stamp,
    );
    let estimate = transform(
        "map",
        "odom",
        Vector3::new(1.51, -2.24, 3.12),
        Rotation::from_wxyz(0.9, 0.1, 0.2, 0.374_165_738_677_394).unwrap(),
        stamp,
    );

    let map_from_base = map_from_odom.compose(&odom_from_base).unwrap();
    let base_from_map = map_from_base.inverse().unwrap();
    let round_trip = map_from_base.compose(&base_from_map).unwrap();
    // A pose error: the estimate against the truth, a transform from a
    // frame to itself that is not the identity.
    let pose_error = estimate.compose(&map_from_odom.inverse().unwrap()).unwrap();
    assert_eq!(pose_error.target(), "map");
    assert_eq!(pose_error.source(), "map");
    assert_ne!(pose_error.translation(), Vector3::new(0.0, 0.0, 0.0));

    let mut chained = map_from_base.clone();
    for _ in 0..50 {
        chained = chained
            .compose(&base_from_map)
            .unwrap()
            .compose(&map_from_base)
            .unwrap();
    }

    for derived in [
        map_from_base,
        base_from_map,
        round_trip,
        pose_error,
        chained,
    ] {
        assert_fixed_point(&derived);
    }
}

#[test]
fn the_negative_zeros_of_an_inverted_zero_translation_survive_the_round_trip() {
    let rotation_only = transform(
        "base",
        "imu",
        Vector3::new(0.0, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    );

    let inverted = rotation_only.inverse().unwrap();

    let negative_zero = (-0.0_f64).to_bits();
    assert_eq!(bits(&inverted)[..3], [negative_zero; 3]);
    assert_fixed_point(&inverted);

    let json = serde_json::to_string(&inverted).unwrap();
    assert!(
        json.contains(r#""translation":{"x":-0.0,"y":-0.0,"z":-0.0}"#),
        "{json}"
    );
    let bytes = postcard::to_allocvec(&inverted).unwrap();
    let decoded: Transform = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(bits(&decoded)[..3], [negative_zero; 3]);
}

#[test]
fn registry_lookup_results_are_decode_fixed_points() {
    let half = core::f64::consts::FRAC_1_SQRT_2;
    let at = |nanos| Stamp::At(Timestamp::from_nanos(nanos));
    let mut registry = Registry::unbounded();
    registry
        .insert_all(&[
            transform(
                "map",
                "odom",
                Vector3::new(1.5, -2.25, 3.125),
                general_rotation(),
                at(1_000_000_000),
            ),
            transform(
                "map",
                "odom",
                Vector3::new(1.7, -2.0, 3.0),
                Rotation::from_wxyz(0.9, 0.1, 0.2, 0.374_165_738_677_394).unwrap(),
                at(2_000_000_000),
            ),
            transform(
                "odom",
                "base",
                Vector3::new(0.1, 0.2, -0.3),
                Rotation::IDENTITY,
                at(900_000_000),
            ),
            transform(
                "odom",
                "base",
                Vector3::new(0.4, 0.1, -0.2),
                Rotation::from_wxyz(half, 0.0, 0.0, half).unwrap(),
                at(2_100_000_000),
            ),
            transform(
                "base",
                "lidar",
                Vector3::new(0.0, 0.0, 0.5),
                Rotation::from_wxyz(0.1, 0.3, 0.5, 0.806_225_774_829_855).unwrap(),
                Stamp::Static,
            ),
            transform(
                "base",
                "camera",
                Vector3::new(0.2, 0.0, 0.1),
                Rotation::from_wxyz(0.5, -0.5, 0.5, -0.5).unwrap(),
                Stamp::Static,
            ),
        ])
        .unwrap();

    for (target, source, nanos) in [
        // A stored sample, an interpolated one, and chains of both.
        ("map", "odom", 1_000_000_000),
        ("map", "odom", 1_333_333_333),
        ("map", "lidar", 1_250_000_000),
        ("map", "lidar", 1_999_999_999),
        // Away from the root: the chain is inverted.
        ("lidar", "map", 1_250_000_000),
        ("odom", "map", 1_700_000_001),
        // Across a common ancestor, over static edges only.
        ("camera", "lidar", 1_500_000_000),
        ("lidar", "camera", -5),
        // A frame to itself.
        ("base", "base", 1_500_000_000),
    ] {
        let found = registry
            .lookup(target, source, Timestamp::from_nanos(nanos))
            .unwrap();

        assert_eq!(found.stamp(), at(nanos));
        assert_fixed_point(&found);
    }
}
