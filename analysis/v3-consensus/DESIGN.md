# transforms v3: consensus design

Date: 2026-11-17. Baseline: master `adccbdd` (v2.4.0). No integrated
v3 implementation exists. This document proposes changes; the prior
maintainer rulings it keeps or reopens are identified where they apply.
Breaking changes require maintainer sign-off.

This is the outcome of a two-designer brainstorm under a new mandate from
the maintainer: v3 is a re-imagining, full refactors are allowed, and
backwards compatibility is not required. The earlier v3 material
([`v3-proposals/`](../v3-proposals/), [`v3-review/`](../v3-review/REPORT.md))
was input, not a boundary.

- **Designers.** Claude and Codex wrote independent positions and
  exchanged them over two rounds. Codex signed off subject to
  corrections in rounds 3 and 4; this document incorporates both sets.
  The final text was not sent back a fifth time
  ([`evidence/codex/`](evidence/codex/)). Claude ran as
  `claude-fable-5-2`; Codex was invoked as `gpt-6-astra` at high effort.
- **Evidence.** Six fact-finding reports (`evidence/map-*.md`), two
  prototypes with measurements, four adversarial critiques of Claude's
  first draft (`evidence/stress-*.md`), and four checks of this document
  ([`evidence/verify-findings.md`](evidence/verify-findings.md)).
  Sub-agents ran at low effort. Probes ran on one x86-64 laptop
  (i7-12850HX, `powersave` governor, other load); nothing ran on embedded
  hardware.
- **Status of the numbers.** Timings are ratios from a noisy machine and
  from a prototype that omits parts of the design ([Limits](#limits)).
  The differential counts were re-run by both designers.

## Verdict

- **The two designers converged on one design.** Two items remain split
  (M1, M10). Fourteen more are agreed but change a ruling, a
  Non-negotiable or a capability, and are the maintainer's
  ([Decisions](#decisions-for-the-maintainer)).
- **The round-2 review is sound in its measurements and narrow in its
  frame.** It answers "what is the smallest safe break?" under two rules
  the mandate removes: frozen names, paths and wire bytes, and "3.0
  changes no number that is right today". Eleven of its twelve items
  survive, most in a stronger form; item 3 (`inverse`) is replaced.
  See [Review of the review](#review-of-the-review).
- **What v3 becomes:** a labelled, owned `Transform` whose every value is
  numerically valid; a unit-by-construction `Rotation`; one concrete
  signed-nanosecond `Timestamp` with no time generics; errors grouped by
  operation family; interned frames and a topology-first resolver inside;
  one feature (`serde`); unconditional `no_std`.
- **Public surface:** 105 named items in 2.4 against 79 here, counted the
  way [`evidence/map-api.md`](evidence/map-api.md) counts (types,
  functions, constants, enum variants, public fields; trait impls
  separately). 79 includes M3 and M15 and excludes M1 and M2.

## The design

### Public API

Signatures only; documentation and routine attributes are omitted.
`Duration` is `core::time::Duration`. The crate root is flat.

```rust
// ---- geometry ----
pub struct Vector3 { pub x: f64, pub y: f64, pub z: f64 }
impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self;
}

/// A unit quaternion. Every value satisfies |n² − 1| <= 8 * f64::EPSILON,
/// where n² is the computed squared norm.
pub struct Rotation { /* private */ }
impl Rotation {
    pub const IDENTITY: Self;
    pub fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Result<Self, TransformError>;
    pub fn w(self) -> f64;
    pub fn x(self) -> f64;
    pub fn y(self) -> f64;
    pub fn z(self) -> f64;
    pub fn rotate(self, v: Vector3) -> Vector3;
}

/// Maps `source`-frame coordinates into `target`: p_target = R p_source + t.
/// Every value has a finite translation, a valid `Rotation` and non-empty
/// frame names.
pub struct Transform { /* private */ }
impl Transform {
    pub fn new(
        target: impl Into<String>,
        source: impl Into<String>,
        translation: Vector3,
        rotation: Rotation,
        stamp: Stamp,
    ) -> Result<Self, TransformError>;
    pub fn target(&self) -> &str;
    pub fn source(&self) -> &str;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Rotation;
    pub fn stamp(&self) -> Stamp;
    pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError>;
    pub fn inverse(&self) -> Result<Self, TransformError>;
}

/// The result of a cross-time lookup. Built only by `Registry`.
pub struct CrossTimeTransform { /* private */ }
impl CrossTimeTransform {
    pub fn target(&self) -> &str;
    pub fn target_time(&self) -> Timestamp;
    pub fn source(&self) -> &str;
    pub fn source_time(&self) -> Timestamp;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Rotation;
}

// ---- time ----
/// Signed nanoseconds since an epoch the caller chooses.
pub struct Timestamp { /* private i64 */ }
impl Timestamp {
    pub const MAX: Self;
    pub const fn from_nanos(nanos: i64) -> Self;
    pub const fn as_nanos(self) -> i64;
    pub fn from_secs_nanos(secs: i64, nanos: u32) -> Option<Self>;        // M3
    pub fn as_secs_nanos(self) -> (i64, u32);                             // M3
    pub fn checked_add(self, d: Duration) -> Option<Self>;                // M15
    pub fn checked_sub(self, d: Duration) -> Option<Self>;                // M15
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration>; // M15
}

pub enum Stamp { Static, At(Timestamp) }
impl Stamp {
    pub const fn at(self) -> Option<Timestamp>;
}

// ---- the tree ----
pub struct Registry { /* private */ }
impl Registry {
    pub fn with_max_age(max_age: Duration) -> Self;
    pub fn unbounded() -> Self;

    pub fn insert(&mut self, transform: &Transform) -> Result<(), InsertError>;
    pub fn insert_all(&mut self, transforms: &[Transform]) -> Result<(), BatchError>;
    pub fn reparent(&mut self, seed: &Transform) -> Result<(), InsertError>;
    pub fn remove_frame(&mut self, child: &str) -> bool;
    pub fn remove_samples_through(&mut self, cutoff: Timestamp);          // M4

    pub fn lookup(&self, target: &str, source: &str, at: Timestamp)
        -> Result<Transform, LookupError>;
    pub fn lookup_across(
        &self,
        target: &str, target_time: Timestamp,
        source: &str, source_time: Timestamp,
        fixed: &str,
    ) -> Result<CrossTimeTransform, LookupError>;
    pub fn latest_common_time(&self, target: &str, source: &str)
        -> Result<Stamp, LookupError>;
}

// ---- errors ----
#[non_exhaustive]
pub enum TransformError {
    NonFinite,
    NonUnitRotation { norm: f64 },
    EmptyFrameName,
    FrameMismatch { expected: String, found: String },
    StampMismatch { lhs: Timestamp, rhs: Timestamp },
}

#[non_exhaustive]
pub enum InsertError {
    SelfEdge { frame: String },
    ParentMismatch { frame: String, pinned: String, offered: String },
    KindMismatch { frame: String, incoming: Stamp },
    Cycle { child: String, parent: String },
    Expired { frame: String, stamp: Timestamp, newest: Timestamp },
    // `reparent` only:
    UnknownFrame { frame: String },
    NoParent { frame: String },
    ParentUnchanged { frame: String, parent: String },
}

pub struct BatchError { pub index: usize, pub error: InsertError }

#[non_exhaustive]
pub enum LookupError {
    UnknownFrame { frame: String },
    Disconnected { a: String, b: String },
    NoData { frame: String },
    OutOfRange { frame: String, requested: Timestamp, earliest: Timestamp, latest: Timestamp },
    NoCommonTime { frame: String, earliest: Timestamp, latest: Timestamp, latest_possible: Timestamp },
    NonFinite,
    NonUnitRotation { norm: f64 },
}
```

Derives and impls:

- `Vector3`, `Rotation`: `Clone`, `Copy`, exact `PartialEq`, `Debug`.
- `Timestamp`: `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
  `Hash`, `Debug`, `Display`. `Display` prints `[-]s.nnnnnnnnn`, computed
  in integers on `unsigned_abs`: −0.5 s prints `-0.500000000`, and
  `i64::MIN` prints without overflow.
- `Stamp`: `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Debug`;
  deliberately unordered.
- `Transform`, `CrossTimeTransform`: `Clone`, exact `PartialEq`, `Debug`.
  `Registry`: `Debug`.
- Errors: `Clone`, `PartialEq`, `Debug`, `Display`, `core::error::Error`.
  `BatchError::source()` returns its `InsertError`. Struct variants are
  `#[non_exhaustive]`: downstream patterns need `..`, and downstream code
  cannot construct them. `BatchError` is a plain two-field struct.
- No arithmetic operator impls. No `approx` impls. No `Default` on any
  type. Only the functions marked `const` are promised const.

Contracts the signatures do not show:

- **Edge direction.** An inserted transform is the edge whose parent is
  `target()` and whose child is `source()`, the same argument positions
  as 2.4's `(parent, child)`. `reparent(seed)` moves `seed.source()`
  under `seed.target()`. Every `frame` field of `InsertError` is the
  child. `UnknownFrame` there means the child is not currently
  known to the registry; `NoParent` means it is currently known only as
  a root.
- **Composition.** `a.compose(&b)` requires `a.source() == b.target()`,
  else `FrameMismatch { expected: a.source, found: b.target }`. The
  result maps `b.source()` into `a.target()`. Two dynamic stamps must be
  equal, else `StampMismatch`; a static operand adopts the other's
  stamp; two static operands give `Static`. `inverse` swaps the labels
  and keeps the stamp; its only failure is `NonFinite`.
- **Errors per method.** Errors are grouped by operation family; each
  method's rustdoc lists the subset it can return. `Rotation::from_wxyz`
  returns `NonFinite` or `NonUnitRotation`; `Transform::new` returns
  `NonFinite` or `EmptyFrameName`. `insert` never returns the three
  `reparent`-only variants. `latest_common_time` never returns
  `OutOfRange`, `NonFinite` or `NonUnitRotation`; the lookups never
  return `NoCommonTime`.
- **Numerical failures.** `NonFinite` reports a non-finite value. A
  finite rotation that fails the normalization postcondition reports
  `NonUnitRotation { norm }`. Neither may escape inside a successful
  transform.
- **`Rotation::rotate`** is raw numerical application: it checks no
  frame and does not promise a finite output for extreme inputs.
- **`Transform::new`** allocates both names when given borrowed strings;
  moving in a `String` avoids the copy.
- **`Timestamp::MAX`** exists as a cleanup cutoff. A dynamic sample
  inserted at `MAX` anchors bounded retention there: later samples older
  than `MAX - max_age` are rejected until that history is removed.
  Samples inside the window are still accepted, and `unbounded()`
  rejects nothing.

### What each element replaces

| v3 | Replaces in 2.4 | Reason |
|---|---|---|
| `Rotation` | `Quaternion` (public fields, unnormalized), `normalize`, `norm`, `conjugate`, `slerp`, `impl Mul`, `QuaternionError`; the tolerance check invoked from three call sites (`new`, decoding, insert) | The invariant lives in one type, and `Transform::new` cannot fail on a rotation norm. The maintained band removes the admitted 1e-6 norm error of the old type; quaternion application keeps rounding-scale error. One type replaces two: 8 named items against 15. Slerp and the quaternion product become private. |
| `Transform`, valid always | `Transform<T>` whose derived values can be off-unit or infinite; `Transform::validate`; insert-time re-validation; the `From<TransformError>` canonicalization; `static_between` | Every constructor, decoder and operation establishes the invariant, so nothing re-validates. Insert-time validation goes only once every such path does. |
| `target()` / `source()` / `stamp()` | `parent()` / `child()` / `timestamp()` | A sibling lookup has no parent-child relation. `parent`/`child` remain for topology calls and their errors. |
| `compose(&self, &Self)` / `inverse(&self)` | by-value `Mul` returning `Result`; `SameFrameMultiplication` | Fallibility is visible, operands are not consumed, `a.compose(&b)?.compose(&c)?` chains. |
| `CrossTimeTransform` | A cross-time result typed as an insertable `Transform` | Review item 1. No `fixed` accessor: the fixed frame is an assumption of the query, not something the result can verify. |
| `Timestamp` (concrete, signed) | `TimePoint`, `TimeError`, `impl TimePoint for SystemTime`, `<T>` on five public types and two traits, the `Result`-returning operators, `as_seconds`, `as_seconds_lossy`, `now`, `try_now`, `zero` | See [Time](#time). |
| `InsertError`, `LookupError`, `TransformError`, `BatchError` | Flat `RegistryError<T>` (14 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError` | Every structural insert error names its frame, and a batch error names its member. |
| `NoData` / `OutOfRange` | `NotFoundAt { covered: Option<(T, T)> }`; `NoCommonTime { covered: None }` | The `Option` payload is what callers must not misread as a timing problem; two variants make that a `match` arm. |
| `insert(&Transform)`, `insert_all(&[Transform])` | `add_transform(Transform)`, `add_transforms(impl IntoIterator)` | A rejected sample stays with the caller (the `Expired` recovery needs it). A slice gives stable indices. |
| `with_max_age` / `unbounded` | `new`, `Default` (unbounded, silently) | Every studied user sets a window; none uses `new`. A window is not an interpolation-gap policy. |
| Features: `serde` only | `std` (default), `serde` | The crate is `#![no_std]` + `alloc` always; the feature matrix halves. |
| Runtime dependencies: `thiserror`, `hashbrown`, `libm`, optional `serde` | the same plus `approx` | `approx` 0.5 trait impls are test support exposed as public API. It stays a dev-dependency. |

Renamed: `get_transform` to `lookup`; `get_transform_at` to
`lookup_across`; `add_transform` to `insert`; `add_transforms` to
`insert_all`; `reparent_frame` to `reparent`; `Quaternion::identity` to
`Rotation::IDENTITY`; `rotate_vector` to `rotate`; `IncompatibleFrames`
to `FrameMismatch`; `TimestampMismatch` to `StampMismatch`;
`NonFiniteValues` to `NonFinite`; `SelfReferentialFrame` to `SelfEdge`;
`ReparentingNotSupported` to `ParentMismatch`; `CycleDetected` to
`Cycle`; `StaticDynamicConflict` to `KindMismatch`; `SampleExpired`
(review item 10) to `Expired`; `NoParentToReplace` to `NoParent`. Lookup
errors drop the `target_frame`/`source_frame` echo and keep the failing
frame.

Deleted without replacement: `Point`, `Transformable`, `Localized`,
`get_transform_for`, `Transform::interpolate` (with `StaticInterpolation`
and `TimestampOutOfRange`), `Transform::validate`, `Vector3::zero`,
`Vector3`'s arithmetic operators, `Stamp::is_static`,
`UNIT_NORM_TOLERANCE` (the constructor doc states 1e-6), and the
`errors`, `geometry` and `time` modules.

### Numerics

- **Construction.** `Rotation::from_wxyz` rejects non-finite components
  and a norm further than 1e-6 from 1.
- **The maintenance rule.** Compute the squared norm in the established
  component order. Inside the 8-EPS band, keep the components unchanged.
  Outside it, multiply each component by `1.0 / libm::sqrt(n²)`, then
  check finiteness and the band postcondition. Construction, products
  and interpolation all use this one private rule.
- **Evidence for the band** ([numerics report](evidence/stress-proto-numerics.md), (b)).
  Without a band, rebuilding a value changed bits in 38-40% of the two
  constructed-input populations. With the 8-EPS band those populations
  showed zero changes in 1.6e8 samples each. This is empirical, plus a
  first-order bound sketch; the runtime postcondition is what enforces
  the invariant.
- **Why not a Newton step on every product.** An unconditional step
  changed product bits in about 45% of sampled pairs and broke bitwise
  right-identity multiplication in 28.8%. In the sampled raw
  five-product population 0.0062% fell outside the band; that is not a
  bound for arbitrary inputs or long chains.
- **Inverse.** The rotation is the conjugate. The translation is
  `s * r[i]` with `r = rotate_raw(conjugate(q), t)` and
  `s = −1.0 / n²(q)`; `rotate_raw` is the sandwich product without a
  further norm correction. The result's translation is checked for
  finiteness. Over two million sampled chains, against an f128 reference
  for the normalized rotation the stored quaternion represents, this
  form had a relative RMS translation error of 0.66 EPS, against 0.99
  for 2.4's normalize-then-conjugate and 2.65 for a plain conjugate over
  raw products. These are population-specific figures, not bounds, and
  the measured products were raw and thresholded-Newton products, not
  this design integrated. Dividing the quaternion itself by n² is a
  different algorithm and leaves the band (17,426 of 528,047
  boundary-heavy inputs in Codex's probe).
- **Finiteness.** Every operation that produces a translation checks all
  three components. No magnitude bound: a bound is not closed under
  composition of derived values (doubling an identity-rotation
  translation of 1.0 overflows on the 1024th doubling).
- **Interpolation ratio.** Integer nanosecond differences divided as
  `f64`. For spans below 2^53 ns both conversions are exact and the
  division is correctly rounded. The sampled seconds-based calculation
  of 2.4 differed from the reference in about 36% of cases, by at most
  2 ulp (reproduced by both designers).
- **Exactness kept.** A direct child-to-parent lookup at a stored
  instant returns the stored geometry bit for bit. Each same-time chain
  resolution inverts at most once; a cross-time lookup resolves two
  legs. `libm` for every transcendental.
- **Not promised.** A bitwise identity law for composition (signed zeros
  are a counterexample with no normalization involved), and bit equality
  with 2.4 for interpolated or multi-hop results. Changed bits are not
  by themselves a regression; they are assessed against independent
  references and justified error bounds.

### Time

`Timestamp` is a signed 64-bit nanosecond count. The epoch is the
caller's; the type does not imply Unix time.

- **What goes:** `TimePoint`, `TimeError`, `src/time/system_time.rs`,
  the `<T>` parameter on `Registry`, `Transform`, `Stamp`, `Point`,
  `RegistryError` and both traits, `Buffer::check_interpolation_time`,
  `GetError::Interpolation`, and most of the 659-line
  `tests/time_traits.rs`. About 450 library lines gross (estimate; the
  replacement was not built).
- **Why it is safe to remove.** With the default `Timestamp` a
  time-arithmetic failure inside a lookup is already unreachable; that
  machinery serves custom clocks only. The one studied user of a
  non-default `T` (dimos, `SystemTime`) carries signed nanoseconds on
  the wire, so its conversion gets shorter. The only third-party
  `TimePoint` impl found is a tests-only mock.
- **What it fixes.** E0283 in all-static setups; lossy seconds in error
  payloads and formatting, where distinct nanosecond stamps print
  identically; and an inferred obstacle to typed error handling in
  generic adapters. Every ROS `(i32, u32)` stamp with `nanosec < 1e9`
  fits, negative seconds included.
- **Details that must be specified.**
  - Ordered differences use `i64::abs_diff`.
  - `from_secs_nanos` rejects `nanos >= 1_000_000_000` and computes in a
    wider intermediate (a checked `i64` multiply then add wrongly
    rejects `i64::MIN`).
  - `as_secs_nanos` floors: −1 ns is `(−1, 999_999_999)`.
  - `checked_add` and `checked_sub` handle the full signed range and
    large `Duration` operands without narrowing early.
  - `checked_duration_since` returns `None` iff `earlier > self`.
- **Cost, stated.**
  - Clocks finer than 1 ns or beyond ±292 years are out.
  - Non-decimal ticks lose exactness. Nearest rounding at 32,768 Hz
    costs up to 0.5 ns per stamp, and quantizing absolute stamps is not
    the same as quantizing each elapsed duration. Codex's floor example
    gave ratios `0.49999180797902842` against `0.50000819202097158`; the
    error depends on the span.
  - Newtype separation of clocks is gone. It was real only across
    different `T`, and no studied user used it.
  - Negative instants become insertable, so an upstream sign bug no
    longer fails at conversion.
  - Going back to generics later is another major release.

### Registry semantics

Unchanged from 2.4 unless listed.

- **Kept:** strict single-parent tree; the first insert pins parent and
  kind; static xor dynamic; explicit atomic `reparent` that keeps kind
  and retention and drops the frame's history; descendant-preserving
  `remove_frame`; batch equals the insert loop and is all-or-nothing; no
  extrapolation; data-driven expiry; caller-managed concurrency;
  duplicate dynamic stamps overwrite; a static update replaces the pose.
- **Expiry (review item 10, D1 to D6).** Under `with_max_age`, an insert
  older than the frame's newest stamp minus `max_age` returns
  `InsertError::Expired` and stores nothing. The boundary sample is
  accepted. A subtraction that is not representable applies no cutoff.
  New or drained histories and a `reparent` seed have no newest stamp to
  expire against. `unbounded()` never reports it.
- **Batch.** Preflight carries a per-frame running newest stamp and the
  staged topology, and builds a commit plan with no logical failure left
  in it. A failed insert or batch interns no frame. `BatchError::index`
  is the zero-based slice index.
- **Batch and expiry together, declared.** One expired member fails the
  whole batch, so a late sample for one frame rejects fresh samples for
  every other frame in the same message; 2.4 stored them. This follows
  from D2. The `insert_all` rustdoc and the `ingest` example show the
  fallback (insert per member, or resubmit without `index`).
- **Cleanup (review item 12).** `remove_samples_through(t)` removes
  every dynamic sample at or before `t`, in every frame. It keeps
  topology and kind, and a drained history keeps no hidden newest
  stamp. `Timestamp::MAX` clears all dynamic history, so no
  `clear_history` is needed (D5 stands).
- **`remove_frame(child)`** returns `true` iff `child` had an edge to a
  parent, which it removes with its history. A root, a name known only
  as a parent, an unknown name and the empty name return `false`.
  Children of a removed frame keep their pins.
- **Lookup diagnosis, one rule for both lookups:** unknown frame on a
  required leg, then disconnected topology, then coverage, then
  numerical failure. Both cross-time legs are resolved before any
  geometry; target leg first within a class. A coverage error names an
  edge on the connecting chain. A numerical failure does not end
  diagnosis before all required coverage is checked.
- **Empty and equal names.** An empty query name is `UnknownFrame`,
  checked before any shortcut. A non-empty frame relative to itself at
  one instant is the identity, registered or not. Every `lookup` result
  carries `Stamp::At(at)`, also over an all-static chain.
- **Cross-time.** Equal endpoint names at different times still resolve
  both legs. A leg whose endpoint is the fixed frame is the identity.
  The caller asserts that the fixed frame is stationary between the two
  times; the registry does not infer it from edge kinds.
- **`latest_common_time`** returns `Static` when the connecting chain
  has no dynamic hop, including a frame relative to itself. A dynamic
  hop holding nothing is `NoData`. `NoCommonTime` names a hop whose
  coverage `earliest..=latest` starts after `latest_possible`, the
  minimum newest stamp of the required dynamic hops.
- **`Disconnected { a, b }`** names the pair that is actually
  disconnected; in a cross-time lookup that can be an endpoint and the
  fixed frame.
- **Names.** Empty names are rejected by `Transform::new` and by
  decoding (M13). A self-named transform is constructible (identity,
  round trips); `insert` rejects it as `SelfEdge`.

### Internals

- **Interned frames.** A vector of slots plus a `hashbrown` name index.
  Every name a stored edge references, roots included, has a slot. Name
  lookup is expected O(1) in the number of frames. Ids are private: no
  handle, no generation. A slot is reclaimed only when no edge or child
  references it, and reclamation never renumbers a live slot.
- **Topology-first resolution.** Index walks find the connecting chain,
  and only its edges are sampled. A successful resolution allocates
  nothing temporary; the labelled result allocates its two names. Error
  construction can allocate. A cold diagnostic walk keeps target-side
  first, lowest-failing-edge attribution.
- **History** stays `BTreeMap<Timestamp, Isometry>`. A sorted `VecDeque`
  made steady inserts 3.3-4.8x faster and out-of-order inserts far
  slower (newest 1% of a 60k window: 9-13x; uniform: 130-230x), and its
  lookup gain did not reproduce end to end.
- **Statics** are one bare isometry, outside the time path.
- **One private isometry** carries all arithmetic, as today.

Prototype against 2.4 ([report](evidence/stress-proto-hotpath.md);
differential run reproduced by both designers): 372,996 successful
comparisons with maximum component difference 0.0; 27,069 of 27,069
direct stored-direction lookups bit-identical; zero named-frame
disagreements among comparable errors. Two difference classes: 73,889
intended `Disconnected`-precedence differences, and 3,344 same-name
queries on an unregistered frame, where the prototype returns
`UnknownFrame` and both 2.4 and this design return the identity. The
prototype does not implement the design on that case.

Speed-up over 2.4, adopted variant only (`BTreeMap` history, `hashbrown`
index), range of three paired runs:

| Workload | Speed-up |
|---|---:|
| One hop, exact stamp, 1k samples | 1.51-2.19x |
| One hop, interpolated | 1.25-1.51x |
| Four dynamic hops, interpolated, either direction | 0.90-1.13x |
| Robot tree, leaf to leaf | 3.31-3.71x |
| Siblings below 16 dynamic ancestors | 24.10-28.46x |
| Static climb, 1,000 deep | 2.25-3.26x |
| Unknown frame, 1,000-deep chain | 1,027-1,270x (50-95 µs against 44-92 ns) |

These support the architecture, not a finished-v3 figure. The prototype
has no result labels, none of the numerical changes, and no removal,
re-parenting, batch, cross-time or latest-common-time path. The
unknown-frame row measures the whole query, not the frame scan alone.

### Serde

- On `Vector3`, `Rotation`, `Timestamp`, `Stamp` and `Transform`,
  default-off. None on `CrossTimeTransform`.
- `Rotation` and `Transform` decode through `from_wxyz` and
  `Transform::new`, never a field-wise derive. A standalone `Vector3`
  decodes unchecked; finiteness is enforced when it enters a
  `Transform`. `Stamp` keeps its explicit tags; a missing or null stamp
  is a decode error.
- The layout is written out explicitly and frozen by tests, not
  inherited from declaration order. Which layout is M10.
- **The hazard.** A signed timestamp is not safely compatible with 2.x
  postcard data: an old `u64` 1 decodes without error as `i64` −1, and
  with 2.4's field and variant order a whole 2.x `Transform` decodes
  `Ok` at a wrong instant (1.7e18 ns becomes 8.5e17; probed). Persisted
  data is migrated with the 2.x decoder; no heuristic dual decoding.

### Acceptance

- `tests/golden_vectors.rs` with the same SciPy digits, re-pointed at
  the new accessors; never regenerated.
- A differential harness against 2.4, kept outside the crate, with the
  declared differences listed: `Disconnected` precedence; `Expired`,
  including whole-batch rejection; the inclusive cutoff; normalization
  of off-band rotations; last-bit changes in interpolated, inverted and
  multi-hop results; `Ok(non-finite)` to `Err`.
- Properties ported from `tests/properties.rs`: the interpolation oracle
  replaced by an in-test reference; the batch property biased to hit the
  running-newest trap; the round-trip property given a magnitude-aware
  bound in place of its 1e-9 absolute pin; new properties for the slot
  lifecycle (remove and re-add under a new parent and kind; a failed
  batch leaves `UnknownFrame` answers unchanged).
- A frozen serde schema with constructor and decode fixed points.
- The gate in both serde modes on all current targets; MSRV 1.85.

## Review of the review

Verdict on each item of [`v3-review/REPORT.md`](../v3-review/REPORT.md):

| # | Item | Verdict |
|---|---|---|
| 1 | `CrossTimeTransform` | Adopt. Six accessors; the cross-leg error rule becomes the general diagnosis rule. |
| 2 | Normalize rotations where built | Adopt, extended: the invariant moves into `Rotation` and holds for derived values. The residual (insert validates but does not normalize) goes. |
| 3 | `inverse` keeps normalizing | Replace with the translation-corrected conjugate. It needs no `sqrt` and improves sampled accuracy. Bit changes remain, and the old 1e-9 absolute pin still failed once in sixteen million sampled chains; acceptance needs a magnitude-aware bound instead of that pin. |
| 4 | Lookups never return non-finite | Adopt, extended to `compose` and `inverse`. The review excluded `*` because a fixture that manufactures infinity then fails; that fixture pins the old contract. |
| 5 | `Disconnected` before `NotFoundAt` | Adopt. |
| 6 | Error shapes | Adopt, extended: errors by operation family, frame fields on every structural insert error, `BatchError`. Keep exact `lhs` and `rhs` stamps instead of a unit `TimestampMismatch`. |
| 7 | Delete the Point family | Adopt. Not safety-neutral: a checked application path goes. See M1. |
| 8 | Delete `Transform::interpolate` | Adopt. |
| 9 | Delete `Timestamp::as_seconds` | Adopt; `as_seconds_lossy`, `now`, `try_now` and the operators go too. |
| 10 | `SampleExpired` | Adopt as settled (D1 to D6), spelled `InsertError::Expired` with the same payload. |
| 11 | Delete `SameFrameMultiplication` | Adopt. |
| 12 | Inclusive cleanup cutoff | Adopt, with a rename (M4). |

Where the review is wrong or too timid:

- **"What 3.0 leaves alone" and "Not in 3.0" are recommendations made
  under the minimal-break premise**, not rulings. Reopened and adopted:
  a `Rotation` type, named methods for operators, typed time payloads, a
  signed `Timestamp`, a conjugate-form `inverse`, `approx` out of the
  public API, explicit retention constructors, internal interning, the
  `ReparentingNotSupported` rename, a flat crate root.
- **It patches at call sites what a type carries once.** Normalization
  in three constructors, a finiteness check at two exits and a
  canonicalizing `From` impl are each a place to forget.
- **It keeps generic time and one public registry error**, and does not
  explore removing `TimePoint` or the `std` feature.
- **It leaves measured hot-path waste alone.** 2.4 samples shared
  ancestors it then discards and does string-map work per hop; its
  unknown-frame path also scans parent references.
- **It overstates what a gap limit would close.** A limit does not
  detect a wrong exact sample or a jump smaller than the limit.
- **"No number that is right today" is not a usable criterion.** The
  design keeps explicit bit-exact guarantees and assesses other changes
  against references and bounds.

Where the review stands against the first drafts of this brainstorm:

- **`BTreeMap` history.** Claude's draft proposed a `VecDeque`; the
  measurement supports the review.
- **Labelled results.** Claude's draft returned unlabelled `Copy`
  geometry. The safety critique showed three losses: a flattened result
  re-inserted under the wrong edge is rejected today by its own labels;
  an adapter could not tell a cross-time result by type; a swapped
  lookup stops describing itself.
- **No Newton step on every product, and no translation bound.**

The review's open decisions, as resolved here: (1) `inverse`, the
translation-corrected conjugate; (2) normalization site, the `Rotation`
type; (3) Point family, delete all four; (5) frame fields, yes, with a
batch index; (6) six accessors; (7) finiteness, 3.0, every operation;
(8) delete `SameFrameMultiplication`; (9) rename to `ParentMismatch`;
(10) differential harness outside the crate; (15) do not publish the
insert check order. Decisions 4 (2.x notice) and 11 (minor-release
policy) are not design questions. Decisions 13 and 14 (porting the
examples; what `ingest` shows on `Expired`) are not addressed here
beyond the batch fallback above. Decision 16 is reopened as M4.

## Not in 3.0

| Item | Reason |
|---|---|
| Maximum interpolation gap | The largest demonstrated interpolation-policy hazard left (`Ok` across an 8 s dropout in all three studied users) and core-or-nowhere. No configuration model is settled. Exact-only lookup would protect an unconfigured user, at the cost of disabling ordinary interpolation until configured; no universal finite default is justified. Optional limits can be added later; changing a default cannot. An unresolved priority, not a promised feature (M12). |
| Public frame ids, typed frames | Lifecycle and ownership contracts with no demonstrated use; typed frames fit middleware-named frames poorly. |
| Unlabelled results, public isometry, total `Mul` | See above. |
| `VecDeque` history, `BTreeMap` name index | Measured; see Internals. |
| Translation bound, unconditional Newton step | See Numerics. |
| Lookup argument struct | Both result types describe their endpoint frames, and `CrossTimeTransform` also describes both supplied times. Neither detects wrong caller intent: swapped times, or a moving frame passed as `fixed`, can return `Ok`. Open in 2.4 as well; recorded as a residual. |
| Named-field rotation input | Would close positional permutation at `from_wxyz`; costs a type. Residual recorded under M8. |
| `f32`, extrapolation, non-linear interpolation, tf2 parity | Non-Goals, unchanged. |
| `clear_history` | Unnecessary: `remove_samples_through(Timestamp::MAX)` clears all dynamic history (every frame at once). |
| Automatic reset detection, capacity limits | No agreed requirement or policy. Residual, silent: a backwards clock jump smaller than `max_age`, and any jump on an `unbounded()` registry, is accepted and two epochs interpolate into one answer. `Expired` narrows this; it does not close it. |
| Frame enumeration, coverage read-out | Additional state-exposing commitments. |
| `can_transform`, a wait layer | Compose on the public API. |
| Caches | Need their own demonstrated need and invalidation design. |
| Euler or axis-angle constructors, a normalizing rotation constructor, math-crate features | Buildable outside. |
| Removing `thiserror` or `hashbrown` | Private dependencies; no contract gain. |

Two residuals kept from 2.4 and recorded, not closed: a self-lookup
proves nothing about a frame's existence (`lookup("typo", "typo", t)` is
the identity); and a self-named transform with non-identity geometry is
constructible and composes.

## Decisions for the maintainer

Split between the designers:

| # | Question | Claude | Codex |
|---|---|---|---|
| M1 | Add `Transform::transform_point(&self, p: Vector3) -> Vector3` (`R p + t`)? Reopens the 2.4 ruling D2 of 2026-11-12 (no scope exception for checked application; revisit on demonstrated need). Unlike the rejected `transform_positions` it checks no frame and no stamp. New since D2: `Point` is deleted, which removes the crate's only application path. | Yes. Three of the seven slips the review lists for hand application are hand-math slips. One method pins the convention in code and gives the golden vectors a crate path again. | No under the scope rule: `rotation().rotate(p)` plus a component-wise add is exact outside. Admissible only as a stated exception. |
| M10 | Which serde layout to freeze? Either way old postcard data is not safely readable and the schema version must travel outside the payload. | (b) Rename the fields to `target`/`source`/`stamp` and order the layout so the frozen 2.4 postcard fixtures fail to decode, pinned by a test. Old JSON then fails loudly; old postcard fails for those fixtures, not provably for all bytes. | (a) Keep 2.4's field names, order and variant indices (`Static = 0`, `At = 1`) with an `i64` stamp. Least churn. JSON timestamp integers inside the new range keep their values, subject to the other new validation rules; larger ones are rejected. Old postcard data can decode silently to wrong instants, so external schema identification stays required. The whole-transform probe confirms this known hazard and does not change the preference. |

Agreed by both, and the maintainer's because each changes a ruling, a
Non-negotiable or a capability:

| # | Item | Note |
|---|---|---|
| M2 | No `lookup_latest` | Both lean no: exact on the public API, and an adapter can hold one guard across two calls. The record of wrong hand-built versions (one of three, for seven releases) is the argument for an exception. |
| M3 | `Timestamp::from_secs_nanos` and `as_secs_nanos` in the core | Both are exact outside, so this is a scope exception, argued on its own. Outbound: a signed type makes the naive `/ %` with a cast to `u32` wrong (−0.5 s gives `(0, 3794967296)`). Inbound is weaker: the two wrong hand-rolled conversions were written against `u64`, and with `i64` the naive form is correct for every valid ROS stamp. |
| M4 | Rename `remove_transforms_before` to `remove_samples_through` | Reopens the ruling of 2026-11-11 (inclusive, name kept), which was a 3.0 ruling. Grounds: a boundary change under an unchanged name is a silent break; a rename is a compile error. |
| M5 | Concrete `Timestamp(i64)`; delete `TimePoint` and `SystemTime` support | Removes a capability. See Time for the cost. |
| M6 | Remove the `std` feature, `now` and `try_now` | The crate is unconditionally `no_std` + `alloc`. Removing `now` removes a misleading convenience; it does not prevent mixing clocks. |
| M7 | Move `approx` to the dev-dependencies | Amends the dependency list in AGENTS.md. |
| M8 | `Rotation` replaces `Quaternion`; `Vector3` loses its operators | A regression in one respect: four positional `f64`, scalar first, replace the `Quaternion { w, x, y, z }` literal both ROS adapters use, and ROS orders the scalar last. `from_wxyz(q.x, q.y, q.z, q.w)` is accepted for every unit quaternion (the xyzw identity becomes a half turn). Accept the residual, or add a named-field input. |
| M9 | Errors by operation family | Replaces the AGENTS.md invariant that every `Registry` call reports one flat `RegistryError`. Each enum stays flat. |
| M11 | Flat crate root and the shorter method names | Cosmetic; one pass. |
| M12 | Interpolation gap limit | Not in 3.0; decide whether to open a design. |
| M13 | Empty names rejected by `Transform::new` and decoding; `lookup("", "", t)` is `UnknownFrame` | Changes the 2.4 ruling D6 of 2026-11-12 (insert boundary only; `""` against itself is the identity). Grounds: public composition joins on `""` too, so the hazard is not registry-only. 3.0 decoders reject payloads 2.4 accepted. |
| M14 | Whole-batch rejection on one expired member | A consequence of D2, now stated; confirm it is intended for TF-message batches. |
| M15 | `Timestamp::checked_add`, `checked_sub`, `checked_duration_since` in the core | Exact outside on `as_nanos`, so a scope exception. Grounds: raw `i64` timestamp subtraction can overflow, and duration arithmetic needs wide intermediates and a checked final range, not an early narrowing of the duration to `i64`. `checked_add` has no demonstrated caller and is the first to cut. |
| M16 | `LookupError::NonUnitRotation` | A precise diagnostic for a failed runtime normalization postcondition. The rounding argument suggests the failure is unreachable but is not a proof. A broader numerical-failure variant is another possible design; labelling a finite failure `NonFinite` is rejected. |

## AGENTS.md under this design

- **Architecture:** `Registry` is interned slots plus a name index;
  `geometry` loses `Point` and gains `Rotation` and
  `CrossTimeTransform`; `time` is one concrete type.
- **Non-negotiables:** the dependency list (M7); the feature rule
  becomes "one mode, plus `serde`"; the slerp bit-pin stays as a
  regression pin, no longer a std-parity pin; "the only documented
  panic" goes with `Timestamp::now`.
- **Invariants that change text:** composition and its errors; the flat
  `RegistryError` bullet (M9); lookup diagnosis (precedence, `NoData`,
  `OutOfRange`, the `NoCommonTime` payload, empty names);
  `TimestampOutOfRange` (deleted with `interpolate`); error formatting
  through `as_seconds_lossy` (deleted); cleanup (inclusive, renamed);
  "validated where they are built" (the invariant is the type's); the
  two "derived values are not re-validated" bullets and insert-time
  validation (deleted); "`from_wxyz` does not normalize" and "anything
  that inverts must normalize first"; the epsilon `PartialEq` bullet
  loses its `approx` reference.
- **Invariants that stand in substance and change names:** direction of
  a transform (target/source); static xor dynamic (`KindMismatch`);
  strict tree and `reparent` (`ParentMismatch`, `Cycle`, `SelfEdge`;
  empty names at construction); batch atomicity (`insert_all`,
  `BatchError`; the expiry check reads a running newest stamp, the other
  checks still read no stored sample); data-driven expiry (an insert
  older than the window is `Expired`, not dropped).
- **Invariants that stand unchanged:** exact frames or an error decided
  before geometry; no extrapolation; one inversion per same-time chain
  and a bit-exact direct lookup; `#[non_exhaustive]` errors.
- **Also changes:** the Style constructor list; the gate and CI feature
  matrix (two modes); the semver combinations in Releasing (two).

## Limits

- No integrated v3 was built. The hot-path prototype and the numerics
  prototype are separate and each omits the other's changes. Small
  timing differences on this machine are inconclusive, and no embedded
  measurement exists.
- The slot lifecycle is unverified: the prototype has no removal,
  re-parenting, batch or reclamation path.
- The 8-EPS band has empirical support and a first-order bound sketch.
  The probes do not prove normalization closure or correct rounding of
  `libm::sqrt`; the runtime postcondition stays required.
- Downstream facts come from the repository's dependents study
  (versions 2.1 to 2.3 and one unmerged pull request). No dependent was
  built against this API. Adapter sketches compiled against a stub of
  Claude's first draft, which is not this API. A stub of an earlier
  revision of this API with equivalent sketches
  ([`evidence/prototypes/api-stub/`](evidence/prototypes/api-stub/))
  was reported clean under clippy pedantic on 1.85 and stable in both
  serde modes and built for `thumbv6m-none-eabi`. It predates
  `LookupError::NonUnitRotation`, `Stamp`'s `Eq`/`Hash`,
  `Registry: Debug` and the `Stamp` variant order, and nothing in it
  computes.
- The gate ran on nothing here.
- Erratum to [`evidence/stress-proto-hotpath.md`](evidence/stress-proto-hotpath.md):
  a lookup one step past the newest sample makes 2 allocations in the
  prototype, not 1 (Codex's rerun).
- Evidence files are snapshots; paths inside them that begin
  `evidence/` are relative to this directory, and some name scratch
  crates that were not kept.
