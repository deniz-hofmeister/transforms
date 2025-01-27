# Transforms

[![tests](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms)
[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Downloads](https://img.shields.io/crates/d/transforms.svg)](https://crates.io/crates/transforms)

A blazingly fast, minimalist and efficient coordinate transform library for robotics and computer vision applications.

## Overview

This library provides functionality for managing coordinate transformations between different frames of reference.

For more detailed information, please refer to the [documentation](https://docs.rs/transforms). 

## Features

- **Interpolation**: Smooth linear interpolation between transforms at different timestamps.
- **Transform Chaining**: Automatic computation of transforms between indirectly connected frames.
- **Static Transforms**: Submitting a timestamp at t=0 will short-circuit the lookup and always return the t=0 transform.
- **Time-based Buffer Management**: Automatic cleanup of old transforms is available with feature = "std", which is default enabled. If the library is used as **no_std** then manual cleanup is required. See the examples.
- **Minimal Dependencies**: This library aims to provide only the core functionality of being a transforms library.

### Note

This library supports **no_std** implementations. By default the feature = "std" is enabled. Disable default features use the no_std version:

```shell
transforms = { version = "0.5.0", default-features = false }
```


## Usage

```rust
use core::time::Duration;
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
registry.add_transform(transform);

// Retrieve the transform
let result = registry.get_transform("base", "sensor", timestamp);
```
For more in-depth examples please see the examples directory.

## Roadmap

Please refer to the [Milestones](https://github.com/deniz-hofmeister/transforms/milestones) page for an up-to-date roadmap.

## Relationship with ROS2's tf2

This library draws inspiration from ROS2's tf2 (Transform Framework 2), a widely-used transform library in the robotics community. While this crate aims to solve the same fundamental problem of transformation tracking, it does so in its own way. The core functionality of this library is similar to tf2.

### Similarities with tf2

- Maintains relationships between coordinate frames in a tree structure.
- Buffers transforms over time.
- Supports transform lookups between arbitrary frames.
- Handles interpolation between transforms.

### Key Differences

- Is a pure Rust implementation, not a wrapper around tf2.
- Makes no attempt to perfectly match the ROS2/tf2 API.
- Focuses on providing an ergonomic Rust-first experience.
- Is independent of ROS2's middleware and fully stand-alone.

A core conceptual difference between ROS2/tf2 and this library is that this library does not attempt to integrate into a communication layer as strongly as tf2 does with the ROS2 RMW DDS layer. If you would like this system to be DDS-enabled then it is left up to the user to write the pub-sub wrapper around this library. This is done based on the ideal that this library should be suitable for monolithic codebases and no_std implementations.

## Non-Goals

This library intentionally limits its scope to rigid body transformations (translation and rotation) commonly used in robotics and computer vision. The following transformations are explicitly not supported and will not be considered for future implementation:

- Scaling transformations
- Skew transformations
- Perspective transformations
- Non-rigid transformations
- Affine transformations beyond rigid body motion
- Converge to parity with ROS2 / tf2
- Non-linear interpolation
- Extrapolation

All the features mentioned above are going beyond the definition of a minimal, precise and predictable library. If you feel like there is a feature here that could be argued as critically-needed, feel free to open an issue with a feature request. We are open to change our minds. 

## Contribution

We are grateful for any form of comments, issues, or constructive criticism. Feel free to reach out or create an issue on this page. Feature requests are also more than welcome.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
