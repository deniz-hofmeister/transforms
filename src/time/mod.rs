//! Time abstractions used by the transform core.
//!
//! `TimePoint` is the trait that any timestamp type must implement.
//! `Timestamp` is the default concrete type provided by this crate.
//!
//! If you need a custom clock, implement `TimePoint` for your own `Copy + Ord`
//! type and use `Registry<YourTimeType>`.
//! With `std`, `std::time::SystemTime` already implements `TimePoint`, so
//! `Registry::<SystemTime>` is ready to use.

pub mod error;
pub mod timestamp;
pub mod traits;
pub use error::TimeError;
#[allow(deprecated)]
pub use error::TimestampError;
pub use timestamp::Timestamp;
pub use traits::TimePoint;

#[cfg(feature = "std")]
mod system_time;
