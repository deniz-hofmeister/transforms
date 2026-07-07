#[cfg(test)]
mod point_tests {
    use crate::{
        Transform, Transformable,
        geometry::{Point, Quaternion, Vector3},
        time::Timestamp,
    };
    use approx::assert_abs_diff_eq;

    #[test]
    fn point_creation() {
        let _ = Point {
            position: Vector3::new(1.0, 2.0, 3.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "a".into(),
        };
    }

    #[test]
    fn transform_rotates_orientation() {
        let theta = core::f64::consts::PI / 2.0;
        let rot_z_90 = Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin());

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "b".into(),
        };

        let transform = Transform {
            translation: Vector3::zero(),
            rotation: rot_z_90,
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        point.transform(&transform).unwrap();

        // The orientation must be rotated (quaternion product), not merely
        // combined component-wise.
        assert_abs_diff_eq!(point.orientation, rot_z_90);
        assert_abs_diff_eq!(point.position, Vector3::new(0.0, 1.0, 0.0), epsilon = 1e-10);
    }
}
