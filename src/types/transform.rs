use crate::types::{Quaternion, Timestamp, Vector3};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub timestamp: Timestamp,
    pub frame: String,
    parent: Option<Weak<RefCell<Transform>>>,
    children: RefCell<Vec<Rc<RefCell<Transform>>>>,
}

impl Transform {
    fn new(
        translation: Vector3,
        rotation: Quaternion,
        timestamp: Timestamp,
        parent: Option<Weak<RefCell<Transform>>>,
    ) -> Rc<Self> {
        Rc::new(Transform {
            translation,
            rotation,
            timestamp,
            parent,
            children: RefCell::new(vec![]),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{Quaternion, Timestamp, Vector3};

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

        let parent = "A";
        let child = "B";

        let _t = Transform::new(transform);
    }
}
