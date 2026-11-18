//! `Registry::lookup_across`: two legs through a fixed frame.

use super::lookup::{
    at, bits, disconnected, mount, no_data, out_of_range, posed, registry_of, sample, tilted,
    transform_bits, unknown, yaw,
};
use crate::{
    CrossTimeTransform, LookupError, Registry, Rotation, Stamp, Timestamp, Vector3,
    geometry::Isometry,
};
use approx::assert_abs_diff_eq;

const T1: i64 = 1_000_000_000;
const T2: i64 = 2_000_000_000;
const T3: i64 = 3_000_000_000;

fn across_bits(across: &CrossTimeTransform) -> [u64; 7] {
    bits(across.translation(), across.rotation())
}

/// `world <- base <- camera` beside `world <- marker`, with rotations and
/// translations that round. `base` and `marker` move between `T1` and
/// `T3`; the camera mount is static.
fn moving_rig() -> Registry {
    registry_of(&[
        posed(
            "world",
            "base",
            Vector3::new(1.1, -2.2, 0.3),
            yaw(0.4),
            Stamp::At(at(T1)),
        ),
        posed(
            "world",
            "base",
            Vector3::new(4.7, 0.9, -0.6),
            yaw(1.9),
            Stamp::At(at(T3)),
        ),
        posed(
            "base",
            "camera",
            Vector3::new(0.2, 0.0, 0.5),
            tilted(),
            Stamp::Static,
        ),
        posed(
            "world",
            "marker",
            Vector3::new(-3.3, 7.1, 0.0),
            yaw(-0.8),
            Stamp::At(at(T1)),
        ),
        posed(
            "world",
            "marker",
            Vector3::new(-2.9, 6.4, 1.2),
            tilted(),
            Stamp::At(at(T3)),
        ),
    ])
}

/// `world <- a <- far`, where the two static edges each translate by 1e308
/// and compose to an overflow, beside the dynamic `world <- near`, which
/// holds one sample at `T2`.
fn overflowing_rig() -> Registry {
    registry_of(&[
        mount("world", "a", 1.0e308),
        mount("a", "far", 1.0e308),
        sample("world", "near", 1.0, T2),
    ])
}

#[test]
fn result_reports_both_frames_both_instants_and_the_geometry() {
    // A camera that moves from x = 1 to x = 2 in the world: where its
    // earlier origin lies in its later frame.
    let registry = registry_of(&[
        sample("world", "camera", 1.0, T1),
        sample("world", "camera", 2.0, T2),
    ]);

    let across = registry
        .lookup_across("camera", at(T2), "camera", at(T1), "world")
        .unwrap();

    assert_eq!(across.target(), "camera");
    assert_eq!(across.target_time(), at(T2));
    assert_eq!(across.source(), "camera");
    assert_eq!(across.source_time(), at(T1));
    assert_eq!(across.translation(), Vector3::new(-1.0, 0.0, 0.0));
    assert_eq!(across.rotation(), Rotation::IDENTITY);
}

#[test]
fn moving_base_is_observed_from_a_static_sensor_at_the_sources_instant() {
    let registry = registry_of(&[
        sample("world", "base", 0.0, T1),
        sample("world", "base", 5.0, T2),
        mount("world", "sensor", 10.0),
    ]);

    // The sensor does not move, so only the base's instant decides.
    for (target_time, source_time, x) in [
        (T2, T1, -10.0),
        (T1, T2, -5.0),
        (T3, T1, -10.0),
        (0, T2, -5.0),
        (T1, 1_500_000_000, -7.5),
    ] {
        let across = registry
            .lookup_across("sensor", at(target_time), "base", at(source_time), "world")
            .unwrap();
        assert_eq!(across.target(), "sensor");
        assert_eq!(across.source(), "base");
        assert_eq!(across.target_time(), at(target_time));
        assert_eq!(across.source_time(), at(source_time));
        assert_eq!(across.translation(), Vector3::new(x, 0.0, 0.0));
    }
}

#[test]
fn source_in_the_past_is_expressed_in_the_target_as_it_is_later() {
    // fixed <- a <- b. At T1 `b` is at (1, 1, 0) in `fixed`; at T2 `a` is
    // at x = 2. The earlier `b` in the later `a` is (-1, 1, 0).
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        sample("fixed", "a", 2.0, T2),
        posed(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(T1)),
        ),
        posed(
            "a",
            "b",
            Vector3::new(0.0, 2.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(T2)),
        ),
    ]);

    let across = registry
        .lookup_across("a", at(T2), "b", at(T1), "fixed")
        .unwrap();

    assert_eq!(across.translation(), Vector3::new(-1.0, 1.0, 0.0));
}

#[test]
fn endpoints_on_separate_branches_meet_in_the_fixed_frame() {
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        sample("fixed", "a", 2.0, T2),
        posed(
            "fixed",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(T1)),
        ),
        posed(
            "fixed",
            "b",
            Vector3::new(0.0, 2.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(T2)),
        ),
    ]);

    let across = registry
        .lookup_across("a", at(T2), "b", at(T1), "fixed")
        .unwrap();

    assert_eq!(across.translation(), Vector3::new(-2.0, 1.0, 0.0));
}

#[test]
fn rotated_target_leg_rotates_the_source_into_the_target() {
    // At T1 `b` is at (1.5, 0, 0) in `fixed`. At T2 `a` sits at the origin,
    // turned a quarter turn about z, which puts that point at (0, -1.5, 0).
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        posed(
            "fixed",
            "a",
            Vector3::new(0.0, 0.0, 0.0),
            yaw(core::f64::consts::FRAC_PI_2),
            Stamp::At(at(T2)),
        ),
        sample("a", "b", 0.5, T1),
        sample("a", "b", 0.5, T2),
    ]);

    let across = registry
        .lookup_across("a", at(T2), "b", at(T1), "fixed")
        .unwrap();

    assert_abs_diff_eq!(across.translation().x, 0.0, epsilon = 1e-15);
    assert_abs_diff_eq!(across.translation().y, -1.5, epsilon = 1e-15);
    assert_abs_diff_eq!(across.translation().z, 0.0, epsilon = 1e-15);
}

#[test]
fn equal_instants_agree_with_a_same_time_lookup() {
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        posed(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Rotation::IDENTITY,
            Stamp::At(at(T1)),
        ),
    ]);

    let same_time = registry.lookup("a", "b", at(T1)).unwrap();
    let across = registry
        .lookup_across("a", at(T1), "b", at(T1), "fixed")
        .unwrap();

    assert_eq!(across.translation(), same_time.translation());
    assert_eq!(across.rotation(), same_time.rotation());
}

#[test]
fn target_leg_that_is_the_fixed_frame_passes_the_source_leg_through_bit_for_bit() {
    let registry = moving_rig();

    for (source, source_time) in [
        ("camera", T1),
        ("camera", T2),
        ("base", 1_700_000_000),
        ("marker", T3),
    ] {
        let leg = registry.lookup("world", source, at(source_time)).unwrap();
        // The target's instant takes no part: the fixed frame is the target.
        for target_time in [T1, T2, 0, i64::MAX] {
            let across = registry
                .lookup_across("world", at(target_time), source, at(source_time), "world")
                .unwrap();
            assert_eq!(across_bits(&across), transform_bits(&leg));
            assert_eq!(across.target(), "world");
            assert_eq!(across.source(), source);
            assert_eq!(across.target_time(), at(target_time));
            assert_eq!(across.source_time(), at(source_time));
        }
    }
}

#[test]
fn source_leg_that_is_the_fixed_frame_gives_the_inverted_target_leg_bit_for_bit() {
    let registry = moving_rig();

    for (target, target_time) in [
        ("camera", T1),
        ("camera", T2),
        ("base", 1_700_000_000),
        ("marker", T3),
    ] {
        let leg = registry
            .lookup("world", target, at(target_time))
            .unwrap()
            .inverse()
            .unwrap();
        for source_time in [T1, T2, 0, i64::MAX] {
            let across = registry
                .lookup_across(target, at(target_time), "world", at(source_time), "world")
                .unwrap();
            assert_eq!(across_bits(&across), transform_bits(&leg));
            assert_eq!(across.target(), target);
            assert_eq!(across.source(), "world");
        }
    }
}

#[test]
fn endpoint_that_is_the_fixed_frame_contributes_no_motion() {
    let registry = registry_of(&[sample("fixed", "a", 1.0, T1), sample("fixed", "a", 2.0, T2)]);

    // The origin of the fixed frame in `a` at T2.
    let across = registry
        .lookup_across("a", at(T2), "fixed", at(T1), "fixed")
        .unwrap();
    assert_eq!(across.translation(), Vector3::new(-2.0, 0.0, 0.0));

    // `a` at T1, seen from the fixed frame.
    let across = registry
        .lookup_across("fixed", at(T2), "a", at(T1), "fixed")
        .unwrap();
    assert_eq!(across.translation(), Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn three_equal_known_names_are_the_identity_at_any_instants() {
    let registry = registry_of(&[sample("fixed", "a", 1.0, T1)]);

    // A root, and a child whose own edge covers neither instant.
    for frame in ["fixed", "a"] {
        let across = registry
            .lookup_across(frame, at(T2), frame, at(T3), frame)
            .unwrap();
        assert_eq!(across.target(), frame);
        assert_eq!(across.source(), frame);
        assert_eq!(across.target_time(), at(T2));
        assert_eq!(across.source_time(), at(T3));
        assert_eq!(
            across_bits(&across),
            bits(Vector3::new(0.0, 0.0, 0.0), Rotation::IDENTITY)
        );
    }
}

#[test]
fn equal_endpoints_at_two_instants_give_the_motion_between_them() {
    let registry = moving_rig();

    let world_from_then = registry.lookup("world", "camera", at(T1)).unwrap();
    let world_from_now = registry.lookup("world", "camera", at(T3)).unwrap();
    let across = registry
        .lookup_across("camera", at(T3), "camera", at(T1), "world")
        .unwrap();

    // The camera then, expressed in the camera now.
    let now = Isometry::new(world_from_now.translation(), world_from_now.rotation()).unwrap();
    let then = Isometry::new(world_from_then.translation(), world_from_then.rotation()).unwrap();
    let expected = now.inverse().unwrap().compose(then).unwrap();
    assert_eq!(
        across_bits(&across),
        bits(expected.translation(), expected.rotation())
    );
    // The camera moved, so this is not the identity.
    assert!(across.translation().x.abs() > 0.1);

    // At equal instants nothing moved, to within rounding.
    let still = registry
        .lookup_across("camera", at(T2), "camera", at(T2), "world")
        .unwrap();
    assert_abs_diff_eq!(still.translation().x, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(still.translation().y, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(still.translation().z, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(still.rotation().w().abs(), 1.0, epsilon = 1e-14);
}

#[test]
fn swapping_the_endpoints_with_their_instants_gives_the_inverse_relationship() {
    let registry = moving_rig();

    for (target, target_time, source, source_time) in [
        ("camera", T3, "marker", T1),
        ("camera", T1, "camera", T3),
        ("base", 1_700_000_000, "marker", 2_600_000_000),
        ("marker", T2, "camera", T2),
    ] {
        let forward = registry
            .lookup_across(target, at(target_time), source, at(source_time), "world")
            .unwrap();
        let backward = registry
            .lookup_across(source, at(source_time), target, at(target_time), "world")
            .unwrap();

        let round_trip = Isometry::new(forward.translation(), forward.rotation())
            .unwrap()
            .compose(Isometry::new(backward.translation(), backward.rotation()).unwrap())
            .unwrap();

        // The error of a round trip grows with the lengths involved.
        let reach = [forward.translation(), backward.translation()]
            .iter()
            .flat_map(|v| [v.x.abs(), v.y.abs(), v.z.abs()])
            .fold(1.0, f64::max);
        let bound = 64.0 * f64::EPSILON * reach;
        assert_abs_diff_eq!(round_trip.translation().x, 0.0, epsilon = bound);
        assert_abs_diff_eq!(round_trip.translation().y, 0.0, epsilon = bound);
        assert_abs_diff_eq!(round_trip.translation().z, 0.0, epsilon = bound);
        let tolerance = 64.0 * f64::EPSILON;
        assert_abs_diff_eq!(round_trip.rotation().w().abs(), 1.0, epsilon = tolerance);
        assert_abs_diff_eq!(round_trip.rotation().x(), 0.0, epsilon = tolerance);
        assert_abs_diff_eq!(round_trip.rotation().y(), 0.0, epsilon = tolerance);
        assert_abs_diff_eq!(round_trip.rotation().z(), 0.0, epsilon = tolerance);
    }
}

#[test]
fn unknown_names_are_reported_in_the_order_target_source_fixed() {
    let registry = registry_of(&[sample("fixed", "a", 1.0, T1), sample("a", "b", 1.0, T1)]);
    let lookup = |target, source, fixed| {
        registry
            .lookup_across(target, at(T1), source, at(T1), fixed)
            .unwrap_err()
    };

    assert_eq!(lookup("x", "y", "z"), unknown("x"));
    assert_eq!(lookup("a", "y", "z"), unknown("y"));
    assert_eq!(lookup("a", "b", "z"), unknown("z"));
    assert_eq!(lookup("x", "b", "fixed"), unknown("x"));
    assert_eq!(lookup("a", "y", "fixed"), unknown("y"));
    assert_eq!(lookup("", "b", "fixed"), unknown(""));
    assert_eq!(lookup("a", "b", ""), unknown(""));
}

#[test]
fn empty_registry_answers_a_cross_time_lookup_with_unknown_frame() {
    let registry = Registry::unbounded();

    assert_eq!(
        registry.lookup_across("a", at(T1), "a", at(T1), "a"),
        Err(unknown("a"))
    );
    assert_eq!(
        registry.lookup_across("a", at(T1), "b", at(T2), "c"),
        Err(unknown("a"))
    );
}

#[test]
fn unknown_fixed_frame_is_reported_although_the_endpoints_connect() {
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        sample("fixed", "a", 1.0, T2),
        sample("a", "b", 1.0, T1),
        sample("a", "b", 1.0, T2),
    ]);

    assert_eq!(
        registry.lookup_across("a", at(T2), "b", at(T1), "nowhere"),
        Err(unknown("nowhere"))
    );
    // Also before any coverage error on the endpoints.
    assert_eq!(
        registry.lookup_across("a", at(T3), "b", at(0), "nowhere"),
        Err(unknown("nowhere"))
    );
}

#[test]
fn disconnected_names_the_endpoint_and_the_fixed_frame_target_leg_first() {
    let registry = registry_of(&[
        sample("world", "a", 1.0, T1),
        sample("world", "b", 1.0, T1),
        sample("island", "x", 1.0, T1),
        sample("island", "y", 1.0, T1),
    ]);
    let lookup = |target, source, fixed| {
        registry
            .lookup_across(target, at(T1), source, at(T1), fixed)
            .unwrap_err()
    };

    // Only the target's leg does not connect.
    assert_eq!(lookup("x", "a", "world"), disconnected("x", "world"));
    // Only the source's leg does not connect.
    assert_eq!(lookup("a", "x", "world"), disconnected("x", "world"));
    // Neither connects: the target's leg is reported.
    assert_eq!(lookup("x", "y", "world"), disconnected("x", "world"));
    assert_eq!(lookup("y", "x", "b"), disconnected("y", "b"));
}

#[test]
fn disconnected_fixed_frame_is_an_error_although_endpoints_and_instants_coincide() {
    let registry = registry_of(&[
        sample("world", "a", 1.0, T1),
        sample("island", "x", 1.0, T1),
    ]);

    assert_eq!(
        registry.lookup_across("a", at(T1), "a", at(T1), "island"),
        Err(disconnected("a", "island"))
    );
    assert_eq!(
        registry.lookup_across("world", at(T1), "a", at(T1), "x"),
        Err(disconnected("world", "x"))
    );
}

#[test]
fn disconnected_on_either_leg_is_reported_before_coverage_on_either_leg() {
    let mut registry = registry_of(&[
        sample("world", "a", 1.0, T1),
        sample("island", "x", 1.0, T1),
    ]);

    // The target's leg is out of range at T3, the source's leg does not
    // connect.
    assert_eq!(
        registry.lookup_across("a", at(T3), "x", at(T1), "world"),
        Err(disconnected("x", "world"))
    );
    assert_eq!(
        registry.lookup_across("x", at(T1), "a", at(T3), "world"),
        Err(disconnected("x", "world"))
    );

    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        registry.lookup_across("a", at(T1), "x", at(T1), "world"),
        Err(disconnected("x", "world"))
    );
}

#[test]
fn coverage_error_is_reported_for_each_leg_at_its_own_instant() {
    // `fixed <- a` is known at T1 and T2; `a <- b` only at T1.
    let registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        sample("fixed", "a", 1.0, T2),
        sample("a", "b", 1.0, T1),
    ]);

    // The source's leg cannot resolve at T2.
    assert_eq!(
        registry.lookup_across("a", at(T1), "b", at(T2), "fixed"),
        Err(out_of_range("b", T2, T1, T1))
    );
    // The target's leg cannot resolve at T3: there is no extrapolation.
    assert_eq!(
        registry.lookup_across("a", at(T3), "b", at(T1), "fixed"),
        Err(out_of_range("a", T3, T1, T2))
    );
    assert!(
        registry
            .lookup_across("a", at(T2), "b", at(T1), "fixed")
            .is_ok()
    );
}

#[test]
fn coverage_error_of_the_target_leg_comes_before_the_source_leg() {
    let registry = registry_of(&[sample("fixed", "a", 1.0, T1), sample("fixed", "b", 1.0, T2)]);

    assert_eq!(
        registry.lookup_across("a", at(T3), "b", at(0), "fixed"),
        Err(out_of_range("a", T3, T1, T1))
    );
    assert_eq!(
        registry.lookup_across("b", at(0), "a", at(T3), "fixed"),
        Err(out_of_range("b", 0, T2, T2))
    );
}

#[test]
fn coverage_error_within_a_leg_checks_the_fixed_frames_side_first() {
    // The fixed frame `f` and the endpoint `e` hang side by side under
    // `top`, and both edges hold one sample at T2.
    let registry = registry_of(&[sample("top", "f", 1.0, T2), sample("top", "e", 1.0, T2)]);

    assert_eq!(
        registry.lookup_across("e", at(T1), "e", at(T2), "f"),
        Err(out_of_range("f", T1, T2, T2))
    );
    assert_eq!(
        registry.lookup_across("e", at(T2), "e", at(T3), "f"),
        Err(out_of_range("f", T3, T2, T2))
    );
}

#[test]
fn drained_edge_on_a_leg_is_reported_as_no_data() {
    let mut registry = registry_of(&[
        sample("fixed", "a", 1.0, T1),
        sample("fixed", "b", 1.0, T2),
        sample("fixed", "b", 1.0, T3),
    ]);
    registry.remove_samples_through(at(T1));

    assert_eq!(
        registry.lookup_across("a", at(T2), "b", at(T2), "fixed"),
        Err(no_data("a"))
    );
    assert_eq!(
        registry.lookup_across("b", at(T2), "a", at(T2), "fixed"),
        Err(no_data("a"))
    );
    // The target's leg is still first.
    assert_eq!(
        registry.lookup_across("b", at(T1), "a", at(T2), "fixed"),
        Err(out_of_range("b", T1, T2, T3))
    );
}

#[test]
fn covered_legs_that_overflow_are_non_finite() {
    let registry = overflowing_rig();

    // The overflow is inside one leg.
    assert_eq!(
        registry.lookup_across("far", at(T2), "near", at(T2), "world"),
        Err(LookupError::NonFinite)
    );
    assert_eq!(
        registry.lookup_across("near", at(T2), "far", at(T2), "world"),
        Err(LookupError::NonFinite)
    );
    assert_eq!(
        registry.lookup_across("world", at(T2), "far", at(T2), "world"),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn finite_legs_whose_combination_overflows_are_non_finite() {
    let registry = registry_of(&[
        mount("world", "left", 1.0e308),
        mount("world", "right", -1.0e308),
    ]);

    assert_eq!(
        registry.lookup_across("left", at(T1), "right", at(T2), "world"),
        Err(LookupError::NonFinite)
    );
    assert!(
        registry
            .lookup_across("left", at(T1), "world", at(T2), "world")
            .is_ok()
    );
}

#[test]
fn coverage_on_either_leg_beats_an_overflow_on_either_leg() {
    let registry = overflowing_rig();
    let expected = out_of_range("near", T1, T2, T2);

    // The target's leg overflows, the source's leg is out of range.
    assert_eq!(
        registry.lookup_across("far", at(T2), "near", at(T1), "world"),
        Err(expected.clone())
    );
    // The target's leg is out of range, the source's leg overflows.
    assert_eq!(
        registry.lookup_across("near", at(T1), "far", at(T2), "world"),
        Err(expected)
    );
}

#[test]
fn coverage_and_overflow_within_one_leg_report_the_coverage() {
    // The leg from `far` up to `world` overflows below the dynamic edge
    // `root <- world`, which holds one sample at T2.
    let registry = registry_of(&[
        sample("root", "world", 1.0, T2),
        mount("world", "a", 1.0e308),
        mount("a", "far", 1.0e308),
    ]);

    assert_eq!(
        registry.lookup_across("far", at(T1), "root", at(T2), "root"),
        Err(out_of_range("world", T1, T2, T2))
    );
    assert_eq!(
        registry.lookup_across("root", at(T2), "far", at(T1), "root"),
        Err(out_of_range("world", T1, T2, T2))
    );
    assert_eq!(
        registry.lookup_across("far", at(T2), "root", at(T2), "root"),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn reparented_edge_decides_by_where_the_fixed_frame_sits() {
    let mut registry = registry_of(&[
        sample("map_a", "odom", 1.0, T1),
        sample("map_a", "odom", 1.5, T2),
        sample("odom", "base", 2.0, T1),
        sample("odom", "base", 3.0, T2),
        sample("odom", "base", 4.0, T3),
    ]);
    registry
        .reparent(&sample("map_b", "odom", 5.0, T2))
        .unwrap();

    // Fixed frame below the moved edge: neither leg crosses it.
    let below = registry
        .lookup_across("base", at(T1), "base", at(T3), "odom")
        .unwrap();
    assert_eq!(below.translation(), Vector3::new(2.0, 0.0, 0.0));

    // Fixed frame above it: both legs cross the moved edge, which now
    // covers only the seed's instant.
    assert_eq!(
        registry.lookup_across("base", at(T1), "base", at(T3), "map_b"),
        Err(out_of_range("odom", T1, T2, T2))
    );
}
