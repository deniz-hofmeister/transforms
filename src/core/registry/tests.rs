#[cfg(test)]
mod registry_tests {
    use crate::{
        Registry, Transformable,
        errors::RegistryError,
        geometry::{Point, Quaternion, Transform, UNIT_NORM_TOLERANCE, Vector3},
        time::{Stamp, Timestamp},
    };
    use approx::assert_abs_diff_eq;
    use core::time::Duration;

    #[test]
    fn basic_chain_linear() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at y=1m
        let t_b_c = Transform::new(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();

        let t_a_c = Transform::new(
            "a",
            "c",
            Vector3::new(1.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        let r = registry.get_transform("a", "c", t_a_b.timestamp().at().unwrap());

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_chain_linear_reverse() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at y=1m
        let t_b_c = Transform::new(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();

        let t_c_a = Transform::new(
            "c",
            "a",
            Vector3::new(-1.0, -1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        let r = registry.get_transform("c", "a", t_a_b.timestamp().at().unwrap());

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_c_a);
    }

    #[test]
    fn basic_chain_rotation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at +90 degrees
        let theta = core::f64::consts::PI / 2.0;
        let t_b_c = Transform::new(
            "b",
            "c",
            Vector3::new(0.0, 0.0, 0.0),
            Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame D at x=1m
        let t_c_d = Transform::new(
            "c",
            "d",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_b_c.clone()).unwrap();
        registry.add_transform(t_c_d.clone()).unwrap();

        let t_a_d = Transform::new(
            "a",
            "d",
            Vector3::new(1.0, 1.0, 0.0),
            Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            Stamp::At(t),
        )
        .unwrap();
        let r = registry.get_transform("a", "d", t_a_b.timestamp().at().unwrap());

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_d);
    }

    #[test]
    fn basic_exact_match() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_c = Transform::new(
            "a",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b.clone()).unwrap();
        registry.add_transform(t_a_c.clone()).unwrap();

        let r = registry.get_transform("a", "b", t_a_b.timestamp().at().unwrap());

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_b);

        let r = registry.get_transform("a", "c", t_a_c.timestamp().at().unwrap());

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_interpolation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at x=1m without rotation
        let t_a_b_0 = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame B at y=1m with 90 degrees rotation around +Z
        let theta = core::f64::consts::PI / 2.0;
        let t_a_b_1 = Transform::new(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
            Stamp::At((t + Duration::from_secs(1)).unwrap()),
        )
        .unwrap();

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();

        let middle_timestamp = Timestamp::from_nanos(u64::midpoint(
            t_a_b_0.timestamp().at().unwrap().as_nanos(),
            t_a_b_1.timestamp().at().unwrap().as_nanos(),
        ));

        let t_a_b_2 = Transform::new(
            "a",
            "b",
            (t_a_b_0.translation() + t_a_b_1.translation()) / 2.0,
            t_a_b_0.rotation().slerp(t_a_b_1.rotation(), 0.5),
            Stamp::At(middle_timestamp),
        )
        .unwrap();

        let r = registry.get_transform("a", "b", middle_timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_b_2);
    }

    #[test]
    fn basic_chained_interpolation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, x=1m without rotation
        let t_a_b_0 = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame B at t=1, x=2m without rotation
        let t_a_b_1 = Transform::new(
            "a",
            "b",
            Vector3::new(2.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At((t + Duration::from_secs(1)).unwrap()),
        )
        .unwrap();
        // Child frame C at t=0, y=1m without rotation
        let t_b_c_0 = Transform::new(
            "b",
            "c",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at t=1, y=2m without rotation
        let t_b_c_1 = Transform::new(
            "b",
            "c",
            Vector3::new(0.0, 2.0, 0.0),
            Quaternion::identity(),
            Stamp::At((t + Duration::from_secs(1)).unwrap()),
        )
        .unwrap();

        registry.add_transform(t_a_b_0.clone()).unwrap();
        registry.add_transform(t_a_b_1.clone()).unwrap();
        registry.add_transform(t_b_c_0.clone()).unwrap();
        registry.add_transform(t_b_c_1.clone()).unwrap();

        let middle_timestamp = Timestamp::from_nanos(u64::midpoint(
            t_a_b_0.timestamp().at().unwrap().as_nanos(),
            t_a_b_1.timestamp().at().unwrap().as_nanos(),
        ));

        let t_a_c = Transform::new(
            "a",
            "c",
            Vector3::new(1.5, 1.5, 0.0),
            Quaternion::identity(),
            Stamp::At(middle_timestamp),
        )
        .unwrap();

        let r = registry.get_transform("a", "c", middle_timestamp);

        assert!(r.is_ok(), "Registry returned Error, expected Ok");
        assert_abs_diff_eq!(r.unwrap(), t_a_c);
    }

    #[test]
    fn basic_branch_navigation() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at t=0, x=1m without rotation
        let t_b_c = Transform::new(
            "b",
            "c",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform::new(
            "b",
            "d",
            Vector3::new(2.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b).unwrap();
        registry.add_transform(t_b_c).unwrap();
        registry.add_transform(t_b_d).unwrap();

        let result = registry.get_transform("c", "d", t);

        assert!(result.is_ok());

        let t_c_d = result.unwrap();
        let t_c_d_expected = Transform::new(
            "c",
            "d",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        assert_abs_diff_eq!(t_c_d, t_c_d_expected);
    }

    #[test]
    fn basic_common_parent_elimination() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Child frame B at t=0, y=1m without rotation
        let t_a_b = Transform::new(
            "a",
            "b",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame C at t=0, x=1m without rotation
        let t_b_c = Transform::new(
            "b",
            "c",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        // Child frame D at t=0, x=2m without rotation
        let t_b_d = Transform::new(
            "b",
            "d",
            Vector3::new(2.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();

        registry.add_transform(t_a_b).unwrap();
        registry.add_transform(t_b_c).unwrap();
        registry.add_transform(t_b_d).unwrap();

        let mut walk_failure = None;
        let target_chain =
            Registry::get_transform_chain("d", "a", t, &registry.data, &mut walk_failure);
        let source_chain =
            Registry::get_transform_chain("c", "a", t, &registry.data, &mut walk_failure);

        assert!(target_chain.is_some());
        assert!(source_chain.is_some());

        let mut target = target_chain.unwrap();
        let mut source = source_chain.unwrap();

        // Both walks climb through "b" to "a"; the shared "a -> b" hop is
        // dropped, leaving one hop on each side.
        Registry::truncate_at_common_parent(&mut target, &mut source);
        assert_eq!(target.len(), 1);
        assert_eq!(source.len(), 1);

        let result = Registry::combine_transforms(target, source)
            .expect("chains are non-empty")
            .expect("combining the truncated chains must succeed");

        assert_eq!(result.parent(), "d");
        assert_eq!(result.child(), "c");
        assert_eq!(result.translation(), Vector3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn a_deep_common_trunk_is_truncated_to_the_divergent_hops() {
        // The same elimination as above, with a trunk deep enough for its
        // removal to matter: both leaves sit four hops under the root and
        // share three of them. Skipping the truncation still yields the
        // right answer — it just composes the shared trunk up and back down
        // again — so the chain lengths are the only place the work becomes
        // visible, and on a deep tree that work is the whole lookup cost.
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        let edges = [
            ("a", "b", Vector3::new(1.0, 0.0, 0.0)),
            ("b", "c", Vector3::new(0.0, 1.0, 0.0)),
            ("c", "d", Vector3::new(0.0, 0.0, 1.0)),
            ("d", "e", Vector3::new(1.0, 0.0, 0.0)),
            ("d", "f", Vector3::new(0.0, 2.0, 0.0)),
        ];
        for (parent, child, translation) in edges {
            registry
                .add_transform(
                    Transform::new(
                        parent,
                        child,
                        translation,
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let mut walk_failure = None;
        let mut target =
            Registry::get_transform_chain("e", "a", t, &registry.data, &mut walk_failure).unwrap();
        let mut source =
            Registry::get_transform_chain("f", "a", t, &registry.data, &mut walk_failure).unwrap();
        assert_eq!(target.len(), 4);
        assert_eq!(source.len(), 4);

        Registry::truncate_at_common_parent(&mut target, &mut source);
        assert_eq!(target.len(), 1);
        assert_eq!(source.len(), 1);

        let result = Registry::combine_transforms(target, source)
            .expect("chains are non-empty")
            .expect("combining the truncated chains must succeed");

        assert_eq!(result.parent(), "e");
        assert_eq!(result.child(), "f");
        // "e" sits at x=1 under "d", "f" at y=2: "f" expressed in "e".
        assert_eq!(result.translation(), Vector3::new(-1.0, 2.0, 0.0));
    }

    #[test]
    fn time_travel_different_frames() {
        // All three frames (fixed, target, source) are different, so both
        // process_get_transform lookups are non-trivial (no identity shortcut).
        //
        // Tree: fixed -> a -> b
        // At t1: a at x=1 in fixed, b at y=1 in a  → b in fixed = (1,1,0)
        // At t2: a at x=2 in fixed, b at y=2 in a  → a in fixed = (2,0,0)
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   = (-2,0,0) + (1,1,0) = (-1, 1, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // fixed -> a at t2: a has moved to x=2
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(2.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        // a -> b at t1: b is at y=1 relative to a
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // a -> b at t2: b is at y=2 relative to a
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.0, 2.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        assert!(
            (tf.translation().x - (-1.0)).abs() < f64::EPSILON,
            "Expected x=-1.0, got {}",
            tf.translation().x
        );
        assert!(
            (tf.translation().y - 1.0).abs() < f64::EPSILON,
            "Expected y=1.0, got {}",
            tf.translation().y
        );
        assert!(
            tf.translation().z.abs() < f64::EPSILON,
            "Expected z=0.0, got {}",
            tf.translation().z
        );
    }

    #[test]
    fn time_travel_same_time() {
        // When source_time == target_time, time travel should match get_transform.
        // Uses target != fixed so both lookups are non-trivial.
        //
        // Tree: fixed -> a -> b, all at time t
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        let regular = registry.get_transform("a", "b", t);
        let time_travel = registry.get_transform_at("a", t, "b", t, "fixed");

        assert!(regular.is_ok());
        assert!(time_travel.is_ok());

        let regular_tf = regular.unwrap();
        let time_travel_tf = time_travel.unwrap();

        assert_abs_diff_eq!(regular_tf.translation(), time_travel_tf.translation());
        assert_abs_diff_eq!(regular_tf.rotation(), time_travel_tf.rotation());
    }

    #[test]
    fn time_travel_with_rotation() {
        // All three frames different, with rotation on the target frame.
        //
        // Tree: fixed -> a -> b
        // At t1: a at (1,0,0) no rotation, b at (0.5,0,0) in a
        //   → b in fixed at t1 = (1.5, 0, 0)
        // At t2: a at origin rotated 90° CCW around z, b at (0.5,0,0) in a
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   a-in-fixed@t2 = {t: (0,0,0), R: 90°}  → inverse = {t: (0,0,0), R: -90°}
        //   R(-90°) * (1.5, 0, 0) = (0, -1.5, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        let theta = core::f64::consts::PI / 2.0;

        // fixed -> a at t1: at (1,0,0), no rotation
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // fixed -> a at t2: at origin, rotated 90° CCW around z
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(0.0, 0.0, 0.0),
                    Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin()),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        // a -> b at t1: b is at (0.5, 0, 0) relative to a
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.5, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // a -> b at t2: b still at (0.5, 0, 0) relative to a
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.5, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(
            result.is_ok(),
            "Time travel with rotation failed: {result:?}"
        );
        let tf = result.unwrap();

        // b was at (1.5, 0, 0) in fixed at t1.
        // a is at origin rotated 90° CCW at t2.
        // In a's frame at t2: R(-90°) * (1.5, 0, 0) = (0, -1.5, 0)
        assert!(
            tf.translation().x.abs() < 1e-10,
            "Expected x=0.0, got {}",
            tf.translation().x
        );
        assert!(
            (tf.translation().y - (-1.5)).abs() < 1e-10,
            "Expected y=-1.5, got {}",
            tf.translation().y
        );
        assert!(
            tf.translation().z.abs() < 1e-10,
            "Expected z=0.0, got {}",
            tf.translation().z
        );
    }

    #[test]
    fn time_travel_branching_tree() {
        // Tree is a <- fixed -> b (source and target on separate branches).
        //
        // At t1: fixed->a is (1,0,0), fixed->b is (0,1,0)
        //   → b in fixed at t1 = (0,1,0)
        // At t2: fixed->a is (2,0,0), fixed->b is (0,2,0)
        //   → a in fixed at t2 = (2,0,0)
        //
        // get_transform_at("a", t2, "b", t1, "fixed")
        //   = "b-at-t1 expressed in a-at-t2"
        //   = inverse(a-in-fixed@t2) * (b-in-fixed@t1)
        //   = (-2,0,0) + (0,1,0) = (-2, 1, 0)
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // fixed -> a at t2
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(2.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        // fixed -> b at t1
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // fixed -> b at t2
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "b",
                    Vector3::new(0.0, 2.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t2),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform_at(
            "a",     // target_frame
            t2,      // target_time
            "b",     // source_frame
            t1,      // source_time
            "fixed", // fixed_frame
        );

        assert!(
            result.is_ok(),
            "Time travel with branching tree failed: {result:?}"
        );
        let tf = result.unwrap();

        assert!(
            (tf.translation().x - (-2.0)).abs() < f64::EPSILON,
            "Expected x=-2.0, got {}",
            tf.translation().x
        );
        assert!(
            (tf.translation().y - 1.0).abs() < f64::EPSILON,
            "Expected y=1.0, got {}",
            tf.translation().y
        );
        assert!(
            tf.translation().z.abs() < f64::EPSILON,
            "Expected z=0.0, got {}",
            tf.translation().z
        );
    }

    #[test]
    fn time_travel_source_equals_fixed_returns_inverted_target_leg() {
        // "Where is the fixed/world origin relative to my platform now" —
        // source_frame == fixed_frame, a routine time-travel query that must
        // resolve to the inverse of the target leg, not error with
        // SameFrameMultiplication.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1; at t2: a has moved to x=2.
        for (t, x) in [(t1, 1.0), (t2, 2.0)] {
            registry
                .add_transform(
                    Transform::new(
                        "fixed",
                        "a",
                        Vector3::new(x, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let result = registry.get_transform_at("a", t2, "fixed", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        // Inverse of fixed -> a at t2 (x=2): the origin sits at x=-2 in "a".
        assert_eq!(tf.parent(), "a");
        assert_eq!(tf.child(), "fixed");
        assert_eq!(tf.timestamp(), Stamp::At(t2));
        assert!(
            (tf.translation().x - (-2.0)).abs() < f64::EPSILON,
            "Expected x=-2.0, got {}",
            tf.translation().x
        );
    }

    #[test]
    fn time_travel_target_equals_fixed_returns_source_leg() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // fixed -> a at t1: a is at x=1; at t2: a has moved to x=2.
        for (t, x) in [(t1, 1.0), (t2, 2.0)] {
            registry
                .add_transform(
                    Transform::new(
                        "fixed",
                        "a",
                        Vector3::new(x, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let result = registry.get_transform_at("fixed", t2, "a", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        // The source leg alone: a at t1 (x=1), stamped with target_time.
        assert_eq!(tf.parent(), "fixed");
        assert_eq!(tf.child(), "a");
        assert_eq!(tf.timestamp(), Stamp::At(t2));
        assert!(
            (tf.translation().x - 1.0).abs() < f64::EPSILON,
            "Expected x=1.0, got {}",
            tf.translation().x
        );
    }

    #[test]
    fn time_travel_all_frames_equal_returns_identity() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // The registry content is irrelevant for the degenerate query, but
        // keep it non-empty to mirror real use.
        registry
            .add_transform(
                Transform::new(
                    "fixed",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform_at("fixed", t2, "fixed", t1, "fixed");
        assert!(result.is_ok(), "get_transform_at failed: {result:?}");
        let tf = result.unwrap();

        assert_eq!(tf.parent(), "fixed");
        assert_eq!(tf.child(), "fixed");
        assert_eq!(tf.timestamp(), Stamp::At(t2));
        assert_eq!(tf.translation(), Vector3::zero());
        assert_eq!(tf.rotation(), Quaternion::identity());
    }

    #[test]
    fn get_transform_at_unknown_fixed_frame_returns_not_found() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        // Tree: fixed -> a -> b, known at both times.
        for &t in &[t1, t2] {
            registry
                .add_transform(
                    Transform::new(
                        "fixed",
                        "a",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
            registry
                .add_transform(
                    Transform::new(
                        "a",
                        "b",
                        Vector3::new(0.0, 1.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        // The fixed frame is not part of the tree: neither leg of the time
        // travel can resolve, so the whole query must fail loudly instead of
        // silently picking another reference — naming the unknown frame.
        let result = registry.get_transform_at("a", t2, "b", t1, "nowhere");
        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "nowhere"),
            "expected UnknownFrame for unknown fixed frame, got {result:?}"
        );
    }

    #[test]
    fn get_transform_at_missing_data_at_requested_times_returns_error() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        // fixed -> a is known at t1 and t2; a -> b only at t1.
        for &t in &[t1, t2] {
            registry
                .add_transform(
                    Transform::new(
                        "fixed",
                        "a",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // The source frame has no data at the requested source time: the
        // b -> fixed leg cannot resolve at t2, and the error names "b" as
        // the frame that could not serve the time.
        let result = registry.get_transform_at("a", t1, "b", t2, "fixed");
        assert!(
            matches!(&result, Err(RegistryError::NotFoundAt { frame, .. }) if frame == "b"),
            "expected NotFoundAt naming frame b for missing source data, got {result:?}"
        );

        // The target frame has no data at the requested target time: the
        // a -> fixed leg cannot resolve at t3 (no extrapolation).
        let result = registry.get_transform_at("a", t3, "b", t1, "fixed");
        assert!(
            matches!(&result, Err(RegistryError::NotFoundAt { frame, .. }) if frame == "a"),
            "expected NotFoundAt naming frame a for missing target data, got {result:?}"
        );
    }

    #[test]
    fn get_transform_for_success_with_point() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "map",
                    "camera",
                    Vector3::new(2.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            t,
            "camera",
        );

        let transform = registry.get_transform_for(&point, "map");

        assert!(transform.is_ok(), "get_transform_for failed: {transform:?}");
        let transform = transform.unwrap();
        assert_eq!(transform.parent(), "map");
        assert_eq!(transform.child(), "camera");
        assert_eq!(transform.timestamp(), Stamp::At(t));

        let result = point.transform(&transform);
        assert!(result.is_ok(), "transform apply failed: {result:?}");
        assert_eq!(point.frame, "map");
        assert_eq!(point.timestamp, t);
        assert_eq!(point.position, Vector3::new(3.0, 0.0, 0.0));
    }

    #[test]
    fn get_transform_for_same_frame_returns_identity_on_empty_registry() {
        let registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        let mut point = Point::new(
            Vector3::new(1.0, 2.0, 3.0),
            Quaternion::identity(),
            t,
            "camera",
        );

        let transform = registry.get_transform_for(&point, "camera");

        assert!(
            transform.is_ok(),
            "same-frame get_transform_for should be Ok: {transform:?}"
        );
        let transform = transform.unwrap();
        assert_eq!(transform.parent(), "camera");
        assert_eq!(transform.child(), "camera");
        assert_eq!(transform.timestamp(), Stamp::At(t));
        assert_eq!(transform.translation(), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.rotation(), Quaternion::identity());

        let result = point.transform(&transform);
        assert!(result.is_ok(), "identity apply failed: {result:?}");
        assert_eq!(point.frame, "camera");
        assert_eq!(point.position, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn get_transform_for_propagates_lookup_error() {
        let registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        let point = Point::new(
            Vector3::new(0.0, 0.0, 0.0),
            Quaternion::identity(),
            t,
            "camera",
        );

        let result = registry.get_transform_for(&point, "map");

        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "map"),
            "expected UnknownFrame on an empty registry, got {result:?}"
        );
    }

    #[test]
    fn add_transform_rejects_cycles() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        // Two-frame cycle: a -> b already exists, so b -> a must be rejected.
        let result = registry.add_transform(
            Transform::new(
                "b",
                "a",
                Vector3::new(0.0, 1.0, 0.0),
                Quaternion::identity(),
                Stamp::At(t),
            )
            .unwrap(),
        );
        assert!(matches!(result, Err(RegistryError::CycleDetected)));

        // The direct lookup keeps working; the poisoning path is gone.
        assert!(registry.get_transform("a", "b", t).is_ok());

        // Three-frame cycle: extend the chain, then try to close it.
        registry
            .add_transform(
                Transform::new(
                    "b",
                    "c",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();
        let result = registry.add_transform(
            Transform::new(
                "c",
                "a",
                Vector3::new(0.0, 0.0, 1.0),
                Quaternion::identity(),
                Stamp::At(t),
            )
            .unwrap(),
        );
        assert!(matches!(result, Err(RegistryError::CycleDetected)));
    }

    #[test]
    fn add_transform_rejects_self_referential_frames() {
        let mut registry = Registry::new();

        let result = registry.add_transform(
            Transform::new(
                "a",
                "a",
                Vector3::new(1.0, 0.0, 0.0),
                Quaternion::identity(),
                Stamp::At(Timestamp::from_nanos(1_000_000_000)),
            )
            .unwrap(),
        );
        assert!(matches!(result, Err(RegistryError::SelfReferentialFrame)));
    }

    #[test]
    fn add_transform_rejects_reparenting() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "world",
                    "object",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // The object is "picked up": its parent changes. Not supported;
        // the frame must be removed first.
        let reparented = Transform::new(
            "gripper",
            "object",
            Vector3::new(0.0, 0.5, 0.0),
            Quaternion::identity(),
            Stamp::At(t2),
        )
        .unwrap();
        let result = registry.add_transform(reparented.clone());
        assert!(matches!(
            result,
            Err(RegistryError::ReparentingNotSupported { current_parent }) if current_parent == "world"
        ));

        // remove_frame is the escape hatch: after removal the new parent is
        // accepted.
        assert!(registry.remove_frame("object"));
        assert!(!registry.remove_frame("object"));
        registry.add_transform(reparented).unwrap();
        assert!(registry.get_transform("gripper", "object", t2).is_ok());
        assert!(registry.get_transform("world", "object", t1).is_err());
    }

    #[test]
    fn remove_transforms_before_keeps_the_parent_pin() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "world",
                    "object",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // Regression test: draining a frame used to release it, so routine
        // cleanup silently turned a rejected re-parenting into an accepted
        // one and changed the topology behind the caller's back.
        registry.remove_transforms_before(t2);
        let reparented = Transform::new(
            "gripper",
            "object",
            Vector3::new(0.0, 0.5, 0.0),
            Quaternion::identity(),
            Stamp::At(t2),
        )
        .unwrap();
        let result = registry.add_transform(reparented.clone());
        assert!(
            matches!(
                result,
                Err(RegistryError::ReparentingNotSupported { ref current_parent }) if current_parent == "world"
            ),
            "cleanup must not release the parent pin, got {result:?}"
        );

        // remove_frame remains the sole escape hatch, drained or not.
        assert!(registry.remove_frame("object"));
        registry.add_transform(reparented).unwrap();
        assert!(registry.get_transform("gripper", "object", t2).is_ok());
    }

    #[test]
    fn remove_transforms_before_keeps_the_buffer_kind() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "world",
                    "object",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // Regression test: draining a frame used to release its kind too, so
        // a moving frame could become an eternal static one that answered
        // confidently at times its data never covered.
        registry.remove_transforms_before(t2);
        let result = registry.add_transform(
            Transform::new(
                "world",
                "object",
                Vector3::new(0.0, 0.5, 0.0),
                Quaternion::identity(),
                Stamp::Static,
            )
            .unwrap(),
        );
        assert!(
            matches!(result, Err(RegistryError::StaticDynamicConflict)),
            "cleanup must not release the static/dynamic kind, got {result:?}"
        );
    }

    #[test]
    fn remove_transforms_before_leaves_drained_frames_diagnosable() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "world",
                    "object",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // A drained frame is known but empty. The lookup must say so —
        // naming the frame that holds no data — instead of claiming the
        // frame was never heard of, which reads as a publisher typo.
        registry.remove_transforms_before(t2);
        let result = registry.get_transform("world", "object", t2);
        assert!(
            matches!(
                &result,
                Err(RegistryError::NotFoundAt { frame, requested, covered, .. })
                    if frame == "object" && *requested == t2 && covered.is_none()
            ),
            "expected NotFoundAt naming the drained frame, got {result:?}"
        );
    }

    #[test]
    fn get_transform_unknown_frame_returns_not_found() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        // The requested frame does not exist. The walk from "b" still resolves
        // up to the root "a", but that partial answer must not be returned as
        // if it were the requested transform; the error names the unknown
        // frame.
        let result = registry.get_transform("b", "does_not_exist", t);
        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "does_not_exist"),
            "expected UnknownFrame for unknown target frame, got {result:?}"
        );

        let result = registry.get_transform("does_not_exist", "b", t);
        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "does_not_exist"),
            "expected UnknownFrame for unknown source frame, got {result:?}"
        );
    }

    #[test]
    // The compared seconds are exactly representable; the assertion is on
    // the reported values, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn get_transform_partial_chain_reports_failing_frame() {
        let mut registry = Registry::new();
        let t0 = Timestamp::from_nanos(1_000_000_000);

        let t1 = (t0 + Duration::from_secs(1)).unwrap();

        // a -> b is only known at t0; b -> c is known at t0 and t1.
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t0),
                )
                .unwrap(),
            )
            .unwrap();
        for &t in &[t0, t1] {
            registry
                .add_transform(
                    Transform::new(
                        "b",
                        "c",
                        Vector3::new(0.0, 1.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        // At t1 only the c -> b hop can be resolved; the chain to "a" is
        // incomplete and must not be returned as a c -> a transform. The
        // error pinpoints "b" as the frame that could not serve t1 and
        // carries the covered range as the cause: b's data ends at t0
        // (1.0s), one second before the requested t1 (2.0s).
        let result = registry.get_transform("c", "a", t1);
        assert!(
            matches!(
                &result,
                Err(RegistryError::NotFoundAt { frame, requested, covered, .. })
                    if frame == "b" && *requested == t1 && *covered == Some((t0, t0))
            ),
            "expected NotFoundAt naming frame b with the covered range, got {result:?}"
        );
    }

    #[test]
    fn get_transform_mid_chain_gap_reports_gap_frame() {
        // Tree: r -> a -> b -> c. The a -> b hop is only known at t1; the
        // others are known at t1 and t3. A query at t2 hits a timestamp gap
        // in the MIDDLE of the chain, so both partial walks stop in
        // different subtrees. That is a transient data gap and must be
        // reported as NotFoundAt naming the gap frame — not
        // IncompatibleFrames, whose "frames do not have a parent-child
        // relationship" message is false here.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        for &t in &[t1, t3] {
            registry
                .add_transform(
                    Transform::new(
                        "r",
                        "a",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
            registry
                .add_transform(
                    Transform::new(
                        "b",
                        "c",
                        Vector3::new(0.0, 0.0, 1.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        // With all hops resolvable (t1) the chain works: the topology is
        // intact and only the data gap at t2 must trip the lookup.
        let result = registry.get_transform("a", "c", t1);
        assert!(
            result.is_ok(),
            "expected chain at t1 to resolve: {result:?}"
        );

        let result = registry.get_transform("a", "c", t2);
        assert!(
            matches!(&result, Err(RegistryError::NotFoundAt { frame, .. }) if frame == "b"),
            "expected NotFoundAt naming the gap frame b, got {result:?}"
        );
    }

    #[test]
    fn get_transform_disconnected_trees_returns_disconnected() {
        // Two disjoint trees: r1 -> a and r2 -> b. There is no path between
        // "a" and "b", which must be reported as Disconnected — not as a
        // failed composition of the two unrelated root transforms, and not
        // as a data gap: both frames exist and both walks complete cleanly,
        // so the disconnection is a statement about the current topology.
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "r1",
                    "a",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();
        registry
            .add_transform(
                Transform::new(
                    "r2",
                    "b",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform("a", "b", t);
        assert!(
            matches!(
                &result,
                Err(RegistryError::Disconnected { target_frame, source_frame })
                    if target_frame == "a" && source_frame == "b"
            ),
            "expected Disconnected for frames in disconnected trees, got {result:?}"
        );
    }

    #[test]
    fn get_transform_unknown_frame_takes_precedence_over_data_gap() {
        // a -> b holds data at t1 only. Querying b -> "nope" at t2 records
        // a data gap during the walk AND asks for a frame that does not
        // exist. The unknown frame is the more fundamental error — no
        // amount of waiting for data can make the lookup succeed — so it
        // must win the diagnosis.
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t1),
                )
                .unwrap(),
            )
            .unwrap();

        let result = registry.get_transform("b", "nope", t2);
        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "nope"),
            "expected UnknownFrame to take precedence over the data gap, got {result:?}"
        );
    }

    #[test]
    fn add_transform_rejects_static_dynamic_mixing() {
        let t_dynamic = Timestamp::from_nanos(1_000_000_000);

        let static_tf = Transform::new(
            "a",
            "b",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::Static,
        )
        .unwrap();
        let dynamic_tf = Transform::new(
            "a",
            "b",
            Vector3::new(2.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t_dynamic),
        )
        .unwrap();

        // Static first, then dynamic.
        let mut registry = Registry::new();

        registry.add_transform(static_tf.clone()).unwrap();
        assert!(
            matches!(
                registry.add_transform(dynamic_tf.clone()),
                Err(RegistryError::StaticDynamicConflict)
            ),
            "dynamic insert into a static child frame must be rejected"
        );

        // Dynamic first, then static.
        let mut registry = Registry::new();

        registry.add_transform(dynamic_tf.clone()).unwrap();
        assert!(
            matches!(
                registry.add_transform(static_tf),
                Err(RegistryError::StaticDynamicConflict)
            ),
            "static insert into a dynamic child frame must be rejected"
        );

        // The registry state is untouched by the rejected insert.
        let result = registry.get_transform("a", "b", t_dynamic);
        assert_eq!(result.unwrap(), dynamic_tf);
    }

    #[test]
    fn remove_transforms_before_removes_old_dynamic_transforms() {
        let mut registry = Registry::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);

        for &t in &[t1, t2] {
            registry
                .add_transform(
                    Transform::new(
                        "a",
                        "b",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        registry.remove_transforms_before(Timestamp::from_nanos(2_000_000_000));

        assert!(
            registry.get_transform("a", "b", t1).is_err(),
            "transforms before the cutoff must be removed"
        );
        assert!(registry.get_transform("a", "b", t2).is_ok());
    }

    #[test]
    fn remove_transforms_before_preserves_static_transforms() {
        let mut registry = Registry::new();

        let static_tf = Transform::new(
            "base",
            "lidar",
            Vector3::new(0.5, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::Static,
        )
        .unwrap();
        registry.add_transform(static_tf.clone()).unwrap();

        // The documented manual-cleanup workflow must not destroy static
        // transforms: they are valid for all time.
        registry.remove_transforms_before(Timestamp::from_nanos(5_000_000_000));

        let query = Timestamp::from_nanos(9_000_000_000);
        let result = registry.get_transform("base", "lidar", query).unwrap();
        assert_eq!(
            result.translation(),
            static_tf.translation(),
            "static transforms must survive manual cleanup"
        );
        // Lookup results carry the requested timestamp, not the static
        // sentinel, so they compose with timestamped data.
        assert_eq!(result.timestamp(), Stamp::At(query));
    }

    #[test]
    fn mixed_static_dynamic_chain_resolves_and_interpolates() {
        let mut registry = Registry::new();

        // Static sensor mount: lidar sits 0.5 m ahead of base.
        registry
            .add_transform(
                Transform::new(
                    "base",
                    "lidar",
                    Vector3::new(0.5, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::Static,
                )
                .unwrap(),
            )
            .unwrap();

        // Dynamic robot pose: base moves from x=1 to x=3 between t1 and t2.
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);
        for (t, x) in [(t1, 1.0), (t2, 3.0)] {
            registry
                .add_transform(
                    Transform::new(
                        "map",
                        "base",
                        Vector3::new(x, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        // Query mid-way: the dynamic hop interpolates to x=2, the static hop
        // contributes its fixed 0.5 offset, and the result carries the query
        // timestamp.
        let mid = Timestamp::from_nanos(2_000_000_000);
        let result = registry.get_transform("map", "lidar", mid).unwrap();

        assert_eq!(result.parent(), "map");
        assert_eq!(result.child(), "lidar");
        assert_eq!(result.timestamp(), Stamp::At(mid));
        assert_abs_diff_eq!(result.translation(), Vector3::new(2.5, 0.0, 0.0));
    }

    #[test]
    // The overflow assertion compares against f64::INFINITY, which is exact;
    // stable clippy flags it where nightly no longer does.
    #[allow(clippy::float_cmp)]
    fn add_transform_rejects_a_republished_chain_that_left_validity() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // Both operands are valid: norm 1.000001 is inside
        // UNIT_NORM_TOLERANCE — an f32-widened rotation, exactly what the
        // tolerance exists to accept. Composition multiplies the norms and
        // `*` deliberately does not re-check, so flattening an ordinary
        // two-hop chain yields a rotation that is out of tolerance while
        // looking entirely unremarkable.
        let q = Quaternion::from_wxyz(1.0 + 1e-6, 0.0, 0.0, 0.0);
        let t_a_b = Transform::new("a", "b", Vector3::new(1.0, 0.0, 0.0), q, Stamp::At(t)).unwrap();
        let t_b_c = Transform::new("b", "c", Vector3::new(1.0, 0.0, 0.0), q, Stamp::At(t)).unwrap();
        let flattened = (t_a_b * t_b_c).unwrap();
        assert!(flattened.rotation().norm() > 1.0 + UNIT_NORM_TOLERANCE);

        // Re-publishing it must fail. Stored, it would scale every vector
        // every later lookup through the frame rotates, and report success.
        assert!(matches!(
            registry.add_transform(flattened),
            Err(RegistryError::NonUnitRotation(_))
        ));

        // The same for a translation that overflowed during composition.
        let far = Transform::new(
            "a",
            "b",
            Vector3::new(1.0e308, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();
        let farther = Transform::new(
            "b",
            "c",
            Vector3::new(1.0e308, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();
        let overflowed = (far * farther).unwrap();
        assert_eq!(overflowed.translation().x, f64::INFINITY);

        assert!(matches!(
            registry.add_transform(overflowed),
            Err(RegistryError::NonFiniteValues)
        ));

        // Nothing was stored, so no lookup can serve either value.
        assert!(registry.get_transform("a", "c", t).is_err());
    }

    #[test]
    fn with_max_age_expires_old_transforms_on_insert() {
        let mut registry = Registry::with_max_age(Duration::from_secs(1));

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(6_000_000_000);
        for &t in &[t1, t2] {
            registry
                .add_transform(
                    Transform::new(
                        "a",
                        "b",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        assert!(
            registry.get_transform("a", "b", t1).is_err(),
            "with_max_age registries must expire entries older than max_age"
        );
        assert!(registry.get_transform("a", "b", t2).is_ok());

        // A registry without max_age keeps everything.
        let mut registry = Registry::new();
        for &t in &[t1, t2] {
            registry
                .add_transform(
                    Transform::new(
                        "a",
                        "b",
                        Vector3::new(1.0, 0.0, 0.0),
                        Quaternion::identity(),
                        Stamp::At(t),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        assert!(registry.get_transform("a", "b", t1).is_ok());
        assert!(registry.get_transform("a", "b", t2).is_ok());
    }

    #[test]
    fn failed_insert_does_not_bypass_cycle_detection() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        // A rejected insert must not leave an empty frame behind in the
        // registry map — here one rejected for naming "a" as its own parent,
        // which still asks the registry to create the frame "a".
        let invalid = Transform::new(
            "a",
            "a",
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap();
        assert!(matches!(
            registry.add_transform(invalid),
            Err(RegistryError::SelfReferentialFrame)
        ));

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        // ...otherwise this valid insert would close the cycle a <-> b
        // without ever hitting the cycle check.
        let result = registry.add_transform(
            Transform::new(
                "b",
                "a",
                Vector3::new(-1.0, 0.0, 0.0),
                Quaternion::identity(),
                Stamp::At(t),
            )
            .unwrap(),
        );
        assert!(matches!(result, Err(RegistryError::CycleDetected)));

        // The stored transform still resolves, unpoisoned.
        let result = registry.get_transform("a", "b", t).unwrap();
        assert_eq!(result.translation(), Vector3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn single_hop_lookup_returns_the_stored_transform_bit_for_bit() {
        // In the documented direction — target is the source's parent — only
        // the source-side chain is walked, and a one-element chain composes
        // into itself: no inversion, no renormalization, no arithmetic at
        // all. The answer is the stored sample, down to the last bit. The
        // pre-rework path reached the same transform through two inversions
        // and returned a translation up to ten ulps away from the stored one.
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_734_000_000_123_456_789);
        let stored = Transform::new(
            "map",
            "lidar",
            Vector3::new(0.1, -2.5, 3.75),
            Quaternion::from_wxyz(0.3, -0.5, 0.7, 0.2)
                .normalize()
                .unwrap(),
            Stamp::At(t),
        )
        .unwrap();
        registry.add_transform(stored.clone()).unwrap();

        let result = registry.get_transform("map", "lidar", t).unwrap();

        assert_eq!(result.parent(), stored.parent());
        assert_eq!(result.child(), stored.child());
        assert_eq!(result.timestamp(), stored.timestamp());
        for (got, expected) in [
            (result.translation().x, stored.translation().x),
            (result.translation().y, stored.translation().y),
            (result.translation().z, stored.translation().z),
            (result.rotation().w, stored.rotation().w),
            (result.rotation().x, stored.rotation().x),
            (result.rotation().y, stored.rotation().y),
            (result.rotation().z, stored.rotation().z),
        ] {
            assert_eq!(
                got.to_bits(),
                expected.to_bits(),
                "component changed: {got} vs {expected}"
            );
        }
    }

    #[test]
    fn same_frame_lookup_returns_identity() {
        let mut registry = Registry::new();
        let t = Timestamp::from_nanos(1_000_000_000);

        registry
            .add_transform(
                Transform::new(
                    "a",
                    "b",
                    Vector3::new(1.0, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(t),
                )
                .unwrap(),
            )
            .unwrap();

        // Identity for a known child frame, a root frame, and an unknown
        // frame alike: a frame relative to itself is always the identity.
        for frame in ["b", "a", "unknown"] {
            let result = registry.get_transform(frame, frame, t).unwrap();
            assert_eq!(result.parent(), frame);
            assert_eq!(result.child(), frame);
            assert_eq!(result.timestamp(), Stamp::At(t));
            assert_eq!(result.translation(), Vector3::zero());
            assert_eq!(result.rotation(), Quaternion::identity());
        }
    }

    #[test]
    fn static_chain_composes_with_timestamped_data() {
        let mut registry = Registry::new();

        // Purely static chain: base -> camera mount.
        registry
            .add_transform(
                Transform::new(
                    "base",
                    "camera",
                    Vector3::new(0.0, 1.0, 0.0),
                    Quaternion::identity(),
                    Stamp::Static,
                )
                .unwrap(),
            )
            .unwrap();

        // A detection stamped at observation time, in the camera frame.
        let t = Timestamp::from_nanos(5_000_000_000);
        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            t,
            "camera",
        );

        // The flagship static-mount workflow: resolve and apply. The lookup
        // result carries the query time, so the application succeeds.
        let tf = registry.get_transform_for(&point, "base").unwrap();
        assert_eq!(tf.timestamp(), Stamp::At(t));
        point.transform(&tf).unwrap();
        assert_eq!(point.frame, "base");
        assert_eq!(point.position, Vector3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn static_transform_applies_directly_to_any_timestamp() {
        // A hand-built static transform (Stamp::Static) is valid
        // for all time when applied through Transformable.
        let static_tf = Transform::new(
            "base",
            "camera",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::Static,
        )
        .unwrap();

        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Timestamp::from_nanos(5_000_000_000),
            "camera",
        );

        point.transform(&static_tf).unwrap();
        assert_eq!(point.frame, "base");
        assert_eq!(point.position, Vector3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn public_types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Registry>();
        assert_send_sync::<RegistryError>();
        assert_send_sync::<Transform>();
        assert_send_sync::<Point>();
        assert_send_sync::<Vector3>();
        assert_send_sync::<Quaternion>();
        assert_send_sync::<Timestamp>();
    }

    /// A transform translated by `x` along the x-axis.
    fn translated(
        parent: &str,
        child: &str,
        timestamp: Stamp,
        x: f64,
    ) -> Transform {
        Transform::new(
            parent,
            child,
            Vector3::new(x, 0.0, 0.0),
            Quaternion::identity(),
            timestamp,
        )
        .unwrap()
    }

    #[test]
    fn an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts() {
        // Two individually finite hops compose to an infinite translation.
        // Where the lookup inverts the chain it notices, and the condition
        // must arrive as the same flat `NonFiniteValues` an insert reports:
        // one spelling per condition, so a caller matching it cannot miss a
        // wrapped copy arriving from the other code path.
        //
        // The opposite direction pins the scope of that claim. A lookup
        // toward an ancestor resolves entirely from the source half and
        // inverts nothing, and a lookup result is deliberately never
        // re-validated, so the overflow is returned as `Ok`. That is what
        // the docs on `get_transform` and `RegistryError::NonFiniteValues`
        // say; if this assertion ever starts failing, they must change with
        // it rather than the other way around.
        let t = Timestamp::from_nanos(1_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::At(t), 1.0e308))
            .unwrap();
        registry
            .add_transform(translated("b", "c", Stamp::At(t), 1.0e308))
            .unwrap();

        let inverting = registry.get_transform("c", "a", t);
        assert!(
            matches!(inverting, Err(RegistryError::NonFiniteValues)),
            "expected a flat NonFiniteValues, got {inverting:?}"
        );

        let ancestor_ward = registry.get_transform("a", "c", t).unwrap();
        assert_eq!(
            ancestor_ward.translation(),
            Vector3::new(f64::INFINITY, 0.0, 0.0)
        );
    }

    #[test]
    fn not_found_at_renders_both_coverage_cases_in_seconds() {
        // Error formatting goes through `TimePoint::as_seconds_lossy`, which
        // is infallible by contract: neither shape of `covered` can fail to
        // render and mask the error being reported. The two shapes must also
        // read differently — a drained frame is not a timing problem.
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::At(t1), 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Stamp::At(t2), 2.0))
            .unwrap();

        let gap = registry.get_transform("a", "b", t3).unwrap_err();
        assert_eq!(
            alloc::format!("{gap}"),
            "transform from b into a at 3 not found (b covers [1, 2])"
        );

        registry.remove_transforms_before(t3);
        let drained = registry.get_transform("a", "b", t3).unwrap_err();
        assert_eq!(
            alloc::format!("{drained}"),
            "transform from b into a at 3 not found (b holds no transforms)"
        );
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn duplicate_timestamp_add_is_a_last_write_wins_upsert() {
        let t = Timestamp::from_nanos(5_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::At(t), 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Stamp::At(t), 2.0))
            .unwrap();
        assert_eq!(
            registry.get_transform("a", "b", t).unwrap().translation().x,
            2.0
        );
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn duplicate_static_add_is_a_last_write_wins_upsert() {
        // Re-publishing a static transform replaces it: last write wins.
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::Static, 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Stamp::Static, 7.0))
            .unwrap();
        let got = registry
            .get_transform("a", "b", Timestamp::from_nanos(3_000_000_000))
            .unwrap();
        assert_eq!(got.translation().x, 7.0);
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn zero_max_age_keeps_only_the_newest_sample() {
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);
        let mut registry = Registry::with_max_age(Duration::ZERO);
        registry
            .add_transform(translated("a", "b", Stamp::At(t1), 1.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Stamp::At(t2), 2.0))
            .unwrap();
        registry
            .add_transform(translated("a", "b", Stamp::At(t3), 3.0))
            .unwrap();

        // Exact hit on the newest sample still works.
        assert_eq!(
            registry
                .get_transform("a", "b", t3)
                .unwrap()
                .translation()
                .x,
            3.0
        );

        // Older samples are gone: the covered range collapsed to [t3, t3].
        match registry.get_transform("a", "b", t2) {
            Err(RegistryError::NotFoundAt {
                frame,
                requested,
                covered,
                ..
            }) => {
                assert_eq!(frame, "b");
                assert_eq!(requested, t2);
                assert_eq!(covered, Some((t3, t3)));
            }
            other => panic!("expected NotFoundAt, got {other:?}"),
        }
    }

    #[test]
    fn latest_common_time_is_bounded_by_the_laggiest_hop() {
        // map -> odom covers [4us, 9us], odom -> base covers [3us, 7us]:
        // the newest instant the whole chain serves is the laggiest hop's
        // newest sample.
        let mut registry = Registry::new();
        for (parent, child, nanos) in [
            ("map", "odom", 4_000),
            ("map", "odom", 9_000),
            ("odom", "base", 3_000),
            ("odom", "base", 7_000),
        ] {
            registry
                .add_transform(translated(
                    parent,
                    child,
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }

        let stamp = registry.latest_common_time("map", "base").unwrap();
        assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(7_000)));

        // Symmetric: both lookup directions serve the same instants.
        let mirrored = registry.latest_common_time("base", "map").unwrap();
        assert_eq!(mirrored, stamp);

        // The answer is servable and maximal: one nanosecond later, the
        // laggy hop has nothing.
        assert!(
            registry
                .get_transform("map", "base", Timestamp::from_nanos(7_000))
                .is_ok()
        );
        assert!(
            registry
                .get_transform("map", "base", Timestamp::from_nanos(7_001))
                .is_err()
        );
    }

    #[test]
    fn latest_common_time_ignores_edges_above_the_common_ancestor() {
        // world -> map went stale long ago; map -> a and map -> b are fresh.
        // The a <-> b chain crosses only the two fresh edges, so the stale
        // edge above their common ancestor must not bound the answer —
        // this is where the retired retry-off-`covered` idiom was merely
        // conservative.
        let mut registry = Registry::new();
        for (parent, child, nanos) in [
            ("world", "map", 1_000),
            ("world", "map", 2_000),
            ("map", "a", 10_000),
            ("map", "a", 20_000),
            ("map", "b", 15_000),
            ("map", "b", 25_000),
        ] {
            registry
                .add_transform(translated(
                    parent,
                    child,
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }

        let stamp = registry.latest_common_time("a", "b").unwrap();
        assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(20_000)));
        assert!(
            registry
                .get_transform("a", "b", Timestamp::from_nanos(20_000))
                .is_ok()
        );

        // A chain that does cross the stale edge has no common instant:
        // world -> map ends at 2us, map -> b starts at 15us.
        let result = registry.latest_common_time("world", "b");
        assert!(
            matches!(
                &result,
                Err(RegistryError::NoCommonTime { frame, covered, .. })
                    if frame == "b"
                        && *covered
                            == Some((
                                Timestamp::from_nanos(15_000),
                                Timestamp::from_nanos(25_000)
                            ))
            ),
            "expected NoCommonTime naming the late-starting hop, got {result:?}"
        );
    }

    #[test]
    fn latest_common_time_reports_disjoint_ranges_as_no_common_time() {
        // a -> b covers [0, 5us], b -> c covers [10us, 20us]: no instant is
        // covered by both. The naive "minimum of the hops' newest samples"
        // would land on 5us — an instant hop c cannot serve — so the
        // refusal, not a plausible-looking instant, is the contract.
        let mut registry = Registry::new();
        for (parent, child, nanos) in [
            ("a", "b", 0),
            ("a", "b", 5_000),
            ("b", "c", 10_000),
            ("b", "c", 20_000),
        ] {
            registry
                .add_transform(translated(
                    parent,
                    child,
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }

        let result = registry.latest_common_time("a", "c");
        assert!(
            matches!(
                &result,
                Err(RegistryError::NoCommonTime { target_frame, source_frame, frame, covered })
                    if target_frame == "a"
                        && source_frame == "c"
                        && frame == "c"
                        && *covered
                            == Some((
                                Timestamp::from_nanos(10_000),
                                Timestamp::from_nanos(20_000)
                            ))
            ),
            "expected NoCommonTime carrying the disjoint range, got {result:?}"
        );

        // And indeed no instant serves: each candidate fails on some hop.
        assert!(
            registry
                .get_transform("a", "c", Timestamp::from_nanos(5_000))
                .is_err()
        );
        assert!(
            registry
                .get_transform("a", "c", Timestamp::from_nanos(10_000))
                .is_err()
        );
    }

    #[test]
    fn latest_common_time_returns_static_for_all_static_chains() {
        // A chain of static hops puts no bound on time: the caller picks
        // the instant, exactly as a lookup would accept any.
        let mut registry: Registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::Static, 1.0))
            .unwrap();
        registry
            .add_transform(translated("b", "c", Stamp::Static, 1.0))
            .unwrap();

        assert_eq!(
            registry.latest_common_time("a", "c").unwrap(),
            Stamp::Static
        );

        // A frame relative to itself serves any instant too — also for a
        // frame the registry has never seen, matching `get_transform`'s
        // identity behavior.
        assert_eq!(
            registry.latest_common_time("nowhere", "nowhere").unwrap(),
            Stamp::Static
        );
    }

    #[test]
    fn latest_common_time_skips_static_hops_in_mixed_chains() {
        // The static mount serves every instant, so only the dynamic hop
        // bounds the answer.
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::Static, 1.0))
            .unwrap();
        for nanos in [3_000, 8_000] {
            registry
                .add_transform(translated(
                    "b",
                    "c",
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }

        assert_eq!(
            registry.latest_common_time("a", "c").unwrap(),
            Stamp::At(Timestamp::from_nanos(8_000))
        );
    }

    #[test]
    fn latest_common_time_reports_drained_frames_as_no_common_time() {
        // A drained frame serves nothing, so the chain has no common
        // instant — reported with `covered: None`, mirroring `NotFoundAt`'s
        // two-case payload.
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("world", "object", Stamp::At(t1), 1.0))
            .unwrap();
        registry.remove_transforms_before(t2);

        let result = registry.latest_common_time("world", "object");
        assert!(
            matches!(
                &result,
                Err(RegistryError::NoCommonTime { frame, covered, .. })
                    if frame == "object" && covered.is_none()
            ),
            "expected NoCommonTime naming the drained frame, got {result:?}"
        );
    }

    #[test]
    fn latest_common_time_ignores_a_drained_edge_above_the_common_ancestor() {
        // The drained world -> map edge sits above the a/b junction, so the
        // a <-> b chain never crosses it. This pins the emptiness check to
        // the truncated chain: hoisting it in front of the shared-suffix
        // drop would refuse a pair `get_transform` happily serves.
        let mut registry = Registry::new();
        registry
            .add_transform(translated(
                "world",
                "map",
                Stamp::At(Timestamp::from_nanos(1_000)),
                1.0,
            ))
            .unwrap();
        for (parent, child, nanos) in [
            ("map", "a", 10_000),
            ("map", "a", 20_000),
            ("map", "b", 15_000),
            ("map", "b", 25_000),
        ] {
            registry
                .add_transform(translated(
                    parent,
                    child,
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }
        registry.remove_transforms_before(Timestamp::from_nanos(5_000));

        let stamp = registry.latest_common_time("a", "b").unwrap();
        assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(20_000)));
        assert!(
            registry
                .get_transform("a", "b", Timestamp::from_nanos(20_000))
                .is_ok()
        );
    }

    #[test]
    fn latest_common_time_answers_at_the_single_shared_instant_of_touching_ranges() {
        // a -> b covers [0, 5us], b -> c covers [5us, 20us]: exactly one
        // instant is covered by both, and it is the answer — the boundary
        // where an off-by-one in the disjointness guard would refuse.
        let mut registry = Registry::new();
        for (parent, child, nanos) in [
            ("a", "b", 0),
            ("a", "b", 5_000),
            ("b", "c", 5_000),
            ("b", "c", 20_000),
        ] {
            registry
                .add_transform(translated(
                    parent,
                    child,
                    Stamp::At(Timestamp::from_nanos(nanos)),
                    1.0,
                ))
                .unwrap();
        }

        let stamp = registry.latest_common_time("a", "c").unwrap();
        assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(5_000)));
        assert!(
            registry
                .get_transform("a", "c", Timestamp::from_nanos(5_000))
                .is_ok()
        );
    }

    #[test]
    fn latest_common_time_diagnoses_unknown_and_disconnected_like_a_lookup() {
        let t = Timestamp::from_nanos(1_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("a", "b", Stamp::At(t), 1.0))
            .unwrap();
        registry
            .add_transform(translated("x", "y", Stamp::At(t), 1.0))
            .unwrap();

        // Same diagnosis, same match arms as a failed `get_transform`.
        let result = registry.latest_common_time("a", "does_not_exist");
        assert!(
            matches!(&result, Err(RegistryError::UnknownFrame(frame)) if frame == "does_not_exist"),
            "expected UnknownFrame, got {result:?}"
        );

        let result = registry.latest_common_time("b", "y");
        assert!(
            matches!(
                &result,
                Err(RegistryError::Disconnected { target_frame, source_frame })
                    if target_frame == "b" && source_frame == "y"
            ),
            "expected Disconnected, got {result:?}"
        );
    }

    #[test]
    fn remove_frame_mid_tree_strands_descendants() {
        // map -> odom -> base_link; removing odom strands base_link, whose
        // buffer keeps its pin to the removed parent. The subsequent lookup
        // is diagnosed relative to the remaining tree: "map" now exists
        // nowhere (it was only ever odom's parent), so the error names it —
        // the documented, deliberately-pinned behavior.
        let t = Timestamp::from_nanos(1_000_000_000);
        let mut registry = Registry::new();
        registry
            .add_transform(translated("map", "odom", Stamp::At(t), 1.0))
            .unwrap();
        registry
            .add_transform(translated("odom", "base_link", Stamp::At(t), 1.0))
            .unwrap();

        assert!(registry.remove_frame("odom"));

        match registry.get_transform("map", "base_link", t) {
            Err(RegistryError::UnknownFrame(frame)) => assert_eq!(frame, "map"),
            other => panic!("expected UnknownFrame, got {other:?}"),
        }
    }
}
