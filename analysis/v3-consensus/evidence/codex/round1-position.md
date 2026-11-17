I would keep one owned, single-time `Transform`, add a distinct `CrossTimeTransform`, and make **every successful transform operation preserve numerical validity**. I would spend new API surface on exact diagnostics, recoverable atomic ingestion, and an optional per-edge interpolation limit—not public isometries, frame handles, or scalar generics.

This is a design recommendation, not an implementation-ready patch. It intentionally goes beyond the review’s minimal-change approach while retaining the standing non-goals and dependency list.

My baseline was `adccbdd`, version 2.4.0. I examined the implementation, relevant tests, examples, benchmarks, proposal/addendum set, both review rounds, warrant report, downstream studies, and 2.4 rulings. I also read the history behind the geometry fold, atomic insertion, empty-name rejection, and buffer-state deletions.

Several findings materially influenced my position:

| Probe against 2.4 | Result |
|---|---|
| Compose two ancestor-directed translations of `1e308` | Lookup returns `Ok` containing infinity; reverse lookup errors |
| Apply an accepted quaternion with norm `1 + 0.9e-6` to a 1 km vector | Result is `1000.0018000008102`: approximately 1.8 mm of artificial scale |
| Compose transforms whose timestamps differ by 1 ns near Unix time `1700000000` seconds | Both error payload timestamps become `1700000000.0`; information is lost before formatting |
| Reinsert a cross-time result | Successfully replaces the honest single-time sample |
| Compose through an empty intermediate frame name | Successfully produces a plausible transform |
| Repeatedly compose three transforms initially bounded by `1e100` | Infinity after 692 doubling rounds, 2,076 products |

The reproducible probes are in [scratch/probe/src/main.rs:6](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/probe/src/main.rs:6). The relevant implementation boundaries are [Transform construction and validation](src/geometry/transform/mod.rs:145), [composition](src/geometry/transform/mod.rs:525), and [cross-time resolution](src/core/registry/mod.rs:1162).

My proposed public surface follows. These are signatures, with implementations, documentation, and routine attributes omitted.

The module organization stays recognizable:

```text
transforms
├── Registry                         re-export
├── Transform                        re-export
├── geometry
│   ├── Vector3
│   ├── Quaternion
│   ├── Transform
│   ├── CrossTimeTransform
│   └── UNIT_NORM_TOLERANCE
├── time
│   ├── Timestamp
│   ├── Stamp
│   ├── TimePoint
│   └── TimeError
└── errors
    ├── RegistryError
    ├── TransformError
    ├── QuaternionError
    └── TimeError
```

Storage, resolution, isometry arithmetic, and leaf error modules remain private. There is no prelude, public `Buffer`, public `Isometry`, public `Rotation`, or separate `Sample`/`Resolved` pair.

**One same-time mapping type remains sufficient.** A resolved transform is a valid relationship that callers may intentionally flatten and republish; its provenance does not require a separate type. Cross-time relationships do.

```rust
pub struct Transform<T: TimePoint = Timestamp> {
    // Private fields.
}

impl<T: TimePoint> Transform<T> {
    pub fn new(
        target_frame: &str,
        source_frame: &str,
        translation: Vector3,
        rotation: Quaternion,
        stamp: Stamp<T>,
    ) -> Result<Self, TransformError<T>>;

    pub fn static_between(
        target_frame: &str,
        source_frame: &str,
        translation: Vector3,
        rotation: Quaternion,
    ) -> Result<Self, TransformError<T>>;

    pub fn target_frame(&self) -> &str;
    pub fn source_frame(&self) -> &str;
    pub fn stamp(&self) -> Stamp<T>;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Quaternion;

    pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError<T>>;
    pub fn inverse(&self) -> Result<Self, TransformError<T>>;
}

pub struct CrossTimeTransform<T: TimePoint = Timestamp> {
    // Private fields; constructed only by Registry.
}

impl<T: TimePoint> CrossTimeTransform<T> {
    pub fn target_frame(&self) -> &str;
    pub fn source_frame(&self) -> &str;
    pub fn target_time(&self) -> T;
    pub fn source_time(&self) -> T;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Quaternion;
}
```

Both types implement `Debug`, `Clone`, and exact `PartialEq`. `Transform` retains its `approx` implementations.

The changes earn their place as follows:

| Element | Replaces in 2.4 | Reason |
|---|---|---|
| `target_frame` / `source_frame` | `parent` / `child` accessors and constructor parameter names | A sibling lookup does not establish a parent-child relationship. Use mapping vocabulary throughout geometry; reserve parent-child terminology for storage topology |
| `stamp` | `timestamp` | Its value is a `Stamp`, which may denote all time |
| Borrowing `compose` | Consuming, `Result`-returning `Mul` | Makes fallibility explicit, avoids cloning operands merely to retain them, and permits ordinary `a.compose(&b)?.compose(&c)?` |
| `CrossTimeTransform` | Cross-time results masquerading as ordinary transforms | Retains both instants and blocks accidental insertion and single-time composition |
| No public `validate` | Public re-validation of potentially invalid derived transforms | Successful construction and arithmetic now establish the invariant |
| No public `interpolate` | `Transform::interpolate` | Registry interpolation remains; no demonstrated caller needs this second metadata-checking interface |
| No `Point`, `Transformable`, `Localized`, `get_transform_for` | The existing observation/application layer | The public accessors already suffice to implement it exactly; the scope rule places it outside |

`CrossTimeTransform` has no public constructor, conversion to `Transform`, composition, inverse, serde implementation, or application method. It also has no stored fixed-frame name: that name supplies a physical assumption to the query, not an invariant the returned geometry can subsequently verify.

Deleting the Point family has a real cost. The review documents an actual wrong-direction application caught by `Point`, not merely hypothetical usage. I nevertheless accept deletion under the stated scope rule and the 2.4 decision against a positions helper. The replacement documentation must provide checked application recipes, including source-frame/time checks, position-versus-vector semantics, finite-result checks, and cross-time relabeling. I would not describe deletion as safety-neutral. See [review item 7](analysis/v3-review/REPORT.md:343) and [2.4’s application-scope ruling](analysis/v2.4-proposal/DESIGN.md:216).

The numerical contract is the most substantial departure from the review:

1. Constructor input must have finite components and satisfy the existing norm tolerance.
2. Accepted rotations are normalized when their computed squared norm differs from one by more than `8 * f64::EPSILON`.
3. Every private isometry-producing operation checks finiteness and restores that same normalization band.
4. Every returned `Transform` and `CrossTimeTransform` therefore has finite components and a rotation within that band.
5. Exact stored-sample lookup bypasses arithmetic and returns the stored geometry bit for bit.
6. Inversion retains normalization before conjugation.
7. No translation magnitude bound is imposed.

The normalization helper must check its postcondition. If normalization cannot establish the band, it returns an error; it must not rely solely on an empirical rounding assumption. This also makes repeated constructor/decode normalization a fixed point for successfully constructed values.

This is **repairing ordinary rounding drift**, not running the old `validate()` after every product and rejecting long legitimate chains. Constructor acceptance and derived-result maintenance are distinct internal operations.

I would apply maintenance to each isometry product and interpolation result, including registry folds. That keeps the internal invariant local and prevents unchecked accumulation from becoming somebody else’s responsibility. Its performance cost is unmeasured.

The review is too timid in excluding public composition from finiteness enforcement because a property-test fixture fails. That fixture explicitly manufactures infinity with multiplication and unwraps the result; it should change when multiplication stops manufacturing invalid transforms. It is not evidence against the stronger contract. See [review item 4](analysis/v3-review/REPORT.md:261) and [the fixture](tests/properties.rs:123).

Likewise, bounded constructor translations do not make unrestricted composition total. Reusing derived operands grows their expression’s leaf multiplicity exponentially. The `1e100` proposal confuses that multiplicity with the number of operations performed.

This contract does **not** promise well-conditioned arithmetic, exact real-number rotations, or successful evaluation whenever the mathematical final result is representable. Intermediate overflow may still produce an error. It promises that success never exports invalid transform numbers.

The registry remains a strict, synchronous tree with caller-managed concurrency:

```rust
pub struct Registry<T: TimePoint = Timestamp> {
    // Private fields.
}

impl<T: TimePoint> Registry<T> {
    pub fn new() -> Self;
    pub fn with_max_age(max_age: Duration) -> Self;

    pub fn add_transform(
        &mut self,
        transform: &Transform<T>,
    ) -> Result<(), RegistryError<T>>;

    pub fn add_transforms(
        &mut self,
        transforms: &[Transform<T>],
    ) -> Result<(), RegistryError<T>>;

    pub fn get_transform(
        &self,
        target_frame: &str,
        source_frame: &str,
        at: T,
    ) -> Result<Transform<T>, RegistryError<T>>;

    pub fn get_transform_at(
        &self,
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
    ) -> Result<CrossTimeTransform<T>, RegistryError<T>>;

    pub fn latest_common_time(
        &self,
        target_frame: &str,
        source_frame: &str,
    ) -> Result<Stamp<T>, RegistryError<T>>;

    pub fn set_max_gap(
        &mut self,
        child_frame: &str,
        max_gap: Option<Duration>,
    ) -> Result<(), RegistryError<T>>;

    pub fn reparent_frame(
        &mut self,
        seed: &Transform<T>,
    ) -> Result<(), RegistryError<T>>;

    pub fn remove_frame(&mut self, child_frame: &str) -> bool;

    // Removes dynamic samples at or before cutoff.
    pub fn remove_transforms_before(&mut self, cutoff: T);
}
```

`Registry` retains `Default` and its summary-oriented `Debug`. No `Clone` is needed.

Borrowed mutation inputs are a deliberate change, not an incidental signature preference. Current insertion consumes the sample even when it rejects it; the proposed expiry recovery consequently rebuilds or pre-clones the sample. Borrowing fixes that without introducing a rejected-value wrapper. Storage already copies geometry and retains names independently.

A slice batch replaces the iterator interface. It provides stable member indices, leaves every input available on failure, and eliminates iterator-consumption semantics and the need to collect owned inputs. The disadvantage is that lazy or non-contiguous producers must materialize a batch; I consider that acceptable for an explicitly atomic transaction.

Batch semantics remain ordered and atomic:

- Success has the same stored state as ordered single insertion.
- Failure is the first rejection that loop would encounter, augmented with its zero-based batch index.
- Expiry uses each frame’s running newest stamp, including earlier batch members.
- Nothing—including eviction—commits until the entire batch passes.
- `[50, 100]` and `[100, 50]` retain the maintainer’s different outcomes under a ten-second window.
- A reported `newest` may belong to an earlier, uncommitted batch member. It does not identify which timestamp is wrong.

Internally, preflight produces a commit plan with no remaining logical checks. I prefer that over committing through fallible insertion and relying on “these errors cannot happen.” The current implementation’s reliance on sample-independent checks is explicit in [batch insertion](src/core/registry/mod.rs:231); expiry makes the staged running state essential.

I retain the settled expiry and cleanup decisions, including:

- Expiry keeps its boundary sample; manual cleanup removes its boundary sample.
- A reparent seed replaces history and is not measured against discarded history.
- Failed retention subtraction applies no cutoff, as D3 specifies.
- No automatic reset detection and no `clear_history`.
- Expiry is checked last, but recovery examples handle reinsertion failure rather than depending on a published precedence guarantee.

The contracts are recorded in [review item 10](analysis/v3-review/REPORT.md:417) and [the draft’s unresolved-order warning](analysis/v3-review/sample-expired-contract.md:1).

Reparenting keeps the child’s static/dynamic kind and gap policy, drops that edge’s history, and leaves descendants intact. Static replacement remains retroactive. `remove_frame` continues to remove the incoming edge, not recursively destroy a subtree. Name reuse intentionally reconnects retained descendants; public IDs would not resolve that semantic question.

**I would ship the optional maximum interpolation gap.** This is the principal surface addition beyond diagnostics and the cross-time type.

Its behavior is narrow:

- It applies to one existing dynamic child edge.
- `None` means unrestricted interpolation between retained samples.
- `Some(Duration::ZERO)` permits exact stored samples only.
- Exact samples always remain eligible.
- An interior request fails when its two bracketing samples exceed the configured gap.
- Unknown frames and roots/static edges fail distinctly.
- Reparenting preserves the policy; removing the edge releases it.
- Configure it under the same exclusive access used to insert the initial sample.

The need is stronger than the review’s “nobody asked” framing suggests: the downstream study records a lost temporal-tolerance behavior in dimos and the same unbounded interpolation exposure in the adapters. The correct implementation requires sealed sample spacing. That meets the core-or-nowhere test, even though it does not establish a universal default. See [dimos’s findings](analysis/dependents/dimos.md:300) and [the proposed gap shape](analysis/dependents/dimos.md:598).

Two corrections are necessary:

- **A gap limit does not detect all future stamps, clock resets, or bad motion.** An incorrect exact sample remains eligible; a sufficiently small gap passes. The review’s suggestion that this mechanism “would close” the future-stamp problem is too broad.
- **Hole-aware latest-common-time is not generally `O(h log n)`.** With zero gap limits and alternating sample timestamps on two edges, the search can inspect many historical samples before proving there is no common instant.

My algorithm starts at the minimum newest timestamp. Each edge either serves that candidate or lowers it to its newest eligible predecessor. Repeat until every edge serves it or the candidate falls before a required edge’s start. Each reduction skips a real sample boundary; termination follows from finite stored histories. Preserve the current fast path when no gap policy applies. Do not expose coverage intervals to make callers implement this themselves.

The error design stays flat for every registry call, but retains facts the caller cannot reconstruct reliably:

```rust
#[non_exhaustive]
pub enum TransformError<T: TimePoint = Timestamp> {
    NonFiniteValues,
    NonUnitRotation {
        norm: f64,
    },
    EmptyFrameName {
        target_frame: String,
        source_frame: String,
    },
    IncompatibleFrames {
        expected: String,
        found: String,
    },
    TimestampMismatch {
        lhs: T,
        rhs: T,
    },
}

#[non_exhaustive]
pub enum RegistryError<T: TimePoint = Timestamp> {
    UnknownFrame {
        frame: String,
    },
    Disconnected {
        frame_a: String,
        frame_b: String,
    },
    NotFoundAt {
        frame: String,
        requested: T,
        covered: Option<(T, T)>,
    },
    NoCommonTime {
        frame: String,
        covered: Option<(T, T)>,
        latest_possible: Option<T>,
    },
    GapExceeded {
        frame: String,
        requested: T,
        before: T,
        after: T,
        max_gap: Duration,
    },
    TimeArithmetic {
        frame: String,
        requested: T,
        before: T,
        after: T,
        cause: TimeError,
    },

    SelfReferentialFrame {
        frame: String,
        batch_index: Option<usize>,
    },
    CycleDetected {
        parent: String,
        child: String,
        batch_index: Option<usize>,
    },
    ParentMismatch {
        frame: String,
        current_parent: String,
        requested_parent: String,
        batch_index: Option<usize>,
    },
    StaticDynamicConflict {
        frame: String,
        incoming: Stamp<T>,
        batch_index: Option<usize>,
    },
    SampleExpired {
        frame: String,
        stamp: T,
        newest: T,
        cutoff: T,
        batch_index: Option<usize>,
    },

    NoParentToReplace {
        frame: String,
    },
    ParentUnchanged {
        frame: String,
        parent: String,
    },
    NoDynamicEdge {
        frame: String,
    },

    NonFiniteValues,
    NonUnitRotation {
        norm: f64,
    },
}

#[non_exhaustive]
pub enum TimeError {
    DurationUnderflow,
    DurationOverflow,
    AccuracyLoss,
}

#[non_exhaustive]
pub enum QuaternionError {
    ZeroLengthNormalization,
    NonFinite,
}
```

Struct variants also receive `#[non_exhaustive]`. All errors implement `Debug`, `Display`, and `core::error::Error`; `TimeArithmetic` exposes its cause through error chaining.

Important choices in this schema:

- `batch_index` is `Some(i)` only for batch insertion. A child name alone cannot distinguish repeated samples of the same edge.
- `StaticDynamicConflict.incoming` identifies the rejected kind and, for dynamic data, its exact stamp. The stored kind is necessarily the opposite.
- `SampleExpired.cutoff` reports the actual comparison boundary.
- `TimeArithmetic` replaces the nested `RegistryError::TransformError(TransformError::TimestampError(...))`, adding the affected edge and interpolation bracket.
- `Disconnected` names an actual disconnected pair. For cross-time lookup, that may be an endpoint and the fixed frame. Symmetric field names avoid pretending that the fixed frame occupied the caller’s source or target role.
- Temporal errors name the failing edge and relevant instant, rather than repeatedly copying the original query endpoints. Adapters can retain broader request context.
- `NoCommonTime` with empty coverage has `latest_possible: None`. Otherwise it supplies the candidate upper bound that lies before the named edge’s start. With gaps, overlapping coverage envelopes no longer imply success.
- There is no public `From<TransformError<T>> for RegistryError<T>`. Registry internals map their own numerical failures directly into the flat variants; they do not need to accept every public composition error.

I reject the review’s unit `TimestampMismatch`. The probe demonstrates loss in the **typed payload**, not merely the text. Exact stamps justify `TransformError<T>`; three downstream signature edits are not a design objection under this mandate. Borrowing operands also removes the review’s argument that callers should capture diagnostics before consuming them. The existing lossy conversion occurs in [composition](src/geometry/transform/mod.rs:542).

Empty names move to construction and deserialization. This revisits 2.4’s D6 enforcement-site decision for v3; it does not claim 2.4 failed to implement that ruling. Public composition also resolves matching frame names, and the probe successfully joins `"a" -> "" -> "b"`. The hazard is therefore not intrinsically registry-only.

Self-named transforms remain constructible for identity and round-trip results; insertion rejects self edges. A same-frame query remains identity for a nonempty, even unregistered name. Empty query names are rejected before that shortcut.

Lookup diagnosis uses one rule:

1. Unknown frames on required, nonidentity legs.
2. Disconnected topology.
3. Coverage, gap, and clock-arithmetic failures.
4. Numerical evaluation failures.

Resolve topology for both cross-time legs before evaluating geometry. Within an error class, use target-leg-first ordering. Availability errors must refer only to connecting edges. This avoids the review’s partly repaired situation where a failure encountered while evaluating one leg can hide a stronger diagnosis on the other.

The time API becomes smaller:

```rust
pub trait TimePoint: Copy + Ord + core::fmt::Debug {
    fn duration_since(self, earlier: Self) -> Result<Duration, TimeError>;
    fn checked_sub(self, duration: Duration) -> Result<Self, TimeError>;
}

pub enum Stamp<T: TimePoint = Timestamp> {
    Static,
    At(T),
}

impl<T: TimePoint> Stamp<T> {
    pub const fn is_static(&self) -> bool;
    pub const fn at(self) -> Option<T>;
}

pub struct Timestamp {
    // Private u64 nanoseconds.
}

impl Timestamp {
    pub const fn zero() -> Self;
    pub const fn from_nanos(nanos: u64) -> Self;
    pub const fn as_nanos(&self) -> u64;
    pub fn as_seconds_lossy(&self) -> f64;

    pub fn checked_add(self, duration: Duration) -> Result<Self, TimeError>;
    pub fn checked_sub(self, duration: Duration) -> Result<Self, TimeError>;
    pub fn duration_since(self, earlier: Self) -> Result<Duration, TimeError>;

    #[cfg(feature = "std")]
    pub fn try_now() -> Result<Self, TimeError>;
}
```

`Stamp` remains `Copy + Eq`, deliberately unordered, with its explicit serde tags. `Timestamp` retains its existing ordering traits and transparent integer encoding. `SystemTime` retains its `TimePoint` implementation under `std`.

This replaces:

- Result-returning timestamp operators with inherent checked methods.
- `Timestamp::as_seconds` with the existing explicitly lossy conversion.
- The panicking `Timestamp::now` convenience with `try_now`.
- Required `TimePoint::as_seconds_lossy` with diagnostics using the already-required `Debug`.

That last deletion matters: a clock should not need a meaningful floating-point seconds representation merely to participate in ordering and interpolation. Default timestamp diagnostics can show their actual integer values, and pre-epoch `SystemTime` no longer gets reduced to `NaN`. Custom `Debug` implementations remain caller-defined; exact typed payloads remain authoritative.

`TimeError::AccuracyLoss` stays: the repository’s picosecond clock genuinely uses it for spans that `Duration` cannot represent. See [tests/time_traits.rs:265](tests/time_traits.rs:265).

The trait contract must require pure, consistent arithmetic and explain its `Duration` representability limit. I would retain D3’s failed-subtraction behavior, but describe it as **“no cutoff is applied”**, not claim it proves every sample is within the configured age.

Removing `now` does not make platform clock access universally panic-free. `try_now` still delegates to the platform’s `SystemTime::now`; documentation must state the actual limitation.

The component types stay deliberately ordinary:

```rust
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self;
    pub const fn zero() -> Self;
}

pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    pub const fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Self;
    pub const fn identity() -> Self;
    pub fn conjugate(self) -> Self;
    pub fn normalize(self) -> Result<Self, QuaternionError>;
    pub fn norm(self) -> f64;
    pub fn rotate_vector(self, vector: Vector3) -> Vector3;
    pub fn slerp(self, other: Self, ratio: f64) -> Self;
}
```

Their existing arithmetic operators, `Copy`, exact `PartialEq`, `approx`, and optional serde implementations remain. Raw component arithmetic has ordinary floating-point behavior; the stronger invariant belongs to transforms. No axis-angle/Euler API, matrix family, or math-library integration feature is added.

For storage, I would simplify the existing representation before considering interning:

```rust
// All private.
struct Buffer<T: TimePoint> {
    parent: String,
    history: History<T>,
}

enum History<T: TimePoint> {
    Static(Isometry),
    Dynamic {
        samples: BTreeMap<T, Isometry>,
        max_gap: Option<Duration>,
    },
}
```

`Registry` retains `HashMap<String, Buffer<T>>` and the registry-wide `max_age`.

This removes three unnecessary representational possibilities:

- A registered buffer without a parent.
- An empty static buffer.
- A static slot carrying a complete `Transform` that duplicates metadata.

It also avoids copying the registry-wide retention policy into each dynamic buffer. These are internal simplifications, not measured speedups. The current static duplication and optional states are visible in [src/core/buffer/mod.rs:50](src/core/buffer/mod.rs:50).

For lookup, use one topology resolver that collects borrowed edge references, identifies the connecting path, and only then samples geometry. Preserve ancestor short-circuits, current fold direction, and at most one inversion per same-time lookup. Do not sample a shared trunk just to discard it.

I would not make zero allocation a public promise. Owned results allow users to release a registry lock immediately. A borrowed result would trade those allocations for lifetimes and potentially longer lock holding; adding both forms would duplicate surface without demonstrated need.

Public frame IDs are rejected. They would require registry ownership, stale-handle, deletion, reuse, and potentially generation-overflow contracts. Typed frame markers fit compile-time frame graphs poorly when the demonstrated users receive arbitrary middleware names. Internal interning remains a benchmark experiment, not part of the v3 design.

`Arc<str>` is not a universal solution: Rust gates `alloc::sync` on pointer atomics, while `Rc` changes thread-safety properties. See the [official `alloc::sync` documentation](https://doc.rust-lang.org/alloc/sync/index.html). None of this requires abandoning the existing string boundary.

For serde, keep the existing `Transform`, `Stamp`, `Timestamp`, and component wire shapes where practical, including the `parent`/`child` wire field names despite Rust accessor renaming. There is no benefit in gratuitous wire churn. Decode must run the same checks and normalization as construction; missing or null stamps remain errors.

Consequences must be explicit: empty-name payloads now fail decoding, accepted off-band rotations normalize, and `Point` decoding disappears. Successful normalized values should be fixed points through binary serde round trips. Do not promise that arbitrary JSON parsers preserve every floating-point bit.

Here is my verdict on the review’s twelve numbered items:

| Item | Verdict | Position |
|---|---|---|
| 1. Cross-time result type | **Adopt modified** | Keep six accessors and no implicit insertion; resolve both legs by error class and use honest leg-neutral error payloads |
| 2. Normalize constructed rotations | **Adopt modified** | Extend the invariant to every derived transform; check normalization’s postcondition |
| 3. Normalize inverse; delete wrapped quaternion variant | **Adopt** | Keep inverse normalization. Raw `QuaternionError` remains for raw quaternion normalization |
| 4. Finite lookup results | **Adopt modified** | Enforce for public composition and all private isometry-producing operations too |
| 5. Disconnected before temporal failure | **Adopt** | Apply consistently to same-time and cross-time queries |
| 6. Error-shape changes | **Adopt modified** | Exact generic timestamps, batch indices, useful frame fields, direct clock errors; reject unit timestamp mismatch |
| 7. Delete Point family | **Adopt** | Accept and document the lost checked-application path |
| 8. Delete public transform interpolation | **Adopt** | Preserve internal interpolation and use independent test expectations |
| 9. Delete `Timestamp::as_seconds` | **Adopt** | Also remove the panicking convenience and unnecessary clock diagnostic method |
| 10. Reject expired insertion | **Adopt** | Preserve settled ordered-batch semantics; borrowing inputs improves recovery |
| 11. Delete `SameFrameMultiplication` | **Adopt** | Ordinary frame compatibility is sufficient once cross-time results have their own type |
| 12. Inclusive manual cleanup | **Adopt** | Keep the settled name, although “before” is imperfect wording |

For the remaining proposals and alternatives:

| Proposal or review position | Verdict | Reason |
|---|---|---|
| Public `Rotation` | **Reject** | A private numerical invariant provides the required guarantee without another input/output type |
| Public `Isometry`; separate `Sample` and `Resolved` | **Reject** | Exports more unchecked composition machinery and wrappers than the demonstrated distinction requires |
| Translation bound and infallible arithmetic | **Reject** | Bound is not closed under reusable composition; finite checking is necessary |
| Newton correction on every product | **Reject** | Prefer thresholded normalization with an explicit checked postcondition; no measured reason to choose Newton |
| Conjugate-only inverse | **Reject** | Retain the better-tested normalization path; reassess only with accuracy and performance evidence |
| Constructor-only or insert-only normalization | **Reject** | Leaves the meaning of a successful transform dependent on its provenance |
| Named fallible arithmetic | **Adopt** | Ergonomics and ownership are legitimate goals even when misuse currently fails to compile |
| Lookup request struct/builder | **Reject** | Named fields are not “un-swappable”; no invariant justifies the additional type |
| Public IDs or typed frames | **Reject** | Lifecycle and ownership complexity without a demonstrated runtime use |
| Internal interning | **Reject for the initial design** | Prototype only against whole-query workloads, including failed lookups |
| Cached depth/root information | **Reject initially** | Mutation can invalidate it. Maintainable caches are possible, but must justify their bookkeeping |
| `VecDeque` history | **Reject** | Existing experiments favor retaining `BTreeMap`; I did not reproduce their timings |
| Required `max_age` / extra `unbounded` constructor | **Reject** | Keep current two modes. Retention duration is neither a sample-count bound nor an adequate outage policy |
| Maximum interpolation gap | **Adopt modified** | Per edge, optional, with hole-aware latest-common-time and honest worst-case complexity |
| Sample/frame capacity limits | **Reject for v3** | No settled allocation-failure or capacity policy; compilation is not a hard-real-time guarantee |
| Freshness policy, automatic reset detection | **Reject** | Requires the caller’s clock and stream assumptions |
| `clear_history` | **Reject** | Retain the maintainer’s decision; inclusive cleanup addresses the maximum-timestamp hole |
| Reject removal with children / cascade removal | **Reject** | Preserve deliberate subtree detachment and descendant history |
| Automatic kind switching | **Reject** | Would turn conflicting publishers into silent state transitions |
| Static-only fixed-frame ancestry check | **Reject** | Static representation does not prove physical stationarity; constant dynamic relationships can be legitimate |
| `latest_common_time -> T` | **Reject** | An all-static chain has no uniquely latest instant |
| `get_latest`, `can_transform`, retryability helpers, waits, bulk lookups | **Reject** | Buildable on existing queries or dependent on application policy |
| Public coverage, history, or static enumeration | **Reject** | Exposes internals rather than the narrow validated answer |
| Application methods, positions slices, `Point -> Pose` | **Reject** | Outside the current scope rule; renaming does not fix the layer’s placement |
| Euler/axis-angle, math-crate features, prelude/flat root | **Reject** | No demonstrated need warrants this surface |
| `f32`, mixed precision, scalar-family generics | **Reject** | Standing non-goal. Any reconsideration requires an explicit maintainer ruling and hardware evidence |
| Signed default timestamp | **Reject** | Existing custom clocks and `SystemTime` address demonstrated pre-epoch use |
| Replace `hashbrown` with `BTreeMap` | **Reject initially** | No demonstrated benefit |
| Optional `approx`; remove `thiserror` | **Reject** | Adds feature combinations or unrelated maintenance without improving the core contract |
| Cross-device replay guarantee | **Reject as an unverified claim** | Adopt cross-target comparison tests as evidence gathering |
| Sample fold, on-chain attribution, dead-state deletion, task examples | **Adopt as already shipped** | Do not count them again as v3 benefits |
| Golden vectors, differential testing, feature/target gate | **Adopt** | Compare against declared semantic changes, not blanket byte preservation |
| “Changes no number that is right today” | **Reject as an acceptance rule** | Correct floating-point computations need not have a unique bit pattern; specify exactness and tolerance obligations separately |

The corrections and “already shipped” addenda are useful factual updates. The warrant report’s old release triggers are not constraints on this exercise: the maintainer has explicitly requested a reimagining. Likewise, “this could be additive later” is not a reason to omit a warranted capability from a coherent v3 design.

The review’s most important remaining mistakes are therefore substantive:

- It protects lookup callers while leaving invalid public transform arithmetic intact.
- It removes exact timestamp diagnostics to avoid migration work.
- It treats a failing old-contract test fixture as a reason to preserve that contract.
- It understates the gap-limit case, while overstating what such a limit can guarantee.
- It counts public items without sufficiently considering what each item commits the crate to. A batch index is a smaller commitment than asking every adapter to reconstruct a failed transaction.

I ran:

```text
cargo test --locked --all-features --quiet
cargo test --locked --no-default-features --quiet

cargo bench --locked --bench benchmarks -- dynamic_chain_4hop \
  --warm-up-time 0.1 --measurement-time 0.2 --noplot
```

Both test runs passed. On this i7-12850HX, using Rust `1.101.0-nightly (b6eeb93c9)`, the short baseline benchmark reported approximately:

| Existing 2.4 benchmark | Estimate |
|---|---:|
| Four-hop interpolated lookup toward ancestor | 1.39 µs |
| Four-hop interpolated lookup toward descendant | 1.15 µs |

These are noisy baseline observations, not a comparison with my proposal. I disregard Criterion’s comparisons against pre-existing saved baselines.

A scratch normalization experiment completed one million repeated products and 100,000 random products with no fixed-point assertion failure. Maximum observed maintained squared-norm error was `1.7763568394002505e-15`; the unmaintained repeated product accumulated approximately `8.58e-11`. See [normalization.rs:2](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/probe/src/bin/normalization.rs:2). This supports feasibility, not a proof or a registry-performance claim.

My most contestable decisions, ranked:

1. **Maintain the numerical invariant after every isometry operation.**  
   Strongest objection: extra norm work and checks may matter substantially on soft-float targets, while ordinary chains already drift very little. A boundary-only strategy is cheaper. I favor the simpler invariant, but its measured cost could change the implementation strategy.

2. **Ship per-edge gap limits now.**  
   Strongest objection: the evidence establishes exposure and lost downstream tolerance, not a requested exact policy. Hole-aware common-time search adds real complexity and potentially long scans. This is the first item I would isolate if release scope must shrink.

3. **Delete the Point/application layer.**  
   Strongest objection: the crate removes an actual checked path and leaves users to copy checks that they may omit. If the maintainer revises the scope rule, a narrow checked application design deserves reconsideration before deletion.

4. **Use borrowed samples and slice batches.**  
   Strongest objection: ownership transfer is familiar, can reuse allocations, and accepts streaming producers. The slice requires materialization. I favor retention of rejected inputs and exact member attribution, but would compare actual adapter code before freezing this signature.

5. **Remove `TimePoint::as_seconds_lossy` and use typed/Debug diagnostics.**  
   Strongest objection: Debug formatting is less friendly and custom implementations can be poor. I favor deleting a required clock operation that does not participate in the time algebra.

6. **Move empty-name rejection to constructors.**  
   Strongest objection: this tightens a recently deliberate boundary and rejects previously decodable records. The public-composition probe is why I consider that tightening warranted in v3.

7. **Keep strings and owned results.**  
   Strongest objection: this preserves allocations and an expensive frame-existence failure path that embedded users may care about. I would change this only after a prototype demonstrates a whole-workload benefit without lifetime or handle hazards.

8. **Replace operators and adopt `TransformError<T>`.**  
   Strongest objection: existing mistakes fail loudly and the new signatures increase downstream migration work. Under a compatibility-free ergonomics mandate, that objection carries less weight than it did in the earlier reviews.

Confidence is high in the reproduced hazards, cross-time separation, exact diagnostic payloads, rejection of the magnitude-bound argument, and preservation of the tree/time invariants. Confidence is moderate in the complete proposed API and lower in its performance tradeoffs.

I did not implement the complete design, build downstream applications, run the full verification gate on a prototype, measure bare-metal hardware, or establish cross-device bitwise behavior. Downstream findings are the repository’s pinned studies, not a fresh ecosystem census. All proposed performance improvements remain unmeasured. The working tree remains clean; scratch work is confined to the requested directory.