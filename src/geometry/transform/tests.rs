//! Unit tests.

use super::Transform;
use crate::{
    BatchError, CrossTimeTransform, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp,
    TransformError, Vector3, geometry::Isometry,
};
use alloc::{
    borrow::Cow,
    boxed::Box,
    format,
    string::{String, ToString},
};
use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;

const ORIGIN: Vector3 = Vector3::new(0.0, 0.0, 0.0);

fn at(nanos: i64) -> Stamp {
    Stamp::At(Timestamp::from_nanos(nanos))
}

fn quarter_turn_z() -> Rotation {
    Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap()
}

/// A transform one unit along x, without rotation.
fn edge(
    target: &str,
    source: &str,
    stamp: Stamp,
) -> Transform {
    Transform::new(
        target,
        source,
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        stamp,
    )
    .unwrap()
}

fn assert_vector_close(
    actual: Vector3,
    expected: Vector3,
    epsilon: f64,
) {
    assert_abs_diff_eq!(actual.x, expected.x, epsilon = epsilon);
    assert_abs_diff_eq!(actual.y, expected.y, epsilon = epsilon);
    assert_abs_diff_eq!(actual.z, expected.z, epsilon = epsilon);
}

// --- Construction -------------------------------------------------------

#[test]
fn new_exposes_the_components_it_was_built_from() {
    let transform = Transform::new(
        "map",
        "base",
        Vector3::new(1.0, 2.0, 3.0),
        quarter_turn_z(),
        at(0),
    )
    .unwrap();

    assert_eq!(transform.target(), "map");
    assert_eq!(transform.source(), "base");
    assert_eq!(transform.translation(), Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(transform.rotation(), quarter_turn_z());
    assert_eq!(transform.stamp(), at(0));
    assert_eq!(
        transform.isometry(),
        Isometry::new(Vector3::new(1.0, 2.0, 3.0), quarter_turn_z()).unwrap()
    );
}

#[test]
fn new_keeps_a_static_stamp() {
    let mount = Transform::new(
        "base",
        "camera",
        Vector3::new(0.1, 0.0, 0.5),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();

    assert_eq!(mount.stamp(), Stamp::Static);
    assert_eq!(mount.target(), "base");
    assert_eq!(mount.source(), "camera");
}

#[test]
fn new_rejects_an_empty_target_an_empty_source_and_both() {
    for (target, source) in [("", "x"), ("x", ""), ("", "")] {
        assert_eq!(
            Transform::new(
                target,
                source,
                Vector3::new(1.0, 0.0, 0.0),
                Rotation::IDENTITY,
                at(0)
            ),
            Err(TransformError::EmptyFrameName),
            "target {target:?}, source {source:?}"
        );
        assert_eq!(
            Transform::new(target, source, ORIGIN, Rotation::IDENTITY, Stamp::Static),
            Err(TransformError::EmptyFrameName)
        );
    }
}

#[test]
fn new_reports_an_empty_name_before_a_non_finite_translation() {
    let bad = Vector3::new(f64::NAN, f64::INFINITY, 0.0);

    for (target, source) in [("", "x"), ("x", ""), ("", "")] {
        assert_eq!(
            Transform::new(target, source, bad, Rotation::IDENTITY, at(0)),
            Err(TransformError::EmptyFrameName)
        );
    }
    assert_eq!(
        Transform::new("a", "b", bad, Rotation::IDENTITY, at(0)),
        Err(TransformError::NonFinite)
    );
}

#[test]
fn new_rejects_a_non_finite_translation_in_any_component() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for translation in [
            Vector3::new(bad, 0.0, 0.0),
            Vector3::new(0.0, bad, 0.0),
            Vector3::new(0.0, 0.0, bad),
        ] {
            for stamp in [at(0), Stamp::Static] {
                assert_eq!(
                    Transform::new("map", "base", translation, Rotation::IDENTITY, stamp),
                    Err(TransformError::NonFinite)
                );
            }
        }
    }
}

#[test]
fn new_accepts_every_finite_translation() {
    let extreme = Vector3::new(f64::MAX, f64::MIN, f64::MIN_POSITIVE);

    let transform = Transform::new("a", "b", extreme, Rotation::IDENTITY, at(0)).unwrap();

    assert_eq!(transform.translation(), extreme);
}

#[test]
fn a_rotation_that_would_corrupt_every_lookup_cannot_reach_new() {
    // A rotation-equivalent but non-unit quaternion scales everything it
    // is applied to, here by 2%. It fails where the rotation is built, so
    // `Transform::new` has no rotation error to report.
    assert_eq!(
        Rotation::from_wxyz(1.01, 0.0, 0.0, 0.0),
        Err(TransformError::NonUnitRotation { norm: 1.01 })
    );
    assert_eq!(
        Rotation::from_wxyz(f64::INFINITY, 0.0, 0.0, 0.0),
        Err(TransformError::NonFinite)
    );

    // A unit norm with f32-grade precision loss stays representable, and
    // the transform built from it holds a unit rotation.
    let f32_grade = Rotation::from_wxyz(1.0 + 1e-8, 0.0, 0.0, 0.0).unwrap();
    let transform = Transform::new("map", "base", ORIGIN, f32_grade, at(0)).unwrap();
    assert_eq!(transform.rotation(), Rotation::IDENTITY);
}

#[test]
fn new_accepts_every_common_string_type_for_a_name() {
    let expected = edge("map", "base", at(0));
    let build = |target: String, source: String| {
        Transform::new(
            target,
            source,
            Vector3::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            at(0),
        )
        .unwrap()
    };
    let v = Vector3::new(1.0, 0.0, 0.0);
    let r = Rotation::IDENTITY;

    // &str and String, in both positions.
    assert_eq!(build("map".into(), "base".into()), expected);
    assert_eq!(
        Transform::new("map", String::from("base"), v, r, at(0)).unwrap(),
        expected
    );
    assert_eq!(
        Transform::new(String::from("map"), "base", v, r, at(0)).unwrap(),
        expected
    );

    // &String: borrowed names are copied and the originals stay usable.
    let (map, base) = (String::from("map"), String::from("base"));
    assert_eq!(Transform::new(&map, &base, v, r, at(0)).unwrap(), expected);
    assert_eq!((map.as_str(), base.as_str()), ("map", "base"));

    // Cow, borrowed and owned.
    let borrowed: Cow<'_, str> = Cow::Borrowed("map");
    let owned: Cow<'_, str> = Cow::Owned(String::from("base"));
    assert_eq!(
        Transform::new(borrowed, owned, v, r, at(0)).unwrap(),
        expected
    );

    // Box<str>.
    let (map, base): (Box<str>, Box<str>) = ("map".into(), "base".into());
    assert_eq!(Transform::new(map, base, v, r, at(0)).unwrap(), expected);
}

#[test]
fn new_keeps_names_verbatim() {
    // Names are opaque: no trimming, no case folding, no separator rule.
    let transform = Transform::new(" Map/α ", "/base", ORIGIN, Rotation::IDENTITY, at(0)).unwrap();

    assert_eq!(transform.target(), " Map/α ");
    assert_eq!(transform.source(), "/base");
}

#[test]
fn new_allows_equal_names_with_any_geometry() {
    // The identity, which a self-named transform ought to be ...
    let identity = Transform::new("a", "a", ORIGIN, Rotation::IDENTITY, at(0)).unwrap();
    assert_eq!(identity.target(), identity.source());

    // ... and an offset and a rotation, a caller error no check catches.
    for stamp in [at(0), Stamp::Static] {
        let offset = Transform::new(
            "a",
            "a",
            Vector3::new(1.0, 2.0, 3.0),
            quarter_turn_z(),
            stamp,
        )
        .unwrap();

        assert_eq!(offset.target(), "a");
        assert_eq!(offset.source(), "a");
        assert_eq!(offset.translation(), Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(offset.rotation(), quarter_turn_z());
    }
}

#[test]
fn from_parts_labels_an_isometry_without_changing_it() {
    let isometry = Isometry::new(Vector3::new(1.0, 2.0, 3.0), quarter_turn_z()).unwrap();

    let transform = Transform::from_parts("map".into(), "base".into(), isometry, at(7));

    assert_eq!(
        transform,
        Transform::new(
            "map",
            "base",
            Vector3::new(1.0, 2.0, 3.0),
            quarter_turn_z(),
            at(7)
        )
        .unwrap()
    );
    assert_eq!(transform.isometry(), isometry);
}

#[test]
fn equality_is_exact_over_every_part() {
    let reference = edge("a", "b", at(1));

    assert_eq!(edge("a", "b", at(1)), reference);
    assert_eq!(reference.clone(), reference);

    assert_ne!(edge("x", "b", at(1)), reference);
    assert_ne!(edge("a", "x", at(1)), reference);
    assert_ne!(edge("b", "a", at(1)), reference);
    assert_ne!(edge("a", "b", at(2)), reference);
    assert_ne!(edge("a", "b", Stamp::Static), reference);
    assert_ne!(
        Transform::new(
            "a",
            "b",
            Vector3::new(1.0 + f64::EPSILON, 0.0, 0.0),
            Rotation::IDENTITY,
            at(1)
        )
        .unwrap(),
        reference
    );
    assert_ne!(
        Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            quarter_turn_z(),
            at(1)
        )
        .unwrap(),
        reference
    );
}

#[test]
fn debug_names_the_frames_and_the_stamp() {
    let text = format!("{:?}", edge("map", "base", at(5)));

    assert!(text.starts_with("Transform {"), "{text}");
    for part in [
        "target: \"map\"",
        "source: \"base\"",
        "translation: Vector3 { x: 1.0, y: 0.0, z: 0.0 }",
        "Rotation { w: 1.0, x: 0.0, y: 0.0, z: 0.0 }",
        "stamp: At(Timestamp(5))",
    ] {
        assert!(text.contains(part), "{part} missing from {text}");
    }
}

// --- Composition --------------------------------------------------------

#[test]
fn compose_adds_translations_and_chains_the_frames() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        at(0),
    )
    .unwrap();
    let t_b_c = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 2.0, 0.0),
        Rotation::IDENTITY,
        at(0),
    )
    .unwrap();

    let result = t_a_b.compose(&t_b_c).unwrap();

    assert_eq!(result.translation(), Vector3::new(1.0, 2.0, 0.0));
    assert_eq!(result.target(), "a");
    assert_eq!(result.source(), "c");
    assert_eq!(result.stamp(), at(0));
}

#[test]
fn compose_rotates_the_right_hand_translation() {
    let t_a_b = Transform::new("a", "b", ORIGIN, quarter_turn_z(), at(0)).unwrap();
    let t_b_c = Transform::new(
        "b",
        "c",
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        at(0),
    )
    .unwrap();

    let result = t_a_b.compose(&t_b_c).unwrap();

    assert_vector_close(
        result.translation(),
        Vector3::new(0.0, 1.0, 0.0),
        f64::EPSILON,
    );
    assert_eq!(result.rotation(), quarter_turn_z());
}

#[test]
fn compose_delegates_the_numbers_to_the_isometries() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 2.0, 3.0),
        quarter_turn_z(),
        at(0),
    )
    .unwrap();
    let t_b_c = Transform::new(
        "b",
        "c",
        Vector3::new(-0.5, 0.25, 4.0),
        Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap(),
        at(0),
    )
    .unwrap();

    let result = t_a_b.compose(&t_b_c).unwrap();

    assert_eq!(
        result.isometry(),
        t_a_b.isometry().compose(t_b_c.isometry()).unwrap()
    );
}

#[test]
fn compose_rejects_reversed_composition() {
    // Only `t_a_b.compose(&t_b_c)` chains. The reversed order composes the
    // underlying numbers in the wrong order and is rejected.
    let t_a_b = edge("a", "b", at(1_000_000_000));
    let t_b_c = edge("b", "c", at(1_000_000_000));

    assert_eq!(
        t_b_c.compose(&t_a_b),
        Err(TransformError::FrameMismatch {
            expected: "c".into(),
            found: "a".into(),
        })
    );
}

#[test]
fn compose_frame_mismatch_names_the_left_source_and_the_right_target() {
    let t_a_b = edge("a", "b", at(1_000_000_000));
    let t_c_d = edge("c", "d", at(1_000_000_000));

    assert_eq!(
        t_a_b.compose(&t_c_d),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "c".into(),
        })
    );
}

#[test]
fn compose_rejects_every_pairing_but_left_source_equals_right_target() {
    let t = at(1_000_000_000);
    let t_a_b = edge("a", "b", t);

    // Same source on both sides: 2.4 named this case separately.
    assert_eq!(
        t_a_b.compose(&edge("c", "b", t)),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "c".into(),
        })
    );
    // Same target on both sides.
    assert_eq!(
        t_a_b.compose(&edge("a", "c", t)),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "a".into(),
        })
    );
    // A transform with itself.
    assert_eq!(
        t_a_b.compose(&t_a_b),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "a".into(),
        })
    );
    // Names differing only in case do not chain.
    assert_eq!(
        t_a_b.compose(&edge("B", "c", t)),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "B".into(),
        })
    );
}

#[test]
fn compose_stamp_mismatch_carries_both_instants_in_operand_order() {
    let t_a_b = edge("a", "b", at(1_000_000_000));
    let t_b_c = edge("b", "c", at(2_000_000_000));

    assert_eq!(
        t_a_b.compose(&t_b_c),
        Err(TransformError::StampMismatch {
            lhs: Timestamp::from_nanos(1_000_000_000),
            rhs: Timestamp::from_nanos(2_000_000_000),
        })
    );

    // One nanosecond apart, before the epoch.
    assert_eq!(
        edge("a", "b", at(-1)).compose(&edge("b", "c", at(-2))),
        Err(TransformError::StampMismatch {
            lhs: Timestamp::from_nanos(-1),
            rhs: Timestamp::from_nanos(-2),
        })
    );
}

#[test]
fn stamp_mismatch_reports_wall_clock_instants_exactly() {
    // Wall-clock nanosecond counts do not convert to f64 seconds exactly.
    // The payload is the instants themselves and the message prints them
    // in integers, so neither loses a digit.
    let t1 = Timestamp::from_nanos(1_783_400_000_123_456_789);
    let t2 = Timestamp::from_nanos(1_783_400_001_123_456_789);

    let error = edge("a", "b", Stamp::At(t1))
        .compose(&edge("b", "c", Stamp::At(t2)))
        .unwrap_err();

    assert_eq!(error, TransformError::StampMismatch { lhs: t1, rhs: t2 });
    assert_eq!(
        error.to_string(),
        "stamps 1783400000.123456789 and 1783400001.123456789 differ"
    );
}

#[test]
fn compose_reports_the_frame_mismatch_when_frames_and_stamps_both_mismatch() {
    let t_a_b = edge("a", "b", at(1));
    let t_c_d = edge("c", "d", at(2));

    assert_eq!(
        t_a_b.compose(&t_c_d),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "c".into(),
        })
    );
}

#[test]
fn compose_reports_a_structural_error_before_a_numerical_one() {
    let far = |target, source, stamp| {
        Transform::new(
            target,
            source,
            Vector3::new(f64::MAX, 0.0, 0.0),
            Rotation::IDENTITY,
            stamp,
        )
        .unwrap()
    };

    // Each pair would overflow if it were composed.
    assert_eq!(
        far("a", "b", at(1)).compose(&far("c", "d", at(1))),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "c".into(),
        })
    );
    assert_eq!(
        far("a", "b", at(1)).compose(&far("b", "c", at(2))),
        Err(TransformError::StampMismatch {
            lhs: Timestamp::from_nanos(1),
            rhs: Timestamp::from_nanos(2),
        })
    );
    assert_eq!(
        far("a", "b", at(1)).compose(&far("b", "c", at(1))),
        Err(TransformError::NonFinite)
    );
}

#[test]
fn compose_of_two_dynamic_operands_at_one_instant_keeps_it() {
    let result = edge("a", "b", at(5))
        .compose(&edge("b", "c", at(5)))
        .unwrap();

    assert_eq!(result.stamp(), at(5));
}

#[test]
fn compose_static_with_dynamic_takes_the_dynamic_stamp() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();
    let t_b_c = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 1.0, 0.0),
        Rotation::IDENTITY,
        at(1_000_000_000),
    )
    .unwrap();

    let expected = Transform::new(
        "a",
        "c",
        Vector3::new(1.0, 1.0, 0.0),
        Rotation::IDENTITY,
        at(1_000_000_000),
    )
    .unwrap();
    assert_eq!(t_a_b.compose(&t_b_c), Ok(expected));
}

#[test]
fn compose_dynamic_with_static_takes_the_dynamic_stamp() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        at(1_000_000_000),
    )
    .unwrap();
    let t_b_c = Transform::new(
        "b",
        "c",
        Vector3::new(0.0, 1.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();

    let expected = Transform::new(
        "a",
        "c",
        Vector3::new(1.0, 1.0, 0.0),
        Rotation::IDENTITY,
        at(1_000_000_000),
    )
    .unwrap();
    assert_eq!(t_a_b.compose(&t_b_c), Ok(expected));
}

#[test]
fn compose_of_two_static_operands_is_static() {
    let result = edge("a", "b", Stamp::Static)
        .compose(&edge("b", "c", Stamp::Static))
        .unwrap();

    assert_eq!(result.stamp(), Stamp::Static);
    assert_eq!(result.translation(), Vector3::new(2.0, 0.0, 0.0));
}

#[test]
fn compose_reports_an_overflowing_translation() {
    let far = |target, source| {
        Transform::new(
            target,
            source,
            Vector3::new(1e308, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::Static,
        )
        .unwrap()
    };

    assert_eq!(
        far("a", "b").compose(&far("b", "c")),
        Err(TransformError::NonFinite)
    );
}

#[test]
fn doubling_a_translation_fails_on_the_step_that_overflows() {
    // No magnitude bound is closed under composition: a self-named unit
    // step doubled again and again leaves f64 on the 1024th doubling, and
    // that step is an error, never an infinite transform.
    let mut step = Transform::new(
        "a",
        "a",
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();

    for doubling in 1..=1023 {
        step = step.compose(&step).unwrap();
        assert!(step.translation().x.is_finite(), "doubling {doubling}");
    }
    assert_eq!(step.translation().x.to_bits(), 2.0_f64.powi(1023).to_bits());
    assert_eq!(step.compose(&step), Err(TransformError::NonFinite));
}

#[test]
fn compose_does_not_consume_its_operands() {
    let t_a_b = edge("a", "b", at(0));
    let t_b_c = edge("b", "c", at(0));

    let first = t_a_b.compose(&t_b_c).unwrap();
    let second = t_a_b.compose(&t_b_c).unwrap();

    assert_eq!(first, second);
    assert_eq!(t_a_b, edge("a", "b", at(0)));
    assert_eq!(t_b_c, edge("b", "c", at(0)));

    // A failed composition leaves them usable too.
    assert!(t_b_c.compose(&t_a_b).is_err());
    assert_eq!(t_a_b.compose(&t_b_c).unwrap(), first);
}

#[test]
fn compose_chains_with_the_question_mark_operator() {
    fn chain(
        a: &Transform,
        b: &Transform,
        c: &Transform,
    ) -> Result<Transform, TransformError> {
        a.compose(b)?.compose(c)
    }
    let t_a_b = edge("a", "b", Stamp::Static);
    let t_b_c = edge("b", "c", at(3));
    let t_c_d = edge("c", "d", at(3));

    let t_a_d = chain(&t_a_b, &t_b_c, &t_c_d).unwrap();

    assert_eq!(t_a_d.target(), "a");
    assert_eq!(t_a_d.source(), "d");
    assert_eq!(t_a_d.translation(), Vector3::new(3.0, 0.0, 0.0));
    assert_eq!(t_a_d.stamp(), at(3));

    // The first failing link is the one reported.
    assert_eq!(
        chain(&t_a_b, &t_c_d, &t_b_c),
        Err(TransformError::FrameMismatch {
            expected: "b".into(),
            found: "c".into(),
        })
    );
    assert_eq!(
        chain(&t_a_b, &t_b_c, &edge("c", "d", at(4))),
        Err(TransformError::StampMismatch {
            lhs: Timestamp::from_nanos(3),
            rhs: Timestamp::from_nanos(4),
        })
    );
}

// --- Inversion ----------------------------------------------------------

#[test]
fn inverse_swaps_the_labels_and_negates_a_pure_translation() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 2.0, 3.0),
        Rotation::IDENTITY,
        at(0),
    )
    .unwrap();

    let t_b_a = t_a_b.inverse().unwrap();

    assert_eq!(t_b_a.translation(), Vector3::new(-1.0, -2.0, -3.0));
    assert_eq!(t_b_a.rotation(), Rotation::IDENTITY);
    assert_eq!(t_b_a.target(), "b");
    assert_eq!(t_b_a.source(), "a");
    // The operand is not consumed.
    assert_eq!(t_a_b.target(), "a");
}

#[test]
fn inverse_keeps_the_stamp() {
    for stamp in [at(0), at(-7), at(i64::MAX), Stamp::Static] {
        let transform = edge("a", "b", stamp);

        assert_eq!(transform.inverse().unwrap().stamp(), stamp);
    }
}

#[test]
fn inverse_delegates_the_numbers_to_the_isometry() {
    let transform = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 2.0, 3.0),
        quarter_turn_z(),
        at(0),
    )
    .unwrap();

    let inverse = transform.inverse().unwrap();

    assert_eq!(inverse.isometry(), transform.isometry().inverse().unwrap());
    assert_eq!(inverse.rotation(), quarter_turn_z().conjugate());
    // The origin of `a` seen from `b`: (-1, -2, -3) turned back a quarter.
    assert_vector_close(
        inverse.translation(),
        Vector3::new(-2.0, 1.0, -3.0),
        4.0 * f64::EPSILON,
    );
}

#[test]
fn compose_with_inverse_is_a_self_named_identity_within_rounding() {
    let t_a_b = Transform::new(
        "a",
        "b",
        Vector3::new(1.0, 2.0, 3.0),
        Rotation::from_wxyz(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0).unwrap(),
        at(0),
    )
    .unwrap();

    let t_b_a = t_a_b.inverse().unwrap();
    let result = t_a_b.compose(&t_b_a).unwrap();

    assert_eq!(result.target(), "a");
    assert_eq!(result.source(), "a");
    assert_vector_close(result.translation(), ORIGIN, 8.0 * 3.0 * f64::EPSILON);
    assert_abs_diff_eq!(result.rotation().w(), 1.0, epsilon = 4.0 * f64::EPSILON);
    assert_abs_diff_eq!(result.rotation().x(), 0.0, epsilon = 4.0 * f64::EPSILON);

    let other_way = t_b_a.compose(&t_a_b).unwrap();
    assert_eq!(other_way.target(), "b");
    assert_eq!(other_way.source(), "b");
    assert_vector_close(other_way.translation(), ORIGIN, 8.0 * 3.0 * f64::EPSILON);
}

#[test]
fn inverse_of_a_rotation_that_arrived_off_the_unit_sphere_does_not_scale() {
    // A rotation that was transmitted as `f32` and widened back arrives a
    // whisker off the unit sphere. It is brought to unit length where it
    // is built, so no drift is left to scale the inverted translation.
    let drifted = Rotation::from_wxyz(1.0 - 9e-7, 0.0, 0.0, 0.0).unwrap();
    let transform = Transform::new("a", "b", Vector3::new(1.0, 2.0, 3.0), drifted, at(0)).unwrap();
    assert_abs_diff_eq!(transform.rotation().w(), 1.0, epsilon = f64::EPSILON);

    let inverted = transform.inverse().unwrap();

    assert_vector_close(
        inverted.translation(),
        Vector3::new(-1.0, -2.0, -3.0),
        4.0 * f64::EPSILON,
    );
}

#[test]
fn inverse_rejects_overflow_in_any_single_translation_component() {
    // A finite, valid input can overflow one output axis while the other
    // two stay finite.
    let cosine = 0.923_879_532_511_286_7;
    let sine = 0.382_683_432_365_089_8;
    for (translation, rotation) in [
        (
            Vector3::new(1.28e308, 1.28e308, 0.0),
            Rotation::from_wxyz(cosine, 0.0, 0.0, sine).unwrap(),
        ),
        (
            Vector3::new(0.0, 1.28e308, 1.28e308),
            Rotation::from_wxyz(cosine, sine, 0.0, 0.0).unwrap(),
        ),
        (
            Vector3::new(1.28e308, 0.0, 1.28e308),
            Rotation::from_wxyz(cosine, 0.0, sine, 0.0).unwrap(),
        ),
    ] {
        let transform = Transform::new("a", "b", translation, rotation, at(0)).unwrap();

        assert_eq!(transform.inverse(), Err(TransformError::NonFinite));
    }
}

#[test]
fn a_pose_error_is_a_self_named_transform_that_is_not_the_identity() {
    fn pose_error(
        estimate: &Transform,
        truth: &Transform,
    ) -> Result<Transform, TransformError> {
        estimate.inverse()?.compose(truth)
    }
    let estimate = Transform::new(
        "map",
        "base",
        Vector3::new(1.0, 2.0, 0.0),
        Rotation::IDENTITY,
        at(10),
    )
    .unwrap();
    let truth = Transform::new(
        "map",
        "base",
        Vector3::new(1.5, 2.0, 0.0),
        quarter_turn_z(),
        at(10),
    )
    .unwrap();

    let error = pose_error(&estimate, &truth).unwrap();

    assert_eq!(error.target(), "base");
    assert_eq!(error.source(), "base");
    assert_eq!(error.stamp(), at(10));
    assert_eq!(error.translation(), Vector3::new(0.5, 0.0, 0.0));
    assert_eq!(error.rotation(), quarter_turn_z());
    assert_ne!(
        error,
        Transform::new("base", "base", ORIGIN, Rotation::IDENTITY, at(10)).unwrap()
    );

    // An estimate that matches the truth has the identity as its error.
    let none = pose_error(&truth, &truth).unwrap();
    assert_eq!(none.target(), "base");
    assert_eq!(none.source(), "base");
    assert_vector_close(none.translation(), ORIGIN, 8.0 * f64::EPSILON);
}

// --- Trait bounds and error texts ---------------------------------------

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

fn assert_error<T: core::error::Error + Clone + PartialEq + Send + Sync + 'static>() {}

#[test]
fn every_public_type_is_send_sync_and_static() {
    assert_send_sync_static::<Vector3>();
    assert_send_sync_static::<Rotation>();
    assert_send_sync_static::<Transform>();
    assert_send_sync_static::<CrossTimeTransform>();
    assert_send_sync_static::<Timestamp>();
    assert_send_sync_static::<Stamp>();
    assert_send_sync_static::<Registry>();
    assert_send_sync_static::<TransformError>();
    assert_send_sync_static::<InsertError>();
    assert_send_sync_static::<BatchError>();
    assert_send_sync_static::<LookupError>();
}

#[test]
fn every_error_is_an_error_that_clones_and_compares() {
    assert_error::<TransformError>();
    assert_error::<InsertError>();
    assert_error::<BatchError>();
    assert_error::<LookupError>();
}

#[test]
fn value_types_have_the_derives_the_design_lists() {
    fn copy_eq_debug<T: Copy + PartialEq + core::fmt::Debug>() {}
    fn clone_eq_debug<T: Clone + PartialEq + core::fmt::Debug>() {}
    fn ordered_hashed<T: Ord + core::hash::Hash + core::fmt::Display>() {}
    fn eq_hashed<T: Eq + core::hash::Hash>() {}

    copy_eq_debug::<Vector3>();
    copy_eq_debug::<Rotation>();
    copy_eq_debug::<Timestamp>();
    copy_eq_debug::<Stamp>();
    clone_eq_debug::<Transform>();
    clone_eq_debug::<CrossTimeTransform>();
    ordered_hashed::<Timestamp>();
    eq_hashed::<Stamp>();
}

#[test]
fn transform_error_messages_are_exact() {
    let cases = [
        (
            TransformError::NonFinite,
            "transform contains a non-finite value",
        ),
        (
            TransformError::NonUnitRotation { norm: 1.01 },
            "rotation with norm 1.01 is not a unit quaternion",
        ),
        (
            TransformError::NonUnitRotation {
                norm: f64::INFINITY,
            },
            "rotation with norm inf is not a unit quaternion",
        ),
        (TransformError::EmptyFrameName, "frame name is empty"),
        (
            TransformError::FrameMismatch {
                expected: "base".into(),
                found: "lidar".into(),
            },
            "frame lidar does not match expected frame base",
        ),
        (
            TransformError::StampMismatch {
                lhs: Timestamp::from_nanos(1_000_000_000),
                rhs: Timestamp::from_nanos(-500_000_000),
            },
            "stamps 1.000000000 and -0.500000000 differ",
        ),
    ];

    for (error, expected) in cases {
        let text = error.to_string();

        assert_eq!(text, expected);
        // Lowercase, no trailing period, one line.
        assert_eq!(text, text.to_lowercase());
        assert!(!text.ends_with('.'), "{text}");
        assert!(!text.contains('\n'), "{text}");
    }
}

#[test]
fn transform_error_has_no_source_and_compares_by_payload() {
    use core::error::Error;

    let mismatch = TransformError::FrameMismatch {
        expected: "b".into(),
        found: "c".into(),
    };

    assert!(mismatch.source().is_none());
    assert_eq!(mismatch.clone(), mismatch);
    assert_ne!(
        mismatch,
        TransformError::FrameMismatch {
            expected: "c".into(),
            found: "b".into(),
        }
    );
    assert_ne!(TransformError::NonFinite, TransformError::EmptyFrameName);
    assert_ne!(
        TransformError::NonUnitRotation { norm: 1.01 },
        TransformError::NonUnitRotation { norm: 0.99 }
    );
}
