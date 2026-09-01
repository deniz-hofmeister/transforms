//! # Registry Module
//!
//! The `registry` module provides the core functionality for managing transforms between different coordinate frames. It maintains a collection of transforms and offers methods to add, retrieve, and chain these transforms.
//!
//! ## Features
//!
//! - **Static Transforms**: The registry can handle static transforms —
//!   transforms carrying `Stamp::Static`, valid for all time; build them
//!   with `Transform::static_between`.
//! - **Dynamic Transforms**: Supports dynamic transforms with timestamps to handle time-varying transformations.
//! - **Interpolation**: Interpolates between transforms if a requested timestamp lies between two known transforms.
//! - **Coverage Query**: `Registry::latest_common_time` reports the newest
//!   instant a chain can serve, or that no instant is commonly covered.
//! - **Re-parenting**: `Registry::reparent_frame` atomically moves a child
//!   frame under a new parent — at the documented price of the frame's
//!   stored history.
//! - **Automatic Buffer Cleanup**: A registry built with `Registry::with_max_age`
//!   automatically cleans up old dynamic transforms on insert; one built with
//!   `Registry::new` keeps them until `remove_transforms_before` is called.
//!
//! ## Usage
//!
//! The `Registry` struct is the main entry point for interacting with the registry.
//!
//! ## Time type selection
//!
//! `Registry` defaults to `Timestamp` in type position, so
//! `let registry: Registry = Registry::new();` is a `Registry<Timestamp>`.
//! The default does not apply in expression position — there the time type
//! is inferred from usage, so annotate it where the surrounding code does
//! not pin it down.
//!
//! You can use custom timestamps by implementing `time::TimePoint` and then
//! constructing `Registry::<CustomTimestamp>::new()`.
//!
//! With the `std` feature enabled, `std::time::SystemTime` already implements
//! `TimePoint`, so `Registry::<SystemTime>::with_max_age(Duration::from_secs(...))`
//! works out of the box.
//!
//! # Examples
//!
//! ```rust
//! # {
//! use transforms::{
//!     Registry,
//!     geometry::{Quaternion, Transform, Vector3},
//!     time::{Stamp, Timestamp},
//! };
//!
//! # #[cfg(feature = "std")]
//! use core::time::Duration;
//! # #[cfg(feature = "std")]
//! let mut registry = Registry::with_max_age(Duration::from_secs(60));
//! # #[cfg(feature = "std")]
//! let t1 = Timestamp::now();
//!
//! # #[cfg(not(feature = "std"))]
//! # let mut registry = Registry::new();
//! # #[cfg(not(feature = "std"))]
//! # let t1 = Timestamp::zero();
//!
//! let t2 = t1;
//!
//! // Define a transform from frame "a" to frame "b"
//! let t_a_b_1 = Transform::new(
//!     "a",
//!     "b",
//!     Vector3::new(1.0, 0.0, 0.0),
//!     Quaternion::identity(),
//!     Stamp::At(t1),
//! )
//! .unwrap();
//!
//! // For validation
//! let t_a_b_2 = t_a_b_1.clone();
//!
//! // Add the transform to the registry
//! registry.add_transform(t_a_b_1).unwrap();
//!
//! // Retrieve the transform from "a" to "b"
//! let result = registry.get_transform("a", "b", t2);
//! assert!(result.is_ok());
//! assert_eq!(result.unwrap(), t_a_b_2);
//! # }
//! ```

use crate::{
    core::{
        Buffer,
        buffer::{Coverage, GetError},
    },
    geometry::{Localized, Quaternion, Transform, Vector3},
    time::{Stamp, TimePoint, Timestamp},
};
use alloc::{collections::VecDeque, string::String, vec::Vec};
pub use error::RegistryError;
use hashbrown::HashMap;

use core::time::Duration;

mod error;

/// One frame's walk to its tree's root: the edges crossed — each keyed by
/// its child frame, in walk order — and the root frame the walk ends on.
type Ancestry<'a, T> = (Vec<(&'a str, &'a Buffer<T>)>, &'a str);

/// A registry for managing transforms between different frames. It can
/// traverse the parent-child tree and calculate the final transform.
/// It will interpolate between two entries if a time is requested that
/// lies in between.
///
/// The `Registry` struct provides methods to add and retrieve transforms
/// between frames.
///
/// # Examples
///
/// ```
/// use transforms::{
///     Registry,
///     geometry::{Quaternion, Transform, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// # #[cfg(feature = "std")]
/// use core::time::Duration;
/// # #[cfg(feature = "std")]
/// let mut registry = Registry::with_max_age(Duration::from_secs(60));
/// # #[cfg(feature = "std")]
/// let t1 = Timestamp::now();
///
/// # #[cfg(not(feature = "std"))]
/// # let mut registry = Registry::new();
/// # #[cfg(not(feature = "std"))]
/// # let t1 = Timestamp::zero();
///
/// let t2 = t1;
///
/// // Define a transform from frame "a" to frame "b"
/// let t_a_b_1 = Transform::new(
///     "a",
///     "b",
///     Vector3::new(1.0, 0.0, 0.0),
///     Quaternion::identity(),
///     Stamp::At(t1),
/// )
/// .unwrap();
///
/// // For validation
/// let t_a_b_2 = t_a_b_1.clone();
///
/// // Add the transform to the registry
/// registry.add_transform(t_a_b_1).unwrap();
///
/// // Retrieve the transform from "a" to "b"
/// let result = registry.get_transform("a", "b", t2);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), t_a_b_2);
/// ```
#[derive(Debug)]
pub struct Registry<T = Timestamp>
where
    T: TimePoint,
{
    /// Maps a child frame name to the buffer of transforms into that frame.
    data: HashMap<String, Buffer<T>>,
    max_age: Option<Duration>,
}

impl<T> Registry<T>
where
    T: TimePoint,
{
    /// Creates a new `Registry` without automatic cleanup.
    ///
    /// **Nothing bounds this registry.** Two consequences follow, and both
    /// are the caller's to manage:
    ///
    /// - *Memory* grows with the insert rate. Transforms are kept until
    ///   removed manually with [`Registry::remove_transforms_before`], and
    ///   frames until [`Registry::remove_frame`].
    /// - *Interpolation* spans any gap between two retained samples, however
    ///   large. A lookup between samples recorded before and after a pause —
    ///   a stalled publisher, a rebooting robot — interpolates straight
    ///   across it and answers confidently, because both neighbors are still
    ///   stored.
    ///
    /// [`Registry::with_max_age`] bounds both at once: evicting on insert
    /// caps the retained window, and no gap between two retained samples can
    /// then exceed `max_age`. Prefer it unless the retention policy is
    /// genuinely the caller's.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, time::Timestamp};
    ///
    /// let registry = Registry::<Timestamp>::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_age: None,
        }
    }

    /// Creates a new `Registry` with automatic cleanup after `max_age`.
    ///
    /// Dynamic transforms older than `max_age` relative to the latest
    /// inserted timestamp of their child frame are removed automatically on
    /// insert (`Duration::ZERO` retains only the newest sample per frame).
    /// Static transforms never expire.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::time::Duration;
    /// use transforms::{Registry, time::Timestamp};
    ///
    /// let mut registry = Registry::<Timestamp>::with_max_age(Duration::from_secs(60));
    /// ```
    #[must_use]
    pub fn with_max_age(max_age: Duration) -> Self {
        Self {
            data: HashMap::new(),
            max_age: Some(max_age),
        }
    }

    /// Adds a transform to the registry.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NonUnitRotation` or
    /// `RegistryError::NonFiniteValues` if the transform's numbers are
    /// unusable. A transform straight from a constructor cannot fail this —
    /// but one composed with `*`, interpolated, inverted, or read back out of
    /// a lookup was deliberately never re-validated, so re-publishing such a
    /// value is checked here rather than silently corrupting every lookup
    /// that later crosses the frame.
    ///
    /// Returns `RegistryError::StaticDynamicConflict` if the transform's
    /// child frame already holds transforms of the opposite kind: a child
    /// frame is either static (`Stamp::Static`) or dynamic (`Stamp::At`),
    /// never both. The kind is decided by the first transform inserted for
    /// the frame.
    ///
    /// Returns `RegistryError::SelfReferentialFrame` if the transform's
    /// parent and child are the same frame,
    /// `RegistryError::ReparentingNotSupported` if the child frame
    /// already has a different parent — this call never rewires the tree;
    /// that takes an explicit [`Registry::reparent_frame`] — and
    /// `RegistryError::CycleDetected` if the
    /// new relationship would create a cycle in the frame tree.
    ///
    /// Inserting at a timestamp the child frame already stores replaces the
    /// stored transform: last write wins. Re-publishing a sample at the
    /// same stamp is an upsert, not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let mut registry = Registry::<Timestamp>::new();
    /// let transform = Transform::new(
    ///     "base",
    ///     "sensor",
    ///     Vector3::new(1.0, 0.0, 0.0),
    ///     Quaternion::identity(),
    ///     Stamp::At(Timestamp::zero()),
    /// )
    /// .unwrap();
    ///
    /// registry.add_transform(transform).unwrap();
    /// ```
    pub fn add_transform(
        &mut self,
        t: Transform<T>,
    ) -> Result<(), RegistryError<T>> {
        Self::process_add_transform(t, &mut self.data, self.max_age)
    }

    /// Retrieves the transform that maps `source`-frame coordinates into
    /// the `target` frame at the requested timestamp.
    ///
    /// # Direction convention
    ///
    /// The returned transform has `parent == target` and `child == source`:
    /// applying it to data expressed in the `source` frame yields that data
    /// expressed in the `target` frame, matching tf2's
    /// `lookupTransform(target_frame, source_frame, time)` and this
    /// registry's own [`Registry::get_transform_at`]. Mind the order — to
    /// bring lidar points into the map frame, ask for
    /// `get_transform("map", "lidar", t)`; swapping the arguments silently
    /// yields the exact inverse.
    ///
    /// The returned transform always carries the requested timestamp, also
    /// when the chain consists of static transforms. Requesting a frame
    /// relative to itself returns the identity transform.
    ///
    /// Interpolation spans any gap between two stored samples, however
    /// large — a lookup between samples recorded before and after a pause
    /// (say, a robot rebooting) interpolates straight across it. Bounding
    /// data freshness is the caller's responsibility, via `max_age` and
    /// insert cadence.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::UnknownFrame` if a requested frame exists
    /// nowhere in the tree, `RegistryError::NotFoundAt` if the lookup
    /// failed at a frame that exists but could not serve the requested time,
    /// and `RegistryError::Disconnected` if both frames exist but live in
    /// trees that no transform chain connects.
    ///
    /// `NotFoundAt` names the frame the walk stopped at, the timestamp asked
    /// for, and `covered`: that frame's covered time range when it holds
    /// data the request falls outside of, or `None` — no range to carry —
    /// when it holds no data at all, the state a frame drained by
    /// [`Registry::remove_transforms_before`] stays in until something is
    /// inserted into it again. "What is the newest instant this chain
    /// *can* serve" is a first-class query: ask
    /// [`Registry::latest_common_time`] and re-request at its answer —
    /// exact for mid-tree targets too, with no retrying.
    ///
    /// Composing, inverting or interpolating the transforms the walk
    /// collected can itself fail: inverting a half-chain that composed to an
    /// infinite translation reports `RegistryError::NonFiniteValues`,
    /// anything else `RegistryError::TransformError`. Those are the two
    /// lookup failures that name no frame.
    ///
    /// A returned transform is *not* re-validated (see [`Transform`]), and
    /// the check above is the inversion's, not the lookup's: a lookup toward
    /// an ancestor — the documented direction — inverts nothing, so a chain
    /// of extreme magnitudes composes to an infinite translation and comes
    /// back as `Ok`. Whether an overflow is reported therefore depends on
    /// the direction asked for. Call [`Transform::validate`] on a result
    /// whose inputs can reach those magnitudes.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::with_max_age(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// # let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// # let t1 = Timestamp::zero();
    ///
    /// let t2 = t1;
    ///
    /// // Define a transform from frame "a" to frame "b"
    /// let t_a_b_1 = Transform::new(
    ///     "a",
    ///     "b",
    ///     Vector3::new(1.0, 0.0, 0.0),
    ///     Quaternion::identity(),
    ///     Stamp::At(t1),
    /// )
    /// .unwrap();
    /// // For validation
    /// let t_a_b_2 = t_a_b_1.clone();
    ///
    /// registry.add_transform(t_a_b_1).unwrap();
    ///
    /// // "b"-frame data expressed in "a": target "a", source "b"
    /// let result = registry.get_transform("a", "b", t2);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), t_a_b_2);
    /// ```
    pub fn get_transform(
        &self,
        target: &str,
        source: &str,
        timestamp: T,
    ) -> Result<Transform<T>, RegistryError<T>> {
        Self::process_get_transform(target, source, timestamp, &self.data)
    }

    /// Retrieves a transform for a specific value into `target_frame`.
    ///
    /// The source frame and timestamp are taken from the value.
    ///
    /// If the value is already in `target_frame`, this returns an identity
    /// transform with `parent == child == target_frame` and the value's
    /// timestamp (via `get_transform`'s same-frame identity).
    ///
    /// # Errors
    ///
    /// Returns a `RegistryError` if a transform cannot be resolved; the
    /// variants are [`Registry::get_transform`]'s.
    pub fn get_transform_for<U>(
        &self,
        value: &U,
        target_frame: &str,
    ) -> Result<Transform<T>, RegistryError<T>>
    where
        U: Localized<T>,
    {
        self.get_transform(target_frame, value.frame(), value.timestamp())
    }

    /// Retrieves a transform between two frames at different timestamps using a fixed frame.
    ///
    /// This is the "time travel" API that allows you to get the transform from a source frame
    /// at one time to a target frame at a different time. This is useful for scenarios like
    /// tracking an object that was detected on a moving platform (e.g., a conveyor belt) and
    /// getting its current position in a static world frame.
    ///
    /// The algorithm works by:
    /// 1. Computing the transform that expresses `source_frame` in `fixed_frame` at `source_time`
    /// 2. Computing the transform that expresses `target_frame` in `fixed_frame` at `target_time`
    /// 3. Combining the two into the requested transform
    ///
    /// `fixed_frame` is a frame that does not change over time, used as an
    /// intermediate reference point (typically a world or map frame).
    ///
    /// Either endpoint may coincide with `fixed_frame`: that leg is then the
    /// identity, so only the other leg is resolved. When `source_frame` and
    /// `target_frame` both coincide with it, the result is the identity
    /// transform carrying `target_time`.
    ///
    /// # Choosing the fixed frame
    ///
    /// **The caller is responsible for ensuring that `fixed_frame` is actually stationary
    /// between `source_time` and `target_time`.** Passing a frame that moves between the
    /// two timestamps will produce a mathematically meaningless result without any error.
    /// Root frames (e.g., `"world"`, `"map"`) that have no parent are always safe choices.
    ///
    /// # Errors
    ///
    /// Returns a `RegistryError` if any of the required transforms cannot be
    /// found at the specified times; the variants are
    /// [`Registry::get_transform`]'s, reported per leg.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    /// # #[cfg(feature = "std")]
    /// use core::time::Duration;
    ///
    /// # #[cfg(feature = "std")]
    /// let mut registry = Registry::with_max_age(Duration::from_secs(60));
    /// # #[cfg(feature = "std")]
    /// let t1 = Timestamp::now();
    /// # #[cfg(feature = "std")]
    /// let t2 = (t1 + Duration::from_secs(1)).unwrap();
    ///
    /// # #[cfg(not(feature = "std"))]
    /// # let mut registry = Registry::new();
    /// # #[cfg(not(feature = "std"))]
    /// # let t1 = Timestamp::from_nanos(1_000_000_000);
    /// # #[cfg(not(feature = "std"))]
    /// # let t2 = Timestamp::from_nanos(2_000_000_000);
    ///
    /// // Tree: fixed -> a -> b
    ///
    /// // fixed -> a at t1: a is at x=1
    /// registry
    ///     .add_transform(
    ///         Transform::new(
    ///             "fixed",
    ///             "a",
    ///             Vector3::new(1.0, 0.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(t1),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// // fixed -> a at t2: a has moved to x=2
    /// registry
    ///     .add_transform(
    ///         Transform::new(
    ///             "fixed",
    ///             "a",
    ///             Vector3::new(2.0, 0.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(t2),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// // a -> b at t1: b is at y=1 relative to a
    /// registry
    ///     .add_transform(
    ///         Transform::new(
    ///             "a",
    ///             "b",
    ///             Vector3::new(0.0, 1.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(t1),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// // Express b-at-t1 in a-at-t2, using "fixed" as the stationary reference
    /// let result = registry.get_transform_at(
    ///     "a",     // target_frame
    ///     t2,      // target_time
    ///     "b",     // source_frame
    ///     t1,      // source_time
    ///     "fixed", // fixed_frame
    /// );
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn get_transform_at(
        &self,
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
    ) -> Result<Transform<T>, RegistryError<T>> {
        Self::process_get_transform_at(
            target_frame,
            target_time,
            source_frame,
            source_time,
            fixed_frame,
            &self.data,
        )
    }

    /// Returns the newest instant [`Registry::get_transform`] can serve for
    /// this pair of frames, without resolving the transform.
    ///
    /// `Stamp::At(t)` reports the newest instant every hop of the
    /// connecting chain covers: the oldest of the dynamic hops' newest
    /// samples. The answer consults only the hops the chain actually
    /// crosses — edges above the two frames' common ancestor do not
    /// constrain it — so it is exact for mid-tree pairs too. It is also
    /// symmetric in its arguments: both lookup directions serve the same
    /// instants. `Stamp::Static` means the chain puts no bound on time at
    /// all — every hop is static, or `target == source` (the identity,
    /// which serves any instant, matching `get_transform`) — so the caller
    /// picks the instant.
    ///
    /// The intended idiom is this call followed by
    /// [`Registry::get_transform`] at the returned instant. The instant
    /// stays within every hop's covered range only while the registry is
    /// unmodified — with the registry behind a lock, make both calls under
    /// the same read guard. Across separate guards, an interleaved write
    /// that only adds samples leaves the follow-up lookup succeeding,
    /// merely no longer at the newest instant; one that evicts — a
    /// [`Registry::with_max_age`] expiry riding on an insert,
    /// [`Registry::remove_transforms_before`], [`Registry::remove_frame`],
    /// a [`Registry::reparent_frame`] dropping a hop's history —
    /// can remove the instant, and the lookup fails loudly
    /// (`RegistryError::NotFoundAt`, or `UnknownFrame`/`Disconnected` once
    /// a frame is gone). Either way the failure is loud: no interleaving
    /// produces a wrong pose.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::UnknownFrame` if a requested frame exists
    /// nowhere in the tree and `RegistryError::Disconnected` if both exist
    /// but no chain connects them — the same variants, matched by the same
    /// arms, as a failed [`Registry::get_transform`]. (When several faults
    /// coexist the two calls may prioritize differently: a lookup reports
    /// its recorded walk failure over `Disconnected`, this query decides
    /// topology first.)
    ///
    /// Returns `RegistryError::NoCommonTime` when no instant is servable
    /// by every hop of the chain, naming the hop that rules it out: either
    /// the dynamic hops' covered ranges are disjoint (`covered` carries
    /// the named frame's range), or a hop holds no data at all
    /// (`covered: None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let mut registry = Registry::new();
    /// for (parent, child, nanos) in [
    ///     ("map", "odom", 4_000),
    ///     ("map", "odom", 9_000),
    ///     ("odom", "base", 3_000),
    ///     ("odom", "base", 7_000), // this hop lags: nothing newer yet
    /// ] {
    ///     registry
    ///         .add_transform(
    ///             Transform::new(
    ///                 parent,
    ///                 child,
    ///                 Vector3::new(1.0, 0.0, 0.0),
    ///                 Quaternion::identity(),
    ///                 Stamp::At(Timestamp::from_nanos(nanos)),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// // The map ← base chain is bounded by its laggiest hop.
    /// let stamp = registry.latest_common_time("map", "base").unwrap();
    /// assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(7_000)));
    ///
    /// // The returned instant is servable: the two-call idiom.
    /// let latest = registry
    ///     .get_transform("map", "base", stamp.at().unwrap())
    ///     .unwrap();
    /// assert_eq!(latest.timestamp(), stamp);
    /// ```
    pub fn latest_common_time(
        &self,
        target: &str,
        source: &str,
    ) -> Result<Stamp<T>, RegistryError<T>> {
        if target == source {
            return Ok(Stamp::Static);
        }

        let (target_edges, target_root) = Self::ancestry(target, &self.data);
        let (source_edges, source_root) = Self::ancestry(source, &self.data);
        if target_root != source_root {
            // The walks ended in different trees: an unknown frame, or two
            // known but disconnected ones — the same diagnosis order as a
            // failed lookup.
            for frame in [target, source] {
                if !Self::frame_exists(frame, &self.data) {
                    return Err(RegistryError::UnknownFrame(frame.into()));
                }
            }
            return Err(RegistryError::Disconnected {
                target_frame: target.into(),
                source_frame: source.into(),
            });
        }

        // Drop the shared tail above the common ancestor: the connecting
        // chain does not cross those edges, so their coverage must not
        // constrain the answer.
        let shared = target_edges
            .iter()
            .rev()
            .zip(source_edges.iter().rev())
            .take_while(|((target_frame, _), (source_frame, _))| target_frame == source_frame)
            .count();
        let chain = target_edges
            .iter()
            .take(target_edges.len() - shared)
            .chain(source_edges.iter().take(source_edges.len() - shared));

        // The newest instant every hop serves is the *minimum* of the
        // dynamic hops' newest samples — provided every hop's range reaches
        // back to it, which the latest-starting range decides.
        let mut common_end: Option<T> = None;
        let mut latest_start: Option<((T, T), &str)> = None;
        for &(frame, buffer) in chain {
            match buffer.coverage() {
                Coverage::AllTime => {}
                Coverage::Empty => {
                    return Err(RegistryError::NoCommonTime {
                        target_frame: target.into(),
                        source_frame: source.into(),
                        frame: frame.into(),
                        covered: None,
                    });
                }
                Coverage::Range { start, end } => {
                    if common_end.is_none_or(|current| end < current) {
                        common_end = Some(end);
                    }
                    if latest_start.is_none_or(|(covered, _)| start > covered.0) {
                        latest_start = Some(((start, end), frame));
                    }
                }
            }
        }

        match (common_end, latest_start) {
            (Some(end), Some((covered, frame))) if covered.0 > end => {
                Err(RegistryError::NoCommonTime {
                    target_frame: target.into(),
                    source_frame: source.into(),
                    frame: frame.into(),
                    covered: Some(covered),
                })
            }
            (Some(end), _) => Ok(Stamp::At(end)),
            // Every hop is static: the chain puts no bound on time.
            (None, _) => Ok(Stamp::Static),
        }
    }

    /// Removes dynamic transforms older than the given threshold.
    ///
    /// Iterates over all buffers and removes their dynamic entries with a
    /// timestamp lower than the input argument. Static transforms are
    /// preserved: they are valid for all time, so cleaning them up by
    /// timestamp would silently destroy them.
    ///
    /// A frame drained of every transform keeps its entry, and with it the
    /// parent frame and the static-or-dynamic kind pinned by its first
    /// insert. Routine cleanup therefore never unpins a frame's parent or
    /// re-opens its kind, and a lookup on a drained frame
    /// fails with `RegistryError::NotFoundAt` naming that frame rather than
    /// reporting it as unknown. Frame entries are released only by
    /// [`Registry::remove_frame`] — a process that mints transient frame
    /// names must call it when a frame retires.
    pub fn remove_transforms_before(
        &mut self,
        timestamp: T,
    ) {
        for buffer in self.data.values_mut() {
            buffer.remove_before(timestamp);
        }
    }

    /// Removes a child frame and all of its transforms from the registry.
    ///
    /// Returns `true` if the frame existed. This is also the escape hatch
    /// for everything no other call reaches: changing a frame's
    /// static-or-dynamic kind, moving a frame *with* its history
    /// (re-insert the history under the new parent —
    /// [`Registry::reparent_frame`] moves a frame but drops what it
    /// stored), and rebuilding a reversed edge.
    ///
    /// Removing a frame that parents other frames strands those
    /// descendants: they keep their pin to the removed parent, so lookups
    /// that crossed the removed frame fail, diagnosed relative to the
    /// remaining tree — which can name a frame other than the one removed.
    /// That pin is also what reconnects them: to move a whole subtree,
    /// remove its root frame and re-add only that frame under the new
    /// parent — the descendants reconnect through it, history intact.
    pub fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.data.remove(child).is_some()
    }

    /// Moves a child frame under a new parent, dropping its stored history.
    ///
    /// `t.child()` is the frame to move, `t.parent()` the new parent, and
    /// `t` itself becomes the frame's first sample under the new pin — the
    /// frame's buffer is replaced by an empty one seeded with `t`. What
    /// that costs depends on the frame's kind. A *dynamic* frame's
    /// coverage collapses to the seed instant, and the collapse is loud: a
    /// lookup at any other instant fails with
    /// `RegistryError::NotFoundAt` carrying `covered: Some((seed, seed))`,
    /// [`Registry::latest_common_time`] reports it the same way (moving
    /// backwards across this call when the seed is older than the dropped
    /// history — it is not monotonic), and the frame's whole subtree stays
    /// dark until the new edge's next publish grows coverage back. A
    /// *static* frame has no time series to lose: the seed replaces the
    /// single stored pose and keeps answering every instant — including
    /// instants before the move, which the old pose answered differently —
    /// so a static re-parent is retroactive and silent, exactly as an
    /// ordinary static re-publish is. Model an attachment as a dynamic
    /// frame where the past must stay honest, and mind
    /// [`Registry::get_transform_at`], whose `fixed_frame` must not change
    /// over time: a static re-parent between its two legs breaks that
    /// precondition without an error.
    ///
    /// To move a frame *without* losing history, use
    /// [`Registry::remove_frame`] and re-add the history under the new
    /// parent — *re-expressed* in it: looking the frame up toward the new
    /// parent at each wanted instant, before the removal, yields exactly
    /// the transforms to re-add, while relabelling old-parent samples
    /// under the new parent's name is accepted and silently moves the
    /// frame.
    ///
    /// What the move preserves: the frame's static-or-dynamic kind and its
    /// `max_age` expiry policy belong to the frame, not to the seed — a
    /// seed of the opposite kind is rejected
    /// (`RegistryError::StaticDynamicConflict`), so a move that must
    /// change kind is not a re-parent: use [`Registry::remove_frame`],
    /// then [`Registry::add_transform`] — the re-add still crosses the
    /// cycle check, so that route is safe, only non-atomic. For a grasped
    /// object the better shape is usually no kind change at all: keep the
    /// frame dynamic and publish the attachment at rate, so a stopped
    /// stream fails loudly where a static pose would keep answering.
    /// Descendants ride along: their pins are untouched, so the whole
    /// subtree answers under the new parent with every descendant's history
    /// intact — at the instants the moved hop still serves, which for a
    /// dynamic frame is the seed's instant until new samples arrive.
    ///
    /// Cost: like [`Registry::remove_frame`] and
    /// [`Registry::remove_transforms_before`], the call frees everything
    /// it drops, so its latency is proportional to the discarded sample
    /// count — and the replacement buffer exists before the old one is
    /// freed, so the atomic path transiently holds one extra map node
    /// (~1.4 KB) of heap. On a nearly-full embedded heap the non-atomic
    /// recipe has the lower peak.
    ///
    /// A new parent the registry has never seen is accepted, exactly as
    /// [`Registry::add_transform`] accepts one — publish order stays
    /// irrelevant. The flip side: a typo in the new parent's name detaches
    /// the subtree rather than erroring, and if the old parent was a root
    /// whose name appeared nowhere else, later lookups toward it diagnose
    /// `UnknownFrame` naming the *old* parent. Validate frame names at the
    /// boundary where they enter the system. If the old parent survives
    /// elsewhere in the tree, lookups toward it fail as `NotFoundAt`
    /// naming the *moved* frame at instants its collapsed coverage misses,
    /// and as `Disconnected` at instants it covers — ask
    /// [`Registry::latest_common_time`] when the question is connectivity
    /// rather than timing.
    ///
    /// Nothing constrains the seed's timestamp against the history the move
    /// drops. A seed older than the frame's newest stored sample is
    /// accepted, and the frame's coverage moves *backwards* to that
    /// instant — a producer whose messages can reorder in flight (any
    /// transport, a replayed log) can therefore re-parent on a stale
    /// message if this call is wired mechanically to incoming data. A seed
    /// stamped *ahead* of the live stream is worse under a
    /// [`Registry::with_max_age`] registry: the seed becomes the expiry
    /// reference, so every later real sample older than `max_age` relative
    /// to it is evicted on the very insert that added it — accepted,
    /// stored, gone — until the stream catches up to the seed's clock.
    /// Gate the call on the *decision* to re-parent, not on message
    /// arrival, and take the seed's stamp from the frame's own data
    /// stream, not from the clock of whatever decided the move. Two more
    /// rules the registry cannot enforce: publish only samples actually
    /// measured relative to the new parent on the new edge — a backfilled
    /// sample that predates the move interpolates against the seed as if
    /// it were — and when re-arranging more than one hop of a chain, seed
    /// the moves at the same instant, or the chain has no common instant
    /// at all until fresh samples arrive on every moved edge.
    ///
    /// With the registry behind a lock, the natural idiom — decide, then
    /// re-parent, or retry a failed [`Registry::add_transform`] as a
    /// re-parent — is a two-call *write* sequence: make both calls under
    /// one write guard, mirroring the read-pair rule on
    /// [`Registry::latest_common_time`]. Across separate guards an
    /// interleaved writer can steal the move (this call then fails with
    /// `ParentUnchanged`) or move the frame elsewhere first (this call
    /// destroys that writer's fresh seed). Either interleaving is loud or
    /// lost-update, never a wrong pose — but the single-guard form has
    /// neither. The call reports nothing about what it replaced — on
    /// success the old parent's name is gone — so a system that must audit
    /// re-parenting records the decision (frame, old parent, new parent,
    /// seed stamp) on the caller's side, under that same write guard.
    ///
    /// # Errors
    ///
    /// Checked in this order, and every rejection leaves the registry
    /// untouched — the move commits only after all of them pass:
    ///
    /// Returns `RegistryError::UnknownFrame` if `t.child()` exists nowhere
    /// in the tree, and `RegistryError::NoParentToReplace` if it is a known
    /// root — a root gains a parent through an ordinary
    /// [`Registry::add_transform`] insert. (Reversing an existing edge is
    /// reachable through neither call; see `NoParentToReplace`'s
    /// documentation for the rebuild recipe.)
    ///
    /// Returns `RegistryError::ParentUnchanged` if `t.parent()` is the
    /// frame's current parent. Deliberately an error, not an upsert:
    /// resolving every failed insert into a re-parent would wipe the
    /// frame's history once and look correct forever after.
    ///
    /// Returns `RegistryError::CycleDetected` if the move would close a
    /// cycle — the one condition that stays uncheckable however much
    /// state the caller tracks (it needs a view of the whole tree), and
    /// the reason this operation exists as a single atomic call rather
    /// than the destroy-then-fail `remove_frame`-and-re-add sequence.
    ///
    /// Returns what [`Registry::add_transform`] would return for the seed
    /// itself: `RegistryError::NonUnitRotation` or
    /// `RegistryError::NonFiniteValues` for unusable numbers,
    /// `RegistryError::SelfReferentialFrame` if `t.parent() == t.child()`,
    /// and `RegistryError::StaticDynamicConflict` if the seed's stamp kind
    /// does not match the frame's.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::{Stamp, Timestamp},
    /// };
    ///
    /// let mut registry = Registry::<Timestamp>::new();
    ///
    /// // odom starts under map_a...
    /// registry
    ///     .add_transform(
    ///         Transform::new(
    ///             "map_a",
    ///             "odom",
    ///             Vector3::new(1.0, 0.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(Timestamp::from_nanos(1_000)),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// // ...until a map switch moves it under map_b. The transform both
    /// // re-pins the frame and seeds its history under the new parent.
    /// registry
    ///     .reparent_frame(
    ///         Transform::new(
    ///             "map_b",
    ///             "odom",
    ///             Vector3::new(2.0, 0.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(Timestamp::from_nanos(2_000)),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// let moved = registry
    ///     .get_transform("map_b", "odom", Timestamp::from_nanos(2_000))
    ///     .unwrap();
    /// assert_eq!(moved.translation(), Vector3::new(2.0, 0.0, 0.0));
    /// ```
    pub fn reparent_frame(
        &mut self,
        t: Transform<T>,
    ) -> Result<(), RegistryError<T>> {
        let Some(old) = self.data.get(t.child()) else {
            return Err(if Self::frame_exists(t.child(), &self.data) {
                // A known root: it has no parent to replace.
                RegistryError::NoParentToReplace(t.child().into())
            } else {
                RegistryError::UnknownFrame(t.child().into())
            });
        };
        // Defensive: unreachable through `Registry`, which registers a
        // buffer only after its pinning first insert succeeds.
        let Some(current_parent) = old.parent() else {
            return Err(RegistryError::NoParentToReplace(t.child().into()));
        };
        if current_parent == t.parent() {
            return Err(RegistryError::ParentUnchanged(t.child().into()));
        }
        // Checked against the tree with the old edge still in place: a walk
        // upward from the new parent cannot cross the moved frame's old
        // out-edge without first arriving at the frame itself — exactly the
        // case that reports the cycle — so the pre-commit tree gives the
        // same answer as the post-commit one would.
        if Self::creates_cycle(t.child(), t.parent(), &self.data) {
            return Err(RegistryError::CycleDetected);
        }

        // The fresh buffer keeps the frame's kind and expiry policy and pins
        // nothing; the seed's insert runs the numeric, self-reference and
        // kind checks and pins the new parent. Only after every check has
        // passed does the single `HashMap::insert` commit the move.
        let mut fresh = old.empty_like();
        let child: String = t.child().into();
        fresh.insert(t)?;
        self.data.insert(child, fresh);
        Ok(())
    }

    /// Adds a transform to the data buffer.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::CycleDetected` if the new relationship would
    /// close a cycle, and the buffer's own rejection — mapped onto the
    /// matching `RegistryError` variant — for everything the child frame's
    /// buffer refuses.
    fn process_add_transform(
        t: Transform<T>,
        data: &mut HashMap<String, Buffer<T>>,
        max_age: Option<Duration>,
    ) -> Result<(), RegistryError<T>> {
        // A new child->parent relationship changes the tree topology; reject
        // it if it would close a cycle. (Existing buffers have their parent
        // pinned, so occupied inserts cannot.)
        if !data.contains_key(t.child()) && Self::creates_cycle(t.child(), t.parent(), data) {
            return Err(RegistryError::CycleDetected);
        }

        if let Some(buffer) = data.get_mut(t.child()) {
            return buffer.insert(t).map_err(Into::into);
        }

        // New frame: fill the buffer BEFORE registering it in the map, so a
        // failed insert cannot leave an empty, parentless frame behind —
        // which would bypass the cycle check on a later insert of the same
        // child frame. The transform's stamp declares the buffer's kind.
        let mut buffer = match (t.timestamp(), max_age) {
            (Stamp::Static, _) => Buffer::static_edge(),
            (Stamp::At(_), Some(max_age)) => Buffer::dynamic_with_max_age(max_age),
            (Stamp::At(_), None) => Buffer::dynamic(),
        };
        let child: String = t.child().into();
        buffer.insert(t)?;
        data.insert(child, buffer);
        Ok(())
    }

    /// Returns `true` if adding the relationship `child -> parent` would
    /// create a cycle in the frame tree.
    ///
    /// Walks upward from `parent` through the pinned buffer parents. The walk
    /// terminates because the existing tree is acyclic: every edge in the
    /// map passed this check at the moment it entered — `add_transform` for
    /// a new frame, `reparent_frame` for a replaced edge — occupied inserts
    /// cannot change a pinned parent, and `remove_frame` only deletes
    /// edges, so the map is acyclic at every instant.
    fn creates_cycle(
        child: &str,
        parent: &str,
        data: &HashMap<String, Buffer<T>>,
    ) -> bool {
        let mut current = parent;
        while let Some(buffer) = data.get(current) {
            match buffer.parent() {
                Some(next) => {
                    if next == child {
                        return true;
                    }
                    current = next;
                }
                None => return false,
            }
        }
        false
    }

    /// Walks from `frame` to its tree's root through the pinned buffer
    /// parents, collecting the edges crossed. A frame with no buffer is its
    /// own root: the edge list is empty. The walk terminates because the
    /// existing tree is acyclic — every edge in it passed the cycle check
    /// when it entered the map, at insertion or at re-parenting.
    fn ancestry<'a>(
        frame: &'a str,
        data: &'a HashMap<String, Buffer<T>>,
    ) -> Ancestry<'a, T> {
        let mut edges = Vec::new();
        let mut current = frame;
        while let Some((buffer, parent)) = data
            .get(current)
            .and_then(|buffer| buffer.parent().map(|parent| (buffer, parent)))
        {
            edges.push((current, buffer));
            current = parent;
        }
        (edges, current)
    }

    /// Returns `true` if the frame appears anywhere in the tree, as a child
    /// (buffer key) or as a parent. Roots exist only as parents, so a
    /// missing buffer alone does not make a frame unknown.
    fn frame_exists(
        frame: &str,
        data: &HashMap<String, Buffer<T>>,
    ) -> bool {
        data.contains_key(frame) || data.values().any(|buffer| buffer.parent() == Some(frame))
    }

    /// Diagnoses a failed lookup, in order of certainty: a requested frame
    /// that exists nowhere in the tree, then a recorded chain-walk failure
    /// (a known frame that could not serve the requested time, whether it
    /// holds data outside that time or no data at all), and otherwise —
    /// both frames known and both walks clean — the frames live in
    /// disconnected trees. The scans run only on the failure path.
    ///
    /// A walk that stopped on a failed *interpolation* is reported as that
    /// failure rather than as a `NotFoundAt`: the frame does cover the
    /// requested time, so neither `covered` shape would describe it.
    fn diagnose_not_found(
        from: &str,
        to: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
        walk_failure: &mut Option<(String, GetError<T>)>,
    ) -> RegistryError<T> {
        for frame in [from, to] {
            if !Self::frame_exists(frame, data) {
                return RegistryError::UnknownFrame(frame.into());
            }
        }
        let (frame, covered) = match walk_failure.take() {
            Some((frame, GetError::NoTransformAvailable)) => (frame, None),
            Some((frame, GetError::OutOfRange { start, end })) => (frame, Some((start, end))),
            Some((_, GetError::Interpolation(cause))) => return cause.into(),
            None => {
                return RegistryError::Disconnected {
                    target_frame: from.into(),
                    source_frame: to.into(),
                };
            }
        };
        RegistryError::NotFoundAt {
            target_frame: from.into(),
            source_frame: to.into(),
            frame,
            requested: timestamp,
            covered,
        }
    }

    /// Retrieves and computes the transform between two frames at a specific timestamp.
    ///
    /// # Errors
    ///
    /// * `RegistryError::UnknownFrame` - If a requested frame exists nowhere in the tree
    /// * `RegistryError::NotFoundAt` - If the lookup failed at a frame that exists but could not
    ///   serve the requested time, either because the request falls outside the data it holds or
    ///   because it holds none
    /// * `RegistryError::Disconnected` - If both frames exist but no chain connects them
    /// * `RegistryError::NonFiniteValues` or `RegistryError::TransformError` - If an operation on
    ///   the resolved chain failed
    fn process_get_transform(
        target: &str,
        source: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<Transform<T>, RegistryError<T>> {
        // A frame relative to itself is the identity, regardless of whether
        // the frame is known: the answer holds either way, and it keeps
        // same-frame queries consistent with `get_transform_for`.
        if target == source {
            return Ok(Transform::unvalidated(
                target.into(),
                source.into(),
                Vector3::zero(),
                Quaternion::identity(),
                Stamp::At(timestamp),
            ));
        }

        let reached = |chain: &VecDeque<Transform<T>>, goal: &str| {
            chain.back().is_some_and(|tf| tf.parent() == goal)
        };

        let mut walk_failure = None;
        let target_chain =
            Self::get_transform_chain(target, source, timestamp, data, &mut walk_failure);

        let result = match target_chain {
            // `source` is an ancestor of `target`: the target-side chain
            // spans the whole path, no source-side walk is needed.
            Some(target_chain) if reached(&target_chain, source) => {
                Self::combine_transforms(target_chain, VecDeque::new())
            }
            target_chain => match (
                target_chain,
                Self::get_transform_chain(source, target, timestamp, data, &mut walk_failure),
            ) {
                // `target` is an ancestor of `source`: the source-side chain
                // spans the whole path by itself.
                (_, Some(source_chain)) if reached(&source_chain, target) => {
                    Self::combine_transforms(VecDeque::new(), source_chain)
                }
                // Both chains ran to the root: drop the shared suffix above
                // the common parent and combine the remainders.
                (Some(mut target_chain), Some(mut source_chain)) => {
                    Self::truncate_at_common_parent(&mut target_chain, &mut source_chain);
                    // The two walks must meet at a common parent; otherwise
                    // they stopped in different subtrees — an unknown frame,
                    // a mid-chain timestamp gap, or disconnected trees — and
                    // no transform exists at this time. Diagnose the failure
                    // instead of letting the junction fail composition with
                    // a misleading IncompatibleFrames.
                    let connected = match (target_chain.back(), source_chain.back()) {
                        (Some(target_top), Some(source_top)) => {
                            target_top.parent() == source_top.parent()
                        }
                        _ => false,
                    };
                    if connected {
                        Self::combine_transforms(target_chain, source_chain)
                    } else {
                        Some(Err(Self::diagnose_not_found(
                            target,
                            source,
                            timestamp,
                            data,
                            &mut walk_failure,
                        )))
                    }
                }
                (Some(target_chain), None) => {
                    Self::combine_transforms(target_chain, VecDeque::new())
                }
                (None, Some(source_chain)) => {
                    Self::combine_transforms(VecDeque::new(), source_chain)
                }
                (None, None) => Some(Err(Self::diagnose_not_found(
                    target,
                    source,
                    timestamp,
                    data,
                    &mut walk_failure,
                ))),
            },
        }
        // Both walks empty without a recorded failure cannot happen today
        // (every call site passes at least one non-empty chain), but if it
        // ever does, it is a failed lookup and diagnosed as such.
        .unwrap_or_else(|| {
            Err(Self::diagnose_not_found(
                target,
                source,
                timestamp,
                data,
                &mut walk_failure,
            ))
        })?;

        // A chain can resolve without ever reaching the requested frame, for
        // example when `source` does not exist in the tree and the walk
        // stopped at the root instead. Verify the combined transform answers
        // the exact question asked; otherwise report it as not found.
        if result.parent() != target || result.child() != source {
            return Err(Self::diagnose_not_found(
                target,
                source,
                timestamp,
                data,
                &mut walk_failure,
            ));
        }

        // The result answers "where is `source` relative to `target` at the
        // requested time", so it carries the requested timestamp — also for
        // chains of static transforms, which are themselves stamped
        // `Stamp::Static`.
        Ok(result.restamped(Stamp::At(timestamp)))
    }

    /// Retrieves a transform between two frames at different timestamps using a fixed frame.
    ///
    /// This implements "time travel" by:
    /// 1. Getting the transform that expresses `source_frame` in `fixed_frame` at `source_time`
    /// 2. Getting the transform that expresses `target_frame` in `fixed_frame` at `target_time`
    /// 3. Combining the two into the requested transform
    ///
    /// `fixed_frame` must be a frame that doesn't change over time (e.g., "world").
    ///
    /// # Errors
    ///
    /// * `RegistryError::UnknownFrame` - If a requested frame exists nowhere in the tree
    /// * `RegistryError::NotFoundAt` - If a leg failed at a frame that exists but could not
    ///   serve the requested time, either because the request falls outside the data it holds or
    ///   because it holds none
    /// * `RegistryError::Disconnected` - If a leg's frames exist but no chain connects them
    /// * `RegistryError::NonFiniteValues` or `RegistryError::TransformError` - If composing the
    ///   two legs failed
    fn process_get_transform_at(
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
        data: &HashMap<String, Buffer<T>>,
    ) -> Result<Transform<T>, RegistryError<T>> {
        // Following tf2's algorithm:
        // 1. Get transform expressing source_frame in fixed_frame at source_time
        // 2. Get transform expressing target_frame in fixed_frame at target_time
        // 3. Compute: T_target_to_fixed.inverse() * T_source_to_fixed
        //
        // process_get_transform(parent, child) returns "child expressed in
        // parent", so process_get_transform(fixed, source) returns "source
        // expressed in fixed".

        // An endpoint coinciding with the fixed frame makes its leg the
        // identity, so no composition is needed; short-circuit those cases.
        // Multiplying with an identity carrying parent == child ==
        // fixed_frame is not an option: `Mul` rejects self-referential
        // operands as `SameFrameMultiplication`.
        if source_frame == fixed_frame && target_frame == fixed_frame {
            return Ok(Transform::unvalidated(
                target_frame.into(),
                source_frame.into(),
                Vector3::zero(),
                Quaternion::identity(),
                Stamp::At(target_time),
            ));
        }
        if source_frame == fixed_frame {
            // The answer is the target leg alone, inverted.
            let result = Self::process_get_transform(fixed_frame, target_frame, target_time, data)?
                .inverse()?;
            return Ok(result.restamped(Stamp::At(target_time)));
        }
        if target_frame == fixed_frame {
            // The answer is the source leg alone.
            let result = Self::process_get_transform(fixed_frame, source_frame, source_time, data)?;
            return Ok(result.restamped(Stamp::At(target_time)));
        }

        // Step 1: Get transform expressing source_frame in fixed_frame at source_time
        let source_to_fixed =
            Self::process_get_transform(fixed_frame, source_frame, source_time, data)?;

        // Step 2: Get transform expressing target_frame in fixed_frame at target_time
        let target_to_fixed =
            Self::process_get_transform(fixed_frame, target_frame, target_time, data)?;

        // The two legs are deliberately resolved at different times — that
        // is the point of the time-travel lookup — so they compose through
        // the private time-agnostic path rather than `Mul`, whose timestamp
        // check exists to catch *accidental* cross-time composition.
        let result = target_to_fixed
            .inverse()?
            .compose_ignoring_time(source_to_fixed)?;

        // The result carries the target time as per the API contract.
        Ok(result.restamped(Stamp::At(target_time)))
    }

    /// Constructs a chain of transforms from a starting frame to a target
    /// frame at a given timestamp, or `None` if the walk yields no
    /// transforms. Diagnosing the reason is the caller's job
    /// (`diagnose_not_found`).
    ///
    /// A buffer lookup failing along the way ends the walk; the first such
    /// failure across all walks of one lookup is recorded in `walk_failure`
    /// so the caller can report it if the lookup fails as a whole.
    fn get_transform_chain(
        from: &str,
        to: &str,
        timestamp: T,
        data: &HashMap<String, Buffer<T>>,
        walk_failure: &mut Option<(String, GetError<T>)>,
    ) -> Option<VecDeque<Transform<T>>> {
        let mut transforms = VecDeque::new();
        let mut current_frame: String = from.into();

        // The frame tree is acyclic by construction (cycles are rejected
        // whenever an edge enters the map — at insertion and at
        // re-parenting), so the walk visits every frame at most once and
        // terminates at a root.
        while let Some(frame_buffer) = data.get(&current_frame) {
            match frame_buffer.get(timestamp) {
                Ok(tf) => {
                    current_frame.clear();
                    current_frame.push_str(tf.parent());
                    transforms.push_back(tf);
                }
                Err(source) => {
                    if walk_failure.is_none() {
                        *walk_failure = Some((current_frame.clone(), source));
                    }
                    break;
                }
            }

            // Reaching `to` completes the chain; walking on to the root would
            // only add work that truncate_at_common_parent discards again.
            if current_frame == to {
                break;
            }
        }

        if transforms.is_empty() {
            None
        } else {
            Some(transforms)
        }
    }

    /// Truncates two transform chains at their common parent frame to optimize the transformation computation.
    fn truncate_at_common_parent(
        from_chain: &mut VecDeque<Transform<T>>,
        to_chain: &mut VecDeque<Transform<T>>,
    ) {
        let mut start_idx = 0;
        for (i, j) in from_chain.iter().rev().zip(to_chain.iter().rev()) {
            if i == j {
                start_idx += 1;
            } else {
                break;
            }
        }

        // Truncate the chains at the common parent frame
        from_chain.truncate(from_chain.len() - start_idx);
        to_chain.truncate(to_chain.len() - start_idx);
    }

    /// Combines the two half-chains of a lookup into the transform that
    /// expresses `source` in `target`.
    ///
    /// Both arguments are walks *upward* from a frame toward the common
    /// ancestor, so each composes in its natural order into "that frame
    /// expressed in the ancestor" without a single inversion. Only the target
    /// half is then inverted, giving
    /// `t_target_common * t_common_source = t_target_source`: at most one
    /// inversion per lookup, against one per hop plus one at the end for the
    /// pass this replaced, which reversed and inverted the source half
    /// element by element and inverted the combined result again. A lookup
    /// toward an ancestor (the documented direction,
    /// `get_transform("map", "lidar", t)`) resolves entirely from the source
    /// half and inverts nothing, so a single-hop lookup at a stored timestamp
    /// returns that stored transform bit for bit.
    ///
    /// Returns `None` when both chains are empty — there is nothing to
    /// combine, and the caller reports the lookup failure through
    /// `diagnose_not_found`.
    ///
    /// # Errors
    ///
    /// * The `RegistryError` a failed transform operation converts into
    fn combine_transforms(
        target_chain: VecDeque<Transform<T>>,
        source_chain: VecDeque<Transform<T>>,
    ) -> Option<Result<Transform<T>, RegistryError<T>>> {
        let target = match Self::compose_chain(target_chain) {
            Ok(composed) => composed,
            Err(e) => return Some(Err(e)),
        };
        let source = match Self::compose_chain(source_chain) {
            Ok(composed) => composed,
            Err(e) => return Some(Err(e)),
        };

        match (target, source) {
            (None, None) => None,
            (Some(target), None) => Some(target.inverse().map_err(Into::into)),
            (None, Some(source)) => Some(Ok(source)),
            (Some(target), Some(source)) => Some(
                target
                    .inverse()
                    .and_then(|inverted| inverted * source)
                    .map_err(Into::into),
            ),
        }
    }

    /// Composes a chain walked upward from a frame into the single transform
    /// expressing that frame in the chain's topmost parent, or `None` for an
    /// empty chain.
    ///
    /// Each element's child is the previous element's parent, so folding from
    /// the front composes them in the order the walk produced them.
    ///
    /// # Errors
    ///
    /// * The `RegistryError` a failed composition converts into
    fn compose_chain(
        chain: VecDeque<Transform<T>>
    ) -> Result<Option<Transform<T>>, RegistryError<T>> {
        let mut iter = chain.into_iter();
        let Some(mut composed) = iter.next() else {
            return Ok(None);
        };

        for transform in iter {
            composed = (transform * composed)?;
        }

        Ok(Some(composed))
    }
}

impl<T> Default for Registry<T>
where
    T: TimePoint,
{
    /// Equivalent to [`Registry::new`], including its unbounded retention and
    /// unbounded interpolation gap — read that constructor's documentation
    /// before taking the default over [`Registry::with_max_age`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
