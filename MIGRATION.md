# Migrating to 2.2

From 2.0.0, no API migration is required. From 1.x or a 2.0 pre-release,
apply the changes below. The current API is documented on
[docs.rs](https://docs.rs/transforms); release history is in
[CHANGELOG.md](CHANGELOG.md).

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
| `SelfReferentialFrame`, `ReparentingNotSupported`, `CycleDetected`, `StaticDynamicConflict` | Insertion violated a topology or kind constraint |
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

## Serde wire format

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
