use alloc::string::String;

use crate::errors::TransformError;

/// Why a buffer refused a transform.
///
/// Crate-internal, and deliberately split from [`GetError`]: an insert
/// cannot fail for want of data and a read cannot fail on a frame pin, so
/// one enum for both would force an unreachable arm on every conversion.
/// `Registry` maps each cause onto the matching public
/// [`RegistryError`](crate::errors::RegistryError) variant.
#[derive(Debug)]
pub(crate) enum InsertError {
    /// The transform's numbers are unusable: a non-unit rotation or a
    /// non-finite component (see [`Transform::validate`](crate::Transform::validate)).
    Invalid(TransformError),

    /// The transform's kind (static or dynamic) does not match the kind the
    /// buffer was constructed with.
    StaticDynamicConflict,

    /// The transform's parent and child are the same frame.
    SelfReferentialFrame,

    /// The transform's parent differs from the one pinned by the first
    /// insert; carries the pinned parent.
    ReparentingNotSupported(String),

    /// The transform's child differs from the one pinned by the first
    /// insert, carrying both. Unreachable through `Registry`, which keys
    /// its buffers by child frame — this is the check that keeps the pin
    /// true if that ever stops holding.
    ChildFrameMismatch {
        /// The child frame the buffer stores.
        pinned: String,
        /// The child frame the rejected transform carries.
        found: String,
    },
}

/// Why a buffer could not serve a requested timestamp.
///
/// Crate-internal; `Registry` turns the first two into
/// [`RegistryError::NotFoundAt`](crate::errors::RegistryError::NotFoundAt),
/// which is why the covered range stays in the timestamp type instead of
/// being converted to seconds here.
#[derive(Debug)]
pub(crate) enum GetError<T> {
    /// The buffer holds no transforms at all.
    NoTransformAvailable,

    /// The buffer holds transforms, but none covering the request; carries
    /// the covered range. There is no extrapolation.
    OutOfRange {
        /// The earliest stored sample.
        start: T,
        /// The latest stored sample.
        end: T,
    },

    /// Interpolating between the two neighboring samples failed.
    Interpolation(TransformError),
}
