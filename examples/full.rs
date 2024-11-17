use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};

/// Dummy transform generator
fn generate_transform(t: Timestamp) -> Transform {
    let x = t.as_seconds().unwrap().sin();
    let y = t.as_seconds().unwrap().cos();
    let z = 0.;

    Transform {
        translation: Vector3 { x, y, z },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        parent: "a".into(),
        child: "b".into(),
        timestamp: t,
    }
}

#[tokio::main]
async fn main() {
    let ttl = Duration::new(10, 0);
    let map = Arc::new(RwLock::new(Registry::new(ttl.into())));

    let time = Timestamp::now();
    let t = generate_transform(time);
}
