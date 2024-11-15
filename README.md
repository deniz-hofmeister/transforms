# Transforms
[![tests](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml/badge.svg)](https://github.com/dHofmeister/transforms/actions/workflows/tests.yml)

⚠️ WORK IN PROGRESS ⚠️

Coordinate frames transforms through time and space

## Features
Transforms offers the ability to transform the coordinates of a 3D point with timestamp, from one reference frame to another.

## Background
The robotics field needs coordinate frame transforms, for example for lidar data, GPS data and many other sources. On top of that, it usually has tight performance constraints. Combine these two and one quickly would need a performant coordinates transform such that data can be processed from the same reference frame, most often at the robot's center of gravity. 

This library draws inspiration from ROS2 and the TF2 package. 

## Usage

Usage is best defined by the registry struct under ```src/types/registry/mod.rs```. This is the main API of the system. For examples of usage, see the tests of the registry under ```src/types/registry/tests.rs```. Please note that the returned transform (Tab) is the transform for the reference frames themselves. To use the transforms to change data from one reference frame to another, use the inverse of the returned transform. ```Pb = Tab^-1 * Pa```.

1 - Create a registry, this will store all the reference frames and track the timelines. The input argument dictates how long the buffer holds on to old data.
```rust
let mut registry = Registry::new(f64::INFINITY);
```

2 - Create a transform
```rust
 let t = Timestamp::now();

 // Child frame B at x=1m without rotation
 let t_a_b = Transform {
     translation: Vector3 {
         x: 1.,
         y: 0.,
         z: 0.,
     },
     rotation: Quaternion {
         w: 1.,
         x: 0.,
         y: 0.,
         z: 0.,
     },
     timestamp: t,
     parent: "a".to_string(),
     child: "b".to_string(),
 };
```

3 - Register the transform
```rust 
registry.add_transform(t_a_b.clone());
```

4 - Request the transform
```rust
let r = registry.get_transform("a", "b", t_a_b.timestamp);
```


## Notes
It is a requirement that the point's timestamp falls within or exactly on the timestamps of known reference frames. For instance, given two reference frames R1 and R2 with known transforms at T=0 and T=10 each, then transformations are possible for points with timestamps in the range T=[0,10], inclusive. Linear interpolation is used for timestamps between known transforms. This approach ensures accurate transformations within the defined time range.

## Limitations
There are a few constraints that this system adheres to. Primarily to keep the system precise and minimal.

### Linear Interpolation Only
This library will only linearly interpolate between coordinate frames and timestamps. There will be no polynomial curve fitting or any other forms of more advanced estimation methods. This is to keep the system simple and performant. If you want more precise transforms then feel free to register more transforms with a finer delta time. After all, the inaccuracies only arise when there are large time gaps between known transforms.

### No Extrapolation
Extrapolation is potentially dangerous and/or unstable. One can argue that linear extrapolation or zero-order-hold extrapolation is still acceptable and perfectly fine to use. This system refuses to do it as it opens up the discussion of "How far of extrapolation should we allow?", "Let the user configure extrapolation..." and many other edge-case discussions that lead to unessecary complexity, discussion and behavior unpredicability. If you really want extrapolation, then you are free to publish a transform into the future and let this library "interpolate" within that.
### Reference Frame names have to be unique within the whole transform tree
If you run into an issue regarding this, consider using prefixes to group your reference frames

### No direct 1-to-1 equivelance to ROS2's TF2 library
This library is inspired by the TF2 library but makes no promise of matching its features, API or implementation. It's by inspiration only.

## Known Issues
The known issues that will be fixed in future releases: 
- Circular reference check, currently a circular reference will cause an infinite loop. Don't create one. 
- ... to be determined

## Roadmap

- v0.1 Exploration of the domain, provide functionality, built for purpose
- v0.2 Feature: Usability in an In-process/monolithic system (Rust)
- v0.3 Feature: IPC Pub/Sub + Request Server (Rust), likely [iceoryx2](https://github.com/eclipse-iceoryx/iceoryx2) as provider
- v0.4 Feature: ROS2 / tf2 bridge
- v0.x Lock-in feature set
- v0.y: Internal refactor for reasons a/b/c
- v1.0 Release (Breaking refactor & release to lock-in API)


## Note
Everything said above is up for change and this crate is still very much a WIP.
