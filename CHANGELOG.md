# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Docs: the README interpolation example stores its dynamic samples at `t=1`
  and `t=3` instead of `t=0` and `t=2` — `t=0` is the static sentinel, so the
  shown sequence failed with `StaticDynamicConflict` and could not
  interpolate.
- Docs: `Transform`'s `==` is described as exact IEEE 754 equality (`NaN`
  components never compare equal, `0.0 == -0.0`), not "bitwise" — the derived
  `PartialEq` was never a bit-level comparison.
- `get_transform_at` resolves when `source_frame` equals `fixed_frame`
  (including all three frames equal) instead of always failing with
  `SameFrameMultiplication`; coinciding-frame legs are now short-circuited
  rather than composed with a self-referential identity.
- `get_transform` reports `NotFound` when its two chain walks stop in
  different subtrees — a mid-chain timestamp gap or frames from disconnected
  trees — instead of `IncompatibleFrames`, whose "frames do not have a
  parent-child relationship" diagnostic is false for a transient data gap.
- `Buffer::insert` pins the child frame the way it already pinned the parent:
  a transform for a different child frame is rejected with the new
  `BufferError::ChildFrameMismatch` variant instead of silently overwriting a
  stored static transform or corrupting interpolation between dynamic ones.

## [2.0.0-alpha.1] - 2026-07-08

### Fixed

- `get_transform` verifies that the resolved chain actually connects the two
  requested frames; querying an unknown frame previously returned the
  transform to the tree root instead of an error.
- Static (`t=0`) and dynamic transforms can no longer mix within one child
  frame, which previously corrupted interpolation or silently shadowed data.
- `Transform` multiplication only accepts valid compositions
  (`t_a_b * t_b_c`); the reversed operand order produced a frame-inconsistent
  result.
- Manual cleanup (`delete_transforms_before`) no longer destroys static
  transforms.
- Error diagnostics survive wall-clock timestamps: messages are formatted via
  the infallible `TimePoint::as_seconds_lossy`, so a conversion error can no
  longer mask the error being reported.
- `no_std` works on real bare-metal targets (CI builds
  `thumbv7em-none-eabihf`): float math falls back to `libm` and dependencies
  no longer pull in `std`. A heap allocator (`alloc`) is required.

### Changed

- **Breaking:** `add_transform` returns `Result` and validates on insertion:
  non-finite values, non-unit rotations (beyond
  `Transform::UNIT_NORM_TOLERANCE`), self-referential frames, re-parenting,
  and cycles are rejected. The frame tree is strict — a child frame's parent
  is pinned by its first insert; `Registry::remove_frame` is the escape hatch
  for re-parenting.
- **Breaking:** the `std` feature is additive: `Registry::new()` /
  `Buffer::new()` (no automatic cleanup) and `with_max_age()` (automatic
  cleanup) exist in both feature modes, along with `Default`. Automatic
  cleanup works in `no_std` too.
- **Breaking:** `==` on geometry types is exact; the unsound `Eq` impl on
  `Transform` and the `PartialOrd` derives on `Quaternion`, `Vector3`, and
  `Point` are removed. Tolerant comparison lives in the `approx` traits
  (`AbsDiffEq`/`RelativeEq`, now implemented for all geometry types).
- **Breaking:** `Registry`'s internal storage is private, and all error enums
  are `#[non_exhaustive]`.
- **Breaking:** the deprecated `TimestampError` alias and the never-produced
  `BufferError::MaxAgeInvalid` variant are removed.
- Lookup results always carry the requested timestamp (also over static
  chains); `get_transform(x, x, t)` returns the identity transform; static
  transforms apply to data of any timestamp through `Transformable`; manual
  cleanup prunes frames left without transforms.
- Out-of-range interpolation reports the new `TimestampOutOfRange` variant
  with the requested time and both range endpoints; `Quaternion::slerp` clamps
  its factor to `[0.0, 1.0]` — there is no extrapolation anywhere.
- `Timestamp::as_seconds` has an honest accuracy contract: it errs beyond
  2^53 nanoseconds (~104 days), where `f64` loses sub-nanosecond accuracy.
- Error `Display` messages are lowercase per the Rust API guidelines.
- Crate upgraded to edition 2024; MSRV is 1.86, verified in CI.

### Added

- `Quaternion::new(w, x, y, z)`, `Timestamp::from_nanos` / `as_nanos`,
  `Registry::remove_frame`, `Buffer::parent` / `is_empty`,
  `Transform::validate`, `TimePoint::as_seconds_lossy`.
- Optional, default-off `serde` feature for the geometry and time types.
- Property-based test suite (proptest) covering the core invariants; fully
  deterministic test fixtures and rewritten, non-mutating benchmarks; panic
  policy enforced with clippy restriction lints and documented in the
  crate-level Reliability section. All public types are `Send + Sync`,
  documented and compile-asserted.
- CI runs the full verification gate: clippy and rustdoc at `-D warnings`,
  MSRV check, `cargo audit`, all examples, bench smoke runs, and a bare-metal
  `no_std` build.

## [1.4.1] - 2026-03-20

- Dependency updates.

## [1.4.0] - 2026-03-12

- `get_transform`, `get_transform_for`, and `get_transform_at` take `&self`
  instead of `&mut self`, enabling concurrent reads.

## [1.3.0] - 2026-03-11

- Added the `Localized` trait and `get_transform_for`, resolving a transform
  directly from a value's frame and timestamp.

## [1.2.0] - 2026-03-03

- Core types generic over time via the `TimePoint` trait;
  `std::time::SystemTime` supported out of the box.
- Added `get_transform_at` ("time travel"): query source and target frames at
  different times through a fixed frame.

## [1.1.0] - 2026-01-22

- Fixed static (`t=0`) and dynamic transforms not coexisting in the same
  tree; buffer expiration uses the latest inserted timestamp instead of
  wall-clock time.

## [1.0.3] - 2025-12-06

- Documentation updates.

## [1.0.2] - 2025-12-02

- Dependency updates.

## [1.0.1] - 2025-07-28

- Dependency updates.

## [1.0.0] - 2025-07-24

- First stable release: `no_std` support, transform chaining, SLERP
  interpolation, `Transformable` trait, automatic buffer cleanup.

[Unreleased]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-alpha.1...HEAD
[2.0.0-alpha.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.4.1...v2.0.0-alpha.1
[1.4.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.3...v1.1.0
[1.0.3]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/deniz-hofmeister/transforms/releases/tag/v1.0.0
