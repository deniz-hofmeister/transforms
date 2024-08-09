#[derive(Debug, Clone, Copy)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[test]
fn test_vector3() {
    let _v = Vector3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
}
