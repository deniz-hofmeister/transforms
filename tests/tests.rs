#[cfg(test)]
mod tests {
    use transforms::types::{Point, Quaternion, Registry, Timestamp, Transform, Vector3};

    fn test_transforms() {
        let registry = Registry::new(u128::MAX);

        // Child frame B at x=1m without rotation
        let t_a_b = Transform {
            transform: Point {
                position: Vector3 {
                    x: 1.,
                    y: 0.,
                    z: 0.,
                },
                orientation: Quaternion {
                    w: 1.,
                    x: 0.,
                    y: 0.,
                    z: 0.,
                },
                timestamp: Timestamp::now(),
            },
            parent: "a",
            child: "b",
        };

        // Child frame C at y=1m with 90 degrees rotation around +Z
        let theta = std::f64::consts::PI / 2.0;
        let t_a_c = Transform {
            transform: Point {
                position: Vector3 {
                    x: 0.,
                    y: 1.,
                    z: 0.,
                },
                orientation: Quaternion {
                    w: (theta / 2.0).cos(),
                    x: 0.,
                    y: 0.,
                    z: (theta / 2.0).sin(),
                },
                timestamp: Timestamp::now(),
            },
            parent: "a",
            child: "c",
        };

        registry.add(t_a_b);
        registry.add(t_a_c);

        registry.get_transform("a", "b");
        registry.get_transform("a", "c");
        registry.get_transform("b", "c");
    }
}
