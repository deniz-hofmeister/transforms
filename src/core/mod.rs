//! Core storage and lookup: the transform Registry and its per-frame Buffers.

pub(crate) mod buffer;
pub(crate) mod registry;

pub use buffer::Buffer;
pub use registry::Registry;
