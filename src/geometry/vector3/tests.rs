#[cfg(test)]
mod vector3_tests {
    use approx::assert_relative_eq;

    use crate::geometry::Vector3;

    #[test]
    fn add() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let expected = Vector3::new(5.0, 7.0, 9.0);
        assert_eq!(v1 + v2, expected);
    }

    #[test]
    fn sub() {
        let v1 = Vector3::new(4.0, 5.0, 6.0);
        let v2 = Vector3::new(1.0, 2.0, 3.0);
        let expected = Vector3::new(3.0, 3.0, 3.0);
        assert_eq!(v1 - v2, expected);
    }

    #[test]
    fn mul_scalar() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        let scalar = 2.0;
        let expected = Vector3::new(2.0, 4.0, 6.0);
        assert_eq!(v * scalar, expected);
    }

    #[test]
    fn div_scalar() {
        let v = Vector3::new(2.0, 4.0, 6.0);
        let scalar = 2.0;
        let expected = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v / scalar, expected);
    }

    #[test]
    fn dot_product() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let expected = 32.0;
        assert_relative_eq!(v1.dot(v2), expected);
    }

    #[test]
    fn cross_product() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let expected = Vector3::new(-3.0, 6.0, -3.0);
        assert_eq!(v1.cross(v2), expected);
    }
}
