#[cfg(test)]
mod point_tests {
    use crate::types::{Point, Quaternion, Timestamp, Vector3};

    #[test]
    fn point_creation() {
        let v = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let q = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let t = Timestamp::now();

        let _p = Point {
            position: v,
            orientation: q,
            timestamp: t,
        };
    }
}
