# Transforms

> [!WARNING]
> This README describes v3. If you use v2.4, see the
> [v2.4.0 source code and README](https://github.com/deniz-hofmeister/transforms/tree/v2.4.0).

[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![no_std](https://img.shields.io/badge/no__std-compatible-brightgreen.svg)](https://docs.rust-embedded.org/book/)
[![tests](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml)

A Rust coordinate transform library inspired by ROS2 tf2, for robotics and
computer vision. It provides familiar frame trees, timestamped buffers,
transform chaining, and interpolation, with a Rust API and no middleware
dependency. It forbids unsafe code and is `no_std` with `alloc`.

[API reference](https://docs.rs/transforms/3.0.0) · [Migration guide](MIGRATION.md) ·
[Changelog](CHANGELOG.md)

## Installation

Requires Rust 1.85 or later.

```toml
[dependencies]
transforms = "3.0.0"
```

The crate is always `no_std` and needs a heap allocator (`alloc`). There is
no `std` feature and nothing to disable for embedded targets. Its runtime
dependencies are `thiserror`, `hashbrown`, `libm`, and the optional `serde`.

| Feature | Default | Effect |
|---|---|---|
| `serde` | No | Adds serialization and deserialization for `Vector3`, `Rotation`, `Timestamp`, `Stamp` and `Transform` |

## Quick start

```rust
use core::time::Duration;
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

let mut registry = Registry::with_max_age(Duration::from_secs(60));
let at = Timestamp::from_nanos(1_000_000_000);

// The sensor origin is one unit along the base frame's X axis.
registry.insert(&Transform::new(
    "base", "sensor",
    Vector3::new(1.0, 0.0, 0.0),
    Rotation::IDENTITY,
    Stamp::At(at),
).unwrap()).unwrap();

// Sensor coordinates expressed in base.
let tf = registry.lookup("base", "sensor", at).unwrap();
assert_eq!(tf.target(), "base");
assert_eq!(tf.source(), "sensor");

// A point measured in the sensor frame, expressed in base.
let p = Vector3::new(2.0, 0.0, 0.0);
let r = tf.rotation().rotate(p);
let t = tf.translation();
let p_target = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
assert_eq!(p_target, Vector3::new(3.0, 0.0, 0.0));
```

The crate does not apply a transform to a point. The last three lines are
the whole recipe: rotate, then add the translation. Translating first,
dropping the translation and applying the transform in the wrong direction
all return an ordinary finite vector that no check can tell from the right
one. See [Transform](https://docs.rs/transforms/3.0.0/transforms/struct.Transform.html#applying-a-transform-to-a-point).

## Conventions and usage

- `Transform::new(target, source, ...)` maps **source coordinates into
  target**. Inserted into a registry, the target is the parent and the
  source is the child. `lookup(target, source, at)` returns that mapping
  between any two connected frames.
- Coordinates and rotation components are `f64`. Use consistent distance
  units and frame axes; the crate does not assign them. `Rotation` is a unit
  quaternion in `(w, x, y, z)` order. `Rotation::from_wxyz` accepts a norm
  within `1e-6` of 1 and scales the components to unit length. See
  [Rotation](https://docs.rs/transforms/3.0.0/transforms/struct.Rotation.html).
- `Timestamp` is a signed 64-bit nanosecond count from an epoch the caller
  chooses. Zero and negative instants are ordinary. The crate reads no
  clock.
- `Stamp::At(t)` is a dynamic sample; `Stamp::Static` is valid for all time
  and is the stamp for a sensor mount.
- Each child has one parent and one kind (static or dynamic), set at its
  first insertion. Cycles are rejected. Inserting a sample at a stored
  instant replaces it.
- `Registry::insert_all` stores a batch all-or-nothing: one rejected member
  leaves the registry unchanged, and the error carries its index. Insert
  one at a time to keep the valid members of a batch.
- `Registry::reparent` moves a child under a new parent atomically,
  dropping that child's stored history; descendants keep theirs. Use
  `remove_frame` and re-insertion to change a kind or keep history.
- `Registry::with_max_age` keeps a window behind each child's newest
  inserted stamp, never wall-clock time. An insert older than the window
  fails with `InsertError::Expired` and stores nothing. The window limits
  history duration, not sample or frame count. `Registry::unbounded`
  retains data until manual removal. Interpolation spans any retained gap;
  no extrapolation is performed.
- `latest_common_time` returns the newest commonly covered instant, or
  `Stamp::Static` for a chain with no dynamic edge. Use that instant for a
  subsequent lookup. For shared registries, keep both calls under one read
  guard. The lookup can still fail numerically.
- `lookup_across` relates two frames at two instants through a fixed frame
  and returns a `CrossTimeTransform`, which carries both instants and
  cannot be inserted or composed. The caller asserts that the fixed frame
  is stationary between the two instants. See its
  [contract](https://docs.rs/transforms/3.0.0/transforms/struct.Registry.html#method.lookup_across).
- A name is known to the registry while a stored edge names it as child or
  parent. Every name in a query must be known, also when target and source
  are equal: an empty registry answers every query with
  `LookupError::UnknownFrame`.

Every `Rotation` is a unit quaternion and every `Transform` has a finite
translation and non-empty frame names, whether it was built, decoded,
composed, inverted or looked up. An operation that cannot produce such a
value returns an error, so no result needs validating.

Geometry equality is exact and compares components; opposite quaternion
signs describe the same rotation without comparing equal. The crate's types
implement no `approx` traits; compare components in tests. Error variants
and payloads are stable; `Display` and `Debug` text are not.

See [Registry](https://docs.rs/transforms/3.0.0/transforms/struct.Registry.html)
for lookup errors, removal semantics, and concurrency.

## Relationship with ROS2 tf2

Like [tf2](https://github.com/ros2/geometry2/blob/rolling/tf2/README.md),
`transforms` resolves coordinates through a tree of buffered frame
relationships, including static transforms and interpolation. The lookup
argument order is also target first, source second.

In ROS2, tf2 provides the transform core and `tf2_ros` adds communication
and waiting facilities. This crate supplies local computation and storage;
applications provide transport, synchronization, and waiting. It is a Rust
implementation, not a binding to tf2, and does not aim for API parity.

| Aspect | ROS2 tf2 / tf2_ros | transforms |
|---|---|---|
| API | C++ and Python APIs | Rust types and `Result` |
| Lookup | `lookupTransform(target, source, time)` | `lookup(target, source, at)` |
| Latest data | A zero lookup time requests the latest transform | Zero is an ordinary instant; use `latest_common_time` |
| Retention | 10-second default; dynamic cache pruned on insert | `unbounded` retains indefinitely; `with_max_age` prunes on insert and rejects samples older than its window |
| Communication | `tf2_ros` broadcasts and listens through ROS2 | No ROS2, DDS, or other transport dependency |
| Waiting | ROS2 buffer supports waiting for transforms | Synchronous queries; caller supplies waiting |
| Embedded use | ROS2 integration uses a hosted runtime | `no_std + alloc`, with `f64` geometry |
| Re-parenting | Parent stored per sample; a new parent silently re-parents | One parent per frame; explicit `reparent` drops that frame's history |

The [tf2 cache implementation](https://github.com/ros2/geometry2/blob/rolling/tf2/src/cache.cpp)
details zero-time lookup and pruning. For ROS integration in Rust, see
[roslibrust_transforms](https://docs.rs/roslibrust_transforms) and
[transforms_io](https://docs.rs/transforms_io); check their supported versions
and middleware separately.

## Performance and embedded use

Time lookup is O(log n) in samples per frame; multi-hop lookup also depends
on chain depth. Only the edges between the two frames and their common
ancestor are sampled. A lookup that fails on coverage or arithmetic walks that chain a second
time to diagnose the cause. A successful lookup allocates the two frame names of its
result; an error allocates the names it reports. The heap must hold the
retained samples, the frame names, and those results.

CI builds for Cortex-M4F/M7, Cortex-M0+, Cortex-M33, and RISC-V targets.
These builds establish compilation, not board latency or memory bounds.
As built there, `f64` arithmetic is software floating point on all of them;
it is hardware only on a Cortex-M7 with the double-precision FPU, built with
`target-cpu=cortex-m7`.
Measure the intended allocator, board, and workload before setting a budget.
`libm` supplies the square root and the trigonometry on every target; tests pin selected interpolation results
on the test host, without guaranteeing cross-device replay.

Run `cargo bench` for benchmarks.

## What's new

3.0.0 changes the public API and the serde wire format. When migrating from
2.x, update the following:

- The crate root is flat, and the names are shorter: `insert`, `insert_all`,
  `lookup`, `lookup_across`, `reparent`, `remove_samples_through`.
- `Rotation` replaces `Quaternion` and is always a unit quaternion.
  `Transform` is always finite, and `compose` and `inverse` replace `*`.
- `Timestamp` is one concrete type of signed nanoseconds. `TimePoint`, the
  `SystemTime` support, the `std` feature and `Timestamp::now` are gone.
- Errors are split by operation: `TransformError`, `InsertError`,
  `BatchError` and `LookupError`.
- `Point`, `Transformable` and `Localized` are removed; a transform is
  applied to a point with the recipe above.
- Several behaviours change without a compile error: a sample older than
  the retention window is rejected with `InsertError::Expired`, the cleanup
  cutoff is inclusive, and a query for a name the registry does not know is
  `UnknownFrame` also when both names are equal.

The [migration guide](MIGRATION.md) lists every change with its
replacement, and the [changelog](CHANGELOG.md) has the full account and the
earlier releases.

## Examples

Each example asserts its results. They are hosted programs; the bare-metal
builds are checked separately.

| Example | Task |
|---|---|
| [basic](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/basic.rs) | Insert, look up across a static and an interpolated edge, apply the result to a point |
| [ingest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/ingest.rs) | Feed a registry from a message stream: checked stamps, a decision per insert error, clock-jump recovery, lookup errors told apart |
| [shared_latest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/shared_latest.rs) | A writer and a reader thread sharing an `RwLock`; look up the newest servable instant |
| [cross_time](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/cross_time.rs) | Coordinates expressed between different instants |

```sh
cargo run --example basic
```

## Non-Goals

The following are outside this crate's scope:

- Scaling transformations
- Skew transformations
- Perspective transformations
- Non-rigid transformations
- Affine transformations beyond rigid body motion
- API parity with ROS2 tf2
- Non-linear interpolation
- Extrapolation
- f32 or mixed-precision arithmetic (every coordinate and rotation is f64)

## AI-Assisted Development

This project uses AI assistance. Since 2.0.0, assisted commits require an
`Assisted-by: <Agent>:<model>` trailer, and the maintainer reviews changes
before merging them. Review is not a warranty: the software is provided
"as is" under the [MIT license](LICENSE), and assessing its fitness for any
use, safety-related ones included, rests with the user. See
[AGENTS.md](AGENTS.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and verification gate,
and [SECURITY.md](SECURITY.md) for vulnerability reporting.

## License

[MIT](LICENSE).
