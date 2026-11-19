# Migration guide

The first part covers 2.x to 3.0. The second part, "Migrating to 2.4", is
the earlier guide for 1.x and 2.0 users and uses 2.x names throughout. The
current API is documented on [docs.rs](https://docs.rs/transforms); release
history is in [CHANGELOG.md](CHANGELOG.md).

# Migrating from 2.x to 3.0

3.0 is not released yet; this guide describes it as it stands. Every 2.x
program needs changes. Each table row carries a tag:

- `compile`: the 2.x code stops compiling, and the compiler points at it.
- `silent`: the 2.x code can compile after the mechanical renames and
  behave differently. Read every `silent` row.

Code fragments show replacements; they assume
`use transforms::{BatchError, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, TransformError, Vector3};`
and `use core::time::Duration;`.

## Imports and features

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `transforms::geometry::{Transform, Quaternion, Vector3, Point}`, `transforms::time::{Stamp, Timestamp, TimePoint}`, `transforms::errors::{..}` | Everything is at the crate root: `use transforms::{Registry, Transform, Rotation, Vector3, Stamp, Timestamp};` and the four error types |
| compile | `Registry<T>`, `Transform<T>`, `Stamp<T>`, `Point<T>`, `RegistryError<T>` | No type parameter. Delete `::<Timestamp>` annotations and `<T: TimePoint>` bounds |
| compile | `features = ["std"]`, or `default-features = false` for `no_std` | Remove both. The crate is always `no_std` with `alloc`, and `serde` is its only feature. `default-features = false` is harmless and no longer needed |
| compile | `approx` traits on `Vector3`, `Quaternion`, `Transform`, `Point` | Removed. In tests, depend on `approx` yourself and compare `f64` components: `assert_abs_diff_eq!(tf.translation().x, 1.0, epsilon = 1e-12)` |

## Renames

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `registry.add_transform(tf)?` | `registry.insert(&tf)?` |
| compile | `registry.add_transforms(iter)?` | `registry.insert_all(&slice)?` |
| compile | `registry.get_transform(target, source, t)?` | `registry.lookup(target, source, t)?` |
| compile | `registry.get_transform_at(target, target_time, source, source_time, fixed)?` | `registry.lookup_across(target, target_time, source, source_time, fixed)?`, returning a `CrossTimeTransform` |
| compile | `registry.reparent_frame(tf)?` | `registry.reparent(&tf)?` |
| compile, silent | `registry.remove_transforms_before(t)` | `registry.remove_samples_through(cutoff)`; the cutoff is inclusive, see [Cleanup](#cleanup) |
| compile | `tf.parent()`, `tf.child()`, `tf.timestamp()` | `tf.target()`, `tf.source()`, `tf.stamp()` |
| compile | `Quaternion::identity()` | `Rotation::IDENTITY` |
| compile | `q.rotate_vector(v)` | `q.rotate(v)` |
| compile | `Timestamp::zero()` | `Timestamp::from_nanos(0)` |
| compile | `Vector3::zero()` | `Vector3::new(0.0, 0.0, 0.0)` |

`remove_frame` and `latest_common_time` keep their names. The frame
arguments of `Transform::new` and of the lookups keep their positions:
2.x `parent` is 3.0 `target`, and 2.x `child` is 3.0 `source`.

## Constructing transforms

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `Transform::new(parent, child, translation, quaternion, Stamp::At(t))?` | `Transform::new(target, source, translation, rotation, Stamp::At(t))?` with a `Rotation` |
| compile | `Transform::static_between(parent, child, translation, quaternion)?` | `Transform::new(target, source, translation, rotation, Stamp::Static)?` |
| compile | `tf.validate()?` on a derived transform | Delete the call. Every `Transform` is valid; an operation that cannot produce one returns an error |
| compile | `Transform::interpolate(..)` | Removed. Insert the samples into a `Registry` and look up between them |
| silent | `Transform::new` accepted `""` as a frame name; `add_transform` rejected it with `RegistryError::EmptyFrameName` | `Transform::new` and decoding return `TransformError::EmptyFrameName`. The error moves from the insert call to the constructor, and `insert` cannot report it |
| silent | `Transform::new` reported `NonUnitRotation` for a bad rotation | The rotation is checked earlier, by `Rotation::from_wxyz`. `Transform::new` reports only `EmptyFrameName` and, after it, `NonFinite` for the translation |

```rust
let mount = Transform::new(
    "base",
    "camera",
    Vector3::new(0.1, 0.0, 0.5),
    Rotation::IDENTITY,
    Stamp::Static,
)?;
let sample = Transform::new(
    "map",
    "base",
    Vector3::new(1.0, 0.0, 0.0),
    Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0)?,
    Stamp::At(Timestamp::from_nanos(1_000_000_000)),
)?;
```

## Rotation replaces Quaternion

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `Quaternion::from_wxyz(w, x, y, z)` (infallible, `const`) | `Rotation::from_wxyz(w, x, y, z)?` returns `Result<Rotation, TransformError>`: `NonFinite` for a NaN or infinite component, `NonUnitRotation { norm, .. }` for a norm further than `1e-6` from 1 |
| compile | `q.w`, `q.x`, `q.y`, `q.z` (public fields); `Quaternion { w, x, y, z }` literals | `q.w()`, `q.x()`, `q.y()`, `q.z()`; build with `from_wxyz` |
| compile | `q.normalize()?` before building a transform | Delete it if the input is within `1e-6` of unit length; `from_wxyz` scales it. Normalize further-off input yourself before the call |
| compile | `q.norm()`, `q.conjugate()`, `q1 * q2`, `q1.slerp(q2, f)` | Removed from the public API. Compose and invert through `Transform::compose` and `Transform::inverse`; interpolate through a `Registry` lookup |
| compile | `geometry::UNIT_NORM_TOLERANCE` | Removed. The tolerance is `1e-6`, documented on `Rotation::from_wxyz` |
| silent | An accepted rotation was stored as given | `from_wxyz` stores the components bit for bit only when their squared norm is within `8 * f64::EPSILON` of 1. Any other accepted input is scaled to unit length, so `rotation.w()` can differ from the `w` passed in. A unit quaternion that travelled as `f32` is the usual case |

The argument order is positional and unchecked, as in 2.x: passing
scalar-last components `(x, y, z, w)` builds a different, valid rotation.

## Composing and inverting

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `(a * b)?`, consuming both | `a.compose(&b)?`, borrowing both. Chains read `a.compose(&b)?.compose(&c)?` |
| compile | `TransformError::SameFrameMultiplication` when both operands had the same child | Removed. Such a composition succeeds |
| silent | Unequal dynamic stamps were reported before unrelated frames | `FrameMismatch` is checked first, then `StampMismatch`. Operands that fail both report `FrameMismatch` |
| silent | `a * b` could return `Ok` with an infinite translation or a rotation drifted off unit length | `compose` and `inverse` return `Err(TransformError::NonFinite)` on overflow, and every result's rotation is unit length. Results can differ from 2.x in their last bits |

`a.compose(&b)` requires `a.source() == b.target()` and maps `b.source()`
into `a.target()`. A static operand takes the other's stamp. `inverse`
swaps the frames, keeps the stamp, and can fail only with `NonFinite`.

## Applying a transform to a point

`Point`, `Transformable`, `Localized` and `Registry::get_transform_for` are
removed without a replacement type. Keep the point's frame and stamp in
your own type, look the transform up, and apply it in three lines:

```rust
fn apply(tf: &Transform, p: Vector3) -> Vector3 {
    let r = tf.rotation().rotate(p);
    let t = tf.translation();
    Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z)
}

// 2.x: registry.get_transform_for(&point, "map")? then point.transform(&tf)?
let tf = registry.lookup("map", "sensor", stamp)?;
let p_map = apply(&tf, p_sensor);
```

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `point.transform(&tf)?` checked that the point's frame was the transform's child and that the stamps matched | Nothing checks this. `apply` returns a finite vector for a point from any frame. Compare `tf.source()` with your point's frame and `tf.stamp()` with its stamp yourself |
| compile | A `Point` carried an orientation, rotated along with its position | Compose instead: build the pose as a `Transform` from the sensor frame and use `tf.compose(&pose)?`, or keep the orientation outside the crate |
| compile | `Vector3` operators `+`, `-`, `* f64`, `/ f64` | Removed. Write the components out, as `apply` does, or convert to a linear-algebra crate's vector |

Rotate first, then add the translation. Adding first, dropping the
translation, or using the inverse transform all return plausible numbers.

## Time

`Timestamp` is a signed `i64` nanosecond count from an epoch you choose,
with a range of about 292 years either side of it. `TimePoint`, the
`SystemTime` implementation, `Timestamp::now` and `try_now` are removed:
the crate reads no clock and has one time type.

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `Timestamp::from_nanos(u64)`, `as_nanos() -> u64` | `Timestamp::from_nanos(i64)`, `as_nanos() -> i64`. Convert unsigned counts with `i64::try_from`, never with `as` |
| compile | `Timestamp::now()`, `Timestamp::try_now()` | Read the clock yourself and convert, see below |
| compile | `Registry<SystemTime>`, `Transform<SystemTime>` | Convert each `SystemTime` to a `Timestamp` at the boundary, see below |
| compile | A custom `impl TimePoint` | Convert your clock to nanoseconds at the boundary. Convert the absolute tick count, and widen before multiplying: `ticks * 1_000_000_000` overflows `i64` after 3.3 days at 32,768 Hz |
| compile | `(t + duration)?`, `(t - duration)?` returning `Result<Timestamp, TimeError>` | `t.checked_add(duration)`, `t.checked_sub(duration)` returning `Option<Timestamp>` |
| compile | `(later - earlier)?`, `later.duration_since(earlier)?` returning `Result<Duration, TimeError>` | `later.checked_duration_since(earlier)` returning `Option<Duration>`; `None` iff `earlier` is later than `later` |
| compile | `t.as_seconds()?`, `t.as_seconds_lossy()` | Removed. `Timestamp` implements `Display` as exact decimal seconds (`-0.500000000`). For an `f64`, convert `as_nanos()` yourself and accept the rounding |
| silent | A negative or pre-epoch instant could not be represented, so a sign bug upstream failed at conversion | Negative instants are ordinary and insertable. Check the sign at your boundary if your clock cannot produce one |

Unsigned nanoseconds, as 2.x stored them:

```rust
fn from_u64_nanos(nanos: u64) -> Option<Timestamp> {
    i64::try_from(nanos).ok().map(Timestamp::from_nanos)
}
```

`SystemTime`, with instants before the Unix epoch mapped to negative
nanoseconds:

```rust
use std::time::{SystemTime, UNIX_EPOCH};

fn from_system_time(t: SystemTime) -> Option<Timestamp> {
    let epoch = Timestamp::from_nanos(0);
    match t.duration_since(UNIX_EPOCH) {
        Ok(after) => epoch.checked_add(after),
        Err(before) => epoch.checked_sub(before.duration()),
    }
}

// 2.x `Timestamp::try_now()`:
let now = from_system_time(SystemTime::now());
```

Both return `None` for an instant outside the `i64` nanosecond range.

For a ROS `builtin_interfaces/Time { sec: i32, nanosec: u32 }`, copy the
`from_ros` and `to_ros` functions from the
[`Timestamp` documentation](https://docs.rs/transforms/latest/transforms/struct.Timestamp.html#converting-a-seconds-nanoseconds-stamp).
They accept negative seconds, reject a `nanosec` of a second or more, and
split with Euclidean division. The 2.4 recipe refused negative seconds;
replace it.

Do not do arithmetic on `as_nanos()` by hand.
`(now.as_nanos() - stamp.as_nanos()) as u64` reads a stamp 2 ms ahead of
the clock as an age of 584 years. Use `now.checked_duration_since(stamp)`
and handle `None`.

## Stamp

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `stamp.is_static()` | `matches!(stamp, Stamp::Static)` or `stamp == Stamp::Static` |
| compile | `stamp.at()` returning `Option<T>` | `match stamp { Stamp::At(t) => Some(t), Stamp::Static => None }` |
| compile | `Stamp<T>` | `Stamp`, which now also implements `Hash`. It stays unordered |

A `lookup` result always carries `Stamp::At(at)`.
`latest_common_time` returns `Stamp::Static` for a chain with no dynamic
edge; any instant is then a valid lookup instant:

```rust
let at = match registry.latest_common_time("map", "lidar")? {
    Stamp::At(newest) => newest,
    Stamp::Static => Timestamp::from_nanos(0),
};
let tf = registry.lookup("map", "lidar", at)?;
```

## Registry construction, insertion and re-parenting

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `Registry::new()`, `Registry::default()` | `Registry::unbounded()`. It keeps every sample until removed; prefer `with_max_age` unless you clean up yourself |
| compile | `Registry::with_max_age(d)` | Unchanged name. See [Expiry](#expiry) for its new error |
| compile | `add_transform(tf: Transform) -> Result<(), RegistryError>` | `insert(&Transform) -> Result<(), InsertError>`. The transform is borrowed, so a rejected one stays with you; remove `.clone()` calls made for retries |
| compile | `add_transforms(impl IntoIterator<Item = Transform>) -> Result<(), RegistryError>` | `insert_all(&[Transform]) -> Result<(), BatchError>`. Collect an iterator into a `Vec` first. `BatchError { index, error }` names the rejected member by its slice index |
| compile | `reparent_frame(tf) -> Result<(), RegistryError>` | `reparent(&Transform) -> Result<(), InsertError>`; `seed.source()` moves under `seed.target()` |
| silent | Insertion re-validated the transform and could report `NonUnitRotation`, `NonFiniteValues` or `EmptyFrameName` | `insert` reports none of these. They are construction errors now |

```rust
match registry.insert_all(&batch) {
    Ok(()) => {}
    Err(BatchError { index, error }) => {
        // Nothing from `batch` was stored. `batch[index]` was rejected.
        eprintln!("member {index}: {error}");
    }
}
```

## Errors

`RegistryError<T>`, `TimeError` and `QuaternionError` are gone. Four types
replace them, one per operation family:

- `TransformError`: `Rotation::from_wxyz`, `Transform::new`, `compose`,
  `inverse`, and decoding.
- `InsertError`: `insert` and `reparent`.
- `BatchError`: `insert_all`. It wraps the `InsertError` and the index.
- `LookupError`: `lookup`, `lookup_across`, `latest_common_time`.

All three enums are `#[non_exhaustive]`, and so is every variant with
fields: a pattern needs `..` inside the braces and a wildcard arm, and your
code cannot construct a variant. Tuple variants became struct variants.
Time payloads are exact `Timestamp` values.

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `RegistryError::SelfReferentialFrame` | `InsertError::SelfEdge { frame, .. }` |
| compile | `RegistryError::ReparentingNotSupported { current_parent }` | `InsertError::ParentMismatch { frame, pinned, offered, .. }`; `pinned` is the old `current_parent` |
| compile | `RegistryError::CycleDetected` | `InsertError::Cycle { child, parent, .. }`, the rejected edge |
| compile | `RegistryError::StaticDynamicConflict` | `InsertError::KindMismatch { frame, incoming, .. }`; `incoming` is the rejected transform's stamp, and the frame is of the other kind |
| silent | (none: the insert returned `Ok` and the sample was evicted) | `InsertError::Expired { frame, stamp, newest, .. }`, see [Expiry](#expiry) |
| compile | `RegistryError::NoParentToReplace(frame)` | `InsertError::NoParent { frame, .. }` (`reparent` only) |
| compile | `RegistryError::ParentUnchanged(frame)` | `InsertError::ParentUnchanged { frame, parent, .. }` (`reparent` only) |
| compile | `RegistryError::UnknownFrame(frame)` from `reparent_frame` | `InsertError::UnknownFrame { frame, .. }` |
| compile | `RegistryError::UnknownFrame(frame)` from a query | `LookupError::UnknownFrame { frame, .. }` |
| compile | `RegistryError::Disconnected { target_frame, source_frame }` | `LookupError::Disconnected { a, b, .. }`. From `lookup` and `latest_common_time`, `a` is the target and `b` the source. From `lookup_across`, `a` is the endpoint whose leg does not connect and `b` is the fixed frame |
| compile | `RegistryError::NotFoundAt { covered: None, frame, .. }` | `LookupError::NoData { frame, .. }` |
| compile | `RegistryError::NotFoundAt { covered: Some((start, end)), frame, requested, .. }` | `LookupError::OutOfRange { frame, requested, earliest, latest, .. }` |
| compile | `RegistryError::NoCommonTime { covered: None, frame, .. }` | `LookupError::NoData { frame, .. }` |
| compile | `RegistryError::NoCommonTime { covered: Some((start, end)), frame, .. }` | `LookupError::NoCommonTime { frame, earliest, latest, latest_possible, .. }`; `latest_possible` is the oldest of the newest samples on the chain |
| compile | `target_frame` and `source_frame` fields on `NotFoundAt` and `NoCommonTime` | Removed. You passed them in; keep them if you log them |
| compile | `RegistryError::NonFiniteValues` from a lookup | `LookupError::NonFinite` |
| compile | `RegistryError::NonUnitRotation(norm)` from a lookup | `LookupError::NonUnitRotation { norm, .. }`; no input is known to reach it |
| compile | `RegistryError::NonFiniteValues`, `NonUnitRotation(norm)`, `EmptyFrameName` from an insert | Not reported by `insert`. See the `TransformError` rows |
| compile | `RegistryError::TransformError(e)` | Removed. No registry error wraps a `TransformError`; the time-arithmetic failures it carried for custom clocks cannot occur |
| compile | `TransformError::NonFiniteValues` | `TransformError::NonFinite` |
| compile | `TransformError::NonUnitRotation(norm)` | `TransformError::NonUnitRotation { norm, .. }` |
| silent | (none: constructors accepted `""`) | `TransformError::EmptyFrameName` |
| compile | `TransformError::IncompatibleFrames { expected, found }` | `TransformError::FrameMismatch { expected, found, .. }` |
| compile | `TransformError::TimestampMismatch { lhs: f64, rhs: f64 }` (lossy seconds) | `TransformError::StampMismatch { lhs, rhs, .. }` with `Timestamp` payloads |
| compile | `TransformError::SameFrameMultiplication { frame }` | Removed; the composition succeeds |
| compile | `TransformError::StaticInterpolation`, `TimestampOutOfRange { .. }` | Removed with `Transform::interpolate` |
| compile | `TransformError::TimestampError(TimeError)` | Removed |
| compile | `TransformError::QuaternionError(QuaternionError)` | Removed |
| compile | `QuaternionError::NonFinite` | `TransformError::NonFinite` from `Rotation::from_wxyz` |
| compile | `QuaternionError::ZeroLengthNormalization` | `TransformError::NonUnitRotation { norm, .. }` from `Rotation::from_wxyz`, with `norm == 0.0` |
| compile | `TimeError::DurationUnderflow`, `DurationOverflow` | `None` from `checked_add`, `checked_sub`, `checked_duration_since` |
| compile | `TimeError::AccuracyLoss` | Removed with `as_seconds` |

A lookup error handler, ported:

```rust
fn describe(error: &LookupError) -> &'static str {
    match error {
        LookupError::UnknownFrame { .. } => "a name is not in the tree",
        LookupError::Disconnected { .. } => "the frames are in different trees",
        // Not a timing problem: no instant can be served.
        LookupError::NoData { .. } => "an edge holds no samples",
        LookupError::OutOfRange { requested, latest, .. } if requested > latest => {
            "wait for newer data"
        }
        LookupError::OutOfRange { .. } => "older than the retained history",
        _ => "numerical failure",
    }
}
```

## Lookup behaviour

| Tag | 2.x | 3.0 |
|---|---|---|
| silent | `get_transform("x", "x", t)` returned the identity for a name the registry had never seen; `latest_common_time("x", "x")` returned `Stamp::Static` | Both return `LookupError::UnknownFrame`. A name is known only while a stored edge names it as child or parent. An empty registry answers every query with `UnknownFrame`. A one-frame system must short-circuit `target == source` itself |
| silent | `get_transform_at` with both endpoints equal to the fixed frame returned the identity for any name | `lookup_across` requires `target`, `source` and `fixed` to be known |
| silent | A lookup between frames in different trees could report `NotFoundAt` for an edge met while walking to a root | It reports `Disconnected`. The order is fixed: unknown name, then `Disconnected`, then `NoData` or `OutOfRange` for an edge of the connecting chain, then a numerical failure |
| silent | With several unknown names, `get_transform_at` named the fixed frame or the source first | The first unknown name in the order target, source, fixed is reported |
| silent | A lookup toward an ancestor could return `Ok` with an infinite translation, while the opposite direction could fail | Both directions return `Err(LookupError::NonFinite)` |
| silent | Interpolated, inverted and multi-hop results | Can differ in their last bits: rotations are held to unit length, `inverse` uses the conjugate, and the interpolation factor comes from integer nanoseconds. A lookup across one edge toward the parent at a stored instant still returns the stored numbers bit for bit |
| silent | An empty name in a query was `UnknownFrame("")` against a known frame and the identity against itself | Always `UnknownFrame`; nothing can store an empty name |

## Cross-time lookups

| Tag | 2.x | 3.0 |
|---|---|---|
| compile | `get_transform_at(..)` returned a `Transform` stamped with the target time, which could be inserted or composed by mistake | `lookup_across(..)` returns a `CrossTimeTransform` with `target()`, `target_time()`, `source()`, `source_time()`, `translation()` and `rotation()`. It cannot be inserted, composed or serialized |
| compile | `tf.timestamp()` on the result | `across.target_time()`; the source time you kept separately is `across.source_time()` |

Apply it to a point with the same three lines, on `across.rotation()` and
`across.translation()`. The fixed frame is not recorded in the result. Both
legs are always resolved against it, so a fixed frame that is not connected
to the endpoints is `Disconnected`, also when the endpoints and times
coincide.

## Cleanup

| Tag | 2.x | 3.0 |
|---|---|---|
| silent | `remove_transforms_before(t)` removed samples strictly older than `t` and kept a sample at `t` | `remove_samples_through(cutoff)` removes samples at or before `cutoff`. Porting the call by renaming it removes one more sample per frame than before |

To keep the strict behaviour, step the cutoff back by one nanosecond:

```rust
// 2.x: registry.remove_transforms_before(t);
if let Some(cutoff) = t.checked_sub(Duration::from_nanos(1)) {
    registry.remove_samples_through(cutoff);
}
```

`checked_sub` returns `None` only at the earliest representable instant,
where nothing is strictly older. Static transforms, parents and kinds are
kept, as before. `remove_samples_through(Timestamp::MAX)` clears all
dynamic history, which no 2.x cutoff could do for a sample at the largest
timestamp. A drained frame reports `LookupError::NoData` and can
restart at any instant.

## Expiry

| Tag | 2.x | 3.0 |
|---|---|---|
| silent | On a `with_max_age` registry, an insert older than the frame's newest timestamp minus `max_age` returned `Ok(())` and was evicted in the same call | The insert returns `InsertError::Expired { frame, stamp, newest }` and stores nothing. Stored state after an accepted insert is unchanged. Nothing stops compiling. An insert loop that uses `?` now stops at the first late sample, and `insert_all` rejects the whole batch for one such member, where 2.3 and 2.4 stored the rest. The sample that moves a frame's newest timestamp ahead is still accepted; `remove_frame` discards that timestamp |

A sample exactly on the boundary is accepted. `Registry::unbounded`, a
drained frame and a `reparent` seed never report `Expired`. To keep the
fresh members of a batch, insert one at a time and decide per error:

```rust
fn insert_each(registry: &mut Registry, batch: &[Transform]) -> Result<(), InsertError> {
    for transform in batch {
        match registry.insert(transform) {
            // A late sample: skip it and keep the rest.
            Ok(()) | Err(InsertError::Expired { .. }) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(())
}
```

`Expired` does not say which stamp is wrong: a late sample and a frame
whose newest stamp ran ahead report it alike. If the stream's clock stepped
back, `remove_frame(frame)` discards that frame's samples and its newest
stamp and releases its parent and kind until its next insert.
`remove_samples_through(newest)` drains that frame and keeps its parent and
kind, and also removes every dynamic sample at or before `newest` from
every other frame.

## Serde

The 3.0 layout is new and is not meant to read 2.x data.

```json
{
  "target": "map",
  "source": "base",
  "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
  "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
  "stamp": { "At": 1000000000 }
}
```

| Tag | 2.x | 3.0 |
|---|---|---|
| silent | `Transform` fields in the order `translation`, `rotation`, `timestamp`, `parent`, `child` | `target`, `source`, `translation`, `rotation`, `stamp`, in that order. The order is part of the binary wire contract |
| silent | Unknown JSON fields in a `Transform`, `Vector3` or `Quaternion` were ignored | `Vector3`, `Rotation` and `Transform` reject unknown fields, so a document carrying both `stamp` and `timestamp` is an error |
| silent | `Timestamp` was a bare `u64` | A bare `i64` |
| silent | A decoded rotation was validated and stored as given; an empty frame name decoded | `Rotation` decodes through `from_wxyz` (checked and scaled to unit length), `Transform` through `Transform::new` (empty names rejected). A standalone `Vector3` decodes unchecked |
| compile | `Point` and `Quaternion` were serializable | `Point` is gone; `Rotation` writes the same `w`, `x`, `y`, `z` fields. `CrossTimeTransform` has no serde support |

`Stamp` keeps its tags: `"Static"` and `{"At": t}`, variant index 0 and 1.
A missing or `null` stamp is still a decode error.

Migrating persisted data:

- A 2.x JSON `Transform` fails to decode in 3.0 on its field names. A 2.x
  postcard `Transform` usually fails as well, and the frozen 2.4 fixtures
  do, but nothing guarantees it: a 2.x payload whose bytes happen to form
  a valid 3.0 transform decodes without error, with wrong frames, geometry
  and stamp. The format carries no version, so tell 2.x data from 3.0 data
  by something outside the payload, for whole transforms too.
- `decode(encode(t)) == t` holds for a codec that preserves every `f64`.
  JSON text needs a correctly rounding parser: `serde_json` reads some
  17-digit decimals one ulp off unless its `float_roundtrip` feature is
  on.
- **A standalone 2.x `Timestamp` or `Stamp` in a binary format decodes
  without error at a wrong instant.** Postcard writes `u64` as a plain
  varint and `i64` zigzag-encoded, so a 2.x `1` reads back as `-1` ns and
  `1_700_000_000_000_000_000` as `850_000_000_000_000_000`. Nothing in the
  payload tells the two apart.
- Decode persisted 2.x data with the 2.x crate, convert it (names,
  `i64::try_from` on the nanoseconds, `Rotation::from_wxyz` on the
  rotation), and re-encode it with 3.0. Do not try both decoders in turn.
- Carry a schema version outside the payload.

# Migrating to 2.4

This part uses 2.x names. From 2.0.0, one behaviour change applies: since
2.4.0 the registry refuses a transform whose parent or child frame name is
empty (see "Empty frame names" below). Nothing else needs migrating. From
1.x or a 2.0 pre-release, apply the changes below.

## Coming from 2.0.0

- `Registry::latest_common_time(target, source)` replaces retries based on
  `NotFoundAt` coverage. Use its returned instant in `get_transform`, under
  the same read guard if shared. `Stamp::Static` means the caller can choose
  any instant. Handle the new `RegistryError::NoCommonTime` variant when
  diagnosing empty or disjoint coverage.
- Since 2.1.2, `latest_common_time` checks custom-clock arithmetic needed to
  interpolate at its answer. It returns a timestamp error if that arithmetic
  fails, without searching earlier instants. Geometry is checked only during
  the subsequent lookup, subject to its numeric limitations.
- `Registry`'s `Debug` output summarizes buffers. Do not parse or snapshot
  its text as a stable interface.
- Since 2.2.0, `Registry::reparent_frame(transform)` moves the transform's
  child under its parent atomically, seeding the new edge with that transform
  and dropping the child's stored history; its kind and `max_age` policy are
  kept. Handle the new `RegistryError::NoParentToReplace` (the child is a
  root) and `RegistryError::ParentUnchanged` (the parent is already the
  requested one) variants. `ReparentingNotSupported`'s message text changed.
- Since 2.2.0, a `NotFoundAt` between connected frames names an edge the
  connecting chain crosses. Earlier releases could name a failing edge above
  the common ancestor, which the lookup never needed.
- Since 2.2.0, a lookup between frames no chain connects reports
  `UnknownFrame`, `Disconnected` or `NotFoundAt` even when the part of the
  tree it walked overflows. Earlier releases could report `NonFiniteValues`
  there, depending on the argument order.
- Since 2.3.0, `Registry::add_transforms(transforms)` inserts a batch
  all-or-nothing: on error the registry is unchanged and the error is the
  first one `add_transform` would report. It replaces hand-written rollback
  around an `add_transform` loop; keep the loop to store the valid
  transforms of a batch.

### Empty frame names

Since 2.4.0, `add_transform` and `add_transforms` reject a transform whose
parent or child is `""` with `RegistryError::EmptyFrameName`, as does
`reparent_frame` for an empty new parent. Earlier releases stored it, so
publishers that each left a name unset shared one frame named `""`, and
lookups between their unrelated trees succeeded. Check names where
transforms are ingested: an unset ROS `frame_id` arrives as `""`.
Constructors and deserialization still accept `""`.

## Coming from 1.x

The following API changes landed in 2.0.0. Code fragments show replacements;
use the imports and surrounding error handling appropriate to your application.

### Construction and insertion

| Old API | Replacement |
|---|---|
| `Registry::new(max_age)` | `Registry::with_max_age(max_age)` |
| `registry.add_transform(tf);` | `registry.add_transform(tf)?;` |
| `Transform { ... }` | `Transform::new(parent, child, translation, rotation, Stamp::At(t))?` |
| Static transform with `timestamp: Timestamp::zero()` | `Transform::static_between(parent, child, translation, rotation)?` |
| `Point { position, orientation, timestamp, frame }` | `Point::new(position, orientation, timestamp, frame)` |
| `Timestamp { t }` | `Timestamp::from_nanos(t)`; narrow wider values with `u64::try_from` |
| `Transform::identity()` | Construct an identity with explicit frames, or query a frame relative to itself |

**Keep `max_age` when migrating.** Removing the constructor argument as rustc
suggests produces `Registry::new()`, which retains history indefinitely.
Both constructors now work with and without `std`.

`Transform` fields are private. Read them with `translation()`, `rotation()`,
`timestamp()`, `parent()`, and `child()`; to change a component, construct a
new transform. `Point` keeps its public fields, but is `#[non_exhaustive]`.
`Vector3` and `Quaternion` retain public fields.

Transform constructors reject non-finite values and rotations outside
`geometry::UNIT_NORM_TOLERANCE`. Insertion validates again because derived
transforms can drift or overflow. It also rejects cycles, self-parenting,
re-parenting, and mixing static and dynamic samples under one child.
Handle the returned `Result`: a rejected insert stores nothing.

### Static transforms and clocks

`Transform::timestamp()` returns `Stamp<T>`: `At(t)` for dynamic samples,
`Static` for all-time validity. Every timestamp, including zero, is ordinary
dynamic data. Migrating a static publisher to `Stamp::At(Timestamp::zero())`
would cover only that one instant; use `Transform::static_between` instead.

`Timestamp` now stores private `u64` nanoseconds, spanning about 584 years
from the chosen epoch. Use `as_nanos()` instead of `.t`. For a Unix-epoch
clock the range ends in 2554. Arithmetic rejects values outside that range;
`Timestamp::try_now()` handles system-clock range errors without panicking.

Custom `TimePoint` implementations must implement `Copy + Ord + Debug` and:

- `duration_since(self, earlier) -> Result<Duration, TimeError>`
- `checked_sub(self, duration) -> Result<Self, TimeError>`
- `as_seconds_lossy(self) -> f64`

Remove `static_timestamp`, `is_static`, `checked_add`, and `as_seconds`
from trait implementations. Ordering and arithmetic must agree. Diagnostic
conversion must be infallible; return `NaN` for unconvertible instants.

### Imports and renames

Use the public re-exports:

```rust
use transforms::{Registry, Transform, Transformable, Localized};
use transforms::geometry::{Point, Quaternion, Vector3, UNIT_NORM_TOLERANCE};
use transforms::time::{Stamp, TimePoint, Timestamp};
use transforms::errors::{RegistryError, TransformError, QuaternionError, TimeError};
```

`core`, `Buffer`, `registry.data`, and leaf modules such as
`geometry::transform` and `time::timestamp` are private. Replace standalone
buffers with a `Registry`; maintain application bookkeeping outside it.

| Old name | Replacement |
|---|---|
| `delete_transforms_before` | `remove_transforms_before` |
| `Timestamp::as_seconds_unchecked` | `Timestamp::as_seconds_lossy` |
| `TimestampError` | `TimeError` |
| `Quaternion::new(w, x, y, z)` (pre-release) | `Quaternion::from_wxyz(w, x, y, z)` |
| `Transform::UNIT_NORM_TOLERANCE` (pre-release) | `geometry::UNIT_NORM_TOLERANCE` |

### Errors

All fallible registry operations return `RegistryError<T>`. `TransformError`
now describes geometry and time operations only; `BufferError` is private
implementation history with no replacement public type.

| Error | Meaning |
|---|---|
| `UnknownFrame(frame)` | An endpoint exists nowhere in the tree |
| `Disconnected { .. }` | Known endpoints have no connecting chain |
| `NotFoundAt { frame, requested, covered, .. }` | A sampled edge cannot serve the request; `covered: None` means it holds no samples |
| `NoCommonTime { frame, covered, .. }` | Connecting edges have empty or disjoint coverage |
| `NonUnitRotation`, `NonFiniteValues` | Numeric validation failed |
| `EmptyFrameName`, `SelfReferentialFrame`, `ReparentingNotSupported`, `CycleDetected`, `StaticDynamicConflict` | An insert or `reparent_frame` violated a topology or kind constraint |
| `NoParentToReplace(frame)`, `ParentUnchanged(frame)` | `reparent_frame` refused: the frame is a root, or already has that parent |
| `TransformError(error)` | A geometry or time operation failed |

`requested` and `covered` retain your time type `T`. With `Some((start, end))`,
a request after `end` needs newer data; one before `start` needs retained
older data. With `None`, changing the requested time cannot help until data
is inserted. A `NotFoundAt` can take precedence over a simultaneous
`Disconnected` error.

`RegistryError::TransformError` never wraps `NonUnitRotation` or
`NonFiniteValues` on registry return paths; those use the flat variants.
`NotFoundAt` no longer carries an error source; inspect its fields.
For custom clocks, `RegistryError<T>` inherits `Send`, `Sync`, and lifetime
constraints from `T`.

All public error enums are `#[non_exhaustive]`; matches need a wildcard arm.
The old `NotFound`, `TransformTreeEmpty`, and `BufferError::MaxAgeInvalid`
variants are removed. `TimestampMismatch { lhs, rhs }`,
`IncompatibleFrames { expected, found }`, and
`SameFrameMultiplication { frame }` now have named fields.
Match variants and payloads, not formatted messages.

### Geometry operations

`==` now compares components exactly. For tolerant component comparisons use
`approx` 0.5 (`AbsDiffEq` or `RelativeEq`); frame and timestamp metadata still
must match exactly. `Transform` no longer implements `Eq`, and `Quaternion`,
`Vector3`, and `Point` no longer implement `PartialOrd`. Quaternion component
comparisons do not account for opposite signs representing the same rotation.

| Removed API | Replacement |
|---|---|
| Quaternion `+`, `-`, `scale` for blending | `q1.slerp(q2, factor)` |
| Quaternion `/` | For unit quaternions, `q2 * q1.conjugate()` |
| `Quaternion::norm_squared()` | Sum the squared components |
| `Quaternion::default()` | `Quaternion::identity()` |
| `Vector3::dot`, `cross` | Compute from the public components or use a linear-algebra crate |
| `Vector3::unit_x/y/z` | `Vector3::new(1.0, 0.0, 0.0)` and the corresponding axes |

`QuaternionError::DivisionByZero` is removed with quaternion division.
`from_wxyz` does not normalize; use `normalize()` when necessary.

### Runtime changes and limitations

- A child's parent and kind stay pinned until `remove_frame` removes its
  incoming edge; since 2.2.0 `reparent_frame` replaces the parent pin and
  keeps the kind. To re-parent a subtree, remove and re-add only its root's
  edge, or call `reparent_frame` on the root. Descendants with unchanged
  immediate parents retain their history.
- `remove_transforms_before` preserves static transforms and frame pins.
  Drained frames report `NotFoundAt { covered: None, .. }`; a fully drained
  buffer resets its expiry reference, allowing earlier timestamps again.
- Static and dynamic edges may form one chain, but their samples cannot
  share a child buffer. Duplicate timestamps remain last-write-wins.
- Same-frame lookups return identity, even for unregistered names.
  `get_transform_at` also accepts endpoints equal to its fixed frame.
- Lookup results carry the requested timestamp, including static chains.
  `get_transform_at` retains only the target time. For differing instants,
  keep both times and apply geometry explicitly; do not use ordinary
  `Transformable` application, composition, or reinsertion. See its
  [contract](https://docs.rs/transforms/latest/transforms/struct.Registry.html#method.get_transform_at).
- Lookups and `Transform::interpolate` do not extrapolate. `Quaternion::slerp`
  clamps its factor to `[0, 1]`. Interpolation spans any retained sample gap.
- Derived transforms are not re-validated. An ancestor-directed lookup can
  return an infinite translation as `Ok`; inversion checks finiteness, so
  the opposite direction may fail. Use `Transform::validate` when needed.
- Composition requires `lhs.child() == rhs.parent()` and equal timestamps
  unless an operand is static. It also rejects equal child frames, including
  a right-hand identity, with `SameFrameMultiplication`.
- Math now uses `libm` in both feature modes. Recorded 1.x floating-point
  results can differ by a few ulps. Feature-mode tests do not establish
  bitwise equality across devices. Single-hop ancestor lookups at stored
  timestamps preserve the stored geometry without inversion.

`get_transform(target, source, time)` retains the old positional argument
order despite renaming `from/to`: `get_transform("map", "lidar", t)` maps
lidar coordinates into map, as with tf2's target/source convention.

## Coming from a 2.0.0 pre-release

Apply the relevant sections above: stable 2.0.0 changed the pre-release API,
including private fields/modules, fallible construction, `Stamp`, narrowed
`Timestamp`, and registry errors. Re-encode persisted payloads as described
below. Earlier pre-release changes are recorded in the
[beta.4 changelog](https://github.com/deniz-hofmeister/transforms/blob/v2.0.0-beta.4/CHANGELOG.md).

## Serde wire format (2.x)

1.x had no serde feature. Pre-release payloads used a different timestamp
shape. Version persisted streams and re-encode them instead of assuming
cross-version compatibility.

`Timestamp` is a bare `u64` nanosecond count. `Stamp` is explicitly tagged:

```json
{
  "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
  "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
  "timestamp": { "At": 1000000000 },
  "parent": "map",
  "child": "base"
}
```

Static transforms use `"timestamp": "Static"`. Missing or null transform
stamps are decode errors. Deserialization validates numeric components;
serialization does not validate derived results.

Struct field order and stamp variant order (`Static` first, `At` second)
are part of the binary wire contract. Integer encoding and endianness belong
to the codec and its configuration; different codecs are not interchangeable.

A 1.x-shaped payload does not decode silently as 2.x data, with one
exception: a 1.x postcard payload stamped exactly `t = 0` (the 1.x static
sentinel) decodes cleanly as `Stamp::Static`, because its single `0x00`
byte reads as variant 0. That is right for a 1.x static publisher and wrong
for a genuine boot-relative `t = 0` sample. Do not rely on it in place of a
version tag.
