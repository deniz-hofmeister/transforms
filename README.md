# transforms

**Where was the camera in the map when it took this image?** `transforms`
keeps a history of how coordinate frames move relative to each other and
answers that question for any two connected frames, at any instant its data
covers. It follows the frame tree, interpolates between samples, and returns
an error rather than extrapolate. For robotics and computer vision in Rust:
`no_std`, no unsafe code, and no ROS or other middleware.

[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![tests](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![no_std](https://img.shields.io/badge/no__std-compatible-brightgreen.svg)](https://docs.rust-embedded.org/book/)

[API reference](https://docs.rs/transforms/3.0.0) · [Examples](#examples) ·
[Coming from tf2](#coming-from-ros2-tf2) · [Migration guide](MIGRATION.md) ·
[Changelog](CHANGELOG.md)

This README describes 3.0. For 2.4, see the
[v2.4.0 source code and README](https://github.com/deniz-hofmeister/transforms/tree/v2.4.0).

## Quick start

A robot base drives through a map and carries a lidar. Where was the lidar
in the map at a given moment?

```text
map ──(moving)──> base ──(fixed mount)──> lidar
```

```rust
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

let mut registry = Registry::unbounded();

// The lidar is mounted 0.5 above the base: static, valid for all time.
registry.insert(&Transform::new(
    "base", "lidar",
    Vector3::new(0.0, 0.0, 0.5),
    Rotation::IDENTITY,
    Stamp::Static,
).unwrap()).unwrap();

// The base drives along the map's x axis: one pose per second.
for (seconds, x) in [(1, 0.0), (2, 1.0)] {
    registry.insert(&Transform::new(
        "map", "base",
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(Timestamp::from_nanos(seconds * 1_000_000_000)),
    ).unwrap()).unwrap();
}

// The lidar in the map at t = 1.5 s: two hops, one of them interpolated.
let at = Timestamp::from_nanos(1_500_000_000);
let map_from_lidar = registry.lookup("map", "lidar", at).unwrap();
assert_eq!(map_from_lidar.translation(), Vector3::new(0.5, 0.0, 0.5));

// A point the lidar measured 2.0 ahead of itself, expressed in the map:
// rotate, then add the translation.
let p = Vector3::new(2.0, 0.0, 0.0);
let r = map_from_lidar.rotation().rotate(p);
let t = map_from_lidar.translation();
let p_map = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
assert_eq!(p_map, Vector3::new(2.5, 0.0, 0.5));
```

`lookup("map", "lidar", at)` maps lidar coordinates into map coordinates:
target first, source second. The crate does not apply a transform to a
point for you. Rotate, then add the translation, as above; a wrong order or
a wrong direction still returns plausible numbers. See
[Transform](https://docs.rs/transforms/3.0.0/transforms/struct.Transform.html#applying-a-transform-to-a-point).

Add it to your `Cargo.toml`:

```toml
[dependencies]
transforms = "3.0.0"
```

Requires Rust 1.85 or later. The optional `serde` feature, off by default,
adds serialization and deserialization for `Vector3`, `Rotation`,
`Timestamp`, `Stamp` and `Transform`.

## What you get

- **A frame tree with history.** Fixed mounts and moving frames live in one
  tree. A lookup chains the edges between two frames and interpolates
  between stored samples: translation linearly, rotation by slerp.
- **Errors instead of guesses.** Nothing is extrapolated. A lookup outside
  the stored data fails and names the frame and the range it covers. Every
  `Transform` the crate returns has a finite translation and a unit
  rotation, so no result needs validating.
- **No middleware and no clock.** No ROS2, DDS or async runtime. Time is
  the stamps you insert; the crate never reads a clock.
- **Built for embedded.** Always `no_std` with `alloc`, and three runtime
  dependencies besides the optional `serde`. CI builds for Cortex-M and
  RISC-V targets.

## Examples

Each example is a hosted program that asserts its results.

| Example | Task |
|---|---|
| [basic](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/basic.rs) | Insert, look up across a static and an interpolated edge, apply the result to a point through a rotated frame |
| [ingest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/ingest.rs) | Feed a registry from a message stream: checked stamps, a decision per insert error, clock-jump recovery, lookup errors told apart |
| [shared_latest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/shared_latest.rs) | A writer and a reader thread sharing an `RwLock`; look up the newest servable instant |
| [cross_time](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/cross_time.rs) | Express an earlier observation in a later camera frame |

From a checkout of the repository:

```sh
cargo run --example basic
```

## Conventions

- **Direction.** `Transform::new(target, source, ...)` maps source
  coordinates into target. Inserted into a registry, the target is the
  parent and the source is the child. A successful
  `lookup(target, source, at)` returns that mapping between any two
  connected frames.
- **Static or timestamped.** `Stamp::Static` is a relationship valid for
  all time, such as a fixed sensor mount. `Stamp::At(t)` is one sample of a
  moving one. A child frame has one parent and one kind, both set by its
  first insert. Cycles are rejected, and an insert at a stored instant
  replaces that sample.
- **Time.** `Timestamp` is a signed 64-bit nanosecond count from an epoch
  you choose. Zero and negative instants are ordinary. Interpolation spans
  any gap between two stored samples and never reaches beyond them.
- **Numbers.** Coordinates and rotation components are `f64`. Keep distance
  units and frame axes consistent; the crate does not assign them.
  `Rotation` is a unit quaternion in `(w, x, y, z)` order, and
  `Rotation::from_wxyz` accepts a norm within `1e-6` of 1. Equality is
  exact: `q` and `-q` describe the same rotation and do not compare equal.
- **Errors.** Match on error variants and their fields, with a wildcard
  arm. `Display` and `Debug` text may change.

## Beyond a lookup

| To | Use |
|---|---|
| Keep a time window of history | `Registry::with_max_age`: a window behind each frame's newest stored stamp, never wall-clock time. An older insert fails with `InsertError::Expired`. It bounds the time span, not the number of samples or frames |
| Keep everything | `Registry::unbounded`, with `remove_samples_through` to remove history by hand |
| Store a batch all-or-nothing | `Registry::insert_all`: one rejected member leaves the registry unchanged, and the error carries its index |
| Find the newest instant a chain can serve | `Registry::latest_common_time`: look up at `t` for `Stamp::At(t)`; `Stamp::Static` means every instant is served |
| Relate two frames at two different instants | `Registry::lookup_across`, through a fixed frame you assert is stationary between them. See its [contract](https://docs.rs/transforms/3.0.0/transforms/struct.Registry.html#method.lookup_across) |
| Move a frame under a new parent | `Registry::reparent`: replaces that frame's history with the transform you pass; its descendants keep theirs |
| Remove a frame, or change its kind | `Registry::remove_frame` discards the frame's edge and every sample on it; insert again afterwards |

See [Registry](https://docs.rs/transforms/3.0.0/transforms/struct.Registry.html)
for lookup errors, removal semantics, and sharing a registry between
threads, and [Rotation](https://docs.rs/transforms/3.0.0/transforms/struct.Rotation.html)
for rotation conventions.

## Coming from ROS2 tf2

The model is the one [tf2](https://github.com/ros2/geometry2/blob/rolling/tf2/README.md)
uses: a tree of buffered frame relationships with static transforms and
interpolation, queried target first, source second. This crate is a Rust
implementation, not a binding to tf2, and does not aim for API parity. It
supplies local computation and storage; transport, synchronization and
waiting belong to your application.

| Aspect | ROS2 tf2 / tf2_ros | transforms |
|---|---|---|
| API | C++ and Python APIs | Rust types and `Result` |
| Lookup | `lookupTransform(target, source, time)` | `lookup(target, source, at)` |
| Latest data | A zero lookup time requests the latest transform | Zero is an ordinary instant; use `latest_common_time` |
| Retention | 10-second default cache; a sample older than the window is rejected | `unbounded` retains indefinitely; `with_max_age` sets the window and rejects older samples |
| Communication | `tf2_ros` broadcasts and listens through ROS2 | No ROS2, DDS, or other transport dependency |
| Waiting | ROS2 buffer supports waiting for transforms | Synchronous queries; caller supplies waiting |
| Embedded use | ROS2 integration uses a hosted runtime | `no_std + alloc`, with `f64` geometry |
| Re-parenting | Parent stored per sample; a dynamic frame's parent can change between samples without an error | One parent per frame; explicit `reparent` replaces that frame's history |

The tf2 column follows its [cache](https://github.com/ros2/geometry2/blob/rolling/tf2/src/cache.cpp)
and [buffer core](https://github.com/ros2/geometry2/blob/rolling/tf2/src/buffer_core.cpp).
For ROS integration in Rust, see
[roslibrust_transforms](https://docs.rs/roslibrust_transforms) and
[transforms_io](https://docs.rs/transforms_io). Their releases at the time
of 3.0.0 (0.26.0 and 0.9.0) depend on `transforms` 2.x.

## Embedded use and performance

The crate is always `no_std` and needs a heap allocator (`alloc`). There is
no `std` feature and nothing to disable for embedded targets. Its runtime
dependencies are `thiserror`, `hashbrown`, `libm`, and the optional `serde`.

CI builds for Cortex-M4F/M7, Cortex-M0+, Cortex-M33, and RISC-V targets.
These builds establish compilation, not board latency or memory bounds. As
built there, `f64` arithmetic is software floating point on all of them; a
Cortex-M7 with the double-precision FPU, built with `target-cpu=cortex-m7`,
uses hardware. `libm` supplies the square root and the trigonometry on
every target; tests pin selected interpolation results on the test host,
without guaranteeing cross-device replay.

A lookup walks the frame tree to the common ancestor of the two frames and
samples only the edges on the path between them. Sampling a dynamic edge is
O(log n) in its stored samples. A successful lookup allocates the two frame
names of its result; an error allocates the names it reports. The heap must
hold the retained samples, the frame names, and those results. Measure the
intended allocator, board, and workload before setting a budget; from a
checkout, `cargo bench` runs the benchmarks.

## What's new

3.0.0 changes the public API and the serde wire format. When migrating from
2.x, update the following:

- The crate root is flat, and the names are shorter: `insert`, `insert_all`,
  `lookup`, `lookup_across`, `reparent`, `remove_samples_through`.
- `Rotation` replaces `Quaternion` and is always a unit quaternion.
  `Transform` is always finite, and `a.compose(&b)` replaces `a * b`.
- `Timestamp` is one concrete type of signed nanoseconds. `TimePoint`, the
  `SystemTime` support, the `std` feature and `Timestamp::now` are gone.
- Errors are split by operation: `TransformError`, `InsertError`,
  `BatchError` and `LookupError`.
- `Point`, `Transformable` and `Localized` are removed; a transform is
  applied to a point with the recipe in the quick start.
- Several behaviours change without a compile error: a sample older than
  the retention window is rejected with `InsertError::Expired`, the cleanup
  cutoff is inclusive, and a query for a name the registry does not know is
  `UnknownFrame` also when both names are equal.
- Persisted 2.x data is not read reliably: 3.0 rejects it or, in a binary
  format, can decode it with a different meaning. Decode it with a 2.x
  release and convert.

The [migration guide](MIGRATION.md) lists every change with its
replacement, and the [changelog](CHANGELOG.md) has the full account and the
earlier releases.

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
