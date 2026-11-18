//! `Registry::lookup`: results, diagnosis order, exactness, interpolation.
//!
//! The helpers at the top are shared with the other read-side test files.

use crate::{LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
use alloc::{format, string::String, vec::Vec};
use approx::assert_abs_diff_eq;

/// The instant `nanos` nanoseconds after the epoch.
pub(super) fn at(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

/// A transform with every part given.
pub(super) fn posed(
    target: &str,
    source: &str,
    translation: Vector3,
    rotation: Rotation,
    stamp: Stamp,
) -> Transform {
    Transform::new(target, source, translation, rotation, stamp).unwrap()
}

/// A dynamic sample that puts `source` at `x` on the x axis of `target`.
pub(super) fn sample(
    target: &str,
    source: &str,
    x: f64,
    nanos: i64,
) -> Transform {
    posed(
        target,
        source,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(at(nanos)),
    )
}

/// A static transform that puts `source` at `x` on the x axis of `target`.
pub(super) fn mount(
    target: &str,
    source: &str,
    x: f64,
) -> Transform {
    posed(
        target,
        source,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
}

/// An unbounded registry holding `transforms`, inserted in order.
pub(super) fn registry_of(transforms: &[Transform]) -> Registry {
    let mut registry = Registry::unbounded();
    for transform in transforms {
        registry.insert(transform).unwrap();
    }
    registry
}

/// A rotation by `radians` about the z axis.
pub(super) fn yaw(radians: f64) -> Rotation {
    let half = radians / 2.0;
    Rotation::from_wxyz(libm::cos(half), 0.0, 0.0, libm::sin(half)).unwrap()
}

/// A rotation about a tilted axis whose components are not round numbers.
pub(super) fn tilted() -> Rotation {
    let norm = libm::sqrt(0.87);
    Rotation::from_wxyz(0.3 / norm, -0.5 / norm, 0.7 / norm, 0.2 / norm).unwrap()
}

/// The seven numbers of a result as bit patterns, so that a comparison also
/// separates `0.0` from `-0.0`.
pub(super) fn bits(
    translation: Vector3,
    rotation: Rotation,
) -> [u64; 7] {
    [
        translation.x.to_bits(),
        translation.y.to_bits(),
        translation.z.to_bits(),
        rotation.w().to_bits(),
        rotation.x().to_bits(),
        rotation.y().to_bits(),
        rotation.z().to_bits(),
    ]
}

/// The bit patterns of a transform's numbers.
pub(super) fn transform_bits(transform: &Transform) -> [u64; 7] {
    bits(transform.translation(), transform.rotation())
}

pub(super) fn unknown(frame: &str) -> LookupError {
    LookupError::UnknownFrame {
        frame: frame.into(),
    }
}

pub(super) fn disconnected(
    a: &str,
    b: &str,
) -> LookupError {
    LookupError::Disconnected {
        a: a.into(),
        b: b.into(),
    }
}

pub(super) fn no_data(frame: &str) -> LookupError {
    LookupError::NoData {
        frame: frame.into(),
    }
}

pub(super) fn out_of_range(
    frame: &str,
    requested: i64,
    earliest: i64,
    latest: i64,
) -> LookupError {
    LookupError::OutOfRange {
        frame: frame.into(),
        requested: at(requested),
        earliest: at(earliest),
        latest: at(latest),
    }
}

const ORIGIN: Vector3 = Vector3::new(0.0, 0.0, 0.0);

/// The tree `r <- a <- b <- c <- d` with the branch `a <- x <- y`. Every
/// edge is dynamic and holds one sample: at 100, or at 200 for the child
/// frames named in `uncovered`, which then cannot serve a lookup at 100.
fn branching_tree(uncovered: &[&str]) -> Registry {
    let mut registry = Registry::unbounded();
    for (parent, child) in [
        ("r", "a"),
        ("a", "b"),
        ("b", "c"),
        ("c", "d"),
        ("a", "x"),
        ("x", "y"),
    ] {
        let nanos = if uncovered.contains(&child) { 200 } else { 100 };
        registry.insert(&sample(parent, child, 1.0, nanos)).unwrap();
    }
    registry
}

/// `root <- top <- a <- b <- c` and `root <- side`. The static edges `a`
/// and `b` each translate by 1e308, so composing them overflows. The three
/// dynamic edges hold one sample each, at the given instants.
fn overflowing_tree(
    c_nanos: i64,
    top_nanos: i64,
    side_nanos: i64,
) -> Registry {
    registry_of(&[
        sample("root", "top", 1.0, top_nanos),
        mount("top", "a", 1.0e308),
        mount("a", "b", 1.0e308),
        sample("b", "c", 1.0, c_nanos),
        sample("root", "side", 1.0, side_nanos),
    ])
}

#[test]
fn chain_toward_an_ancestor_adds_the_hops() {
    let registry = registry_of(&[
        posed(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
        posed(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
    ]);

    let result = registry.lookup("a", "c", at(1_000_000_000)).unwrap();

    assert_eq!(
        result,
        posed(
            "a",
            "c",
            Vector3::new(1.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        )
    );
}

#[test]
fn chain_toward_a_descendant_is_the_inverse_relationship() {
    let registry = registry_of(&[
        posed(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
        posed(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
    ]);

    let result = registry.lookup("c", "a", at(1_000_000_000)).unwrap();

    assert_eq!(result.target(), "c");
    assert_eq!(result.source(), "a");
    assert_eq!(result.stamp(), Stamp::At(at(1_000_000_000)));
    assert_eq!(result.translation(), Vector3::new(-1.0, -1.0, 0.0));
    assert_eq!(result.rotation(), Rotation::IDENTITY);
}

#[test]
fn chain_with_a_rotated_hop_rotates_the_hops_below_it() {
    let quarter_turn = yaw(core::f64::consts::FRAC_PI_2);
    let registry = registry_of(&[
        sample("a", "b", 1.0, 1_000_000_000),
        posed("b", "c", ORIGIN, quarter_turn, Stamp::At(at(1_000_000_000))),
        sample("c", "d", 1.0, 1_000_000_000),
    ]);

    let result = registry.lookup("a", "d", at(1_000_000_000)).unwrap();

    assert_abs_diff_eq!(result.translation().x, 1.0, epsilon = 1e-15);
    assert_abs_diff_eq!(result.translation().y, 1.0, epsilon = 1e-15);
    assert_abs_diff_eq!(result.translation().z, 0.0, epsilon = 1e-15);
    assert_abs_diff_eq!(result.rotation().w(), quarter_turn.w(), epsilon = 1e-15);
    assert_abs_diff_eq!(result.rotation().z(), quarter_turn.z(), epsilon = 1e-15);
}

#[test]
fn siblings_resolve_through_their_common_parent() {
    let registry = registry_of(&[
        posed(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
        sample("b", "c", 1.0, 1_000_000_000),
        sample("b", "d", 2.0, 1_000_000_000),
    ]);

    let c_from_d = registry.lookup("c", "d", at(1_000_000_000)).unwrap();
    assert_eq!(c_from_d.translation(), Vector3::new(1.0, 0.0, 0.0));

    let d_from_c = registry.lookup("d", "c", at(1_000_000_000)).unwrap();
    assert_eq!(d_from_c.target(), "d");
    assert_eq!(d_from_c.source(), "c");
    assert_eq!(d_from_c.translation(), Vector3::new(-1.0, 0.0, 0.0));
}

#[test]
fn leaves_under_a_deep_common_trunk_resolve_through_the_lowest_shared_frame() {
    let mut registry = Registry::unbounded();
    for (parent, child, translation) in [
        ("a", "b", Vector3::new(1.0, 0.0, 0.0)),
        ("b", "c", Vector3::new(0.0, 1.0, 0.0)),
        ("c", "d", Vector3::new(0.0, 0.0, 1.0)),
        ("d", "e", Vector3::new(1.0, 0.0, 0.0)),
        ("d", "f", Vector3::new(0.0, 2.0, 0.0)),
    ] {
        registry
            .insert(&posed(
                parent,
                child,
                translation,
                Rotation::IDENTITY,
                Stamp::At(at(1_000_000_000)),
            ))
            .unwrap();
    }

    // `e` sits at x = 1 under `d`, `f` at y = 2: `f` expressed in `e`.
    let result = registry.lookup("e", "f", at(1_000_000_000)).unwrap();

    assert_eq!(result.target(), "e");
    assert_eq!(result.source(), "f");
    assert_eq!(result.translation(), Vector3::new(-1.0, 2.0, 0.0));
}

#[test]
fn result_frames_are_the_requested_frames_in_every_direction() {
    let registry = branching_tree(&[]);

    for (target, source) in [
        ("r", "d"),
        ("d", "r"),
        ("a", "c"),
        ("c", "a"),
        ("y", "d"),
        ("d", "y"),
        ("x", "b"),
        ("d", "d"),
    ] {
        let result = registry.lookup(target, source, at(100)).unwrap();
        assert_eq!(result.target(), target);
        assert_eq!(result.source(), source);
        assert_eq!(result.stamp(), Stamp::At(at(100)));
    }
}

#[test]
fn result_over_an_all_static_chain_carries_the_requested_instant() {
    let registry = registry_of(&[mount("a", "b", 1.0), mount("b", "c", 2.0)]);

    for instant in [at(0), at(-5), at(1_000_000_000), Timestamp::MAX] {
        for (target, source, x) in [("a", "c", 3.0), ("c", "a", -3.0), ("a", "b", 1.0)] {
            let result = registry.lookup(target, source, instant).unwrap();
            assert_eq!(result.stamp(), Stamp::At(instant));
            assert_eq!(result.target(), target);
            assert_eq!(result.source(), source);
            assert_eq!(result.translation(), Vector3::new(x, 0.0, 0.0));
        }
    }
}

#[test]
fn known_frame_relative_to_itself_is_the_identity() {
    let mut registry = registry_of(&[sample("a", "b", 1.0, 100), mount("b", "c", 1.0)]);

    // A root, a dynamic child at a stored instant and outside its range,
    // and a static child.
    for (frame, nanos) in [("a", 100), ("b", 100), ("b", 5), ("b", 500), ("c", 7)] {
        let result = registry.lookup(frame, frame, at(nanos)).unwrap();
        assert_eq!(result.target(), frame);
        assert_eq!(result.source(), frame);
        assert_eq!(result.stamp(), Stamp::At(at(nanos)));
        assert_eq!(
            transform_bits(&result),
            bits(ORIGIN, Rotation::IDENTITY),
            "{frame} at {nanos}"
        );
    }

    // A drained frame stays known, and its own edge is not on the chain.
    registry.remove_samples_through(Timestamp::MAX);
    let result = registry.lookup("b", "b", at(100)).unwrap();
    assert_eq!(transform_bits(&result), bits(ORIGIN, Rotation::IDENTITY));
}

#[test]
fn empty_registry_answers_every_lookup_with_unknown_frame() {
    let registry = Registry::unbounded();

    assert_eq!(registry.lookup("a", "a", at(0)), Err(unknown("a")));
    assert_eq!(registry.lookup("a", "b", at(0)), Err(unknown("a")));
    assert_eq!(registry.lookup("", "", at(0)), Err(unknown("")));
}

#[test]
fn equal_unknown_names_are_an_unknown_frame_and_not_the_identity() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100)]);

    assert_eq!(
        registry.lookup("nowhere", "nowhere", at(100)),
        Err(unknown("nowhere"))
    );
}

#[test]
fn empty_name_is_an_unknown_frame() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100)]);

    assert_eq!(registry.lookup("", "b", at(100)), Err(unknown("")));
    assert_eq!(registry.lookup("b", "", at(100)), Err(unknown("")));
    assert_eq!(registry.lookup("", "", at(100)), Err(unknown("")));
}

#[test]
fn unknown_frame_names_the_missing_endpoint_on_either_side() {
    let registry = registry_of(&[sample("a", "b", 1.0, 1_000_000_000)]);

    assert_eq!(
        registry.lookup("b", "does_not_exist", at(1_000_000_000)),
        Err(unknown("does_not_exist"))
    );
    assert_eq!(
        registry.lookup("does_not_exist", "b", at(1_000_000_000)),
        Err(unknown("does_not_exist"))
    );
}

#[test]
fn unknown_target_is_reported_before_an_unknown_source() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100)]);

    assert_eq!(
        registry.lookup("first", "second", at(100)),
        Err(unknown("first"))
    );
    assert_eq!(
        registry.lookup("second", "first", at(100)),
        Err(unknown("second"))
    );
}

#[test]
fn unknown_frame_takes_precedence_over_a_coverage_gap() {
    // `a <- b` holds data at t1 only. No amount of waiting for data makes a
    // lookup of an unknown frame succeed, so that is the diagnosis.
    let mut registry = registry_of(&[sample("a", "b", 1.0, 1_000_000_000)]);

    assert_eq!(
        registry.lookup("b", "nope", at(2_000_000_000)),
        Err(unknown("nope"))
    );
    assert_eq!(
        registry.lookup("nope", "b", at(2_000_000_000)),
        Err(unknown("nope"))
    );

    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        registry.lookup("b", "nope", at(2_000_000_000)),
        Err(unknown("nope"))
    );
}

#[test]
fn frames_in_separate_trees_are_disconnected() {
    let registry = registry_of(&[
        sample("r1", "a", 1.0, 1_000_000_000),
        sample("r2", "b", 1.0, 1_000_000_000),
    ]);

    assert_eq!(
        registry.lookup("a", "b", at(1_000_000_000)),
        Err(disconnected("a", "b"))
    );
    assert_eq!(
        registry.lookup("b", "a", at(1_000_000_000)),
        Err(disconnected("b", "a"))
    );
    // Roots of two trees, and a root against the other tree's leaf.
    assert_eq!(
        registry.lookup("r1", "r2", at(1_000_000_000)),
        Err(disconnected("r1", "r2"))
    );
    assert_eq!(
        registry.lookup("r2", "a", at(1_000_000_000)),
        Err(disconnected("r2", "a"))
    );
}

#[test]
fn disconnected_is_reported_before_any_coverage_error() {
    let mut registry = registry_of(&[sample("r1", "a", 1.0, 200), sample("r2", "b", 1.0, 200)]);

    // Both edges are out of range at 100.
    assert_eq!(
        registry.lookup("a", "b", at(100)),
        Err(disconnected("a", "b"))
    );
    assert_eq!(
        registry.lookup("b", "a", at(100)),
        Err(disconnected("b", "a"))
    );

    // Both edges are empty.
    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        registry.lookup("a", "b", at(100)),
        Err(disconnected("a", "b"))
    );
    assert_eq!(
        registry.lookup("b", "a", at(200)),
        Err(disconnected("b", "a"))
    );
}

#[test]
fn out_of_range_carries_the_request_and_the_covered_range() {
    let registry = registry_of(&[sample("a", "b", 1.0, 10), sample("a", "b", 2.0, 20)]);

    for (target, source) in [("a", "b"), ("b", "a")] {
        assert_eq!(
            registry.lookup(target, source, at(9)),
            Err(out_of_range("b", 9, 10, 20))
        );
        assert_eq!(
            registry.lookup(target, source, at(21)),
            Err(out_of_range("b", 21, 10, 20))
        );
        assert!(registry.lookup(target, source, at(10)).is_ok());
        assert!(registry.lookup(target, source, at(20)).is_ok());
    }
}

#[test]
fn drained_edge_reports_no_data_and_not_a_range() {
    let mut registry = registry_of(&[sample("a", "b", 1.0, 10), sample("a", "b", 2.0, 20)]);
    registry.remove_samples_through(at(20));

    for nanos in [9, 10, 15, 20, 21] {
        assert_eq!(registry.lookup("a", "b", at(nanos)), Err(no_data("b")));
        assert_eq!(registry.lookup("b", "a", at(nanos)), Err(no_data("b")));
    }
}

#[test]
fn coverage_errors_render_the_range_or_its_absence() {
    let mut registry = registry_of(&[
        sample("a", "b", 1.0, 1_000_000_000),
        sample("a", "b", 2.0, 2_000_000_000),
    ]);

    let gap = registry.lookup("a", "b", at(3_000_000_000)).unwrap_err();
    assert_eq!(
        format!("{gap}"),
        "frame b covers 1.000000000 to 2.000000000 and not 3.000000000"
    );

    registry.remove_samples_through(at(2_000_000_000));
    let drained = registry.lookup("a", "b", at(3_000_000_000)).unwrap_err();
    assert_eq!(format!("{drained}"), "frame b holds no samples");
}

#[test]
fn chain_that_resolves_only_partway_reports_the_failing_edge() {
    // `a <- b` is known at t0 only; `b <- c` at t0 and t1. At t1 only the
    // lower hop resolves, and that partial answer must not be returned.
    let registry = registry_of(&[
        sample("a", "b", 1.0, 1_000_000_000),
        sample("b", "c", 1.0, 1_000_000_000),
        sample("b", "c", 1.0, 2_000_000_000),
    ]);
    let expected = out_of_range("b", 2_000_000_000, 1_000_000_000, 1_000_000_000);

    assert_eq!(
        registry.lookup("c", "a", at(2_000_000_000)),
        Err(expected.clone())
    );
    assert_eq!(registry.lookup("a", "c", at(2_000_000_000)), Err(expected));
}

#[test]
fn gap_in_the_middle_of_a_chain_reports_the_edge_with_the_gap() {
    // `r <- a <- b <- c`: the middle edge is known at t1 only, the others
    // at t1 and t3.
    let registry = registry_of(&[
        sample("r", "a", 1.0, 1_000_000_000),
        sample("r", "a", 1.0, 3_000_000_000),
        sample("b", "c", 1.0, 1_000_000_000),
        sample("b", "c", 1.0, 3_000_000_000),
        sample("a", "b", 1.0, 1_000_000_000),
    ]);

    assert!(registry.lookup("a", "c", at(1_000_000_000)).is_ok());
    assert_eq!(
        registry.lookup("a", "c", at(2_000_000_000)),
        Err(out_of_range(
            "b",
            2_000_000_000,
            1_000_000_000,
            1_000_000_000
        ))
    );
    assert_eq!(
        registry.lookup("r", "c", at(2_000_000_000)),
        Err(out_of_range(
            "b",
            2_000_000_000,
            1_000_000_000,
            1_000_000_000
        ))
    );
}

#[test]
fn coverage_error_names_the_lowest_failing_edge_of_a_side() {
    let registry = branching_tree(&["b", "c"]);

    // From `d` upward the edges are d, c, b, a: `c` is the lowest failing.
    assert_eq!(
        registry.lookup("d", "r", at(100)),
        Err(out_of_range("c", 100, 200, 200))
    );
    assert_eq!(
        registry.lookup("r", "d", at(100)),
        Err(out_of_range("c", 100, 200, 200))
    );
    // A chain that starts above `c` meets `b` first.
    assert_eq!(
        registry.lookup("b", "r", at(100)),
        Err(out_of_range("b", 100, 200, 200))
    );
}

#[test]
fn coverage_error_prefers_the_targets_side_of_the_chain() {
    // `y` and `d` meet at `a`. `x` fails on the side of `y`, and `b` and
    // `c` fail on the side of `d`.
    let registry = branching_tree(&["x", "b", "c"]);

    assert_eq!(
        registry.lookup("y", "d", at(100)),
        Err(out_of_range("x", 100, 200, 200))
    );
    assert_eq!(
        registry.lookup("d", "y", at(100)),
        Err(out_of_range("c", 100, 200, 200))
    );
}

#[test]
fn coverage_error_falls_to_the_sources_side_when_the_targets_side_is_covered() {
    let registry = branching_tree(&["b"]);

    assert_eq!(
        registry.lookup("y", "d", at(100)),
        Err(out_of_range("b", 100, 200, 200))
    );
    assert_eq!(
        registry.lookup("d", "y", at(100)),
        Err(out_of_range("b", 100, 200, 200))
    );
}

#[test]
fn coverage_error_never_names_an_edge_above_the_meeting_point() {
    // map <- odom <- {cam, base <- lidar}. At 50 two edges cannot serve:
    // `odom` and `base`. The cam/lidar chain meets at `odom` and never
    // crosses the edge above it.
    let registry = registry_of(&[
        sample("map", "odom", 1.0, 0),
        sample("odom", "cam", 1.0, 0),
        sample("odom", "cam", 1.0, 100),
        sample("odom", "base", 1.0, 200),
        sample("base", "lidar", 1.0, 0),
        sample("base", "lidar", 1.0, 100),
    ]);

    for (target, source) in [("cam", "lidar"), ("lidar", "cam")] {
        assert_eq!(
            registry.lookup(target, source, at(50)),
            Err(out_of_range("base", 50, 200, 200))
        );
    }
}

#[test]
fn lookup_is_unaffected_by_an_uncovered_edge_above_the_meeting_point() {
    // `a` is the meeting point of `y` and `d`; its own edge is out of range.
    let registry = branching_tree(&["a"]);

    let result = registry.lookup("y", "d", at(100)).unwrap();
    // `y` is at x = 2 in `a`, `d` at x = 3.
    assert_eq!(result.translation(), Vector3::new(1.0, 0.0, 0.0));
    assert!(registry.lookup("a", "d", at(100)).is_ok());
    assert!(registry.lookup("d", "a", at(100)).is_ok());

    // The same chain through the uncovered edge fails on it.
    assert_eq!(
        registry.lookup("r", "d", at(100)),
        Err(out_of_range("a", 100, 200, 200))
    );
}

#[test]
fn lookup_is_unaffected_by_a_drained_edge_above_the_meeting_point() {
    let mut registry = registry_of(&[
        sample("world", "map", 1.0, 1_000),
        sample("map", "a", 1.0, 10_000),
        sample("map", "a", 1.0, 20_000),
        sample("map", "b", 2.0, 15_000),
        sample("map", "b", 2.0, 25_000),
    ]);
    registry.remove_samples_through(at(5_000));

    let result = registry.lookup("a", "b", at(20_000)).unwrap();
    assert_eq!(result.translation(), Vector3::new(1.0, 0.0, 0.0));
    assert!(registry.lookup("map", "b", at(20_000)).is_ok());
    assert_eq!(
        registry.lookup("world", "b", at(20_000)),
        Err(no_data("map"))
    );
}

#[test]
fn covered_chain_that_overflows_is_non_finite_and_never_ok() {
    let registry = overflowing_tree(100, 100, 100);

    for (target, source) in [
        ("top", "c"),
        ("c", "top"),
        ("root", "c"),
        ("c", "root"),
        ("side", "c"),
        ("c", "side"),
        ("top", "b"),
        ("b", "top"),
    ] {
        assert_eq!(
            registry.lookup(target, source, at(100)),
            Err(LookupError::NonFinite),
            "{target} <- {source}"
        );
    }

    // Each hop alone is finite.
    assert!(registry.lookup("top", "a", at(100)).is_ok());
    assert!(registry.lookup("a", "c", at(100)).is_ok());
}

#[test]
fn siblings_whose_halves_are_finite_and_whose_combination_overflows_are_non_finite() {
    let registry = registry_of(&[
        mount("top", "left", 1.0e308),
        mount("top", "right", -1.0e308),
    ]);

    assert!(registry.lookup("top", "left", at(0)).is_ok());
    assert!(registry.lookup("right", "top", at(0)).is_ok());
    assert_eq!(
        registry.lookup("left", "right", at(0)),
        Err(LookupError::NonFinite)
    );
    assert_eq!(
        registry.lookup("right", "left", at(0)),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn coverage_below_an_overflow_is_reported_instead_of_the_overflow() {
    let registry = overflowing_tree(200, 100, 100);
    let expected = out_of_range("c", 100, 200, 200);

    for (target, source) in [("root", "c"), ("c", "root"), ("top", "c"), ("c", "side")] {
        assert_eq!(
            registry.lookup(target, source, at(100)),
            Err(expected.clone()),
            "{target} <- {source}"
        );
    }
}

#[test]
fn coverage_above_an_overflow_is_reported_instead_of_the_overflow() {
    // The walk from `c` overflows at `a`, before it reaches `top`.
    let registry = overflowing_tree(100, 200, 100);
    let expected = out_of_range("top", 100, 200, 200);

    for (target, source) in [("root", "c"), ("c", "root"), ("side", "c"), ("c", "side")] {
        assert_eq!(
            registry.lookup(target, source, at(100)),
            Err(expected.clone()),
            "{target} <- {source}"
        );
    }

    // `top` is the meeting point here, so its edge is not on the chain.
    assert_eq!(
        registry.lookup("top", "c", at(100)),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn coverage_on_the_other_side_of_an_overflow_is_reported_instead_of_the_overflow() {
    let registry = overflowing_tree(100, 100, 200);
    let expected = out_of_range("side", 100, 200, 200);

    // With `c` as the target its overflowing half is walked first.
    assert_eq!(registry.lookup("c", "side", at(100)), Err(expected.clone()));
    assert_eq!(registry.lookup("side", "c", at(100)), Err(expected));
}

#[test]
fn drained_edge_above_an_overflow_is_reported_as_no_data() {
    let mut registry = overflowing_tree(100, 50, 100);
    registry.remove_samples_through(at(50));

    assert_eq!(registry.lookup("root", "c", at(100)), Err(no_data("top")));
    assert_eq!(registry.lookup("c", "root", at(100)), Err(no_data("top")));
    assert_eq!(registry.lookup("c", "side", at(100)), Err(no_data("top")));
}

#[test]
fn frames_and_topology_are_diagnosed_before_an_overflow() {
    let registry = registry_of(&[
        sample("root", "a", 1.0, 1_000_000_000),
        mount("a", "b", 1.0e308),
        mount("b", "c", 1.0e308),
        mount("island", "x", 1.0),
    ]);

    for (target, source) in [("c", "missing"), ("missing", "c")] {
        assert_eq!(
            registry.lookup(target, source, at(1_000_000_000)),
            Err(unknown("missing"))
        );
    }
    assert_eq!(
        registry.lookup("c", "island", at(1_000_000_000)),
        Err(disconnected("c", "island"))
    );
    assert_eq!(
        registry.lookup("island", "c", at(1_000_000_000)),
        Err(disconnected("island", "c"))
    );
    assert_eq!(
        registry.lookup("c", "root", at(2_000_000_000)),
        Err(out_of_range(
            "a",
            2_000_000_000,
            1_000_000_000,
            1_000_000_000
        ))
    );
}

#[test]
fn single_hop_toward_the_parent_at_a_stored_instant_is_the_stored_transform_bit_for_bit() {
    let instant = 1_734_000_000_123_456_789;
    let stored = posed(
        "map",
        "lidar",
        Vector3::new(1.1, -2.2, 3.3),
        tilted(),
        Stamp::At(at(instant)),
    );
    let registry = registry_of(&[
        posed(
            "map",
            "lidar",
            Vector3::new(0.7, 0.1, -0.4),
            yaw(0.3),
            Stamp::At(at(instant - 1_000)),
        ),
        stored.clone(),
        posed(
            "map",
            "lidar",
            Vector3::new(-5.0, 0.2, 0.9),
            yaw(-1.1),
            Stamp::At(at(instant + 1_000)),
        ),
    ]);

    let result = registry.lookup("map", "lidar", at(instant)).unwrap();

    assert_eq!(result, stored);
    assert_eq!(transform_bits(&result), transform_bits(&stored));
}

#[test]
fn single_static_hop_toward_the_parent_is_the_stored_pose_bit_for_bit() {
    let stored = posed(
        "base",
        "camera",
        Vector3::new(0.1, 0.2, -0.3),
        tilted(),
        Stamp::Static,
    );
    let registry = registry_of(core::slice::from_ref(&stored));

    let result = registry.lookup("base", "camera", at(42)).unwrap();

    assert_eq!(result.stamp(), Stamp::At(at(42)));
    assert_eq!(transform_bits(&result), transform_bits(&stored));
}

/// Three hops with rotations and translations that round in every product.
fn rounding_hops(stamp: Stamp) -> [Transform; 3] {
    [
        posed("a", "b", Vector3::new(1.1, -2.2, 3.3), tilted(), stamp),
        posed("b", "c", Vector3::new(-0.7, 0.4, 0.9), yaw(0.7), stamp),
        posed(
            "c",
            "d",
            Vector3::new(0.3, 0.6, -1.3),
            Rotation::from_wxyz(0.5, 0.5, -0.5, 0.5).unwrap(),
            stamp,
        ),
    ]
}

#[test]
fn two_hops_toward_an_ancestor_equal_the_composed_hops_bit_for_bit() {
    let stamp = Stamp::At(at(1_000));
    let [a_from_b, b_from_c, _] = rounding_hops(stamp);
    let registry = registry_of(&[a_from_b.clone(), b_from_c.clone()]);

    let expected = a_from_b.compose(&b_from_c).unwrap();
    let result = registry.lookup("a", "c", at(1_000)).unwrap();

    assert_eq!(result, expected);
    assert_eq!(transform_bits(&result), transform_bits(&expected));
}

#[test]
fn several_hops_toward_an_ancestor_equal_the_composed_hops_bit_for_bit() {
    let stamp = Stamp::At(at(1_000));
    let [a_from_b, b_from_c, c_from_d] = rounding_hops(stamp);
    let registry = registry_of(&[a_from_b.clone(), b_from_c.clone(), c_from_d.clone()]);

    // The hops in chain order, each composed onto the hops below it. No
    // hop is inverted.
    let expected = a_from_b
        .compose(&b_from_c.compose(&c_from_d).unwrap())
        .unwrap();
    let result = registry.lookup("a", "d", at(1_000)).unwrap();

    assert_eq!(result, expected);
    assert_eq!(transform_bits(&result), transform_bits(&expected));
}

#[test]
fn several_static_and_dynamic_hops_toward_an_ancestor_equal_the_composed_hops_bit_for_bit() {
    let [a_from_b, _, c_from_d] = rounding_hops(Stamp::At(at(1_000)));
    let [_, b_from_c, _] = rounding_hops(Stamp::Static);
    let registry = registry_of(&[a_from_b.clone(), b_from_c.clone(), c_from_d.clone()]);

    let expected = a_from_b
        .compose(&b_from_c.compose(&c_from_d).unwrap())
        .unwrap();
    let result = registry.lookup("a", "d", at(1_000)).unwrap();

    assert_eq!(result, expected);
    assert_eq!(transform_bits(&result), transform_bits(&expected));
}

#[test]
fn lookup_toward_a_descendant_is_the_inverse_of_the_ancestor_lookup_bit_for_bit() {
    let registry = registry_of(&rounding_hops(Stamp::At(at(1_000))));

    for (ancestor, descendant) in [("a", "d"), ("a", "b"), ("b", "d")] {
        let expected = registry
            .lookup(ancestor, descendant, at(1_000))
            .unwrap()
            .inverse()
            .unwrap();
        let result = registry.lookup(descendant, ancestor, at(1_000)).unwrap();

        assert_eq!(result, expected);
        assert_eq!(transform_bits(&result), transform_bits(&expected));
    }
}

#[test]
fn sibling_lookup_is_the_inverted_target_half_composed_with_the_source_half_bit_for_bit() {
    let stamp = Stamp::At(at(1_000));
    let registry = registry_of(&[
        posed("root", "top", Vector3::new(9.0, 9.0, 9.0), yaw(2.0), stamp),
        posed("top", "p", Vector3::new(1.1, -2.2, 3.3), tilted(), stamp),
        posed("p", "q", Vector3::new(-0.7, 0.4, 0.9), yaw(0.7), stamp),
        posed("top", "u", Vector3::new(0.3, 0.6, -1.3), yaw(-1.9), stamp),
        posed(
            "u",
            "v",
            Vector3::new(4.4, 0.1, 0.2),
            tilted(),
            Stamp::Static,
        ),
    ]);

    let top_from_q = registry.lookup("top", "q", at(1_000)).unwrap();
    let top_from_v = registry.lookup("top", "v", at(1_000)).unwrap();
    let expected = top_from_q.inverse().unwrap().compose(&top_from_v).unwrap();
    let result = registry.lookup("q", "v", at(1_000)).unwrap();

    assert_eq!(result, expected);
    assert_eq!(transform_bits(&result), transform_bits(&expected));
}

#[test]
fn interpolation_is_linear_in_translation_and_slerp_in_rotation() {
    let quarter_turn = yaw(core::f64::consts::FRAC_PI_2);
    let registry = registry_of(&[
        posed(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
        posed(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            quarter_turn,
            Stamp::At(at(2_000_000_000)),
        ),
    ]);

    let result = registry.lookup("a", "b", at(1_500_000_000)).unwrap();

    assert_eq!(result.stamp(), Stamp::At(at(1_500_000_000)));
    assert_eq!(result.translation(), Vector3::new(0.5, 0.5, 0.0));
    let eighth_turn = yaw(core::f64::consts::FRAC_PI_4);
    assert_abs_diff_eq!(result.rotation().w(), eighth_turn.w(), epsilon = 1e-15);
    assert_abs_diff_eq!(result.rotation().x(), 0.0, epsilon = 1e-15);
    assert_abs_diff_eq!(result.rotation().y(), 0.0, epsilon = 1e-15);
    assert_abs_diff_eq!(result.rotation().z(), eighth_turn.z(), epsilon = 1e-15);
    assert_eq!(
        result.rotation(),
        Rotation::IDENTITY.slerp(quarter_turn, 0.5).unwrap()
    );
}

#[test]
fn interpolation_ratio_comes_from_integer_nanoseconds() {
    // Near this instant a second count held in an `f64` resolves no finer
    // than 256 ns, so a ratio computed from seconds could not be one third.
    let start = 1_700_000_000_000_000_001;
    let registry = registry_of(&[
        posed(
            "a",
            "b",
            Vector3::new(1.0, 10.0, -3.0),
            Rotation::IDENTITY,
            Stamp::At(at(start)),
        ),
        posed(
            "a",
            "b",
            Vector3::new(2.0, 40.0, 7.0),
            yaw(1.0),
            Stamp::At(at(start + 3)),
        ),
    ]);

    let ratio = 1.0 / 3.0;
    let keep = 1.0 - ratio;
    let expected = Vector3::new(
        keep * 1.0 + ratio * 2.0,
        keep * 10.0 + ratio * 40.0,
        keep * -3.0 + ratio * 7.0,
    );
    let result = registry.lookup("a", "b", at(start + 1)).unwrap();

    assert_eq!(
        bits(result.translation(), result.rotation()),
        bits(expected, Rotation::IDENTITY.slerp(yaw(1.0), ratio).unwrap())
    );
}

#[test]
fn interpolation_brackets_the_request_with_its_nearest_samples() {
    let registry = registry_of(&[
        sample("a", "b", 0.0, 0),
        sample("a", "b", 10.0, 10),
        sample("a", "b", 110.0, 20),
    ]);

    for (nanos, x) in [(5, 5.0), (15, 60.0), (10, 10.0), (0, 0.0), (20, 110.0)] {
        let result = registry.lookup("a", "b", at(nanos)).unwrap();
        assert_eq!(result.translation(), Vector3::new(x, 0.0, 0.0));
    }
}

#[test]
fn every_dynamic_hop_of_a_chain_is_interpolated() {
    let registry = registry_of(&[
        sample("a", "b", 1.0, 1_000_000_000),
        sample("a", "b", 2.0, 2_000_000_000),
        posed(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(1_000_000_000)),
        ),
        posed(
            "b",
            "c",
            Vector3::new(0.0, 2.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(2_000_000_000)),
        ),
    ]);

    let result = registry.lookup("a", "c", at(1_500_000_000)).unwrap();

    assert_eq!(result.translation(), Vector3::new(1.5, 1.5, 0.0));
    assert_eq!(result.stamp(), Stamp::At(at(1_500_000_000)));
}

#[test]
fn static_and_dynamic_edges_mix_in_one_chain() {
    // A static sensor mount under a base that moves from x = 1 to x = 3.
    let registry = registry_of(&[
        mount("base", "lidar", 0.5),
        sample("map", "base", 1.0, 1_000_000_000),
        sample("map", "base", 3.0, 3_000_000_000),
    ]);

    let result = registry.lookup("map", "lidar", at(2_000_000_000)).unwrap();
    assert_eq!(result.target(), "map");
    assert_eq!(result.source(), "lidar");
    assert_eq!(result.stamp(), Stamp::At(at(2_000_000_000)));
    assert_eq!(result.translation(), Vector3::new(2.5, 0.0, 0.0));

    let reverse = registry.lookup("lidar", "map", at(2_000_000_000)).unwrap();
    assert_eq!(reverse.translation(), Vector3::new(-2.5, 0.0, 0.0));

    // The static edge does not widen the dynamic edge's range.
    assert_eq!(
        registry.lookup("map", "lidar", at(3_000_000_001)),
        Err(out_of_range(
            "base",
            3_000_000_001,
            1_000_000_000,
            3_000_000_000
        ))
    );
}

#[test]
fn static_edge_below_a_dynamic_edge_serves_every_instant_of_the_dynamic_range() {
    let registry = registry_of(&[
        sample("map", "base", 1.0, 10),
        sample("map", "base", 2.0, 20),
        mount("base", "arm", 1.0),
        sample("arm", "tool", 0.0, 0),
        sample("arm", "tool", 4.0, 40),
    ]);

    let result = registry.lookup("map", "tool", at(15)).unwrap();
    assert_eq!(result.translation(), Vector3::new(4.0, 0.0, 0.0));
}

#[test]
fn deep_chain_resolves_in_both_directions() {
    let names: Vec<String> = (0..=64).map(|index| format!("f{index}")).collect();
    let mut registry = Registry::unbounded();
    for pair in names.windows(2) {
        let [parent, child] = pair else {
            unreachable!()
        };
        registry.insert(&sample(parent, child, 1.0, 100)).unwrap();
    }

    let down = registry.lookup("f0", "f64", at(100)).unwrap();
    assert_eq!(down.translation(), Vector3::new(64.0, 0.0, 0.0));
    assert_eq!(down.rotation(), Rotation::IDENTITY);

    let up = registry.lookup("f64", "f0", at(100)).unwrap();
    assert_eq!(up.translation(), Vector3::new(-64.0, 0.0, 0.0));

    let inner = registry.lookup("f10", "f50", at(100)).unwrap();
    assert_eq!(inner.translation(), Vector3::new(40.0, 0.0, 0.0));

    assert_eq!(
        registry.lookup("f0", "f64", at(101)),
        Err(out_of_range("f64", 101, 100, 100))
    );
}

#[test]
fn lookup_between_two_equal_samples_returns_their_rotation_bit_for_bit() {
    // Interpolating between two samples of one in-band rotation must not
    // renormalize it: the midpoint is that rotation.
    let rotation = Rotation::from_wxyz(1.0 + f64::EPSILON, 0.0, 0.0, 0.0).unwrap();
    let mut registry = Registry::unbounded();
    for nanos in [0, 2] {
        let sample = Transform::new(
            "map",
            "base",
            Vector3::new(1.0, 2.0, 3.0),
            rotation,
            Stamp::At(Timestamp::from_nanos(nanos)),
        )
        .unwrap();
        registry.insert(&sample).unwrap();
    }

    let between = registry
        .lookup("map", "base", Timestamp::from_nanos(1))
        .unwrap();

    assert_eq!(between.rotation().w().to_bits(), 0x3FF0_0000_0000_0001);
    assert_eq!(between.translation(), Vector3::new(1.0, 2.0, 3.0));
}
