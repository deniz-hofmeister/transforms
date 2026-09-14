# Transforms

[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![tests](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml)

A Rust coordinate transform library inspired by ROS2 tf2, for robotics and
computer vision. It provides familiar frame trees, timestamped buffers,
transform chaining, and interpolation, with a Rust API and no middleware
dependency. It supports `no_std` with `alloc`.

[API reference](https://docs.rs/transforms) · [Migration guide](MIGRATION.md) ·
[Changelog](CHANGELOG.md)

## Installation

Requires Rust 1.85 or later.

```toml
[dependencies]
transforms = "2.1.3"
```

For `no_std`, disable default features and provide a heap allocator:

```toml
[dependencies]
transforms = { version = "2.1.3", default-features = false }
```

| Feature | Default | Effect |
|---|---|---|
| `std` | Yes | Adds `Timestamp::now()`, `Timestamp::try_now()`, and `SystemTime` support |
| `serde` | No | Adds serialization and deserialization for geometry and time types |

## Quick start

```rust
use core::time::Duration;
use transforms::{
    Registry, Transformable,
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

let mut registry = Registry::with_max_age(Duration::from_secs(60));
let timestamp = Timestamp::from_nanos(1_000_000_000);

// The sensor origin is one unit along the base frame's X axis.
registry.add_transform(Transform::new(
    "base", "sensor",
    Vector3::new(1.0, 0.0, 0.0),
    Quaternion::identity(),
    Stamp::At(timestamp),
).unwrap()).unwrap();

let mut point = Point::new(
    Vector3::new(2.0, 0.0, 0.0),
    Quaternion::identity(),
    timestamp,
    "sensor",
);

// Resolve sensor coordinates into base, then apply the transform.
let transform = registry.get_transform_for(&point, "base").unwrap();
point.transform(&transform).unwrap();
assert_eq!(point.frame, "base");
assert_eq!(point.position, Vector3::new(3.0, 0.0, 0.0));
```

## Conventions and usage

- `Transform::new(parent, child, ...)` maps **child coordinates into parent**.
  `get_transform(target, source, time)` returns that mapping from source to
  target. Positions are rotated first, then translated.
- Coordinates and quaternion components are `f64`. Use consistent distance
  units and frame axes; the crate does not assign them. Quaternions use
  `(w, x, y, z)` order and Hamilton multiplication; `q1 * q2` applies `q2`
  first. See [Quaternion](https://docs.rs/transforms/latest/transforms/geometry/struct.Quaternion.html).
- `Stamp::At(t)` is a dynamic sample; `Stamp::Static` is valid for all time.
  Use `Transform::static_between` for sensor mounts. Zero is an ordinary
  dynamic timestamp.
- Each child has one parent and one kind (static or dynamic), fixed at its
  first insertion. Cycles are rejected. Re-publishing a sample at the same
  timestamp replaces it.
- `Registry::with_max_age` evicts on insertion relative to that child's
  newest inserted timestamp, never wall-clock time. It limits history
  duration, not sample or frame count. `Registry::new` retains data until
  manual removal. Interpolation spans any retained gap; no extrapolation
  is performed.
- `latest_common_time` returns the newest commonly covered instant, or
  `Stamp::Static` for an all-static chain. Use that instant for a subsequent
  lookup. For shared registries, keep both calls under one read guard.
  Geometry errors can still occur during the lookup.
- `get_transform_at` relates two instants but stores only the target stamp.
  Keep both instants separately and apply its geometry explicitly. See its
  [application restrictions](https://docs.rs/transforms/latest/transforms/struct.Registry.html#method.get_transform_at).

Constructors and deserialization reject non-finite transform components and
non-unit rotations. Derived transforms are not re-validated: composition can
accumulate rotation drift or overflow. Call `Transform::validate` before
applying or serializing a derived result when numeric validity is required;
registry insertion always validates again.

Geometry equality and `approx` comparisons compare components; opposite
quaternion signs can represent the same rotation without comparing equal.
`approx` 0.5 is part of the public API. Error variants and payloads are stable;
`Display` and `Debug` text are not.

See [Registry](https://docs.rs/transforms/latest/transforms/struct.Registry.html)
for lookup errors, removal semantics, custom clocks, and concurrency.

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
| API | C++ and Python APIs | Rust types, traits, and `Result` |
| Lookup | `lookupTransform(target, source, time)` | `get_transform(target, source, time)` |
| Latest data | A zero lookup time requests the latest transform | Zero is an ordinary instant; use `latest_common_time` |
| Retention | 10-second default; dynamic cache pruned on insert | `new` retains indefinitely; `with_max_age` evicts on insert |
| Communication | `tf2_ros` broadcasts and listens through ROS2 | No ROS2, DDS, or other transport dependency |
| Waiting | ROS2 buffer supports waiting for transforms | Synchronous queries; caller supplies waiting |
| Embedded use | ROS2 integration uses a hosted runtime | `no_std + alloc`, with `f64` geometry |

The [tf2 cache implementation](https://github.com/ros2/geometry2/blob/rolling/tf2/src/cache.cpp)
details zero-time lookup and pruning. For ROS integration in Rust, see
[roslibrust_transforms](https://docs.rs/roslibrust_transforms) and
[transforms_io](https://docs.rs/transforms_io); check their supported versions
and middleware separately.

## Performance and embedded use

Time lookup is O(log n) in samples per frame; multi-hop lookup also depends
on chain depth. Failed lookups may scan all frames to diagnose the cause.
Lookups allocate owned results. `no_std` requires enough heap for retained
samples, frame names, and temporary lookup allocations.

CI builds for Cortex-M4F/M7, Cortex-M0+, Cortex-M33, and RISC-V targets.
These builds establish compilation, not board latency or memory bounds.
Measure the intended allocator, board, and workload before setting a budget.
`libm` is used in both feature modes; tests pin selected interpolation results
on the test host, without guaranteeing cross-device replay.

The [v2 feasibility report](analysis/v2-feasibility/REPORT.md) records host
measurements and reproduction commands. Run `cargo bench` for benchmarks.

## What's new

2.1.3 reduces dynamic-history memory use and lookup allocations. Public APIs,
serialization, and numerical behavior are unchanged. See the
[changelog](CHANGELOG.md) for earlier releases.

## Examples

All examples are hosted programs. The `no_std_*` examples disable this
library's `std` feature; their executable and logging still use `std`.

| Example | Demonstrates |
|---|---|
| [std_minimal](examples/std_minimal.rs) | Point transformation and interpolation |
| [std_full](examples/std_full.rs) | Tokio readers and a writer sharing an `RwLock` |
| [std_advanced](examples/std_advanced.rs) | Coordinates expressed between different instants |
| [no_std_minimal](examples/no_std_minimal.rs) | Insertion and lookup without the library's `std` feature |
| [no_std_full](examples/no_std_full.rs) | Point transformation, interpolation, and manual cleanup |
| [no_std_advanced](examples/no_std_advanced.rs) | Cross-time lookup and manual cleanup |

```sh
cargo run --example std_full
cargo run --example no_std_minimal --no-default-features
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
`Assisted-by: <Agent>:<model>` trailer. The maintainer reviews contributions
and takes responsibility for the code. See [AGENTS.md](AGENTS.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and verification gate,
and [SECURITY.md](SECURITY.md) for vulnerability reporting.

## License

[MIT](LICENSE).
