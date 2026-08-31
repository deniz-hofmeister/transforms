# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0] - Unreleased

### Added

- `Registry::reparent_frame(transform)`: atomic native re-parenting.
  The transform's child is the frame to move, its parent the new
  parent, and the transform itself seeds the frame's history under the
  new pin. Every check — root/unknown diagnosis, unchanged parent,
  cycle detection, and the ordinary insert checks on the seed — runs
  before any mutation, so a rejection leaves the registry untouched;
  the previously documented `remove_frame`-then-re-add route could
  destroy the frame first and only then discover the cycle. The price
  is the frame's stored history, dropped loudly (coverage collapses to
  the seed's instant); the frame's static-or-dynamic kind and its
  `max_age` expiry policy are deliberately preserved — a seed of the
  opposite kind is rejected with `StaticDynamicConflict` rather than
  quietly rewriting a time series as one eternal pose. Descendants
  ride along with their history intact.
- `RegistryError::NoParentToReplace(frame)`: `reparent_frame`'s
  refusal for a known root — a root gains a parent through an ordinary
  `add_transform` insert, not a re-parent.
- `RegistryError::ParentUnchanged(frame)`: `reparent_frame`'s refusal
  when the "new" parent is the current one. An error, not an upsert:
  resolving every failed insert into a re-parent would wipe the
  frame's history once and look correct forever after.

### Fixed

- The README, MIGRATION.md and the `Registry::remove_frame` docs
  taught that moving a subtree requires removing and re-adding each
  descendant. False and destructive: descendants keep their pin to the
  removed frame, so re-adding only the subtree's root reconnects the
  whole subtree with every descendant's history intact. The corrected
  recipe is now documented and pinned by a test.

### Changed

- `RegistryError::ReparentingNotSupported`'s message and documentation
  now point to `reparent_frame` as the deliberate re-parenting path
  (the variant name predates the feature; renaming it would be a
  breaking change, deferred to a 3.0).

## [2.1.0] - 2026-08-27

### Added

- `Registry::latest_common_time(target, source)`: returns the newest
  instant `get_transform` can serve for the pair — the oldest of the
  connecting chain's dynamic hops' newest samples, consulting only the
  hops the chain actually crosses, so the answer is exact for mid-tree
  pairs too. All-static chains (and `target == source`) return
  `Stamp::Static`: the chain puts no bound on time and the caller picks
  the instant. The intended idiom is this call followed by
  `get_transform` at the returned instant, under one lock guard when the
  registry is shared.
- `RegistryError::NoCommonTime`: `latest_common_time`'s refusal when no
  instant is servable by every hop, naming the hop that rules it out —
  `covered: Some(range)` when the chain's covered ranges are disjoint,
  `covered: None` when the hop holds no data at all. Unknown and
  disconnected frames report the same `UnknownFrame` and `Disconnected`
  variants as a failed lookup.

### Changed

- The retry-off-`covered` "latest available" idiom is retired from the
  `NotFoundAt` documentation in favor of `latest_common_time`, which is
  exact where the loop was conservative (mid-tree targets) and does not
  spend a failed chain walk per attempt. `examples/std_full.rs` now
  demonstrates the new idiom instead of hardcoding a lookup lag coupled
  to the writer's publish rate.
- `Registry`'s `Debug` output summarizes each frame's buffer — frames,
  kind, sample count, covered range — instead of dumping every stored
  sample. `Debug` output is not a stability surface; match on error
  variants and read accessors, never on formatted text.

## [2.0.0] - 2026-08-22

The 1.x line's silent-wrong-answer surfaces are closed: 2.0.0 validates
what enters the registry, diagnoses what fails, and pins its numerics
bit for bit across feature modes. A migration guide from 1.4.1 lives in
[MIGRATION.md](MIGRATION.md). Five pre-releases were published on the
way — alpha.1 and beta.1 through beta.4, with no release candidate —
and this section is the consolidated `v1.4.1 → v2.0.0` delta, their
sections folded together; the per-pre-release history stays readable in
the [changelog as published with
beta.4](https://github.com/deniz-hofmeister/transforms/blob/v2.0.0-beta.4/CHANGELOG.md).

### Changed

- **Breaking:** the static-transform sentinel is eliminated, not moved:
  `Transform.timestamp` is retyped from `T` to `Stamp<T> { Static, At(T) }`.
  Staticness is a variant of the type, so **no timestamp value is
  reserved** — every instant a clock can produce, including `t=0` on the
  boot-relative clocks the embedded story courts and `UNIX_EPOCH` for
  `SystemTime`, is ordinary dynamic data. On the wire a zero-stamped
  message is an ordinary dynamic sample, and a message *missing* its
  timestamp field is rejected rather than silently becoming an eternal
  static transform. `Transform::static_between` builds static
  transforms; every other transform passes its stamp to `Transform::new`
  as `Stamp::At(...)`.
- **Breaking:** `Transform`'s fields are private and its constructors
  validate. `Transform::new(parent, child, translation, rotation, stamp)` and
  `Transform::static_between(parent, child, translation, rotation)` return
  `Result<Self, TransformError>`, rejecting non-finite components and
  rotations whose norm leaves the unit tolerance — published as the
  module-level const `geometry::UNIT_NORM_TOLERANCE`; `translation()`,
  `rotation()`, `timestamp()`, `parent()` and `child()` read the components
  back, and the struct is `#[non_exhaustive]`. In 1.4.1 nothing validated a
  transform anywhere, which left the public `Mul` and
  `Transformable::transform` applying geometric garbage silently — a
  norm-1.01 rotation scaled everything it touched by 2% and returned `Ok`,
  and a NaN translation was stored and served. Validation sits at
  construction *and* at insert: `add_transform` still reports
  `NonUnitRotation` and `NonFiniteValues`, because results of `*`, `inverse`,
  `interpolate` and registry lookups are deliberately not re-validated —
  rotation norms drift a few ulps per composition and re-checking a long
  chain would reject legitimate results — so a caller who flattens a chain
  and re-publishes it still hands the registry a value only the insert path
  checks. `Transform::validate` stays public for transforms of uncontrolled
  provenance.
- **Breaking:** `add_transform` returns `Result` and validates on insertion:
  non-finite values, non-unit rotations, self-referential frames,
  re-parenting, and cycles are rejected. The frame tree is strict — a child
  frame's parent and its static-or-dynamic kind are pinned by its first
  insert, the pins survive the frame being drained of every sample, and
  `Registry::remove_frame` is the only release (and the escape hatch for
  re-parenting). Lookups on a drained frame report `NotFoundAt` with
  `covered: None` rather than `UnknownFrame`, so routine cleanup can never
  quietly re-open a refused re-parenting or change of kind.
- **Breaking:** `Registry::new(max_age)` splits into `Registry::new()` —
  no automatic cleanup — and `Registry::with_max_age(max_age)`, and both
  exist in both feature modes: automatic cleanup no longer requires
  `std`. `Registry` also implements `Default`. Mind rustc's suggestion
  on the 1.x call site: "remove the extra argument" compiles into a
  registry that never evicts — if you had a `max_age`, you want
  `with_max_age` (MIGRATION.md break 1 spells this out).
- **Breaking:** every `Registry` call reports one flat error type,
  `errors::RegistryError<T>`. 1.4.1 answered a failed lookup with the
  catch-all `TransformError::NotFound(from, to)` — or, for frames in
  different trees, with `IncompatibleFrames` — and could return a
  partway-resolved chain as `Ok` across a mid-chain data gap; lookups
  now diagnose —
  `UnknownFrame` when a requested frame exists nowhere in the tree,
  `Disconnected` when both frames exist but no chain connects them, and
  `NotFoundAt` when the walk stopped at a frame that cannot serve the
  requested time, mirroring tf2's LookupException / ConnectivityException
  / ExtrapolationException — so a mid-chain data gap and frames from
  disconnected trees no longer masquerade as "frames do not have a
  parent-child relationship". `NotFoundAt` carries `frame`,
  `requested: T` and `covered: Option<(T, T)>` directly — `Some` a gap in
  data the frame holds, `None` a frame holding nothing at all — and its
  docs work through the terminating latest-available retry idiom, with a
  property test pinning its exactness for root-target lookups. The
  payloads are typed in the registry's own time type instead of
  pre-formatted seconds, so they compare against the clock the caller
  asked with; `Display` still renders seconds through
  `TimePoint::as_seconds_lossy`. Insert rejections are flat variants of
  the same enum — `NonUnitRotation`, `NonFiniteValues`,
  `SelfReferentialFrame`, `ReparentingNotSupported { current_parent }`,
  `CycleDetected`, `StaticDynamicConflict` — and the two validation
  causes are flat on every path that reports them, including the
  half-chain inversion by which a lookup rejects an overflowed
  translation, so no condition has two spellings to match on. (A lookup
  still does not re-validate its result: the ancestor-ward direction
  inverts nothing and returns such a translation as `Ok`.)
  `TransformError` shrinks to what its name says — the geometry and time
  failures of the `Transform` constructors, `inverse`, `interpolate`, `*`
  and `Transformable::transform`.
- **Breaking:** every error payload field is named, and every
  frame-related error names its frames: `TimestampMismatch { lhs, rhs }`,
  `TimestampOutOfRange { requested, start, end }`,
  `IncompatibleFrames { expected, found }`,
  `SameFrameMultiplication { frame }`,
  `Disconnected { target_frame, source_frame }`, and `NotFoundAt
  { target_frame, source_frame, frame, requested, covered }` — the
  lookup-argument fields carry the `_frame` suffix because a field literally
  named `source` belongs to the error trait's source-chaining convention.
  All public error enums are `#[non_exhaustive]`.
- **Breaking:** `Buffer` and the `core` module are private, and
  `Registry`'s internal storage is too. `Registry` — at the crate root,
  where it was already re-exported — is the entire public entry point.
  The standalone-buffer API is gone with it, `BufferError` included: a
  single buffer enforces only its own parent and child pins, while the
  cycle check needs a view of the whole tree and lives in `Registry`, so
  hand-held buffers were a second, weaker way to store transforms for a
  use case nobody had — one that could close a cycle nothing would
  reject. Its storage is now free to change without a major release.
- **Breaking:** every public type has a single canonical path
  (`geometry::Point`, `time::Timestamp`, ...): the leaf modules are private,
  matching the error-module pattern. Error types live at `errors::*`.
- **Breaking:** `TimePoint` is pure time arithmetic, in exactly the three
  methods the core calls — `duration_since`, `checked_sub`, and
  `as_seconds_lossy` — and gains `Debug` as a supertrait next to
  `Copy + Ord`. `static_timestamp()` and `is_static()` are gone with the
  sentinel (the trait is unsealed and `is_static` was overridable
  independently of `static_timestamp`, silently breaking kind detection),
  and by 2.0 neither `checked_add` nor `as_seconds` had a call site
  left (`Stamp` took over the timestamp arithmetic, `as_seconds_lossy`
  the error formatting) — on a soft-float MCU target an unused checked
  conversion is a double-precision divide nobody asked for. `as_seconds_lossy` is
  required rather than defaulted, which states its contract in the type
  system: error formatting cannot fail. The `Debug` bound stops a clock
  type without its own derive from making `Transform<YourClock>`
  silently unprintable in the diagnostics that report a bad lookup.
- **Breaking:** `Timestamp`'s inner nanosecond field is private and narrows
  from `u128` to `u64`; `from_nanos(u64)` / `as_nanos() -> u64` are the API.
  u64 nanoseconds span ~584 years (mid-2554 from the Unix epoch) — past the
  service life of anything this crate positions — while shrinking per-sample
  stamp storage and removing multi-word arithmetic from the 32-bit MCU
  targets. The stamp's integer narrows from 16 bytes to 8 and sheds the
  16-byte alignment it imposed on everything stored beside it: measured on
  x86-64, `Transform<Timestamp>` is 120 bytes where the `u128` made it 128,
  and the buffer's `BTreeMap` key goes from 16 bytes to 8. On the wire the
  stamp is a native integer in every serde format (see the serde entry
  under Added): variable-width encodings write the same bytes they would
  for a `u128` of equal value, bincode 1.x and bincode 2's
  `config::legacy()` write eight fixed bytes, and MessagePack emits a
  native integer rather than the 16-byte blob a `u128` forces.
  `Timestamp::now()` gains a second (2554) way to panic, and `try_now`
  reports it as `TimeError::DurationOverflow`.
- **Breaking:** `Timestamp::as_seconds_unchecked` is renamed
  `as_seconds_lossy`, matching the `TimePoint` vocabulary — the operation
  is lossy, not unsafe. `Timestamp::as_seconds` gains an honest accuracy
  contract: it errs beyond 2^53 nanoseconds (~104 days), where `f64`
  loses sub-nanosecond accuracy.
- **Breaking:** `Registry::delete_transforms_before` is renamed
  `Registry::remove_transforms_before`. The crate spelled one operation two
  ways — `remove_frame` next to `delete_transforms_before` — inviting readers
  to look for a distinction that never existed.
- **Breaking:** `==` on geometry types is exact IEEE 754 equality; the
  unsound `Eq` impl on `Transform` and the `PartialOrd` derives on
  `Quaternion`, `Vector3`, and `Point` are removed. Tolerant comparison
  lives in the `approx` traits (`AbsDiffEq`/`RelativeEq`, now implemented
  for all geometry types).
- **Breaking:** `Quaternion` keeps only its rotation algebra — `*`,
  `conjugate`, `normalize`, `norm`, `rotate_vector`, `slerp` and the
  constructors. The vector-space surface leaves the public API: `+`, `-`,
  `/` (with its `QuaternionError::DivisionByZero`) and the `Default` impl
  had no caller outside their own unit tests, `norm_squared` served only
  the removed `/`, and `scale` survives privately inside `normalize` and
  `slerp`. Summing rotations and renormalizing is the classic silent
  wrong answer, and `/` returned `Ok` with all-NaN components for a NaN
  divisor and an all-zero, finite quaternion for an overflowing one.
  `q2 / q1` on unit quaternions is `q2 * q1.conjugate()` up to the
  divisor's norm drift — the conjugate spelling is the more accurate.
  Slerp's blend moved to a private helper, unchanged bit for bit under
  the pinned slerp tests.
- **Breaking:** `Vector3` loses `dot`, `cross` and
  `unit_x`/`unit_y`/`unit_z` — `dot` and `cross` were exercised only by
  their own unit tests, the unit constructors by nothing at all, the
  crate already points more general needs at a linear-algebra library,
  and the public components make each a one-liner. The operator set
  stays complete: `+`, `-`, both scalar multiplications and `/` remain.
- **Breaking:** `Point` gains `Point::new(position, orientation, timestamp,
  frame)` and `#[non_exhaustive]`, which ends its struct literal. Its fields
  stay public: a point is a data record, not an invariant carrier — the
  invariants live on the `Transform` applied to it.
- **Behavior change:** float math is `libm`'s in every feature mode. 1.x
  used the platform's own `sqrt`/`sin`/`acos` — so an interpolated
  rotation depended on the host's math library, and a desktop replay of a
  flight log could differ from the flight controller that produced it, in
  a domain where the two are compared to decide which one was right. The
  `std` feature now changes which API exists, never a computed value.
  Against `std` math on glibc this moves interpolated rotations by up to
  four ulps per component (measured over a sweep of arcs and
  interpolation factors); `sqrt` is unaffected, being correctly rounded
  everywhere. Three slerp cases — interior, near-antipodal, and
  near-identity — are pinned bit for bit and run in both feature modes.
- **Behavior change:** a lookup inverts at most once. Both halves of a
  resolved chain are composed in their natural direction, deleting a
  reverse-and-invert pass that cost one inversion per hop plus one at the
  end — in the crate's own documented direction
  (`get_transform("map", "lidar", t)`), the one the README teaches. Measured
  on x86-64: a single hop drops from 11 heap allocations (1456 B) to 5
  (488 B), four hops from 23 to 11, and 64 hops from 271 to 135. That
  direction is no longer renormalized on the way out, which is what makes a
  single-hop lookup at a stored timestamp return the stored transform bit for
  bit; in 1.x it came back through two inversions, up to ten ulps off in
  the translation.
- **Behavior change:** same-frame lookups succeed: `get_transform(x, x, t)`
  returns the identity transform instead of an error. Lookup results
  always carry the requested timestamp, also over all-static chains, and
  a static transform applies to data of any timestamp through
  `Transformable`.
- **Behavior change:** there is no extrapolation anywhere. An
  out-of-range `Transform::interpolate` reports `TimestampOutOfRange`
  with the requested time and both range endpoints (a registry lookup
  reports `NotFoundAt` with the frame's covered range), and
  `Quaternion::slerp` clamps its factor to `[0.0, 1.0]`.
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
- `get_transform_at` composes its two legs through a private
  time-agnostic path instead of stamping both legs static to bypass
  `Mul`'s timestamp check, as 1.x did.
- Error `Display` messages are lowercase per the Rust API guidelines,
  and their exact strings are documented as not a stability surface.
- Crate upgraded to edition 2024; MSRV is 1.86, verified in CI.

### Removed

- `Transform::identity()`. It returned empty parent and child frames —
  self-referential, so it could be neither inserted nor composed — and
  existed only as a base for field-poking, which private fields end. The
  registry synthesizes its own identity for same-frame lookups.
- `TransformError::NotFound` — 1.x's primary lookup-miss error since
  1.0 — replaced by the diagnosed `UnknownFrame` / `Disconnected` /
  `NotFoundAt` reported through `RegistryError`.
- `TransformError::TransformTreeEmpty`. 1.4.1 produced it from a
  same-frame lookup on a frame that held data; that lookup now returns
  the identity, and its only remaining `return` sat on an empty chain no
  public call can reach. Removing an enum variant after stable would be
  a breaking change, so it goes now.
- The deprecated `TimestampError` alias (use `TimeError`) and the
  never-produced `BufferError::MaxAgeInvalid` variant. `BufferError`
  itself is gone with `Buffer` — see the privatization entry.

### Added

- MIGRATION.md: a migration guide from 1.4.1, organized by compile
  errors first and runtime behavior changes second, with a dedicated
  section for users coming from a 2.0.0 pre-release. It ships in the
  published crate alongside this changelog.
- Optional, default-off `serde` feature for the geometry and time types,
  `no_std`-compatible, with the wire format frozen by golden-byte tests.
  `Stamp` is an explicitly tagged enum — `{"At": 1753142400000000000}`
  and `"Static"` in JSON, a variant index ahead of the payload in
  non-self-describing formats, the width of index and integer belonging
  to the codec (postcard and bincode 2's `config::standard()` write a
  1-byte index and a LEB128 varint; bincode 1.x and bincode 2's
  `config::legacy()` a fixed 4-byte index and fixed-width integer) — and
  `Timestamp` is `#[serde(transparent)]`, a bare nanosecond integer
  rather than a one-field record. No magic value appears on the wire and
  staticness is always spelled: a `timestamp` that is *missing* or
  `null` is a decode error rather than a silently minted eternal static
  transform. Deserializing a `Transform` runs constructor validation, so
  a denormalized rotation or non-finite component on the wire is a
  decode error too. The `std` feature forwards `serde?/std`, so
  `Transform<SystemTime>` serializes for downstream users who enable
  `std` + `serde` without depending on serde's `std` feature themselves.
- `Timestamp::try_now()`: panic-free counterpart of `now()`, returning
  `TimeError::DurationUnderflow` on a pre-epoch system clock and
  `TimeError::DurationOverflow` on one set past the u64 nanosecond range.
- `Quaternion::from_wxyz(w, x, y, z)`: the constructor states its
  component order in its name, because the other common convention puts
  the scalar part last and a caller passing `(x, y, z, w)` builds a
  perfectly valid unit quaternion describing the wrong rotation — an
  error no validation can catch. `Quaternion` literals still work.
- `Timestamp::from_nanos` / `as_nanos`, `Registry::remove_frame`,
  `Transform::validate`, and `TimePoint::as_seconds_lossy` (required;
  implement it as best-effort, yielding `NaN` over a plausible-looking
  number, because it formats error messages).
- `TransformError::StaticInterpolation`: a static transform used as an
  interpolation endpoint is rejected explicitly instead of falling
  through to an incidental error.
- `QuaternionError::NonFinite`: `Quaternion::normalize` rejects
  non-finite inputs explicitly — 1.x normalized a NaN quaternion to
  `Ok` with all-NaN components.
- Two of the six examples (`std_full`, `no_std_full`) now demonstrate a
  static sensor mount chained with dynamic edges — the feature was
  previously unexercised outside the test suite.
- Golden vectors computed outside the crate (`tests/golden_vectors.rs`):
  five poses derived with SciPy and asserted against literal digits — a
  quarter turn of yaw plus an offset applied to a known point, a two-hop
  chain in both directions, a rotation about no coordinate axis, and an
  interpolated lookup against a reference slerp. Every other test builds
  its expectation with the same conventions as the code under test, so a
  convention flipped consistently — a transposed rotation, a swapped
  quaternion product — passed all of them.
- Property-based test suite (proptest) covering the core invariants;
  fully deterministic test fixtures and non-mutating benchmarks; panic
  policy enforced with clippy restriction lints and documented in the
  crate-level Reliability section. All public types are `Send + Sync`,
  documented and compile-asserted.
- Behavioral pin tests for commitments that freeze at stable: duplicate-
  timestamp upserts, `SameFrameMultiplication`, `max_age` boundary
  semantics (`Duration::ZERO`, inclusive boundary, out-of-order inserts),
  static transforms over custom clocks including range extremes,
  interior-point and near-antipodal slerp, `Point` error paths, mid-tree
  `remove_frame`, exact `NotFoundAt` payloads, and postcard golden-bytes
  tests for both `Stamp` arms freezing the serde wire format for
  non-self-describing formats (struct field order and `Stamp`'s variant
  order are part of the wire contract). Plus the tests a mutation run
  demanded: `inverse` renormalizing a rotation that drifted within
  tolerance, the exact acceptance boundary around `UNIT_NORM_TOLERANCE`,
  slerp's lerp/slerp switchover, `interpolate` returning the earlier
  endpoint when both endpoints share a stamp, the post-truncation chain
  lengths of a deep common trunk, and error formatting under a clock
  whose instants cannot be expressed as seconds.
- Benchmarks for the shapes real users hit: a realistic 6-edge robot tree
  (mixed static/dynamic), a 3-hop interpolating dynamic chain,
  `get_transform_at`, a 100k-resident insert bench pinning the eviction fix
  at depth, a 4-hop chain measured in both lookup directions, and a
  lookup past the newest sample — the failure a consumer running ahead of
  its publisher hits every tick. Dynamic samples rotate about a
  non-axis-aligned axis, so interpolation runs slerp's sin-weighted
  branch — the dominant float cost on a soft-float target.
- CI runs the full verification gate — `tests/test_all.sh` verbatim in a
  `gate` job, making the script the single source of truth: builds,
  tests, clippy and rustdoc at `-D warnings` across all four feature
  combinations, formatting, examples, bench smoke runs, and bare-metal
  builds for `thumbv7em-none-eabihf`, `thumbv6m-none-eabi` and
  `thumbv8m.main-none-eabihf` (CI adds `riscv32imc-unknown-none-elf`,
  a native ARM64 test job, an MSRV check on 1.86, and `cargo audit`).
- `Cargo.lock` is committed, so every checkout resolves the same dependency
  versions. It also ships in the published tarball — `cargo package` includes
  it — where it governs builds of this crate itself and never a downstream
  crate's own resolution.

### Fixed

- `get_transform` verifies that the resolved chain actually connects the two
  requested frames; querying an unknown frame previously returned the
  transform to the tree root instead of an error.
- Static and dynamic transforms can no longer mix within one child
  frame, which previously corrupted interpolation or silently shadowed
  data: in 1.x the most recent insert decided how the whole buffer was
  read, so one query could answer three different ways across three
  inserts. Mixing is rejected with `StaticDynamicConflict`; chaining
  static and dynamic *frames* is unaffected.
- `t = 0` is usable as an ordinary dynamic timestamp: the most natural
  insert loop there is (`for i in 0.. { insert(tf(i * step)) }`) no
  longer turns its first sample into an eternal static transform — in
  1.x the zero sentinel made insert `i = 0` static and served it at
  every instant, each later insert re-deciding the buffer's kind.
  Pinned by a regression test, and the property-test timestamp strategy
  includes zero.
- `Transform` multiplication only accepts valid compositions
  (`t_a_b * t_b_c`); the reversed operand order produced a frame-inconsistent
  result.
- Manual cleanup (`remove_transforms_before`) no longer destroys static
  transforms — a static transform is valid for all time.
- `Registry::remove_transforms_before` resets each frame's `max_age` expiry
  reference along with its samples: previously a wiped buffer kept its
  pre-wipe latest timestamp, so a restarted stream at earlier times was
  evicted by the very insert that added it — the insert reported no
  error and the frame stayed empty.
- `get_transform_at` resolves when `source` equals the fixed frame
  (including all three frames equal) instead of always failing with
  `SameFrameMultiplication`; coinciding-frame legs are short-circuited
  rather than composed with a self-referential identity.
- Error diagnostics survive wall-clock timestamps: messages are formatted via
  the infallible `TimePoint::as_seconds_lossy`, so a conversion error can no
  longer mask the error being reported.
- `no_std` works on real bare-metal targets: float math goes through
  `libm` and dependencies no longer pull in `std`. A heap allocator
  (`alloc`) is required.
- Docs: `Registry::new` states what its lack of a `max_age` costs — not
  only unbounded retention, but an unbounded interpolation gap, since a
  lookup between samples recorded either side of a publisher stall
  interpolates straight across it. `Registry::with_max_age` bounds both,
  and the `Default` impl points at the same explanation.
- Docs: duplicate-timestamp inserts are documented as last-write-wins
  upserts; `remove_frame` documents that it strands descendants of a
  mid-tree frame; interpolation is documented to span interior gaps of any
  size (bounding freshness is the caller's job); the O(log n) lookup claim is
  qualified (per-frame; linear in chain depth; O(frames) failure
  diagnosis); the `approx` 0.5 public-API commitment is recorded;
  allocation-failure behavior and the deterministic-hasher trade-off are
  stated for `no_std`.
- Docs: the scalar type is a commitment, not an accident. f32 and
  mixed-precision arithmetic are Non-Goals (README and crate root, one
  identical list), and the README publishes the envelope that commitment
  implies: measured per-operation cost and allocation counts on x86-64,
  about 320 B of resident heap per stored sample while both frame names are
  32 characters or shorter, and a platform × rate × chain-depth table that
  says which workloads fit an MCU and which run out of SRAM first.
- Docs: `Quaternion::normalize` documents the threshold it actually applies
  — a norm below `f64::EPSILON` — instead of a figure that was wrong by
  eight orders of magnitude in the permissive direction; a doctest pins
  both sides of the boundary.
- Docs: `Transformable` states the map an implementation owes — rotate,
  then translate; orientation composed with the transform's rotation on
  the left; a free vector takes the rotation only — and a `Point` test
  pins the left composition against hand-derived digits from a
  non-commuting starting orientation.
- Docs: the README concurrency snippet uses `RwLock`, matching the `&self`
  read design it exists to demonstrate, and points at `examples/std_full.rs`
  as the compiled version. That example publishes each sample at the
  instant the sample describes and reads at a stamp its publisher already
  covers, which is what no extrapolation requires of a reader.
- Docs: serde feature-gating is stated on every serde-capable type
  (rustdoc cannot banner derive-generated impls — verified against the
  docs.rs configuration, which the gate builds).

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

[2.2.0]: https://github.com/deniz-hofmeister/transforms/compare/v2.1.0...master
[2.1.0]: https://github.com/deniz-hofmeister/transforms/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/deniz-hofmeister/transforms/compare/v1.4.1...v2.0.0
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
