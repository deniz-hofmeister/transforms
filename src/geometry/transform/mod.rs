//! Labelled rigid-body transforms.

use super::{Isometry, Rotation, TransformError, Vector3};
use crate::time::Stamp;
use alloc::string::String;

/// Maps `source`-frame coordinates into the `target` frame at a stamp:
/// `p_target = R * p_source + t`.
///
/// Every value is valid: the translation is finite, the rotation is a
/// [`Rotation`], and neither frame name is empty. [`Transform::new`],
/// decoding, [`compose`](Self::compose), [`inverse`](Self::inverse) and
/// every [`Registry`](crate::Registry) lookup establish that or return an
/// error, so a transform never needs to be validated by its user. The
/// fields are private; read them through the accessors.
///
/// Inserted into a [`Registry`](crate::Registry), a transform is the edge
/// whose parent is [`target`](Self::target) and whose child is
/// [`source`](Self::source).
///
/// The two names may be equal. A pose error
/// `estimate.inverse()?.compose(&truth)?` is an ordinary `base`-to-`base`
/// transform that is not the identity, and no check tells it from a
/// hand-built `a`-to-`a` transform with an offset, which is a mistake.
/// [`Registry::insert`](crate::Registry::insert) rejects every transform
/// whose names are equal.
///
/// Equality is exact. With `serde`, a transform encodes as `target`,
/// `source`, `translation`, `rotation`, `stamp`, in that order, and decodes
/// through [`Transform::new`] with the same checks; unknown fields are an
/// error. Decoding an encoded transform returns an equal one under a codec
/// that preserves every `f64`; JSON text needs a parser that rounds
/// correctly.
///
/// # Applying a transform to a point
///
/// Rotate, then translate. The crate does not do this for you, and the
/// variants that are easy to write by hand (translating first, dropping the
/// translation, applying the transform in the wrong direction) all return
/// an ordinary vector that no check can tell from the right one.
///
/// ```
/// use transforms::{Rotation, Stamp, Timestamp, Transform, Vector3};
///
/// // `sensor` in `map`: rolled, pitched and yawed by 30, 45 and 60 degrees.
/// let tf = Transform::new(
///     "map",
///     "sensor",
///     Vector3::new(-1.0, 0.5, 0.25),
///     Rotation::from_wxyz(
///         0.723_317_411_364_711_8,
///         0.391_903_837_329_119_9,
///         0.200_562_121_146_575_12,
///         0.531_975_695_182_166_8,
///     )
///     .unwrap(),
///     Stamp::At(Timestamp::from_nanos(0)),
/// )
/// .unwrap();
///
/// // A point measured in `sensor` ...
/// let p = Vector3::new(0.3, -0.7, 1.1);
///
/// // ... expressed in `map`.
/// let r = tf.rotation().rotate(p);
/// let t = tf.translation();
/// let p_target = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
///
/// assert!((p_target.x - 0.312_544_181_470_241).abs() < 1e-12);
/// assert!((p_target.y - 0.300_345_740_105_364_36).abs() < 1e-12);
/// assert!((p_target.z - 0.415_426_564_355_733_33).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    target: String,
    source: String,
    isometry: Isometry,
    stamp: Stamp,
}

impl Transform {
    /// Builds a transform that maps `source`-frame coordinates into the
    /// `target` frame.
    ///
    /// `stamp` says when it holds: `Stamp::At(t)` for a sample of a moving
    /// relationship, `Stamp::Static` for a fixed one such as a sensor
    /// mount. Borrowed names are copied; pass a `String` to move it in.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::EmptyFrameName`] if either name is empty,
    /// and otherwise [`TransformError::NonFinite`] if a translation
    /// component is NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Rotation, Stamp, Timestamp, Transform, TransformError, Vector3};
    ///
    /// let mount = Transform::new(
    ///     "base",
    ///     "camera",
    ///     Vector3::new(0.1, 0.0, 0.5),
    ///     Rotation::IDENTITY,
    ///     Stamp::Static,
    /// )
    /// .unwrap();
    /// assert_eq!(mount.target(), "base");
    /// assert_eq!(mount.source(), "camera");
    ///
    /// let unnamed = Transform::new(
    ///     "base",
    ///     "",
    ///     Vector3::new(0.0, 0.0, 0.0),
    ///     Rotation::IDENTITY,
    ///     Stamp::At(Timestamp::from_nanos(0)),
    /// );
    /// assert_eq!(unnamed, Err(TransformError::EmptyFrameName));
    /// ```
    pub fn new(
        target: impl Into<String>,
        source: impl Into<String>,
        translation: Vector3,
        rotation: Rotation,
        stamp: Stamp,
    ) -> Result<Self, TransformError> {
        let target = target.into();
        if target.is_empty() {
            return Err(TransformError::EmptyFrameName);
        }
        let source = source.into();
        if source.is_empty() {
            return Err(TransformError::EmptyFrameName);
        }
        Ok(Self {
            target,
            source,
            isometry: Isometry::new(translation, rotation)?,
            stamp,
        })
    }

    /// Labels an isometry the registry resolved.
    ///
    /// The names are the ones a query was answered for, and the registry
    /// knows no empty name, so the result is valid without a check.
    pub(crate) fn from_parts(
        target: String,
        source: String,
        isometry: Isometry,
        stamp: Stamp,
    ) -> Self {
        Self {
            target,
            source,
            isometry,
            stamp,
        }
    }

    /// The frame the transform maps coordinates into.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The frame the transform maps coordinates from.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Where the source frame's origin sits in the target frame.
    #[must_use]
    pub fn translation(&self) -> Vector3 {
        self.isometry.translation()
    }

    /// How the source frame is oriented in the target frame.
    #[must_use]
    pub fn rotation(&self) -> Rotation {
        self.isometry.rotation()
    }

    /// When the transform holds: at one instant, or for all time.
    #[must_use]
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }

    /// The numbers without the frames and stamp.
    pub(crate) fn isometry(&self) -> Isometry {
        self.isometry
    }

    /// Composes two transforms: with `self` mapping `b` into `a` and `rhs`
    /// mapping `c` into `b`, the result maps `c` into `a`.
    ///
    /// Two dynamic operands must hold at the same instant. A static operand
    /// holds at every instant and takes the other's stamp; two static
    /// operands give a static result.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::FrameMismatch`] unless
    /// `self.source() == rhs.target()`, then
    /// [`TransformError::StampMismatch`] for two dynamic operands at
    /// different instants. Returns [`TransformError::NonFinite`] if the
    /// composed translation overflows, and
    /// [`TransformError::NonUnitRotation`] if the composed rotation cannot
    /// be brought back to unit length, which no input is known to reach.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Rotation, Stamp, Transform, Vector3};
    ///
    /// let edge = |target, source, x| {
    ///     Transform::new(
    ///         target,
    ///         source,
    ///         Vector3::new(x, 0.0, 0.0),
    ///         Rotation::IDENTITY,
    ///         Stamp::Static,
    ///     )
    ///     .unwrap()
    /// };
    /// let map_from_base = edge("map", "base", 1.0);
    /// let base_from_lidar = edge("base", "lidar", 2.0);
    ///
    /// let map_from_lidar = map_from_base.compose(&base_from_lidar).unwrap();
    /// assert_eq!(map_from_lidar.target(), "map");
    /// assert_eq!(map_from_lidar.source(), "lidar");
    /// assert_eq!(map_from_lidar.translation(), Vector3::new(3.0, 0.0, 0.0));
    ///
    /// // The other order does not chain.
    /// assert!(base_from_lidar.compose(&map_from_base).is_err());
    /// ```
    pub fn compose(
        &self,
        rhs: &Self,
    ) -> Result<Self, TransformError> {
        if self.source != rhs.target {
            return Err(TransformError::FrameMismatch {
                expected: self.source.clone(),
                found: rhs.target.clone(),
            });
        }
        let stamp = match (self.stamp, rhs.stamp) {
            (Stamp::Static, stamp) | (stamp, Stamp::Static) => stamp,
            (Stamp::At(lhs), Stamp::At(rhs)) => {
                if lhs != rhs {
                    return Err(TransformError::StampMismatch { lhs, rhs });
                }
                Stamp::At(lhs)
            }
        };
        Ok(Self {
            target: self.target.clone(),
            source: rhs.source.clone(),
            isometry: self.isometry.compose(rhs.isometry)?,
            stamp,
        })
    }

    /// The same relationship read the other way round: the frames swapped,
    /// the stamp kept.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::NonFinite`] if the inverted translation
    /// overflows, which translations near `f64::MAX` reach.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Rotation, Stamp, Transform, Vector3};
    ///
    /// let map_from_base = Transform::new(
    ///     "map",
    ///     "base",
    ///     Vector3::new(1.0, 2.0, 3.0),
    ///     Rotation::IDENTITY,
    ///     Stamp::Static,
    /// )
    /// .unwrap();
    ///
    /// let base_from_map = map_from_base.inverse().unwrap();
    /// assert_eq!(base_from_map.target(), "base");
    /// assert_eq!(base_from_map.source(), "map");
    /// assert_eq!(base_from_map.translation(), Vector3::new(-1.0, -2.0, -3.0));
    /// ```
    pub fn inverse(&self) -> Result<Self, TransformError> {
        Ok(Self {
            target: self.source.clone(),
            source: self.target.clone(),
            isometry: self.isometry.inverse()?,
            stamp: self.stamp,
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Transform {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        // Names first: a 2.x payload leads with a float, whose low bytes a
        // leading name consumes, so every later float is misaligned and the
        // rotation check rejects nearly every such decode. Not all: bytes
        // that happen to form a valid transform still decode.
        let mut transform = serializer.serialize_struct("Transform", 5)?;
        transform.serialize_field("target", &self.target)?;
        transform.serialize_field("source", &self.source)?;
        transform.serialize_field("translation", &self.isometry.translation())?;
        transform.serialize_field("rotation", &self.isometry.rotation())?;
        transform.serialize_field("stamp", &self.stamp)?;
        transform.end()
    }
}

/// The record serde reads a [`Transform`] from: the five fields, in the
/// order they are written, before [`Transform::new`] has checked them.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(rename = "Transform", deny_unknown_fields)]
struct TransformRecord {
    target: String,
    source: String,
    translation: Vector3,
    rotation: Rotation,
    stamp: Stamp,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let record = TransformRecord::deserialize(deserializer)?;
        Self::new(
            record.target,
            record.source,
            record.translation,
            record.rotation,
            record.stamp,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
