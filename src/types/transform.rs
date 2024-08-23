use crate::types::Point;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Transform {
    pub transform: Point,
    parent: Option<Weak<RefCell<Transform>>>,
    children: RefCell<Vec<Rc<RefCell<Transform>>>>,
}

impl Transform {
    fn new(transform: Point) -> Rc<Self> {
        Rc::new(Transform {
            transform,
            parent: None,
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

        let transform = Point {
            position: v,
            orientation: q,
            timestamp: t,
        };

        let parent = "A";
        let child = "B";

        // let _t = Transform {
        //     transform,
        //     parent,
        //     child,
        // };
    }
}
