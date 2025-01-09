#[cfg(test)]
mod point_tests {
    use crate::{
        geometry::{Point, Quaternion, Vector3},
        time::Timestamp,
    };
    use alloc::string::String;

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
        let t = Timestamp { t: 0 };
        let f = String::from("a");

        let _p = Point {
            position: v,
            orientation: q,
            timestamp: t,
            frame: f,
        };
    }
}
