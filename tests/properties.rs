//! Property-based tests for the core geometric and registry invariants.
//!
//! These run in both feature modes: nothing here relies on `std`-gated
//! library APIs such as `Timestamp::now()`.

use approx::abs_diff_eq;
use proptest::prelude::*;
use transforms::{
    Registry, Transformable,
    errors::TransformError,
    geometry::{Point, Quaternion, Transform, Vector3},
    time::Timestamp,
};

/// Tolerance for comparing computed against expected geometry.
const EPSILON: f64 = 1e-9;

/// Upper bound (exclusive) of the generated timestamp range, in nanoseconds.
const MAX_NANOS: u128 = 1_000_000_000_000_000;

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
            Quaternion::new(half.cos(), s * x, s * y, s * z)
                .normalize()
                .ok()
        })
}

/// Dynamic nanosecond timestamps; `t = 0` is the static sentinel and is
/// deliberately excluded.
fn timestamps() -> impl Strategy<Value = Timestamp> {
    (1..MAX_NANOS).prop_map(Timestamp::from_nanos)
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
        let original = Transform {
            translation,
            rotation,
            timestamp,
            parent: "a".into(),
            child: "b".into(),
        };

        let roundtrip = original.inverse().unwrap().inverse().unwrap();

        prop_assert_eq!(roundtrip.parent.as_str(), "a");
        prop_assert_eq!(roundtrip.child.as_str(), "b");
        prop_assert_eq!(roundtrip.timestamp, timestamp);
        prop_assert!(
            abs_diff_eq!(roundtrip.translation, original.translation, epsilon = EPSILON),
            "translation drifted: {:?} vs {:?}",
            roundtrip.translation,
            original.translation,
        );
        prop_assert!(
            abs_diff_eq!(roundtrip.rotation, original.rotation, epsilon = EPSILON),
            "rotation drifted: {:?} vs {:?}",
            roundtrip.rotation,
            original.rotation,
        );
    }

    #[test]
    fn transform_and_inverse_return_point_to_origin(
        translation in translations(),
        rotation in unit_quaternions(),
        position in translations(),
        timestamp in timestamps(),
    ) {
        let transform = Transform {
            translation,
            rotation,
            timestamp,
            parent: "a".into(),
            child: "b".into(),
        };
        let mut point = Point {
            position,
            orientation: Quaternion::identity(),
            timestamp,
            frame: "b".into(),
        };

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
        start in 1..MAX_NANOS,
        span in 1..1_000_000_000_u128,
        outside in 1..1_000_000_000_u128,
    ) {
        let from = Transform {
            translation: translation_from,
            rotation: rotation_from,
            timestamp: Timestamp::from_nanos(start),
            parent: "a".into(),
            child: "b".into(),
        };
        let to = Transform {
            translation: translation_to,
            rotation: rotation_to,
            timestamp: Timestamp::from_nanos(start + span),
            parent: "a".into(),
            child: "b".into(),
        };

        let at_from = Transform::interpolate(&from, &to, from.timestamp).unwrap();
        prop_assert_eq!(at_from.timestamp, from.timestamp);
        prop_assert_eq!(at_from.parent.as_str(), "a");
        prop_assert_eq!(at_from.child.as_str(), "b");
        prop_assert!(abs_diff_eq!(at_from.translation, from.translation, epsilon = EPSILON));
        prop_assert!(same_rotation(at_from.rotation, from.rotation));

        let at_to = Transform::interpolate(&from, &to, to.timestamp).unwrap();
        prop_assert_eq!(at_to.timestamp, to.timestamp);
        prop_assert_eq!(at_to.parent.as_str(), "a");
        prop_assert_eq!(at_to.child.as_str(), "b");
        prop_assert!(abs_diff_eq!(at_to.translation, to.translation, epsilon = EPSILON));
        prop_assert!(same_rotation(at_to.rotation, to.rotation));

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
            let transform = Transform {
                translation: *translation,
                rotation: *rotation,
                timestamp,
                parent: format!("f{i}"),
                child: format!("f{}", i + 1),
            };
            transform.validate().unwrap();
            registry.add_transform(transform).unwrap();
        }

        let leaf = format!("f{}", links.len());
        let forward = registry.get_transform("f0", &leaf, timestamp).unwrap();
        prop_assert_eq!(forward.parent.as_str(), "f0");
        prop_assert_eq!(forward.child.as_str(), leaf.as_str());
        prop_assert_eq!(forward.timestamp, timestamp);

        let backward = registry.get_transform(&leaf, "f0", timestamp).unwrap();
        prop_assert_eq!(backward.parent.as_str(), leaf.as_str());
        prop_assert_eq!(backward.child.as_str(), "f0");
        prop_assert_eq!(backward.timestamp, timestamp);

        // The reverse lookup is the inverse: composing the two yields the
        // identity transform of the root frame.
        let composed = (forward * backward).unwrap();
        prop_assert!(
            abs_diff_eq!(composed.translation, Vector3::zero(), epsilon = EPSILON),
            "composed translation is not zero: {:?}",
            composed.translation,
        );
        prop_assert!(
            same_rotation(composed.rotation, Quaternion::identity()),
            "composed rotation is not the identity: {:?}",
            composed.rotation,
        );
    }

    #[test]
    fn validate_rejects_norms_beyond_tolerance(
        rotation in unit_quaternions(),
        deviation in 2e-6..1e-3_f64,
        above in any::<bool>(),
    ) {
        let factor = if above { 1.0 + deviation } else { 1.0 - deviation };
        let transform = Transform {
            translation: Vector3::zero(),
            rotation: rotation.scale(factor),
            timestamp: Timestamp::from_nanos(1),
            parent: "a".into(),
            child: "b".into(),
        };

        let result = transform.validate();
        prop_assert!(
            matches!(result, Err(TransformError::NonUnitRotation(_))),
            "norm deviation {deviation} must be rejected, got {result:?}",
        );
    }

    #[test]
    fn validate_accepts_norms_within_tolerance(
        rotation in unit_quaternions(),
        deviation in -5e-7..5e-7_f64,
    ) {
        let transform = Transform {
            translation: Vector3::zero(),
            rotation: rotation.scale(1.0 + deviation),
            timestamp: Timestamp::from_nanos(1),
            parent: "a".into(),
            child: "b".into(),
        };

        let result = transform.validate();
        prop_assert!(
            result.is_ok(),
            "norm deviation {deviation} must be accepted, got {result:?}",
        );
    }
}
