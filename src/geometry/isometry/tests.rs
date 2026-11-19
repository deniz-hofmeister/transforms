//! Unit tests.

use super::Isometry;
use crate::{Rotation, Vector3, geometry::NumericError};
use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;

/// The cosine and sine of an eighth of a turn's half angle (22.5 degrees):
/// a quarter-turn-free rotation whose components are all inexact.
const COS_PI_8: f64 = 0.923_879_532_511_286_7;
const SIN_PI_8: f64 = 0.382_683_432_365_089_8;

fn quarter_turn_z() -> Rotation {
    Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap()
}

fn half_turn_x() -> Rotation {
    Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap()
}

/// Rotations with inexact components, about different axes.
fn rotations() -> [Rotation; 5] {
    [
        Rotation::IDENTITY,
        quarter_turn_z(),
        Rotation::from_wxyz(COS_PI_8, SIN_PI_8, 0.0, 0.0).unwrap(),
        Rotation::from_wxyz(
            0.723_317_411_364_711_8,
            0.391_903_837_329_119_9,
            0.200_562_121_146_575_12,
            0.531_975_695_182_166_8,
        )
        .unwrap(),
        Rotation::from_wxyz(-0.1, 0.7, -0.7, 0.1).unwrap(),
    ]
}

fn bits(v: Vector3) -> [u64; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

fn largest_component(v: Vector3) -> f64 {
    v.x.abs().max(v.y.abs()).max(v.z.abs())
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
fn new_keeps_the_translation_and_rotation_it_was_given() {
    let translation = Vector3::new(1.0, -2.0, 3.0);
    let isometry = Isometry::new(translation, quarter_turn_z()).unwrap();

    assert_eq!(bits(isometry.translation()), bits(translation));
    assert_eq!(isometry.rotation(), quarter_turn_z());
}

#[test]
fn new_rejects_a_non_finite_translation_in_any_component() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for translation in [
            Vector3::new(bad, 0.0, 0.0),
            Vector3::new(0.0, bad, 0.0),
            Vector3::new(0.0, 0.0, bad),
        ] {
            assert_eq!(
                Isometry::new(translation, Rotation::IDENTITY),
                Err(NumericError::NonFinite)
            );
        }
    }
}

#[test]
fn new_accepts_every_finite_translation() {
    for translation in [
        Vector3::new(f64::MAX, f64::MIN, 0.0),
        Vector3::new(f64::MIN_POSITIVE, -0.0, 5e-324),
    ] {
        assert!(Isometry::new(translation, Rotation::IDENTITY).is_ok());
    }
}

#[test]
fn identity_is_the_zero_translation_and_no_rotation() {
    assert_eq!(bits(Isometry::IDENTITY.translation()), [0, 0, 0]);
    assert_eq!(Isometry::IDENTITY.rotation(), Rotation::IDENTITY);
}

// --- Composition --------------------------------------------------------

#[test]
fn compose_rotates_the_right_hand_translation_then_adds_the_left() {
    // Half turns are exact, so the expected values are too.
    let a = Isometry::new(Vector3::new(1.0, 0.0, 0.0), half_turn_x()).unwrap();
    let b = Isometry::new(Vector3::new(1.0, 2.0, 3.0), Rotation::IDENTITY).unwrap();

    // a * b: b's translation is turned by a's rotation, then a's is added.
    let a_b = a.compose(b).unwrap();
    assert_eq!(a_b.translation(), Vector3::new(2.0, -2.0, -3.0));
    assert_eq!(a_b.rotation(), half_turn_x());

    // b * a: b does not rotate, so the translations add.
    let b_a = b.compose(a).unwrap();
    assert_eq!(b_a.translation(), Vector3::new(2.0, 2.0, 3.0));
    assert_eq!(b_a.rotation(), half_turn_x());
}

#[test]
fn compose_multiplies_the_rotations_left_to_right() {
    let a = Isometry::new(Vector3::new(1.0, 2.0, 3.0), quarter_turn_z()).unwrap();
    let b = Isometry::new(Vector3::new(1.0, 0.0, 0.0), half_turn_x()).unwrap();

    let a_b = a.compose(b).unwrap();

    // The quarter turn about z takes b's x offset to y.
    assert_vector_close(
        a_b.translation(),
        Vector3::new(1.0, 3.0, 3.0),
        4.0 * f64::EPSILON,
    );
    assert_eq!(
        a_b.rotation(),
        quarter_turn_z().product(half_turn_x()).unwrap()
    );
    assert_ne!(
        a_b.rotation(),
        half_turn_x().product(quarter_turn_z()).unwrap()
    );
}

#[test]
fn compose_maps_a_point_as_its_operands_do_in_turn() {
    let apply = |isometry: Isometry, p: Vector3| {
        let r = isometry.rotation().rotate(p);
        let t = isometry.translation();
        Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z)
    };
    let p = Vector3::new(0.3, -0.7, 1.1);
    let translations = [
        Vector3::new(-1.0, 0.5, 0.25),
        Vector3::new(4.0, -3.0, 2.0),
        Vector3::new(0.0, 0.0, 0.0),
    ];

    for (i, &ra) in rotations().iter().enumerate() {
        for (j, &rb) in rotations().iter().enumerate() {
            let a = Isometry::new(translations[i % 3], ra).unwrap();
            let b = Isometry::new(translations[j % 3], rb).unwrap();

            assert_vector_close(
                apply(a.compose(b).unwrap(), p),
                apply(a, apply(b, p)),
                64.0 * f64::EPSILON,
            );
        }
    }
}

#[test]
fn compose_with_the_identity_keeps_the_value() {
    for rotation in rotations() {
        let isometry = Isometry::new(Vector3::new(1.0, -2.0, 3.0), rotation).unwrap();

        assert_eq!(isometry.compose(Isometry::IDENTITY).unwrap(), isometry);
        assert_eq!(
            Isometry::IDENTITY.compose(isometry).unwrap().rotation(),
            rotation
        );
        assert_eq!(
            Isometry::IDENTITY.compose(isometry).unwrap().translation(),
            isometry.translation()
        );
    }
}

#[test]
fn compose_reports_an_overflowing_translation() {
    let far = Isometry::new(Vector3::new(1e308, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    assert_eq!(far.compose(far), Err(NumericError::NonFinite));

    // One overflowing component is enough, on any axis.
    for translation in [
        Vector3::new(f64::MAX, 0.0, 0.0),
        Vector3::new(0.0, f64::MIN, 0.0),
        Vector3::new(0.0, 0.0, f64::MAX),
    ] {
        let edge = Isometry::new(translation, Rotation::IDENTITY).unwrap();
        assert_eq!(edge.compose(edge), Err(NumericError::NonFinite));
    }

    // The rotation of a large right-hand translation overflows on its own.
    let turned = Isometry::new(Vector3::new(0.0, 0.0, 0.0), quarter_turn_z()).unwrap();
    let diagonal =
        Isometry::new(Vector3::new(f64::MAX, f64::MAX, 0.0), Rotation::IDENTITY).unwrap();
    assert_eq!(turned.compose(diagonal), Err(NumericError::NonFinite));

    // The largest translation survives a composition that does not grow it.
    let top = Isometry::new(Vector3::new(f64::MAX, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    assert_eq!(top.compose(Isometry::IDENTITY), Ok(top));
}

// --- Inversion ----------------------------------------------------------

#[test]
fn inverse_of_a_pure_translation_negates_it() {
    let isometry = Isometry::new(Vector3::new(1.0, 2.0, 3.0), Rotation::IDENTITY).unwrap();

    let inverse = isometry.inverse().unwrap();

    assert_eq!(inverse.translation(), Vector3::new(-1.0, -2.0, -3.0));
    assert_eq!(inverse.rotation(), Rotation::IDENTITY);
}

#[test]
fn inverse_rotation_is_the_conjugate() {
    for rotation in rotations() {
        let isometry = Isometry::new(Vector3::new(1.0, -2.0, 3.0), rotation).unwrap();

        assert_eq!(isometry.inverse().unwrap().rotation(), rotation.conjugate());
    }
}

#[test]
fn inverse_translation_is_the_conjugate_rotated_original_over_the_negated_squared_norm() {
    for rotation in rotations() {
        for translation in [
            Vector3::new(1.0, -2.0, 3.0),
            Vector3::new(-1.0, 0.5, 0.25),
            Vector3::new(1.5e9, -2.5e-9, 7.0),
        ] {
            let isometry = Isometry::new(translation, rotation).unwrap();

            let rotated = rotation.conjugate().rotate(translation);
            let scale = -1.0 / rotation.norm_squared();
            let expected = Vector3::new(scale * rotated.x, scale * rotated.y, scale * rotated.z);

            assert_eq!(
                bits(isometry.inverse().unwrap().translation()),
                bits(expected)
            );
        }
    }
}

#[test]
fn inverse_divides_out_the_squared_norm_of_an_off_unit_rotation() {
    // A rotation at the edge of the band: 1 + 8 EPS in the squared norm.
    // The sandwich product scales by that, and the inverse divides it out.
    let stretched = Rotation::from_wxyz(1.0 + 4.0 * f64::EPSILON, 0.0, 0.0, 0.0).unwrap();
    assert_eq!(
        (stretched.norm_squared() - 1.0).to_bits(),
        (8.0 * f64::EPSILON).to_bits()
    );
    let isometry = Isometry::new(Vector3::new(1.0, 2.0, 3.0), stretched).unwrap();

    let inverse = isometry.inverse().unwrap();

    assert_vector_close(
        inverse.translation(),
        Vector3::new(-1.0, -2.0, -3.0),
        2.0 * f64::EPSILON,
    );
}

#[test]
fn inverse_of_a_zero_translation_compares_equal_to_zero() {
    for rotation in rotations() {
        let isometry = Isometry::new(Vector3::new(0.0, 0.0, 0.0), rotation).unwrap();

        let inverse = isometry.inverse().unwrap();

        // The components are negative zeros, which equal positive ones.
        assert_eq!(inverse.translation(), Vector3::new(0.0, 0.0, 0.0));
    }
    assert_eq!(Isometry::IDENTITY.inverse(), Ok(Isometry::IDENTITY));
}

#[test]
fn compose_with_inverse_is_the_identity_within_a_magnitude_aware_bound() {
    // The translation error of `x * x^-1` grows with the translation, so
    // the bound is relative to its largest component: an absolute pin
    // fails for a far-away frame and tests nothing for a near one.
    for rotation in rotations() {
        for scale in [1e-6, 1.0, 1e3, 1e9, 1e15, 1e150] {
            let translation = Vector3::new(1.0 * scale, -2.0 * scale, 3.0 * scale);
            let isometry = Isometry::new(translation, rotation).unwrap();
            let inverse = isometry.inverse().unwrap();
            let bound = 8.0 * f64::EPSILON * largest_component(translation);

            for round_trip in [
                isometry.compose(inverse).unwrap(),
                inverse.compose(isometry).unwrap(),
            ] {
                let residual = round_trip.translation();
                assert!(
                    largest_component(residual) <= bound,
                    "residual {residual:?} exceeds {bound:e} for {isometry:?}"
                );

                let r = round_trip.rotation();
                assert_abs_diff_eq!(r.w(), 1.0, epsilon = 8.0 * f64::EPSILON);
                assert_abs_diff_eq!(r.x(), 0.0, epsilon = 2.0 * f64::EPSILON);
                assert_abs_diff_eq!(r.y(), 0.0, epsilon = 2.0 * f64::EPSILON);
                assert_abs_diff_eq!(r.z(), 0.0, epsilon = 2.0 * f64::EPSILON);
            }
        }
    }
}

#[test]
fn inverse_of_an_inverse_returns_to_the_original_within_rounding() {
    for rotation in rotations() {
        let translation = Vector3::new(1.0, -2.0, 3.0);
        let isometry = Isometry::new(translation, rotation).unwrap();

        let back = isometry.inverse().unwrap().inverse().unwrap();

        // The conjugate is exact, so the rotation returns bit for bit.
        assert_eq!(back.rotation(), rotation);
        assert_vector_close(back.translation(), translation, 16.0 * f64::EPSILON);
    }
}

#[test]
fn inverse_reports_overflow_in_any_single_translation_component() {
    // A finite, valid input can overflow one output axis while the other
    // two stay finite: the rotated component is 1.28e308 * (cos 45 + sin 45).
    for (translation, rotation) in [
        (
            Vector3::new(1.28e308, 1.28e308, 0.0),
            Rotation::from_wxyz(COS_PI_8, 0.0, 0.0, SIN_PI_8).unwrap(),
        ),
        (
            Vector3::new(0.0, 1.28e308, 1.28e308),
            Rotation::from_wxyz(COS_PI_8, SIN_PI_8, 0.0, 0.0).unwrap(),
        ),
        (
            Vector3::new(1.28e308, 0.0, 1.28e308),
            Rotation::from_wxyz(COS_PI_8, 0.0, SIN_PI_8, 0.0).unwrap(),
        ),
    ] {
        let isometry = Isometry::new(translation, rotation).unwrap();
        assert_eq!(isometry.inverse(), Err(NumericError::NonFinite));
    }
}

#[test]
fn inverse_near_f64_max_is_reported_or_exact() {
    // Under a rotation, f64::MAX on two axes overflows.
    let turned = Isometry::new(Vector3::new(f64::MAX, f64::MAX, 0.0), quarter_turn_z()).unwrap();
    assert_eq!(turned.inverse(), Err(NumericError::NonFinite));

    // Without one, negation is exact and the extreme value inverts.
    let straight =
        Isometry::new(Vector3::new(f64::MAX, f64::MIN, 0.0), Rotation::IDENTITY).unwrap();
    assert_eq!(
        straight.inverse().unwrap().translation(),
        Vector3::new(f64::MIN, f64::MAX, 0.0)
    );
}

// --- Interpolation ------------------------------------------------------

#[test]
fn interpolate_at_ratio_zero_is_the_first_endpoint() {
    let from = Isometry::new(Vector3::new(1.0, -2.0, 3.0), Rotation::IDENTITY).unwrap();
    let to = Isometry::new(Vector3::new(5.0, 6.0, -7.0), quarter_turn_z()).unwrap();

    assert_eq!(from.interpolate(to, 0.0), Ok(from));
}

#[test]
fn interpolate_at_ratio_one_is_the_second_endpoint() {
    let from = Isometry::new(Vector3::new(1.0, -2.0, 3.0), Rotation::IDENTITY).unwrap();
    let to = Isometry::new(Vector3::new(5.0, 6.0, -7.0), quarter_turn_z()).unwrap();

    assert_eq!(from.interpolate(to, 1.0), Ok(to));
}

#[test]
fn interpolate_at_one_half_is_the_midpoint() {
    let from = Isometry::new(Vector3::new(1.0, -2.0, 3.0), Rotation::IDENTITY).unwrap();
    let to = Isometry::new(Vector3::new(5.0, 6.0, -7.0), quarter_turn_z()).unwrap();

    let middle = from.interpolate(to, 0.5).unwrap();

    // Linear in translation: halves of small integers are exact.
    assert_eq!(middle.translation(), Vector3::new(3.0, 2.0, -2.0));
    // Slerp in rotation: half of a quarter turn about z.
    let rotation = middle.rotation();
    assert_abs_diff_eq!(rotation.w(), COS_PI_8, epsilon = f64::EPSILON);
    assert_abs_diff_eq!(rotation.x(), 0.0, epsilon = f64::EPSILON);
    assert_abs_diff_eq!(rotation.y(), 0.0, epsilon = f64::EPSILON);
    assert_abs_diff_eq!(rotation.z(), SIN_PI_8, epsilon = f64::EPSILON);
    assert_eq!(
        rotation,
        Rotation::IDENTITY.slerp(quarter_turn_z(), 0.5).unwrap()
    );
}

#[test]
fn interpolate_weights_each_endpoint_with_its_own_factor() {
    // Asymmetric: swapped weights would give 4.0 where 2.0 is expected.
    let from = Isometry::new(Vector3::new(1.0, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    let to = Isometry::new(Vector3::new(5.0, 0.0, 0.0), Rotation::IDENTITY).unwrap();

    let quarter = from.interpolate(to, 0.25).unwrap();

    assert_eq!(quarter.translation(), Vector3::new(2.0, 0.0, 0.0));
    assert_eq!(quarter.rotation(), Rotation::IDENTITY);
}

#[test]
fn interpolate_between_equal_endpoints_keeps_extreme_translations() {
    // The blend of a value with itself stays in range at the top of f64.
    let top = Isometry::new(Vector3::new(f64::MAX, f64::MIN, 0.0), Rotation::IDENTITY).unwrap();

    assert_eq!(top.interpolate(top, 0.5), Ok(top));
}

#[test]
fn interpolate_reports_an_overflowing_translation() {
    // The translation blend is not clamped, only the slerp is. A ratio
    // outside [0.0, 1.0], which no caller passes, pushes the blend of two
    // opposite extremes out of range, and the result is refused.
    let low = Isometry::new(Vector3::new(f64::MIN, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    let high = Isometry::new(Vector3::new(f64::MAX, 0.0, 0.0), Rotation::IDENTITY).unwrap();

    assert_eq!(low.interpolate(high, 2.0), Err(NumericError::NonFinite));
    assert_eq!(high.interpolate(low, -1.0), Err(NumericError::NonFinite));

    for (from, to) in [
        (
            Vector3::new(0.0, f64::MIN, 0.0),
            Vector3::new(0.0, f64::MAX, 0.0),
        ),
        (
            Vector3::new(0.0, 0.0, f64::MIN),
            Vector3::new(0.0, 0.0, f64::MAX),
        ),
    ] {
        let from = Isometry::new(from, Rotation::IDENTITY).unwrap();
        let to = Isometry::new(to, Rotation::IDENTITY).unwrap();
        assert_eq!(from.interpolate(to, 2.0), Err(NumericError::NonFinite));
    }

    // Inside the interval the same endpoints blend to a finite value.
    assert_eq!(
        low.interpolate(high, 0.5).unwrap().translation(),
        Vector3::new(0.0, 0.0, 0.0)
    );
}

#[test]
fn interpolate_reports_a_nan_ratio() {
    let from = Isometry::new(Vector3::new(1.0, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    let to = Isometry::new(Vector3::new(2.0, 0.0, 0.0), quarter_turn_z()).unwrap();

    assert_eq!(from.interpolate(to, f64::NAN), Err(NumericError::NonFinite));
}
