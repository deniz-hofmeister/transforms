use crate::types::{Quaternion, Timestamp, Vector3};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub timestamp: Timestamp,
    pub frame: String,
    pub parent: String,
    _parent: Option<Weak<RefCell<Transform>>>,
    _children: RefCell<Vec<Rc<RefCell<Transform>>>>,
}

impl Transform {
    pub fn new(
        translation: Vector3,
        rotation: Quaternion,
        timestamp: Timestamp,
        frame: &str,
        parent: Option<Weak<RefCell<Transform>>>,
    ) -> Rc<Self> {
        Rc::new(Transform {
            translation,
            rotation,
            timestamp,
            frame: frame.to_string(),
            parent,
            children: RefCell::new(Vec::new()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
        let f = "map";
        let p = None;

        let _t = Transform::new(v, q, t, f, p);
    }
}
