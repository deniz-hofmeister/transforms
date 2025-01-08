#[cfg(test)]
mod transform_tests {
    use crate::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
    };

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
        let timestamp = Timestamp { time: 0 };
        let parent = "map".into();
        let child = "base".into();

        let _t = Transform {
            translation,
            rotation,
            timestamp,
            parent,
            child,
        };
    }
}
