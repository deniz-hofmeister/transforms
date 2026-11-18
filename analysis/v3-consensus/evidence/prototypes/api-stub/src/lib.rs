#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(unused_variables, clippy::missing_errors_doc, clippy::must_use_candidate, clippy::needless_pass_by_value, clippy::missing_panics_doc)] // stub artefacts
use libm as _;
extern crate alloc;
use alloc::string::String;
use core::time::Duration;

// ---- geometry ----
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vector3 { pub x: f64, pub y: f64, pub z: f64 }
impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self { Self { x, y, z } }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rotation { w: f64, x: f64, y: f64, z: f64 }
impl Rotation {
    pub const IDENTITY: Self = Self { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
    pub fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Result<Self, TransformError> { todo!() }
    pub fn w(self) -> f64 { todo!() }
    pub fn x(self) -> f64 { todo!() }
    pub fn y(self) -> f64 { todo!() }
    pub fn z(self) -> f64 { todo!() }
    pub fn rotate(self, v: Vector3) -> Vector3 { todo!() }
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform { target: String, source: String, translation: Vector3, rotation: Rotation, stamp: Stamp }
impl Transform {
    pub fn new(
        target: impl Into<String>,
        source: impl Into<String>,
        translation: Vector3,
        rotation: Rotation,
        stamp: Stamp,
    ) -> Result<Self, TransformError> { todo!() }
    pub fn target(&self) -> &str { todo!() }
    pub fn source(&self) -> &str { todo!() }
    pub fn translation(&self) -> Vector3 { todo!() }
    pub fn rotation(&self) -> Rotation { todo!() }
    pub fn stamp(&self) -> Stamp { todo!() }
    pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError> { todo!() }
    pub fn inverse(&self) -> Result<Self, TransformError> { todo!() }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CrossTimeTransform { target: String, source: String, tt: Timestamp, st: Timestamp, translation: Vector3, rotation: Rotation }
impl CrossTimeTransform {
    pub fn target(&self) -> &str { todo!() }
    pub fn target_time(&self) -> Timestamp { todo!() }
    pub fn source(&self) -> &str { todo!() }
    pub fn source_time(&self) -> Timestamp { todo!() }
    pub fn translation(&self) -> Vector3 { todo!() }
    pub fn rotation(&self) -> Rotation { todo!() }
}

// ---- time ----
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp(i64);
impl core::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { todo!() }
}
impl Timestamp {
    pub const MAX: Self = Self(i64::MAX);
    pub const fn from_nanos(nanos: i64) -> Self { Self(nanos) }
    pub const fn as_nanos(self) -> i64 { self.0 }
    pub fn from_secs_nanos(secs: i64, nanos: u32) -> Option<Self> { todo!() }
    pub fn as_secs_nanos(self) -> (i64, u32) { todo!() }
    pub fn checked_add(self, d: Duration) -> Option<Self> { todo!() }
    pub fn checked_sub(self, d: Duration) -> Option<Self> { todo!() }
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> { todo!() }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stamp { At(Timestamp), Static }
impl Stamp {
    pub const fn at(self) -> Option<Timestamp> { match self { Self::At(t) => Some(t), Self::Static => None } }
}

// ---- the tree ----
pub struct Registry { _p: hashbrown::HashMap<String, usize> }
impl Registry {
    pub fn with_max_age(max_age: Duration) -> Self { todo!() }
    pub fn unbounded() -> Self { todo!() }

    pub fn insert(&mut self, transform: &Transform) -> Result<(), InsertError> { todo!() }
    pub fn insert_all(&mut self, transforms: &[Transform]) -> Result<(), BatchError> { todo!() }
    pub fn reparent(&mut self, seed: &Transform) -> Result<(), InsertError> { todo!() }
    pub fn remove_frame(&mut self, child: &str) -> bool { todo!() }
    pub fn remove_samples_through(&mut self, cutoff: Timestamp) { todo!() }

    pub fn lookup(&self, target: &str, source: &str, at: Timestamp)
        -> Result<Transform, LookupError> { todo!() }
    pub fn lookup_across(
        &self,
        target: &str, target_time: Timestamp,
        source: &str, source_time: Timestamp,
        fixed: &str,
    ) -> Result<CrossTimeTransform, LookupError> { todo!() }
    pub fn latest_common_time(&self, target: &str, source: &str)
        -> Result<Stamp, LookupError> { todo!() }
}

// ---- errors ----
#[derive(Clone, PartialEq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransformError {
    #[error("non-finite value")]
    NonFinite,
    #[error("rotation norm {norm} is not 1")]
    #[non_exhaustive]
    NonUnitRotation { norm: f64 },
    #[error("empty frame name")]
    EmptyFrameName,
    #[error("frame mismatch: expected {expected:?}, found {found:?}")]
    #[non_exhaustive]
    FrameMismatch { expected: String, found: String },
    #[error("stamp mismatch: {lhs} and {rhs}")]
    #[non_exhaustive]
    StampMismatch { lhs: Timestamp, rhs: Timestamp },
}

#[derive(Clone, PartialEq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InsertError {
    #[error("self edge on {frame:?}")]
    #[non_exhaustive]
    SelfEdge { frame: String },
    #[error("{frame:?} is pinned to {pinned:?}, offered {offered:?}")]
    #[non_exhaustive]
    ParentMismatch { frame: String, pinned: String, offered: String },
    #[error("kind mismatch on {frame:?}: {incoming:?}")]
    #[non_exhaustive]
    KindMismatch { frame: String, incoming: Stamp },
    #[error("cycle: {child:?} under {parent:?}")]
    #[non_exhaustive]
    Cycle { child: String, parent: String },
    #[error("sample at {stamp} for {frame:?} expired, newest {newest}")]
    #[non_exhaustive]
    Expired { frame: String, stamp: Timestamp, newest: Timestamp },
    #[error("unknown frame {frame:?}")]
    #[non_exhaustive]
    UnknownFrame { frame: String },
    #[error("{frame:?} has no parent")]
    #[non_exhaustive]
    NoParent { frame: String },
    #[error("{frame:?} already under {parent:?}")]
    #[non_exhaustive]
    ParentUnchanged { frame: String, parent: String },
}

#[derive(Clone, PartialEq, Debug, thiserror::Error)]
#[error("batch member {index} rejected")]
pub struct BatchError { pub index: usize, #[source] pub error: InsertError }

#[derive(Clone, PartialEq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LookupError {
    #[error("unknown frame {frame:?}")]
    #[non_exhaustive]
    UnknownFrame { frame: String },
    #[error("{a:?} and {b:?} are disconnected")]
    #[non_exhaustive]
    Disconnected { a: String, b: String },
    #[error("{frame:?} holds no data")]
    #[non_exhaustive]
    NoData { frame: String },
    #[error("{frame:?} covers {earliest}..{latest}, requested {requested}")]
    #[non_exhaustive]
    OutOfRange { frame: String, requested: Timestamp, earliest: Timestamp, latest: Timestamp },
    #[error("no common time: {frame:?} covers {earliest}..{latest}, rest ends {latest_possible}")]
    #[non_exhaustive]
    NoCommonTime { frame: String, earliest: Timestamp, latest: Timestamp, latest_possible: Timestamp },
    #[error("non-finite result")]
    NonFinite,
}
