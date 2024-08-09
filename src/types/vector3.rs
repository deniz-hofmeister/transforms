#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vector3_creation() {
        let _v = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
    }
}
