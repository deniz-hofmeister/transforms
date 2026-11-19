//! Unit tests.

use super::{Quaternion, Rotation};
use crate::{TransformError, Vector3, geometry::NumericError};
use alloc::{format, vec::Vec};
use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;

/// The widest distance from 1 a rotation's squared norm may have, written
/// out here so that the tests pin the value and not the constant's name.
const BAND: f64 = 8.0 * f64::EPSILON;

/// The four components as raw bit patterns, for exact comparison.
fn bits(q: Rotation) -> [u64; 4] {
    [
        q.w().to_bits(),
        q.x().to_bits(),
        q.y().to_bits(),
        q.z().to_bits(),
    ]
}

fn in_band(q: Rotation) -> bool {
    (q.norm_squared() - 1.0).abs() <= BAND
}

/// Builds a rotation from components of any non-zero length by dividing
/// them by their norm first.
fn unit(
    w: f64,
    x: f64,
    y: f64,
    z: f64,
) -> Rotation {
    let norm = libm::sqrt(w * w + x * x + y * y + z * z);
    Rotation::from_wxyz(w / norm, x / norm, y / norm, z / norm).unwrap()
}

/// A rotation of `angle` radians about the z-axis.
fn rotation_about_z(angle: f64) -> Rotation {
    unit(libm::cos(angle / 2.0), 0.0, 0.0, libm::sin(angle / 2.0))
}

/// The same rotation with every component negated.
fn negated(q: Rotation) -> Rotation {
    Rotation::from_wxyz(-q.w(), -q.x(), -q.y(), -q.z()).unwrap()
}

fn dot(
    a: Rotation,
    b: Rotation,
) -> f64 {
    a.w() * b.w() + a.x() * b.x() + a.y() * b.y() + a.z() * b.z()
}

/// Rotational agreement up to sign (q and -q are the same rotation).
fn same_rotation_up_to_sign(
    a: Rotation,
    b: Rotation,
) -> bool {
    (dot(a, b).abs() - 1.0).abs() < 1e-12
}

fn assert_components_close(
    actual: Rotation,
    expected: Rotation,
    epsilon: f64,
) {
    assert_abs_diff_eq!(actual.w(), expected.w(), epsilon = epsilon);
    assert_abs_diff_eq!(actual.x(), expected.x(), epsilon = epsilon);
    assert_abs_diff_eq!(actual.y(), expected.y(), epsilon = epsilon);
    assert_abs_diff_eq!(actual.z(), expected.z(), epsilon = epsilon);
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

/// A fixed-seed linear congruential generator: the same rotations on every
/// run and every host.
struct Sweep(u64);

impl Sweep {
    /// The next value in `[0.0, 1.0)`.
    fn fraction(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let high = u32::try_from(self.0 >> 32).unwrap();
        f64::from(high) / 4_294_967_296.0
    }

    /// The next value in `[-1.0, 1.0)`.
    fn signed(&mut self) -> f64 {
        2.0 * self.fraction() - 1.0
    }

    /// The next rotation, uniform in direction over the 3-sphere's cube
    /// projection, which is spread enough for a sweep.
    fn rotation(&mut self) -> Rotation {
        loop {
            let (w, x, y, z) = (self.signed(), self.signed(), self.signed(), self.signed());
            if w * w + x * x + y * y + z * z > 0.01 {
                return unit(w, x, y, z);
            }
        }
    }
}

// --- Construction -------------------------------------------------------

#[test]
fn identity_is_the_scalar_one() {
    assert_eq!(bits(Rotation::IDENTITY), [1.0_f64.to_bits(), 0, 0, 0]);
    assert_eq!(
        Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0),
        Ok(Rotation::IDENTITY)
    );
}

#[test]
fn accessors_return_the_components_in_scalar_first_order() {
    let q = Rotation::from_wxyz(0.5, -0.5, 0.5, -0.5).unwrap();

    assert_eq!(q.w().to_bits(), 0.5_f64.to_bits());
    assert_eq!(q.x().to_bits(), (-0.5_f64).to_bits());
    assert_eq!(q.y().to_bits(), 0.5_f64.to_bits());
    assert_eq!(q.z().to_bits(), (-0.5_f64).to_bits());
}

#[test]
fn in_band_input_is_stored_bit_for_bit() {
    // One, four and eight EPSILON off in the squared norm: each inside the
    // band, each kept as given.
    let inputs = [
        (0.5, 0.5, 0.5, 0.5),
        (FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2),
        (0.0, -FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0),
        (1.0 + 2.0 * f64::EPSILON, 0.0, 0.0, 0.0),
        (1.0 - 2.0 * f64::EPSILON, 0.0, 0.0, 0.0),
        (0.0, 0.0, 1.0 + 4.0 * f64::EPSILON, 0.0),
        (0.0, 1.0 - 4.0 * f64::EPSILON, 0.0, 0.0),
        (0.819_648_017_845_479_5, 0.572_867_460_100_481_3, 0.0, 0.0),
        (-0.0, 0.0, -1.0, -0.0),
    ];

    for (w, x, y, z) in inputs {
        let q = Rotation::from_wxyz(w, x, y, z).unwrap();
        assert_eq!(
            bits(q),
            [w.to_bits(), x.to_bits(), y.to_bits(), z.to_bits()],
            "({w:e}, {x:e}, {y:e}, {z:e}) must be stored unchanged"
        );
        assert!(in_band(q));
    }
}

#[test]
fn the_band_ends_at_eight_epsilon_in_the_squared_norm() {
    // (1 + 4 EPS)^2 rounds to 1 + 8 EPS: the last squared norm kept as is.
    let last_kept = 1.0 + 4.0 * f64::EPSILON;
    assert_eq!(
        (last_kept * last_kept - 1.0).to_bits(),
        (8.0 * f64::EPSILON).to_bits()
    );
    let kept = Rotation::from_wxyz(last_kept, 0.0, 0.0, 0.0).unwrap();
    assert_eq!(kept.w().to_bits(), last_kept.to_bits());

    // (1 + 5 EPS)^2 rounds to 1 + 10 EPS: outside, so the input is scaled.
    let first_scaled = 1.0 + 5.0 * f64::EPSILON;
    let scaled = Rotation::from_wxyz(first_scaled, 0.0, 0.0, 0.0).unwrap();
    assert_ne!(scaled.w().to_bits(), first_scaled.to_bits());
    assert_eq!(scaled, Rotation::IDENTITY);

    let below = 1.0 - 5.0 * f64::EPSILON;
    let scaled = Rotation::from_wxyz(0.0, below, 0.0, 0.0).unwrap();
    assert_ne!(scaled.x().to_bits(), below.to_bits());
    assert!(in_band(scaled));
}

#[test]
fn rebuilding_a_rotation_from_its_own_components_changes_no_bit() {
    let mut sweep = Sweep(0x5EED_0FA5_BAD0_CAFE);
    let mut population = Vec::new();

    let mut previous = sweep.rotation();
    let mut chain = previous;
    for _ in 0..2_000 {
        let next = sweep.rotation();
        population.push(next);
        population.push(previous.product(next).unwrap());
        population.push(next.conjugate());
        population.push(previous.slerp(next, sweep.fraction()).unwrap());
        // A long chain of products, each fed the previous result.
        chain = chain.product(next).unwrap();
        population.push(chain);
        previous = next;
    }
    assert_eq!(population.len(), 10_000);

    for q in population {
        assert!(in_band(q), "{q:?} left the band");
        let rebuilt = Rotation::from_wxyz(q.w(), q.x(), q.y(), q.z()).unwrap();
        assert_eq!(bits(rebuilt), bits(q), "{q:?} changed when rebuilt");
    }
}

#[test]
fn rebuilding_slerps_between_near_parallel_rotations_changes_no_bit() {
    // The normalized-lerp branch, which the wide sweep above does not
    // reach: each pair is closer than the switchover.
    let mut sweep = Sweep(0x0DD_BA11);

    for _ in 0..2_000 {
        let from = sweep.rotation();
        let nudge = unit(1.0, 1e-9 * sweep.signed(), 1e-9 * sweep.signed(), 0.0);
        let to = from.product(nudge).unwrap();
        let q = from.slerp(to, sweep.fraction()).unwrap();

        assert!(in_band(q), "{q:?} left the band");
        let rebuilt = Rotation::from_wxyz(q.w(), q.x(), q.y(), q.z()).unwrap();
        assert_eq!(bits(rebuilt), bits(q), "{q:?} changed when rebuilt");
    }
}

#[test]
fn the_input_tolerance_is_one_part_in_a_million_of_the_norm() {
    let with_norm = |norm: f64| Rotation::from_wxyz(norm, 0.0, 0.0, 0.0);

    for accepted in [1.0 + 9e-7, 1.0 - 9e-7, 1.0 + 1e-8, 1.0 - 1e-8] {
        let q = with_norm(accepted).unwrap();
        assert!(in_band(q), "{accepted:e} must be brought into the band");
    }
    for rejected in [1.0 + 1.1e-6, 1.0 - 1.1e-6, 1.01, 0.99, 2.0, 0.5] {
        assert_eq!(
            with_norm(rejected),
            Err(TransformError::NonUnitRotation { norm: rejected })
        );
    }
}

#[test]
fn the_input_tolerance_boundary_is_exact_on_both_sides() {
    // The norm of `(w, 0, 0, 0)` is `w` itself, so these inputs set the
    // measured norm to the bit.
    let with_norm = |norm: f64| Rotation::from_wxyz(norm, 0.0, 0.0, 0.0);
    let rejects = |norm: f64| {
        let Err(TransformError::NonUnitRotation { norm: reported }) = with_norm(norm) else {
            return false;
        };
        reported.to_bits() == norm.to_bits()
    };

    // A norm further than 1e-6 from 1 is rejected, so `1 + 1e-6` is still
    // accepted and the next representable norm above it is not.
    let accepted: f64 = 1.0 + 1e-6;
    let rejected = f64::from_bits(accepted.to_bits() + 1);
    assert!(with_norm(accepted).is_ok());
    assert!(rejects(rejected));

    // Below 1 the spacing is half as wide, so `1 - 1e-6` rounds to slightly
    // more than the tolerance away from 1 and is rejected, while the next
    // norm back toward 1 is accepted. The boundary is symmetric in intent
    // and not in bits.
    let rejected: f64 = 1.0 - 1e-6;
    let accepted = f64::from_bits(rejected.to_bits() + 1);
    assert!(rejects(rejected));
    assert!(with_norm(accepted).is_ok());

    // The same boundary with the length on another component, negative.
    assert!(Rotation::from_wxyz(0.0, 0.0, -(1.0 + 1e-6), 0.0).is_ok());
    assert!(Rotation::from_wxyz(0.0, 0.0, -(1.0 - 1e-6), 0.0).is_err());
}

#[test]
fn accepted_out_of_band_input_is_scaled_to_unit_length() {
    // 9e-7 short of unit length: accepted, and no longer short afterwards.
    let drifted = 1.0 - 9e-7;
    let q = Rotation::from_wxyz(drifted, 0.0, 0.0, 0.0).unwrap();
    assert_ne!(q.w().to_bits(), drifted.to_bits());
    assert!(in_band(q));
    assert_abs_diff_eq!(q.w(), 1.0, epsilon = f64::EPSILON);

    // The direction is kept: every component is scaled by the same factor.
    let q = Rotation::from_wxyz(0.5 + 2e-7, 0.5 + 2e-7, 0.5 + 2e-7, 0.5 + 2e-7).unwrap();
    assert!(in_band(q));
    assert_eq!(
        bits(q),
        bits(Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap())
    );
}

#[test]
fn f32_widened_input_is_normalized_into_the_band() {
    // A unit quaternion that travelled as `f32`: its squared norm is 3e-8
    // short of 1, four hundred million times the band.
    let widened = f64::from(core::f32::consts::FRAC_1_SQRT_2);
    assert!((widened * widened + widened * widened - 1.0).abs() > 1e-8);

    let q = Rotation::from_wxyz(widened, widened, 0.0, 0.0).unwrap();

    assert!(in_band(q));
    assert_ne!(q.w().to_bits(), widened.to_bits());
    assert_eq!(
        bits(q),
        [FRAC_1_SQRT_2.to_bits(), FRAC_1_SQRT_2.to_bits(), 0, 0]
    );

    // A sweep of unit quaternions narrowed to `f32` and widened back.
    let mut sweep = Sweep(0xF32);
    for _ in 0..1_000 {
        let exact = sweep.rotation();
        let narrow = |component: f64| {
            // Decimal text is the one lossless route from f64 to f32 that
            // needs no cast.
            let text = format!("{component:e}");
            f64::from(text.parse::<f32>().unwrap())
        };
        let q = Rotation::from_wxyz(
            narrow(exact.w()),
            narrow(exact.x()),
            narrow(exact.y()),
            narrow(exact.z()),
        )
        .unwrap();

        assert!(in_band(q));
        assert_components_close(q, exact, 1e-7);
    }
}

#[test]
fn non_finite_components_are_rejected_in_every_position() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for position in 0..4 {
            let mut components = [1.0, 0.0, 0.0, 0.0];
            components[position] = bad;
            let [w, x, y, z] = components;

            assert_eq!(
                Rotation::from_wxyz(w, x, y, z),
                Err(TransformError::NonFinite),
                "{bad} in position {position}"
            );
        }
    }
    assert_eq!(
        Rotation::from_wxyz(f64::NAN, f64::NAN, f64::NAN, f64::NAN),
        Err(TransformError::NonFinite)
    );
}

#[test]
fn finite_components_with_an_overflowing_squared_norm_are_not_unit() {
    for (w, x, y, z) in [
        (1e200, 0.0, 0.0, 0.0),
        (0.0, -1e200, 0.0, 0.0),
        (f64::MAX, f64::MAX, f64::MAX, f64::MAX),
        (1.0, 0.0, 0.0, f64::MIN),
    ] {
        assert_eq!(
            Rotation::from_wxyz(w, x, y, z),
            Err(TransformError::NonUnitRotation {
                norm: f64::INFINITY
            })
        );
    }
}

#[test]
fn the_zero_quaternion_is_rejected_with_its_norm() {
    assert_eq!(
        Rotation::from_wxyz(0.0, 0.0, 0.0, 0.0),
        Err(TransformError::NonUnitRotation { norm: 0.0 })
    );
    // Components too small to square to anything: the norm underflows to 0.
    assert_eq!(
        Rotation::from_wxyz(1e-200, 1e-200, 0.0, 0.0),
        Err(TransformError::NonUnitRotation { norm: 0.0 })
    );
}

#[test]
fn a_denormalized_rotation_is_rejected_with_the_measured_norm() {
    // Rotation-equivalent but 1% long: it would scale everything it is
    // applied to.
    assert_eq!(
        Rotation::from_wxyz(1.01, 0.0, 0.0, 0.0),
        Err(TransformError::NonUnitRotation { norm: 1.01 })
    );
    // Never normalized at all: the norm of (1, 2, 3, 4) is sqrt(30).
    assert_eq!(
        Rotation::from_wxyz(1.0, 2.0, 3.0, 4.0),
        Err(TransformError::NonUnitRotation {
            norm: libm::sqrt(30.0)
        })
    );
}

#[test]
fn scalar_last_components_are_accepted_as_a_different_rotation() {
    // The residual the positional order leaves open: the identity written
    // scalar last, (x, y, z, w) = (0, 0, 0, 1), is a unit quaternion too.
    // Read scalar first it is a half turn about z.
    let misread = Rotation::from_wxyz(0.0, 0.0, 0.0, 1.0).unwrap();

    assert_ne!(misread, Rotation::IDENTITY);
    assert_eq!(
        misread.rotate(Vector3::new(1.0, 2.0, 3.0)),
        Vector3::new(-1.0, -2.0, 3.0)
    );
}

#[test]
fn equality_is_exact_and_separates_q_from_minus_q() {
    let q = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    let minus_q = Rotation::from_wxyz(-0.5, -0.5, -0.5, -0.5).unwrap();

    assert_eq!(q, Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap());
    assert_ne!(q, minus_q);
    // The same rotation all the same.
    let v = Vector3::new(1.0, 2.0, 3.0);
    assert_eq!(q.rotate(v), minus_q.rotate(v));

    let one_ulp_off = Rotation::from_wxyz(1.0 - f64::EPSILON / 2.0, 0.0, 0.0, 0.0).unwrap();
    assert_ne!(one_ulp_off, Rotation::IDENTITY);
}

#[test]
fn debug_prints_the_components_under_the_public_name() {
    assert_eq!(
        format!("{:?}", Rotation::from_wxyz(0.5, -0.5, 0.5, -0.5).unwrap()),
        "Rotation { w: 0.5, x: -0.5, y: 0.5, z: -0.5 }"
    );
}

// --- Rotating vectors ---------------------------------------------------

#[test]
fn identity_rotates_nothing() {
    let v = Vector3::new(1.0, 2.0, 3.0);

    assert_eq!(Rotation::IDENTITY.rotate(v), v);
}

#[test]
fn rotate_maps_known_vectors() {
    // A quarter turn about z: x to y, y to -x, z fixed.
    let quarter_z = Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap();
    assert_vector_close(
        quarter_z.rotate(Vector3::new(1.0, 0.0, 0.0)),
        Vector3::new(0.0, 1.0, 0.0),
        f64::EPSILON,
    );
    assert_vector_close(
        quarter_z.rotate(Vector3::new(0.0, 1.0, 0.0)),
        Vector3::new(-1.0, 0.0, 0.0),
        f64::EPSILON,
    );
    assert_vector_close(
        quarter_z.rotate(Vector3::new(0.0, 0.0, 1.0)),
        Vector3::new(0.0, 0.0, 1.0),
        f64::EPSILON,
    );

    // A quarter turn about x: y to z (right-hand rule).
    let quarter_x = Rotation::from_wxyz(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0).unwrap();
    assert_vector_close(
        quarter_x.rotate(Vector3::new(0.0, 1.0, 0.0)),
        Vector3::new(0.0, 0.0, 1.0),
        f64::EPSILON,
    );

    // Half turns are exact.
    let half_x = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();
    assert_eq!(
        half_x.rotate(Vector3::new(1.0, 2.0, 3.0)),
        Vector3::new(1.0, -2.0, -3.0)
    );
    let half_y = Rotation::from_wxyz(0.0, 0.0, 1.0, 0.0).unwrap();
    assert_eq!(
        half_y.rotate(Vector3::new(1.0, 2.0, 3.0)),
        Vector3::new(-1.0, 2.0, -3.0)
    );

    // A third of a turn about (1, 1, 1) cycles the axes: x to y to z.
    let third = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    assert_eq!(
        third.rotate(Vector3::new(1.0, 2.0, 3.0)),
        Vector3::new(3.0, 1.0, 2.0)
    );
}

#[test]
fn rotate_is_raw_arithmetic_that_can_overflow() {
    // No error channel: components near the limit of f64 leave the range.
    let quarter_z = Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap();
    let rotated = quarter_z.rotate(Vector3::new(f64::MAX, f64::MAX, 0.0));

    assert!(!(rotated.x.is_finite() && rotated.y.is_finite()));
}

// --- The raw quaternion arithmetic --------------------------------------

#[test]
fn raw_product_is_the_hamilton_product() {
    let q1 = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };
    let q2 = Quaternion {
        w: 5.0,
        x: 6.0,
        y: 7.0,
        z: 8.0,
    };

    let product = q1.product(q2);

    assert_eq!(
        [product.w, product.x, product.y, product.z].map(f64::to_bits),
        [-60.0_f64, 12.0, 30.0, 24.0].map(f64::to_bits)
    );
}

#[test]
fn raw_conjugate_negates_the_vector_part() {
    let q = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };

    let conjugate = q.conjugate();

    assert_eq!(
        [conjugate.w, conjugate.x, conjugate.y, conjugate.z].map(f64::to_bits),
        [1.0_f64, -2.0, -3.0, -4.0].map(f64::to_bits)
    );
}

#[test]
fn raw_scale_multiplies_every_component() {
    let q = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };

    let scaled = q.scale(2.0);

    assert_eq!(
        [scaled.w, scaled.x, scaled.y, scaled.z].map(f64::to_bits),
        [2.0_f64, 4.0, 6.0, 8.0].map(f64::to_bits)
    );
}

#[test]
fn raw_norm_squared_sums_the_squares() {
    let q = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };

    assert_eq!(q.norm_squared().to_bits(), 30.0_f64.to_bits());
}

#[test]
fn weighted_sum_pairs_each_weight_with_its_operand() {
    let q1 = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };
    let q2 = Quaternion {
        w: 5.0,
        x: 6.0,
        y: 7.0,
        z: 8.0,
    };

    // Asymmetric weights: a swapped weight/operand pairing yields
    // (4.0, 5.0, 6.0, 7.0) and fails, where 0.5/0.5 could not tell.
    let sum = Quaternion::weighted_sum(q1, 0.75, q2, 0.25);

    assert_eq!(
        [sum.w, sum.x, sum.y, sum.z].map(f64::to_bits),
        [2.0_f64, 3.0, 4.0, 5.0].map(f64::to_bits)
    );
}

#[test]
fn into_rotation_reports_what_it_cannot_bring_to_unit_length() {
    // Reachable only from raw components: every public path checks its
    // input before this rule runs.
    let zero = Quaternion {
        w: 0.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    assert_eq!(zero.into_rotation(), Err(NumericError::NonFinite));

    let nan = Quaternion {
        w: f64::NAN,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    assert_eq!(nan.into_rotation(), Err(NumericError::NonFinite));

    // Any finite, non-zero length is brought into the band.
    let long = Quaternion {
        w: 1.0,
        x: 2.0,
        y: 3.0,
        z: 4.0,
    };
    let rotation = long.into_rotation().unwrap();
    assert!(in_band(rotation));
    assert_components_close(rotation, unit(1.0, 2.0, 3.0, 4.0), f64::EPSILON);
}

// --- Rotation::norm_squared, conjugate, product -------------------------

#[test]
fn norm_squared_sums_the_squares_in_component_order() {
    assert_eq!(
        Rotation::IDENTITY.norm_squared().to_bits(),
        1.0_f64.to_bits()
    );
    assert_eq!(
        Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5)
            .unwrap()
            .norm_squared()
            .to_bits(),
        1.0_f64.to_bits()
    );

    let quarter = Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap();
    assert_eq!(
        quarter.norm_squared().to_bits(),
        (FRAC_1_SQRT_2 * FRAC_1_SQRT_2 + FRAC_1_SQRT_2 * FRAC_1_SQRT_2).to_bits()
    );

    let mut sweep = Sweep(0x2_0015);
    for _ in 0..1_000 {
        let rotation = sweep.rotation();
        let [w, x, y, z] = bits(rotation).map(f64::from_bits);
        assert_eq!(
            rotation.norm_squared().to_bits(),
            (w * w + x * x + y * y + z * z).to_bits()
        );
    }
}

#[test]
fn conjugate_negates_the_vector_part_and_keeps_the_squared_norm() {
    let q = Rotation::from_wxyz(0.5, -0.5, 0.5, 0.5).unwrap();
    assert_eq!(
        q.conjugate(),
        Rotation::from_wxyz(0.5, 0.5, -0.5, -0.5).unwrap()
    );
    assert_eq!(Rotation::IDENTITY.conjugate(), Rotation::IDENTITY);

    let mut sweep = Sweep(0xC0_1175);
    for _ in 0..2_000 {
        let q = sweep.rotation();
        let conjugate = q.conjugate();

        assert_eq!(
            bits(conjugate),
            [
                q.w().to_bits(),
                (-q.x()).to_bits(),
                (-q.y()).to_bits(),
                (-q.z()).to_bits()
            ]
        );
        assert_eq!(
            conjugate.norm_squared().to_bits(),
            q.norm_squared().to_bits()
        );
        assert_eq!(bits(conjugate.conjugate()), bits(q));
    }
}

#[test]
fn conjugate_rotates_back() {
    let mut sweep = Sweep(0xBAC_C0DE);
    let v = Vector3::new(0.3, -0.7, 1.1);

    for _ in 0..500 {
        let q = sweep.rotation();
        assert_vector_close(q.conjugate().rotate(q.rotate(v)), v, 8.0 * f64::EPSILON);
    }
}

#[test]
fn product_is_the_hamilton_product_of_the_components() {
    // Exact in binary: every term is a product of halves, zeros and ones.
    let q1 = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    let q2 = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();

    assert_eq!(
        q1.product(q2).unwrap(),
        Rotation::from_wxyz(-0.5, 0.5, 0.5, -0.5).unwrap()
    );
    assert_eq!(
        q2.product(q1).unwrap(),
        Rotation::from_wxyz(-0.5, 0.5, -0.5, 0.5).unwrap()
    );
}

#[test]
fn product_is_not_commutative() {
    let q1 = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    let q2 = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();

    assert_ne!(q1.product(q2).unwrap(), q2.product(q1).unwrap());
}

#[test]
fn product_applies_the_right_hand_rotation_first() {
    let quarter_z = Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap();
    let quarter_x = Rotation::from_wxyz(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0).unwrap();
    let v = Vector3::new(1.0, 0.0, 0.0);

    // z first takes x to y, then x takes y to z.
    let z_then_x = quarter_x.product(quarter_z).unwrap();
    assert_vector_close(
        z_then_x.rotate(v),
        Vector3::new(0.0, 0.0, 1.0),
        4.0 * f64::EPSILON,
    );

    // x first leaves x alone, then z takes it to y.
    let x_then_z = quarter_z.product(quarter_x).unwrap();
    assert_vector_close(
        x_then_z.rotate(v),
        Vector3::new(0.0, 1.0, 0.0),
        4.0 * f64::EPSILON,
    );

    // The product rotates as its factors do in turn, over a sweep.
    let mut sweep = Sweep(0x0_4DE4);
    let v = Vector3::new(0.3, -0.7, 1.1);
    for _ in 0..500 {
        let (a, b) = (sweep.rotation(), sweep.rotation());
        assert_vector_close(
            a.product(b).unwrap().rotate(v),
            a.rotate(b.rotate(v)),
            16.0 * f64::EPSILON,
        );
    }
}

#[test]
fn product_with_the_identity_changes_no_bit() {
    let mut sweep = Sweep(0x1D);

    for _ in 0..1_000 {
        let q = sweep.rotation();
        assert_eq!(q.product(Rotation::IDENTITY).unwrap(), q);
        assert_eq!(Rotation::IDENTITY.product(q).unwrap(), q);
    }
}

#[test]
fn product_stays_inside_the_band() {
    let mut sweep = Sweep(0xBA_2D);
    let mut chain = Rotation::IDENTITY;

    for _ in 0..5_000 {
        let (a, b) = (sweep.rotation(), sweep.rotation());
        assert!(in_band(a.product(b).unwrap()));
        // The product with the conjugate is the identity to rounding.
        let round_trip = a.product(a.conjugate()).unwrap();
        assert!(in_band(round_trip));
        assert_components_close(round_trip, Rotation::IDENTITY, 4.0 * f64::EPSILON);

        chain = chain.product(a).unwrap();
        assert!(in_band(chain));
    }
}

// --- Slerp --------------------------------------------------------------

#[test]
fn slerp_returns_the_endpoints_at_zero_and_one() {
    let q1 = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    let q2 = Rotation::IDENTITY;

    assert_components_close(q1.slerp(q2, 0.0).unwrap(), q1, f64::EPSILON);
    assert_components_close(q1.slerp(q2, 1.0).unwrap(), q2, f64::EPSILON);

    let mut sweep = Sweep(0xE0D);
    for _ in 0..500 {
        let (from, to) = (sweep.rotation(), sweep.rotation());
        // The far endpoint is reached up to the sign the shorter arc picks.
        let to = if dot(from, to) < 0.0 { negated(to) } else { to };

        assert_components_close(from.slerp(to, 0.0).unwrap(), from, 4.0 * f64::EPSILON);
        assert_components_close(from.slerp(to, 1.0).unwrap(), to, 4.0 * f64::EPSILON);
    }
}

#[test]
fn slerp_midpoint_halves_the_angle() {
    // From no rotation to a half turn about x: a quarter turn about x.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(0.0, 1.0, 0.0, 0.0).unwrap();

    let result = q1.slerp(q2, 0.5).unwrap();

    let expected = Rotation::from_wxyz(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0).unwrap();
    assert_components_close(result, expected, f64::EPSILON);
}

#[test]
fn slerp_result_is_in_the_band_for_nearby_rotations() {
    let q1 = unit(0.9999, 0.0001, 0.0, 0.0);
    let q2 = unit(0.9998, 0.0002, 0.0, 0.0);

    let result = q1.slerp(q2, 0.5).unwrap();

    assert!(in_band(result));
}

#[test]
fn slerp_uses_shortest_path_for_antipodal_quaternions() {
    // q and -q are one rotation: there is no arc between them.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(-1.0, 0.0, 0.0, 0.0).unwrap();

    let result = q1.slerp(q2, 0.5).unwrap();

    assert_eq!(result, Rotation::IDENTITY);
}

#[test]
fn slerp_takes_the_shorter_arc() {
    // 0.1 and 6.0 radians about z are 0.383 radians apart the short way
    // round, through 0, and 5.9 the long way. Their dot is negative, so
    // the midpoint must lie at -0.0416 radians, not at 3.05.
    let q1 = rotation_about_z(0.1);
    let q2 = rotation_about_z(6.0);
    assert!(dot(q1, q2) < 0.0);

    let midpoint = q1.slerp(q2, 0.5).unwrap();

    let short_way = rotation_about_z(0.1 - (0.1 + core::f64::consts::TAU - 6.0) / 2.0);
    assert!(
        same_rotation_up_to_sign(midpoint, short_way),
        "{midpoint:?} is not on the shorter arc"
    );
    assert!(!same_rotation_up_to_sign(midpoint, rotation_about_z(3.05)));
}

#[test]
fn slerp_clamps_t_to_the_unit_interval() {
    let q1 = Rotation::IDENTITY;
    let q2 = rotation_about_z(core::f64::consts::FRAC_PI_2);

    // No extrapolation: out-of-range factors saturate at the endpoints.
    assert_eq!(q1.slerp(q2, 2.0), q1.slerp(q2, 1.0));
    assert_eq!(q1.slerp(q2, f64::INFINITY), q1.slerp(q2, 1.0));
    assert_eq!(q1.slerp(q2, -0.5), q1.slerp(q2, 0.0));
    assert_eq!(q1.slerp(q2, f64::NEG_INFINITY), q1.slerp(q2, 0.0));
    assert_ne!(q1.slerp(q2, 1.0), q1.slerp(q2, 0.0));
}

#[test]
fn slerp_reports_a_nan_factor() {
    let q1 = Rotation::IDENTITY;
    let q2 = rotation_about_z(1.0);

    assert_eq!(q1.slerp(q2, f64::NAN), Err(NumericError::NonFinite));
    // On the near-parallel branch too.
    assert_eq!(q1.slerp(q1, f64::NAN), Err(NumericError::NonFinite));
}

#[test]
fn slerp_interior_points_follow_the_shortest_geodesic() {
    // Negating the second operand forces the shortest-path flip, so the
    // interior points run through the sin-weighted branch on the effective
    // arc 0.1 -> 3.0 radians about z.
    let q1 = rotation_about_z(0.1);
    let q2 = negated(rotation_about_z(3.0));

    for t in [0.25, 0.5, 0.7] {
        let s = q1.slerp(q2, t).unwrap();
        assert!(in_band(s));
        let expected = rotation_about_z(0.1 + 2.9 * t);
        assert!(
            same_rotation_up_to_sign(s, expected),
            "slerp at t={t} left the geodesic: {s:?} vs {expected:?}"
        );
    }
}

#[test]
fn slerp_takes_the_trig_branch_right_up_to_the_switchover() {
    // The normalized-lerp shortcut fires only above `1 - f64::EPSILON`,
    // where two rotations are numerically indistinguishable. A 0.06 radian
    // arc has a dot of cos(0.03), about 0.9996: close, and twelve orders of
    // magnitude outside that. It must still run the sin-weighted branch,
    // the one that advances the rotation angle linearly in `t`. Widening
    // the switchover to a rotation-scale epsilon would swallow this case.
    let q1 = Rotation::IDENTITY;
    let q2 = rotation_about_z(0.06);

    let quarter = q1.slerp(q2, 0.25).unwrap();
    assert_components_close(quarter, rotation_about_z(0.015), 1e-12);

    // The shortcut would land 4e-7 off that geodesic here, five orders of
    // magnitude outside the tolerance above, and invisible to a norm
    // check, because the shortcut normalizes its result too.
    let blended = Quaternion::weighted_sum(q1.0, 0.75, q2.0, 0.25)
        .into_rotation()
        .unwrap();
    assert!(in_band(blended));
    assert!(
        (blended.z() - quarter.z()).abs() > 1e-9,
        "the two branches must be distinguishable here: {blended:?} vs {quarter:?}"
    );
}

#[test]
fn slerp_near_antipodal_pair_stays_unit_and_on_the_small_arc() {
    // dot = -cos(0.01), about -0.99995: the numerically riskiest region.
    // After the shortest-path flip the effective arc is 0.02 radians, so
    // the midpoint is the 0.01-radian rotation, up to sign.
    let q1 = Rotation::IDENTITY;
    let q2 = negated(rotation_about_z(0.02));

    let s = q1.slerp(q2, 0.5).unwrap();

    assert!(in_band(s));
    assert!(
        same_rotation_up_to_sign(s, rotation_about_z(0.01)),
        "near-antipodal midpoint off the small arc: {s:?}"
    );
}

#[test]
fn slerp_between_near_parallel_rotations_is_a_normalized_lerp() {
    // 2e-8 radians apart: the dot rounds to exactly 1.0, which is above
    // the switchover, so no arc sine is divided by. The blend of
    // (1, 0) and (1, 1e-8) at a quarter is (1, 2.5e-9), already unit to
    // rounding.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(1.0, 1e-8, 0.0, 0.0).unwrap();
    assert_eq!(dot(q1, q2).to_bits(), 1.0_f64.to_bits());

    let result = q1.slerp(q2, 0.25).unwrap();

    assert_eq!(
        bits(result),
        [1.0_f64.to_bits(), (1e-8_f64 * 0.25).to_bits(), 0, 0]
    );
    assert_eq!(q1.slerp(q2, 0.0).unwrap(), q1);
    assert_eq!(q1.slerp(q2, 1.0).unwrap(), q2);

    // A rotation interpolated toward itself is itself at every factor.
    let q = Rotation::from_wxyz(0.5, 0.5, 0.5, 0.5).unwrap();
    for t in [0.0, 0.25, 0.5, 1.0] {
        assert_eq!(q.slerp(q, t).unwrap(), q);
    }
}

// The four tests below pin `slerp` bit for bit instead of comparing it
// within a tolerance. `sqrt`, `sin`, and `acos` are `libm`'s, so a platform
// intrinsic put back into the math path moves the last bits (by two ulps in
// `w` in the interior case below, on x86-64 glibc), and that is precisely
// what a tolerant comparison cannot see. The operands are decimal literals
// rather than computed rotations so that no platform trig function reaches
// the inputs either. A `libm` upgrade that moves a last bit fails these
// too; re-derive the constants then, deliberately.

#[test]
fn slerp_interior_point_is_bit_pinned() {
    // Halfway along a 1.22-radian arc about x (operands: cos and sin of
    // 0.61), so the trig branch runs on a dot of 0.82. The exact answer is
    // (cos(0.305), sin(0.305)), whose correctly rounded doubles are
    // 0x3FEE_85EA_0B55_5D66 and 0x3FD3_3800_DDAF_16F5: one ulp below each
    // pinned value. The operand is inside the band and stored as written.
    let q1 = Rotation::IDENTITY;
    let q2 =
        Rotation::from_wxyz(0.819_648_017_845_479_5, 0.572_867_460_100_481_3, 0.0, 0.0).unwrap();
    assert_eq!(
        bits(q2),
        [
            0.819_648_017_845_479_5_f64.to_bits(),
            0.572_867_460_100_481_3_f64.to_bits(),
            0,
            0
        ]
    );

    assert_eq!(
        bits(q1.slerp(q2, 0.5).unwrap()),
        [0x3FEE_85EA_0B55_5D67, 0x3FD3_3800_DDAF_16F6, 0, 0]
    );
}

#[test]
fn slerp_near_antipodal_point_is_bit_pinned() {
    // The negated 0.02-radian rotation about z: dot is -cos(0.01), so the
    // shortest-path flip fires and the trig branch runs on the 0.01-radian
    // remainder, where acos gives up most of its input precision. Here the
    // pinned values are the correctly rounded cos(0.005) and sin(0.005)
    // exactly. The operand is inside the band and stored as written.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(
        -0.999_950_000_416_665_3,
        0.0,
        0.0,
        -0.009_999_833_334_166_664,
    )
    .unwrap();
    assert_eq!(
        bits(q2),
        [
            (-0.999_950_000_416_665_3_f64).to_bits(),
            0,
            0,
            (-0.009_999_833_334_166_664_f64).to_bits()
        ]
    );

    assert_eq!(
        bits(q1.slerp(q2, 0.5).unwrap()),
        [0x3FEF_FFE5_C920_A926, 0, 0, 0x3F74_7ADB_B006_A986]
    );
}

#[test]
fn slerp_near_identity_point_is_bit_pinned() {
    // The components (1, 0.0001, 0, 0) have a norm 5e-9 above 1: inside
    // the input tolerance and outside the band, so `from_wxyz` scales them
    // and this also pins `libm::sqrt` in the maintenance rule. The stored
    // rotation is 2e-4 radians from the identity and its dot with it is
    // 1 - 5e-9, below the switchover: the trig branch runs.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(1.0, 0.0001, 0.0, 0.0).unwrap();
    assert_eq!(
        bits(q2),
        [0x3FEF_FFFF_FD50_CE24, 0x3F1A_36E2_E8E9_4FFD, 0, 0]
    );

    assert_eq!(
        bits(q1.slerp(q2, 0.25).unwrap()),
        [0x3FEF_FFFF_FFD5_0CE1, 0x3EFA_36E2_E999_3BFB, 0, 0]
    );
}

#[test]
fn slerp_near_parallel_point_is_bit_pinned() {
    // The linear-blend branch: the dot of the two operands is exactly 1.0.
    // The blend's squared norm is inside the band, so the blend is kept as
    // computed.
    let q1 = Rotation::IDENTITY;
    let q2 = Rotation::from_wxyz(1.0, 1e-8, 0.0, 0.0).unwrap();
    assert_eq!(
        bits(q2),
        [0x3FF0_0000_0000_0000, 0x3E45_798E_E230_8C3A, 0, 0]
    );

    assert_eq!(
        bits(q1.slerp(q2, 0.25).unwrap()),
        [0x3FF0_0000_0000_0000, 0x3E25_798E_E230_8C3A, 0, 0]
    );
}

#[test]
fn slerp_between_equal_in_band_rotations_keeps_their_bits() {
    // The blend of a rotation with itself is that rotation, and it is
    // already inside the band: the maintenance rule keeps it bit for bit.
    // Normalizing the blend first would round `w` down to 1.0.
    let q = Rotation::from_wxyz(1.0 + f64::EPSILON, 0.0, 0.0, 0.0).unwrap();
    assert_eq!(q.w().to_bits(), 0x3FF0_0000_0000_0001);

    assert_eq!(bits(q.slerp(q, 0.5).unwrap()), bits(q));
}
