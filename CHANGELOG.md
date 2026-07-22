# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0-rc.1] - Unreleased

Release-candidate cut driven by a full release-readiness audit: the last
pre-stable API corrections, a performance fix on the embedded hot path,
and the migration/documentation work for stable. A migration guide from
1.x now lives in [MIGRATION.md](MIGRATION.md). This cut deliberately
breaks the beta-series API freeze — with near-zero beta adoption, the
cost of these one-way-door fixes is as close to zero as it will ever be.

### Changed

- **Breaking:** the static-transform sentinel moves from `t=0` to
  `Timestamp::STATIC` (`u128::MAX` nanoseconds). Zero was the first
  reading of exactly the boot-relative clocks the embedded story courts,
  collided with `UNIX_EPOCH`, and made zero-initialized wire messages
  silently static; the new sentinel is a value no clock produces
  organically, so every real instant — including `t=0` — is ordinary
  dynamic data. `Transform::static_between` builds static transforms
  without spelling the sentinel out. `SystemTime` keeps `UNIX_EPOCH` as
  its sentinel (no wall-clock data predates it).
- **Breaking:** every error payload field is named: `TimestampMismatch
  { lhs, rhs }`, `TimestampOutOfRange { requested, start, end }`,
  `Disconnected { target_frame, source_frame }`, and `NotFoundAt
  { target_frame, source_frame, frame, source }` — the lookup-argument
  fields carry the `_frame` suffix because a field literally named
  `source` belongs to the error trait's source-chaining convention,
  which `NotFoundAt`'s boxed `BufferError` keeps.
- **Breaking:** every public type has a single canonical path
  (`geometry::Point`, `core::Buffer`, `time::Timestamp`, ...): the leaf
  modules are private, matching the error-module pattern. Error types
  live at `errors::*`.
- **Breaking:** `UNIT_NORM_TOLERANCE` is a module-level const
  (re-exported at `geometry::UNIT_NORM_TOLERANCE`) instead of an
  associated const on `Transform<T>` that demanded a turbofish.
- `get_transform_at` composes its two legs through a private
  time-agnostic path instead of stamping them with the static sentinel
  to bypass `Mul`'s timestamp check — the sentinel has exactly one
  meaning again.

- **Breaking:** `TransformError::TransformTreeEmpty` is removed. It was
  provably unconstructible from any public path; removing an enum variant
  after stable would be a breaking change, so it goes now, following the
  precedent of `NotFound` and `MaxAgeInvalid`.
- **Breaking:** `IncompatibleFrames` and `SameFrameMultiplication` are
  struct variants carrying frame context —
  `IncompatibleFrames { expected, found }` and
  `SameFrameMultiplication { frame }` — completing the diagnosis model
  introduced in beta.3, where every frame-related error names its frames.
- **Breaking:** `Buffer::get` takes the timestamp by value, matching every
  sibling API (`TimePoint` is `Copy`).
- **Breaking:** `Timestamp`'s inner nanosecond field is private;
  `from_nanos`/`as_nanos` are the API. The serde wire format is unchanged.
- **Breaking:** `Timestamp::as_seconds_unchecked` is renamed
  `as_seconds_lossy`, matching the `TimePoint` vocabulary — the operation
  is lossy, not unsafe.
- `get_transform`'s parameters are renamed `target`/`source` (previously
  `from`/`to`; positional call sites are unaffected), aligning with
  `get_transform_at` and tf2's `lookupTransform`, and its docs gain an
  explicit direction-convention section — the old names read backwards and
  silently produced the inverse for plain-English callers.
- `with_max_age` eviction pops expired entries from the front of the
  ordered map — O(log n + evicted) per insert instead of a full-buffer
  scan (previously ~144 µs per insert at 60k live entries, the README
  Quick Start configuration at 1 kHz). A 60k-entry steady-state benchmark
  guards the regression.
- `TimePoint::checked_add` stays in the trait by decision: it completes
  the time algebra for downstream generic code, although the crate itself
  only calls `checked_sub`.

### Added

- `Timestamp::try_now()`: panic-free counterpart of `now()`, returning
  `TimeError::DurationUnderflow` on a pre-epoch system clock.
- Behavioral pin tests for commitments that freeze at stable: duplicate-
  timestamp upserts, `SameFrameMultiplication`, `max_age` boundary
  semantics (`Duration::ZERO`, inclusive boundary, out-of-order inserts),
  MAX-valued static sentinels, interior-point and near-antipodal slerp,
  `Point` error paths, mid-tree `remove_frame`, exact `NotFoundAt`
  payloads, and a postcard golden-bytes test freezing the serde wire
  format for non-self-describing formats (struct field order is part of
  the wire contract).

### Fixed

- Docs: duplicate-timestamp inserts are documented as last-write-wins
  upserts; `remove_frame` documents that it strands descendants of a
  mid-tree frame; interpolation is documented to span interior gaps of any
  size (bounding freshness is the caller's job); error `Display` strings
  are documented as not a stability surface; the O(log n) lookup claim is
  qualified (per-frame; linear in chain depth; O(frames) failure
  diagnosis); `TimePoint::static_timestamp` has contract language
  including a boot-relative-clock warning; the `approx` 0.5 public-API
  commitment is recorded; allocation-failure behavior and the
  deterministic-hasher trade-off are stated for `no_std`.
- Docs: the README no longer claims `Registry::new()` is shorthand for
  `Registry::<Timestamp>::new()` — default type parameters do not apply in
  expression position, and inference can land on any `TimePoint`.
- Docs: the vague serde `u128` caveat is replaced with the verified
  format-support matrix (`serde_json`/`postcard`/`bincode` fully support
  it; `rmp-serde` emits a 16-byte binary blob foreign consumers won't read
  as an integer).
- Docs: the serde feature-gating is now stated on every serde-capable
  type (rustdoc cannot banner derive-generated impls — verified against
  the docs.rs configuration, which the gate now builds; the crate also
  opts into `doc(auto_cfg)` for future rustdoc support); the
  `no_std_full` example imports `core::time::Duration` in its `no_std`
  branch; `Buffer` docs say B-tree instead of "binary tree" and the
  crate docs no longer call the public `Buffer` type "internal".
- CHANGELOG: the beta.3 entry called the removed `TransformError::NotFound`
  "never-produced". That was wrong — it was the primary 1.x lookup-miss
  error and beta.1/beta.2 still produced it; the entry below is corrected
  accordingly.
- AGENTS.md: the normative lookup invariant referenced the removed
  `NotFound` variant; it now names `UnknownFrame` / `Disconnected` /
  `NotFoundAt`. The release checklist loses a garbled fragment and gains
  the consolidation, semver-check, README-pin, and GitHub-release steps.

## [2.0.0-beta.4] - 2026-07-18

### Fixed

- Docs: examples covering both feature modes no longer render the `std` and
  `no_std` setup lines back to back; the `no_std` lines are hidden, so
  docs.rs shows a single coherent snippet while both variants still compile
  and run as doctests under their feature mode.

## [2.0.0-beta.3] - 2026-07-18

### Changed

- **Breaking:** failed lookups are diagnosed instead of collapsing into one
  catch-all: `get_transform` and `get_transform_at` report
  `TransformError::UnknownFrame` when a requested frame exists nowhere in
  the tree, `TransformError::NotFoundAt` when the chain walk stopped at a
  frame whose buffer holds data but cannot serve the requested time (naming
  that frame and carrying the `BufferError` as the error source), and
  `TransformError::Disconnected` when both frames exist but no chain
  connects them — mirroring tf2's LookupException / ExtrapolationException /
  ConnectivityException. The catch-all `TransformError::NotFound` variant —
  the primary lookup-miss error since 1.0 — is removed in favor of the
  diagnosed variants. (This entry originally called it "never-produced",
  which was wrong; corrected in beta.5.)
- A miss on a non-empty buffer reports `TransformError::TimestampOutOfRange`
  with the requested time and the covered range (via
  `BufferError::TransformError`), distinguishing a lookup that is merely too
  new (latency) from stale or missing data; `BufferError::NoTransformAvailable`
  is reserved for a buffer holding no transforms at all.

## [2.0.0-beta.2] - 2026-07-18

### Added

- `Buffer::child()` returns the buffer's pinned child frame, symmetric with
  `Buffer::parent()`.

## [2.0.0-beta.1] - 2026-07-17

### Fixed

- Docs: `Transform::interpolate` documents the reachable
  `TransformError::TimestampError` path (an endpoint span too large to
  represent as a `Duration`), previously absent from its `# Errors` set.
- Docs: the README no longer claims automatic cleanup is unavailable in
  `no_std` — `Registry::with_max_age` works in both feature modes; manual
  cleanup is only required for registries built with `Registry::new`.
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

## [1.1.1] - 2026-01-22

- Republish of 1.1.0 with no code changes. Exists on crates.io only; there
  is no corresponding git tag.

## [1.1.0] - 2026-01-22

- Fixed static (`t=0`) and dynamic transforms not coexisting in the same
  tree; buffer expiration uses the latest inserted timestamp instead of
  wall-clock time.

## [1.0.3] - 2025-12-06

- Documentation updates.

## [1.0.2] - 2025-12-02

- Dependency updates. Tagged in git but never published to crates.io.

## [1.0.1] - 2025-07-28

- Dependency updates.

## [1.0.0] - 2025-07-24

- First stable release: `no_std` support, transform chaining, SLERP
  interpolation, `Transformable` trait, automatic buffer cleanup.

[2.0.0-rc.1]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-beta.4...v2.0.0-rc.1
[2.0.0-beta.4]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-beta.3...v2.0.0-beta.4
[2.0.0-beta.3]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-beta.2...v2.0.0-beta.3
[2.0.0-beta.2]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-beta.1...v2.0.0-beta.2
[2.0.0-beta.1]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0-alpha.1...v2.0.0-beta.1
[2.0.0-alpha.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.4.1...v2.0.0-alpha.1
[1.4.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.1.0...v1.2.0
[1.1.1]: https://crates.io/crates/transforms/1.1.1
[1.1.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.3...v1.1.0
[1.0.3]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/deniz-hofmeister/transforms/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/deniz-hofmeister/transforms/releases/tag/v1.0.0
