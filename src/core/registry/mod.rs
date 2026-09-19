//! Frame-tree storage and transform lookup.

use crate::{
    core::{
        Buffer,
        buffer::{Coverage, GetError},
    },
    errors::TransformError,
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

/// A frame tree with timestamped transform history.
///
/// Lookups interpolate between samples and compose the connecting transforms.
/// Use [`Registry::with_max_age`] for automatic eviction or [`Registry::new`]
/// for manual retention. Buffers are private implementation details.
///
/// The time type defaults to [`Timestamp`] in type annotations. Where inference
/// has no timestamp to use, write `Registry::<Timestamp>::new()`. Custom clocks
/// implement [`TimePoint`]; `std::time::SystemTime` is supported with `std`.
///
/// `Registry<T>` is `Send` and `Sync` when `T` is. An external `RwLock` permits
/// concurrent readers; writers need exclusive access, and readers may wait for
/// writers. Keep dependent queries under one guard.
#[derive(Debug)]
pub struct Registry<T = Timestamp>
where
    T: TimePoint,
{
    /// Maps a child frame name to its transforms into the parent frame.
    data: HashMap<String, Buffer<T>>,
    max_age: Option<Duration>,
}

impl<T> Registry<T>
where
    T: TimePoint,
{
    /// Creates a registry without automatic cleanup.
    ///
    /// Samples remain until [`Registry::remove_transforms_before`] or
    /// [`Registry::remove_frame`] removes them. Memory use and interpolation
    /// gaps are unbounded. Use [`Registry::with_max_age`] to limit the retained
    /// time window.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_age: None,
        }
    }

    /// Creates a registry with eviction on insertion.
    ///
    /// Dynamic samples older than the child's newest inserted timestamp minus
    /// `max_age` are removed. The boundary is inclusive; `Duration::ZERO`
    /// retains only the newest sample. Wall-clock time is never consulted,
    /// and static transforms never expire.
    ///
    /// This bounds retained history duration, not sample or frame count.
    /// Available with and without `std`.
    #[must_use]
    pub fn with_max_age(max_age: Duration) -> Self {
        Self {
            data: HashMap::new(),
            max_age: Some(max_age),
        }
    }

    /// Inserts a transform, replacing any sample at the same timestamp.
    ///
    /// The first insert pins the child's parent and static/dynamic kind until
    /// [`Registry::remove_frame`] releases it. Numeric validation runs on every
    /// insert, including derived transforms that were not checked after composition.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::NonFiniteValues`] or [`RegistryError::NonUnitRotation`]
    ///   if numeric validation fails.
    /// - [`RegistryError::StaticDynamicConflict`] if the child's kind differs.
    /// - [`RegistryError::SelfReferentialFrame`] if parent equals child.
    /// - [`RegistryError::ReparentingNotSupported`] if the child's parent differs.
    ///   [`Registry::reparent_frame`] moves a frame deliberately.
    /// - [`RegistryError::CycleDetected`] if a new edge would close a cycle.
    pub fn add_transform(
        &mut self,
        t: Transform<T>,
    ) -> Result<(), RegistryError<T>> {
        Self::process_add_transform(t, &mut self.data, self.max_age)
    }

    /// Maps `source`-frame coordinates into `target` at `timestamp`.
    ///
    /// The argument order matches tf2's `lookupTransform(target, source, time)`.
    /// The result has `parent == target`, `child == source`, and
    /// `Stamp::At(timestamp)`, including over all-static chains. Equal frame
    /// names return the identity, even if the frame has not been registered.
    ///
    /// Interpolation spans any gap between retained samples; there is no
    /// extrapolation. Use [`Registry::latest_common_time`] to find the newest
    /// commonly covered instant. Neither query measures age against a clock.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownFrame`] for an unknown endpoint,
    /// [`RegistryError::NotFoundAt`] for a recorded sampling failure, or
    /// [`RegistryError::Disconnected`] for known frames with no connecting chain.
    /// A sampling failure takes precedence over a simultaneous disconnection;
    /// see the error variants for payloads.
    ///
    /// Geometry or time operations can fail with [`RegistryError::TransformError`]
    /// or [`RegistryError::NonFiniteValues`]. The result is not re-validated:
    /// inversion rejects an infinite translation, but a lookup toward an ancestor
    /// inverts nothing and can return an overflow as `Ok`. Call
    /// [`Transform::validate`] if inputs can reach such magnitudes.
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
    /// let t = Timestamp::from_nanos(1);
    /// let mut registry = Registry::new();
    /// registry
    ///     .add_transform(
    ///         Transform::new(
    ///             "map",
    ///             "lidar",
    ///             Vector3::new(2.0, 0.0, 0.0),
    ///             Quaternion::identity(),
    ///             Stamp::At(t),
    ///         )
    ///         .unwrap(),
    ///     )
    ///     .unwrap();
    ///
    /// let transform = registry.get_transform("map", "lidar", t).unwrap();
    /// assert_eq!(transform.parent(), "map");
    /// assert_eq!(transform.child(), "lidar");
    /// assert_eq!(transform.translation(), Vector3::new(2.0, 0.0, 0.0));
    /// ```
    pub fn get_transform(
        &self,
        target: &str,
        source: &str,
        timestamp: T,
    ) -> Result<Transform<T>, RegistryError<T>> {
        Self::process_get_transform(target, source, timestamp, &self.data)
    }

    /// Resolves a transform using `value`'s frame and timestamp.
    ///
    /// Equivalent to `get_transform(target_frame, value.frame(), value.timestamp())`.
    /// This does not modify `value`; apply the result separately through
    /// [`Transformable`](crate::Transformable).
    ///
    /// # Errors
    ///
    /// The same as [`Registry::get_transform`].
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

    /// Maps source-time coordinates into the target frame at another time.
    ///
    /// This is tf2-style "time travel": a historical position is re-expressed in
    /// another frame at `target_time`. It does not predict an object's motion.
    /// The two lookups connect through `fixed_frame`, which the caller must ensure
    /// is stationary between the instants. Having no parent does not establish
    /// physical stationarity.
    ///
    /// An endpoint equal to `fixed_frame` needs no lookup for that leg. If both
    /// endpoints equal it, the result is the identity at `target_time`.
    ///
    /// # Temporal metadata limitation
    ///
    /// The result stores only `target_time`. When the times differ, retain both
    /// instants and apply the rotation and translation explicitly to source-time
    /// coordinates, labeling the output with the target frame and time.
    ///
    /// Do not apply such a result through [`Transformable`](crate::Transformable),
    /// compose it as a single-time transform, or insert it into a registry.
    /// Those operations cannot check the lost source time; insertion can overwrite
    /// a valid target-time sample. Serialization also loses that provenance.
    /// [`Transform::validate`] checks numbers only.
    ///
    /// # Errors
    ///
    /// The same lookup and numeric failures as [`Registry::get_transform`],
    /// reported for either leg or their composition.
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
    /// let source_time = Timestamp::from_nanos(1);
    /// let target_time = Timestamp::from_nanos(2);
    /// let mut registry = Registry::new();
    /// for (time, x) in [(source_time, 1.0), (target_time, 2.0)] {
    ///     registry
    ///         .add_transform(
    ///             Transform::new(
    ///                 "world",
    ///                 "camera",
    ///                 Vector3::new(x, 0.0, 0.0),
    ///                 Quaternion::identity(),
    ///                 Stamp::At(time),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// // The camera's old origin, expressed in its later frame.
    /// let transform = registry
    ///     .get_transform_at("camera", target_time, "camera", source_time, "world")
    ///     .unwrap();
    /// let position = transform.rotation().rotate_vector(Vector3::zero()) + transform.translation();
    /// assert_eq!(position, Vector3::new(-1.0, 0.0, 0.0));
    /// assert_eq!(transform.timestamp(), Stamp::At(target_time));
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

    /// Returns the newest instant covered by every connecting hop.
    ///
    /// For dynamic chains, the answer is the minimum of the newest samples,
    /// provided it is at least the maximum of the oldest samples. Only edges
    /// between the requested frames count; edges above their common ancestor do
    /// not. The answer is symmetric in the frame arguments.
    ///
    /// Returns `Stamp::Static` for all-static chains or equal frame names, even
    /// unregistered ones. The caller then chooses an instant.
    ///
    /// Use the returned instant with [`Registry::get_transform`]. If the registry
    /// is shared, make both calls under one read guard: intervening writes can
    /// change the geometry, coverage, or topology. This query checks time arithmetic
    /// but does not evaluate geometry; the lookup can still fail numerically.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::UnknownFrame`] or [`RegistryError::Disconnected`] for
    ///   invalid topology. Unlike `get_transform`, this query checks topology first.
    /// - [`RegistryError::NoCommonTime`] for empty or disjoint coverage.
    /// - [`RegistryError::TransformError`] wrapping
    ///   [`TransformError::TimestampError`] if interpolation at the newest common
    ///   instant needs an unrepresentable interval or offset. Only the neighboring
    ///   samples are checked; no earlier instant is tried after an arithmetic error.
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
    ///     ("map", "odom", 4),
    ///     ("map", "odom", 9),
    ///     ("odom", "base", 3),
    ///     ("odom", "base", 7),
    /// ] {
    ///     registry
    ///         .add_transform(
    ///             Transform::new(
    ///                 parent,
    ///                 child,
    ///                 Vector3::zero(),
    ///                 Quaternion::identity(),
    ///                 Stamp::At(Timestamp::from_nanos(nanos)),
    ///             )
    ///             .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// let stamp = registry.latest_common_time("map", "base").unwrap();
    /// assert_eq!(stamp, Stamp::At(Timestamp::from_nanos(7)));
    /// let transform = registry
    ///     .get_transform("map", "base", stamp.at().unwrap())
    ///     .unwrap();
    /// assert_eq!(transform.timestamp(), stamp);
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
        for &(frame, buffer) in chain.clone() {
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
            (Some(end), _) => {
                for &(_, buffer) in chain {
                    buffer
                        .check_interpolation_time(end)
                        .map_err(TransformError::from)?;
                }
                Ok(Stamp::At(end))
            }
            // Every hop is static: the chain puts no bound on time.
            (None, _) => Ok(Stamp::Static),
        }
    }

    /// Removes dynamic samples strictly older than `timestamp`.
    ///
    /// Static transforms and frame entries remain. A drained frame keeps its parent
    /// and kind, and lookups crossing its edge report [`RegistryError::NotFoundAt`]
    /// with `covered: None`. A fully drained buffer resets its expiry reference,
    /// allowing a stream to restart at earlier timestamps.
    ///
    /// Use [`Registry::remove_frame`] to release a frame and its pins.
    pub fn remove_transforms_before(
        &mut self,
        timestamp: T,
    ) {
        for buffer in self.data.values_mut() {
            buffer.remove_before(timestamp);
        }
    }

    /// Removes a child's incoming edge and all samples stored on that edge.
    ///
    /// Returns whether the child had an edge. A root known only as a parent has
    /// no edge to remove. Descendant edges and their samples remain: the removed
    /// child becomes their subtree's root, disconnected from its former ancestors.
    ///
    /// To re-parent a subtree, remove its root's incoming edge and insert one
    /// under the new parent. Descendants whose immediate parents are unchanged
    /// need no removal; their stored history remains available.
    /// [`Registry::reparent_frame`] performs that move atomically at the price
    /// of the root's stored history; removal remains the route for changing a
    /// frame's kind or keeping its history.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
    ///     Registry,
    ///     geometry::{Quaternion, Transform, Vector3},
    ///     time::Timestamp,
    /// };
    ///
    /// let mut registry: Registry = Registry::new();
    /// for (parent, child) in [("world", "base"), ("base", "sensor")] {
    ///     registry
    ///         .add_transform(
    ///             Transform::static_between(parent, child, Vector3::zero(), Quaternion::identity())
    ///                 .unwrap(),
    ///         )
    ///         .unwrap();
    /// }
    ///
    /// assert!(registry.remove_frame("base"));
    /// registry
    ///     .add_transform(
    ///         Transform::static_between("new_world", "base", Vector3::zero(), Quaternion::identity())
    ///             .unwrap(),
    ///     )
    ///     .unwrap();
    /// let transform = registry
    ///     .get_transform("new_world", "sensor", Timestamp::zero())
    ///     .unwrap();
    /// assert_eq!(transform.parent(), "new_world");
    /// assert_eq!(transform.child(), "sensor");
    /// ```
    pub fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.data.remove(child).is_some()
    }

    /// Moves a child frame under a new parent, dropping its stored history.
    ///
    /// `t.child()` is the frame to move and `t.parent()` its new parent. The
    /// child's buffer is replaced by an empty one seeded with `t`, keeping the
    /// frame's static or dynamic kind and its `max_age` policy. Descendants keep
    /// their pins and history. Every check runs before any mutation, so a
    /// rejection leaves the registry unchanged.
    ///
    /// A dynamic frame's coverage collapses to the seed's instant: other
    /// instants report [`RegistryError::NotFoundAt`] with
    /// `covered: Some((seed, seed))` until new samples arrive, and
    /// [`Registry::latest_common_time`] can move backwards. A static frame's
    /// replaced pose answers every instant, past included, as any static
    /// re-publish does; a static move between the legs of
    /// [`Registry::get_transform_at`] changes its fixed frame without an error.
    ///
    /// The seed's timestamp is not checked against the dropped history. A stale
    /// seed moves coverage backwards; under [`Registry::with_max_age`], a seed
    /// ahead of the live stream evicts later samples older than `max_age`
    /// relative to it as they are inserted. Take the seed's stamp from the
    /// frame's own stream and call this on a decision to re-parent, not on
    /// message arrival. Samples published on the new edge should be measured
    /// relative to the new parent.
    ///
    /// An unregistered new parent is accepted, as in [`Registry::add_transform`].
    /// If the old parent was a root named nowhere else, it disappears and later
    /// lookups toward it report [`RegistryError::UnknownFrame`]. The call does
    /// not report the replaced parent; record it beforehand if needed.
    ///
    /// To change a frame's kind or keep its history, use
    /// [`Registry::remove_frame`] and re-insert, re-expressing kept samples
    /// relative to the new parent. If the registry is shared, make the decision
    /// and this call under one write guard: an interleaved writer can pre-empt
    /// the move, failing this call with `ParentUnchanged`, or seed a different
    /// parent that this call then replaces. Dropped samples are freed here, so
    /// latency scales with their count, and one extra buffer exists briefly.
    ///
    /// # Errors
    ///
    /// Checked in this order:
    ///
    /// - [`RegistryError::UnknownFrame`] if the child exists nowhere, or
    ///   [`RegistryError::NoParentToReplace`] if it is a root. A root gains a
    ///   parent through [`Registry::add_transform`]; see that variant for
    ///   reversing an edge.
    /// - [`RegistryError::ParentUnchanged`] if the parent is already `t.parent()`.
    ///   Re-parenting drops history, so this is not an upsert.
    /// - [`RegistryError::CycleDetected`] if the move would close a cycle. This
    ///   check needs the whole tree, which is why the move is one atomic call.
    /// - The seed's own insertion errors: [`RegistryError::NonUnitRotation`],
    ///   [`RegistryError::NonFiniteValues`], [`RegistryError::SelfReferentialFrame`],
    ///   and [`RegistryError::StaticDynamicConflict`] if the seed's kind differs
    ///   from the frame's.
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
        let mut current_frame = from;

        // The frame tree is acyclic by construction (cycles are rejected
        // whenever an edge enters the map — at insertion and at
        // re-parenting), so the walk visits every frame at most once and
        // terminates at a root.
        while let Some((frame_buffer, parent)) = data
            .get(current_frame)
            .and_then(|buffer| buffer.parent().map(|parent| (buffer, parent)))
        {
            match frame_buffer.get(timestamp) {
                Ok(tf) => {
                    current_frame = parent;
                    transforms.push_back(tf);
                }
                Err(source) => {
                    if walk_failure.is_none() {
                        *walk_failure = Some((current_frame.into(), source));
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
