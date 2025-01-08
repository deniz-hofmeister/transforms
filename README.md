# Transforms Library

[![tests](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Downloads](https://img.shields.io/crates/d/transforms.svg)](https://crates.io/crates/transforms)

A blazingly fast and efficient coordinate transform library for robotics and computer vision applications.

## Overview

This library provides functionality for managing coordinate transformations between different frames of reference. It supports both synchronous and asynchronous operations through feature flags, making it suitable for both real-time and event-driven applications.

For more detailed information, please refer to the [documentation](https://docs.rs/transforms). 

⚠️ DEPRECATED: To view the async-specific documentation, use:

```bash
cargo doc --open --features async
```

## Features

- **Interpolation**: Smooth linear interpolation between transforms at different timestamps.
- **Transform Chaining**: Automatic computation of transforms between indirectly connected frames.
- **Thread-safe Operations**: Safe concurrent access to the transform registry.
- **Time-based Buffer Management**: Automatic cleanup of old transforms.
## Usage

```rust
use std::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};

let mut registry = Registry::new(Duration::from_secs(60));
let timestamp = Timestamp::now();

// Create a transform from frame "base" to frame "sensor"
let transform = Transform {
    translation: Vector3::new(1.0, 0.0, 0.0),
    rotation: Quaternion::identity(),
    timestamp,
    parent: "base".into(),
    child: "sensor".into(),
};

// Add the transform to the registry
registry.add_transform(transform).unwrap();

// Retrieve the transform
let result = registry.get_transform("base", "sensor", timestamp).unwrap();
```

## Relationship with ROS2's tf2

This library draws inspiration from ROS2's tf2 (Transform Framework 2), a widely-used transform library in the robotics community. While this crate aims to solve the same fundamental problem of transformation tracking, it does so in its own way.

### Similarities with tf2

- Maintains relationships between coordinate frames in a tree structure.
- Buffers transforms over time.
- Supports transform lookups between arbitrary frames.
- Handles interpolation between transforms.
- Provides both synchronous and asynchronous APIs.

### Key Differences

- Is a pure Rust implementation, not a wrapper around tf2.
- Makes no attempt to perfectly match the ROS2/tf2 API.
- Focuses on providing an ergonomic Rust-first experience.
- Is independent of ROS2's middleware and communication system.

## Non-Goals

This library intentionally limits its scope to rigid body transformations (translation and rotation) commonly used in robotics and computer vision. The following transformations are explicitly not supported and will not be considered for future implementation:

- Scaling transformations
- Skew transformations
- Perspective transformations
- Non-rigid transformations
- Affine transformations beyond rigid body motion
- Converge to parity with ROS2 / tf2
- Extrapolation

## Roadmap

The current goals are twofold. 
- Make this library no_std compatible and allow the std functionality behind a feature flag.
- Removal of async, to allow users to implement the async functionality their own way. See [issue](https://github.com/deniz-hofmeister/transforms/issues/27)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
