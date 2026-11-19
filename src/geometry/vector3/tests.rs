//! Unit tests.

use super::Vector3;
use alloc::format;

#[test]
fn new_stores_the_components_in_argument_order() {
    let v = Vector3::new(1.0, -2.0, 3.5);

    assert_eq!(v.x.to_bits(), 1.0_f64.to_bits());
    assert_eq!(v.y.to_bits(), (-2.0_f64).to_bits());
    assert_eq!(v.z.to_bits(), 3.5_f64.to_bits());
}

#[test]
fn new_is_usable_in_a_const_context() {
    const V: Vector3 = Vector3::new(1.0, 2.0, 3.0);

    assert_eq!(V, Vector3::new(1.0, 2.0, 3.0));
}

#[test]
fn fields_are_public_and_carry_no_invariant() {
    let mut v = Vector3::new(0.0, 0.0, 0.0);
    v.x = f64::NAN;
    v.y = f64::INFINITY;
    v.z = f64::NEG_INFINITY;

    assert!(v.x.is_nan());
    assert_eq!(v.y.to_bits(), f64::INFINITY.to_bits());
    assert_eq!(v.z.to_bits(), f64::NEG_INFINITY.to_bits());
}

#[test]
fn equality_is_exact_ieee_comparison() {
    assert_eq!(Vector3::new(1.0, 2.0, 3.0), Vector3::new(1.0, 2.0, 3.0));

    // One ulp apart is not equal: there is no tolerance.
    let next = f64::from_bits(1.0_f64.to_bits() + 1);
    assert_ne!(Vector3::new(1.0, 2.0, 3.0), Vector3::new(next, 2.0, 3.0));
    assert_ne!(Vector3::new(1.0, 2.0, 3.0), Vector3::new(1.0, 2.0, -3.0));

    // The two zeros compare equal, and NaN equals nothing, itself included.
    assert_eq!(Vector3::new(0.0, 0.0, 0.0), Vector3::new(-0.0, -0.0, -0.0));
    let nan = Vector3::new(f64::NAN, 0.0, 0.0);
    assert_ne!(nan, nan);
}

#[test]
fn a_vector_is_copied_not_moved() {
    let v = Vector3::new(1.0, 2.0, 3.0);
    let copy = v;

    assert_eq!(v, copy);
}

#[test]
fn debug_names_the_type_and_its_components() {
    assert_eq!(
        format!("{:?}", Vector3::new(1.0, -2.0, 0.5)),
        "Vector3 { x: 1.0, y: -2.0, z: 0.5 }"
    );
}

#[test]
fn zero_is_the_origin() {
    assert_eq!(Vector3::ZERO.x.to_bits(), 0.0_f64.to_bits());
    assert_eq!(Vector3::ZERO.y.to_bits(), 0.0_f64.to_bits());
    assert_eq!(Vector3::ZERO.z.to_bits(), 0.0_f64.to_bits());
}

#[test]
fn is_finite_requires_every_component_to_be_finite() {
    assert!(Vector3::new(1.0, -2.0, 3.0).is_finite());
    assert!(Vector3::new(f64::MAX, f64::MIN, f64::MIN_POSITIVE).is_finite());

    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(!Vector3::new(bad, 0.0, 0.0).is_finite());
        assert!(!Vector3::new(0.0, bad, 0.0).is_finite());
        assert!(!Vector3::new(0.0, 0.0, bad).is_finite());
    }
}
