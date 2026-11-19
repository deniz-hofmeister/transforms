# transforms

A frame tree with a memory. Tell it how the coordinate frames of your robot
relate and how they move, then ask where one frame was relative to another,
at any instant the data between them covers.

```text
time             0 s              1 s              2 s
base in map      x = 0.0                           x = 1.0     two samples
camera on base   z = +0.25, axes aligned, at all times         one static pose

Where was the camera's origin in the map at 1 s?     (0.5, 0.0, 0.25)
```

`transforms` finds the path between the two frames and combines the poses
along it, interpolating wherever your time falls between two samples. It is
a library, not a system: no ROS, no middleware, no clock, no `std`, no
unsafe code. It needs Rust 1.85 and a heap allocator.

```toml
[dependencies]
transforms = "3.0.0"
```

Coming from tf2? [Start here](#if-you-know-ros2-tf2). Targeting a
microcontroller? [Start here](#on-a-microcontroller).

## The picture, in code

```rust
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

let mut frames = Registry::unbounded();

// The camera sits 0.25 above the base, at all times.
frames.insert(&Transform::new(
    "base", "camera",
    Vector3::new(0.0, 0.0, 0.25),
    Rotation::IDENTITY,
    Stamp::Static,
).unwrap()).unwrap();

// The base was at x = 0.0 at 0 s and at x = 1.0 at 2 s.
for (nanos, x) in [(0, 0.0), (2_000_000_000, 1.0)] {
    frames.insert(&Transform::new(
        "map", "base",
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(Timestamp::from_nanos(nanos)),
    ).unwrap()).unwrap();
}

// Where was the camera in the map at 1 s?
let at = Timestamp::from_nanos(1_000_000_000);
let camera_in_map = frames.lookup("map", "camera", at).unwrap();
assert_eq!(camera_in_map.translation(), Vector3::new(0.5, 0.0, 0.25));
```

Nobody inserted a camera pose in the map, and nobody inserted a sample at
1 s. The lookup got both the frame and the time from the data it was given.

## How it thinks

**Frames form a tree, built one edge at a time.**
`Transform::new("map", "base", ...)` gives the pose of `base` in `map`. Use
it to turn `base` coordinates into `map` coordinates. When you insert it,
`map` becomes the parent and `base` the child. A lookup takes its two
frames in the same order, target first and source second, but they can be
any two frames of one tree: it finds where their branches meet and combines
only the edges on the path between them. A registry can hold several trees.

**An edge is static or dynamic.** A static edge holds one pose that answers
every instant: `Stamp::Static`, for a sensor bolted to a chassis. A dynamic
edge holds timestamped samples: `Stamp::At(t)`. Asked for a stored instant,
it returns that sample. Asked for an instant between two samples, it
interpolates, translation linearly and rotation by slerp, however far apart
the samples are. Asked for anything before its first sample or after its
last, it has no answer.

**Time is a number you bring.** A `Timestamp` is a signed count of
nanoseconds from an epoch of your choosing, and the crate never reads a
clock. Recorded, simulated and live timestamps all work. Use one time base
per registry, or convert your stamps to one before inserting or querying.
Zero is an instant like any other.

**It refuses rather than guesses.** Building a rotation rejects a
quaternion too far from unit length, and building a transform rejects a
translation that is not finite. An insert that would give a frame a second
parent, or close a cycle, fails. A lookup reports missing coverage or a
numerical failure. When it succeeds, you get a unit rotation, a finite
translation and the two frame names you asked for.

## What it says when it cannot answer

Errors are typed, so code can match on them. The text is for people.

```rust
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

let origin = Vector3::new(0.0, 0.0, 0.0);
let pose = |target: &str, source: &str, stamp: Stamp| {
    Transform::new(target, source, origin, Rotation::IDENTITY, stamp).unwrap()
};
let seconds = |s: i64| Timestamp::from_nanos(s * 1_000_000_000);

// Two samples for `base` in `map`, at 0 s and 2 s.
let mut frames = Registry::unbounded();
frames.insert(&pose("map", "base", Stamp::At(seconds(0)))).unwrap();
frames.insert(&pose("map", "base", Stamp::At(seconds(2)))).unwrap();

// A time past the last sample.
let error = frames.lookup("map", "base", seconds(3)).unwrap_err();
assert_eq!(
    error.to_string(),
    "frame base covers 0.000000000 to 2.000000000 and not 3.000000000",
);

// A frame that no stored edge names.
let error = frames.lookup("map", "lidar", seconds(1)).unwrap_err();
assert_eq!(error.to_string(), "frame lidar is not known");

// A second parent for a frame that already has one.
let error = frames.insert(&pose("odom", "base", Stamp::At(seconds(1)))).unwrap_err();
assert_eq!(error.to_string(), "frame base has parent map, not odom");

// A static pose for an edge that is already dynamic.
let error = frames.insert(&pose("map", "base", Stamp::Static)).unwrap_err();
assert_eq!(
    error.to_string(),
    "frame base is dynamic and cannot take a static transform",
);

// Quaternion components with norm √2.
let error = Rotation::from_wxyz(1.0, 1.0, 0.0, 0.0).unwrap_err();
assert_eq!(
    error.to_string(),
    "rotation with norm 1.4142135623730951 is not a unit quaternion",
);
```

## What you will want to do next

### Move a point from one frame into another

The crate hands you the transform and leaves the arithmetic to you: rotate
the point, then add the translation.

```rust
use transforms::{Rotation, Stamp, Transform, Vector3};

// In this example +x is forward. The camera sits 1.0 ahead of the base
// and looks backwards: half a turn about z.
let camera_in_base = Transform::new(
    "base", "camera",
    Vector3::new(1.0, 0.0, 0.0),
    Rotation::from_wxyz(0.0, 0.0, 0.0, 1.0).unwrap(),
    Stamp::Static,
).unwrap();

// A point 2.0 along the camera's +x axis, in camera coordinates.
let p = Vector3::new(2.0, 0.0, 0.0);

// The same point in base coordinates.
let r = camera_in_base.rotation().rotate(p);
let t = camera_in_base.translation();
let p_in_base = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
assert_eq!(p_in_base, Vector3::new(-1.0, 0.0, 0.0));
```

Add the translation first and the x coordinate comes out as `-3.0`, with
no complaint from anything. Your code must also make sure that the point
is in the transform's source frame, at the time you looked up.

### Get the newest pose instead of the pose at a time

There is no "time zero means latest". Ask `latest_common_time(target,
source)` for the newest instant that every dynamic edge between the two
frames covers, then look up at that instant. If the path has no dynamic
edge, it returns `Stamp::Static` and any instant will do. The
[shared_latest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/shared_latest.rs)
example does this from a reader thread while a writer keeps inserting.

### Relate two measurements taken at different times

To express an old lidar return in the camera frame as it is at exposure
time, ask for
`lookup_across("camera", exposure_time, "lidar", return_time, "map")` and
apply the transform it returns to the point. The last argument is a frame
you declare fixed between the two instants; the crate takes your word for
it. The
[cross_time](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/cross_time.rs)
example does this for an object on a conveyor.

### Feed it from a message stream

Convert each incoming transform to a `Transform` and call `insert`. Use
`insert_all` to apply a batch in order: if any member is rejected, the
registry stays as it was. The
[ingest](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/ingest.rs)
example uses ROS-style messages. It converts stamps and scalar-last
quaternions, handles rejected samples, and shows how to recover after a bad
stamp pushes one frame's window too far ahead.

### Keep a time window of history

`Registry::unbounded()` never drops a sample because of its age.
`Registry::with_max_age(d)` keeps, for each dynamic frame, a window of `d`
behind its newest sample, and rejects a sample that arrives older than
that. The window limits the time span, not the number of samples or
frames, so the heap still follows your sample rate.

### Change the tree

The first time you insert a frame as a child, you fix its parent and
whether its edge is static or dynamic. After that, a new pose at a stored
instant replaces the old one, and an insert that would change the parent or
the kind is an error. `reparent` replaces the frame's edge and its history
with the pose you supply. `remove_frame` drops the frame's edge to its
parent, with every sample on it.

### Share it between threads

A `Registry` is `Send` and `Sync`, and it takes no locks of its own. Put it
behind an `RwLock`: lookups need `&self`, inserts need `&mut self`.

### Save and load transforms

The optional `serde` feature serializes `Vector3`, `Rotation`, `Timestamp`,
`Stamp` and `Transform`. Decoding runs the same checks as building, so a
decoded transform is as well formed as any other.

The [API reference](https://docs.rs/transforms/3.0.0) has the exact
contract of every call, and
[basic](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/examples/basic.rs)
is a longer first program. From a checkout, `cargo run --example basic`
runs it.

## If you know ROS2 tf2

If you know the buffer of
[tf2](https://github.com/ros2/geometry2/blob/rolling/tf2/README.md), you
know the idea: a tree of frames with history, queried with the target frame
first and the source frame second, as `lookupTransform` is. Three things
will surprise you:

1. **Time zero is not "latest".** It is an ordinary instant. Use
   `latest_common_time`.
2. **Nothing listens, broadcasts or waits.** There is no `tf2_ros` here. A
   query uses the data already in the registry; transport and waiting are
   your application's.
3. **A frame cannot drift to another parent.** In tf2, a later dynamic
   sample can name a different parent. Here `insert` rejects that, and
   `reparent` makes the change explicitly.

It is not a binding to tf2 and does not try to match its API.

## On a microcontroller

The crate is always `no_std` and needs `alloc`; there is no `std` feature
to turn off. Its direct dependencies are `thiserror`, `hashbrown` and
`libm`, plus `serde` if you enable it. CI builds it for
`thumbv7em-none-eabihf` (Cortex-M4F/M7), `thumbv6m-none-eabi` (Cortex-M0+),
`thumbv8m.main-none-eabihf` (Cortex-M33) and `riscv32imc-unknown-none-elf`.

Three things to know before you budget for it. Every translation and
rotation component is `f64`, and on those targets as CI builds them that
arithmetic runs in software. A registry keeps its frames, names and samples
on the heap, and a successful lookup allocates the two frame names of its
result and nothing else. And the CI builds show that it compiles, not how
long a lookup takes on your board.

## How fast

Each time is the median of three criterion estimates on a Jetson AGX Orin.

| Lookup | Time |
|---|---|
| One edge with 1,000 samples, at its newest sample | 141 ns |
| The same edge, between two samples | 248 ns |
| `lookup("map", "camera_optical", t)` across four edges, three of them interpolated | 651 ns |

The [benchmark record](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/analysis/v3-consensus/evidence/bench-v3-orin.md)
has the setup and the other results. From a checkout, `cargo bench`
measures your own machine.

## Why you might trust it

When a robot acts on a transform, a plausible wrong answer is worse than an
error, and the crate is built around that. Unsafe code is forbidden, and
the crate has no documented panic. Every transform it returns has a finite
translation and a unit rotation, whether you built it, decoded it or looked
it up. And the tests compare poses against reference values computed with
SciPy, because tests that only compare a library with itself cannot see a
convention that is flipped consistently.

It is provided as is, under the [MIT license](LICENSE).

## When to use something else

- You are writing a ROS2 node that must listen to or broadcast TF: use
  `tf2_ros`, or in Rust [tf2_rs](https://github.com/olingo99/tf2_rs), which
  wraps tf2's `BufferCore` and integrates with `rclrs`.
  [roslibrust_transforms](https://docs.rs/roslibrust_transforms/0.26.0)
  0.26.0 and [transforms_io](https://docs.rs/transforms_io/0.9.0) 0.9.0 put
  transport on top of this crate; both require `transforms` 2.x.
- Your coordinate systems are known at compile time, or you need geodetic
  coordinates: [sguaba](https://github.com/helsing-ai/sguaba) gives each
  coordinate system its own type. Here, frames are names looked up at run
  time.
- You have one rigid transform and want its math, with no tree and no
  history: nalgebra's
  [`Isometry3`](https://docs.rs/nalgebra/latest/nalgebra/geometry/type.Isometry3.html)
  is enough.

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

## What's new

3.0.0 is a redesign and not a drop-in upgrade from 2.x. Two warnings before
you start: some behaviour changes without a compile error, and saved 2.x
data must be decoded with a 2.x release and converted, because some 2.x
binary records decode in 3.0 without an error and with the wrong value. The
[migration guide](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/MIGRATION.md)
shows what to change or remove and marks the changes that compile
silently; the
[changelog](https://github.com/deniz-hofmeister/transforms/blob/v3.0.0/CHANGELOG.md)
has the rest. Still on 2.4? Its README is
[here](https://github.com/deniz-hofmeister/transforms/tree/v2.4.0).

## AI-Assisted Development

This project uses AI assistance. Since 2.0.0, assisted commits require an
`Assisted-by: <Agent>:<model>` trailer, and the maintainer reviews changes
before merging them. Review is not a warranty: the software is provided
"as is" under the [MIT license](LICENSE), and assessing its fitness for any
use, safety-related ones included, rests with the user. See
[AGENTS.md](AGENTS.md).

To contribute, read [CONTRIBUTING.md](CONTRIBUTING.md). To report a
vulnerability, read [SECURITY.md](SECURITY.md).
