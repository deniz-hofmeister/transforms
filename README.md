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
- **`no_std` compatible**: builds and runs on bare-metal targets, with a heap allocator. All arithmetic is `f64`, which is software-emulated on the single-precision FPUs most Cortex-M boards carry — see the [supported envelope](#supported-envelope) for the rates and tree depths that fit an MCU.
- **Memory safe**: Uses `#![forbid(unsafe_code)]` throughout.
- **Inspired by tf2**: Familiar concepts for robotics developers, but with a Rust-first API.

## Features

- **Transform Interpolation**: Smooth interpolation between transforms at different timestamps using spherical linear interpolation (SLERP) for rotations and linear interpolation for translations.
- **Transform Chaining**: Automatic computation of transforms between indirectly connected frames by traversing the frame tree.
- **Static Transforms**: Transforms carrying `Stamp::Static` are valid for all time; build them with `Transform::static_between`. No timestamp value is reserved — every instant, including `t=0` on boot-relative clocks, is ordinary dynamic data.
- **Time-based Buffer Management**: `Registry::with_max_age` cleans up old transforms automatically; `Registry::new` keeps them until manual cleanup. Both work with and without `std`.
- **O(log n) Lookups**: Efficient transform retrieval using `BTreeMap` storage — O(log n) in stored samples per frame, linear in chain depth for indirect frames.
- **Transformable Trait**: Implement on your own types to make them transformable between coordinate frames.
- **Transform Into**: Resolve and apply transforms directly from a `Localized` value with `get_transform_for`, eliminating manual frame and timestamp bookkeeping.

## What's New

Full version history lives in [CHANGELOG.md](CHANGELOG.md).

### v2.0.0 highlights

- **Correct by construction**: a `Transform` is validated where it is built —
  `Transform::new` and `Transform::static_between` return `Result` and reject
  non-finite values and non-unit rotations, deserialization runs the same
  check, and the private fields keep a built transform valid. The frame tree
  is strict (single pinned parent, no cycles), and lookups either answer the
  exact question asked or return an error — the silent-wrong-answer failure
  modes of 1.x are gone.
- **Tested on deployment architectures**: CI executes the full test suite
  natively on x86_64 and ARM64 (Raspberry Pi, NVIDIA Jetson).
- **Real `no_std`**: builds for bare-metal targets — CI proves it on
  `thumbv7em-none-eabihf` (STM32 F4/F7/H7 flight controllers),
  `thumbv6m-none-eabi` (RP2040), `thumbv8m.main-none-eabihf` (Cortex-M33),
  and `riscv32imc-unknown-none-elf` (ESP32-C3) — the `std` feature is
  additive, and automatic cleanup (`with_max_age`) works in both modes.
- **One flat error per call**: every `Registry` method reports
  `RegistryError<T>` — insertion and lookup alike — with the lookup payloads
  typed in your own time type instead of pre-formatted seconds. Diagnosing a
  failed lookup is a single `match`, not three nested ones.
- **Rust-first API cleanup**: exact `==` with tolerant comparison in the
  `approx` traits, `#[non_exhaustive]` errors, private internals, optional
  `serde` support, an enforced panic policy, and MSRV 1.86.
- **A stated envelope**: `f64` is a commitment — f32 and mixed precision are
  Non-Goals — and the [Performance](#performance) section publishes what
  that costs: measured per-operation timings and allocation counts, ~320 B
  of resident heap per stored sample under short frame names, and the rates
  and tree depths that do and do not fit an MCU.

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
transforms = "2.0.0"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enables `Timestamp::now()`, its panic-free `Timestamp::try_now()`, and the `SystemTime` time type |
| `serde` | No | `Serialize`/`Deserialize` for the geometry and time types |

Minimum supported Rust version: 1.86 (checked in CI).

Note on `serde`: `Timestamp` is `#[serde(transparent)]`, so it serializes as
the bare `u64` nanosecond count — an integer every serde format encodes
natively, and `serde_json`, `postcard`, `bincode` (1.x and 2.x), and
MessagePack via `rmp-serde` all round-trip the full range, with a
foreign-language consumer reading a plain number. `Stamp` is an explicitly
tagged enum — `{"At": 1753142400000000000}` and `"Static"` in JSON — so
staticness is spelled out rather than implied by an absent value: a
`timestamp` field that is missing or `null` is a decode error, never an
eternal static transform. Struct field order and `Stamp`'s variant order
are part of the wire contract for non-self-describing formats.
Deserializing a `Transform` runs the constructors' validation, so a
denormalized rotation or a non-finite component is a deserialization error
rather than a transform that answers lookups with plausible nonsense.

Note on `approx`: the `AbsDiffEq`/`RelativeEq` impls on the geometry types
make `approx` 0.5 part of this crate's public API — a deliberate
commitment, since tolerant comparison is the documented alternative to the
exact `==`.

For `no_std` environments (requires a heap allocator):

```toml
[dependencies]
transforms = { version = "2.0.0", default-features = false }
```

## Quick Start

```rust
use core::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
    Registry,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry with 60-second transform buffer
    let mut registry = Registry::with_max_age(Duration::from_secs(60));
    let timestamp = Timestamp::now();

    // Define a transform: sensor is 1 meter along X-axis from base
    let transform = Transform::new(
        "base",
        "sensor",
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(timestamp),
    )?;

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

pub fn add_transform(&mut self, transform: Transform<T>) -> Result<(), RegistryError<T>>
pub fn get_transform(&self, target: &str, source: &str, timestamp: T) -> Result<Transform<T>, RegistryError<T>>
pub fn get_transform_for<U: Localized<T>>(&self, value: &U, target_frame: &str) -> Result<Transform<T>, RegistryError<T>>
pub fn get_transform_at(&self, target_frame: &str, target_time: T, source_frame: &str, source_time: T, fixed_frame: &str) -> Result<Transform<T>, RegistryError<T>>
pub fn remove_transforms_before(&mut self, timestamp: T)
pub fn remove_frame(&mut self, child: &str) -> bool
```

Every registry call reports `errors::RegistryError<T>`, one flat
`#[non_exhaustive]` enum: `NonUnitRotation`, `NonFiniteValues`,
`SelfReferentialFrame`, `ReparentingNotSupported`, `CycleDetected` and
`StaticDynamicConflict` from insertion; `UnknownFrame`, `Disconnected` and
`NotFoundAt` from lookups. One `match` reaches every cause and every
payload — `NotFoundAt` carries the frame the walk stopped at, the
`requested: T` timestamp, and `covered: Option<(T, T)>`: `Some(range)` is a
gap in data the frame holds (a timing question), `None` is a frame holding
nothing at all (waiting will not help). The timestamps stay in your own
time type, so they compare directly against the clock you asked with. The
one wrapping variant, `RegistryError::TransformError`, reports a geometry
or time failure of an operation on the resolved chain; it never carries
`NonUnitRotation` or `NonFiniteValues`, which have exactly one spelling.

### Core Types

| Type | Description |
|------|-------------|
| `Transform<T = Timestamp>` | Rigid body transformation (translation + rotation + timestamp + frames), validated at construction |
| `Vector3` | 3D vector with x, y, z components (f64) |
| `Quaternion` | Quaternion for rotations (expected unit norm) with w, x, y, z components (f64) |
| `Timestamp` | Time representation in nanoseconds (u64, ~584 years of range) |
| `Stamp<T = Timestamp>` | When a transform is valid: `At(T)` for one instant, `Static` for all time |
| `TimePoint` | Trait for custom timestamp types used by `Transform` and `Registry` |
| `Point` | Example transformable type with position, orientation, timestamp, frame (public fields, built with `Point::new`) |

For complete API documentation, see [docs.rs/transforms](https://docs.rs/transforms).

## Architecture

`Registry` is the entire public entry point; the buffers below it are
crate-private storage, shown here because they explain the lookup costs:

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

### Buffer (internal)

Time-indexed storage for transforms between a specific child-parent frame pair, owned by the registry and not reachable from outside the crate. A dynamic buffer uses a `BTreeMap<T, Transform<T>>` for O(log n) lookups with automatic interpolation for timestamps between stored values; a static buffer stores its single transform inline and serves it for any requested time.

### Transform

The core data structure representing a rigid body transformation:

```rust
// Fields are private: construction validates, and no field can be poked back out of it.
impl<T: TimePoint> Transform<T> {
    pub fn new(parent: &str, child: &str, translation: Vector3, rotation: Quaternion, timestamp: Stamp<T>) -> Result<Self, TransformError>
    pub fn static_between(parent: &str, child: &str, translation: Vector3, rotation: Quaternion) -> Result<Self, TransformError>

    pub fn translation(&self) -> Vector3   // Position offset (x, y, z)
    pub fn rotation(&self) -> Quaternion   // Orientation (w, x, y, z)
    pub fn timestamp(&self) -> Stamp<T>    // Stamp::At(t) sample, or Stamp::Static
    pub fn parent(&self) -> &str           // Destination frame
    pub fn child(&self) -> &str            // Source frame

    pub fn inverse(&self) -> Result<Self, TransformError>
    pub fn validate(&self) -> Result<(), TransformError>
}
```

`new` and `static_between` reject non-finite components and rotations whose
norm deviates from `1.0` by more than `geometry::UNIT_NORM_TOLERANCE`. Values
*derived* from validated transforms — `inverse`, `interpolate`, `*`
composition, and registry lookups — are not re-checked, because rotation norms
drift by a few ulps per composition and rejecting that would fail legitimate
long chains; `validate` is there for a transform whose provenance you do not
control.

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

The `Localized` trait provides frame and timestamp introspection, while `Transformable` handles applying transforms. They are separate so that pure geometry types can implement `Transformable` without needing frame/timestamp metadata. The library provides a `Point` type as a reference implementation of both traits, and the `Transformable` docs state the exact map an implementation owes: rotate, then translate, with the transform's rotation on the left of the orientation composition.

## Usage Examples

### Static vs Dynamic Transforms

Static transforms (built with `Transform::static_between`, carrying
`Stamp::Static`) are ideal for fixed relationships like sensor mounts.
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
)?;

// Dynamic transform: robot position (changes over time)
let robot_position = Transform::new(
    "map",
    "base",
    Vector3::new(x, y, 0.0),
    Quaternion::identity(),
    Stamp::At(Timestamp::now()),
)?;
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
    time::{Stamp, Timestamp},
    Transformable,
};

// Create a point in the camera frame
let mut point = Point::new(
    Vector3::new(1.0, 0.0, 0.0),
    Quaternion::identity(),
    Timestamp::now(),
    "camera",
);

// Get the transform that maps camera-frame coordinates into the base frame
let transform = registry.get_transform("base", "camera", point.timestamp)?;

// Transform the point (mutates point.frame to "base")
point.transform(&transform)?;
```

### Transform Into Target Frame

Use `get_transform_for` to resolve and apply a transform in one step, without manually specifying the source frame or timestamp:

```rust
// Create a point in the camera frame
let mut point = Point::new(
    Vector3::new(1.0, 0.0, 0.0),
    Quaternion::identity(),
    Timestamp::now(),
    "camera",
);

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
    time::{Stamp, Timestamp},
    Registry,
};
use core::time::Duration;

// Registry::new() has no automatic cleanup; Registry::with_max_age works in
// no_std too if you prefer automatic expiry
let mut registry = Registry::new();

// Create timestamp manually (no Timestamp::now() in no_std)
let timestamp = (Timestamp::zero() + Duration::from_secs(100)).unwrap();

let transform = Transform::new(
    "a",
    "b",
    Vector3::new(1.0, 0.0, 0.0),
    Quaternion::identity(),
    Stamp::At(timestamp),
)
.unwrap();

registry.add_transform(transform).unwrap();

// Manual cleanup for registries built without with_max_age
let cutoff = (Timestamp::zero() + Duration::from_secs(50)).unwrap();
registry.remove_transforms_before(cutoff);
```

### Concurrent Access

Every lookup takes `&self` and the registry has no interior mutability, so
concurrent readers need no exclusive access: wrap it in an `RwLock` and only
the publisher blocks.

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(60))));

// Writer task - exclusive access
let registry_writer = registry.clone();
tokio::spawn(async move {
    registry_writer.write().await.add_transform(transform).unwrap();
});

// Reader task - shared access, does not block other readers
let registry_reader = registry.clone();
tokio::spawn(async move {
    let result = registry_reader.read().await.get_transform("a", "b", timestamp);
});
```

`examples/std_full.rs` is this pattern as a program that compiles and runs
(`cargo run --example std_full`), including how a reader picks a timestamp
its publishers already cover.

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
- **Embedded-capable**: runs in `no_std` with a heap allocator; how much tree and how much rate fit is set by `f64` math and per-sample memory, both quantified in [Supported envelope](#supported-envelope)
- **Bring your own transport**: If you need distributed transforms, wrap with your preferred pub-sub system (DDS, MQTT, ZeroMQ, custom protocol, etc.)

This design makes the library suitable for:

- Monolithic robotics applications
- Embedded systems and microcontrollers, at the rates and depths the supported envelope covers
- Simulations and testing without ROS2
- Applications with custom communication requirements

## `TimePoint` vs `Timestamp`

In plain terms:

- `TimePoint` is a trait (an interface). It says what a time type must do so transforms can be stored, compared, and interpolated: be `Copy + Ord + Debug` and provide `duration_since`, `checked_sub`, and `as_seconds_lossy`.
- `Timestamp` is the default struct (a concrete type). It stores time as nanoseconds in a `u64`, which covers about 584 years from the clock's epoch — mid-2554 for a Unix-epoch clock.

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
- **At most one inversion per lookup**: each half of the chain is composed in
  its natural direction, so a lookup toward an ancestor
  (`get_transform("map", "lidar", t)`) inverts nothing at all — a single-hop
  lookup at a stored timestamp returns that stored transform bit for bit
- **Automatic cleanup**: `with_max_age` registries prevent unbounded memory
  growth; eviction pops expired entries from the front of the map,
  O(log n + evicted) per insert
- **Allocation profile**: a single-hop lookup performs 5 heap allocations
  toward an ancestor and 6 in the reverse direction (~0.5 KB churn),
  regardless of buffer size, plus ~2 per additional hop (135 at 64 hops) —
  frame names are `String`s; insertion into an existing frame does not clone
  the frame name
- **All arithmetic is `f64`**: on single-precision-FPU cores (Cortex-M4F,
  M33) transform math runs through soft-float; only double-precision FPUs
  (M7-class) execute it in hardware
- **Identical numbers in both feature modes**: `sqrt`, `sin`, and `acos`
  come from [libm](https://crates.io/crates/libm) with and without `std`,
  never from the platform's own math library, so a desktop replay
  reproduces the target's interpolated rotations bit for bit

### Measured cost

On x86-64 (Intel i7-1065G7, release + LTO, counting global allocator),
against frames holding 1000 dynamic samples each:

| Operation | Time | Allocations |
|---|---|---|
| `add_transform`, steady state under `with_max_age` | ~0.4 µs | 2 |
| `get_transform`, 1 hop, at a stored stamp | ~0.6 µs | 5 |
| `get_transform`, 1 hop, interpolated | ~0.7 µs | 5 |
| `get_transform`, 4 hops toward an ancestor, interpolated | ~1.9 µs | 11 |
| `get_transform` rejecting an unknown frame among 1000 frames | ~9 µs | 3 |

Resident memory is about **320 B per stored sample while both frame names
are 32 characters or shorter** — a 120-byte `Transform`, its entry in the
ordered map, and the two frame-name strings, including allocator block
granularity. Every sample owns its own copy of both names, so the figure
rises with them: each name adds another 32 B per sample for every further
32 characters. A ROS-style pair of 45-character namespaced names therefore
costs ~64 B more, about **385 B per sample**, and a dynamic edge published
at 1 kHz under a one-second `max_age` holds ~320 KB under short names but
~385 KB under that pair. At equal name length 32-bit targets are smaller
(`Transform` is 96 B there), but the name strings are not — so size an MCU
heap from the names you actually publish, not from the headline figure.

### Supported envelope

The crate commits to `f64` (see [Non-Goals](#non-goals)), so on cores
without a double-precision FPU every coordinate operation is emulated in
software. That, together with the per-sample memory above, is what decides
fitness:

| Platform | Workload | Memory for a 1 s window | Basis |
|---|---|---|---|
| x86-64 / ARM64 SBC (Raspberry Pi, Jetson) | 1 kHz tick: 6 dynamic edges published and 3 lookups of 3–5 hops, ~11 µs/tick ≈ 1% of one core | ~1.9 MB, against gigabytes | measured |
| Cortex-M7 (STM32 F7/H7 — hardware `f64`) | between the rows above and below: the one named MCU class that does not pay soft-float | same per-sample figure | neither measured nor estimated |
| Cortex-M4F / M33 (`f64` in software) | ~100 Hz, mostly-static tree, one or two dynamic edges: single-digit percent of the core | ~64 KB of a 192 KB SRAM | estimated |
| Cortex-M4F / M33 | 1 kHz over 6 dynamic edges: **does not fit** — RAM runs out before CPU does | ~1.9 MB against 192 KB SRAM | estimated |
| Cortex-M0+ / RV32IMC (no FPU) | static trees and occasional lookups; one four-hop lookup is estimated above 1 ms | ~32 KB per dynamic edge at 100 Hz | estimated |

The estimated rows come from first principles — the soft-float symbols a
bare-metal build links, scaled by the x86-64 measurements above — and
nothing here was executed on target, so treat them as ±2×. The memory
column is arithmetic on the short-name per-sample figure above, so it
bounds the 32-bit rows only for frame names that short — namespaced names
push every row up.

Static transforms cost one sample forever, so publishing fixed mounts with
`Transform::static_between` is the cheapest way to keep an embedded tree
inside this envelope; `with_max_age` bounds the rest.

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
- f32 or mixed-precision arithmetic (every coordinate and rotation is f64)

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
