# transforms v3: consensus design

Date: 2026-11-17. Baseline: master `adccbdd` (v2.4.0). No integrated
v3 implementation exists. This document proposes changes; the prior
maintainer rulings it keeps or reopens are identified where they apply.
The maintainer ruled on every open decision on 2026-11-17, and
revised three rulings the same day after a review of their consequences
([Rulings](#rulings)); the text below is the ruled design. Those
rulings settle the design, not a release: implementation has not been
authorized, and breaking changes still need sign-off per release.

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
  ([`evidence/verify-findings.md`](evidence/verify-findings.md)), and
  a review of the rulings' consequences by Codex and four checkers
  ([`evidence/rulings-findings.md`](evidence/rulings-findings.md)).
  Sub-agents ran at low effort. Probes ran on one x86-64 laptop
  (i7-12850HX, `powersave` governor, other load); nothing ran on embedded
  hardware.
- **Status of the numbers.** Timings are ratios from a noisy machine and
  from a prototype that omits parts of the design ([Limits](#limits)).
  The differential counts were re-run by both designers.

## Verdict

- **The two designers converged on one design**, split on two items
  (M1, M10). The maintainer then ruled on all sixteen decisions and
  five residual questions. Three rulings depart from the text the
  designers agreed, all toward a smaller or stricter core
  ([Rulings](#rulings)).
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
- **Public surface:** 105 named items in 2.4 against 76 here, counted the
  way [`evidence/map-api.md`](evidence/map-api.md) counts (types,
  functions, constants, enum variants, public fields; trait impls
  separately). The designers' text had 79; the rulings removed
  `from_secs_nanos`, `as_secs_nanos` and `Stamp::at`.

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
    pub fn checked_add(self, d: Duration) -> Option<Self>;
    pub fn checked_sub(self, d: Duration) -> Option<Self>;
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration>;
}

pub enum Stamp { Static, At(Timestamp) }

// ---- the tree ----
pub struct Registry { /* private */ }
impl Registry {
    pub fn with_max_age(max_age: Duration) -> Self;
    pub fn unbounded() -> Self;

    pub fn insert(&mut self, transform: &Transform) -> Result<(), InsertError>;
    pub fn insert_all(&mut self, transforms: &[Transform]) -> Result<(), BatchError>;
    pub fn reparent(&mut self, seed: &Transform) -> Result<(), InsertError>;
    pub fn remove_frame(&mut self, child: &str) -> bool;
    pub fn remove_samples_through(&mut self, cutoff: Timestamp);

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
  `EmptyFrameName` (target checked first, then source) or, after that,
  `NonFinite`. `insert` never returns the three
  `reparent`-only variants. `latest_common_time` never returns
  `OutOfRange`, `NonFinite` or `NonUnitRotation`; the lookups never
  return `NoCommonTime`.
- **Numerical failures.** `NonFinite` reports a non-finite value. A
  finite rotation that fails the normalization postcondition reports
  `NonUnitRotation { norm }`. Neither may escape inside a successful
  transform.
- **Equal names.** `Transform::new` accepts `target == source` with any
  geometry. Derived self-named values are ordinary: a pose error
  `est.inverse()?.compose(&truth)?` is a `base`-to-`base` transform that
  is not the identity. A hand-built `a`-to-`a` transform with an offset
  is a caller error no check catches; `insert` rejects every self-named
  transform as `SelfEdge`.
- **Applying a transform to a point** is done outside, and the
  `Transform` rustdoc carries the recipe as a doc test that asserts the
  digits of golden vector 4:

  ```rust
  let r = tf.rotation().rotate(p);
  let t = tf.translation();
  let p_target = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
  ```

  Nine plausible hand-written variants (rotate after adding, forgotten
  translation, wrong direction, a typo in the add) all return a finite
  `Vector3`, and no check in this API sees any of them.
- **`Rotation::rotate`** is raw numerical application: it checks no
  frame and does not promise a finite output for extreme inputs.
- **`Transform::new`** allocates both names when given borrowed strings;
  moving in a `String` avoids the copy.
- **A `Static` answer from `latest_common_time`** means every instant
  is served; the rustdoc shows the `match` and says any `Timestamp`
  (for example `Timestamp::from_nanos(0)`) is a valid lookup instant
  then. A `lookup` result's stamp is always `At`.
- **`Timestamp::MAX`** exists as a cleanup cutoff, not as a stamp or a
  lookup instant. A dynamic sample
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
`Vector3`'s arithmetic operators, `Stamp::is_static`, `Stamp::at`,
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
- **Surface, as ruled.** `from_nanos`, `as_nanos`, `MAX`, and three
  checked operations. No seconds-and-nanoseconds conversions.
  - `checked_add` and `checked_sub` handle the full signed range and
    any `Duration`, without narrowing the duration to `i64` early.
  - `checked_duration_since` returns `None` iff `earlier > self`.
  - They are in the core because the hand-written forms are
    wrong-answer generators at ordinary timestamps:
    `(now - stamp) as u64` reads a stamp 2 ms ahead of the clock as an
    age of 584 years, bare `abs_diff` loses the direction, and
    `now + timeout.as_nanos() as i64` with `Duration::MAX` is a deadline
    1 ns in the past. 2.4 returned an error in each case.
  - Internally, ordered differences use `i64::abs_diff`, and the expiry
    cutoff `newest − max_age` is computed exactly; only a result below
    `i64::MIN` counts as not representable.
  - `Display` is sign-magnitude decimal seconds, not the floored
    `(sec, nanosec)` pair: −0.5 s prints `-0.500000000` and splits as
    `(−1, 500_000_000)`.
- **The ROS-style conversion is documented, not exported.** The
  `Timestamp` rustdoc carries this example, followed by assertions so
  the doc test executes it (−1 ns, −0.5 s, a denormalized `nanosec`,
  `Timestamp::MAX`). It was compiled and tested for this document
  against a stand-in type, clean under clippy pedantic:

  ```rust
  const NANOS_PER_SEC: i64 = 1_000_000_000;

  /// `builtin_interfaces/Time { sec: i32, nanosec: u32 }` to a `Timestamp`.
  fn from_ros(sec: i32, nanosec: u32) -> Option<Timestamp> {
      let nanosec = i64::from(nanosec);
      if nanosec >= NANOS_PER_SEC {
          return None; // denormalized stamp
      }
      // Cannot overflow: |sec| * 1e9 + nanosec < 2.2e18 < i64::MAX.
      Some(Timestamp::from_nanos(i64::from(sec) * NANOS_PER_SEC + nanosec))
  }

  /// A `Timestamp` to `(sec, nanosec)`. Euclidean division floors, so the
  /// nanosecond part is always in `0..1e9`: -1 ns is `(-1, 999_999_999)`.
  fn to_ros(t: Timestamp) -> Option<(i32, u32)> {
      let nanos = t.as_nanos();
      let sec = i32::try_from(nanos.div_euclid(NANOS_PER_SEC)).ok()?;
      let nanosec = u32::try_from(nanos.rem_euclid(NANOS_PER_SEC)).ok()?;
      Some((sec, nanosec))
  }
  ```

  The trap it documents: `/` and `%` truncate toward zero, so the naive
  split of −0.5 s followed by a cast gives `(0, 3794967296)`.
- **Cost, stated.**
  - Clocks finer than 1 ns or beyond ±292 years are out.
  - A non-nanosecond clock converts at the boundary. Converting the
    absolute tick count rounds each stamp independently (at most 0.5 ns
    at 32,768 Hz), so nothing drifts; the mapping is monotone and
    injective, so a query at a stored tick still hits the stored sample.
    Only the interpolation ratio moves, by about 1 ns over the sample
    span (`0.4999918` for the midpoint of two adjacent ticks). 2.4's
    generic does not avoid this: `TimePoint::duration_since` returns a
    nanosecond `Duration` and the ratio is computed from it. The user's
    conversion must not overflow: `ticks * 1_000_000_000` exceeds `i64`
    after 3.3 days at 32,768 Hz and needs a split or 128-bit form.
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
  Children of a removed frame keep their pins, and its former
  parent becomes unknown if no other edge names it.
- **Lookup diagnosis, one rule for both lookups:** unknown frame on a
  required leg, then disconnected topology, then coverage, then
  numerical failure. Both cross-time legs are resolved before any
  geometry; target leg first within a class. A coverage error names an
  edge on the connecting chain. A numerical failure does not end
  diagnosis before all required coverage is checked.
- **Known names.** A name is known iff a stored edge names it as child
  or parent when the call returns; a drained edge still counts. Every
  name in a query must be known, else `UnknownFrame`. Names are checked
  before any leg is formed, in the order target, source, fixed, and the
  first unknown one is reported. The empty name is never known.
- **Consequences of that rule, declared.**
  - An empty registry answers every query with `UnknownFrame`,
    self-lookups included. A lone frame cannot be made known, because a
    self-edge is rejected; a one-frame system short-circuits
    `target == source` itself.
  - `remove_frame` and `reparent` make a parent unknown when no other
    edge names it. A removed child stays known while it has children.
  - A failed insert, batch or `reparent` makes no name known.
  - 2.4 returned the identity for equal unregistered names; the dimos
    test asserting that for `("map", "map")` breaks.
- **Equal names.** In `lookup`, a known frame relative to itself is the
  identity; in `latest_common_time` it is `Static`. Every `lookup`
  result carries `Stamp::At(at)`, also over an all-static chain.
- **Cross-time.** `lookup_across` always resolves both legs to `fixed`,
  also when the endpoint names and the two times coincide, so a
  disconnected fixed frame is `Disconnected` there. Only a leg whose
  endpoint is the fixed frame is the identity. The caller asserts that
  the fixed frame is stationary between the two times; the registry
  does not infer it from edge kinds.
- **`latest_common_time`** returns `Static` when the connecting chain
  has no dynamic hop, including a known frame relative to itself. A dynamic
  hop holding nothing is `NoData`. `NoCommonTime` names a hop whose
  coverage `earliest..=latest` starts after `latest_possible`, the
  minimum newest stamp of the required dynamic hops.
- **`Disconnected { a, b }`** names the pair that is actually
  disconnected; in a cross-time lookup that can be an endpoint and the
  fixed frame.
- **Names.** Empty names are rejected by `Transform::new` and by
  decoding. A self-named transform is constructible; `insert` rejects it
  as `SelfEdge`.

### Internals

- **Interned frames.** A vector of slots plus a `hashbrown` name index.
  Every name a stored edge references, roots included, has a slot.
  Slots are an implementation detail of the known-names rule: a name
  leaves the index in the same call that removes its last edge
  reference, whenever the memory is reused. Name
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
disagreements among comparable errors. Two difference classes, both now
declared differences from 2.4: 73,889 `Disconnected`-precedence
differences, and 3,344 same-name queries on an unregistered frame,
where 2.4 returns the identity and the prototype, like the ruled
design, returns `UnknownFrame`.

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

The finished implementation, measured on 2026-11-18
([report](evidence/bench-v3-orin.md)): the crate's own benchmark suite,
commit `93cc052` against 2.4.0, on a Jetson AGX Orin, median of three
paired runs:

| Workload | Speed-up |
|---|---:|
| One hop, exact stamp, 1k samples | 1.29x |
| One hop, interpolated | 1.15x |
| Four dynamic hops, interpolated, either direction | 1.09x and 1.16x |
| Robot tree, leaf to leaf | 3.23x |
| Static climb, 1,000 deep | 1.84x |
| Unknown frame, 1,000-deep chain | 1,029x (48 µs against 47 ns) |
| `latest_common_time`, four hops | 4.84x |
| Failed lookup past the newest sample | 2.52x |
| Cross-time lookup, 1k samples | 1.28x |
| Insert into a prewarmed history | 1.14-1.16x |

v3 is faster on every benchmark both suites share. The gains from
resolving topology first carried over from the prototype; the one-hop and
static-climb gains are smaller than the prototype's, which had no result
labels and none of the numerical checks. The siblings workload has no
benchmark in the 2.4 suite and was not compared.

### Serde

- On `Vector3`, `Rotation`, `Timestamp`, `Stamp` and `Transform`,
  default-off. None on `CrossTimeTransform`.
- `Rotation` and `Transform` decode through `from_wxyz` and
  `Transform::new`, never a field-wise derive. A standalone `Vector3`
  decodes unchecked; finiteness is enforced when it enters a
  `Transform`. `Stamp` keeps its explicit tags; a missing or null stamp
  is a decode error.
- The layout is written out explicitly and frozen by tests, not
  inherited from declaration order. As ruled (M10): `Transform` is
  `target`, `source`, `translation`, `rotation`, `stamp`, in that order;
  `Stamp`'s variants are `Static`, then `At`.
- **Why names first.** A leading name consumes the low bytes of 2.4's
  first float, so every later float is misaligned and the rotation-norm
  check rejects. In a sweep of all 240 field and variant orders
  ([`evidence/prototypes/serde-layout-sweep/`](evidence/prototypes/serde-layout-sweep/)),
  this order decoded 0 of 200,000 sampled 2.4 transforms. That is
  empirical, not a proof. "The frozen 2.4 fixtures fail" alone is too
  weak a rule: 238 of 240 orders satisfy it, and 22 of those accept
  ordinary 2.4 data with wrong frames and stamps (`B`-from-`C` at 65 ns
  decoded as `A`-from-`B` at −34 ns in Codex's probe).
- **The pin.** A test asserts that each frozen 2.4 fixture in
  `tests/serde.rs` fails to decode, alone and followed by trailing
  bytes, and not merely by running out of input. It is a regression
  pin, not a schema detector.
- **JSON.** 2.x documents fail on the missing `target` field. Unknown
  fields are rejected, so a half-migrated document carrying both
  `stamp` and `timestamp` is an error.
- **Not protected.** A standalone `Stamp` or `Timestamp` in 2.x postcard
  decodes without error at a wrong instant. Decoding with the wrong
  schema can corrupt frames as well as time, so the schema version
  travels outside the payload.
- **The hazard.** A signed timestamp is not safely compatible with 2.x
  postcard data: an old `u64` 1 decodes without error as `i64` −1, and
  with 2.4's field and variant order a whole 2.x `Transform` decodes
  `Ok` at a wrong instant (1.7e18 ns becomes 8.5e17; probed). Persisted
  data is migrated with the 2.x decoder; no heuristic dual decoding.

### Acceptance

- `tests/golden_vectors.rs` with the same SciPy digits, re-pointed at
  the new accessors; never regenerated. Vectors 1 and 4 become tests of
  `rotate` plus the documented recipe; vectors 2 and 3 remain the pins
  of the translation convention through the crate's own code.
- A differential harness against 2.4, kept outside the crate, with the
  declared differences listed: `Disconnected` precedence; `Expired`,
  including whole-batch rejection; the inclusive cutoff; normalization
  of off-band rotations; last-bit changes in interpolated, inverted and
  multi-hop results; `Ok(non-finite)` to `Err`; `UnknownFrame` for
  equal unregistered names in `lookup`, `latest_common_time` and
  `lookup_across`; the order in which unknown names are reported;
  empty names rejected at construction. Two more were found by the
  harness and declared on 2026-11-18: which leg's failure a cross-time
  lookup reports when both fail (the target leg first, as "Registry
  semantics" prescribes), and finite against non-finite results at the
  overflow edge of `f64`, where the two versions agree once the
  translations are scaled down.
- Properties ported from `tests/properties.rs`: the interpolation oracle
  replaced by an in-test reference; the batch property biased to hit the
  running-newest trap; the round-trip property given a magnitude-aware
  bound in place of its 1e-9 absolute pin; new properties for the
  known-names rule (remove and re-add under a new parent and kind; a
  parent losing its last edge; a failed insert, batch or `reparent`
  leaves `UnknownFrame` answers unchanged).
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
| 7 | Delete the Point family | Adopt. Not safety-neutral: a checked application path goes, and M1 ruled against a replacement. |
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

Ruled out on 2026-11-17 under the scope rule, each exact on the public
API: `Transform::transform_point` (M1), `lookup_latest` (M2),
`Timestamp::from_secs_nanos` / `as_secs_nanos` (M3), `Stamp::at`.

Residuals recorded, not closed: a self-named transform with
non-identity geometry is constructible and composes; every hand-written
point application returns a finite vector.

## Rulings

Maintainer rulings of 2026-11-17. "Departs" marks a ruling that differs
from the text the designers agreed; the design above already reflects
all of them.

| # | Question | Ruling | Note |
|---|---|---|---|
| M1 | Add `Transform::transform_point` (`R p + t`)? Designers split: Claude yes, Codex no. | **Not added**; confirmed after the review. | 2.4 ruling D2 stands. The `Transform` rustdoc carries the application recipe as a doc test on golden-vector digits. |
| M2 | Add `lookup_latest`? | **Not added.** | `latest_common_time` then `lookup`, under one guard held by the adapter. |
| M3 | `Timestamp::from_secs_nanos` / `as_secs_nanos` in the core? | **Neither.** Departs. | A middleware shape, exact outside. The `Timestamp` rustdoc carries a correct ROS-style conversion example (see Time). |
| M4 | Rename `remove_transforms_before` to `remove_samples_through`? | **Renamed.** | Supersedes the ruling of 2026-11-11 that kept the name. |
| M5 | Concrete `Timestamp(i64)`; delete `TimePoint` and `SystemTime` support? | **Yes**, as a newtype, not a bare `i64`. | The newtype names the unit at the call site and keeps instants apart from durations and other integers. |
| M6 | Remove the `std` feature, `now`, `try_now`? | **Removed.** | Unconditional `no_std` + `alloc`. |
| M7 | `approx` to the dev-dependencies? | **Moved.** | Amends the dependency list in AGENTS.md. |
| M8 | `Rotation` replaces `Quaternion`; `Vector3` loses its operators? | **Yes, with positional `from_wxyz`.** | Residual accepted: `from_wxyz(q.x, q.y, q.z, q.w)` passes for every unit quaternion (the xyzw identity becomes a half turn). No named-field input. |
| M9 | Errors by operation family? | **Yes.** | Replaces the flat-`RegistryError` invariant in AGENTS.md. Each enum stays flat. |
| M10 | Which serde layout? Designers split: Claude new, Codex 2.4's. | **New layout, name-first order fixed.** | `target, source, translation, rotation, stamp`; `Static, At`. The first ruling left the order to a fixture test, which the review showed is too weak. |
| M11 | Flat crate root and shorter method names? | **Both.** | |
| M12 | Maximum interpolation gap? | **Deferred.** | Not in 3.0; a separate design after it. Still the largest open hazard. |
| M13 | Empty names rejected at construction and decoding; empty query name is `UnknownFrame`? | **Yes.** | Supersedes 2.4 ruling D6. |
| M14 | One expired member fails the whole batch? | **Yes.** | All-or-nothing stands; the fallback is documented in `insert_all` and the `ingest` example. |
| M15 | `checked_add` / `checked_sub` / `checked_duration_since` in the core? | **All three.** Revised. | First ruled none, on the designers' claim that the hand-written forms fail only near the `i64` limits. The review showed they fail at ordinary timestamps (see Time). |
| M16 | What reports a failed normalization postcondition in a lookup? | **`LookupError::NonUnitRotation { norm }`.** | |
| R1 | Conveniences `Rotation::IDENTITY`, `Timestamp::MAX`, `Stamp::at`? | **Keep the two constants; cut `Stamp::at`.** Departs. | `Stamp::at` is a two-arm `match`. |
| R2 | Self-lookup of a name the registry does not know? | **`UnknownFrame`**; confirmed after the review. Departs. | The designers kept 2.4's identity, and the dependents study had rejected this change. Known is defined by stored edges; the consequences are declared in Registry semantics. |
| R3 | `Transform::new` with equal names and non-identity geometry? | **Allowed.** Revised. | First ruled rejected. The review showed the rule cannot hold for derived values (a pose error is a legal self-named transform), makes the crate emit bytes its decoder refuses, and rejects near-identity rotations by last-bit rounding. |
| R4 | `lookup_across` with five positional arguments? | **Accepted as is.** | Residual: swapped times or a moving `fixed` frame return `Ok`. |
| R5 | `lookup_across` with equal endpoints and times and a disconnected `fixed`? | **`Disconnected`.** | Both legs are always resolved, as in 2.4. |

Details the designers filled in while applying the rulings, for the
maintainer to confirm: `Transform::new` reports `EmptyFrameName` before
`NonFinite`; unknown names are reported in the order target, source,
fixed; JSON decoding rejects unknown fields.

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
  `OutOfRange`, the `NoCommonTime` payload, empty names, the order of
  reported unknown names, and "a frame relative to itself is the
  identity" now holding for known frames only);
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
- **Also changes:** the Style constructor list and the Tests bullet's
  `now()` and `zero()` clauses; "time arithmetic is checked, always"
  stands; the gate and CI feature
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
  `Registry: Debug` and the `Stamp` variant order; its adapter sketches
  call `from_secs_nanos`, `as_secs_nanos` and `Stamp::at`, which the
  rulings removed; and nothing in it computes.
- The gate ran on nothing here.
- Codex reviewed the first set of rulings (round 5). It has not seen
  the text after the three revisions.
- The known-names rule and the serde layout were checked against 2.4
  and against stand-in types; no v3 registry or decoder exists.
- Erratum to [`evidence/stress-proto-hotpath.md`](evidence/stress-proto-hotpath.md):
  a lookup one step past the newest sample makes 2 allocations in the
  prototype, not 1 (Codex's rerun).
- Evidence files are snapshots; paths inside them that begin
  `evidence/` are relative to this directory, and some name scratch
  crates that were not kept.
