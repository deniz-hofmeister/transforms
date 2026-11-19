//! Frame-tree storage and transform lookup.

use crate::{
    geometry::{CrossTimeTransform, Isometry, NumericError, Transform},
    time::{Stamp, Timestamp},
};
use core::time::Duration;
pub use error::{BatchError, InsertError, LookupError};
use frames::{FrameId, Frames};
use hashbrown::HashMap;
use history::{Gap, History, SampleError};

mod error;
mod frames;
mod history;

/// What the insert checks compare a transform against: the parent and kind
/// its child frame is pinned to, and the newest instant the frame holds.
#[derive(Clone, Copy)]
struct Pin<'a> {
    parent: &'a str,
    is_static: bool,
    /// The newest stored instant of a dynamic frame that holds samples.
    newest: Option<Timestamp>,
}

/// The pins a batch has set so far, by child frame: the edges it creates,
/// and the running newest instant of every frame it adds samples to. A
/// batch is checked against these before the stored frames, so each member
/// meets the state the members before it would have left.
type Staged<'a> = HashMap<&'a str, Pin<'a>>;

/// Why a resolution failed, before the public error is built: missing
/// coverage on an edge, or a numerical failure.
enum Fault {
    Gap { edge: FrameId, gap: Gap },
    Numeric(NumericError),
}

/// A failed resolution, diagnosed: coverage decides before numerics, and a
/// cross-time lookup needs to tell the two apart across its legs.
enum Failure {
    Coverage(LookupError),
    Numeric(NumericError),
}

impl From<Failure> for LookupError {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Coverage(error) => error,
            Failure::Numeric(error) => error.into(),
        }
    }
}

/// A frame tree with timestamped transform history.
///
/// Each inserted [`Transform`] is an edge of the tree: its
/// [`target`](Transform::target) is the parent and its
/// [`source`](Transform::source) the child. A child frame's first insert
/// pins its parent and its kind, static or dynamic. Lookups compose the
/// edges that connect two frames, interpolating dynamic edges between
/// their stored samples; there is no extrapolation.
///
/// A name is known to the registry while a stored edge names it, as child
/// or as parent. Every name in a query must be known.
///
/// Build one with [`Registry::with_max_age`] to keep a time window per
/// frame, or [`Registry::unbounded`] to keep everything until it is
/// removed by hand.
///
/// `Registry` is `Send` and `Sync`. An external `RwLock` permits concurrent
/// readers; writers need exclusive access. Keep queries that depend on each
/// other, such as [`latest_common_time`](Self::latest_common_time) followed
/// by [`lookup`](Self::lookup), under one guard.
///
/// # Examples
///
/// ```
/// use core::time::Duration;
/// use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
///
/// let mut registry = Registry::with_max_age(Duration::from_secs(10));
/// let t = Timestamp::from_nanos(1_000_000_000);
///
/// // A moving base, and a lidar mounted on it.
/// registry
///     .insert(
///         &Transform::new(
///             "map",
///             "base",
///             Vector3::new(1.0, 0.0, 0.0),
///             Rotation::IDENTITY,
///             Stamp::At(t),
///         )
///         .unwrap(),
///     )
///     .unwrap();
/// registry
///     .insert(
///         &Transform::new(
///             "base",
///             "lidar",
///             Vector3::new(0.0, 0.0, 0.5),
///             Rotation::IDENTITY,
///             Stamp::Static,
///         )
///         .unwrap(),
///     )
///     .unwrap();
///
/// // Lidar coordinates expressed in the map.
/// let map_from_lidar = registry.lookup("map", "lidar", t).unwrap();
/// assert_eq!(map_from_lidar.translation(), Vector3::new(1.0, 0.0, 0.5));
/// ```
#[derive(Debug)]
pub struct Registry {
    frames: Frames,
    max_age: Option<Duration>,
}

impl Registry {
    /// Creates a registry whose dynamic frames each keep a window of
    /// `max_age` behind their newest stamp.
    ///
    /// An insert removes the frame's samples older than its newest stamp
    /// minus `max_age`. An insert that is itself older than that boundary
    /// stores nothing and fails with [`InsertError::Expired`]. A sample
    /// exactly on the boundary is kept, and `Duration::ZERO` keeps only the
    /// newest sample and rejects every older one. Where the subtraction is
    /// not representable, nothing is removed and nothing is rejected. Wall
    /// clock time is never consulted, and static transforms never expire.
    ///
    /// The window follows the stamps inserted, and an insert never moves a
    /// frame's newest stamp back. A sample stamped ahead of its frame's
    /// stream is accepted and takes the window with it: the stream's next
    /// samples fail with `Expired` until the stream reaches the boundary. A
    /// stream that steps back by less than `max_age` is stored without an
    /// error beside the samples ahead of it, and lookups interpolate
    /// between the two. Whoever owns the clock must detect such a step; see
    /// [`InsertError::Expired`] for discarding a frame's newest stamp.
    ///
    /// This bounds the duration of retained history, not the number of
    /// samples or frames, and it is not a limit on the gap a lookup
    /// interpolates across.
    #[must_use]
    pub fn with_max_age(max_age: Duration) -> Self {
        Self {
            frames: Frames::default(),
            max_age: Some(max_age),
        }
    }

    /// Creates a registry that keeps every sample until it is removed.
    ///
    /// Samples stay until [`Registry::remove_samples_through`],
    /// [`Registry::remove_frame`] or [`Registry::reparent`] removes them:
    /// memory use is unbounded, no insert is ever rejected as expired, and
    /// a stream whose clock steps back is stored beside its older run, with
    /// lookups interpolating between the two.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            frames: Frames::default(),
            max_age: None,
        }
    }

    /// Stores a transform as the edge from its
    /// [`source`](Transform::source), the child, to its
    /// [`target`](Transform::target), the parent.
    ///
    /// The child's first insert pins its parent and its kind: a
    /// `Stamp::Static` transform makes a static edge, a `Stamp::At` sample
    /// a dynamic one. A later static insert replaces the pose, and a later
    /// sample at a stored instant replaces that sample.
    /// [`Registry::reparent`] moves a frame under another parent;
    /// [`Registry::remove_frame`] releases both pins.
    ///
    /// On an error the registry is unchanged, and no name has become
    /// known. The transform is borrowed, so a rejected one stays with the
    /// caller.
    ///
    /// # Errors
    ///
    /// - [`InsertError::SelfEdge`] if the two frames are the same.
    /// - [`InsertError::ParentMismatch`] if the child has another parent.
    /// - [`InsertError::KindMismatch`] if the child's kind differs.
    /// - [`InsertError::Cycle`] if a new edge would close a cycle.
    /// - [`InsertError::Expired`] if the sample is older than the child's
    ///   retention window; see [`Registry::with_max_age`].
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{InsertError, Registry, Rotation, Stamp, Transform, Vector3};
    ///
    /// let mount = |parent, child| {
    ///     Transform::new(
    ///         parent,
    ///         child,
    ///         Vector3::new(0.0, 0.0, 0.1),
    ///         Rotation::IDENTITY,
    ///         Stamp::Static,
    ///     )
    ///     .unwrap()
    /// };
    /// let mut registry = Registry::unbounded();
    /// registry.insert(&mount("base", "camera")).unwrap();
    ///
    /// // `camera` is pinned under `base`.
    /// let moved = mount("arm", "camera");
    /// assert!(matches!(
    ///     registry.insert(&moved),
    ///     Err(InsertError::ParentMismatch { .. })
    /// ));
    /// ```
    pub fn insert(
        &mut self,
        transform: &Transform,
    ) -> Result<(), InsertError> {
        self.check(transform, &Staged::new())?;
        self.commit(transform)
    }

    /// Stores a batch of transforms, all or nothing.
    ///
    /// On success the registry holds what calling [`Registry::insert`] on
    /// each member in order would leave. On an error it is unchanged:
    /// nothing from the batch is stored, no sample is evicted, and no name
    /// has become known. The error is the first one that loop would
    /// report, with the index of the member that caused it.
    ///
    /// The order of a batch decides its result, as it does for the loop. A
    /// member can name a frame an earlier member created, and under
    /// [`Registry::with_max_age`] each sample is measured against its
    /// frame's newest stamp at its turn, earlier members included. With a
    /// 10 s window, one frame's samples at 50 s and then 100 s are both
    /// accepted, and the first is evicted by the second before the call
    /// returns; in the other order the batch fails with
    /// [`InsertError::Expired`] and stores neither.
    ///
    /// One rejected member rejects the whole batch, so a late sample for
    /// one frame also rejects fresh samples for every other frame in the
    /// same message. To keep those, insert the members one at a time, or
    /// resubmit the batch without the member at
    /// [`index`](BatchError::index).
    ///
    /// # Errors
    ///
    /// A [`BatchError`] carrying the zero-based index of the rejected
    /// member and the [`InsertError`] that [`Registry::insert`] documents.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{InsertError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let t = Timestamp::from_nanos(1);
    /// let edge = |parent, child| {
    ///     Transform::new(
    ///         parent,
    ///         child,
    ///         Vector3::new(1.0, 0.0, 0.0),
    ///         Rotation::IDENTITY,
    ///         Stamp::At(t),
    ///     )
    ///     .unwrap()
    /// };
    /// let mut registry = Registry::unbounded();
    ///
    /// // `base` enters the tree in the same batch that hangs `lidar` from it.
    /// registry
    ///     .insert_all(&[edge("map", "base"), edge("base", "lidar")])
    ///     .unwrap();
    /// let transform = registry.lookup("map", "lidar", t).unwrap();
    /// assert_eq!(transform.translation(), Vector3::new(2.0, 0.0, 0.0));
    ///
    /// // The second member closes a cycle, so the first is not stored either.
    /// let rejected = registry
    ///     .insert_all(&[edge("lidar", "camera"), edge("camera", "map")])
    ///     .unwrap_err();
    /// assert_eq!(rejected.index, 1);
    /// assert!(matches!(rejected.error, InsertError::Cycle { .. }));
    /// assert!(registry.lookup("lidar", "camera", t).is_err());
    /// ```
    pub fn insert_all(
        &mut self,
        transforms: &[Transform],
    ) -> Result<(), BatchError> {
        // Every member is checked before any is stored, against the stored
        // frames and against the pins the members before it set. Those pins
        // are all a check reads, so a batch that passes here has no failure
        // left in it.
        let mut staged = Staged::new();
        for (index, transform) in transforms.iter().enumerate() {
            let pin = self
                .check(transform, &staged)
                .map_err(|error| BatchError { index, error })?;
            staged.insert(transform.source(), pin);
        }
        drop(staged);

        for (index, transform) in transforms.iter().enumerate() {
            self.commit(transform)
                .map_err(|error| BatchError { index, error })?;
        }
        Ok(())
    }

    /// Moves a child frame under a new parent, dropping its stored history.
    ///
    /// `seed.source()` is the frame to move and `seed.target()` its new
    /// parent. The frame's edge is replaced by one that holds only the
    /// seed, of the frame's own kind. Descendants keep their pins and
    /// history. Every check runs before any change, so a rejected call
    /// leaves the registry unchanged and makes no name known.
    ///
    /// A dynamic frame's coverage collapses to the seed's instant: other
    /// instants report [`LookupError::OutOfRange`] until new samples
    /// arrive, and [`Registry::latest_common_time`] can move backwards. A
    /// static frame's replaced pose answers every instant, past included.
    ///
    /// The seed's stamp is not measured against the dropped history, so it
    /// never fails as expired. A stale seed moves coverage backwards, and
    /// under [`Registry::with_max_age`] a seed ahead of the live stream
    /// makes later samples older than `max_age` behind it fail with
    /// [`InsertError::Expired`]. Take the seed's stamp from the frame's own
    /// stream, and express samples published on the new edge relative to
    /// the new parent.
    ///
    /// The new parent need not be known. The old parent stops being known
    /// if no other edge names it, and the call does not report it; record
    /// it beforehand if needed. To change a frame's kind or keep its
    /// history, use [`Registry::remove_frame`] and insert again.
    ///
    /// # Errors
    ///
    /// - [`InsertError::UnknownFrame`] if the frame to move is not known,
    ///   or [`InsertError::NoParent`] if it is a root.
    /// - [`InsertError::ParentUnchanged`] if it is already under that
    ///   parent.
    /// - [`InsertError::SelfEdge`] if the seed names one frame twice.
    /// - [`InsertError::Cycle`] if the new parent is below the frame.
    /// - [`InsertError::KindMismatch`] if the seed's kind differs from the
    ///   frame's.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let sample = |parent, x, nanos| {
    ///     Transform::new(
    ///         parent,
    ///         "odom",
    ///         Vector3::new(x, 0.0, 0.0),
    ///         Rotation::IDENTITY,
    ///         Stamp::At(Timestamp::from_nanos(nanos)),
    ///     )
    ///     .unwrap()
    /// };
    /// let mut registry = Registry::unbounded();
    ///
    /// // `odom` starts under `map_a` ...
    /// registry.insert(&sample("map_a", 1.0, 1_000)).unwrap();
    ///
    /// // ... until a map switch moves it under `map_b`. The seed both
    /// // re-pins the frame and starts its history under the new parent.
    /// registry.reparent(&sample("map_b", 2.0, 2_000)).unwrap();
    ///
    /// let moved = registry
    ///     .lookup("map_b", "odom", Timestamp::from_nanos(2_000))
    ///     .unwrap();
    /// assert_eq!(moved.translation(), Vector3::new(2.0, 0.0, 0.0));
    /// // `map_a` was named by no other edge.
    /// assert!(
    ///     registry
    ///         .lookup("map_a", "odom", Timestamp::from_nanos(1_000))
    ///         .is_err()
    /// );
    /// ```
    pub fn reparent(
        &mut self,
        seed: &Transform,
    ) -> Result<(), InsertError> {
        let (frame, parent) = (seed.source(), seed.target());
        let Some(child) = self.frames.id(frame) else {
            return Err(InsertError::UnknownFrame {
                frame: frame.into(),
            });
        };
        let Some(edge) = self.frames.edge(child) else {
            return Err(InsertError::NoParent {
                frame: frame.into(),
            });
        };
        if self.frames.name(edge.parent) == parent {
            return Err(InsertError::ParentUnchanged {
                frame: frame.into(),
                parent: parent.into(),
            });
        }
        if frame == parent {
            return Err(InsertError::SelfEdge {
                frame: frame.into(),
            });
        }
        // Walked with the old edge still in place. A walk upward from the
        // new parent cannot cross that edge without first arriving at the
        // frame itself, which is the case that reports the cycle, so the
        // tree before the move gives the answer the tree after it would.
        if self
            .frames
            .id(parent)
            .is_some_and(|new_parent| self.frames.is_at_or_above(child, new_parent))
        {
            return Err(InsertError::Cycle {
                child: frame.into(),
                parent: parent.into(),
            });
        }
        if edge.history.is_static() != (seed.stamp() == Stamp::Static) {
            return Err(InsertError::KindMismatch {
                frame: frame.into(),
                incoming: seed.stamp(),
            });
        }

        // The seed is not measured against the history it replaces: there
        // is no newest stamp left to expire it against.
        self.frames.reattach(
            child,
            parent,
            History::seeded(seed.isometry(), seed.stamp()),
        );
        Ok(())
    }

    /// Removes a child frame's edge to its parent, with every sample stored
    /// on it.
    ///
    /// Returns `true` iff `child` had such an edge. A root, a name known
    /// only as a parent, an unknown name and the empty name return `false`
    /// and change nothing.
    ///
    /// The frame's parent and kind are released, and so is its newest
    /// stamp, which [`Registry::with_max_age`] measures against. Its
    /// children keep their pins and their history: the removed frame
    /// becomes the root of their subtree, disconnected from its former
    /// ancestors, and stays known while it has children. Its former parent
    /// stops being known if no other edge names it.
    ///
    /// To move a subtree, [`Registry::reparent`] does it in one call at the
    /// price of the moved frame's history. Removing and inserting again is
    /// the route for changing a frame's kind, or for keeping its history
    /// re-expressed relative to the new parent.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let mount = |parent, child| {
    ///     Transform::new(
    ///         parent,
    ///         child,
    ///         Vector3::new(0.0, 0.0, 0.0),
    ///         Rotation::IDENTITY,
    ///         Stamp::Static,
    ///     )
    ///     .unwrap()
    /// };
    /// let mut registry = Registry::unbounded();
    /// registry.insert(&mount("world", "base")).unwrap();
    /// registry.insert(&mount("base", "sensor")).unwrap();
    ///
    /// assert!(registry.remove_frame("base"));
    /// // `world` was named only by the removed edge.
    /// assert!(!registry.remove_frame("world"));
    ///
    /// registry.insert(&mount("new_world", "base")).unwrap();
    /// let transform = registry
    ///     .lookup("new_world", "sensor", Timestamp::from_nanos(0))
    ///     .unwrap();
    /// assert_eq!(transform.target(), "new_world");
    /// ```
    pub fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.frames
            .id(child)
            .is_some_and(|id| self.frames.detach(id))
    }

    /// Removes every dynamic sample at or before `cutoff`, in every frame.
    ///
    /// Static transforms stay, and so does the tree: a drained frame keeps
    /// its parent, its kind and its name, and lookups across its edge
    /// report [`LookupError::NoData`]. A drained frame has no newest stamp
    /// left, so under [`Registry::with_max_age`] its stream can restart at
    /// any instant.
    ///
    /// The cutoff is inclusive, so a frame's newest sample is always
    /// removable: [`Timestamp::MAX`] clears all dynamic history. Use
    /// [`Registry::remove_frame`] to discard one frame's samples and
    /// release its pins.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let mut registry = Registry::unbounded();
    /// for nanos in [10, 20, 30] {
    ///     registry
    ///         .insert(
    ///             &Transform::new(
    ///                 "map",
    ///                 "base",
    ///                 Vector3::new(0.0, 0.0, 0.0),
    ///                 Rotation::IDENTITY,
    ///                 Stamp::At(Timestamp::from_nanos(nanos)),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// registry.remove_samples_through(Timestamp::from_nanos(20));
    /// assert!(matches!(
    ///     registry.lookup("map", "base", Timestamp::from_nanos(20)),
    ///     Err(LookupError::OutOfRange { .. })
    /// ));
    /// assert!(
    ///     registry
    ///         .lookup("map", "base", Timestamp::from_nanos(30))
    ///         .is_ok()
    /// );
    /// ```
    pub fn remove_samples_through(
        &mut self,
        cutoff: Timestamp,
    ) {
        for history in self.frames.histories_mut() {
            history.remove_through(cutoff);
        }
    }

    /// Maps `source`-frame coordinates into `target` at the instant `at`.
    ///
    /// The result has `target` and `source` as its frames and carries
    /// `Stamp::At(at)`, also over a chain of static edges. A known frame
    /// relative to itself is the identity.
    ///
    /// Dynamic edges are interpolated between the two samples that bracket
    /// `at`, linearly in translation and by slerp in rotation, across
    /// whatever gap lies between them. There is no extrapolation: use
    /// [`Registry::latest_common_time`] to find the newest instant the
    /// chain serves. A lookup from a frame to one of its ancestors at a
    /// stored instant returns the stored numbers bit for bit when it
    /// crosses one edge.
    ///
    /// # Errors
    ///
    /// Diagnosed in this order:
    ///
    /// - [`LookupError::UnknownFrame`] if `target` or, failing that,
    ///   `source` is not known. An empty registry knows no name, so it
    ///   answers every lookup with this, equal names included.
    /// - [`LookupError::Disconnected`] if no chain connects the frames.
    /// - [`LookupError::NoData`] or [`LookupError::OutOfRange`] for an edge
    ///   on the connecting chain that cannot serve `at`: the lowest such
    ///   edge on the target's side, else on the source's.
    /// - [`LookupError::NonFinite`] or [`LookupError::NonUnitRotation`] if
    ///   the chain is covered and combining it failed numerically.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let mut registry = Registry::unbounded();
    /// for (nanos, x) in [(0, 0.0), (10, 1.0)] {
    ///     registry
    ///         .insert(
    ///             &Transform::new(
    ///                 "map",
    ///                 "base",
    ///                 Vector3::new(x, 0.0, 0.0),
    ///                 Rotation::IDENTITY,
    ///                 Stamp::At(Timestamp::from_nanos(nanos)),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// let at = Timestamp::from_nanos(5);
    /// let transform = registry.lookup("map", "base", at).unwrap();
    /// assert_eq!(transform.target(), "map");
    /// assert_eq!(transform.source(), "base");
    /// assert_eq!(transform.translation(), Vector3::new(0.5, 0.0, 0.0));
    ///
    /// // The stamp of a lookup result is always `At`.
    /// let Stamp::At(stamp) = transform.stamp() else {
    ///     unreachable!()
    /// };
    /// assert_eq!(stamp, at);
    /// ```
    pub fn lookup(
        &self,
        target: &str,
        source: &str,
        at: Timestamp,
    ) -> Result<Transform, LookupError> {
        let target_id = self.known(target)?;
        let source_id = self.known(source)?;
        let top = self
            .frames
            .meeting_point(target_id, source_id)
            .ok_or_else(|| disconnected(target, source))?;
        let isometry = self.resolve(target_id, source_id, top, at)?;
        Ok(Transform::from_parts(
            target.into(),
            source.into(),
            isometry,
            Stamp::At(at),
        ))
    }

    /// Maps coordinates in `source` at `source_time` into `target` at
    /// `target_time`, through a frame that stays put between the two.
    ///
    /// This is tf2-style "time travel": a position measured in the past is
    /// re-expressed in a frame as it is now. It does not predict motion.
    /// The source is resolved against `fixed` at `source_time`, the target
    /// against `fixed` at `target_time`, and the two are combined.
    ///
    /// The caller asserts that `fixed` is stationary between the two
    /// instants; the registry does not infer it, and a frame with no parent
    /// is not thereby stationary. Swapped times and a moving `fixed` frame
    /// both return an ordinary result.
    ///
    /// Both legs are always resolved, also when the endpoints and the times
    /// coincide, so a `fixed` frame that is not connected to them is an
    /// error. A leg whose endpoint is `fixed` itself is the identity.
    ///
    /// # Errors
    ///
    /// Diagnosed in this order, the target's leg first within each step:
    ///
    /// - [`LookupError::UnknownFrame`] for the first unknown name among
    ///   `target`, `source` and `fixed`.
    /// - [`LookupError::Disconnected`] naming the endpoint that no chain
    ///   connects to `fixed`.
    /// - [`LookupError::NoData`] or [`LookupError::OutOfRange`] for an edge
    ///   on a leg's chain that cannot serve that leg's instant.
    /// - [`LookupError::NonFinite`] or [`LookupError::NonUnitRotation`] if
    ///   both legs are covered and combining them failed numerically.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// let source_time = Timestamp::from_nanos(1);
    /// let target_time = Timestamp::from_nanos(2);
    /// let mut registry = Registry::unbounded();
    /// for (time, x) in [(source_time, 1.0), (target_time, 2.0)] {
    ///     registry
    ///         .insert(
    ///             &Transform::new(
    ///                 "world",
    ///                 "camera",
    ///                 Vector3::new(x, 0.0, 0.0),
    ///                 Rotation::IDENTITY,
    ///                 Stamp::At(time),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// // Where the camera's earlier origin lies in its later frame.
    /// let across = registry
    ///     .lookup_across("camera", target_time, "camera", source_time, "world")
    ///     .unwrap();
    /// assert_eq!(across.translation(), Vector3::new(-1.0, 0.0, 0.0));
    /// assert_eq!(across.target_time(), target_time);
    /// assert_eq!(across.source_time(), source_time);
    /// ```
    pub fn lookup_across(
        &self,
        target: &str,
        target_time: Timestamp,
        source: &str,
        source_time: Timestamp,
        fixed: &str,
    ) -> Result<CrossTimeTransform, LookupError> {
        let target_id = self.known(target)?;
        let source_id = self.known(source)?;
        let fixed_id = self.known(fixed)?;

        // Topology of both legs before the coverage of either, and all
        // coverage before any numerical failure.
        let target_top = self
            .frames
            .meeting_point(fixed_id, target_id)
            .ok_or_else(|| disconnected(target, fixed))?;
        let source_top = self
            .frames
            .meeting_point(fixed_id, source_id)
            .ok_or_else(|| disconnected(source, fixed))?;
        let target_leg = self.resolve(fixed_id, target_id, target_top, target_time);
        let source_leg = self.resolve(fixed_id, source_id, source_top, source_time);
        let (fixed_from_target, fixed_from_source) = match (target_leg, source_leg) {
            (Err(Failure::Coverage(error)), _)
            | (Ok(_) | Err(Failure::Numeric(_)), Err(Failure::Coverage(error))) => {
                return Err(error);
            }
            (Err(Failure::Numeric(error)), _) | (Ok(_), Err(Failure::Numeric(error))) => {
                return Err(error.into());
            }
            (Ok(target_leg), Ok(source_leg)) => (target_leg, source_leg),
        };

        // The legs hold at different instants on purpose, so they combine
        // as bare geometry. A leg that never left the fixed frame takes no
        // part in the arithmetic: the other leg's numbers pass through
        // untouched.
        let isometry = match (target_id == fixed_id, source_id == fixed_id) {
            (true, _) => fixed_from_source,
            (false, true) => fixed_from_target.inverse()?,
            (false, false) => fixed_from_target.inverse()?.compose(fixed_from_source)?,
        };
        Ok(CrossTimeTransform::new(
            target.into(),
            target_time,
            source.into(),
            source_time,
            isometry,
        ))
    }

    /// The newest instant at which `source` can be looked up in `target`.
    ///
    /// Only the edges on the chain that connects the two frames count. The
    /// answer is the oldest of their newest samples, provided every one of
    /// them reaches back to it, and it does not depend on the order of the
    /// arguments. No clock is consulted, and no geometry is evaluated: a
    /// lookup at the returned instant is covered, and can still fail
    /// numerically.
    ///
    /// `Stamp::Static` means the chain has no dynamic edge, which includes
    /// a known frame relative to itself. Every instant is served then, and
    /// any [`Timestamp`], for example `Timestamp::from_nanos(0)`, is a
    /// valid lookup instant.
    ///
    /// If the registry is shared, make this call and the lookup that uses
    /// its answer under one read guard.
    ///
    /// # Errors
    ///
    /// Diagnosed in this order:
    ///
    /// - [`LookupError::UnknownFrame`] if `target` or, failing that,
    ///   `source` is not known.
    /// - [`LookupError::Disconnected`] if no chain connects the frames.
    /// - [`LookupError::NoData`] for a dynamic edge on the chain that holds
    ///   no samples.
    /// - [`LookupError::NoCommonTime`] if the edges' coverage is disjoint.
    ///
    /// # Examples
    ///
    /// A "latest" lookup asks for the newest instant the chain serves. The
    /// wall clock is not that instant: it is normally past a dynamic edge's
    /// newest sample, and a lookup there fails.
    ///
    /// ```
    /// use transforms::{LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
    ///
    /// fn latest(
    ///     registry: &Registry,
    ///     target: &str,
    ///     source: &str,
    /// ) -> Result<Transform, LookupError> {
    ///     let at = match registry.latest_common_time(target, source)? {
    ///         Stamp::At(newest) => newest,
    ///         // Every edge is static and serves any instant.
    ///         Stamp::Static => Timestamp::from_nanos(0),
    ///     };
    ///     registry.lookup(target, source, at)
    /// }
    ///
    /// let mut registry = Registry::unbounded();
    /// for (parent, child, stamp) in [
    ///     ("map", "odom", Stamp::At(Timestamp::from_nanos(4))),
    ///     ("map", "odom", Stamp::At(Timestamp::from_nanos(9))),
    ///     ("odom", "base", Stamp::At(Timestamp::from_nanos(3))),
    ///     ("odom", "base", Stamp::At(Timestamp::from_nanos(7))),
    ///     ("base", "lidar", Stamp::Static),
    /// ] {
    ///     registry
    ///         .insert(
    ///             &Transform::new(
    ///                 parent,
    ///                 child,
    ///                 Vector3::new(0.0, 0.0, 0.0),
    ///                 Rotation::IDENTITY,
    ///                 stamp,
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// let newest = Stamp::At(Timestamp::from_nanos(7));
    /// assert_eq!(registry.latest_common_time("map", "lidar"), Ok(newest));
    /// assert_eq!(latest(&registry, "map", "lidar").unwrap().stamp(), newest);
    ///
    /// assert_eq!(
    ///     registry.latest_common_time("base", "lidar"),
    ///     Ok(Stamp::Static)
    /// );
    /// assert!(latest(&registry, "base", "lidar").is_ok());
    /// ```
    pub fn latest_common_time(
        &self,
        target: &str,
        source: &str,
    ) -> Result<Stamp, LookupError> {
        let target_id = self.known(target)?;
        let source_id = self.known(source)?;
        let Some(top) = self.frames.meeting_point(target_id, source_id) else {
            return Err(disconnected(target, source));
        };

        // The newest instant every edge serves is the oldest of the dynamic
        // edges' newest samples, provided every range reaches back to it,
        // which the latest-starting range decides. Edges above the meeting
        // point are not on the chain and must not constrain the answer.
        let mut latest_possible: Option<Timestamp> = None;
        let mut latest_start: Option<(FrameId, Timestamp, Timestamp)> = None;
        for end in [target_id, source_id] {
            let mut id = end;
            while id != top {
                let Some(edge) = self.frames.edge(id) else {
                    break;
                };
                if !edge.history.is_static() {
                    let Some((earliest, latest)) = edge.history.range() else {
                        return Err(LookupError::NoData {
                            frame: self.frames.name(id).into(),
                        });
                    };
                    if latest_possible.is_none_or(|current| latest < current) {
                        latest_possible = Some(latest);
                    }
                    if latest_start.is_none_or(|(_, start, _)| earliest > start) {
                        latest_start = Some((id, earliest, latest));
                    }
                }
                id = edge.parent;
            }
        }

        match (latest_possible, latest_start) {
            (Some(latest_possible), Some((frame, earliest, latest)))
                if earliest > latest_possible =>
            {
                Err(LookupError::NoCommonTime {
                    frame: self.frames.name(frame).into(),
                    earliest,
                    latest,
                    latest_possible,
                })
            }
            (Some(latest_possible), _) => Ok(Stamp::At(latest_possible)),
            // No dynamic edge: the chain puts no bound on time.
            (None, _) => Ok(Stamp::Static),
        }
    }

    /// The pin a stored edge holds for `child`; `None` for a frame with no
    /// edge to a parent.
    fn pin(
        &self,
        child: &str,
    ) -> Option<Pin<'_>> {
        let edge = self.frames.edge(self.frames.id(child)?)?;
        Some(Pin {
            parent: self.frames.name(edge.parent),
            is_static: edge.history.is_static(),
            newest: edge.history.range().map(|(_, latest)| latest),
        })
    }

    /// Runs every insert check on `transform` without storing anything,
    /// and returns the pin its child frame holds once it is stored.
    ///
    /// `staged` holds the pins earlier members of a batch set; a single
    /// insert passes none. The checks read pins and nothing else: a parent,
    /// a kind and a newest instant, none of which a later insert of the
    /// same batch can invalidate. That is what makes checking a whole batch
    /// first and storing it after equal to the insert loop.
    ///
    /// # Errors
    ///
    /// The errors [`Registry::insert`] documents. Expiry is the last check,
    /// so a sample that reports it has the frame's parent and kind.
    fn check<'a>(
        &'a self,
        transform: &'a Transform,
        staged: &Staged<'a>,
    ) -> Result<Pin<'a>, InsertError> {
        let (frame, parent, stamp) = (transform.source(), transform.target(), transform.stamp());
        if frame == parent {
            return Err(InsertError::SelfEdge {
                frame: frame.into(),
            });
        }

        let Some(pin) = staged.get(frame).copied().or_else(|| self.pin(frame)) else {
            // A new edge changes the tree, so it is the one case that can
            // close a cycle: an edge that exists has passed this already,
            // and its parent cannot change.
            if self.closes_cycle(frame, parent, staged) {
                return Err(InsertError::Cycle {
                    child: frame.into(),
                    parent: parent.into(),
                });
            }
            return Ok(match stamp {
                Stamp::Static => Pin {
                    parent,
                    is_static: true,
                    newest: None,
                },
                Stamp::At(at) => Pin {
                    parent,
                    is_static: false,
                    newest: Some(at),
                },
            });
        };

        if pin.parent != parent {
            return Err(InsertError::ParentMismatch {
                frame: frame.into(),
                pinned: pin.parent.into(),
                offered: parent.into(),
            });
        }
        let at = match (pin.is_static, stamp) {
            (true, Stamp::Static) => return Ok(pin),
            (false, Stamp::At(at)) => at,
            _ => {
                return Err(InsertError::KindMismatch {
                    frame: frame.into(),
                    incoming: stamp,
                });
            }
        };
        let Some(newest) = pin.newest else {
            // A drained history has no newest instant to expire against.
            return Ok(Pin {
                newest: Some(at),
                ..pin
            });
        };
        // Where the boundary is not representable, nothing is older.
        let expired = self
            .max_age
            .and_then(|max_age| newest.checked_sub(max_age))
            .is_some_and(|cutoff| at < cutoff);
        if expired {
            return Err(InsertError::Expired {
                frame: frame.into(),
                stamp: at,
                newest,
            });
        }
        Ok(Pin {
            newest: Some(newest.max(at)),
            ..pin
        })
    }

    /// Whether the new edge `parent <- child` would close a cycle in the
    /// tree of stored and staged edges: whether a walk upward from `parent`
    /// arrives at `child`.
    ///
    /// The walk ends because that tree is acyclic: every edge in it passed
    /// this check when it entered.
    fn closes_cycle(
        &self,
        child: &str,
        parent: &str,
        staged: &Staged<'_>,
    ) -> bool {
        let mut current = parent;
        loop {
            if current == child {
                return true;
            }
            let pin = staged.get(current).copied().or_else(|| self.pin(current));
            match pin {
                Some(pin) => current = pin.parent,
                None => return false,
            }
        }
    }

    /// Stores a transform that [`Self::check`] has accepted.
    ///
    /// # Errors
    ///
    /// Returns `InsertError::KindMismatch` if the stamp is of the other
    /// kind than the stored edge. The check refuses that first, so this is
    /// unreachable; it is reported rather than assumed so that a broken
    /// check surfaces instead of a transform being dropped.
    fn commit(
        &mut self,
        transform: &Transform,
    ) -> Result<(), InsertError> {
        let (frame, isometry, stamp) =
            (transform.source(), transform.isometry(), transform.stamp());
        let max_age = self.max_age;
        let stored = self.frames.id(frame);
        match stored.and_then(|id| self.frames.history_mut(id)) {
            Some(history) => {
                if !history.store(isometry, stamp, max_age) {
                    return Err(InsertError::KindMismatch {
                        frame: frame.into(),
                        incoming: stamp,
                    });
                }
            }
            None => {
                self.frames
                    .attach(frame, transform.target(), History::seeded(isometry, stamp));
            }
        }
        Ok(())
    }

    /// The id of a name in a query.
    ///
    /// # Errors
    ///
    /// Returns `LookupError::UnknownFrame` for a name no stored edge names.
    fn known(
        &self,
        name: &str,
    ) -> Result<FrameId, LookupError> {
        self.frames
            .id(name)
            .ok_or_else(|| LookupError::UnknownFrame { frame: name.into() })
    }

    /// Resolves the isometry that maps `source` into `target` at `at`,
    /// given the frame `top` where their chains meet.
    ///
    /// Only the edges between the two frames and `top` are sampled. Each
    /// side composes upward in the order it is walked, and the target's
    /// side is then inverted once: a lookup toward an ancestor inverts
    /// nothing, and across one edge at a stored instant it returns the
    /// stored isometry untouched.
    ///
    /// # Errors
    ///
    /// `Failure::Coverage` for an edge that cannot serve `at`, in
    /// preference to `Failure::Numeric` for arithmetic that failed.
    #[inline]
    fn resolve(
        &self,
        target: FrameId,
        source: FrameId,
        top: FrameId,
        at: Timestamp,
    ) -> Result<Isometry, Failure> {
        let halves = self
            .ascend(target, top, at)
            .and_then(|target_half| Ok((target_half, self.ascend(source, top, at)?)));
        let (top_from_target, top_from_source) = match halves {
            Ok(halves) => halves,
            Err(fault) => return Err(self.diagnose(target, source, top, at, fault)),
        };
        let combined = match (top_from_target, top_from_source) {
            (None, None) => Ok(Isometry::IDENTITY),
            (None, Some(top_from_source)) => Ok(top_from_source),
            (Some(top_from_target), None) => top_from_target.inverse(),
            (Some(top_from_target), Some(top_from_source)) => top_from_target
                .inverse()
                .and_then(|target_from_top| target_from_top.compose(top_from_source)),
        };
        // Every edge of the chain has been sampled by now, so coverage is
        // settled and a failure here is the numerical one to report.
        combined.map_err(Failure::Numeric)
    }

    /// Samples and composes the edges from `from` up to `top`: the isometry
    /// that maps `from` into `top`, or `None` if they are the same frame.
    #[inline]
    fn ascend(
        &self,
        from: FrameId,
        top: FrameId,
        at: Timestamp,
    ) -> Result<Option<Isometry>, Fault> {
        let mut id = from;
        let mut composed: Option<Isometry> = None;
        while id != top {
            let Some(edge) = self.frames.edge(id) else {
                return Err(Fault::Gap {
                    edge: id,
                    gap: Gap::Empty,
                });
            };
            let hop = edge.history.sample(at).map_err(|error| match error {
                SampleError::Gap(gap) => Fault::Gap { edge: id, gap },
                SampleError::Numeric(error) => Fault::Numeric(error),
            })?;
            composed = Some(match composed {
                None => hop,
                Some(below) => hop.compose(below).map_err(Fault::Numeric)?,
            });
            id = edge.parent;
        }
        Ok(composed)
    }

    /// Decides what a failed resolution reports. Runs only on that path.
    ///
    /// Coverage comes before numerics, so the whole chain is re-read for
    /// its coverage alone, whatever stopped the first walk: a numerical
    /// failure low on the chain must not hide an edge further up that
    /// cannot serve the instant at all. The edge reported is the lowest
    /// failing one on the target's side, else on the source's.
    #[cold]
    #[inline(never)]
    fn diagnose(
        &self,
        target: FrameId,
        source: FrameId,
        top: FrameId,
        at: Timestamp,
        fault: Fault,
    ) -> Failure {
        let uncovered = self
            .first_gap(target, top, at)
            .or_else(|| self.first_gap(source, top, at));
        match (uncovered, fault) {
            (Some((edge, gap)), _) | (None, Fault::Gap { edge, gap }) => {
                Failure::Coverage(self.coverage_error(edge, gap, at))
            }
            (None, Fault::Numeric(error)) => Failure::Numeric(error),
        }
    }

    /// The lowest edge between `from` and `top` that does not cover `at`.
    fn first_gap(
        &self,
        from: FrameId,
        top: FrameId,
        at: Timestamp,
    ) -> Option<(FrameId, Gap)> {
        let mut id = from;
        while id != top {
            let Some(edge) = self.frames.edge(id) else {
                return Some((id, Gap::Empty));
            };
            if let Some(gap) = edge.history.gap(at) {
                return Some((id, gap));
            }
            id = edge.parent;
        }
        None
    }

    /// The public error for an edge that does not cover `at`.
    fn coverage_error(
        &self,
        edge: FrameId,
        gap: Gap,
        at: Timestamp,
    ) -> LookupError {
        let frame = self.frames.name(edge).into();
        match gap {
            Gap::Empty => LookupError::NoData { frame },
            Gap::Outside { earliest, latest } => LookupError::OutOfRange {
                frame,
                requested: at,
                earliest,
                latest,
            },
        }
    }
}

/// The error for two known frames that no chain connects.
fn disconnected(
    a: &str,
    b: &str,
) -> LookupError {
    LookupError::Disconnected {
        a: a.into(),
        b: b.into(),
    }
}

#[cfg(test)]
mod tests;
