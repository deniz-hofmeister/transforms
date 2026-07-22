# Transforms

[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![tests](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![no_std](https://img.shields.io/badge/no__std-compatible-brightgreen.svg)](https://docs.rust-embedded.org/book/)
[![Downloads](https://img.shields.io/crates/d/transforms.svg)](https://crates.io/crates/transforms)

A fast, middleware-independent coordinate transform library for Rust.

## Overview

`transforms` is a pure Rust library for managing coordinate transformations between different reference frames. It is designed for robotics and computer vision applications where tracking spatial relationships between sensors, actuators, and world coordinates is essential.

**Key characteristics:**

- **Middleware-independent**: No ROS2, DDS, or any communication layer dependencies. Use it standalone or wrap it with your own pub-sub system. Checkout [roslibrust_transforms](https://docs.rs/roslibrust_transforms/latest/roslibrust_transforms/) if you are looking for a wrapped system.
- **`no_std` compatible**: Works in embedded and resource-constrained environments.
- **Memory safe**: Uses `#![forbid(unsafe_code)]` throughout.
- **Inspired by tf2**: Familiar concepts for robotics developers, but with a Rust-first API.

## Features

- **Transform Interpolation**: Smooth interpolation between transforms at different timestamps using spherical linear interpolation (SLERP) for rotations and linear interpolation for translations.
- **Transform Chaining**: Automatic computation of transforms between indirectly connected frames by traversing the frame tree.
- **Static Transforms**: Transforms carrying the static sentinel (`Timestamp::STATIC`) are valid for all time; build them with `Transform::static_between`. Every real instant — including `t=0` on boot-relative clocks — is ordinary dynamic data.
- **Time-based Buffer Management**: `Registry::with_max_age` cleans up old transforms automatically; `Registry::new` keeps them until manual cleanup. Both work with and without `std`.
- **O(log n) Lookups**: Efficient transform retrieval using `BTreeMap` storage — O(log n) in stored samples per frame, linear in chain depth for indirect frames.
- **Transformable Trait**: Implement on your own types to make them transformable between coordinate frames.
- **Transform Into**: Resolve and apply transforms directly from a `Localized` value with `get_transform_for`, eliminating manual frame and timestamp bookkeeping.

## What's New

Full version history lives in [CHANGELOG.md](CHANGELOG.md).

### v2.0.0 highlights

- **Correct by construction**: transforms are validated on insertion (finite
  values, unit rotations), the frame tree is strict (single pinned parent,
  no cycles), and lookups either answer the exact question asked or return an
  error — the silent-wrong-answer failure modes of 1.x are gone.
- **Tested on deployment architectures**: CI executes the full test suite
  natively on x86_64 and ARM64 (Raspberry Pi, NVIDIA Jetson).
- **Real `no_std`**: builds for bare-metal targets — CI proves it on
  `thumbv7em-none-eabihf` (STM32 F4/F7/H7 flight controllers),
  `thumbv6m-none-eabi` (RP2040), `thumbv8m.main-none-eabihf` (Cortex-M33),
  and `riscv32imc-unknown-none-elf` (ESP32-C3) — the `std` feature is
  additive, and automatic cleanup (`with_max_age`) works in both modes.
- **Rust-first API cleanup**: exact `==` with tolerant comparison in the
  `approx` traits, `#[non_exhaustive]` errors, private internals, optional
  `serde` support, an enforced panic policy, and MSRV 1.86.

`add_transform` is now fallible — the headline migration for 1.x users:

```rust
registry.add_transform(transform)?;
```

The full list of breaking changes with before/after code lives in
[MIGRATION.md](MIGRATION.md).

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
transforms = "2.0.0-beta.5"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enables `Timestamp::now()` and the `SystemTime` time type |
| `serde` | No | `Serialize`/`Deserialize` for the geometry and time types |

Minimum supported Rust version: 1.86 (checked in CI).

Note on `serde`: `Timestamp` serializes its nanosecond value as a `u128`.
The formats robotics users typically pair with this crate fully support it:
`serde_json`, `postcard`, and `bincode` (1.x and 2.x) all round-trip the
full range. MessagePack via `rmp-serde` round-trips Rust-to-Rust but
encodes the `u128` as a 16-byte binary blob that foreign-language consumers
will not read as an integer. Struct field order is part of the wire
contract for non-self-describing formats.
Deserialization does not validate — like hand-built transforms, deserialized
ones are validated when they enter a `Registry`.

Note on `approx`: the `AbsDiffEq`/`RelativeEq` impls on the geometry types
make `approx` 0.5 part of this crate's public API — a deliberate
commitment, since tolerant comparison is the documented alternative to the
exact `==`.

For `no_std` environments (requires a heap allocator; float math falls back to
[libm](https://crates.io/crates/libm)):

```toml
[dependencies]
transforms = { version = "2.0.0-beta.5", default-features = false }
```

## Quick Start

```rust
use core::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry with 60-second transform buffer
    let mut registry = Registry::with_max_age(Duration::from_secs(60));
    let timestamp = Timestamp::now();

    // Define a transform: sensor is 1 meter along X-axis from base
    let transform = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp,
        parent: "base".into(),
        child: "sensor".into(),
    };

    // Add and retrieve the transform (target frame first, then source:
    // "sensor"-frame data expressed in "base")
    registry.add_transform(transform)?;
    let result = registry.get_transform("base", "sensor", timestamp)?;

    println!("Transform: {result:?}");
    Ok(())
}
```

## API Reference

### Registry

```rust
// No automatic cleanup (also available via Default)
pub fn new() -> Self

// Automatic cleanup of transforms older than max_age
pub fn with_max_age(max_age: Duration) -> Self

pub fn add_transform(&mut self, transform: Transform<T>) -> Result<(), BufferError>
pub fn get_transform(&self, target: &str, source: &str, timestamp: T) -> Result<Transform<T>, TransformError>
pub fn get_transform_for<U: Localized<T>>(&self, value: &U, target_frame: &str) -> Result<Transform<T>, TransformError>
pub fn get_transform_at(&self, target_frame: &str, target_time: T, source_frame: &str, source_time: T, fixed_frame: &str) -> Result<Transform<T>, TransformError>
pub fn delete_transforms_before(&mut self, timestamp: T)
pub fn remove_frame(&mut self, child: &str) -> bool
```

### Core Types

| Type | Description |
|------|-------------|
| `Transform<T = Timestamp>` | Rigid body transformation (translation + rotation + timestamp + frames) |
| `Vector3` | 3D vector with x, y, z components (f64) |
| `Quaternion` | Quaternion for rotations (expected unit norm) with w, x, y, z components (f64) |
| `Timestamp` | Time representation in nanoseconds (u128) |
| `TimePoint` | Trait for custom timestamp types used by `Transform`, `Buffer`, and `Registry` |
| `Point` | Example transformable type with position, orientation, timestamp, frame |

For complete API documentation, see [docs.rs/transforms](https://docs.rs/transforms).

## Architecture

The library is organized around three core components:

```
┌─────────────────────────────────────────────────────────┐
│                       Registry                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  HashMap<child_frame, Buffer>                   │    │
│  │  ┌─────────────┐  ┌─────────────┐               │    │
│  │  │ Buffer "b"  │  │ Buffer "c"  │  ...          │    │
│  │  │ parent: "a" │  │ parent: "b" │               │    │
│  │  │ ┌─────────┐ │  │ ┌─────────┐ │               │    │
│  │  │ │Transform│ │  │ │Transform│ │               │    │
│  │  │ │  @ t=1  │ │  │ │  @ t=1  │ │               │    │
│  │  │ │Transform│ │  │ │Transform│ │               │    │
│  │  │ │  @ t=2  │ │  │ │  @ t=2  │ │               │    │
│  │  │ └─────────┘ │  │ └─────────┘ │               │    │
│  │  └─────────────┘  └─────────────┘               │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Registry

The main interface for managing transforms. It stores `Buffer` instances (one per child frame) and handles:

- Adding new transforms
- Retrieving transforms between any two frames (with automatic chaining)
- Traversing the frame tree to compute indirect transforms
- Automatic cleanup of expired transforms (with `Registry::with_max_age`)

### Buffer

Time-indexed storage for transforms between a specific child-parent frame pair. Uses a `BTreeMap<T, Transform<T>>` for O(log n) lookups with automatic interpolation for timestamps between stored values.

### Transform

The core data structure representing a rigid body transformation:

```rust
pub struct Transform<T = Timestamp>
where
    T: TimePoint,
{
    pub translation: Vector3,   // Position offset (x, y, z)
    pub rotation: Quaternion,   // Orientation (w, x, y, z)
    pub timestamp: T,           // When this transform is valid
    pub parent: String,         // Destination frame
    pub child: String,          // Source frame
}
```

### Localized and Transformable Traits

Implement `Transformable` on your own types to make them transformable, and `Localized` to enable automatic transform lookup via `get_transform_for`:

```rust
pub trait Localized<T = Timestamp>
where
    T: TimePoint,
{
    fn frame(&self) -> &str;
    fn timestamp(&self) -> T;
}

pub trait Transformable<T = Timestamp>
where
    T: TimePoint,
{
    fn transform(&mut self, transform: &Transform<T>) -> Result<(), TransformError>;
}
```

The `Localized` trait provides frame and timestamp introspection, while `Transformable` handles applying transforms. They are separate so that pure geometry types can implement `Transformable` without needing frame/timestamp metadata. The library provides a `Point` type as a reference implementation of both traits.

## Usage Examples

### Static vs Dynamic Transforms

Static transforms (built with `Transform::static_between`, carrying the
`Timestamp::STATIC` sentinel) are ideal for fixed relationships like sensor mounts.
A given child frame is either static or dynamic: mixing the two kinds for the same
child frame is rejected by `add_transform` with a `StaticDynamicConflict` error.

The frame tree is strict: a child frame's parent is pinned by its first
transform (re-parenting is rejected — remove the frame with
`Registry::remove_frame` and re-add it to change its parent), a frame cannot
be its own parent, and cycles are rejected at insertion. Removing a
mid-tree frame strands its descendants (they keep their pin to the removed
parent), so re-parent a subtree by removing and re-adding each descendant.
Re-publishing a transform at an already-stored timestamp replaces that
sample: last write wins. Native re-parenting support may become a feature
in a later release.

```rust
// Static transform: camera mount position (never changes)
let camera_mount: Transform = Transform::static_between(
    "base",
    "camera",
    Vector3::new(0.1, 0.0, 0.5),
    Quaternion::identity(),
);

// Dynamic transform: robot position (changes over time)
let robot_position = Transform {
    translation: Vector3::new(x, y, 0.0),
    rotation: Quaternion::identity(),
    timestamp: Timestamp::now(),
    parent: "map".into(),
    child: "base".into(),
};
```

### Transform Chaining

Query transforms between frames that aren't directly connected:

```rust
// Add transforms: map -> base -> arm -> gripper
registry.add_transform(map_to_base)?;
registry.add_transform(base_to_arm)?;
registry.add_transform(arm_to_gripper)?;

// Query: map -> gripper (automatically chains through base and arm)
let result = registry.get_transform("map", "gripper", timestamp)?;
```

The library automatically traverses the frame tree and composes the necessary transforms.

### Transform Interpolation

When querying at a timestamp between two stored transforms, the library interpolates:

```rust
// Store transforms at t=0 and t=2
registry.add_transform(transform_at_t0)?;
registry.add_transform(transform_at_t2)?;

// Query at t=1: automatically interpolates between t=0 and t=2
let interpolated = registry.get_transform("a", "b", timestamp_at_t1)?;
```

- **Translation**: Linear interpolation
- **Rotation**: Spherical linear interpolation (SLERP)

Interpolation spans any gap between two stored samples, however large —
bounding data freshness is the caller's job, via `max_age` and insert
cadence. There is no extrapolation beyond the stored range.

### Point Transformation

Transform points between coordinate frames using the `Transformable` trait:

```rust
use transforms::{
    geometry::{Point, Quaternion, Transform, Vector3},
    time::Timestamp,
    Transformable,
};

// Create a point in the camera frame
let mut point = Point {
    position: Vector3::new(1.0, 0.0, 0.0),
    orientation: Quaternion::identity(),
    timestamp: Timestamp::now(),
    frame: "camera".into(),
};

// Get the transform that maps camera-frame coordinates into the base frame
let transform = registry.get_transform("base", "camera", point.timestamp)?;

// Transform the point (mutates point.frame to "base")
point.transform(&transform)?;
```

### Transform Into Target Frame

Use `get_transform_for` to resolve and apply a transform in one step, without manually specifying the source frame or timestamp:

```rust
// Create a point in the camera frame
let mut point = Point {
    position: Vector3::new(1.0, 0.0, 0.0),
    orientation: Quaternion::identity(),
    timestamp: Timestamp::now(),
    frame: "camera".into(),
};

// Resolve transform from the point's frame to map, then apply it
let transform = registry.get_transform_for(&point, "map")?;
point.transform(&transform)?;
// point.frame is now "map"
```

If the point is already in the target frame, an identity transform is returned. This works with any type that implements `Localized`.

### Inverse Transforms

Compute the inverse of a transform:

```rust
let base_to_sensor = registry.get_transform("base", "sensor", timestamp)?;
let sensor_to_base = base_to_sensor.inverse()?;
```

### `no_std` Usage

The same API is available in `no_std` environments, including automatic
cleanup via `Registry::with_max_age`; only a registry built with
`Registry::new` requires manual cleanup:

```rust
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};
use core::time::Duration;

// Registry::new() has no automatic cleanup; Registry::with_max_age works in
// no_std too if you prefer automatic expiry
let mut registry = Registry::new();

// Create timestamp manually (no Timestamp::now() in no_std)
let timestamp = (Timestamp::zero() + Duration::from_secs(100)).unwrap();

let transform = Transform {
    translation: Vector3::new(1.0, 0.0, 0.0),
    rotation: Quaternion::identity(),
    timestamp,
    parent: "a".into(),
    child: "b".into(),
};

registry.add_transform(transform).unwrap();

// Manual cleanup for registries built without with_max_age
let cutoff = (Timestamp::zero() + Duration::from_secs(50)).unwrap();
registry.delete_transforms_before(cutoff);
```

### Concurrent Access

For multi-threaded applications, wrap the registry in appropriate synchronization primitives:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let registry = Arc::new(Mutex::new(Registry::with_max_age(Duration::from_secs(60))));

// Writer task
let registry_writer = registry.clone();
tokio::spawn(async move {
    let mut r = registry_writer.lock().await;
    r.add_transform(transform).unwrap();
});

// Reader task
let registry_reader = registry.clone();
tokio::spawn(async move {
    let r = registry_reader.lock().await;
    let result = r.get_transform("a", "b", timestamp);
});
```

## Comparison with ROS2 tf2

This library draws inspiration from ROS2's tf2 (Transform Framework 2), solving the same fundamental problem of coordinate frame tracking. Here's how they compare:

### Similarities

| Concept | Description |
|---------|-------------|
| **Frame Tree** | Both maintain parent-child relationships between coordinate frames |
| **Time Buffering** | Both store transforms over time for historical lookups |
| **Interpolation** | Both interpolate between transforms for intermediate timestamps |
| **Transform Chaining** | Both compute transforms between non-adjacent frames automatically |
| **Static Transforms** | Both support transforms that don't change over time |

### Key Differences

| Aspect | ROS2 tf2 | transforms |
|--------|----------|------------|
| **Distribution** | Distributed across nodes via DDS | Single-process, local only |
| **Middleware** | Tightly coupled to ROS2/DDS | None - completely standalone |
| **Language** | C++ with Python/other bindings | Pure Rust |
| **`no_std`** | Not supported | Fully supported |
| **Async Pattern** | `waitForTransform()` with callbacks | Synchronous (user manages async) |
| **Error Handling** | C++ exceptions | Rust `Result` types |
| **Buffer Default** | 10 seconds | User-configured |
| **Cleanup** | Automatic background process | Automatic (`with_max_age`) or manual (`Registry::new`), both modes |

### Middleware Independence

A core design principle of this library is **middleware independence**. Unlike tf2, which is deeply integrated with ROS2's DDS-based communication layer, this library has zero middleware dependencies. If you are looking for a crate which drop in integrates with ROS [roslibrust_transforms](https://docs.rs/roslibrust_transforms/latest/roslibrust_transforms/) is an option.

This means:

- **No ROS2 required**: Use in any Rust application, not just ROS2 nodes
- **No DDS overhead**: No network traffic, serialization, or distributed consensus
- **Embedded-friendly**: Works in `no_std` environments with minimal footprint
- **Bring your own transport**: If you need distributed transforms, wrap with your preferred pub-sub system (DDS, MQTT, ZeroMQ, custom protocol, etc.)

This design makes the library suitable for:

- Monolithic robotics applications
- Embedded systems and microcontrollers
- Simulations and testing without ROS2
- Applications with custom communication requirements

## `TimePoint` vs `Timestamp`

In plain terms:

- `TimePoint` is a trait (an interface). It says what a time type must do so transforms can be stored, compared, and interpolated.
- `Timestamp` is the default struct (a concrete type). It stores time as nanoseconds in a `u128`.

Use `Timestamp` if you want the default behavior.
`Registry` defaults its type parameter to `Timestamp`: in type position,
`let registry: Registry = Registry::new()` is `Registry<Timestamp>`. In
expression position the type is inferred from usage, so annotate if the
surrounding code doesn't pin it down.
If you need a custom clock or custom time representation, implement `TimePoint` and use `Registry::<CustomTimestamp>`.
With `std`, `std::time::SystemTime` support is already implemented, so `Registry::<SystemTime>` works out of the box.

## Performance

- **O(log n) time lookups**: transforms are stored in `BTreeMap` indexed by
  timestamp; multi-hop lookups scale linearly with chain depth, and a failed
  lookup runs an O(frames) diagnosis scan to name the cause
- **Early-exit chain resolution**: walks stop as soon as the target frame is reached
- **Automatic cleanup**: `with_max_age` registries prevent unbounded memory
  growth; eviction pops expired entries from the front of the map,
  O(log n + evicted) per insert
- **Allocation profile**: a single-hop lookup performs ~11 heap allocations
  (~1.5 KB churn) regardless of buffer size, plus ~4 per additional hop —
  frame names are `String`s; insertion into an existing frame does not clone
  the frame name
- **All arithmetic is `f64`**: on single-precision-FPU cores (Cortex-M4F,
  M33) transform math runs through soft-float; only double-precision FPUs
  (M7-class) execute it in hardware

Benchmarks are available in the `benches/` directory. Run with:

```bash
cargo bench
```

## Non-Goals

This library intentionally limits its scope to **rigid body transformations** (translation and rotation). The following are explicitly not supported:

- Scaling transformations
- Skew transformations
- Perspective transformations
- Non-rigid transformations
- Affine transformations beyond rigid body motion
- API parity with ROS2 tf2
- Non-linear interpolation
- Extrapolation

This focused scope keeps the library fast, predictable, and specialized for robotics applications. For more general transformation needs, consider a linear algebra or computer graphics library.

## Examples

The `examples/` directory contains complete working examples:

| Example | Description |
|---------|-------------|
| `std_minimal.rs` | Registry basics: transform a point between frames, with interpolation |
| `std_full.rs` | Concurrent async usage with Tokio (parallel readers and a writer) |
| `std_advanced.rs` | Time travel between frames with `get_transform_at` |
| `no_std_minimal.rs` | Minimal `no_std` usage: add and retrieve a transform |
| `no_std_full.rs` | Point transform and interpolation with manual cleanup |
| `no_std_advanced.rs` | Time travel in `no_std` with manual cleanup |

Run examples with:

```bash
cargo run --example std_full
cargo run --example no_std_minimal --no-default-features
```

## AI-Assisted Development

Parts of this library have been developed with AI assistance (Claude Code),
including some work that predates v2.0.0. Starting with v2.0.0, AI-assisted
contributions follow a formal framework:

- Every AI-assisted commit is disclosed with an `Assisted-by:` commit trailer
  (following the Linux kernel convention, e.g.
  `Assisted-by: Claude:claude-fable-5`), making AI involvement
  machine-queryable from v2.0.0 onward:

  ```bash
  git log --grep="Assisted-by:"
  ```

- The standards, invariants, and conventions that AI agents must follow when
  working on this repository are documented in [AGENTS.md](AGENTS.md).
- Every AI-assisted change is reviewed, tested, and understood by the
  maintainer before merging.

I, the owner of this repo, take full responsibility for every line in this repository, however it was
produced.

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

This applies to contributors as well as the maintainer: AI-assisted
contributions must follow the standards in [AGENTS.md](AGENTS.md) and carry the
`Assisted-by:` commit trailer described above.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
