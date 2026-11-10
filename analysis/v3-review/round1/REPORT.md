# transforms v3: review and proposal

Date: 2026-11-10. Baseline: master `7cc2a65` (released v2.3.0 plus docs-only
commits). Nothing here is implemented. Opinions, not decisions, with one
exception: item 10 was added on maintainer review the same day.

This report reviews the v3 material in
[`analysis/v3-proposals/`](../../v3-proposals/), the ledger in
[`analysis/v3-warrant/REPORT.md`](../../v3-warrant/REPORT.md) and the dependent
study in [`analysis/dependents/`](../../dependents/README.md), and proposes
what a 3.0 should contain. The notes were treated as inspiration, not as a
specification.

## Verdict

- **The notes name the right hazards and over-build the fixes.** A public
  `Rotation` newtype, a translation bound, a per-composition Newton step, a
  `Resolved` result type, a required `max_age`, `TransformError<T>` and
  frame fields on every insert error are all larger than what they close.
- **The proposed 3.0 is smaller than the ledger.** Three behaviour fixes
  that only a major can make, one diagnosis flip, one new insert error,
  three small error-shape repairs and four deletions. One new public type
  and one new error variant. The public surface shrinks.
- **A v3 of this size is warranted after 2.4, and is not urgent.** No user
  has hit any of its hazards. The known downstream cost is one public
  signature and three test lines in roslibrust_transforms, one match
  arm in transforms_io if the optional rename is taken, and the insert
  behaviour of item 10 in every adapter that sets `max_age`.
- **The stress test cut three ideas:** operators to named methods, frame
  fields on insert errors, and an error for expired inserts. The first two
  are listed under [Not in 3.0](#not-in-30). The maintainer reinstated the
  third; it is [item 10](#10-an-insert-the-window-discards-is-an-error).

## Method

- **Scope.** The request listed six lenses: performance, safety,
  ergonomics, errors, fixes over v2, and dependents. Four were added
  before the review started, because the six left them uncovered:
  - embedded targets, the scalar type and cross-target determinism;
  - time, topology and lifecycle (split from geometry, since the dependent
    study puts the reachable silent answers there);
  - prior art, by web research;
  - migration, wire format, release strategy and the coherence of the v3
    documents with each other.
- **Round 1.** One reviewer per lens, each with probes against the crate,
  followed by an independent skeptic told to refute every finding
  (20 agents). Statuses below are the skeptics'.
- **Round 2.** A draft proposal was attacked by five critics (minimalism,
  alternative designs, dependent cost from real sources, demand search on
  GitHub, consistency against the code and `AGENTS.md`), and its three
  contested behaviour changes were prototyped in scratch copies and run
  against the existing test suite (8 agents). The library diffs are in
  [`prototypes/`](prototypes/).
- **Not done.** See [Limits](#limits). Nothing was measured on hardware,
  the gate ran on no prototype, and no dependent's code was compiled.

## The proposed 3.0

### 1. A cross-time result type

`get_transform_at` returns `geometry::CrossTimeTransform<T>` instead of a
`Transform`.

- **Shape.** Private fields. Accessors `target_frame()`, `target_time()`,
  `source_frame()`, `source_time()`, `translation()`, `rotation()`.
  Derives `Debug`, `Clone`, `PartialEq`. No `Mul`, `inverse`, serde,
  conversion into `Transform`, `fixed_frame` field or apply method. The
  method keeps its name and argument order.
- **What it closes.** Today's result carries only the target stamp, can be
  inserted, and composes as a single-time transform
  (`cross_time_result_keeps_only_target_stamp_and_remains_insertable`).
  The type blocks accidental insertion and composition. It does not block
  deliberate forging through `Transform::new`, which is no worse than
  forging any sample.
- **Why accessors and no apply.** Every studied user converts results to
  its own math types, so a result without geometry accessors is useless to
  them. With `Point` deleted (item 7) there is no checked path left to be
  inverted.
- **Why not delete the method.** A probe rebuilt the answer on the 2.3
  public API in five statements. It was bit-identical on 7 of 9 cases,
  failed with `SameFrameMultiplication` when an endpoint equals the fixed
  frame, and needed one leg restamped through `Transform::new`, which
  defeats the equal-stamp guard. That is a wrong-answer generator, so the
  scope rule keeps the method in the core.
- **Open detail.** The two legs are resolved as `(fixed, source)` and
  `(fixed, target)`, so a leg error can put the caller's target in a
  `source_frame` field (`src/core/registry/mod.rs:1101-1113`). Decide the
  payload when the type is built.
- **Changes to the [cross-time addendum](../../v3-proposals/transforms-v3-addendum-cross-time.md).**
  `transform_point` is dropped with `Point`. The accessors return
  `Quaternion`, since no `Rotation` type is proposed.

### 2. Rotations are unit where they are built

`Transform::new`, `static_between` and `Deserialize` normalize the rotation
after the existing tolerance check.

- **The hazard is the accepted band, not drift.** A rotation with norm
  `1 + 0.9e-6` is accepted and stored as given. Applying it scales by the
  squared norm: +1.8e-6 on one hop, +7.2e-6 over four hops, and up to
  2.6e-5 relative on translations in a random forest of such edges
  (probes). Forward lookups scale and inverse lookups normalize, so a
  round trip does not return the input. The warrant report rated the
  payoff small from the drift figure (4e-17 per composition); the band is
  the larger effect.
- **Reach.** Quaternions from `f32` sources sit about 1e-8 to 1e-7 off
  unit norm (arithmetic, not measured on data). That is inside the
  tolerance, so most ROS traffic is stored unnormalized today.
- **A dead band is required.** Unconditional normalization is not
  idempotent: normalizing again changes bits in 15.6% of exact-unit inputs
  (1e7 samples). The prototype leaves the rotation untouched when its
  squared norm is within `8 * f64::EPSILON` of 1. After one normalization
  the observed maximum deviation was 4 EPS over 3e7 samples; the
  first-order bound of about 7 EPS is unproven. With the band, rebuilding
  a built value is a fixed point.
- **Alternatives probed and rejected.**
  - Normalizing only inside composition leaves single-hop results at
    +1.8e-6 and hands adapters a non-unit quaternion.
  - A scale-invariant `rotate_vector` leaves the composed rotation at norm
    +3.6e-6, past the insert tolerance.
  - A public `Rotation` newtype closes the same paths and breaks every
    construction site; `Quaternion` keeps its public fields.
  - Tightening the tolerance would reject `f32`-sourced data.
- **Declared silent change.** Rotations in the band are stored
  normalized, so `tf.rotation()` no longer equals the quaternion passed
  in, and round-trip bytes of such data change. For exactly normalized
  inputs, results differ at the 1e-15 level (28% of lookups in the forest
  probe changed last bits, from item 3). A single-hop lookup still returns
  the stored transform bit for bit.
- **Not included.** The Newton step: lookup chains drift about 1.8e-15 at
  depth 20, and a user loop over `*` needs about 2e10 compositions to
  reach the insert tolerance. Such loops own their renormalization, by
  rebuilding through `Transform::new`. The bound `B` is not needed either
  (item 4).

### 3. `inverse` is the conjugate

- With item 2, no built rotation sits at the tolerance edge, so the
  normalization in `Isometry::inverse` and the
  `TransformError::QuaternionError` variant go. An inverting one-hop
  lookup was 10-13% faster in the prototype.
- This retires the `AGENTS.md` sentence "Anything that inverts a rotation
  must normalize first" and reverses v3-1's retraction of the same idea.
  The premise of that retraction, accepted inputs at the tolerance edge,
  is what item 2 removes.
- **Residual.** `Buffer::insert` validates and does not normalize. A
  derived transform that a caller drifted to within 1e-6 and re-published
  is inverted with up to 2e-6 relative error. `Transform::inverse` stays
  fallible with `NonFiniteValues`.

### 4. No lookup returns a non-finite value

- One finiteness check at the exit of `get_transform` and
  `get_transform_at`, reporting the existing `NonFiniteValues`.
- Today `Ok(inf)` is not only the documented ancestor-ward case. Sibling
  lookups return it in both directions, and `get_transform_at` has no
  check (probe).
- The cost was within timing noise on host. Non-finite values are sticky
  through the arithmetic, so one check at the exit is exact.
- **Not on `*`.** A gate there closes no lookup hazard, and the test suite
  deliberately mints infinite transforms through `*` to exercise the
  insert check. `Buffer::insert` keeps `Transform::validate`.
- This replaces the bound `B = 1e100`, which closes the same hazard with
  a constant, a narrower decode and a second rule for `Point`.
- It changes a documented behaviour (`Ok(inf)` becomes `Err`), so it is
  listed for 3.0. It is the one item here that a maintainer ruling could
  move to 2.x, as D5 did for empty names.

### 5. `Disconnected` before `NotFoundAt`

- Precedence becomes `UnknownFrame > Disconnected > NotFoundAt` for
  lookups. `latest_common_time` already decides this way.
- Today two known frames in different trees report `NotFoundAt` with a
  covered range when the data misses the instant. That tells a waiter that
  more data may help. It never will.
- **The minimal change** is deleting the `walk_failure` fallback in
  `diagnose_not_found` (`src/core/registry/mod.rs:963`).
- **The resolver rewrite is a separate change and is not part of the
  warrant.** The clean topology-first prototype is net −241 library lines,
  bit-identical on values over more than 1M comparisons, and differs only
  in this one error class. Its performance is mixed on host: up to 3.7x
  faster for interpolated lookups that skip a long shared trunk, 1.4x
  slower for exact-stamp lookups into an inner ancestor, 9-17% slower for
  interpolated root-ward chains, and no fewer allocations. Take it only as
  a deletion, and only if a variant recovers the root-ward cost. The 2.x
  fast path with a fallback is rejected: two resolvers can diverge.
- **Declared silent change.** A caller that retries on `NotFoundAt` stops
  on a disconnected pair. No studied user matches a lookup variant.

### 6. Error shapes

- **Struct variants become `#[non_exhaustive]`** (`Disconnected`,
  `NotFoundAt`, `NoCommonTime` and the parent-pin variant in
  `RegistryError`; `IncompatibleFrames` and `SameFrameMultiplication` in
  `TransformError`), so a field can be added in a minor. Tuple and unit
  variants stay frozen.
- **`TimestampMismatch` becomes a unit variant.** After items 7 and 8 its
  only producer is `*`, whose caller holds both operands. The `f64`
  seconds payload prints equal values for stamps 1 ns apart at epoch
  scale (probe). This replaces `TransformError<T>`, which would add a
  type parameter for one variant and break roslibrust's public conversion
  trait.
- **Optional: rename `ReparentingNotSupported` to `ParentMismatch`.** Its
  rustdoc defers the rename to 3.0, and "not supported" misleads now that
  `reparent_frame` exists. It is the one known break in transforms_io
  (`mt_pubsub_backend.rs:342`). It is cosmetic and carries no weight in
  the warrant.

### 7. Delete `Point`, `Transformable`, `Localized`, `get_transform_for`

- **Evidence.** No published dependent calls them (grep of six users'
  sources; GitHub code search found none, which is weak evidence because
  the search tokenizes `::` poorly). `Point` is unvalidated: a NaN
  position or a zero quaternion goes through `transform` as `Ok`. It
  allocates a `String` per element, and it is a pose under the name
  Point. Each trait has one implementor.
- **Demand that does exist.** Three people used `Point` in 0.3.x and 1.x
  issues and discussions (#43, #45, #65/#66), and the maintainer's #76
  wanted a helper. The same threads show `Point` confusing its users.
- **No replacement at 3.0.** A checked apply needs only caller-supplied
  labels and the public accessors, so it is buildable outside. If a
  caller appears, the shape is one method on `Transform` that checks the
  frame and stamp once and maps a slice, not a hardened `Point`.
- **Coupling.** With no library-side apply, users apply rotations by hand.
  Item 2 is what makes that safe.
- **Obligations.** Rewrite the README headline example, show the one-line
  apply with its direction in the docs, and re-point
  `tests/golden_vectors.rs:99-106,205-212` and `tests/properties.rs:205`
  without touching the expected numbers. `Point`'s pinned wire format
  (`tests/serde.rs:67,313`) stops decoding.
- **Dissent.** The geometry lens would harden `Point` instead: private
  fields and a fallible constructor. That adds surface for a type no
  dependent uses.

### 8. Delete `Transform::interpolate`

- No registry path calls it; the buffer interpolates on `Isometry`. It is
  the only producer of `StaticInterpolation` and `TimestampOutOfRange`,
  and of the `"a -> b"` string form of `IncompatibleFrames`. It can be
  rebuilt on the public `Quaternion::slerp` and a lerp (not probed).
- No user found. `tests/properties.rs:751` uses it as the oracle for
  registry interpolation and needs an in-test reference instead.

### 9. Delete `Timestamp::as_seconds`

- It returns `AccuracyLoss` above 2^53 ns, about 104 days, so it is `Err`
  for every wall-clock stamp. `as_seconds_lossy` stays.
- `TimeError::AccuracyLoss` stays. A custom clock uses it, and
  `tests/time_traits.rs:277,317-329` pin that path.

### 10. An insert the window discards is an error

Added on maintainer review, 2026-11-10. Three lenses had ranked this the
most serious silent defect not in the ledger, and the stress test turned
the fix down. The reasons it gave are kept below as accepted costs.

- **The hazard.** On a `with_max_age` registry, an insert older than the
  frame's newest stamp minus `max_age` is stored, evicted at once, and
  reported as `Ok(())`. One future-stamped sample therefore evicts a
  frame's history, and every later normal insert vanishes with `Ok(())`
  (probe). A clock reset behaves the same.
- **The change.** Such an insert returns
  `RegistryError::SampleExpired { frame, stamp, newest }` and stores
  nothing. The boundary is the eviction boundary: a sample exactly at
  newest minus `max_age` is accepted, a strictly older one is rejected.
  Where newest is smaller than `max_age` the sample is accepted, as in
  eviction. A registry without `max_age` never reports it, static frames
  are unaffected, and the `reparent_frame` seed is exempt because it
  replaces the history.
- **Batches fail as a whole** (maintainer decision). `add_transforms`
  stays atomic and equal to the insert loop. Each member is checked
  against the frame's running newest stamp, which includes earlier
  members of the same batch, and the first expired member rejects the
  batch with nothing stored. Order inside a batch therefore matters, as
  it does in the loop: with a 10 s window, one frame at `[50 s, 100 s]`
  is accepted and `[100 s, 50 s]` is rejected.
- **Alternative considered: check against the batch's newest stamp.**
  That rule rejects `[50 s, 100 s]` too. It is independent of order, and
  it refuses a batch that carries a future-stamped glitch after a normal
  sample, which the loop rule accepts. It was not taken because the batch
  would stop equalling the loop: the `add_transforms` rustdoc promises
  the state and the first error of inserting in order, and
  `add_transforms_matches_the_add_transform_loop_or_changes_nothing`
  (`tests/properties.rs:640`) pins it. Both stay unchanged under the loop
  rule.
- **Why an error.** The crate never consults a wall clock, so it cannot
  tell a future-stamped glitch from new data and cannot refuse the poison
  itself. Reporting the discarded sample is the only thing the core can
  do, and `Ok(())` for a sample that is not stored is a false return
  value. `newest` lets a caller that has a clock tell a late sample from
  a poisoned edge. The check is buildable outside for a single insert
  (`latest_common_time` on one hop returns the edge's newest stamp), but
  a batch needs the running-newest logic, which is the copy that goes
  wrong.
- **What it does not do.**
  - The poison insert is still accepted. The error fires on the next
    normal insert.
  - A jump smaller than `max_age` still interleaves two epochs silently.
    The docs must not describe the error as reset detection.
  - Lookups do not change. A lookup driven by `latest_common_time` keeps
    serving the frozen pose; a lookup at a real stamp already fails with
    `NotFoundAt`.
- **Costs accepted.**
  - Routine late arrival beyond `max_age` becomes `Err`, which aborts an
    insert loop that uses `?`.
  - One expired member rejects a whole TF message in dimos (PR #4722,
    open), where 2.3 stores the rest. After a poisoned frame, the healthy
    frames in the same message stop updating until the caller filters by
    the `frame` in the error and retries.
  - transforms_io and roslibrust_transforms log per rejected transform
    and expose no reset call, so they log once per message.
  - The error does not carry the rejected transform. A caller that wants
    to retry clones before the call.
  - `Buffer::check` reads stored state and takes the batch's running
    newest stamp. Its rustdoc invariant, that an accepted check implies
    an accepted insert whatever is inserted in between
    (`src/core/buffer/mod.rs:343`), no longer holds as written.
  - This reverses the dependents addendum, which voted 0 of 3 against an
    `Expired` error.
- **Recovery needs no new API.** `remove_transforms_before` past the
  newest sample drains the buffer and resets its expiry reference; it is
  registry-wide. Two `reparent_frame` calls clear one frame.
  `clear_history` is not proposed (see [Not in 3.0](#not-in-30)).
- **Compatibility.** `RegistryError` is `#[non_exhaustive]`, so nothing
  stops compiling. It is a silent row in MIGRATION, and 3.0 only: the
  `with_max_age` rustdoc describes eviction on insert and three tests pin
  the old contract.
- **Prototype.** +106/−9 library lines including `clear_history`;
  `add_transforms` equals the insert loop, including the first error and
  unchanged state on error (proptest, 2000 cases).

### What 3.0 leaves alone

`Stamp` and its serde tags, the flat `RegistryError`, the strict tree and
`reparent_frame`, descendant-preserving `remove_frame`, no extrapolation,
`libm` everywhere, `BTreeMap` history, `latest_common_time`, the
constructors `Registry::new` and `with_max_age`, the operators, the
Non-Goals, MSRV 1.85 and edition 2024. The type names, module paths and
constructor names of `Registry`, `Transform`, `Vector3`, `Quaternion`,
`Stamp`, `Timestamp`, `TimePoint`, `RegistryError`, `TransformError` and
`TimeError` do not change: they are re-exported down a chain
(transforms_io, then pelorus_syncio).

## Not in 3.0

Each row is a ledger item, a position from the v3 notes, or an idea from
this review, with the reason it is left out.

| Item | Reason |
|---|---|
| Public `Rotation`, public `Isometry` | Items 2-4 close the same hazards without a new type. About 17 construction sites in dependents would break (count not re-verified) |
| Translation bound `B` | Item 4 is exact and needs no constant. `B` narrows decoding and leaves `Point` open |
| Newton renormalization | Not needed for lookups. It changes bits on 28% of identity compositions and diverges for norms above √3. About 1 ns on host |
| `Resolved` result type; target/source on every result | A second type that closes nothing. `get_transform` keeps `Transform`; only the cross-time type says target/source |
| Operators to named methods | Closes no hazard. About 60 sites in this repository and four in roslibrust's doctests. Dissent: the ergonomics lens calls it the least idiomatic part of the API |
| Frame fields on insert errors | A single-insert caller holds the transform. Only `add_transforms` is blind, and its one user is an unmerged PR. Name-cloning in adapters stays either way, because the value errors are frame-less |
| `TransformError<T>` | Replaced by the unit `TimestampMismatch` (item 6) |
| `RegistryError::TransformError` to `Time(TimeError)` | The conversion is total only once registry composition is infallible, which needs the resolver rewrite. `tests/time_traits.rs:317` pins the current arm |
| Required `max_age`; `Registry::unbounded()` | Bounds memory only. An 8 s dropout is still interpolated inside a 10 s window. Every studied user already passes `with_max_age`. Document 122 B per sample on `new` instead (measured) |
| `clear_history` | Additive, so not a v3 item. Recovery from item 10 is buildable today: `remove_transforms_before` past the newest sample (registry-wide), or two `reparent_frame` calls for one frame. A registry-wide empty clear is not buildable for a generic `T` when a sample sits at the type's maximum. whirl_tf hand-rolls one through `remove_frame`, and dimos lost reset recovery in its migration. Revisit on a request |
| A maximum interpolation gap | Additive, so never a ledger row. No design and no request. A time bound also misses the exact hazard, which is more than 180 degrees between adjacent samples |
| `f32` or a scalar parameter | 3.0 stays `f64` with no scalar parameter, which keeps a concrete `f32` module additive later. The unmeasured hardware cost does not block 3.0 |
| `FrameId`, interning | Hashing is 8-16% of a hop (measured). `Arc` does not build on thumbv6m or riscv32imc (probe) |
| `get_latest`, `can_transform`, bulk lookups, caches, typed frames, `is_transient` | Buildable on the public API, or a new source of wrong answers |
| Self-edge and empty-name checks in `Transform::new` | `get_transform(x, x)` returns an `(x, x)` identity, which must stay constructible and serializable. Both checks stay at the insert boundary |
| Euler and axis-angle constructors, `mint`/`nalgebra`/`glam` features | Scope rule: buildable outside. No dependent builds quaternions from angles |
| approx impls | Keep. approx 0.6 has been at rc2 since 2026-02-05; decide when it is stable |
| Removing `thiserror` | Possible on MSRV 1.85, adds about 60 lines, and is unrelated to v3 |

## Ships in 2.x

None of these needs a break. Items already in the
[2.4 proposal](../../v2.4-proposal/DESIGN.md) are marked.

- Reject empty frame names (Part D) and the documentation corrections
  (Part C). Both are in the 2.4 proposal.
- Document the silent drop of expired inserts with the reset recipe
  (Part C item 5 covers the first half). 2.x keeps the drop; item 10
  replaces it in 3.0.
- One sentence for the embedded claim: `f64` runs in software on every
  gate target unless an M7 is built with `target-cpu=cortex-m7` and has a
  double-precision FPU (asm probe: 36 `__aeabi_dmul` by default, none with
  the flag).
- The `latest_common_time` example unwraps `.at()`, which panics when
  copied onto an all-static chain. Show the `match`.
- A rustdoc recipe for "can a wait outlast this error", worded as likely,
  not certain: a late backfill can make an older instant servable.
- Internal: drop `Buffer.child`, `InsertError::ChildFrameMismatch` and its
  unreachable `From` arm; store an `Isometry` in the static slot.
- `#[deprecated]` on `Point`, `Transformable`, `Localized` and
  `get_transform_for`, with a link to a discussion, as the one cheap way
  to hear from users nobody found. This needs maintainer sign-off: the
  gate runs clippy with `-D warnings`, so in-repo uses need
  `#[allow(deprecated)]`.

## Release plan

- **Order.** 2.4, then the 3.0 prototype on a branch through the full
  gate, then one `3.0.0-rc` built against transforms_io, then the release.
- **Warrant.** The warrant report's hardware trigger contradicts v3-3 §6
  (a concrete `f32` module is additive), and its deprecation trigger
  cannot fire under the 2026-11-03 stance. What remains is a maintainer
  decision. The argument for deciding yes after 2.4: item 2 changes values
  for most real traffic, and that only gets more expensive to change as
  users arrive.
- **Wire.** Postcard layouts and JSON names stay frozen. 3.0 reads every
  2.x `Transform` payload. Two things change: near-unit rotations are
  normalized on decode, and `Point` payloads stop decoding. Bit-exact
  JSON round trips need `serde_json`'s `float_roundtrip`; its default
  parser is off by one ulp on 10.5% of random values, independent of this
  change.
- **`tests/v2_compatibility.rs`.** Three tests flip: disconnected
  precedence (item 5), the rotation half of the exact-sample test
  (item 2), and the insertable cross-time result (item 1). Six stay,
  including the operators test and the `right_identity` test.
- **Other pinned tests that flip.**
  `an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts`,
  `inverse_renormalizes_a_rotation_that_drifted_off_the_unit_sphere`, and
  `add_transform_rejects_a_republished_chain_that_left_validity`, whose
  premise disappears: two tolerance-edge rotations no longer compose past
  the tolerance. Item 10 flips
  `out_of_order_insert_does_not_regress_latest_timestamp`,
  `zero_max_age_out_of_order_insert_is_ok_but_immediately_evicted` and
  `dynamic_history_matches_a_reference_map_after_upserts_expiry_and_cleanup`.
  Golden vectors, slerp bit pins, frozen postcard bytes and
  all 36 doctests passed unchanged on the item 2-4 prototype.
- **Acceptance.** Add a dual-version differential harness (2.3 beside
  3.0 with an allow-list of intended flips) and a corpus of 2.3-encoded
  blobs decoded by 3.0. `cargo semver-checks` cannot see any of the silent
  rows.
- **MIGRATION.** One row per item, tagged loud, silent or wire. Silent:
  items 2, 3, 4, 5 and 10.
- **`AGENTS.md`.** Six invariant bullets need rewording (validated where
  built, derived values not re-validated, the unit-rotation bullet, the
  lookup precedence bullet, the `TimestampOutOfRange` sentence, the buffer
  expiry bullet), and the `Point` mentions in Architecture and Style go.
- **A policy sentence to decide.** D5 let a minor reject empty names,
  while the warrant report warns against silent behaviour changes. One
  rule would classify every row: a minor may reject an input class the
  docs never promised to accept, when accepting it produced a wrong
  answer; a minor never changes the value returned for an accepted input
  or a declared error precedence. This relaxes the API-stability section,
  so it is the maintainer's to adopt.

## Decisions for the maintainer

| # | Question | Recommendation |
|---|---|---|
| 1 | Cut a 3.0 of this size after 2.4? | Yes, not urgently |
| 2 | Delete `Point` and the two traits, or harden `Point`? | Delete; deprecate in 2.x first |
| 3 | Normalize at construction with an `8 * EPSILON` band? | Yes, with a proof or a pinned proptest for the band |
| 4 | Finiteness gate on lookups: 3.0 or a 2.x fix? | 3.0, unless ruled a bug fix like D5 |
| 5 | Resolver rewrite after the precedence flip? | Only as a deletion that does not slow root-ward lookups |
| 6 | Rename `ReparentingNotSupported`? | Optional; take it only if 3.0 happens |
| 7 | Keep the operators? | Yes |
| 8 | Error for expired inserts? | Decided 2026-11-10: yes, in 3.0 (item 10); `add_transforms` fails as a whole. The review had recommended no |
| 9 | Adopt the policy sentence? | Yes |

## Corrections to earlier documents

None is applied here. Each was confirmed by a probe or a code read.

- **v3-3 finding 2 and §1.** `Ok(inf)` is not ancestor-ward only; sibling
  lookups and `get_transform_at` return it too. "Stored normalized"
  glosses over non-idempotent normalization.
- **v3-3 finding 10.** The missing `alloc::sync` also affects
  riscv32imc, a CI target.
- **v3-3 §2.** "target/source for anything a query returns" implies a new
  return type for `get_transform`, which conflicts with its own §7.
- **Warrant report, `Rotation` row.** "Small user payoff" rests on drift.
  The accepted band is the effect.
- **Warrant report, triggers.** See [Release plan](#release-plan).
- **Warrant report, ergonomics item 3.** The example comments "transform
  from camera to base" read consistently as mapping camera coordinates
  into base. That is a vocabulary difference, not drift.
- **Dependents README.** "Per-edge clock-reset detection needs access the
  API does not expose" is wrong: a single-hop `latest_common_time`
  returns the edge's newest stamp.
- **Dependents study, dimos.** PR #4722 was rebased on 2026-11-09; its
  head is `fc941c1`, not `92936a7`. It is still open.
- **Cross-time addendum.** "An additive type helps only callers who
  migrate" is overstated: `#[deprecated]` warns every direct caller. Only
  transitive users are unprotected.
- **v3-2 and the perf notes.** Live memory is 122 B per sample, not the
  64 B payload: sequentially appended B-tree nodes stay about half full.

## Limits

- Nothing ran on hardware or under QEMU. All timings are host x86-64.
  thumbv6m and thumbv8m are not installed locally.
- `tests/test_all.sh` ran on no prototype. The prototypes ran
  `cargo test` with default and no default features; the item 2-4
  prototype also ran with `serde`.
- No dependent's code was compiled. Compile-error counts are from grep
  against this proposal's text.
- The dead-band bound is empirical, and `libm::sqrt` was not checked on
  soft-float targets.
- Claims about tf2, sguaba, cu-transform and schiebung come from one web
  pass and were not re-fetched by the skeptic. No decision here rests on
  them.
- Private users are invisible. crates.io lists three reverse
  dependencies.
- Not covered by any lens: lock contention under `RwLock`, allocator
  failure on small-RAM parts, and `SystemTime` as the time type in
  benchmarks.

## Prototypes

Library-only diffs against `7cc2a65`. They are evidence, not patches:
none is gate-clean, and none updates docs or tests.

| File | Content | Lines | Existing tests that fail |
|---|---|---|---|
| [`a2-normalize-conjugate-finite-gate.diff`](prototypes/a2-normalize-conjugate-finite-gate.diff) | Items 2, 3 and 4, plus a gate on `*` that this report does not propose | +51/−28 | 5, all intended flips; one comes from the `*` gate |
| [`a4-topology-first-resolver.diff`](prototypes/a4-topology-first-resolver.diff) | The clean resolver rewrite of item 5 | +75/−300 | 1, the precedence pin; three unit tests of deleted internals removed |
| [`sample-expired-clear-history.diff`](prototypes/sample-expired-clear-history.diff) | Item 10, plus `clear_history`, which this report does not propose | +106/−9 | 3, all pins of the old contract |
