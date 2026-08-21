//! Poses computed outside this crate, asserted against literal numbers.
//!
//! Every other test in the suite checks the crate against itself: expected
//! values are built with the same constructors, the same quaternion
//! multiplication and the same composition order as the code under test. A
//! convention flipped consistently — a transposed rotation, a swapped
//! quaternion multiplication order, `(parent, child)` read the other way
//! round — is invisible to all of them, because both sides of every
//! assertion move together. These vectors are the outside opinion: SciPy
//! computed them, this file hard-codes the digits, and nothing here derives
//! an expectation from the crate.
//!
//! Do not regenerate these numbers with the crate's own arithmetic. If one
//! of them fails, either a convention changed — which is a breaking change
//! to what every stored pose means — or the reference below has to be
//! re-derived deliberately, from outside.
//!
//! Reference, SciPy 1.18.0 / NumPy 2.5.1 (`Rotation` is scalar-last, so the
//! quaternions below are read out with `scalar_first=True` to match
//! `Quaternion::from_wxyz`):
//!
//! ```python
//! import numpy as np
//! from scipy.spatial.transform import Rotation, Slerp
//!
//! # 1: 90 deg yaw, offset (2, 1, 0), applied to (1, 0, 0)
//! R1 = Rotation.from_euler("z", 90, degrees=True)
//! R1.as_quat(scalar_first=True)            # rotation of map -> base
//! R1.apply([1.0, 0.0, 0.0]) + [2.0, 1.0, 0.0]
//!
//! # 2: two-hop chain map -> odom -> base
//! R_map_odom = Rotation.from_euler("z", 30, degrees=True)
//! R_odom_base = Rotation.from_euler("y", 45, degrees=True)
//! R_map_base = R_map_odom * R_odom_base
//! t_map_base = R_map_odom.apply([0.5, -0.25, 3.0]) + [1.0, 2.0, 0.0]
//!
//! # 3: the reverse lookup
//! R_base_map = R_map_base.inv()
//! t_base_map = -R_base_map.apply(t_map_base)
//!
//! # 4: intrinsic roll/pitch/yaw 30/45/60, offset (-1, 0.5, 0.25)
//! R4 = Rotation.from_euler("XYZ", [30, 45, 60], degrees=True)
//! R4.apply([0.3, -0.7, 1.1]) + [-1.0, 0.5, 0.25]
//!
//! # 5: the midpoint of a slerp between two further rotations
//! R_a = Rotation.from_euler("z", 10, degrees=True)
//! R_b = Rotation.from_euler("XYZ", [20, 40, 80], degrees=True)
//! Slerp([0.0, 1.0], Rotation.concatenate([R_a, R_b]))(0.5)
//! ```

use approx::assert_abs_diff_eq;
use transforms::{
    Registry, Transformable,
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

/// Tolerance for comparing a computed pose against the reference digits.
/// Wide enough for a different order of the same operations, far too tight
/// for a different convention: the smallest disagreement any of the flips
/// below would produce is 0.5 in a component.
const EPSILON: f64 = 1e-12;

const T: Timestamp = Timestamp::from_nanos(1_000_000_000);

fn stamped(
    parent: &str,
    child: &str,
    translation: Vector3,
    rotation: Quaternion,
) -> Transform {
    Transform::new(parent, child, translation, rotation, Stamp::At(T)).unwrap()
}

#[test]
fn a_quarter_turn_of_yaw_plus_an_offset_places_a_known_point() {
    // map -> base is a 90 degree yaw at (2, 1, 0), so base-frame (1, 0, 0)
    // — one metre ahead of the robot — lands at (2, 2, 0) in the map. The
    // number that matters is the y: a transposed rotation puts the point at
    // (2, 0, 0) instead, and every self-consistent test in the suite would
    // still pass.
    //
    // Both non-zero components are cos(45 degrees) = sin(45 degrees), the
    // one input here spelled as a constant rather than as SciPy's digits:
    // SciPy printed the `z` one ulp lower, 1e-16 away and fourteen orders
    // of magnitude below the tolerance, and quoting the digits would have
    // clippy ask for this constant anyway.
    let quarter_turn = core::f64::consts::FRAC_1_SQRT_2;
    let mut registry = Registry::new();
    registry
        .add_transform(stamped(
            "map",
            "base",
            Vector3::new(2.0, 1.0, 0.0),
            Quaternion::from_wxyz(quarter_turn, 0.0, 0.0, quarter_turn),
        ))
        .unwrap();

    let mut point = Point::new(
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        T,
        "base",
    );
    let transform = registry.get_transform_for(&point, "map").unwrap();
    point.transform(&transform).unwrap();

    assert_eq!(point.frame, "map");
    assert_abs_diff_eq!(
        point.position,
        Vector3::new(2.0, 2.0, 0.0),
        epsilon = EPSILON
    );
}

#[test]
fn a_two_hop_chain_matches_the_reference_composition() {
    // map -> odom is a 30 degree yaw at (1, 2, 0); odom -> base a 45 degree
    // pitch at (0.5, -0.25, 3). The two rotations do not commute, so the
    // composed quaternion pins the multiplication order as well as the
    // direction of the chain walk.
    let mut registry = Registry::new();
    registry
        .add_transform(stamped(
            "map",
            "odom",
            Vector3::new(1.0, 2.0, 0.0),
            Quaternion::from_wxyz(0.965_925_826_289_068_3, 0.0, 0.0, 0.258_819_045_102_520_76),
        ))
        .unwrap();
    registry
        .add_transform(stamped(
            "odom",
            "base",
            Vector3::new(0.5, -0.25, 3.0),
            Quaternion::from_wxyz(0.923_879_532_511_286_7, 0.0, 0.382_683_432_365_089_8, 0.0),
        ))
        .unwrap();

    let t_map_base = registry.get_transform("map", "base", T).unwrap();

    assert_abs_diff_eq!(
        t_map_base.translation(),
        Vector3::new(1.558_012_701_892_219_2, 2.033_493_649_053_89, 3.0),
        epsilon = EPSILON
    );
    assert_abs_diff_eq!(
        t_map_base.rotation(),
        Quaternion::from_wxyz(
            0.892_399_100_832_522_8,
            -0.099_045_760_541_287_62,
            0.369_643_810_614_386_1,
            0.239_117_618_394_334_49
        ),
        epsilon = EPSILON
    );

    // Asking the other way round returns the reference inverse, not the
    // same pose: the argument order decides which frame the result maps
    // into, and swapping it silently yields this instead.
    let t_base_map = registry.get_transform("base", "map", T).unwrap();

    assert_abs_diff_eq!(
        t_base_map.translation(),
        Vector3::new(
            0.448_287_736_084_027_17,
            -0.982_050_807_568_877_4,
            -3.794_352_951_035_258_5
        ),
        epsilon = EPSILON
    );
    assert_abs_diff_eq!(
        t_base_map.rotation(),
        Quaternion::from_wxyz(
            0.892_399_100_832_522_8,
            0.099_045_760_541_287_62,
            -0.369_643_810_614_386_1,
            -0.239_117_618_394_334_49
        ),
        epsilon = EPSILON
    );
}

#[test]
fn a_rotation_about_no_coordinate_axis_places_a_known_point() {
    // Intrinsic roll/pitch/yaw of 30/45/60 degrees: all four quaternion
    // components are non-zero and distinct, so a rotation applied with the
    // components read in another order — the scalar-last convention, or a
    // conjugated rotation — moves the point visibly.
    let mut registry = Registry::new();
    registry
        .add_transform(stamped(
            "map",
            "sensor",
            Vector3::new(-1.0, 0.5, 0.25),
            Quaternion::from_wxyz(
                0.723_317_411_364_711_8,
                0.391_903_837_329_119_9,
                0.200_562_121_146_575_12,
                0.531_975_695_182_166_8,
            ),
        ))
        .unwrap();

    let mut point = Point::new(
        Vector3::new(0.3, -0.7, 1.1),
        Quaternion::identity(),
        T,
        "sensor",
    );
    let transform = registry.get_transform_for(&point, "map").unwrap();
    point.transform(&transform).unwrap();

    assert_abs_diff_eq!(
        point.position,
        Vector3::new(
            0.312_544_181_470_241,
            0.300_345_740_105_364_36,
            0.415_426_564_355_733_33
        ),
        epsilon = EPSILON
    );
}

#[test]
fn an_interpolated_lookup_matches_the_reference_slerp() {
    // Two samples one second apart, queried halfway between them. The
    // rotation is SciPy's `Slerp` midpoint of the same pair, so this pins
    // the interpolation against an outside implementation rather than
    // against the crate's own slerp.
    let later = Timestamp::from_nanos(2_000_000_000);
    let mut registry = Registry::new();
    registry
        .add_transform(stamped(
            "map",
            "base",
            Vector3::zero(),
            Quaternion::from_wxyz(0.996_194_698_091_745_5, 0.0, 0.0, 0.087_155_742_747_658_17),
        ))
        .unwrap();
    registry
        .add_transform(
            Transform::new(
                "map",
                "base",
                Vector3::new(4.0, -2.0, 6.0),
                Quaternion::from_wxyz(
                    0.670_734_316_285_678_8,
                    0.341_506_350_946_109_6,
                    0.153_134_767_662_329_03,
                    0.640_342_589_676_229_4,
                ),
                Stamp::At(later),
            )
            .unwrap(),
        )
        .unwrap();

    let midpoint = registry
        .get_transform("map", "base", Timestamp::from_nanos(1_500_000_000))
        .unwrap();

    assert_abs_diff_eq!(
        midpoint.translation(),
        Vector3::new(2.0, -1.0, 3.0),
        epsilon = EPSILON
    );
    assert_abs_diff_eq!(
        midpoint.rotation(),
        Quaternion::from_wxyz(
            0.897_706_867_382_619_8,
            0.183_914_608_153_606_66,
            0.082_469_098_191_837_52,
            0.391_786_478_844_298_46
        ),
        epsilon = EPSILON
    );
}
