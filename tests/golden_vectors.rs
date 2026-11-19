//! Poses computed outside this crate, asserted against literal numbers.
//!
//! Every other test in the suite checks the crate against itself: expected
//! values are built with the same constructors, the same quaternion
//! multiplication and the same composition order as the code under test. A
//! convention flipped consistently (a transposed rotation, a swapped
//! quaternion multiplication order, `(target, source)` read the other way
//! round) is invisible to all of them, because both sides of every
//! assertion move together. These vectors are the outside opinion: SciPy
//! computed them, this file hard-codes the digits, and nothing here derives
//! an expectation from the crate.
//!
//! Do not regenerate these numbers with the crate's own arithmetic. If one
//! of them fails, either a convention changed, which is a breaking change
//! to what every stored pose means, or the reference below has to be
//! re-derived deliberately, from outside.
//!
//! The crate does not apply a transform to a point. Vectors 1 and 4
//! therefore test `Rotation::rotate` together with the application recipe
//! of the `Transform` documentation, written out in the test. Vectors 2
//! and 3 pin the translation convention through the crate's own code: a
//! registry lookup in each direction, `Transform::compose` and
//! `Transform::inverse`.
//!
//! Reference, SciPy 1.18.0 / NumPy 2.5.1 (`Rotation` is scalar-last, so the
//! quaternions below are read out with `scalar_first=True` to match
//! `Rotation::from_wxyz`):
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
//!
//! # 6: a quarter of the way along the same pair (SciPy 1.18.1 / NumPy
//! # 2.5.2, which reproduce the digits of 5 exactly)
//! Slerp([0.0, 1.0], Rotation.concatenate([R_a, R_b]))(0.25)
//! 0.25 * np.array([4.0, -2.0, 6.0])       # lerp from a zero translation
//! ```

use approx::assert_abs_diff_eq;
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

/// Tolerance for comparing a computed pose against the reference digits.
/// Wide enough for a different order of the same operations, far too tight
/// for a different convention: the smallest disagreement any of the flips
/// below would produce is 0.5 in a component.
const EPSILON: f64 = 1e-12;

const T: Timestamp = Timestamp::from_nanos(1_000_000_000);

fn stamped(
    target: &str,
    source: &str,
    translation: Vector3,
    rotation: Rotation,
) -> Transform {
    Transform::new(target, source, translation, rotation, Stamp::At(T)).unwrap()
}

/// Compares a vector against reference digits, component by component.
fn assert_vector_matches(
    actual: Vector3,
    expected: Vector3,
) {
    assert_abs_diff_eq!(actual.x, expected.x, epsilon = EPSILON);
    assert_abs_diff_eq!(actual.y, expected.y, epsilon = EPSILON);
    assert_abs_diff_eq!(actual.z, expected.z, epsilon = EPSILON);
}

/// Compares a rotation against reference digits in `w, x, y, z` order.
fn assert_rotation_matches(
    actual: Rotation,
    expected: [f64; 4],
) {
    let [w, x, y, z] = expected;
    assert_abs_diff_eq!(actual.w(), w, epsilon = EPSILON);
    assert_abs_diff_eq!(actual.x(), x, epsilon = EPSILON);
    assert_abs_diff_eq!(actual.y(), y, epsilon = EPSILON);
    assert_abs_diff_eq!(actual.z(), z, epsilon = EPSILON);
}

/// The application recipe of the `Transform` documentation: rotate, then
/// translate, component by component.
fn apply(
    tf: &Transform,
    p: Vector3,
) -> Vector3 {
    let r = tf.rotation().rotate(p);
    let t = tf.translation();
    Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z)
}

#[test]
fn a_quarter_turn_of_yaw_plus_an_offset_places_a_known_point() {
    // map -> base is a 90 degree yaw at (2, 1, 0), so base-frame (1, 0, 0),
    // one metre ahead of the robot, lands at (2, 2, 0) in the map. The
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
    let tf = stamped(
        "map",
        "base",
        Vector3::new(2.0, 1.0, 0.0),
        Rotation::from_wxyz(quarter_turn, 0.0, 0.0, quarter_turn).unwrap(),
    );

    let p = Vector3::new(1.0, 0.0, 0.0);
    let r = tf.rotation().rotate(p);
    let t = tf.translation();
    let p_target = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);

    assert_vector_matches(p_target, Vector3::new(2.0, 2.0, 0.0));
}

#[test]
fn a_quarter_turn_of_yaw_places_a_known_point_through_a_registry_lookup() {
    // The same vector with the transform taken from a lookup, as the 2.4
    // test obtained it: storage and retrieval must not turn the rotation.
    let quarter_turn = core::f64::consts::FRAC_1_SQRT_2;
    let mut registry = Registry::unbounded();
    registry
        .insert(&stamped(
            "map",
            "base",
            Vector3::new(2.0, 1.0, 0.0),
            Rotation::from_wxyz(quarter_turn, 0.0, 0.0, quarter_turn).unwrap(),
        ))
        .unwrap();

    let tf = registry.lookup("map", "base", T).unwrap();

    assert_eq!(tf.target(), "map");
    assert_eq!(tf.source(), "base");
    assert_vector_matches(
        apply(&tf, Vector3::new(1.0, 0.0, 0.0)),
        Vector3::new(2.0, 2.0, 0.0),
    );
}

/// The two edges of vectors 2 and 3: map -> odom is a 30 degree yaw at
/// (1, 2, 0); odom -> base a 45 degree pitch at (0.5, -0.25, 3).
fn two_hop_edges() -> (Transform, Transform) {
    (
        stamped(
            "map",
            "odom",
            Vector3::new(1.0, 2.0, 0.0),
            Rotation::from_wxyz(0.965_925_826_289_068_3, 0.0, 0.0, 0.258_819_045_102_520_76)
                .unwrap(),
        ),
        stamped(
            "odom",
            "base",
            Vector3::new(0.5, -0.25, 3.0),
            Rotation::from_wxyz(0.923_879_532_511_286_7, 0.0, 0.382_683_432_365_089_8, 0.0)
                .unwrap(),
        ),
    )
}

/// Vector 2: the reference pose of base in map.
fn assert_is_reference_map_from_base(tf: &Transform) {
    assert_eq!(tf.target(), "map");
    assert_eq!(tf.source(), "base");
    assert_vector_matches(
        tf.translation(),
        Vector3::new(1.558_012_701_892_219_2, 2.033_493_649_053_89, 3.0),
    );
    assert_rotation_matches(
        tf.rotation(),
        [
            0.892_399_100_832_522_8,
            -0.099_045_760_541_287_62,
            0.369_643_810_614_386_1,
            0.239_117_618_394_334_49,
        ],
    );
}

/// Vector 3: the reference pose of map in base.
fn assert_is_reference_base_from_map(tf: &Transform) {
    assert_eq!(tf.target(), "base");
    assert_eq!(tf.source(), "map");
    assert_vector_matches(
        tf.translation(),
        Vector3::new(
            0.448_287_736_084_027_17,
            -0.982_050_807_568_877_4,
            -3.794_352_951_035_258_5,
        ),
    );
    assert_rotation_matches(
        tf.rotation(),
        [
            0.892_399_100_832_522_8,
            0.099_045_760_541_287_62,
            -0.369_643_810_614_386_1,
            -0.239_117_618_394_334_49,
        ],
    );
}

#[test]
fn a_two_hop_chain_matches_the_reference_composition() {
    // The two rotations do not commute, so the composed quaternion pins the
    // multiplication order as well as the direction of the chain walk.
    let (t_map_odom, t_odom_base) = two_hop_edges();
    let mut registry = Registry::unbounded();
    registry.insert(&t_map_odom).unwrap();
    registry.insert(&t_odom_base).unwrap();

    let t_map_base = registry.lookup("map", "base", T).unwrap();
    assert_is_reference_map_from_base(&t_map_base);

    // Asking the other way round returns the reference inverse, not the
    // same pose: the argument order decides which frame the result maps
    // into, and swapping it silently yields this instead.
    let t_base_map = registry.lookup("base", "map", T).unwrap();
    assert_is_reference_base_from_map(&t_base_map);
}

#[test]
fn compose_and_inverse_match_the_reference_composition() {
    // The same two vectors through `Transform::compose` and
    // `Transform::inverse`, with no registry involved.
    let (t_map_odom, t_odom_base) = two_hop_edges();

    let t_map_base = t_map_odom.compose(&t_odom_base).unwrap();
    assert_is_reference_map_from_base(&t_map_base);

    let t_base_map = t_map_base.inverse().unwrap();
    assert_is_reference_base_from_map(&t_base_map);

    // The inverse composed hop by hop, in the only order that chains.
    let hop_by_hop = t_odom_base
        .inverse()
        .unwrap()
        .compose(&t_map_odom.inverse().unwrap())
        .unwrap();
    assert_is_reference_base_from_map(&hop_by_hop);
}

/// Vector 4's transform: intrinsic roll/pitch/yaw of 30/45/60 degrees at
/// (-1, 0.5, 0.25).
fn sensor_in_map() -> Transform {
    stamped(
        "map",
        "sensor",
        Vector3::new(-1.0, 0.5, 0.25),
        Rotation::from_wxyz(
            0.723_317_411_364_711_8,
            0.391_903_837_329_119_9,
            0.200_562_121_146_575_12,
            0.531_975_695_182_166_8,
        )
        .unwrap(),
    )
}

#[test]
fn a_rotation_about_no_coordinate_axis_places_a_known_point() {
    // All four quaternion components are non-zero and distinct, so a
    // rotation applied with the components read in another order (the
    // scalar-last convention, or a conjugated rotation) moves the point
    // visibly.
    let tf = sensor_in_map();

    let p = Vector3::new(0.3, -0.7, 1.1);
    let r = tf.rotation().rotate(p);
    let t = tf.translation();
    let p_target = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);

    assert_vector_matches(
        p_target,
        Vector3::new(
            0.312_544_181_470_241,
            0.300_345_740_105_364_36,
            0.415_426_564_355_733_33,
        ),
    );
}

#[test]
fn a_rotation_about_no_coordinate_axis_places_a_known_point_through_a_registry_lookup() {
    let mut registry = Registry::unbounded();
    registry.insert(&sensor_in_map()).unwrap();

    let tf = registry.lookup("map", "sensor", T).unwrap();

    assert_vector_matches(
        apply(&tf, Vector3::new(0.3, -0.7, 1.1)),
        Vector3::new(
            0.312_544_181_470_241,
            0.300_345_740_105_364_36,
            0.415_426_564_355_733_33,
        ),
    );
}

/// Two samples of map -> base one second apart, at `T` and `T` + 1 s: `R_a`
/// at the origin, then `R_b` at (4, -2, 6).
fn two_samples_one_second_apart() -> Registry {
    let later = Timestamp::from_nanos(2_000_000_000);
    let mut registry = Registry::unbounded();
    registry
        .insert(&stamped(
            "map",
            "base",
            Vector3::new(0.0, 0.0, 0.0),
            Rotation::from_wxyz(0.996_194_698_091_745_5, 0.0, 0.0, 0.087_155_742_747_658_17)
                .unwrap(),
        ))
        .unwrap();
    registry
        .insert(
            &Transform::new(
                "map",
                "base",
                Vector3::new(4.0, -2.0, 6.0),
                Rotation::from_wxyz(
                    0.670_734_316_285_678_8,
                    0.341_506_350_946_109_6,
                    0.153_134_767_662_329_03,
                    0.640_342_589_676_229_4,
                )
                .unwrap(),
                Stamp::At(later),
            )
            .unwrap(),
        )
        .unwrap();
    registry
}

#[test]
fn an_interpolated_lookup_matches_the_reference_slerp() {
    // Two samples one second apart, queried halfway between them. The
    // rotation is SciPy's `Slerp` midpoint of the same pair, so this pins
    // the interpolation against an outside implementation rather than
    // against the crate's own slerp.
    let registry = two_samples_one_second_apart();

    let midpoint = registry
        .lookup("map", "base", Timestamp::from_nanos(1_500_000_000))
        .unwrap();

    assert_vector_matches(midpoint.translation(), Vector3::new(2.0, -1.0, 3.0));
    assert_rotation_matches(
        midpoint.rotation(),
        [
            0.897_706_867_382_619_8,
            0.183_914_608_153_606_66,
            0.082_469_098_191_837_52,
            0.391_786_478_844_298_46,
        ],
    );
}

#[test]
fn an_off_centre_interpolated_lookup_weights_the_nearer_sample() {
    // The same pair, queried a quarter of the way in. The midpoint above is
    // symmetric: weighting the samples the wrong way round, with `ratio`
    // and `1 - ratio` swapped, returns the same pose there. Here it returns
    // the pose at three quarters, off by 3.0 in the translation's z and by
    // 0.28 in the rotation's.
    let registry = two_samples_one_second_apart();

    let quarter = registry
        .lookup("map", "base", Timestamp::from_nanos(1_250_000_000))
        .unwrap();

    assert_vector_matches(quarter.translation(), Vector3::new(1.0, -0.5, 1.5));
    assert_rotation_matches(
        quarter.rotation(),
        [
            0.964_361_020_365_767_5,
            0.093_647_992_278_183_17,
            0.041_992_670_121_166_11,
            0.243_873_925_620_300_9,
        ],
    );
}
