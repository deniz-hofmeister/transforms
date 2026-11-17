# transforms v3 — designer A's position (draft 1, pre-exchange)

Baseline: master `adccbdd` (v2.4.0). Mandate: re-imagining, no backwards
compatibility. Evidence files (same directory as this one): `map-api.md`,
`map-internals.md`, `map-dependents.md`, `map-v3-history.md`,
`map-2.4-state.md`, `map-prior-art.md`.

## 0. Verdict on the round-2 review (`analysis/v3-review/REPORT.md`)

The review is careful and its measurements are the best evidence in the
repository, but it answers "what is the smallest safe break?" under two
self-imposed rules the new mandate removes: frozen type names/paths/wire
bytes for the re-export chain, and "3.0 changes no number that is right
today". Consequences:

- It patches hazards at call sites (normalize in three constructors, a
  finiteness check at two exits, a one-off `CrossTimeTransform`) where a type
  would carry the invariant once. Its own residual list (insert validates but
  does not normalize; `inverse` stays fallible; a zero-norm rule; the
  `From<TransformError>` canonicalization; "derived values are not valid by
  construction") is exactly what a unit-by-construction `Rotation` deletes.
- It rejects the conjugate inverse on bit changes and a flaky tolerance pin,
  while conceding the numerics are sound and 22% faster.
- It deletes the only checked application path (`Point`) but keeps labels on
  results "so a check can be built outside" — labels that then check nothing.
- It never questions the generic `T: TimePoint`, the flat mixed
  `RegistryError`, `String` frames in every value, the dependency list, or the
  `std` feature.
- It leaves the measured hot-path wastes alone (data-first resolve samples
  edges above the common ancestor; a hash per hop; 2 result `String`s = 39% of
  a one-hop exact lookup; `frame_exists` is O(frames): 94 µs at 1k frames).

What it gets right and I keep: `SampleExpired` (D1–D6), the inclusive cleanup
cutoff, `Disconnected` before `NotFoundAt`, no non-finite lookup result,
deleting `Point`/`Transformable`/`Localized`/`get_transform_for`/
`Transform::interpolate`/`as_seconds`, rejecting public `FrameId`, typed
frames, f32, caches, `can_transform`, a core wait layer.

## 1. Public API (the whole of it)

Flat crate root, no public sub-modules. ~12 type names, down from 105 named
items to roughly 45.

```rust
// ---- geometry: plain values, Copy, no frames, no time ----
pub struct Vector3 { pub x: f64, pub y: f64, pub z: f64 }   // From<[f64;3]>, Into<[f64;3]>; no operators

pub struct Rotation { /* private unit quaternion */ }
impl Rotation {
    pub const IDENTITY: Self;
    /// Err unless finite and | |q| - 1 | <= UNIT_NORM_TOLERANCE; stored normalized.
    pub fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Result<Self, GeometryError>;
    pub fn w(self) -> f64; pub fn x(self) -> f64; pub fn y(self) -> f64; pub fn z(self) -> f64;
    pub fn inverse(self) -> Self;                    // conjugate, total
    pub fn rotate(self, v: Vector3) -> Vector3;      // total
}
impl Mul for Rotation { type Output = Rotation; }    // total; one sqrt-free Newton renormalization step

/// A rigid transform: p_parent = rotation * p_child + translation.
pub struct Transform { /* private: translation, rotation */ }
impl Transform {
    pub const IDENTITY: Self;
    pub fn new(translation: Vector3, rotation: Rotation) -> Result<Self, GeometryError>; // finite check only
    pub fn translation(self) -> Vector3;
    pub fn rotation(self) -> Rotation;
    pub fn inverse(self) -> Self;                           // total
    pub fn transform_point(self, p: Vector3) -> Vector3;    // R p + t
    pub fn transform_vector(self, v: Vector3) -> Vector3;   // R v
}
impl Mul for Transform { type Output = Transform; }  // total (IEEE overflow can yield inf; see §3)

// ---- time: concrete, exact ----
pub struct Timestamp(/* i64 nanoseconds */);          // Copy, Ord, Hash, Display (exact s.nnnnnnnnn)
impl Timestamp {
    pub const MIN: Self; pub const MAX: Self;
    pub const fn from_nanos(n: i64) -> Self;
    pub const fn as_nanos(self) -> i64;
    pub fn from_secs_nanos(secs: i64, nanos: u32) -> Option<Self>;   // checked; the ROS/NTP shape
    pub fn checked_add(self, d: Duration) -> Option<Self>;
    pub fn checked_sub(self, d: Duration) -> Option<Self>;
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration>;
}
#[cfg(feature = "std")] impl TryFrom<SystemTime> for Timestamp { .. }

pub enum Stamp { At(Timestamp), Static }              // unordered, explicitly tagged serde; From<Timestamp>

// ---- the tree ----
/// One sample of the edge parent <- child. Named fields are the named arguments.
pub struct Edge<'a> { pub parent: &'a str, pub child: &'a str, pub transform: Transform, pub stamp: Stamp }

pub struct Registry { /* private */ }
impl Registry {
    pub fn with_max_age(max_age: Duration) -> Self;
    pub fn unbounded() -> Self;                        // no `new`, no Default: retention is a decision

    pub fn insert(&mut self, edge: Edge<'_>) -> Result<(), InsertError>;
    /// All-or-nothing; equals the insert loop. The error names the failing member.
    pub fn insert_all<'a>(&mut self, edges: impl IntoIterator<Item = Edge<'a>>) -> Result<(), BatchError>;
    pub fn reparent(&mut self, seed: Edge<'_>) -> Result<(), InsertError>;
    pub fn remove_frame(&mut self, child: &str) -> bool;
    pub fn remove_samples_through(&mut self, cutoff: Timestamp);   // inclusive; MAX clears all dynamic history

    /// Expresses `source` coordinates in `target` at `at`: p_target = result.transform_point(p_source).
    pub fn lookup(&self, target: &str, source: &str, at: Timestamp) -> Result<Transform, LookupError>;
    pub fn lookup_across(&self, target: (&str, Timestamp), source: (&str, Timestamp), fixed: &str)
        -> Result<Transform, LookupError>;
    pub fn latest_common_time(&self, target: &str, source: &str) -> Result<Stamp, LookupError>;
}

// ---- errors: one enum per operation, each flat, all #[non_exhaustive], Clone + PartialEq + Debug + Display + core::error::Error ----
pub enum GeometryError { NonFinite, NonUnitRotation { norm: f64 } }
pub enum InsertError {
    EmptyFrameName,
    SelfParent { frame: String },
    ParentMismatch { frame: String, pinned: String, offered: String },
    KindMismatch { frame: String, pinned_static: bool },      // static xor dynamic
    Cycle { child: String, parent: String },
    Expired { frame: String, stamp: Timestamp, newest: Timestamp },
    // reparent only:
    UnknownFrame { frame: String }, NoParent { frame: String }, ParentUnchanged { frame: String },
}
pub struct BatchError { pub index: usize, pub error: InsertError }
pub enum LookupError {
    UnknownFrame { frame: String },
    Disconnected { target: String, source: String },
    NoData { frame: String },                                  // the edge holds nothing
    OutOfRange { frame: String, requested: Timestamp, earliest: Timestamp, latest: Timestamp },
    NoCommonTime { frame: String, .. },                        // latest_common_time only
    NonFinite,                                                 // only if the exit check stays (§3)
}
```

### What each element replaces, and why

| v3 | Replaces (2.4) | Why it earns its place |
|---|---|---|
| `Rotation` (unit, private) | `Quaternion` (pub fields, unnormalized), `normalize`, `conjugate`, `norm`, `slerp`, `QuaternionError`, `UNIT_NORM_TOLERANCE` checks in 3 places | The invariant lives in one type instead of being re-checked at constructors, insert, and lookups. `inverse`, `*`, `rotate` become total. Not an extra type: the open quaternion goes. Slerp becomes private. |
| `Transform` (geometry only, `Copy`) | `Transform<T>` (labelled, stamped, 2 `String`s), private `Isometry`, `Hop`, `Transform::validate`, `inverse -> Result`, `Mul -> Result`, `interpolate`, `static_between`, `CrossTimeTransform` (review item 1) | One arithmetic path instead of two (public `Transform` ops have zero internal callers today). Lookup results allocate nothing. A cross-time result is the same plain geometry: there is no insertable labelled value to mis-insert, so the special type is unnecessary. `transform_point`/`transform_vector` pin the application convention in code (golden vectors can pin a crate path again). |
| `Edge<'a>` (named pub fields, borrowed names) | `Transform::new(parent, child, ..)` positional, owned `String`s (122 ns of a ~314 ns insert is alloc+drop) | Struct-literal fields are named arguments (parent/child swap is visible). No allocation. It is input-only, so it needs no invariant of its own. It is also the batch item. |
| No labels on results | `parent()`/`child()`/`timestamp()` on results; `IncompatibleFrames`, `SameFrameMultiplication`, `TimestampMismatch` | The caller named the frames one line earlier. No studied user consumes the labels through a checked path (zero uses of `Point`, `Mul`, `inverse`); with `Point` gone they check nothing. A typed-frame layer is a companion crate on this API. |
| `Timestamp(i64 ns)`, concrete | generic `T: TimePoint` on `Registry`, `Transform`, `Stamp`, `RegistryError`, `Point`, traits; `TimePoint`; `TimeError`; `as_seconds_lossy`; `Result`-returning operators; `Timestamp::now`/`try_now`; `impl TimePoint for SystemTime` | Removes E0283 on `Registry::new()`, the `RegistryError::TransformError(TimestampError(..))` nesting, lossy `f64` seconds in `Display` (a 1 ns miss prints as inside the range), and `tests/time_traits.rs`. Signed: every ROS `(i32, u32)` stamp converts losslessly (two of four hand-rolled conversions alias or clamp today). No panic left in the crate. Cost: dimos uses `SystemTime` as `T` and converts at the boundary instead. |
| `InsertError` / `LookupError` / `GeometryError` | flat `RegistryError<T>` (14 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError` | A lookup `match` no longer has to consider `CycleDetected`. Every structural insert error names its frame (ranked #1 need in the dependents study). `NotFoundAt { covered: Option }` splits into `NoData` and `OutOfRange` — the Option-shaped payload is the thing the AGENTS.md invariant warns callers not to misread. Errors derive `Clone + PartialEq`. |
| `BatchError { index, error }` | "the error does not identify the rejected transform" | Core-or-nowhere: the loop can identify the member only by committing the others. |
| `with_max_age` / `unbounded` | `new` + `Default` (unbounded, silently) | Every studied user sets `max_age`; nobody uses `new`. Unbounded memory and unbounded interpolation gaps should be typed out. |
| `remove_samples_through` | `remove_transforms_before` with a silently changed boundary | The review's item 12 changes the boundary under an unchanged name; a rename makes it loud. `Timestamp::MAX` gives the full dynamic clear without a `clear_history`. |
| `lookup_across((frame, time), (frame, time), fixed)` | `get_transform_at(&str, T, &str, T, &str)` | Pairs bind each time to its frame. |
| deps: `libm` (+ optional `serde`) | `thiserror`, `approx`, `hashbrown` | Three small error enums need no proc macro; `approx` impls are test support exposed as public API; with interned frames only two names are looked up per query, so a `BTreeMap<Box<str>, u32>` is deterministic, DoS-free and dependency-free (to be measured). |

## 2. Internals

- **Interned frames.** `Vec<Slot { name: Box<str>, parent: Option<u32>, edge: Option<Buffer>, children: u32 }>` plus a name index. Every mentioned name (roots included) has a slot, so `frame_exists` is O(1). Walks are index chases (measured stand-in: 162 ns → 5.6 ns for 4 hops). `FrameId` stays private: no generations, no brand. A slot is reclaimed when it has no edge and no children.
- **Topology-first resolve, one pass, no allocation.** Compute both depths by walking to the root (index chases), lift, find the common ancestor, then sample only the connecting edges while folding. One inversion at most. The same routine serves `latest_common_time`; the second "diagnosis" pass disappears. Error precedence `UnknownFrame > Disconnected > NoData/OutOfRange`, target side first.
- **History as a sorted `VecDeque<(Timestamp, Transform)>`** (64-byte entries). Measured stand-in: bracketing lookup 112 → 46 ns, append+evict 129 → 11 ns at n = 1k. Out-of-order insert is O(min(i, n−i)); realistic late samples land near the back. The 2.1.3 experiment measured 42× slower for uniform out-of-order inserts into 60k samples (22 µs each) — that is the cost, stated.
- **Statics are a bare `Transform`**, outside the time path.
- Unconditionally `#![no_std]` + `alloc` internally; `std` only adds `TryFrom<SystemTime>`.

## 3. Numerics

- `Rotation` is unit to a few ulps always: constructors normalize (review item 2's rule moved into the type), and `*`/slerp apply one Newton step `q·(3−|q|²)/2` (v3-3: holds |‖q‖−1| ≤ 2.2e-16 over 1e9 compositions; ~13 flops on ~87). So `inverse` is the conjugate and is exact to rounding, insert never rejects a derived rotation, and "valid by construction" is true of every value, not only built ones.
- Single-hop lookup at a stored stamp stays bit-exact (no composition runs).
- Translation overflow: either keep one finiteness check at lookup exit (`LookupError::NonFinite`), or bound |t| at construction (v3-3's `B`) so lookups cannot overflow. I lean to the exit check: exact, no magic constant. Public `Transform * Transform` follows IEEE (can produce inf); insert rejects non-finite.
- libm everywhere, golden vectors re-pointed at `transform_point` with the same SciPy digits.

## 4. Candidates I am unsure about (to argue out)

1. **`lookup_latest(target, source) -> Result<(Stamp, Transform), LookupError>`.** transforms_io's hand-built "latest" was wrong for seven releases (`now()` succeeds silently on post-dated hops); both adapters hide the registry so users cannot hold one guard across `latest_common_time` + `get_transform`. That is "wrong-answer generator outside" evidence. Against: 2.4 ruled no `get_latest`; one more method.
2. **Maximum interpolation gap.** Hazard reproduced in all three users (`Ok(4.0)` across an 8 s dropout), core-or-nowhere, dimos lost a bound it had. No settled shape: registry-wide is wrong for mixed-rate chains; per-query tolerance has the same flaw; per-edge needs configuration surface. Keep out of 3.0 unless a shape survives review.
3. **A second, normalizing rotation constructor** (`0.7071, 0, 0, 0.7071` is rejected at 1e-6 where tf2 accepts it).
4. **Frame enumeration** (`frames()`): transforms_io keeps a shadow log of statics for lack of a read-out. AGENTS.md says no raw state.
5. **Drop the `std` feature entirely.**
6. **Lookup argument struct** vs positional `(target, source, at)`.

## 5. Kept from 2.4 without change

Strict single-parent tree; first insert pins parent and kind; explicit `reparent`; descendant-preserving `remove_frame`; atomic batch equal to the loop; no extrapolation; data-driven expiry; caller-managed concurrency; exact `==`; `#[non_exhaustive]` errors; `#![forbid(unsafe_code)]`; all Non-Goals; MSRV 1.85.
