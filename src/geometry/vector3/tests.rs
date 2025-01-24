#[cfg(test)]
mod vector3_tests {
    use approx::assert_relative_eq;

    use crate::geometry::Vector3;

    #[test]
    fn add() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = Vector3 {
            x: 5.0,
            y: 7.0,
            z: 9.0,
        };
        assert_eq!(
            v1 + v2,
            expected,
            "Addition of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1 + v2
        );
    }

    #[test]
    fn sub() {
        let v1 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let v2 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let expected = Vector3 {
            x: 3.0,
            y: 3.0,
            z: 3.0,
        };
        assert_eq!(
            v1 - v2,
            expected,
            "Subtraction of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1 - v2
        );
    }

    #[test]
    fn mul_scalar() {
        let v = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let scalar = 2.0;
        let expected = Vector3 {
            x: 2.0,
            y: 4.0,
            z: 6.0,
        };
        assert_eq!(
            v * scalar,
            expected,
            "Multiplication of {:?} by scalar {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v,
            scalar,
            expected,
            v * scalar
        );
    }

    #[test]
    fn div_scalar() {
        let v = Vector3 {
            x: 2.0,
            y: 4.0,
            z: 6.0,
        };
        let scalar = 2.0;
        let expected = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        assert_eq!(
            v / scalar,
            expected,
            "Division of {:?} by scalar {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v,
            scalar,
            expected,
            v / scalar
        );
    }

    #[test]
    fn dot_product() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = 32.0;
        assert_relative_eq!(v1.dot(v2), expected);
    }

    #[test]
    fn cross_product() {
        let v1 = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vector3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let expected = Vector3 {
            x: -3.0,
            y: 6.0,
            z: -3.0,
        };
        assert_eq!(
            v1.cross(v2),
            expected,
            "Cross product of {:?} and {:?} was incorrect. Expected: {:?}, Got: {:?}",
            v1,
            v2,
            expected,
            v1.cross(v2)
        );
    }
}
