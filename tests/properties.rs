//! Property-based tests for the core geometric and registry invariants.
//!
//! These run in both feature modes: nothing here relies on `std`-gated
//! library APIs such as `Timestamp::now()`.

use approx::abs_diff_eq;
use proptest::prelude::*;
use transforms::{
    Registry, Transformable,
    errors::{RegistryError, TransformError},
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

/// Tolerance for comparing computed against expected geometry.
const EPSILON: f64 = 1e-9;

/// 2^53 nanoseconds: the boundary beyond which `f64` can no longer represent
/// every nanosecond count exactly, and where `Timestamp::as_seconds` starts
/// refusing to convert.
const ACCURACY_CLIFF_NANOS: u64 = 1 << 53;

/// Finite translations, bounded so accumulated floating-point rounding stays
/// well below `EPSILON`.
fn translations() -> impl Strategy<Value = Vector3> {
    let axis = -100_000.0..100_000.0_f64;
    (axis.clone(), axis.clone(), axis).prop_map(|(x, y, z)| Vector3::new(x, y, z))
}

/// Unit quaternions built from a random axis and angle.
fn unit_quaternions() -> impl Strategy<Value = Quaternion> {
    let axis = -1.0..1.0_f64;
    (
        axis.clone(),
        axis.clone(),
        axis,
        0.0..core::f64::consts::TAU,
    )
        .prop_filter_map("axis too short to normalize", |(x, y, z, angle)| {
            let norm = (x * x + y * y + z * z).sqrt();
            if norm < 1e-3 {
                return None;
            }
            let half = angle / 2.0;
            let s = half.sin() / norm;
            Quaternion::from_wxyz(half.cos(), s * x, s * y, s * z)
                .normalize()
                .ok()
        })
}

/// Denormalizes a rotation by scaling every component: the tolerance tests
/// need norms deliberately off 1, and the public surface offers no
/// quaternion scaling — a rotation is not a vector here.
fn scaled(
    q: Quaternion,
    factor: f64,
) -> Quaternion {
    Quaternion::from_wxyz(q.w * factor, q.x * factor, q.y * factor, q.z * factor)
}

/// Nanosecond magnitudes spanning the regimes that behave differently:
/// small counts, both sides of the 2^53 `f64` accuracy cliff, 2020s
/// wall-clock nanoseconds (where an `f64` ulp is already ~256 ns), and the
/// top of the `u64` range. `headroom` keeps the highest band that far below
/// `u64::MAX`, for callers that add a span to the drawn value.
fn timestamp_nanos(headroom: u64) -> impl Strategy<Value = u64> {
    let top = u64::MAX - headroom;
    prop_oneof![
        0..1_000_000_000_000_000_u64,
        ACCURACY_CLIFF_NANOS - 1_000_000_000..ACCURACY_CLIFF_NANOS + 1_000_000_000,
        1_700_000_000_000_000_000_u64..1_800_000_000_000_000_000,
        top - 1_000_000_000..=top,
    ]
}

/// Dynamic nanosecond timestamps. `t = 0` is included: no value is
/// reserved — staticness is `Stamp::Static`, not a sentinel instant.
fn timestamps() -> impl Strategy<Value = Timestamp> {
    timestamp_nanos(0).prop_map(Timestamp::from_nanos)
}

/// The quaternion dot product; `|dot| ≈ 1` for unit quaternions means both
/// represent the same rotation (`q` and `-q` are the same rotation).
fn rotation_dot(
    a: Quaternion,
    b: Quaternion,
) -> f64 {
    a.w * b.w + a.x * b.x + a.y * b.y + a.z * b.z
}

fn same_rotation(
    a: Quaternion,
    b: Quaternion,
) -> bool {
    (rotation_dot(a, b).abs() - 1.0).abs() < EPSILON
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn inverse_twice_returns_the_original(
        translation in translations(),
        rotation in unit_quaternions(),
        timestamp in timestamps(),
    ) {
        let original = Transform::new("a", "b", translation, rotation, Stamp::At(timestamp)).unwrap();

        let roundtrip = original.inverse().unwrap().inverse().unwrap();

        prop_assert_eq!(roundtrip.parent(), "a");
        prop_assert_eq!(roundtrip.child(), "b");
        prop_assert_eq!(roundtrip.timestamp(), Stamp::At(timestamp));
        prop_assert!(
            abs_diff_eq!(roundtrip.translation(), original.translation(), epsilon = EPSILON),
            "translation drifted: {:?} vs {:?}",
            roundtrip.translation(),
            original.translation(),
        );
        prop_assert!(
            abs_diff_eq!(roundtrip.rotation(), original.rotation(), epsilon = EPSILON),
            "rotation drifted: {:?} vs {:?}",
            roundtrip.rotation(),
            original.rotation(),
        );
    }

    #[test]
    fn transform_and_inverse_return_point_to_origin(
        translation in translations(),
        rotation in unit_quaternions(),
        position in translations(),
        timestamp in timestamps(),
    ) {
        let transform = Transform::new("a", "b", translation, rotation, Stamp::At(timestamp)).unwrap();
        let mut point = Point::new(position, Quaternion::identity(), timestamp, "b");

        point.transform(&transform).unwrap();
        prop_assert_eq!(point.frame.as_str(), "a");

        point.transform(&transform.inverse().unwrap()).unwrap();
        prop_assert_eq!(point.frame.as_str(), "b");
        prop_assert!(
            abs_diff_eq!(point.position, position, epsilon = EPSILON),
            "position drifted: {:?} vs {:?}",
            point.position,
            position,
        );
    }

    #[test]
    fn slerp_endpoints_match_the_inputs(
        q1 in unit_quaternions(),
        q2 in unit_quaternions(),
    ) {
        let at_zero = q1.slerp(q2, 0.0);
        prop_assert!(
            same_rotation(at_zero, q1),
            "slerp at 0.0 is not q1: {at_zero:?} vs {q1:?}",
        );

        let at_one = q1.slerp(q2, 1.0);
        prop_assert!(
            same_rotation(at_one, q2),
            "slerp at 1.0 is not q2: {at_one:?} vs {q2:?}",
        );
    }

    #[test]
    fn slerp_output_is_unit_norm_at_interior_points(
        q1 in unit_quaternions(),
        q2 in unit_quaternions(),
        t in 0.0f64..=1.0,
    ) {
        // Covers the shortest-path flip and both interpolation branches:
        // the output of slerp must always be a valid rotation.
        let interpolated = q1.slerp(q2, t);
        prop_assert!(
            (interpolated.norm() - 1.0).abs() < 1e-9,
            "slerp output is not unit at t={t}: norm {}",
            interpolated.norm(),
        );
    }

    #[test]
    fn interpolate_is_exact_at_endpoints_and_rejects_outside(
        translation_from in translations(),
        translation_to in translations(),
        rotation_from in unit_quaternions(),
        rotation_to in unit_quaternions(),
        // `span` and `outside` are each below a second, so two seconds of
        // headroom keeps `start + span + outside` inside the range. A start
        // of at least 1 keeps the "strictly before" probe below it.
        start in timestamp_nanos(2_000_000_000).prop_map(|nanos| nanos.max(1)),
        span in 1..1_000_000_000_u64,
        outside in 1..1_000_000_000_u64,
    ) {
        let from = Transform::new("a", "b", translation_from, rotation_from, Stamp::At(Timestamp::from_nanos(start))).unwrap();
        let to = Transform::new("a", "b", translation_to, rotation_to, Stamp::At(Timestamp::from_nanos(start + span))).unwrap();

        let at_from = Transform::interpolate(&from, &to, from.timestamp().at().unwrap()).unwrap();
        prop_assert_eq!(at_from.timestamp(), from.timestamp());
        prop_assert_eq!(at_from.parent(), "a");
        prop_assert_eq!(at_from.child(), "b");
        prop_assert!(abs_diff_eq!(at_from.translation(), from.translation(), epsilon = EPSILON));
        prop_assert!(same_rotation(at_from.rotation(), from.rotation()));

        let at_to = Transform::interpolate(&from, &to, to.timestamp().at().unwrap()).unwrap();
        prop_assert_eq!(at_to.timestamp(), to.timestamp());
        prop_assert_eq!(at_to.parent(), "a");
        prop_assert_eq!(at_to.child(), "b");
        prop_assert!(abs_diff_eq!(at_to.translation(), to.translation(), epsilon = EPSILON));
        prop_assert!(same_rotation(at_to.rotation(), to.rotation()));

        // Strictly before the covered range (saturates to 0, still < start).
        let before = Timestamp::from_nanos(start.saturating_sub(outside));
        let result = Transform::interpolate(&from, &to, before);
        prop_assert!(
            matches!(result, Err(TransformError::TimestampOutOfRange { .. })),
            "expected TimestampOutOfRange before the range, got {result:?}",
        );

        // Strictly after the covered range.
        let after = Timestamp::from_nanos(start + span + outside);
        let result = Transform::interpolate(&from, &to, after);
        prop_assert!(
            matches!(result, Err(TransformError::TimestampOutOfRange { .. })),
            "expected TimestampOutOfRange after the range, got {result:?}",
        );
    }

    #[test]
    fn registry_chain_roundtrip_composes_to_identity(
        links in proptest::collection::vec((translations(), unit_quaternions()), 2..=5),
        timestamp in timestamps(),
    ) {
        let mut registry = Registry::new();
        for (i, (translation, rotation)) in links.iter().enumerate() {
            let transform = Transform::new(&format!("f{i}"), &format!("f{}", i + 1), *translation, *rotation, Stamp::At(timestamp)).unwrap();
            registry.add_transform(transform).unwrap();
        }

        let leaf = format!("f{}", links.len());
        let forward = registry.get_transform("f0", &leaf, timestamp).unwrap();
        prop_assert_eq!(forward.parent(), "f0");
        prop_assert_eq!(forward.child(), leaf.as_str());
        prop_assert_eq!(forward.timestamp(), Stamp::At(timestamp));

        let backward = registry.get_transform(&leaf, "f0", timestamp).unwrap();
        prop_assert_eq!(backward.parent(), leaf.as_str());
        prop_assert_eq!(backward.child(), "f0");
        prop_assert_eq!(backward.timestamp(), Stamp::At(timestamp));

        // The reverse lookup is the inverse: composing the two yields the
        // identity transform of the root frame.
        let composed = (forward * backward).unwrap();
        prop_assert!(
            abs_diff_eq!(composed.translation(), Vector3::zero(), epsilon = EPSILON),
            "composed translation is not zero: {:?}",
            composed.translation(),
        );
        prop_assert!(
            same_rotation(composed.rotation(), Quaternion::identity()),
            "composed rotation is not the identity: {:?}",
            composed.rotation(),
        );
    }

    #[test]
    fn latest_available_retry_is_exact_for_a_root_target(
        ranges in proptest::collection::vec((0_u64..1_000_000, 1_u64..1_000_000), 2..=4),
    ) {
        // Hop i covers [start, start + span]; the target f0 is the root, so
        // every frame a failed lookup reports is on the resolved chain and
        // the retry guard documented on `RegistryError::NotFoundAt` is
        // exact.
        let mut registry = Registry::new();
        for (i, (start, span)) in ranges.iter().enumerate() {
            for nanos in [*start, start + span] {
                let transform = Transform::new(
                    &format!("f{i}"),
                    &format!("f{}", i + 1),
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(Timestamp::from_nanos(nanos)),
                )
                .unwrap();
                registry.add_transform(transform).unwrap();
            }
        }
        let leaf = format!("f{}", ranges.len());
        let newest_common_end = ranges.iter().map(|(start, span)| start + span).min().unwrap();
        let latest_common_start = ranges.iter().map(|(start, _)| *start).max().unwrap();

        // Ask beyond every range, then lower the request onto each failing
        // frame's covered end — never raise it.
        let mut requested = Timestamp::from_nanos(2_000_001);
        let mut retries = 0_usize;
        let outcome = loop {
            match registry.get_transform("f0", &leaf, requested) {
                Ok(transform) => break Ok(transform),
                Err(RegistryError::NotFoundAt {
                    covered: Some((_, end)),
                    ..
                }) if end < requested => {
                    requested = end;
                    retries += 1;
                    prop_assert!(retries <= ranges.len(), "one retry per hop exceeded");
                }
                Err(error) => break Err(error),
            }
        };

        if latest_common_start <= newest_common_end {
            // The ranges share instants: the loop must land exactly on the
            // newest commonly covered one.
            let transform = outcome.unwrap();
            prop_assert_eq!(
                transform.timestamp(),
                Stamp::At(Timestamp::from_nanos(newest_common_end))
            );
        } else {
            // No instant is covered by every hop: the loop must error, not
            // fabricate a pose.
            prop_assert!(outcome.is_err());
        }
    }

    #[test]
    fn new_rejects_norms_beyond_tolerance(
        rotation in unit_quaternions(),
        deviation in 2e-6..1e-3_f64,
        above in any::<bool>(),
    ) {
        let factor = if above { 1.0 + deviation } else { 1.0 - deviation };
        let result = Transform::new(
            "a",
            "b",
            Vector3::zero(),
            scaled(rotation, factor),
            Stamp::At(Timestamp::from_nanos(1)),
        );

        prop_assert!(
            matches!(result, Err(TransformError::NonUnitRotation(_))),
            "norm deviation {deviation} must be rejected, got {result:?}",
        );
    }

    #[test]
    fn new_accepts_norms_within_tolerance(
        rotation in unit_quaternions(),
        deviation in -5e-7..5e-7_f64,
    ) {
        let result = Transform::new(
            "a",
            "b",
            Vector3::zero(),
            scaled(rotation, 1.0 + deviation),
            Stamp::At(Timestamp::from_nanos(1)),
        );

        prop_assert!(
            result.is_ok(),
            "norm deviation {deviation} must be accepted, got {result:?}",
        );
    }
}
