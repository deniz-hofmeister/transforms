#[cfg(test)]
mod point_tests {
    use crate::{
        geometry::{Point, Quaternion, Vector3},
        time::Timestamp,
    };

    #[test]
    fn point_creation() {
        let _ = Point {
            position: Vector3::new(1.0, 2.0, 3.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "a".into(),
        };
    }
}
