#[cfg(test)]
mod transform_tests {
    use crate::types::{Quaternion, Timestamp, Transform, Vector3};

    #[test]
    fn transform_creation() {
        let translation = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let rotation = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let timestamp = Timestamp::now();
        let parent = "map".to_string();
        let child = "base".to_string();

        let _t = Transform {
            translation,
            rotation,
            timestamp,
            parent,
            child,
        };
    }
}
