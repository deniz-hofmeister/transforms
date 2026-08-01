use alloc::string::String;

use thiserror::Error;

use crate::errors::TransformError;

/// Error type for buffer insertion and retrieval.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BufferError {
    /// The buffer holds no transforms at all. A non-empty buffer that
    /// cannot serve a requested timestamp reports
    /// `TransformError::TimestampOutOfRange` (wrapped in
    /// [`TransformError`](Self::TransformError)) instead, carrying the
    /// covered range.
    #[error("the buffer holds no transforms")]
    NoTransformAvailable,

    /// The transform's kind (static or dynamic) does not match the buffer's
    /// declared kind, fixed at construction (`Buffer::static_edge` vs.
    /// `Buffer::dynamic`); a child frame is either static or dynamic, never
    /// both. Fires even on an empty buffer — the kind is a property of the
    /// buffer, not of what it currently stores.
    #[error("cannot mix static and dynamic transforms for the same child frame")]
    StaticDynamicConflict,

    /// The transform's parent and child are the same frame.
    #[error("a frame cannot be its own parent")]
    SelfReferentialFrame,

    /// The child frame already has a different parent. Re-parenting is not
    /// supported; remove the frame first (`Registry::remove_frame`) and
    /// re-add it under its new parent.
    #[error("re-parenting is not supported (the child frame's parent is {0})")]
    ReparentingNotSupported(String),

    /// The buffer already holds transforms of a different child frame. A
    /// buffer stores the history of exactly one parent-child pair, pinned by
    /// the first insert; accepting another child would silently overwrite or
    /// corrupt the stored data.
    #[error("the buffer already stores a different child frame ({0})")]
    ChildFrameMismatch(String),

    /// Inserting the transform would create a cycle in the frame tree.
    #[error("inserting the transform would create a cycle in the frame tree")]
    CycleDetected,

    /// A transform operation failed during retrieval.
    #[error("transform error: {0}")]
    TransformError(#[from] TransformError),
}
