use crate::types::{Quaternion, Timestamp, Vector3};
mod error;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Point {
    pub position: Vector3,
    pub orientation: Quaternion,
    pub timestamp: Timestamp,
}

#[cfg(test)]
mod tests;
