# transforms

Find the position and orientation of one coordinate frame relative to
another, at the instant a measurement was taken.

```text
map
└── base        drives from x = 0 m at 0 s to x = 1 m at 2 s
    └── camera  fixed 0.25 m above the base, axes aligned
```

The camera takes a picture at 1 s, halfway between the two base samples.
`lookup("map", "camera", exposure)` combines the interpolated base pose
with the fixed mount and returns the camera's pose in map coordinates. Its
origin is at **(0.5, 0, 0.25) m**.

`transforms` stores the timestamped poses you supply and answers such
lookups between any two connected frames, interpolating translation
linearly and rotation by slerp. Where the stored samples do not cover the
requested instant, the lookup returns an error; nothing is extrapolated.
If you use ROS2, this is the job of the tf2 buffer; here there is no ROS,
no middleware and no clock underneath.

Rigid-body transforms only, in `f64`. `no_std`, needs a heap allocator, no
unsafe code.

[API reference](https://docs.rs/transforms/3.0.0) · [Examples](#examples) ·
[Coming from tf2](#coming-from-ros2-tf2) ·
[Embedded use](#embedded-use-and-performance) ·
[Using 2.4?](https://github.com/deniz-hofmeister/transforms/tree/v2.4.0)

[![Crates.io](https://img.shields.io/crates/v/transforms.svg)](https://crates.io/crates/transforms)
[![Documentation](https://docs.rs/transforms/badge.svg)](https://docs.rs/transforms/3.0.0)
[![tests](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/deniz-hofmeister/transforms/actions/workflows/tests.yml)

## Try it

```toml
[dependencies]
transforms = "3.0.0"
```

```rust
use transforms::{LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

let mut frames = Registry::unbounded();

// The camera mount is static: one pose, valid at every instant.
let mount = Transform::new(
    "base", "camera",
    Vector3::new(0.0, 0.0, 0.25),
    Rotation::IDENTITY,
    Stamp::Static,
).unwrap();
frames.insert(&mount).unwrap();

// The base is dynamic: one timestamped sample at 0 s and one at 2 s.
for (nanos, x) in [(0, 0.0), (2_000_000_000, 1.0)] {
    let pose = Transform::new(
        "map", "base",
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(Timestamp::from_nanos(nanos)),
    ).unwrap();
    frames.insert(&pose).unwrap();
}

// The camera in the map at the exposure, 1 s.
let exposure = Timestamp::from_nanos(1_000_000_000);
let camera = frames.lookup("map", "camera", exposure).unwrap();
assert_eq!(camera.translation(), Vector3::new(0.5, 0.0, 0.25));

// The newest base sample is at 2 s. A lookup at 3 s is out of range.
let late = Timestamp::from_nanos(3_000_000_000);
let error = frames.lookup("map", "camera", late).unwrap_err();
assert!(matches!(error, LookupError::OutOfRange { .. }));
```

`lookup("map", "camera", t)` returns the transform that maps camera
coordinates into map coordinates: the frame you want coordinates in comes
first, the frame they are in now comes second.

Requires Rust 1.85 or later. The optional `serde` feature adds
serialization and deserialization for `Vector3`, `Rotation`, `Timestamp`,
`Stamp` and `Transform`.

## What to know first

- **Direction.** `Transform::new(target, source, ...)` describes the same
  mapping as a lookup: source coordinates into target coordinates. When you
  insert a transform, its target becomes the parent and its source the
  child. You can query any two frames of the same tree, siblings included.
- **Points.** The crate returns the transform and leaves applying it to
  you: rotate the point, then add the translation. The point must be
  expressed in the source frame at the lookup time. Reversing the mapping
  or translating before rotating can give a plausible, wrong result. See
  the [recipe](https://docs.rs/transforms/3.0.0/transforms/struct.Transform.html#applying-a-transform-to-a-point).
- **Quaternions.** `Rotation::from_wxyz(w, x, y, z)` is scalar first. A
  quaternion passed in `(x, y, z, w)` order can pass validation and give
  the wrong orientation.
- **Time.** A `Timestamp` is signed nanoseconds from an epoch you choose;
  express every timestamp in a registry on the same time base. The crate
  never reads a clock. Zero is an ordinary instant, not "latest":
  `latest_common_time` reports the newest instant covered by every dynamic
  edge on the path, or `Stamp::Static` if the path has none.
- **Coverage.** A lookup interpolates between stored samples, across a gap
  of any length, and never beyond the first or the last. Every dynamic
  edge on the path between the two frames must cover the requested instant;
  static edges serve every instant.
- **Inserts.** A frame's first insert as a child fixes its parent and
  whether it is static or dynamic, and an insert that would change either
  fails. A new pose at a stored timestamp replaces the old one. To change a
  parent, call `reparent`, which replaces that frame's history.
- **Retention.** `Registry::unbounded()` never evicts by age.
  `Registry::with_max_age(d)` keeps a window of `d` behind each dynamic
  frame's newest sample and rejects an insert older than that. The window
  bounds the time span, not the number of samples or frames.

The [API reference](https://docs.rs/transforms/3.0.0/transforms/struct.Registry.html)
has the contract of every call, including batch inserts (`insert_all`),
lookups between two different instants (`lookup_across`), removal, and
sharing a registry between threads.

## Examples

| Example | Task |
|---|---|
| [basic](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/basic.rs) | Look up a camera pose across a static and an interpolated edge and apply it to a point through a rotated frame; compose and invert |
| [ingest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/ingest.rs) | Feed a registry from a stream of ROS-style messages: convert stamps and quaternion order, handle rejected samples and a clock jump |
| [shared_latest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/shared_latest.rs) | Share a registry between a writer and a reader thread and look up the newest covered instant |
| [cross_time](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/cross_time.rs) | Express an earlier observation in a later camera frame |

Each one asserts its results. From a checkout of the repository, run one
with `cargo run --example basic`.

## Correctness

- **Every transform is well formed.** Every `Rotation` is a unit
  quaternion, to within rounding, and every `Transform` has a finite
  translation and non-empty frame names, whether it was built, decoded,
  composed, inverted or looked up. An operation that cannot produce such a
  value returns an error.
- **Typed errors.** Building a transform, inserting into a registry and
  looking up each return a `Result` with an error specific to the
  operation. The crate has no documented panic.
- **Checked against an outside reference.** The tests assert poses against
  reference values computed with SciPy. Tests that only compare the crate
  with itself cannot see a convention that is flipped consistently.

## Coming from ROS2 tf2

The model is [tf2](https://github.com/ros2/geometry2/blob/rolling/tf2/README.md)'s:
a tree of buffered frame relationships with static transforms and
interpolation, queried target first, source second. This crate is a Rust
implementation, not a binding, and does not aim for API parity.

| | ROS2 tf2 / tf2_ros | transforms |
|---|---|---|
| Latest data | A zero lookup time requests the latest transform | Zero is an ordinary instant; ask `latest_common_time` |
| Transport and waiting | `tf2_ros` listens, broadcasts and can wait for a transform | None: queries are synchronous, and your application supplies transport and waiting |
| Parent of a frame | Dynamic samples can name different parents; tf2 does not interpolate across such a change | Fixed by the first insert; `reparent` is explicit and replaces that frame's history |
| Dynamic history | 10-second cache by default | No age limit, or the window you set with `with_max_age` |

The tf2 column follows
[buffer_core.hpp](https://github.com/ros2/geometry2/blob/rolling/tf2/include/tf2/buffer_core.hpp)
(zero time, default cache),
[cache.cpp](https://github.com/ros2/geometry2/blob/rolling/tf2/src/cache.cpp)
(parents) and `tf2_ros`'s
[buffer.hpp](https://github.com/ros2/geometry2/blob/rolling/tf2_ros/include/tf2_ros/buffer.hpp)
(waiting). To feed a registry from ROS-style messages, start from the
[ingest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/ingest.rs)
example.

## When something else fits better

- **A ROS2 node that listens to or broadcasts TF.** Use `tf2_ros`, or in
  Rust [tf2_rs](https://github.com/olingo99/tf2_rs), which wraps tf2's
  `BufferCore` and integrates with `rclrs`.
  [roslibrust_transforms](https://docs.rs/roslibrust_transforms/0.26.0)
  0.26.0 and [transforms_io](https://docs.rs/transforms_io/0.9.0) 0.9.0 add
  transport on top of this crate; both require `transforms` 2.x.
- **Coordinate systems known at compile time, or geodetic coordinates.**
  [sguaba](https://github.com/helsing-ai/sguaba) types each coordinate by
  its coordinate system. Here, frames are names resolved at run time.
- **One rigid transform and its math, with no tree and no history.**
  nalgebra's [`Isometry3`](https://docs.rs/nalgebra/latest/nalgebra/geometry/type.Isometry3.html)
  is enough.

## Embedded use and performance

The crate is always `no_std` and needs a heap allocator (`alloc`). There is
no `std` feature and nothing to disable for embedded targets. Its direct
runtime dependencies are `thiserror`, `hashbrown`, `libm`, and the optional
`serde`.

CI builds for `thumbv7em-none-eabihf` (Cortex-M4F/M7), `thumbv6m-none-eabi`
(Cortex-M0+), `thumbv8m.main-none-eabihf` (Cortex-M33) and
`riscv32imc-unknown-none-elf`. These builds establish compilation, not
latency or memory use on a board. With those targets' default settings,
`f64` arithmetic uses software routines.

A registry uses the heap for its frame entries, frame names and dynamic
samples. A successful lookup allocates only the two frame names of its
result.

Host benchmark, on a Jetson AGX Orin, as medians of three criterion runs: a
lookup across one edge holding 1,000 samples took 141 ns at a stored
instant and 248 ns interpolated, and `lookup("map", "camera_optical", t)`
across four edges, three of them interpolated, took 651 ns. These are not
latency bounds and not microcontroller figures; the
[benchmark record](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/analysis/v3-consensus/evidence/bench-v3-orin.md)
has the setup.

## What's new

3.0.0 redesigns the API and is not a drop-in upgrade from 2.x. Everything
is at the crate root, `Rotation` replaces `Quaternion`, `Timestamp` is one
signed nanosecond count, errors are split by operation, and the serde wire
format changed.

- Some behaviour changes without a compile error. Read the rows tagged
  `silent` in the
  [migration guide](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/MIGRATION.md).
- Decode saved 2.x data with a 2.x release, then convert it. Some 2.x
  binary records decode in 3.0 without an error and with the wrong value.

The [changelog](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/CHANGELOG.md)
has the details.

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
