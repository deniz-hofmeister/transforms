use transforms::types::{Point, Quaternion, Timestamp, Transform, Vector3};

fn main() {
    let timestamp = Timestamp::now();

    let point = Point {
        position: Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        orientation: Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        timestamp: timestamp.clone(),
        frame: "b".into(),
    };

    let t = Transform {
        translation: Vector3 {
            x: 1.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        parent: "a".into(),
        child: "b".into(),
        timestamp,
    };
}
