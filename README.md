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

- **Middleware-independent**: No ROS2, DDS, or any communication layer dependencies. Use it standalone or wrap it with your own pub-sub system.
- **`no_std` compatible**: Works in embedded and resource-constrained environments.
- **Memory safe**: Uses `#![forbid(unsafe_code)]` throughout.
- **Inspired by tf2**: Familiar concepts for robotics developers, but with a Rust-first API.

## Features

- **Transform Interpolation**: Smooth interpolation between transforms at different timestamps using spherical linear interpolation (SLERP) for rotations and linear interpolation for translations.
- **Transform Chaining**: Automatic computation of transforms between indirectly connected frames by traversing the frame tree.
- **Static Transforms**: Transforms with timestamp `t=0` are treated as static and bypass time-based lookups.
- **Time-based Buffer Management**: Automatic cleanup of old transforms (with `std` feature) or manual cleanup (for `no_std`).
- **O(log n) Lookups**: Efficient transform retrieval using `BTreeMap` storage.
- **Transformable Trait**: Implement on your own types to make them transformable between coordinate frames.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
transforms = "1.0.3"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enables automatic buffer cleanup and `Timestamp::now()` |

For `no_std` environments:

```toml
[dependencies]
transforms = { version = "1.0.2", default-features = false }
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
    let mut registry = Registry::new(Duration::from_secs(60));
    let timestamp = Timestamp::now();

    // Define a transform: sensor is 1 meter along X-axis from base
    let transform = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp,
        parent: "base".into(),
        child: "sensor".into(),
    };

    // Add and retrieve the transform
    registry.add_transform(transform);
    let result = registry.get_transform("base", "sensor", timestamp)?;

    println!("Transform: {:?}", result);
    Ok(())
}
```

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
│  │  │ │  @ t=0  │ │  │ │  @ t=1  │ │               │    │
│  │  │ │Transform│ │  │ │Transform│ │               │    │
│  │  │ │  @ t=1  │ │  │ │  @ t=2  │ │               │    │
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
- Automatic cleanup of expired transforms (with `std` feature)

### Buffer

Time-indexed storage for transforms between a specific child-parent frame pair. Uses a `BTreeMap<Timestamp, Transform>` for O(log n) lookups with automatic interpolation for timestamps between stored values.

### Transform

The core data structure representing a rigid body transformation:

```rust
pub struct Transform {
    pub translation: Vector3,   // Position offset (x, y, z)
    pub rotation: Quaternion,   // Orientation (w, x, y, z)
    pub timestamp: Timestamp,   // When this transform is valid
    pub parent: String,         // Destination frame
    pub child: String,          // Source frame
}
```

### Transformable Trait

Implement this trait on your own types to make them transformable:

```rust
pub trait Transformable {
    fn transform(&mut self, transform: &Transform) -> Result<(), TransformError>;
}
```

The library provides a `Point` type as a reference implementation.

## Usage Examples

### Static vs Dynamic Transforms

Static transforms (timestamp = 0) are ideal for fixed relationships like sensor mounts:

```rust
// Static transform: camera mount position (never changes)
let camera_mount = Transform {
    translation: Vector3::new(0.1, 0.0, 0.5),
    rotation: Quaternion::identity(),
    timestamp: Timestamp::zero(),  // Static!
    parent: "base".into(),
    child: "camera".into(),
};

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
registry.add_transform(map_to_base);
registry.add_transform(base_to_arm);
registry.add_transform(arm_to_gripper);

// Query: map -> gripper (automatically chains through base and arm)
let result = registry.get_transform("map", "gripper", timestamp)?;
```

The library automatically traverses the frame tree and composes the necessary transforms.

### Transform Interpolation

When querying at a timestamp between two stored transforms, the library interpolates:

```rust
// Store transforms at t=0 and t=2
registry.add_transform(transform_at_t0);
registry.add_transform(transform_at_t2);

// Query at t=1: automatically interpolates between t=0 and t=2
let interpolated = registry.get_transform("a", "b", timestamp_at_t1)?;
```

- **Translation**: Linear interpolation
- **Rotation**: Spherical linear interpolation (SLERP)

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

// Get transform from camera to base
let transform = registry.get_transform("camera", "base", point.timestamp)?;

// Transform the point (mutates point.frame to "base")
point.transform(&transform)?;
```

### Inverse Transforms

Compute the inverse of a transform:

```rust
let base_to_sensor = registry.get_transform("base", "sensor", timestamp)?;
let sensor_to_base = base_to_sensor.inverse()?;
```

### `no_std` Usage

In `no_std` environments, you must manually manage buffer cleanup:

```rust
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};
use core::time::Duration;

// Create registry (no max_age parameter in no_std)
let mut registry = Registry::new();

// Create timestamp manually (no Timestamp::now() in no_std)
let timestamp = Timestamp::zero() + Duration::from_secs(100);

let transform = Transform {
    translation: Vector3::new(1.0, 0.0, 0.0),
    rotation: Quaternion::identity(),
    timestamp,
    parent: "a".into(),
    child: "b".into(),
};

registry.add_transform(transform);

// Manual cleanup required in no_std
let cutoff = Timestamp::zero() + Duration::from_secs(50);
registry.delete_transforms_before(cutoff);
```

### Concurrent Access

For multi-threaded applications, wrap the registry in appropriate synchronization primitives:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let registry = Arc::new(Mutex::new(Registry::new(Duration::from_secs(60))));

// Writer task
let registry_writer = registry.clone();
tokio::spawn(async move {
    let mut r = registry_writer.lock().await;
    r.add_transform(transform);
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
| **Cleanup** | Automatic background process | Automatic (std) or manual (no_std) |

### Middleware Independence

A core design principle of this library is **middleware independence**. Unlike tf2, which is deeply integrated with ROS2's DDS-based communication layer, this library has zero middleware dependencies.

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

## Performance

- **O(log n) lookups**: Transforms are stored in `BTreeMap` indexed by timestamp
- **Automatic cleanup**: Prevents unbounded memory growth (with `std` feature)
- **Minimal allocations**: Efficient internal data structures

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

## API Reference

### Registry

```rust
// std feature
pub fn new(max_age: Duration) -> Self

// no_std
pub fn new() -> Self

pub fn add_transform(&mut self, transform: Transform)
pub fn get_transform(&mut self, from: &str, to: &str, timestamp: Timestamp) -> Result<Transform, TransformError>
pub fn delete_transforms_before(&mut self, timestamp: Timestamp)
```

### Core Types

| Type | Description |
|------|-------------|
| `Transform` | Rigid body transformation (translation + rotation + timestamp + frames) |
| `Vector3` | 3D vector with x, y, z components (f64) |
| `Quaternion` | Unit quaternion for rotations with w, x, y, z components (f64) |
| `Timestamp` | Time representation in nanoseconds (u128) |
| `Point` | Example transformable type with position, orientation, timestamp, frame |

For complete API documentation, see [docs.rs/transforms](https://docs.rs/transforms).

## Examples

The `examples/` directory contains complete working examples:

| Example | Description |
|---------|-------------|
| `std_minimal.rs` | Basic async usage with Tokio |
| `std_full.rs` | Complete feature demonstration |
| `no_std_minimal.rs` | Minimal no_std usage |
| `no_std_full.rs` | Full no_std features with manual cleanup |

Run examples with:

```bash
cargo run --example std_full
cargo run --example no_std_minimal --no-default-features
```

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
