//! The public API as a downstream crate sees it.
//!
//! Everything here goes through `use transforms::{..}` only. Error variants
//! are `#[non_exhaustive]`, so the assertions bind their fields in patterns
//! with `..` instead of constructing expected values.

use core::{
    fmt::{Debug, Display},
    hash::Hash,
    time::Duration,
};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashSet},
};
use transforms::{
    BatchError, CrossTimeTransform, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp,
    Transform, TransformError, Vector3,
};

const ZERO: Vector3 = Vector3::new(0.0, 0.0, 0.0);
const SECOND: i64 = 1_000_000_000;

fn ts(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

fn at(nanos: i64) -> Stamp {
    Stamp::At(ts(nanos))
}

/// An edge with no rotation whose child sits at `(x, 0, 0)` in its parent.
fn edge(
    target: &str,
    source: &str,
    x: f64,
    stamp: Stamp,
) -> Transform {
    Transform::new(
        target,
        source,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        stamp,
    )
    .unwrap()
}

/// A registry holding every given edge, inserted in order.
fn registry_of(edges: &[(&str, &str, f64, Stamp)]) -> Registry {
    let mut registry = Registry::unbounded();
    for &(target, source, x, stamp) in edges {
        registry.insert(&edge(target, source, x, stamp)).unwrap();
    }
    registry
}

/// The 2.4 compatibility fixture: a signed zero in the translation and a
/// rotation that is inside the input tolerance and outside the unit band.
fn sample(
    target: &str,
    source: &str,
    nanos: i64,
) -> Transform {
    Transform::new(
        target,
        source,
        Vector3::new(-0.0, 2.0, -3.0),
        Rotation::from_wxyz(1.0 - 9e-7, 0.0, 0.0, 0.0).unwrap(),
        at(nanos),
    )
    .unwrap()
}

// ---- ported from the 2.4 `tests/tests.rs` ----

#[test]
fn interpolated_chain_lookup_matches_the_expected_transform() {
    let t = ts(SECOND);
    let after = |millis| Stamp::At(t.checked_add(Duration::from_millis(millis)).unwrap());
    let moved = |target: &str, source: &str, x, y, stamp| {
        Transform::new(
            target,
            source,
            Vector3::new(x, y, 0.0),
            Rotation::IDENTITY,
            stamp,
        )
        .unwrap()
    };

    let mut registry = Registry::unbounded();
    // Frame b moves from x = 1 m to x = 2 m over one second.
    registry
        .insert(&moved("a", "b", 1.0, 0.0, Stamp::At(t)))
        .unwrap();
    registry
        .insert(&moved("a", "b", 2.0, 0.0, after(1_000)))
        .unwrap();
    // Frame c moves from y = 1 m to y = 2 m, half a second later.
    registry
        .insert(&moved("b", "c", 0.0, 1.0, after(500)))
        .unwrap();
    registry
        .insert(&moved("b", "c", 0.0, 2.0, after(1_500)))
        .unwrap();

    let middle = t.checked_add(Duration::from_millis(750)).unwrap();
    let expected = moved("a", "c", 1.75, 1.25, Stamp::At(middle));

    assert_eq!(registry.lookup("a", "c", middle), Ok(expected));
}

#[test]
fn lookup_outside_an_edge_coverage_reports_the_frame_and_its_range() {
    let t = ts(SECOND);
    let after = |secs| t.checked_add(Duration::from_secs(secs)).unwrap();
    let mut registry = Registry::unbounded();
    registry.insert(&edge("a", "b", 1.0, Stamp::At(t))).unwrap();
    registry
        .insert(&edge("a", "b", 2.0, Stamp::At(after(1))))
        .unwrap();
    registry
        .insert(&edge("b", "c", 1.0, Stamp::At(after(2))))
        .unwrap();
    registry
        .insert(&edge("b", "c", 2.0, Stamp::At(after(3))))
        .unwrap();

    // The b -> c edge covers [t + 2 s, t + 3 s]. One `match` reaches the
    // frame of the failing edge, the instant asked for and the covered
    // range, all as exact timestamps.
    match registry.lookup("a", "c", t) {
        Err(LookupError::OutOfRange {
            frame,
            requested,
            earliest,
            latest,
            ..
        }) => {
            assert_eq!(frame, "c");
            assert_eq!(requested, t);
            assert_eq!(earliest, after(2));
            assert_eq!(latest, after(3));
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

// ---- ported from the 2.4 `tests/v2_compatibility.rs` ----

#[test]
fn disconnected_frames_report_disconnected_before_missing_coverage() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", 1)).unwrap();
    registry.insert(&sample("x", "y", 1)).unwrap();

    // Neither edge serves t = 2. The topology decides first.
    let lookup = registry.lookup("b", "y", ts(2));
    assert!(
        matches!(&lookup, Err(LookupError::Disconnected { a, b, .. }) if a == "b" && b == "y"),
        "{lookup:?}"
    );
    let latest = registry.latest_common_time("b", "y");
    assert!(
        matches!(&latest, Err(LookupError::Disconnected { a, b, .. }) if a == "b" && b == "y"),
        "{latest:?}"
    );
}

#[test]
fn failed_lookup_names_the_failing_edge_on_the_connecting_chain() {
    let mut registry = Registry::unbounded();
    for (target, source, nanos) in [
        ("root", "common", 1),
        ("common", "target", 2),
        ("common", "source", 1),
    ] {
        registry.insert(&sample(target, source, nanos)).unwrap();
    }
    // The edge above `common` cannot serve t = 2 either, and the chain
    // between the endpoints never crosses it.
    let found = registry.lookup("target", "source", ts(2));
    assert!(
        matches!(
            &found,
            Err(LookupError::OutOfRange { frame, requested, earliest, latest, .. })
                if frame == "source"
                    && *requested == ts(2)
                    && *earliest == ts(1)
                    && *latest == ts(1)
        ),
        "{found:?}"
    );
}

#[test]
fn exact_sample_preserves_signed_zero_and_holds_a_normalized_rotation() {
    let stored = sample("parent", "child", 7);
    let mut registry = Registry::unbounded();
    registry.insert(&stored).unwrap();

    let found = registry.lookup("parent", "child", ts(7)).unwrap();

    assert_eq!(found, stored);
    assert_eq!(found.translation().x.to_bits(), (-0.0_f64).to_bits());
    assert_eq!(
        found.rotation().w().to_bits(),
        stored.rotation().w().to_bits()
    );

    // 2.4 stored the off-unit `w` as given. The constructor now scales it
    // into the unit band.
    let w = found.rotation().w();
    assert_ne!(w.to_bits(), (1.0_f64 - 9e-7).to_bits());
    assert!((w * w - 1.0).abs() <= 8.0 * f64::EPSILON, "{w}");
}

#[test]
fn attaching_an_existing_root_changes_descendant_depth() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("root", "leaf", 1)).unwrap();
    registry.insert(&sample("world", "root", 1)).unwrap();

    let found = registry.lookup("world", "leaf", ts(1)).unwrap();
    assert_eq!(found.target(), "world");
    assert_eq!(found.source(), "leaf");

    assert!(registry.remove_frame("root"));
    let detached = registry.lookup("world", "leaf", ts(1));
    assert!(
        matches!(&detached, Err(LookupError::UnknownFrame { frame, .. }) if frame == "world"),
        "{detached:?}"
    );
    assert!(registry.lookup("root", "leaf", ts(1)).is_ok());
}

#[test]
fn unbounded_registry_interpolates_across_long_gaps() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("parent", "child", 0)).unwrap();
    registry
        .insert(&sample("parent", "child", 30 * SECOND))
        .unwrap();

    assert!(registry.lookup("parent", "child", ts(15 * SECOND)).is_ok());
}

#[test]
fn composing_with_a_self_named_identity_returns_the_left_operand() {
    // 2.4 rejected this as `SameFrameMultiplication`. A self-named
    // transform is now an ordinary operand.
    let lhs = sample("parent", "child", 1);
    let identity = Transform::new("child", "child", ZERO, Rotation::IDENTITY, at(1)).unwrap();

    let composed = lhs.compose(&identity).unwrap();

    assert_eq!(composed, lhs);
}

#[test]
fn cross_time_result_carries_both_instants_and_is_not_a_transform() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("world", "source", 1)).unwrap();
    registry.insert(&sample("world", "target", 2)).unwrap();

    let found: CrossTimeTransform = registry
        .lookup_across("target", ts(2), "source", ts(1), "world")
        .unwrap();

    assert_eq!(found.target(), "target");
    assert_eq!(found.target_time(), ts(2));
    assert_eq!(found.source(), "source");
    assert_eq!(found.source_time(), ts(1));

    // 2.4 returned an insertable `Transform` stamped with the target time.
    // Storing the result is now a deliberate act: the caller builds the
    // transform and chooses the stamp.
    let rebuilt = Transform::new(
        found.target(),
        found.source(),
        found.translation(),
        found.rotation(),
        Stamp::At(found.target_time()),
    )
    .unwrap();
    let mut receiver = Registry::unbounded();
    receiver.insert(&rebuilt).unwrap();
    assert_eq!(receiver.lookup("target", "source", ts(2)), Ok(rebuilt));
}

#[test]
fn constant_dynamic_reference_is_accepted_as_a_fixed_frame() {
    let mut registry = Registry::unbounded();
    for (target, source, nanos) in [
        ("root", "reference", 1),
        ("root", "reference", 2),
        ("reference", "source", 1),
        ("reference", "target", 2),
    ] {
        registry.insert(&sample(target, source, nanos)).unwrap();
    }

    assert!(
        registry
            .lookup_across("target", ts(2), "source", ts(1), "reference")
            .is_ok()
    );
}

// ---- trait bounds and const usability ----

#[test]
fn public_types_satisfy_their_documented_trait_bounds() {
    fn shareable<X: Send + Sync + 'static>() {}
    fn error<
        X: core::error::Error + Clone + PartialEq + Debug + Display + Send + Sync + 'static,
    >() {
    }
    fn instant<X: Ord + Hash + Display + Debug + Copy>() {}
    fn stamp<X: Eq + Hash + Debug + Copy>() {}
    fn value<X: Clone + PartialEq + Debug>() {}
    fn plain<X: Copy + PartialEq + Debug>() {}
    fn debug<X: Debug>() {}

    shareable::<Vector3>();
    shareable::<Rotation>();
    shareable::<Transform>();
    shareable::<CrossTimeTransform>();
    shareable::<Timestamp>();
    shareable::<Stamp>();
    shareable::<Registry>();

    error::<TransformError>();
    error::<InsertError>();
    error::<BatchError>();
    error::<LookupError>();

    instant::<Timestamp>();
    stamp::<Stamp>();
    value::<Transform>();
    value::<CrossTimeTransform>();
    plain::<Vector3>();
    plain::<Rotation>();
    debug::<Registry>();
}

#[test]
fn registry_formats_with_debug() {
    let registry = registry_of(&[("map", "base", 1.0, at(1))]);
    let printed = format!("{registry:?}");
    assert!(printed.starts_with("Registry"), "{printed}");
}

const CONST_VECTOR: Vector3 = Vector3::new(1.0, 2.0, 3.0);
const CONST_INSTANT: Timestamp = Timestamp::from_nanos(-5);
const CONST_NANOS: i64 = CONST_INSTANT.as_nanos();
const CONST_ROTATION: Rotation = Rotation::IDENTITY;
const CONST_CUTOFF: Timestamp = Timestamp::MAX;
const CONST_MAX_NANOS: i64 = Timestamp::MAX.as_nanos();

#[test]
fn promised_const_items_are_usable_in_const_context() {
    assert_eq!(CONST_VECTOR, Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(CONST_INSTANT, ts(-5));
    assert_eq!(CONST_NANOS, -5);
    assert_eq!(
        CONST_ROTATION,
        Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0).unwrap()
    );
    assert_eq!(CONST_CUTOFF, ts(i64::MAX));
    assert_eq!(CONST_MAX_NANOS, i64::MAX);
}

#[test]
fn timestamp_and_stamp_work_as_set_keys() {
    let instants: BTreeSet<Timestamp> = [ts(3), ts(-1), ts(3), ts(0)].into_iter().collect();
    assert_eq!(
        instants.into_iter().collect::<Vec<_>>(),
        [ts(-1), ts(0), ts(3)]
    );

    let stamps: HashSet<Stamp> = [Stamp::Static, at(1), at(1), Stamp::Static, at(2)]
        .into_iter()
        .collect();
    assert_eq!(stamps.len(), 3);
    assert!(stamps.contains(&Stamp::Static));

    let hashed: HashSet<Timestamp> = [ts(1), ts(1)].into_iter().collect();
    assert_eq!(hashed.len(), 1);
}

// ---- geometry ----

#[test]
fn vector3_exposes_its_three_components() {
    let mut v = Vector3::new(1.0, -2.0, 3.5);
    assert_eq!((v.x, v.y, v.z), (1.0, -2.0, 3.5));
    v.z = 4.0;
    assert_eq!(v, Vector3::new(1.0, -2.0, 4.0));
    assert_ne!(v, CONST_VECTOR);
}

#[test]
fn rotation_identity_has_scalar_one_and_rotates_nothing() {
    let identity = Rotation::IDENTITY;
    assert_eq!(
        (identity.w(), identity.x(), identity.y(), identity.z()),
        (1.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(identity.rotate(CONST_VECTOR), CONST_VECTOR);
    assert_eq!(
        format!("{identity:?}"),
        "Rotation { w: 1.0, x: 0.0, y: 0.0, z: 0.0 }"
    );
}

#[test]
fn rotation_from_wxyz_reads_components_scalar_first() {
    // A half turn about x.
    let half_turn = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();
    assert_eq!(
        (half_turn.w(), half_turn.x(), half_turn.y(), half_turn.z()),
        (0.0, 1.0, 0.0, 0.0)
    );
    assert_eq!(
        half_turn.rotate(Vector3::new(1.0, 2.0, 3.0)),
        Vector3::new(1.0, -2.0, -3.0)
    );
}

#[test]
fn rotation_from_wxyz_keeps_unit_input_bit_for_bit() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let quarter_turn = Rotation::from_wxyz(h, 0.0, 0.0, h).unwrap();
    assert_eq!(quarter_turn.w().to_bits(), h.to_bits());
    assert_eq!(quarter_turn.z().to_bits(), h.to_bits());
}

#[test]
fn rotation_from_wxyz_scales_an_f32_quaternion_to_unit_length() {
    let widened = f64::from(core::f32::consts::FRAC_1_SQRT_2);
    let rotation = Rotation::from_wxyz(widened, widened, 0.0, 0.0).unwrap();

    assert_ne!(rotation.w(), widened);
    let n2 = rotation.w() * rotation.w()
        + rotation.x() * rotation.x()
        + rotation.y() * rotation.y()
        + rotation.z() * rotation.z();
    assert!((n2 - 1.0).abs() <= 8.0 * f64::EPSILON, "{n2}");
}

#[test]
fn rotation_from_wxyz_rejects_non_finite_and_off_unit_input() {
    for components in [
        [f64::NAN, 0.0, 0.0, 0.0],
        [1.0, f64::INFINITY, 0.0, 0.0],
        [1.0, 0.0, f64::NEG_INFINITY, 0.0],
        [1.0, 0.0, 0.0, f64::NAN],
    ] {
        let [w, x, y, z] = components;
        assert_eq!(
            Rotation::from_wxyz(w, x, y, z),
            Err(TransformError::NonFinite)
        );
    }

    let doubled = Rotation::from_wxyz(2.0, 0.0, 0.0, 0.0);
    assert!(
        matches!(doubled, Err(TransformError::NonUnitRotation { norm, .. }) if norm == 2.0),
        "{doubled:?}"
    );
    let zero = Rotation::from_wxyz(0.0, 0.0, 0.0, 0.0);
    assert!(
        matches!(zero, Err(TransformError::NonUnitRotation { norm, .. }) if norm == 0.0),
        "{zero:?}"
    );
    // Just outside the 1e-6 input tolerance, on either side.
    for w in [1.0 + 2e-6, 1.0 - 2e-6] {
        assert!(matches!(
            Rotation::from_wxyz(w, 0.0, 0.0, 0.0),
            Err(TransformError::NonUnitRotation { .. })
        ));
    }
    // Finite components whose squared norm overflows.
    let huge = Rotation::from_wxyz(1e200, 0.0, 0.0, 0.0);
    assert!(
        matches!(huge, Err(TransformError::NonUnitRotation { norm, .. }) if norm.is_infinite()),
        "{huge:?}"
    );
}

#[test]
fn transform_new_accepts_borrowed_and_owned_names() {
    // Adapter sketch 1: every common string shape is a frame name.
    let owned = String::from("owned");
    let referenced = String::from("referenced");
    let cow: Cow<'_, str> = Cow::Borrowed("cow");
    let boxed: Box<str> = "boxed".into();
    let rotation = Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0).unwrap();

    let a = Transform::new("borrowed", owned, ZERO, rotation, Stamp::Static).unwrap();
    assert_eq!((a.target(), a.source()), ("borrowed", "owned"));

    let b = Transform::new(&referenced, cow, ZERO, Rotation::IDENTITY, at(0)).unwrap();
    assert_eq!((b.target(), b.source()), ("referenced", "cow"));
    // The borrowed `String` is still the caller's.
    assert_eq!(referenced, "referenced");

    let c = Transform::new(boxed, 'c', ZERO, rotation, Stamp::Static).unwrap();
    assert_eq!((c.target(), c.source()), ("boxed", "c"));
}

#[test]
fn transform_accessors_return_what_was_built() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let rotation = Rotation::from_wxyz(h, 0.0, h, 0.0).unwrap();
    let tf = Transform::new("map", "base", CONST_VECTOR, rotation, at(42)).unwrap();

    assert_eq!(tf.target(), "map");
    assert_eq!(tf.source(), "base");
    assert_eq!(tf.translation(), CONST_VECTOR);
    assert_eq!(tf.rotation(), rotation);
    assert_eq!(tf.stamp(), at(42));
    assert_eq!(tf.clone(), tf);
    assert!(format!("{tf:?}").contains("map"));

    let fixed = Transform::new("map", "base", CONST_VECTOR, rotation, Stamp::Static).unwrap();
    assert_eq!(fixed.stamp(), Stamp::Static);
    assert_ne!(fixed, tf);
}

#[test]
fn transform_new_rejects_an_empty_name_before_a_non_finite_translation() {
    let bad = Vector3::new(f64::NAN, 0.0, 0.0);
    for (target, source) in [("", "b"), ("a", ""), ("", "")] {
        assert_eq!(
            Transform::new(target, source, ZERO, Rotation::IDENTITY, Stamp::Static),
            Err(TransformError::EmptyFrameName)
        );
        assert_eq!(
            Transform::new(target, source, bad, Rotation::IDENTITY, Stamp::Static),
            Err(TransformError::EmptyFrameName)
        );
    }
}

#[test]
fn transform_new_rejects_a_non_finite_translation() {
    for translation in [
        Vector3::new(f64::NAN, 0.0, 0.0),
        Vector3::new(0.0, f64::INFINITY, 0.0),
        Vector3::new(0.0, 0.0, f64::NEG_INFINITY),
    ] {
        assert_eq!(
            Transform::new("a", "b", translation, Rotation::IDENTITY, at(0)),
            Err(TransformError::NonFinite)
        );
    }
    // No magnitude bound: the largest finite value is a translation.
    assert!(
        Transform::new(
            "a",
            "b",
            Vector3::new(f64::MAX, f64::MIN, 0.0),
            Rotation::IDENTITY,
            at(0)
        )
        .is_ok()
    );
}

#[test]
fn transform_new_accepts_equal_names_with_any_geometry() {
    let offset = Transform::new("a", "a", CONST_VECTOR, Rotation::IDENTITY, at(0)).unwrap();
    assert_eq!(offset.target(), offset.source());
    assert_eq!(offset.translation(), CONST_VECTOR);
}

#[test]
fn compose_chains_frames_and_adds_translations() {
    let map_from_base = edge("map", "base", 1.0, at(5));
    let base_from_lidar = edge("base", "lidar", 2.0, at(5));

    let map_from_lidar = map_from_base.compose(&base_from_lidar).unwrap();

    assert_eq!(map_from_lidar, edge("map", "lidar", 3.0, at(5)));
    // The operands are borrowed and still usable, and calls chain.
    let lidar_from_tip = edge("lidar", "tip", 4.0, at(5));
    let chained = map_from_base
        .compose(&base_from_lidar)
        .unwrap()
        .compose(&lidar_from_tip)
        .unwrap();
    assert_eq!(chained, edge("map", "tip", 7.0, at(5)));
}

#[test]
fn compose_rotates_the_right_hand_translation_into_the_left_hand_frame() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let yaw = Rotation::from_wxyz(h, 0.0, 0.0, h).unwrap();
    let map_from_base = Transform::new(
        "map",
        "base",
        Vector3::new(2.0, 1.0, 0.0),
        yaw,
        Stamp::Static,
    )
    .unwrap();
    let base_from_tip = edge("base", "tip", 1.0, Stamp::Static);

    let map_from_tip = map_from_base.compose(&base_from_tip).unwrap();

    // One metre ahead of a robot that faces +y.
    let t = map_from_tip.translation();
    approx::assert_abs_diff_eq!(t.x, 2.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(t.y, 2.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(t.z, 0.0, epsilon = 1e-12);
    assert_eq!(map_from_tip.rotation(), yaw);
}

#[test]
fn compose_reports_frames_that_do_not_chain() {
    let map_from_base = edge("map", "base", 1.0, at(1));
    let odom_from_lidar = edge("odom", "lidar", 1.0, at(2));

    // The frame check comes before the stamp check.
    let mismatch = map_from_base.compose(&odom_from_lidar);
    assert!(
        matches!(
            &mismatch,
            Err(TransformError::FrameMismatch { expected, found, .. })
                if expected == "base" && found == "odom"
        ),
        "{mismatch:?}"
    );

    // The reversed operand order does not chain either.
    let base_from_lidar = edge("base", "lidar", 1.0, at(1));
    let reversed = base_from_lidar.compose(&map_from_base);
    assert!(
        matches!(
            &reversed,
            Err(TransformError::FrameMismatch { expected, found, .. })
                if expected == "lidar" && found == "map"
        ),
        "{reversed:?}"
    );
}

#[test]
fn compose_requires_equal_dynamic_stamps_and_lets_static_adopt() {
    let dynamic = edge("a", "b", 1.0, at(1));
    let later = edge("b", "c", 1.0, at(2));
    let fixed = edge("b", "c", 1.0, Stamp::Static);
    let fixed_above = edge("z", "a", 1.0, Stamp::Static);

    let mismatch = dynamic.compose(&later);
    assert!(
        matches!(
            mismatch,
            Err(TransformError::StampMismatch { lhs, rhs, .. }) if lhs == ts(1) && rhs == ts(2)
        ),
        "{mismatch:?}"
    );

    assert_eq!(dynamic.compose(&fixed).unwrap().stamp(), at(1));
    assert_eq!(fixed_above.compose(&dynamic).unwrap().stamp(), at(1));
    assert_eq!(
        fixed_above
            .compose(&edge("a", "b", 1.0, Stamp::Static))
            .unwrap()
            .stamp(),
        Stamp::Static
    );
}

#[test]
fn compose_reports_an_overflowing_translation() {
    let far = edge("a", "b", f64::MAX, Stamp::Static);
    let farther = edge("b", "c", f64::MAX, Stamp::Static);
    assert_eq!(far.compose(&farther), Err(TransformError::NonFinite));
}

#[test]
fn inverse_swaps_the_frames_and_keeps_the_stamp() {
    let map_from_base =
        Transform::new("map", "base", CONST_VECTOR, Rotation::IDENTITY, at(9)).unwrap();

    let base_from_map = map_from_base.inverse().unwrap();

    assert_eq!(base_from_map.target(), "base");
    assert_eq!(base_from_map.source(), "map");
    assert_eq!(base_from_map.stamp(), at(9));
    assert_eq!(base_from_map.translation(), Vector3::new(-1.0, -2.0, -3.0));
    assert_eq!(base_from_map.rotation(), Rotation::IDENTITY);
    assert_eq!(base_from_map.inverse(), Ok(map_from_base));

    let fixed = edge("a", "b", 1.0, Stamp::Static).inverse().unwrap();
    assert_eq!(fixed.stamp(), Stamp::Static);
}

#[test]
fn inverse_conjugates_the_rotation() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let yaw = Rotation::from_wxyz(h, 0.0, 0.0, h).unwrap();
    let tf = Transform::new("map", "base", Vector3::new(2.0, 1.0, 0.0), yaw, at(0)).unwrap();

    let inverse = tf.inverse().unwrap();

    let r = inverse.rotation();
    assert_eq!((r.w(), r.x(), r.y(), r.z()), (h, 0.0, 0.0, -h));
    // The map origin seen from the robot: -R^T t.
    let t = inverse.translation();
    approx::assert_abs_diff_eq!(t.x, -1.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(t.y, 2.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(t.z, 0.0, epsilon = 1e-12);
}

#[test]
fn inverse_reports_an_overflowing_translation() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let yaw = Rotation::from_wxyz(h, 0.0, 0.0, h).unwrap();
    let far = Transform::new(
        "a",
        "b",
        Vector3::new(f64::MAX, f64::MAX, 0.0),
        yaw,
        Stamp::Static,
    )
    .unwrap();
    assert_eq!(far.inverse(), Err(TransformError::NonFinite));
}

#[test]
fn a_pose_error_is_an_ordinary_self_named_transform() {
    let estimate = edge("map", "base", 1.0, at(3));
    let truth = edge("map", "base", 1.5, at(3));

    let error = estimate.inverse().unwrap().compose(&truth).unwrap();

    assert_eq!(error.target(), "base");
    assert_eq!(error.source(), "base");
    assert_eq!(error.translation(), Vector3::new(0.5, 0.0, 0.0));
    assert_eq!(error.stamp(), at(3));
}

// ---- insert ----

#[test]
fn insert_borrows_the_transform_and_stores_the_edge() {
    let tf = Transform::new("map", "base", CONST_VECTOR, Rotation::IDENTITY, at(1)).unwrap();
    let mut registry = Registry::unbounded();

    registry.insert(&tf).unwrap();

    // The transform is still the caller's, and a direct lookup at the
    // stored instant returns it.
    assert_eq!(registry.lookup("map", "base", ts(1)), Ok(tf));
}

#[test]
fn insert_at_a_stored_instant_replaces_the_sample() {
    let mut registry = registry_of(&[("map", "base", 1.0, at(1)), ("map", "base", 5.0, at(1))]);
    assert_eq!(
        registry.lookup("map", "base", ts(1)),
        Ok(edge("map", "base", 5.0, at(1)))
    );
    // Still one sample: the instant next to it is not covered.
    assert!(matches!(
        registry.lookup("map", "base", ts(2)),
        Err(LookupError::OutOfRange { .. })
    ));
    registry.insert(&edge("map", "base", 9.0, at(1))).unwrap();
    assert_eq!(
        registry.lookup("map", "base", ts(1)).unwrap().translation(),
        Vector3::new(9.0, 0.0, 0.0)
    );
}

#[test]
fn insert_of_a_static_transform_replaces_the_pose_for_every_instant() {
    let registry = registry_of(&[
        ("base", "camera", 1.0, Stamp::Static),
        ("base", "camera", 2.0, Stamp::Static),
    ]);
    for nanos in [i64::MIN, -1, 0, 7, i64::MAX] {
        assert_eq!(
            registry.lookup("base", "camera", ts(nanos)),
            Ok(edge("base", "camera", 2.0, at(nanos)))
        );
    }
}

#[test]
fn insert_rejects_a_self_named_transform_as_self_edge() {
    let mut registry = Registry::unbounded();
    for stamp in [Stamp::Static, at(1)] {
        let rejected = registry.insert(&edge("a", "a", 0.0, stamp));
        assert!(
            matches!(&rejected, Err(InsertError::SelfEdge { frame, .. }) if frame == "a"),
            "{rejected:?}"
        );
    }
    // Also for a frame that already has a parent.
    registry.insert(&edge("root", "a", 1.0, at(1))).unwrap();
    assert!(matches!(
        registry.insert(&edge("a", "a", 0.0, at(1))),
        Err(InsertError::SelfEdge { .. })
    ));
}

#[test]
fn insert_rejects_a_second_parent_as_parent_mismatch() {
    let mut registry = registry_of(&[("base", "camera", 1.0, at(1))]);

    let rejected = registry.insert(&edge("arm", "camera", 1.0, at(2)));

    assert!(
        matches!(
            &rejected,
            Err(InsertError::ParentMismatch { frame, pinned, offered, .. })
                if frame == "camera" && pinned == "base" && offered == "arm"
        ),
        "{rejected:?}"
    );
    // The rejected parent did not become known.
    assert!(matches!(
        registry.lookup("arm", "camera", ts(1)),
        Err(LookupError::UnknownFrame { .. })
    ));
}

#[test]
fn insert_rejects_a_change_of_kind_in_both_orders() {
    let mut fixed = registry_of(&[("a", "b", 1.0, Stamp::Static)]);
    let rejected = fixed.insert(&edge("a", "b", 2.0, at(5)));
    assert!(
        matches!(
            &rejected,
            Err(InsertError::KindMismatch { frame, incoming, .. })
                if frame == "b" && *incoming == at(5)
        ),
        "{rejected:?}"
    );

    let mut moving = registry_of(&[("a", "b", 2.0, at(5))]);
    let rejected = moving.insert(&edge("a", "b", 1.0, Stamp::Static));
    assert!(
        matches!(
            &rejected,
            Err(InsertError::KindMismatch { frame, incoming, .. })
                if frame == "b" && *incoming == Stamp::Static
        ),
        "{rejected:?}"
    );
    // The stored sample still answers, and only at its instant.
    assert!(moving.lookup("a", "b", ts(5)).is_ok());
    assert!(moving.lookup("a", "b", ts(6)).is_err());
}

#[test]
fn insert_rejects_an_edge_that_would_close_a_cycle() {
    let mut registry = registry_of(&[("a", "b", 1.0, at(1)), ("b", "c", 1.0, at(1))]);

    let rejected = registry.insert(&edge("c", "a", 1.0, at(1)));

    assert!(
        matches!(
            &rejected,
            Err(InsertError::Cycle { child, parent, .. }) if child == "a" && parent == "c"
        ),
        "{rejected:?}"
    );
    assert_eq!(
        registry.lookup("a", "c", ts(1)),
        Ok(edge("a", "c", 2.0, at(1)))
    );
}

#[test]
fn a_failed_insert_makes_no_name_known() {
    let mut registry = registry_of(&[("a", "b", 1.0, at(1))]);

    assert!(registry.insert(&edge("new", "new", 0.0, at(1))).is_err());
    assert!(registry.insert(&edge("other", "b", 0.0, at(1))).is_err());

    for name in ["new", "other"] {
        let found = registry.lookup(name, name, ts(1));
        assert!(
            matches!(&found, Err(LookupError::UnknownFrame { frame, .. }) if frame == name),
            "{found:?}"
        );
    }
}

// ---- retention ----

#[test]
fn bounded_registry_evicts_stored_samples_behind_the_window() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 0.0, at(100 * SECOND)))
        .unwrap();
    registry
        .insert(&edge("a", "b", 5.0, at(200 * SECOND)))
        .unwrap();

    let evicted = registry.lookup("a", "b", ts(100 * SECOND));
    assert!(
        matches!(
            &evicted,
            Err(LookupError::OutOfRange { earliest, latest, .. })
                if *earliest == ts(200 * SECOND) && *latest == ts(200 * SECOND)
        ),
        "{evicted:?}"
    );
}

#[test]
fn bounded_registry_rejects_a_sample_behind_the_window_as_expired() {
    // 2.4 accepted such a sample and evicted it in the same call.
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 0.0, at(100 * SECOND)))
        .unwrap();

    // Exactly on the boundary: accepted.
    registry
        .insert(&edge("a", "b", 1.0, at(90 * SECOND)))
        .unwrap();

    // One nanosecond older: rejected, and nothing is stored.
    let late = edge("a", "b", 2.0, at(90 * SECOND - 1));
    let rejected = registry.insert(&late);
    assert!(
        matches!(
            &rejected,
            Err(InsertError::Expired { frame, stamp, newest, .. })
                if frame == "b" && *stamp == ts(90 * SECOND - 1) && *newest == ts(100 * SECOND)
        ),
        "{rejected:?}"
    );
    let uncovered = registry.lookup("a", "b", ts(90 * SECOND - 1));
    assert!(
        matches!(
            &uncovered,
            Err(LookupError::OutOfRange { earliest, latest, .. })
                if *earliest == ts(90 * SECOND) && *latest == ts(100 * SECOND)
        ),
        "{uncovered:?}"
    );
    // The window is per frame: another frame starts wherever it likes.
    registry.insert(&edge("a", "c", 0.0, at(SECOND))).unwrap();
}

#[test]
fn zero_max_age_keeps_only_the_newest_sample() {
    let mut registry = Registry::with_max_age(Duration::ZERO);
    registry.insert(&edge("a", "b", 1.0, at(5))).unwrap();
    registry.insert(&edge("a", "b", 2.0, at(5))).unwrap();
    assert!(matches!(
        registry.insert(&edge("a", "b", 3.0, at(4))),
        Err(InsertError::Expired { .. })
    ));
    registry.insert(&edge("a", "b", 4.0, at(6))).unwrap();

    assert!(registry.lookup("a", "b", ts(5)).is_err());
    assert_eq!(
        registry.lookup("a", "b", ts(6)),
        Ok(edge("a", "b", 4.0, at(6)))
    );
}

#[test]
fn unbounded_registry_never_reports_expired() {
    let mut registry = Registry::unbounded();
    registry.insert(&edge("a", "b", 1.0, at(i64::MAX))).unwrap();
    registry.insert(&edge("a", "b", 0.0, at(i64::MIN))).unwrap();
    assert!(registry.lookup("a", "b", ts(0)).is_ok());
}

#[test]
fn a_sample_at_the_maximum_instant_anchors_the_window_there() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(Timestamp::MAX)))
        .unwrap();

    let inside = Timestamp::MAX.checked_sub(Duration::from_secs(5)).unwrap();
    let outside = Timestamp::MAX.checked_sub(Duration::from_secs(11)).unwrap();
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(inside)))
        .unwrap();
    let rejected = registry.insert(&edge("a", "b", 0.0, Stamp::At(outside)));
    assert!(
        matches!(
            &rejected,
            Err(InsertError::Expired { newest, .. }) if *newest == Timestamp::MAX
        ),
        "{rejected:?}"
    );

    // Removing that history lifts the anchor.
    registry.remove_samples_through(Timestamp::MAX);
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(outside)))
        .unwrap();
}

#[test]
fn an_unrepresentable_cutoff_rejects_nothing() {
    // newest - max_age lies below the representable range.
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 1.0, at(i64::MIN + 5)))
        .unwrap();
    registry.insert(&edge("a", "b", 0.0, at(i64::MIN))).unwrap();

    assert_eq!(
        registry.lookup("a", "b", ts(i64::MIN)),
        Ok(edge("a", "b", 0.0, at(i64::MIN)))
    );
}

#[test]
fn eviction_spares_static_edges() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 1.0, Stamp::Static))
        .unwrap();
    registry
        .insert(&edge("b", "c", 0.0, at(100 * SECOND)))
        .unwrap();
    registry
        .insert(&edge("b", "c", 5.0, at(200 * SECOND)))
        .unwrap();

    assert!(registry.lookup("b", "c", ts(100 * SECOND)).is_err());
    assert_eq!(
        registry.lookup("a", "c", ts(200 * SECOND)),
        Ok(edge("a", "c", 6.0, at(200 * SECOND)))
    );
    assert_eq!(
        registry.lookup("a", "b", ts(0)),
        Ok(edge("a", "b", 1.0, at(0)))
    );
}

// ---- insert_all ----

#[test]
fn insert_all_stores_what_the_insert_loop_would() {
    let batch = [
        edge("map", "odom", 1.0, at(1)),
        // `odom` enters the tree in this batch.
        edge("odom", "base", 2.0, at(1)),
        edge("base", "lidar", 0.5, Stamp::Static),
        edge("map", "odom", 3.0, at(3)),
        // A repeated instant: the later member wins.
        edge("odom", "base", 4.0, at(1)),
    ];
    let mut batched = Registry::unbounded();
    batched.insert_all(&batch).unwrap();
    let mut looped = Registry::unbounded();
    for member in &batch {
        looped.insert(member).unwrap();
    }

    for (target, source, nanos) in [
        ("map", "odom", 1),
        ("map", "odom", 2),
        ("map", "odom", 3),
        ("map", "lidar", 1),
        ("lidar", "map", 1),
        ("map", "lidar", 2),
        ("base", "lidar", 99),
    ] {
        assert_eq!(
            batched.lookup(target, source, ts(nanos)),
            looped.lookup(target, source, ts(nanos)),
            "{target} <- {source} at {nanos}"
        );
    }
    assert_eq!(
        batched.lookup("map", "lidar", ts(1)),
        Ok(edge("map", "lidar", 5.5, at(1)))
    );
    assert_eq!(
        batched.latest_common_time("map", "lidar"),
        looped.latest_common_time("map", "lidar")
    );
}

#[test]
fn insert_all_accepts_an_empty_batch() {
    let mut registry = Registry::unbounded();
    assert_eq!(registry.insert_all(&[]), Ok(()));
    assert!(registry.lookup("a", "a", ts(0)).is_err());
}

#[test]
fn insert_all_stores_nothing_when_a_member_is_rejected() {
    let mut registry = registry_of(&[("map", "base", 1.0, at(1))]);
    let batch = [
        edge("map", "base", 2.0, at(2)),
        edge("base", "lidar", 1.0, at(2)),
        edge("lidar", "map", 1.0, at(2)),
        edge("x", "x", 1.0, at(2)),
    ];

    let rejected = registry.insert_all(&batch).unwrap_err();

    // The first failure of the loop: member 2 closes a cycle. Member 3
    // would fail as well and is not the one reported.
    let BatchError { index, error } = &rejected;
    assert_eq!(*index, 2);
    assert!(
        matches!(
            error,
            InsertError::Cycle { child, parent, .. } if child == "map" && parent == "lidar"
        ),
        "{error:?}"
    );
    assert_eq!(batch[*index].source(), "map");

    // Nothing was stored and no name became known.
    assert!(matches!(
        registry.lookup("map", "base", ts(2)),
        Err(LookupError::OutOfRange { .. })
    ));
    let lidar = registry.lookup("map", "lidar", ts(2));
    assert!(
        matches!(&lidar, Err(LookupError::UnknownFrame { frame, .. }) if frame == "lidar"),
        "{lidar:?}"
    );
    assert_eq!(
        registry.lookup("map", "base", ts(1)),
        Ok(edge("map", "base", 1.0, at(1)))
    );
}

#[test]
fn batch_error_exposes_its_fields_and_its_source() {
    let mut registry = Registry::unbounded();
    let rejected = registry
        .insert_all(&[edge("a", "b", 1.0, at(1)), edge("c", "b", 1.0, at(1))])
        .unwrap_err();

    assert_eq!(rejected.index, 1);
    assert!(matches!(rejected.error, InsertError::ParentMismatch { .. }));

    let source = std::error::Error::source(&rejected).unwrap();
    let inner = source.downcast_ref::<InsertError>().unwrap();
    assert_eq!(inner, &rejected.error);
    assert_eq!(source.to_string(), rejected.error.to_string());

    // A plain two-field struct: downstream code can build and compare one.
    let rebuilt = BatchError {
        index: 1,
        error: rejected.error.clone(),
    };
    assert_eq!(rebuilt, rejected);
    assert_ne!(
        BatchError {
            index: 0,
            error: rejected.error.clone(),
        },
        rejected
    );
}

#[test]
fn insert_all_sees_the_pins_earlier_members_set() {
    let mut registry = Registry::unbounded();

    // The second member contradicts the parent the first one pins.
    let parent = registry
        .insert_all(&[edge("a", "b", 1.0, at(1)), edge("z", "b", 1.0, at(2))])
        .unwrap_err();
    assert_eq!(parent.index, 1);
    assert!(
        matches!(
            &parent.error,
            InsertError::ParentMismatch { frame, pinned, offered, .. }
                if frame == "b" && pinned == "a" && offered == "z"
        ),
        "{parent:?}"
    );

    // The second member contradicts the kind the first one fixes.
    let kind = registry
        .insert_all(&[
            edge("a", "b", 1.0, Stamp::Static),
            edge("a", "b", 1.0, at(2)),
        ])
        .unwrap_err();
    assert_eq!(kind.index, 1);
    assert!(
        matches!(
            &kind.error,
            InsertError::KindMismatch { frame, incoming, .. } if frame == "b" && *incoming == at(2)
        ),
        "{kind:?}"
    );

    // The registry is still empty.
    assert!(matches!(
        registry.lookup("a", "b", ts(1)),
        Err(LookupError::UnknownFrame { .. })
    ));
}

#[test]
fn insert_all_measures_expiry_against_the_running_newest_stamp() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));

    // Newest first: the second member is behind the window the first one
    // opens, although the registry holds nothing yet.
    let rejected = registry
        .insert_all(&[
            edge("a", "b", 1.0, at(100 * SECOND)),
            edge("a", "b", 0.0, at(50 * SECOND)),
        ])
        .unwrap_err();
    assert_eq!(rejected.index, 1);
    assert!(
        matches!(
            &rejected.error,
            InsertError::Expired { frame, stamp, newest, .. }
                if frame == "b" && *stamp == ts(50 * SECOND) && *newest == ts(100 * SECOND)
        ),
        "{rejected:?}"
    );
    assert!(matches!(
        registry.lookup("a", "b", ts(100 * SECOND)),
        Err(LookupError::UnknownFrame { .. })
    ));

    // Oldest first: both are accepted, and the second evicts the first.
    registry
        .insert_all(&[
            edge("a", "b", 0.0, at(50 * SECOND)),
            edge("a", "b", 1.0, at(100 * SECOND)),
        ])
        .unwrap();
    assert!(registry.lookup("a", "b", ts(50 * SECOND)).is_err());
    assert!(registry.lookup("a", "b", ts(100 * SECOND)).is_ok());
}

#[test]
fn one_expired_member_rejects_fresh_samples_of_other_frames() {
    // 2.4 stored the fresh samples and silently dropped the late one.
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("map", "base", 0.0, at(100 * SECOND)))
        .unwrap();
    let batch = [
        edge("map", "arm", 1.0, at(100 * SECOND)),
        edge("map", "base", 1.0, at(SECOND)),
        edge("map", "head", 1.0, at(100 * SECOND)),
    ];

    let rejected = registry.insert_all(&batch).unwrap_err();

    assert_eq!(rejected.index, 1);
    assert!(matches!(rejected.error, InsertError::Expired { .. }));
    assert!(matches!(
        registry.lookup("map", "arm", ts(100 * SECOND)),
        Err(LookupError::UnknownFrame { .. })
    ));

    // The documented fallback: resubmit without the rejected member.
    let rest: Vec<Transform> = batch
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != rejected.index)
        .map(|(_, member)| member.clone())
        .collect();
    registry.insert_all(&rest).unwrap();
    assert!(registry.lookup("arm", "head", ts(100 * SECOND)).is_ok());
}

// ---- reparent ----

#[test]
fn reparent_moves_a_dynamic_frame_and_collapses_its_history_to_the_seed() {
    let mut registry = registry_of(&[
        ("map_a", "odom", 1.0, at(1_000)),
        ("map_a", "odom", 1.5, at(1_500)),
        ("odom", "base", 0.25, Stamp::Static),
    ]);

    registry
        .reparent(&edge("map_b", "odom", 2.0, at(2_000)))
        .unwrap();

    assert_eq!(
        registry.lookup("map_b", "odom", ts(2_000)),
        Ok(edge("map_b", "odom", 2.0, at(2_000)))
    );
    // Descendants keep their pins and follow the move.
    assert_eq!(
        registry.lookup("map_b", "base", ts(2_000)),
        Ok(edge("map_b", "base", 2.25, at(2_000)))
    );
    // The earlier history is gone.
    let earlier = registry.lookup("map_b", "odom", ts(1_500));
    assert!(
        matches!(
            &earlier,
            Err(LookupError::OutOfRange { frame, earliest, latest, .. })
                if frame == "odom" && *earliest == ts(2_000) && *latest == ts(2_000)
        ),
        "{earlier:?}"
    );
    // The old parent was named by no other edge.
    let old = registry.lookup("map_a", "odom", ts(1_000));
    assert!(
        matches!(&old, Err(LookupError::UnknownFrame { frame, .. }) if frame == "map_a"),
        "{old:?}"
    );
    // The kind is kept: the frame is still dynamic.
    assert!(matches!(
        registry.insert(&edge("map_b", "odom", 0.0, Stamp::Static)),
        Err(InsertError::KindMismatch { .. })
    ));
    registry
        .insert(&edge("map_b", "odom", 3.0, at(3_000)))
        .unwrap();
}

#[test]
fn reparent_moves_a_static_frame_and_answers_every_instant() {
    let mut registry = registry_of(&[
        ("base", "camera", 1.0, Stamp::Static),
        ("base", "arm", 1.0, Stamp::Static),
    ]);

    registry
        .reparent(&edge("arm", "camera", 0.5, Stamp::Static))
        .unwrap();

    assert_eq!(
        registry.lookup("base", "camera", ts(-7)),
        Ok(edge("base", "camera", 1.5, at(-7)))
    );
    // `base` is still named by the arm's edge.
    assert_eq!(
        registry.latest_common_time("base", "camera"),
        Ok(Stamp::Static)
    );
}

#[test]
fn reparent_reports_each_of_its_refusals() {
    let mut registry = registry_of(&[("a", "b", 1.0, at(1)), ("b", "c", 1.0, at(1))]);

    let unknown = registry.reparent(&edge("a", "q", 1.0, at(1)));
    assert!(
        matches!(&unknown, Err(InsertError::UnknownFrame { frame, .. }) if frame == "q"),
        "{unknown:?}"
    );

    let root = registry.reparent(&edge("z", "a", 1.0, at(1)));
    assert!(
        matches!(&root, Err(InsertError::NoParent { frame, .. }) if frame == "a"),
        "{root:?}"
    );

    let unchanged = registry.reparent(&edge("a", "b", 1.0, at(2)));
    assert!(
        matches!(
            &unchanged,
            Err(InsertError::ParentUnchanged { frame, parent, .. }) if frame == "b" && parent == "a"
        ),
        "{unchanged:?}"
    );

    let own_parent = registry.reparent(&edge("b", "b", 1.0, at(1)));
    assert!(
        matches!(&own_parent, Err(InsertError::SelfEdge { frame, .. }) if frame == "b"),
        "{own_parent:?}"
    );

    let cycle = registry.reparent(&edge("c", "b", 1.0, at(1)));
    assert!(
        matches!(
            &cycle,
            Err(InsertError::Cycle { child, parent, .. }) if child == "b" && parent == "c"
        ),
        "{cycle:?}"
    );

    let kind = registry.reparent(&edge("z", "b", 1.0, Stamp::Static));
    assert!(
        matches!(
            &kind,
            Err(InsertError::KindMismatch { frame, incoming, .. })
                if frame == "b" && *incoming == Stamp::Static
        ),
        "{kind:?}"
    );

    // Every refusal left the tree as it was, and made no name known.
    assert_eq!(
        registry.lookup("a", "c", ts(1)),
        Ok(edge("a", "c", 2.0, at(1)))
    );
    for name in ["q", "z"] {
        assert!(matches!(
            registry.lookup(name, name, ts(1)),
            Err(LookupError::UnknownFrame { .. })
        ));
    }
}

#[test]
fn a_reparent_seed_is_never_expired() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 1.0, at(100 * SECOND)))
        .unwrap();

    registry.reparent(&edge("z", "b", 2.0, at(SECOND))).unwrap();

    assert_eq!(
        registry.lookup("z", "b", ts(SECOND)),
        Ok(edge("z", "b", 2.0, at(SECOND)))
    );
    // The window now follows the seed.
    registry
        .insert(&edge("z", "b", 3.0, at(2 * SECOND)))
        .unwrap();
}

// ---- remove_frame ----

#[test]
fn remove_frame_reports_whether_an_edge_was_removed() {
    let mut registry = registry_of(&[
        ("world", "base", 1.0, Stamp::Static),
        ("base", "sensor", 1.0, at(1)),
    ]);

    assert!(!registry.remove_frame("world"));
    assert!(!registry.remove_frame("nowhere"));
    assert!(!registry.remove_frame(""));
    assert!(registry.remove_frame("base"));
    assert!(!registry.remove_frame("base"));

    // `world` was named only by the removed edge. `base` stays known as
    // the parent of `sensor`, whose pin and history are untouched.
    let world = registry.lookup("world", "sensor", ts(1));
    assert!(
        matches!(&world, Err(LookupError::UnknownFrame { frame, .. }) if frame == "world"),
        "{world:?}"
    );
    assert_eq!(
        registry.lookup("base", "sensor", ts(1)),
        Ok(edge("base", "sensor", 1.0, at(1)))
    );
}

#[test]
fn remove_frame_releases_the_parent_and_the_kind() {
    let mut registry = registry_of(&[
        ("world", "base", 1.0, Stamp::Static),
        ("base", "sensor", 1.0, Stamp::Static),
    ]);
    assert!(registry.remove_frame("base"));

    // A new parent and a new kind are both accepted.
    registry.insert(&edge("odom", "base", 4.0, at(3))).unwrap();

    assert_eq!(
        registry.lookup("odom", "sensor", ts(3)),
        Ok(edge("odom", "sensor", 5.0, at(3)))
    );
    assert_eq!(registry.latest_common_time("odom", "sensor"), Ok(at(3)));
}

#[test]
fn remove_frame_of_a_leaf_forgets_both_names_of_a_lone_edge() {
    let mut registry = registry_of(&[("a", "b", 1.0, at(1))]);
    assert!(registry.remove_frame("b"));
    for name in ["a", "b"] {
        assert!(matches!(
            registry.lookup(name, name, ts(1)),
            Err(LookupError::UnknownFrame { .. })
        ));
    }
}

#[test]
fn remove_frame_discards_the_newest_stamp_of_a_bounded_frame() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 1.0, at(100 * SECOND)))
        .unwrap();
    assert!(registry.insert(&edge("a", "b", 1.0, at(SECOND))).is_err());

    assert!(registry.remove_frame("b"));

    registry.insert(&edge("a", "b", 1.0, at(SECOND))).unwrap();
}

// ---- remove_samples_through ----

#[test]
fn remove_samples_through_removes_the_cutoff_sample_itself() {
    // 2.4's `remove_transforms_before` kept the sample at the cutoff.
    let mut registry = registry_of(&[
        ("map", "base", 1.0, at(10)),
        ("map", "base", 2.0, at(20)),
        ("map", "base", 3.0, at(30)),
    ]);

    registry.remove_samples_through(ts(20));

    let removed = registry.lookup("map", "base", ts(20));
    assert!(
        matches!(
            &removed,
            Err(LookupError::OutOfRange { earliest, latest, .. })
                if *earliest == ts(30) && *latest == ts(30)
        ),
        "{removed:?}"
    );
    assert_eq!(
        registry.lookup("map", "base", ts(30)),
        Ok(edge("map", "base", 3.0, at(30)))
    );
}

#[test]
fn remove_samples_through_the_maximum_clears_dynamic_history_and_keeps_the_tree() {
    let mut registry = registry_of(&[
        ("map", "base", 1.0, at(10)),
        ("map", "base", 1.0, at(i64::MAX)),
        ("base", "lidar", 0.5, Stamp::Static),
        ("map", "arm", 1.0, at(-5)),
    ]);

    registry.remove_samples_through(Timestamp::MAX);

    // Drained frames stay known, pinned and dynamic.
    for (frame, nanos) in [("base", 10), ("base", i64::MAX), ("arm", -5)] {
        let drained = registry.lookup("map", frame, ts(nanos));
        assert!(
            matches!(&drained, Err(LookupError::NoData { frame: empty, .. }) if empty == frame),
            "{drained:?}"
        );
    }
    assert!(matches!(
        registry.insert(&edge("other", "base", 1.0, at(1))),
        Err(InsertError::ParentMismatch { .. })
    ));
    assert!(matches!(
        registry.insert(&edge("map", "base", 1.0, Stamp::Static)),
        Err(InsertError::KindMismatch { .. })
    ));
    assert_eq!(
        registry.lookup("map", "map", ts(0)).unwrap().translation(),
        ZERO
    );
    // Static transforms are untouched.
    assert_eq!(
        registry.lookup("base", "lidar", ts(0)),
        Ok(edge("base", "lidar", 0.5, at(0)))
    );
    // A drained frame restarts.
    registry.insert(&edge("map", "base", 2.0, at(1))).unwrap();
    assert_eq!(
        registry.lookup("map", "lidar", ts(1)),
        Ok(edge("map", "lidar", 2.5, at(1)))
    );
}

#[test]
fn a_drained_frame_keeps_no_hidden_newest_stamp() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("a", "b", 1.0, at(100 * SECOND)))
        .unwrap();
    assert!(registry.insert(&edge("a", "b", 1.0, at(SECOND))).is_err());

    registry.remove_samples_through(ts(100 * SECOND));

    registry.insert(&edge("a", "b", 1.0, at(SECOND))).unwrap();
}

#[test]
fn remove_samples_through_before_every_sample_changes_nothing() {
    let mut registry = registry_of(&[("map", "base", 1.0, at(10))]);
    registry.remove_samples_through(ts(9));
    assert!(registry.lookup("map", "base", ts(10)).is_ok());
}

// ---- lookup ----

#[test]
fn lookup_returns_a_stored_sample_bit_for_bit_and_its_inverse_the_other_way() {
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let stored = Transform::new(
        "map",
        "base",
        Vector3::new(0.1, -0.2, 0.3),
        Rotation::from_wxyz(h, 0.0, h, 0.0).unwrap(),
        at(7),
    )
    .unwrap();
    let mut registry = Registry::unbounded();
    registry.insert(&stored).unwrap();

    let forward = registry.lookup("map", "base", ts(7)).unwrap();
    assert_eq!(forward, stored);
    for (found, given) in [
        (forward.translation().x, 0.1_f64),
        (forward.translation().y, -0.2),
        (forward.translation().z, 0.3),
        (forward.rotation().w(), h),
        (forward.rotation().y(), h),
    ] {
        assert_eq!(found.to_bits(), given.to_bits());
    }

    assert_eq!(
        registry.lookup("base", "map", ts(7)).unwrap(),
        stored.inverse().unwrap()
    );
}

#[test]
fn lookup_interpolates_between_samples_and_never_extrapolates() {
    let registry = registry_of(&[("map", "base", 0.0, at(0)), ("map", "base", 1.0, at(10))]);

    assert_eq!(
        registry.lookup("map", "base", ts(5)),
        Ok(edge("map", "base", 0.5, at(5)))
    );
    for outside in [-1, 11] {
        let found = registry.lookup("map", "base", ts(outside));
        assert!(
            matches!(
                &found,
                Err(LookupError::OutOfRange { frame, requested, earliest, latest, .. })
                    if frame == "base"
                        && *requested == ts(outside)
                        && *earliest == ts(0)
                        && *latest == ts(10)
            ),
            "{found:?}"
        );
    }
}

#[test]
fn lookup_between_siblings_goes_through_their_common_parent() {
    let registry = registry_of(&[
        ("world", "left", 1.0, at(1)),
        ("world", "right", 3.0, at(1)),
    ]);
    assert_eq!(
        registry.lookup("left", "right", ts(1)),
        Ok(edge("left", "right", 2.0, at(1)))
    );
    assert_eq!(
        registry.lookup("right", "left", ts(1)),
        Ok(edge("right", "left", -2.0, at(1)))
    );
}

#[test]
fn lookup_of_a_known_frame_relative_to_itself_is_the_identity() {
    let registry = registry_of(&[("a", "b", 1.0, at(5))]);
    // The root and the child, at an instant the edge does not cover.
    for name in ["a", "b"] {
        assert_eq!(
            registry.lookup(name, name, ts(99)),
            Ok(Transform::new(name, name, ZERO, Rotation::IDENTITY, at(99)).unwrap())
        );
    }
}

#[test]
fn lookup_over_a_static_chain_carries_the_requested_instant() {
    let registry = registry_of(&[
        ("a", "b", 1.0, Stamp::Static),
        ("b", "c", 1.0, Stamp::Static),
    ]);
    for nanos in [i64::MIN, 0, i64::MAX] {
        assert_eq!(
            registry.lookup("a", "c", ts(nanos)).unwrap().stamp(),
            at(nanos)
        );
    }
}

#[test]
fn equal_unknown_names_are_unknown_frames_in_every_query() {
    // 2.4 returned the identity for equal unregistered names.
    let empty = Registry::unbounded();
    let populated = registry_of(&[("a", "b", 1.0, at(1))]);
    for registry in [&empty, &populated] {
        let lookup = registry.lookup("map", "map", ts(1));
        assert!(
            matches!(&lookup, Err(LookupError::UnknownFrame { frame, .. }) if frame == "map"),
            "{lookup:?}"
        );
        let latest = registry.latest_common_time("map", "map");
        assert!(
            matches!(&latest, Err(LookupError::UnknownFrame { frame, .. }) if frame == "map"),
            "{latest:?}"
        );
        let across = registry.lookup_across("map", ts(1), "map", ts(1), "map");
        assert!(
            matches!(&across, Err(LookupError::UnknownFrame { frame, .. }) if frame == "map"),
            "{across:?}"
        );
    }
}

#[test]
fn unknown_names_are_reported_in_the_order_target_source_fixed() {
    let registry = registry_of(&[("a", "b", 1.0, at(1))]);
    let unknown = |result: Result<(), LookupError>| match result {
        Err(LookupError::UnknownFrame { frame, .. }) => frame,
        other => panic!("expected UnknownFrame, got {other:?}"),
    };

    assert_eq!(unknown(registry.lookup("p", "q", ts(1)).map(drop)), "p");
    assert_eq!(unknown(registry.lookup("a", "q", ts(1)).map(drop)), "q");
    assert_eq!(unknown(registry.lookup("p", "b", ts(1)).map(drop)), "p");

    assert_eq!(
        unknown(registry.latest_common_time("p", "q").map(drop)),
        "p"
    );
    assert_eq!(
        unknown(registry.latest_common_time("a", "q").map(drop)),
        "q"
    );

    let across = |target, source, fixed| {
        registry
            .lookup_across(target, ts(1), source, ts(1), fixed)
            .map(drop)
    };
    assert_eq!(unknown(across("p", "q", "r")), "p");
    assert_eq!(unknown(across("a", "q", "r")), "q");
    assert_eq!(unknown(across("a", "b", "r")), "r");
}

#[test]
fn the_empty_name_is_never_known() {
    let registry = registry_of(&[("a", "b", 1.0, at(1))]);
    for result in [
        registry.lookup("", "b", ts(1)).map(drop),
        registry.lookup("a", "", ts(1)).map(drop),
        registry.lookup("", "", ts(1)).map(drop),
        registry.latest_common_time("", "b").map(drop),
        registry.lookup_across("a", ts(1), "b", ts(1), "").map(drop),
    ] {
        assert!(
            matches!(&result, Err(LookupError::UnknownFrame { frame, .. }) if frame.is_empty()),
            "{result:?}"
        );
    }
}

#[test]
fn lookup_reports_an_overflowing_chain_as_non_finite() {
    let registry = registry_of(&[
        ("a", "b", f64::MAX, Stamp::Static),
        ("b", "c", f64::MAX, Stamp::Static),
    ]);
    assert_eq!(
        registry.lookup("a", "c", ts(0)),
        Err(LookupError::NonFinite)
    );
    // Each edge alone is fine.
    assert!(registry.lookup("a", "b", ts(0)).is_ok());
}

#[test]
fn missing_coverage_is_reported_before_a_numerical_failure() {
    let registry = registry_of(&[
        ("a", "b", f64::MAX, Stamp::Static),
        ("b", "c", f64::MAX, Stamp::Static),
        ("c", "d", 1.0, at(5)),
    ]);
    let found = registry.lookup("a", "d", ts(6));
    assert!(
        matches!(&found, Err(LookupError::OutOfRange { frame, .. }) if frame == "d"),
        "{found:?}"
    );
    assert_eq!(
        registry.lookup("a", "d", ts(5)),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn lookup_names_the_lowest_failing_edge_on_the_target_side_first() {
    let registry = registry_of(&[
        ("root", "t1", 0.0, at(1)),
        ("t1", "t2", 0.0, at(1)),
        ("root", "s1", 0.0, at(1)),
        ("s1", "s2", 0.0, at(1)),
    ]);
    let failing = |target, source| match registry.lookup(target, source, ts(2)) {
        Err(LookupError::OutOfRange { frame, .. }) => frame,
        other => panic!("expected OutOfRange, got {other:?}"),
    };
    assert_eq!(failing("t2", "s2"), "t2");
    assert_eq!(failing("s2", "t2"), "s2");
    assert_eq!(failing("root", "s2"), "s2");
    assert_eq!(failing("t1", "root"), "t1");
}

// ---- lookup_across ----

#[test]
fn lookup_across_relates_a_frame_to_itself_at_two_instants() {
    let registry = registry_of(&[
        ("world", "camera", 1.0, at(1)),
        ("world", "camera", 2.0, at(2)),
    ]);

    let across = registry
        .lookup_across("camera", ts(2), "camera", ts(1), "world")
        .unwrap();

    assert_eq!(across.target(), "camera");
    assert_eq!(across.source(), "camera");
    assert_eq!(across.target_time(), ts(2));
    assert_eq!(across.source_time(), ts(1));
    assert_eq!(across.translation(), Vector3::new(-1.0, 0.0, 0.0));
    assert_eq!(across.rotation(), Rotation::IDENTITY);

    let copy = across.clone();
    assert_eq!(copy, across);
    assert!(format!("{across:?}").contains("camera"));
    let swapped = registry
        .lookup_across("camera", ts(1), "camera", ts(2), "world")
        .unwrap();
    assert_ne!(swapped, across);
    assert_eq!(swapped.translation(), Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn lookup_across_with_the_fixed_frame_as_an_endpoint_uses_the_other_leg_alone() {
    let registry = registry_of(&[
        ("world", "camera", 1.0, at(1)),
        ("world", "camera", 2.0, at(2)),
    ]);

    // The target leg is the identity: its instant is not sampled.
    let from_camera = registry
        .lookup_across("world", ts(99), "camera", ts(1), "world")
        .unwrap();
    assert_eq!(from_camera.translation(), Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(from_camera.target_time(), ts(99));

    let into_camera = registry
        .lookup_across("camera", ts(2), "world", ts(-99), "world")
        .unwrap();
    assert_eq!(into_camera.translation(), Vector3::new(-2.0, 0.0, 0.0));
    assert_eq!(into_camera.source_time(), ts(-99));
}

#[test]
fn lookup_across_names_the_endpoint_that_does_not_reach_the_fixed_frame() {
    let registry = registry_of(&[("a", "b", 1.0, at(1)), ("x", "y", 1.0, at(1))]);
    let pair =
        |target, source, fixed| match registry.lookup_across(target, ts(1), source, ts(1), fixed) {
            Err(LookupError::Disconnected { a, b, .. }) => (a, b),
            other => panic!("expected Disconnected, got {other:?}"),
        };

    // Equal endpoints and times: both legs are still resolved.
    assert_eq!(pair("b", "b", "y"), ("b".into(), "y".into()));
    // Only the source's leg is disconnected.
    assert_eq!(pair("b", "y", "a"), ("y".into(), "a".into()));
    // Both are: the target's leg is reported.
    assert_eq!(pair("x", "y", "a"), ("x".into(), "a".into()));
    // Topology is decided before coverage.
    assert!(matches!(
        registry.lookup_across("b", ts(50), "y", ts(50), "a"),
        Err(LookupError::Disconnected { .. })
    ));
}

#[test]
fn lookup_across_reports_the_target_leg_coverage_first() {
    let registry = registry_of(&[
        ("world", "target", 1.0, at(10)),
        ("world", "source", 1.0, at(20)),
    ]);
    let failing = |target_time, source_time| match registry.lookup_across(
        "target",
        ts(target_time),
        "source",
        ts(source_time),
        "world",
    ) {
        Err(LookupError::OutOfRange {
            frame, requested, ..
        }) => (frame, requested),
        other => panic!("expected OutOfRange, got {other:?}"),
    };

    assert_eq!(failing(11, 21), ("target".into(), ts(11)));
    assert_eq!(failing(10, 21), ("source".into(), ts(21)));
    assert!(
        registry
            .lookup_across("target", ts(10), "source", ts(20), "world")
            .is_ok()
    );
}

// ---- latest_common_time ----

#[test]
fn latest_common_time_is_the_oldest_newest_sample_of_the_chain() {
    let registry = registry_of(&[
        ("map", "odom", 0.0, at(4)),
        ("map", "odom", 0.0, at(9)),
        ("odom", "base", 0.0, at(3)),
        ("odom", "base", 0.0, at(7)),
        ("base", "lidar", 0.0, Stamp::Static),
    ]);

    assert_eq!(registry.latest_common_time("map", "lidar"), Ok(at(7)));
    assert_eq!(registry.latest_common_time("lidar", "map"), Ok(at(7)));
    assert_eq!(registry.latest_common_time("map", "odom"), Ok(at(9)));
    // The answer is an instant the lookup serves.
    assert!(registry.lookup("map", "lidar", ts(7)).is_ok());
    assert!(registry.lookup("map", "lidar", ts(8)).is_err());
}

#[test]
fn latest_common_time_is_static_when_no_dynamic_edge_is_crossed() {
    let registry = registry_of(&[
        ("map", "odom", 0.0, at(4)),
        ("odom", "base", 0.0, Stamp::Static),
        ("base", "lidar", 0.0, Stamp::Static),
    ]);

    assert_eq!(
        registry.latest_common_time("odom", "lidar"),
        Ok(Stamp::Static)
    );
    // A known frame relative to itself crosses no edge, dynamic or not.
    assert_eq!(
        registry.latest_common_time("odom", "odom"),
        Ok(Stamp::Static)
    );
    assert_eq!(registry.latest_common_time("map", "map"), Ok(Stamp::Static));
}

#[test]
fn latest_common_time_reports_a_drained_edge_as_no_data() {
    let mut registry = registry_of(&[
        ("map", "odom", 0.0, at(4)),
        ("odom", "base", 0.0, Stamp::Static),
    ]);
    registry.remove_samples_through(Timestamp::MAX);

    let latest = registry.latest_common_time("map", "base");
    assert!(
        matches!(&latest, Err(LookupError::NoData { frame, .. }) if frame == "odom"),
        "{latest:?}"
    );
}

#[test]
fn latest_common_time_reports_disjoint_coverage_as_no_common_time() {
    let registry = registry_of(&[
        ("a", "b", 0.0, at(1)),
        ("a", "b", 0.0, at(2)),
        ("b", "c", 0.0, at(5)),
        ("b", "c", 0.0, at(6)),
    ]);

    for (target, source) in [("a", "c"), ("c", "a")] {
        let latest = registry.latest_common_time(target, source);
        assert!(
            matches!(
                &latest,
                Err(LookupError::NoCommonTime { frame, earliest, latest, latest_possible, .. })
                    if frame == "c"
                        && *earliest == ts(5)
                        && *latest == ts(6)
                        && *latest_possible == ts(2)
            ),
            "{latest:?}"
        );
    }
}

#[test]
fn latest_common_time_ignores_edges_above_the_common_ancestor() {
    let registry = registry_of(&[
        // Disjoint from everything below it, and not on the chain.
        ("root", "common", 0.0, at(100)),
        ("common", "left", 0.0, at(1)),
        ("common", "left", 0.0, at(5)),
        ("common", "right", 0.0, at(3)),
        ("common", "right", 0.0, at(8)),
    ]);

    assert_eq!(registry.latest_common_time("left", "right"), Ok(at(5)));
    assert!(matches!(
        registry.latest_common_time("root", "left"),
        Err(LookupError::NoCommonTime { .. })
    ));
}

// ---- Display ----

/// Asserts the exact message and the conventions every message follows:
/// lowercase, one clause, no trailing period.
fn assert_message(
    error: &dyn Display,
    expected: &str,
) {
    let message = error.to_string();
    assert_eq!(message, expected);
    assert_eq!(message, message.to_lowercase());
    assert!(!message.ends_with('.'), "{message}");
    assert!(!message.contains(". "), "{message}");
    assert!(!message.contains('\n'), "{message}");
}

#[test]
fn transform_error_messages_are_exact() {
    assert_message(
        &Rotation::from_wxyz(f64::NAN, 0.0, 0.0, 0.0).unwrap_err(),
        "transform contains a non-finite value",
    );
    assert_message(
        &Rotation::from_wxyz(2.0, 0.0, 0.0, 0.0).unwrap_err(),
        "rotation with norm 2 is not a unit quaternion",
    );
    assert_message(
        &Transform::new("", "b", ZERO, Rotation::IDENTITY, Stamp::Static).unwrap_err(),
        "frame name is empty",
    );
    assert_message(
        &edge("a", "b", 0.0, at(1))
            .compose(&edge("c", "d", 0.0, at(1)))
            .unwrap_err(),
        "frame c does not match expected frame b",
    );
    assert_message(
        &edge("a", "b", 0.0, at(1))
            .compose(&edge("b", "c", 0.0, at(-1_500_000_000)))
            .unwrap_err(),
        "stamps 0.000000001 and -1.500000000 differ",
    );
}

#[test]
fn insert_error_messages_are_exact() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert_all(&[
            edge("a", "b", 0.0, at(20 * SECOND)),
            edge("b", "c", 0.0, Stamp::Static),
        ])
        .unwrap();

    assert_message(
        &registry.insert(&edge("a", "a", 0.0, at(1))).unwrap_err(),
        "frame a cannot be its own parent",
    );
    assert_message(
        &registry.insert(&edge("x", "b", 0.0, at(1))).unwrap_err(),
        "frame b has parent a, not x",
    );
    assert_message(
        &registry
            .insert(&edge("a", "b", 0.0, Stamp::Static))
            .unwrap_err(),
        "frame b is dynamic and cannot take a static transform",
    );
    assert_message(
        &registry.insert(&edge("b", "c", 0.0, at(1))).unwrap_err(),
        "frame c is static and cannot take a dynamic transform",
    );
    assert_message(
        &registry.insert(&edge("c", "a", 0.0, at(1))).unwrap_err(),
        "placing a under c would close a cycle",
    );
    assert_message(
        &registry
            .insert(&edge("a", "b", 0.0, at(SECOND)))
            .unwrap_err(),
        "sample for b at 1.000000000 is older than the retention window behind 20.000000000",
    );
    assert_message(
        &registry.reparent(&edge("a", "q", 0.0, at(1))).unwrap_err(),
        "frame q is not known",
    );
    assert_message(
        &registry.reparent(&edge("z", "a", 0.0, at(1))).unwrap_err(),
        "frame a has no parent to replace",
    );
    assert_message(
        &registry.reparent(&edge("a", "b", 0.0, at(1))).unwrap_err(),
        "frame b is already under a",
    );
    assert_message(
        &registry
            .insert_all(&[
                edge("a", "d", 0.0, Stamp::Static),
                edge("d", "d", 0.0, Stamp::Static),
            ])
            .unwrap_err(),
        "batch member 1 was rejected",
    );
}

#[test]
fn lookup_error_messages_are_exact() {
    let mut registry = registry_of(&[
        ("a", "b", 0.0, at(SECOND)),
        ("a", "b", 0.0, at(2 * SECOND)),
        ("b", "c", 0.0, at(5 * SECOND)),
        ("b", "c", 0.0, at(6 * SECOND)),
        ("x", "y", 0.0, at(1)),
        ("m", "n", f64::MAX, Stamp::Static),
        ("n", "o", f64::MAX, Stamp::Static),
    ]);

    assert_message(
        &registry.lookup("a", "q", ts(0)).unwrap_err(),
        "frame q is not known",
    );
    assert_message(
        &registry.lookup("b", "y", ts(0)).unwrap_err(),
        "frames b and y are not connected",
    );
    assert_message(
        &registry.lookup("a", "b", ts(3 * SECOND)).unwrap_err(),
        "frame b covers 1.000000000 to 2.000000000 and not 3.000000000",
    );
    assert_message(
        &registry.latest_common_time("a", "c").unwrap_err(),
        "frame c covers 5.000000000 to 6.000000000 and the rest of the chain ends at 2.000000000",
    );
    assert_message(
        &registry.lookup("m", "o", ts(0)).unwrap_err(),
        "lookup produced a non-finite value",
    );

    registry.remove_samples_through(Timestamp::MAX);
    assert_message(
        &registry.lookup("a", "b", ts(SECOND)).unwrap_err(),
        "frame b holds no samples",
    );
}

// ---- adapter sketches: what a middleware bridge writes on this API ----

/// Sketch 2: a batch failure names its member.
fn ingest(
    registry: &mut Registry,
    message: &[Transform],
) -> Result<(), String> {
    match registry.insert_all(message) {
        Ok(()) => Ok(()),
        Err(BatchError { index, error }) => {
            let bad = &message[index];
            Err(format!(
                "member {index} ({} <- {}): {error}",
                bad.target(),
                bad.source()
            ))
        }
    }
}

#[test]
fn adapter_reports_the_rejected_member_of_a_batch() {
    let mut registry = Registry::unbounded();
    let message = [
        edge("map", "odom", 1.0, at(1)),
        edge("odom", "base", 1.0, at(1)),
        edge("base", "odom", 1.0, at(1)),
    ];

    assert_eq!(
        ingest(&mut registry, &message),
        Err("member 2 (base <- odom): frame odom has parent map, not base".into())
    );
    assert_eq!(ingest(&mut registry, &message[..2]), Ok(()));
    assert!(registry.lookup("map", "base", ts(1)).is_ok());
}

/// Sketch 3: what a waiting caller does with each lookup failure.
#[derive(Debug, PartialEq)]
enum Wait {
    Retry,
    Fail(String),
}

fn classify(error: LookupError) -> Wait {
    match error {
        // More data can still arrive.
        LookupError::UnknownFrame { .. }
        | LookupError::Disconnected { .. }
        | LookupError::NoData { .. } => Wait::Retry,
        LookupError::OutOfRange {
            frame,
            requested,
            latest,
            ..
        } => {
            if requested > latest {
                Wait::Retry
            } else {
                // Behind the history: no later sample can serve it.
                Wait::Fail(frame)
            }
        }
        LookupError::NoCommonTime { frame, .. } => Wait::Fail(frame),
        LookupError::NonFinite => Wait::Fail("non-finite".into()),
        LookupError::NonUnitRotation { norm, .. } => Wait::Fail(format!("norm {norm}")),
        // The enum is `#[non_exhaustive]`.
        other => Wait::Fail(other.to_string()),
    }
}

#[test]
fn adapter_classifies_every_lookup_error() {
    let mut registry = registry_of(&[
        ("a", "b", 0.0, at(10)),
        ("a", "b", 0.0, at(20)),
        ("b", "c", 0.0, at(50)),
        ("x", "y", 0.0, at(10)),
        ("m", "n", f64::MAX, Stamp::Static),
        ("n", "o", f64::MAX, Stamp::Static),
    ]);

    assert_eq!(
        classify(registry.lookup("a", "nowhere", ts(10)).unwrap_err()),
        Wait::Retry
    );
    assert_eq!(
        classify(registry.lookup("b", "y", ts(10)).unwrap_err()),
        Wait::Retry
    );
    assert_eq!(
        classify(registry.lookup("a", "b", ts(21)).unwrap_err()),
        Wait::Retry
    );
    assert_eq!(
        classify(registry.lookup("a", "b", ts(9)).unwrap_err()),
        Wait::Fail("b".into())
    );
    assert_eq!(
        classify(registry.latest_common_time("a", "c").unwrap_err()),
        Wait::Fail("c".into())
    );
    assert_eq!(
        classify(registry.lookup("m", "o", ts(0)).unwrap_err()),
        Wait::Fail("non-finite".into())
    );

    registry.remove_samples_through(Timestamp::MAX);
    let drained = registry.lookup("a", "b", ts(10)).unwrap_err();
    assert!(matches!(drained, LookupError::NoData { .. }));
    assert_eq!(classify(drained), Wait::Retry);
}

/// Sketch 4: a wait that is bounded by a number of attempts. `pump` feeds
/// the registry and returns `false` when no more data will come.
fn wait_for(
    registry: &mut Registry,
    at: Timestamp,
    attempts: usize,
    mut pump: impl FnMut(&mut Registry) -> bool,
) -> Result<Transform, String> {
    for _ in 0..attempts {
        match registry.lookup("map", "lidar", at) {
            Ok(transform) => return Ok(transform),
            Err(error) => match classify(error) {
                Wait::Retry => {
                    if !pump(registry) {
                        return Err("source closed".into());
                    }
                }
                Wait::Fail(reason) => return Err(reason),
            },
        }
    }
    Err("timeout".into())
}

#[test]
fn adapter_wait_loop_returns_once_the_data_has_arrived() {
    let mut registry = registry_of(&[("map", "base", 1.0, at(5))]);
    let mut pending = vec![
        edge("map", "base", 3.0, at(15)),
        edge("base", "lidar", 0.5, Stamp::Static),
    ];

    // First `lidar` is unknown, then `base` ends before the request, then
    // the lookup interpolates.
    let found = wait_for(&mut registry, ts(10), 5, |registry| {
        pending
            .pop()
            .is_some_and(|next| registry.insert(&next).is_ok())
    });

    assert_eq!(found, Ok(edge("map", "lidar", 2.5, at(10))));
    assert!(pending.is_empty());
}

#[test]
fn adapter_wait_loop_gives_up_after_its_bound() {
    let mut registry = registry_of(&[
        ("map", "base", 1.0, at(5)),
        ("base", "lidar", 0.5, Stamp::Static),
    ]);

    let mut pumped = 0;
    let timed_out = wait_for(&mut registry, ts(10), 3, |_| {
        pumped += 1;
        true
    });
    assert_eq!(timed_out, Err("timeout".into()));
    assert_eq!(pumped, 3);

    assert_eq!(
        wait_for(&mut registry, ts(10), 3, |_| false),
        Err("source closed".into())
    );
    // A request behind the history fails at once, without pumping.
    assert_eq!(
        wait_for(&mut registry, ts(1), 3, |_| panic!("must not pump")),
        Err("base".into())
    );
}

#[test]
fn adapter_flattens_a_chain_and_republishes_it() {
    // Sketch 5.
    let mut registry = registry_of(&[
        ("map", "odom", 1.0, at(10)),
        ("odom", "lidar", 0.5, Stamp::Static),
    ]);
    let flat = registry.lookup("map", "lidar", ts(10)).unwrap();

    // Re-inserted as it is, the result's own labels reject it: `lidar`
    // hangs under `odom`, not under `map`.
    let rejected = registry.insert(&flat);
    assert!(
        matches!(
            &rejected,
            Err(InsertError::ParentMismatch { frame, pinned, offered, .. })
                if frame == "lidar" && pinned == "odom" && offered == "map"
        ),
        "{rejected:?}"
    );

    // Republished under a new name, as a sample and as a static transform.
    let renamed = Transform::new(
        "map",
        "lidar_flat",
        flat.translation(),
        flat.rotation(),
        flat.stamp(),
    )
    .unwrap();
    registry.insert(&renamed).unwrap();
    let fixed = Transform::new(
        flat.target(),
        "lidar_static",
        flat.translation(),
        flat.rotation(),
        Stamp::Static,
    )
    .unwrap();
    registry.insert(&fixed).unwrap();

    assert_eq!(
        registry
            .lookup("lidar", "lidar_flat", ts(10))
            .unwrap()
            .translation(),
        ZERO
    );
    assert_eq!(
        registry
            .lookup("lidar_static", "lidar", ts(10))
            .unwrap()
            .translation(),
        ZERO
    );

    // The flattened transform and its inverse compose to a self-named
    // identity.
    let round_trip = flat.compose(&flat.inverse().unwrap()).unwrap();
    assert_eq!((round_trip.target(), round_trip.source()), ("map", "map"));
    assert_eq!(round_trip.translation(), ZERO);
    assert_eq!(round_trip.rotation(), Rotation::IDENTITY);
}

#[test]
fn adapter_applies_a_cross_time_result_to_a_point() {
    // Sketch 6: a point seen one metre ahead of the robot at t = 10, in
    // the robot's frame at t = 20, after the robot drove two metres on and
    // turned a quarter turn to the left.
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let mut registry = registry_of(&[("odom", "base", 1.0, at(10))]);
    registry
        .insert(
            &Transform::new(
                "odom",
                "base",
                Vector3::new(3.0, 0.0, 0.0),
                Rotation::from_wxyz(h, 0.0, 0.0, h).unwrap(),
                at(20),
            )
            .unwrap(),
        )
        .unwrap();

    let across = registry
        .lookup_across("base", ts(20), "base", ts(10), "odom")
        .unwrap();
    let p = Vector3::new(1.0, 0.0, 0.0);
    let r = across.rotation().rotate(p);
    let t = across.translation();
    let p_now = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);

    // The point sits at x = 2 in `odom`: one metre behind the robot, which
    // now faces +y, so it lies on the robot's left.
    approx::assert_abs_diff_eq!(p_now.x, 0.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(p_now.y, 1.0, epsilon = 1e-12);
    approx::assert_abs_diff_eq!(p_now.z, 0.0, epsilon = 1e-12);
    assert_eq!((across.target(), across.target_time()), ("base", ts(20)));
    assert_eq!((across.source(), across.source_time()), ("base", ts(10)));
}

/// Sketch 7: the newest transform a chain serves.
fn latest(
    registry: &Registry,
    target: &str,
    source: &str,
) -> Result<(Stamp, Transform), LookupError> {
    let stamp = registry.latest_common_time(target, source)?;
    let at = match stamp {
        Stamp::At(newest) => newest,
        // Every edge is static and serves any instant.
        Stamp::Static => Timestamp::from_nanos(0),
    };
    Ok((stamp, registry.lookup(target, source, at)?))
}

#[test]
fn adapter_looks_up_the_latest_transform_of_dynamic_and_static_chains() {
    let registry = registry_of(&[
        ("map", "odom", 1.0, at(4)),
        ("map", "odom", 2.0, at(9)),
        ("odom", "base", 1.0, at(3)),
        ("odom", "base", 2.0, at(7)),
        ("base", "lidar", 0.5, Stamp::Static),
    ]);

    let (stamp, transform) = latest(&registry, "map", "lidar").unwrap();
    assert_eq!(stamp, at(7));
    assert_eq!(transform.stamp(), at(7));
    approx::assert_abs_diff_eq!(transform.translation().x, 1.6 + 2.0 + 0.5, epsilon = 1e-12);

    let (stamp, transform) = latest(&registry, "base", "lidar").unwrap();
    assert_eq!(stamp, Stamp::Static);
    assert_eq!(transform, edge("base", "lidar", 0.5, at(0)));

    assert!(matches!(
        latest(&registry, "map", "nowhere"),
        Err(LookupError::UnknownFrame { .. })
    ));
}

/// Sketch 8: a clock reset. The rejected sample was only borrowed, so the
/// adapter still holds it and can store it after discarding the frame.
fn insert_after_reset(
    registry: &mut Registry,
    transform: &Transform,
) -> Result<(), InsertError> {
    match registry.insert(transform) {
        Err(InsertError::Expired {
            frame,
            stamp,
            newest,
            ..
        }) if stamp < newest => {
            registry.remove_frame(&frame);
            registry.insert(transform)
        }
        result => result,
    }
}

#[test]
fn adapter_recovers_from_expired_and_keeps_the_sample() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&edge("map", "base", 1.0, at(100 * SECOND)))
        .unwrap();
    let restarted = edge("map", "base", 2.0, at(50 * SECOND));

    let rejected = registry.insert(&restarted);
    assert!(
        matches!(
            &rejected,
            Err(InsertError::Expired { frame, .. }) if frame == restarted.source()
        ),
        "{rejected:?}"
    );

    insert_after_reset(&mut registry, &restarted).unwrap();

    assert_eq!(
        registry.lookup("map", "base", ts(50 * SECOND)),
        Ok(restarted)
    );
    assert!(registry.lookup("map", "base", ts(100 * SECOND)).is_err());
    // Other refusals pass through unchanged.
    assert!(matches!(
        insert_after_reset(&mut registry, &edge("odom", "base", 0.0, at(1))),
        Err(InsertError::ParentMismatch { .. })
    ));
}

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// Sketch 9, the conversion of the `Timestamp` documentation:
/// `builtin_interfaces/Time { sec: i32, nanosec: u32 }` to a `Timestamp`.
fn from_ros(
    sec: i32,
    nanosec: u32,
) -> Option<Timestamp> {
    let nanosec = i64::from(nanosec);
    if nanosec >= NANOS_PER_SEC {
        return None; // denormalized stamp
    }
    // Cannot overflow: |sec| * 1e9 + nanosec < 2.2e18 < i64::MAX.
    Some(Timestamp::from_nanos(
        i64::from(sec) * NANOS_PER_SEC + nanosec,
    ))
}

/// A `Timestamp` to `(sec, nanosec)`. Euclidean division floors, so the
/// nanosecond part is always in `0..1e9`: -1 ns is `(-1, 999_999_999)`.
fn to_ros(t: Timestamp) -> Option<(i32, u32)> {
    let nanos = t.as_nanos();
    let sec = i32::try_from(nanos.div_euclid(NANOS_PER_SEC)).ok()?;
    let nanosec = u32::try_from(nanos.rem_euclid(NANOS_PER_SEC)).ok()?;
    Some((sec, nanosec))
}

#[test]
fn adapter_converts_ros_stamps_in_both_directions() {
    assert_eq!(to_ros(ts(-1)), Some((-1, 999_999_999)));
    assert_eq!(from_ros(-1, 500_000_000), Some(ts(-500_000_000)));
    assert_eq!(to_ros(ts(-500_000_000)), Some((-1, 500_000_000)));
    assert_eq!(from_ros(0, 1_000_000_000), None);
    assert_eq!(to_ros(Timestamp::MAX), None);
    assert_eq!(to_ros(ts(i64::MIN)), None);

    // Every normalized ROS stamp fits and survives the round trip.
    for (sec, nanosec) in [
        (0, 0),
        (1, 1),
        (-1, 0),
        (1_700_000_000, 123_456_789),
        (i32::MAX, 999_999_999),
        (i32::MIN, 0),
        (i32::MIN, 999_999_999),
    ] {
        let t = from_ros(sec, nanosec).unwrap();
        assert_eq!(to_ros(t), Some((sec, nanosec)));
    }

    // A converted stamp is a registry instant like any other.
    let stamp = from_ros(-1, 500_000_000).unwrap();
    let registry = registry_of(&[("map", "base", 1.0, Stamp::At(stamp))]);
    let found = registry.lookup("map", "base", stamp).unwrap();
    let Stamp::At(found_at) = found.stamp() else {
        panic!("a lookup result is stamped with an instant");
    };
    assert_eq!(to_ros(found_at), Some((-1, 500_000_000)));
}

/// Sketch 10: the window `[t - back, t + ahead]` around an instant, or
/// `None` where an end is not representable.
fn window(
    t: Timestamp,
    back: Duration,
    ahead: Duration,
) -> Option<(Timestamp, Timestamp)> {
    Some((t.checked_sub(back)?, t.checked_add(ahead)?))
}

#[test]
fn adapter_shifts_instants_with_checked_arithmetic() {
    let t = ts(1_700_000_000_123_456_789);

    assert_eq!(
        window(t, Duration::from_secs(1), Duration::from_millis(5)),
        Some((ts(1_699_999_999_123_456_789), ts(1_700_000_000_128_456_789)))
    );
    assert_eq!(window(t, Duration::from_secs(1), Duration::MAX), None);
    assert_eq!(window(t, Duration::MAX, Duration::ZERO), None);
    assert_eq!(
        window(ts(0), Duration::from_nanos(1), Duration::ZERO),
        Some((ts(-1), ts(0)))
    );

    // The age of a sample relative to a clock reading.
    let now = t.checked_add(Duration::from_millis(30)).unwrap();
    assert_eq!(
        now.checked_duration_since(t),
        Some(Duration::from_millis(30))
    );
    assert_eq!(t.checked_duration_since(now), None);
}
