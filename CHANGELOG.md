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

- **Breaking:** the static-transform sentinel is eliminated, not moved:
  `Transform.timestamp` is retyped from `T` to `Stamp<T> { Static, At(T) }`.
  Staticness is a variant of the type, so **no timestamp value is
  reserved** — every instant a clock can produce, including `t=0` on the
  boot-relative clocks the embedded story courts and `UNIX_EPOCH` for
  `SystemTime`, is ordinary dynamic data. In the new wire format a
  zero-stamped message is an ordinary dynamic sample, and a message
  *missing* its timestamp field is rejected rather than silently becoming
  an eternal static transform.
  `Transform::static_between` builds static transforms; every other
  transform passes its stamp to `Transform::new` as `Stamp::At(...)`.
- **Breaking:** `TimePoint` loses `static_timestamp()` and `is_static()`
  and becomes pure time arithmetic. This closes a soundness hole — the
  trait is unsealed and `is_static` was overridable independently of
  `static_timestamp`, silently breaking kind detection — and custom clock
  impls no longer invent sentinel values.
- **Breaking:** `Buffer` and the `core` module are private. `Registry` — at
  the crate root, where it was already re-exported — is the entire public
  entry point. The standalone-buffer API is gone with it: a single buffer
  enforces only its own parent and child pins, while the cycle check needs a
  view of the whole tree and lives in `Registry`, so hand-held buffers were a
  second, weaker way to store transforms for a use case nobody had — one that
  could close a cycle nothing would reject. Its storage is now free to
  change without a major release. A child frame is still static xor dynamic,
  fixed by its first insert.
- **Breaking:** serde: `Stamp` serializes as an optional timestamp —
  `Stamp::At(t)` as `t` itself (JSON shape for dynamic transforms is
  unchanged), `Stamp::Static` as `null`; postcard/bincode gain a 1-byte
  `Option` tag. No magic value appears on the wire.
- **Breaking:** every error payload field is named: `TimestampMismatch
  { lhs, rhs }`, `TimestampOutOfRange { requested, start, end }`,
  `Disconnected { target_frame, source_frame }`, and `NotFoundAt
  { target_frame, source_frame, frame, requested, covered }` — the
  lookup-argument fields carry the `_frame` suffix because a field literally
  named `source` belongs to the error trait's source-chaining convention.
- **Breaking:** one flat error type for the registry. `Registry::add_transform`
  and all three lookups return `errors::RegistryError<T>`; `BufferError` is
  gone from the public API together with the `Buffer` it belonged to, and
  `TransformError` shrinks to what its name says — the geometry and time
  failures of the `Transform` constructors, `inverse`, `interpolate`, `*`
  and `Transformable::transform` — losing `UnknownFrame`, `Disconnected` and
  `NotFoundAt`. Diagnosing a failed lookup took three nested matches
  (`TransformError::NotFoundAt` → `Box<BufferError>` →
  `TransformError::TimestampOutOfRange`) for one question: can this frame
  answer at this time? It is now one: `RegistryError::NotFoundAt` carries
  `frame`, `requested: T` and `covered: Option<(T, T)>` directly — `Some` a
  gap in data the frame holds, `None` a frame holding nothing at all — and
  the two mutually recursive error types are no longer mutually recursive.
  The lookup payloads are typed in the registry's own time type instead of
  pre-formatted seconds, so they compare against the clock the caller asked
  with; `Display` still renders seconds through `TimePoint::as_seconds_lossy`.
  Insert rejections are flat variants of the same enum — `NonUnitRotation`,
  `NonFiniteValues`, `SelfReferentialFrame`,
  `ReparentingNotSupported { current_parent }`, `CycleDetected`,
  `StaticDynamicConflict` — and the two validation causes are flat on every
  path, including a lookup that overflows a translation across a long chain,
  so no condition has two spellings to match on. `RegistryError` is
  `#[non_exhaustive]`, like every error type here.
- **Breaking:** every public type has a single canonical path
  (`geometry::Point`, `time::Timestamp`, ...): the leaf modules are private,
  matching the error-module pattern. Error types live at `errors::*`.
- **Breaking:** `Quaternion::new(w, x, y, z)` is renamed
  `Quaternion::from_wxyz(w, x, y, z)`. `new` said nothing about component
  order while the other common convention puts the scalar part last, so a
  caller passing `(x, y, z, w)` built a perfectly valid unit quaternion
  describing the wrong rotation — an error no validation can catch, because
  nothing about the result is invalid. The name now states the order at every
  call site. `Quaternion` literals are unaffected.
- **Breaking:** `Registry::delete_transforms_before` is renamed
  `Registry::remove_transforms_before`. The crate spelled one operation two
  ways — `remove_frame` next to `delete_transforms_before` — inviting readers
  to look for a distinction that never existed.
- **Breaking:** `UNIT_NORM_TOLERANCE` is a module-level const
  (re-exported at `geometry::UNIT_NORM_TOLERANCE`) instead of an
  associated const on `Transform<T>` that demanded a turbofish.
- `get_transform_at` composes its two legs through a private
  time-agnostic path instead of fabricating staticness on them to bypass
  `Mul`'s timestamp check.

- **Breaking:** `TransformError::TransformTreeEmpty` is removed. It was
  provably unconstructible from any public path; removing an enum variant
  after stable would be a breaking change, so it goes now, following the
  precedent of `NotFound` and `MaxAgeInvalid`.
- **Breaking:** `IncompatibleFrames` and `SameFrameMultiplication` are
  struct variants carrying frame context —
  `IncompatibleFrames { expected, found }` and
  `SameFrameMultiplication { frame }` — completing the diagnosis model
  introduced in beta.3, where every frame-related error names its frames.
- **Breaking:** `Timestamp`'s inner nanosecond field is private and narrows
  from `u128` to `u64`; `from_nanos(u64)` / `as_nanos() -> u64` are the API.
  u64 nanoseconds span ~584 years (mid-2554 from the Unix epoch) — past the
  service life of anything this crate positions — while halving per-sample
  stamp storage and removing multi-word arithmetic from the 32-bit MCU
  targets. The serde wire shape is unchanged for every representable value:
  a JSON integer, a postcard LEB128 varint, byte-identical golden vectors.
  MessagePack now emits a native integer instead of the 16-byte blob a
  `u128` forced, so foreign-language consumers read it as a number.
  `Timestamp::now()` gains a second (2554) way to panic, and `try_now`
  reports it as `TimeError::DurationOverflow`.
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
- **Breaking:** `TimePoint` is slimmed to the three methods the core
  actually calls — `duration_since`, `checked_sub`, and `as_seconds_lossy`
  — and gains `Debug` as a supertrait next to `Copy + Ord`. `checked_add`
  had no call site in the crate, and `as_seconds` existed only to feed the
  default `as_seconds_lossy`, whose own docs told implementors to override
  it; on a soft-float MCU target that unused checked conversion is a
  double-precision divide nobody asked for. `as_seconds_lossy` is now
  required rather than defaulted, which also states its contract in the
  type system: error formatting cannot fail. The `Debug` bound stops a
  clock type without its own derive from making `Transform<YourClock>`
  silently unprintable in the diagnostics that report a bad lookup.
- **Behavior change:** float math is `libm`'s in every feature mode.
  `Quaternion::norm`, `normalize`, and `slerp` called `f64`'s own
  `sqrt`/`sin`/`acos` under `std` and `libm`'s without it, so an
  interpolated rotation depended on the host's math library: a desktop
  replay of a flight log could differ from the flight controller that
  produced it, in a domain where the two are compared to decide which one
  was right. The `std` feature now changes which API exists, never a
  computed value. Under `std` on glibc this moves interpolated rotations by
  up to four ulps per component (measured over a sweep of arcs and
  interpolation factors; the pinned interior case moves two in `w`); `sqrt`
  is unaffected, being correctly rounded everywhere. Three slerp cases —
  interior, near-antipodal, and near-identity — are pinned bit for bit and
  run in both feature modes.
- **Breaking:** `Transform`'s fields are private and its constructors
  validate. `Transform::new(parent, child, translation, rotation, stamp)` and
  `Transform::static_between(parent, child, translation, rotation)` return
  `Result<Self, TransformError>`, rejecting non-finite components and
  rotations whose norm leaves `UNIT_NORM_TOLERANCE`; `translation()`,
  `rotation()`, `timestamp()`, `parent()` and `child()` read the components
  back, and the struct is `#[non_exhaustive]`. Validation previously sat at
  the registry boundary, which left the public `Mul` and
  `Transformable::transform` applying geometric garbage silently — a
  norm-1.01 rotation scaled everything it touched by 2% and returned `Ok`,
  and `static_between`, the README's recommended mount-builder, accepted a
  norm-2 quaternion without complaint. Validation is now *added* at
  construction, not moved there: `add_transform` still reports
  `NonUnitRotation` and `NonFiniteValues`, because results of `*`, `inverse`,
  `interpolate` and registry lookups are deliberately not re-validated —
  rotation norms drift a few ulps per composition and re-checking a long
  chain would reject legitimate results — so a caller who flattens a chain
  and re-publishes it still hands the registry a value only the insert path
  checks. `Transform::validate` stays public for transforms of uncontrolled
  provenance.
- **Breaking:** deserializing a `Transform` runs that validation. The
  `Deserialize` impl reads a shadow record and converts through `TryFrom`, so
  a denormalized rotation or a non-finite component is a deserialization
  error. This closes the documented "deserialization does not validate" gap;
  the wire format is unchanged, golden bytes included.
- **Breaking:** `Transform::identity()` is removed. It returned empty parent
  and child frames — self-referential, so it could be neither inserted nor
  composed — and existed only as a base for field-poking, which private
  fields end. The registry synthesizes its own identity for same-frame
  lookups.
- **Breaking:** `Point` gains `Point::new(position, orientation, timestamp,
  frame)` and `#[non_exhaustive]`, which ends its struct literal. Its fields
  stay public: a point is a data record, not an invariant carrier — the
  invariants live on the `Transform` applied to it.
- **Behavior change:** a lookup inverts at most once. Both halves of a
  resolved chain are now composed in their natural direction, deleting a
  reverse-and-invert pass that cost one inversion per hop plus one at the end
  — in the crate's own documented direction
  (`get_transform("map", "lidar", t)`), the one the README teaches. Measured
  on x86-64: a single hop drops from 11 heap allocations (1456 B) to 5
  (488 B), four hops from 23 to 11, and 64 hops from 271 to 135. That
  direction is no longer renormalized on the way out, which is what makes a
  single-hop lookup at a stored timestamp return the stored transform bit for
  bit; previously it came back through two inversions, up to ten ulps off in
  the translation.

### Added

- `Timestamp::try_now()`: panic-free counterpart of `now()`, returning
  `TimeError::DurationUnderflow` on a pre-epoch system clock and
  `TimeError::DurationOverflow` on one set past the u64 nanosecond range.
- Behavioral pin tests for commitments that freeze at stable: duplicate-
  timestamp upserts, `SameFrameMultiplication`, `max_age` boundary
  semantics (`Duration::ZERO`, inclusive boundary, out-of-order inserts),
  static transforms over custom clocks including range extremes,
  interior-point and near-antipodal slerp, `Point` error paths, mid-tree
  `remove_frame`, exact `NotFoundAt` payloads, and postcard golden-bytes
  tests for both `Stamp` arms freezing the serde wire format for
  non-self-describing formats (struct field order is part of the wire
  contract).
- `TransformError::StaticInterpolation`: a static transform used as an
  interpolation endpoint is rejected explicitly instead of falling
  through to an incidental error.
- Two of the six examples (`std_full`, `no_std_full`) now demonstrate a
  static sensor mount chained with dynamic edges — the feature was
  previously unexercised outside the test suite.
- Benchmarks for the shapes real users hit: a realistic 6-edge robot tree
  (mixed static/dynamic), a 3-hop interpolating dynamic chain,
  `get_transform_at`, and a 100k-resident insert bench pinning the
  eviction fix at depth.
- CI: clippy and MSRV jobs now also cover the `serde` feature (std and
  no_std), closing the thinnest spot in the matrix.

### Fixed

- `Registry::remove_transforms_before` resets each frame's `max_age` expiry
  reference along with its samples: previously a wiped buffer kept its
  pre-wipe latest timestamp, so a restarted stream at earlier times was
  evicted by the very insert that added it — `Ok(())` returned, the frame
  stayed empty.
- The `std` feature forwards `serde?/std`, so `Transform<SystemTime>`
  implements `Serialize`/`Deserialize` for downstream users who enable
  `std` + `serde` without depending on serde's `std` feature themselves.
- `t = 0` is usable as an ordinary dynamic timestamp: the most natural
  insert loop there is (`for i in 0.. { insert(tf(i * step)) }`) no
  longer silently creates a static buffer at `i = 0` and fails with
  `StaticDynamicConflict` at `i = 1`. Pinned by a regression test and the
  property-test timestamp strategy now includes zero.
- A dynamic child frame drained by `remove_transforms_before` could silently
  flip to static on the next insert (the kind was a flag re-decided on
  emptiness) and then reject dynamic samples. The kind is now declared when
  the frame is created and structural — the flip is unrepresentable, pinned
  by a regression test.
- **Behavior change:** `Registry::remove_transforms_before` no longer drops
  frames it leaves empty. Dropping them un-pinned the frame's parent and
  its static/dynamic kind, so routine cleanup silently re-opened decisions
  the registry had already refused: a rejected re-parenting became an
  accepted one and changed the topology behind the caller's back, and a
  moving frame could become an eternal static one that answered
  confidently at times its data never covered. A drained frame now keeps
  its pins, lookups on it report `NotFoundAt` naming that frame instead of
  `UnknownFrame`, and `Registry::remove_frame` is the only way to release
  a frame — long-running processes that mint transient frame names must
  call it. Three regression tests pin the pin, the kind, and the
  diagnosis. `NotFoundAt`'s documentation now separates its two causes,
  because a drained frame is the first one reachable from `Registry`:
  `TimestampOutOfRange` carries the frame's covered range and is a timing
  question, `NoTransformAvailable` carries no range and means the frame
  holds nothing — retrying or widening the window will not make it answer.
- Docs: `Registry::new` states what its lack of a `max_age` costs — not
  only unbounded retention, but an unbounded interpolation gap, since a
  lookup between samples recorded either side of a publisher stall
  interpolates straight across it. `Registry::with_max_age` bounds both,
  and the `Default` impl points at the same explanation.
- Docs: duplicate-timestamp inserts are documented as last-write-wins
  upserts; `remove_frame` documents that it strands descendants of a
  mid-tree frame; interpolation is documented to span interior gaps of any
  size (bounding freshness is the caller's job); error `Display` strings
  are documented as not a stability surface; the O(log n) lookup claim is
  qualified (per-frame; linear in chain depth; O(frames) failure
  diagnosis); the `approx` 0.5 public-API
  commitment is recorded; allocation-failure behavior and the
  deterministic-hasher trade-off are stated for `no_std`.
- Docs: the README no longer claims `Registry::new()` is shorthand for
  `Registry::<Timestamp>::new()` — default type parameters do not apply in
  expression position, and inference can land on any `TimePoint`.
- Docs: the vague serde `u128` caveat is replaced with the format-support
  statement the narrowed `u64` stamp makes simple — every serde format
  encodes it as a native integer.
- Docs: the serde feature-gating is now stated on every serde-capable
  type (rustdoc cannot banner derive-generated impls — verified against
  the docs.rs configuration, which the gate now builds; the crate also
  opts into `doc(auto_cfg)` for future rustdoc support); the
  `no_std_full` example imports `core::time::Duration` in its `no_std`
  branch; the buffer docs say B-tree instead of "binary tree".
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
