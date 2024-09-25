#[cfg(test)]
mod test {
    use crate::types::{Quaternion, Timestamp, Transform, Vector3};

    #[test]
    fn transform_creation() {
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
        let f = "base";
        let p = "map";

        let _t = Transform::new(v, q, t, f, p);
    }
}
