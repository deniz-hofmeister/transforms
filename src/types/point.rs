use crate::types::{Quaternion, Timestamp, Vector3};

#[derive(Debug, Clone)]
pub struct Point {
    pub position: Vector3,
    pub orientation: Quaternion,
    pub timestamp: Timestamp,
}
