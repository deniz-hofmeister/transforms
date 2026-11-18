# Prior art map for a v3 of `transforms`

Date: 2026-11-17. Method: primary sources fetched through a summarizing web-fetch tool (so quotes are
as relayed by that tool, not read byte-for-byte by me). Tags: **[V]** = fetched from the primary source
this session; **[U]** = from memory / secondary, unverified. Statements about the crate's current state
come from AGENTS.md, not from re-reading `src/` (no file:line citations are made for the repo; this
pass was web-only).

---

## (a) ROS 2 tf2 `BufferCore`

Sources: `ros2/geometry2` rolling — `tf2/src/buffer_core.cpp`, `tf2/src/cache.cpp`,
`tf2/src/static_cache.cpp`, `tf2/include/tf2/{buffer_core,time_cache,exceptions}.hpp`.

### Facts

- **Interning [V]**: `frameIDs_` is `std::unordered_map<std::string, CompactFrameID>`;
  `frameIDs_reverse_` is `std::vector<std::string>`; `frames_` is
  `std::vector<std::shared_ptr<TimeCacheInterface>>`. `lookupOrInsertFrameNumber` assigns
  `CompactFrameID(frames_.size())`, so the ID is the index into both vectors. ID 0 is the sentinel
  `"NO_PARENT"` (`frameIDs_["NO_PARENT"] = 0`, `frames_[0]` null). IDs are never freed.
  `CompactFrameID` is `uint32_t` **[U]** (typedef lives in `transform_storage.hpp`, not fetched).
- **Each sample stores its parent [V, partly]**: `getData` checks
  `p_temp_1->frame_id_ == p_temp_2->frame_id_` on the bracketing samples; if they differ it does **not**
  interpolate and returns the older sample unchanged, with its own stamp, and still returns `true`.
  So re-parenting is implicit, per-sample, and silently degrades interpolation to a hold.
- **TimeCache storage [V]**: `std::list<TransformStorage>`, newest at front. `insertData` prunes first,
  rejects only `stamp < latest - max_storage_time` (returns false, logged upstream as `TF_OLD_DATA`),
  finds position by linear `find_if`. Same-stamp, different-value samples are **both kept**; only exact
  duplicates are dropped. `findClosest` is a linear walk from newest. Interpolation ratio is unclamped
  doubles; lerp + slerp. Default storage 10 s (`TIMECACHE_DEFAULT_MAX_STORAGE_TIME`).
- **Single-sample dynamic frame [V]**: served only on an exact stamp match; otherwise
  `TF2_NO_DATA_FOR_EXTRAPOLATION_ERROR`.
- **Static cache [V]**: one `TransformStorage` + `populated_`; `insertData` overwrites silently;
  `getData` stamps the output with the requested time; `getLatest/OldestTimestamp` return
  `TimePoint()` (zero). In `setTransformImpl`, a static insert on a dynamic frame (and vice versa)
  **replaces the cache**, discarding history — kind changes are silent.
- **`time == 0` means "latest" [V]**: `walkToTopParent` substitutes `getLatestCommonTime` when
  `time == TimePointZero`; `findClosest` returns the front sample for zero. A reserved in-band value.
  `getLatestCommonTime` converts `TimePoint::max()` (all-static chain) to zero — a second in-band
  sentinel.
- **`walkToTopParent` [V]**: walk source→root accumulating; stop at null cache or failed `gather`
  (sets `extrapolation_might_have_occurred`); early finalize if target met; walk target→root; early
  finalize if source met; if walks do not meet, return the extrapolation error if one occurred, else
  `TF2_CONNECTIVITY_ERROR`; else finalize `FullPath` and prune the shared tail. Loop guard is a depth
  counter, `MAX_GRAPH_DEPTH = 1000`, error "The tf tree is invalid because it contains a loop." —
  cycles are detected at **lookup**, not at insert.
- **`getLatestCommonTime` [V]**: min of per-hop latest stamps over the two walks up to the common
  parent. No max-of-starts guard: it can return an instant an individual hop cannot serve.
- **`setTransform` validation [V]**: rejects self-transform, empty child/parent ids, NaN, and
  quaternion norm deviation beyond `QUATERNION_NORMALIZATION_TOLERANCE = 10e-3` (= 0.01). All
  rejections **log and return `false`**; nothing is thrown, the caller gets a bool. Leading slashes are
  silently stripped on insert but rejected on lookup.
- **Errors [V]**: `TransformException : std::runtime_error`; subclasses `ConnectivityException`,
  `LookupException`, `ExtrapolationException` (→ `Backward…`, `Forward…`, `NoDataForExtrapolation…`),
  `InvalidArgumentException`, `TimeoutException`. `TF2Error` enum has 10 codes. Payload is a formatted
  string only — no structured frame/time fields to match on.
- **Locking [V]**: one `std::mutex frame_mutex_` over everything; two more for the transformable
  request/callback machinery. Three `TODO(anyone): This is incorrect, but better than nothing.`
  comments in `lookupVelocity`, `addTransformableRequest`, `testTransformableRequests`.
- **Known pain points**: multiple static publishers stomping each other → `static_transform_mux`
  exists as a workaround for geometry2#181 **[U — search-result level, issue not opened]**;
  geometry2#437 asks for cycle errors **[U — same]**; ros/geometry2#388 reports a re-parented dynamic
  frame behaving like a static one **[U — same]**. I could not find and verify a canonical
  "design regrets" document; treat any such claim as unverified.

### Lessons for v3

- **Copy**: dense integer frame IDs with `Vec`-indexed storage and a reverse name table. The hot path
  (the walk) then never hashes or compares strings; only the two endpoint names are resolved. This is
  the single biggest structural performance idea tf2 has that a `HashMap<String, Buffer>` keyed design
  lacks (current crate hashes a `String` per hop, per AGENTS.md architecture).
- **Copy**: the walk shape (two upward walks, meet at common ancestor, compose each half, one
  inversion). The crate already does this.
- **Do NOT copy**: sentinel ID 0 / "NO_PARENT" in-band (use `Option<FrameId>` with a `NonZero` niche);
  time 0 = latest; per-sample parent; silent cache replacement on kind change; silent static
  overwrite; bool-returning insert; string-only error payloads; cycle detection at lookup via depth
  cap; `std::list` storage (pointer-chasing, linear search); keeping two samples at one stamp;
  slash-stripping asymmetry; min-only `getLatestCommonTime`. The crate's AGENTS.md invariants already
  reject every one of these; tf2's source confirms each is a real, shipped wrong-answer generator, not
  a hypothetical.
- **Open question tf2 raises**: IDs never freed. With `remove_frame` in the API, v3 must choose:
  tombstone (ID stays reserved, name table grows monotonically) or generational reuse (see (e)).

---

## (b) Rust crates

### sguaba 0.10.7 [V] (docs.rs)

- Frames are **types**: `Coordinate<In>`, `Vector<In>`, `RigidBodyTransform<From, To>`,
  `Rotation<From, To>`, declared with `system!(struct PlaneFrd using FRD)`.
- **Every** `RigidBodyTransform` constructor is `unsafe` — including `identity()` — on the argument that
  a wrong transform "defeats type safety". This repurposes `unsafe` for a logical (non-memory)
  invariant.
- Composition is type-checked: `<From, Over> * <Over, To> -> <From, To>`; docs concede operand order
  "do[es] not match the mathematical convention". `translation()` is described as "perhaps
  counter-intuitively" aligned with `new`. Direction is a documented source of confusion even with
  typed frames.
- Deserialize: "the coordinate system of the deserialized value is _not_ checked".
- Explicit non-goal: "does not attempt to provide time-variant type safety".
- Depends on `nalgebra ^0.35` and `uom ^0.38` (both required); `no_std` needs the `libm` feature.

Lesson: compile-time frames solve a different problem (a fixed, small set of frames known at build
time; no time axis; no tree). A runtime tree keyed by names received over the wire cannot use them.
What transfers: (1) the composition rule expressed as a signature — the crate already checks
`lhs.child == rhs.parent` at runtime; (2) the admission that direction naming is the residual hazard.
Do NOT copy: `unsafe` as a logical-invariant marker (incompatible with `#![forbid(unsafe_code)]` and
with a `Result`-first ethic); the nalgebra+uom dependency weight; math-order-reversed operators.
A possible *optional* v3 idea worth debating, not adopting by default: a zero-cost typed layer on top
of the public API in a companion crate — by AGENTS.md "Scope", that is "can be built on the public
API → outside".

### nalgebra 0.35.0 [V]

- `Unit<T>` has a private field; constructors `new_normalize`, `try_new(value, min_norm) -> Option`,
  `new_and_get`, `const fn new_unchecked`; escape hatch `as_mut_unchecked`. Drift is acknowledged:
  `renormalize()` "is useful when repeated computations might cause a drift in the norm", plus
  `renormalize_fast` (first-order Taylor).
- Whether `Unit`'s `Deserialize` validates the norm: **not determined** from the page **[U]** (my
  recollection is that it does not; unverified).
- `Isometry<T, R, const D>` has **public** `rotation` and `translation` fields; `inv_mul` computes
  `self.inverse() * rhs` cheaper; `inverse_transform_point` avoids materializing the inverse;
  `lerp_slerp` **panics** at 180 degrees, `try_lerp_slerp` returns `None`. No frame labels.

Lessons: (1) the `Unit<T>` pattern — the unit-norm invariant carried by a type with a private field —
is the idiomatic Rust answer to "Rotations are expected to be unit quaternions; `from_wxyz` does not
normalize". Today the crate validates at `Transform` construction but `Quaternion` has public fields
and no invariant; a v3 could move the invariant onto the rotation type (`Rotation`/`UnitQuaternion`
with private fields, fallible `new`, explicit `new_normalize`) so `Transform::new` cannot fail on
rotation norm at all, shrinking the error surface. (2) `inv_mul`/`inverse_transform_point` are the
precedent for "invert at most once / apply inverse without materializing". (3) The 180-degree slerp
case must be a defined result or an error, never a panic — check what the crate's slerp does at
antipodal inputs (not verified in this pass).
Do NOT copy: generics over scalar/rotation/dimension (Non-Goals: f64 only), public fields on the
isometry, `new_unchecked` in the public API, `Option`-returning constructors (no diagnosis).

### glam 0.34.3 [V]

- Concrete per-scalar types (`DAffine3` in `glam::f64`, `Affine3A` for f32) — no scalar generics.
- `DAffine3` has public `matrix3`/`translation`; no rigid/isometry type; `inverse`: "if the transform
  is not invertible the result will be invalid"; normalization is checked only "when `glam_assert` is
  enabled" (panic).

Lesson: concrete f64 types are mainstream and vindicate the crate's f64-only stance. Do NOT copy the
"garbage in, garbage out, optional debug panic" contract — it is the exact silent-wrong-answer mode
the crate exists to prevent. Not a candidate dependency either (affine, not rigid).

### Rust tf ports [V/U]

- `transforms` itself is the top hit for a pure-Rust tf. Downstream: **`roslibrust_transforms` 0.26.0
  depends on `transforms ^2.1`** and wraps `Registry` in an async `TransformManager` subscribing to
  `/tf` and `/tf_static` [V, docs.rs]. `transforms_io` layers r2r / ros2-client message handling on
  `transforms` [V at search-result level only].
- `tf2_rs` 0.1.0 wraps the C++ `tf2::BufferCore` via `cxx`, integrates with `rclrs`, API
  `lookup_transform("map", "base_link", LookupTime::Latest)` [search-result level].
- `tf_rosrust` / rosrust `tf` / r2r-native tf: not found in search; existence and API **[U]**.

Lessons: (1) there are real downstream wrappers whose shape is "lock + async wait-for-transform +
message conversion" — consistent with AGENTS.md's ruling that the wait layer lives outside. v3 should
make that wrapper cheap to write: `&self` lookups, `Send + Sync` where the target allows, structured
`NotFoundAt { covered }` so a waiter can decide to wait vs fail without string parsing. (2)
`LookupTime::Latest` as an enum is the typed alternative to tf2's time-0 sentinel; the crate's
`latest_common_time` + explicit lookup achieves the same without a second code path and exposes the
TOCTOU honestly. A `Latest` variant would be API growth needing its own argument. (3) A v3 break has a
known blast radius: at least two published dependents.

### uom 0.38.0 [V]

- Dimension-typed quantities via typenum; "zero runtime cost over using the raw storage type" for
  floats; no_std by disabling `std`.
Lesson: do NOT adopt. A rigid-body transform has one length unit and one angle representation
(quaternion); the unit hazard is at the boundary (metres vs millimetres from a driver), which a doc
sentence handles. uom would add a dependency (maintainer approval required), typenum compile cost,
and nothing the core computes would be safer.

---

## (c) Drake, GTSAM, Sophus

- **Drake monogram notation [V]** (drake.mit.edu doxygen): `X_AB` = pose of frame B in frame A;
  `p_BP = X_BA * p_AP`; `X_WC = X_WA * X_AB * X_BC` with "correctness assured by pairwise matching of
  frame symbols"; `X_FG = (X_GF)^-1`; the notation "provides a means to avoid many common bugs because
  verification can be achieved by rote pattern matching of symbols". Enforcement is **naming only**;
  `RigidTransform` carries no frame labels (doc page does not claim any; runtime enforcement absent
  **[U]** for the class itself).
- **GTSAM `Pose3` [V]** (`gtsam/geometry/Pose3.h`): "(R,t)"; `transformFrom` = pose→world,
  `transformTo` = world→pose; frame names (`wTa`, `aTb`) exist only in comments. No enforcement.
- **Sophus `SE3` [V]** (`sophus/se3.hpp`): unit quaternion + translation; SO3 class invariant keeps the
  quaternion normalized ("typically no need to call [normalize] directly"); `setQuaternion`
  normalizes; `SOPHUS_ENSURE` runtime checks on matrix constructors; `bar_T_foo` naming in comments;
  templated on scalar. Whether SO3 renormalizes after every product: **[U]** (so3.hpp not fetched).

Lessons: (1) All three C++ libraries rely on naming discipline; the crate is already stricter —
`Transform` carries `(parent, child)` and composition checks adjacency at runtime. That runtime check
is the crate's differentiator and v3 should keep it on the public `Transform` while keeping the
unlabelled `Isometry` private (as now). (2) Adopt the monogram convention **in docs, tests and
variable names** as normative: `t_a_b` already appears in tests; AGENTS.md could state "`t_a_b` maps
b-coordinates into a; `t_a_b * t_b_c = t_a_c`". Costs nothing, zero API. (3) `transformFrom` /
`transformTo` (GTSAM) and sguaba's "counter-intuitively" show that *method names for direction* are
where users err; v3 should pick one verb whose direction is fixed by the stored labels (e.g.
applying a `Transform(parent, child)` to a child-frame value yields a parent-frame value, and reject
anything else via `Localized` frame check) rather than offering paired from/to methods.
Do NOT copy: scalar templates, invariant-by-comment, renormalize-on-every-op (hides provenance bugs
and breaks the crate's "stored transform returned bit for bit" exactness), matrix constructors.

---

## (d) Errors and time types

### Errors

- **`std::io::ErrorKind` [V]**: `#[non_exhaustive]`, `Copy + Eq + Hash + Ord`; "This list is intended
  to grow over time and it is not recommended to exhaustively match against it"; "use `_` to match
  'all other errors'".
- **thiserror 2.0.21 [V]**: "Thiserror deliberately does not appear in your public API"; switching to
  handwritten impls is non-breaking. no_std support in 2.x via `default-features = false` and
  `core::error::Error` (stable since Rust 1.81) **[U]** — page did not state it; the crate already
  builds this way on bare-metal per its gate, which is indirect confirmation.
- snafu: not fetched; context-selector style **[U]**.

Lessons: (1) The crate's flat, `#[non_exhaustive]`, payload-carrying `RegistryError<T>` is already
closer to best practice than the `io::Error { kind, payload }` split, which exists because io errors
wrap OS codes and boxed customs — a problem this crate does not have. Do NOT copy the kind/payload
split wholesale. (2) What `ErrorKind` does offer: a `Copy + Eq + Hash` discriminant usable for
metrics/retry tables without cloning `String` payloads. If v3 errors keep owned frame names, a
`fn kind(&self) -> ErrorKind`-style accessor is the conventional answer — but it is API growth and
needs a demonstrated need. A cheaper alternative with the same effect: make payloads cheap (`FrameId`
or interned name handle instead of `String`), so the error itself is `Copy`/small. (3) Error
generic over `T` (time type) is unusual among mature crates, but justified here (payload in caller's
time type). Check `core::error::Error` is implemented in no_std mode (MSRV 1.85 > 1.81, so it can
be). (4) tf2's distinction Backward / Forward / NoData extrapolation maps onto
`NotFoundAt { requested, covered: Option<(T,T)> }` — the structured payload subsumes all three and a
caller can derive "wait vs give up" from `requested > covered.1`. Do not add three variants.

### Time types

- **`core::time::Duration`**: u64 s + u32 ns, unsigned, `checked_*` ops, `as_secs_f64` (from memory,
  stable std API) **[U but well-known]**.
- **embassy-time 0.5.1 `Instant` [V]**: u64 ticks since boot; `Copy + Ord`; `MIN`/`MAX`;
  `duration_since` panics, `checked_duration_since -> Option<Duration>`,
  `saturating_duration_since`, `checked_add/checked_sub(Duration) -> Option<Instant>`. Tick rate is
  a compile-time feature **[U]**. `Hash` not listed.
- **fugit 0.6.0 [V]** (docs.rs latest; newer than I remembered, re-check before citing): wrapping
  instants have wrap-aware comparison that is "not transitive, so there is no PartialOrd or Ord";
  monotonic-kind instants are `Ord` by plain integer compare, with "Nothing checks the claim that a
  monotonic timeline does not wrap."
- **hifitime 4.3.2 `Epoch` [V]**: `Duration` (i16 centuries + u64 ns) + `TimeScale`;
  `Copy + Ord + Hash`; how `Ord` treats differing time scales not determined **[U]**; several
  panicking constructors with `maybe_*` fallible twins.

Lessons: (1) The `TimePoint` surface in AGENTS.md (`Copy + Ord + Debug`, `duration_since`,
`checked_sub`, `as_seconds_lossy`) is implementable for embassy `Instant` (u64, Ord, checked ops),
hifitime `Epoch`, `SystemTime`, and fugit *monotonic* instants. It is **not** implementable for fugit
wrapping instants (no `Ord`) — and must not be: a `BTreeMap` keyed by a non-transitive order is
undefined behaviour of the logical kind. v3 docs should state "`Ord` must be a total order consistent
with elapsed time; wrapping tick counters do not qualify" as a trait precondition. (2) The crate must
not ship impls for these crates (dependency rule); the test of the abstraction is that a downstream
impl is ~15 lines. A doc example implementing `TimePoint` for a u64-tick newtype is the cheap proof.
(3) Interpolation needs a ratio `(t - t0) / (t1 - t0)`: with `duration_since -> core::time::Duration`
the ratio is computable exactly enough in f64 from two `Duration`s; a trait that returned raw ticks
would be tighter for embedded but leaks representation. Keep `Duration`. (4) Naming precedent is
uniform: `checked_duration_since`, `checked_add`, `checked_sub`. If `TimePoint::duration_since` is
fallible, the ecosystem name for that is `checked_duration_since`; a v3 rename is cheap and aligns
with both std and embassy. (5) Do NOT copy: panicking `duration_since`, `saturating_*` (a saturated
age is a silent wrong answer), time scales (out of scope), `now()` in the trait.

---

## (e) Interning and generational indices without atomics

- **`alloc::sync` is gated [V]** (rust-lang/rust `library/alloc/src/lib.rs`):
  `#[cfg(all(not(no_rc), not(no_sync), target_has_atomic = "ptr"))] pub mod sync;`. `rc` has no atomic
  gate. thumbv6m-none-eabi is Tier 2, "core and alloc", soft-float only [V]; that it lacks CAS (hence
  `target_has_atomic = "ptr"` false, hence no `Arc`) is stated in AGENTS.md and consistent with the
  cfg, but the platform page fetched did not say it **[U for the page; consistent]**.
  Consequence: `Arc<str>` frame names are unavailable on RP2040 without `portable-atomic` (+ its
  critical-section/unsafe-assume-single-core feature) — a new dependency. `Rc<str>` works everywhere
  but makes `Registry: !Send`.
- **slotmap 1.1.1 [V]**: no_std with `alloc`; slots are `(value, version)`; "After deletion a key
  stays invalid"; max 2^32 - 2 elements; version wraps after many reuses; uses "carefully checked
  unsafe code"; `new_key_type!` gives distinct key types; `HopSlotMap` deprecated. Key layout u32
  index + u32 version **[U]**.
- **string-interner 0.20.0 [V]**: depends on `hashbrown ^0.16`; default hasher foldhash
  `RandomState`, which "relies on a one-time random initialization of shared global state" (suspect
  on targets without atomics — unverified); backends Bucket/String/Buffer; symbols are not tagged with
  their interner (foreign-symbol misuse is unchecked except partially in `BufferBackend`). no_std
  status not stated on the page **[U]**.
- lasso: not fetched; its `ThreadedRodeo` needs atomics, `Rodeo` does not **[U]**.

Lessons:
1. **Hand-roll, do not depend.** The needed interner is ~40 lines on what the crate already has:
   `hashbrown::HashMap<Box<str>, FrameId>` + `Vec<Slot>` where `Slot` owns the name, parent
   `Option<FrameId>`, kind, and buffer; `FrameId(NonZeroU32)` or `u32`. This is tf2's layout minus the
   sentinel. No new dependency, no unsafe, no atomics; works on thumbv6m. slotmap is disqualified by
   the dependency rule and adds nothing a `Vec<Option<Slot>>` + generation counter does not.
2. **Parent as `FrameId` in the slot** turns the chain walk into `Vec` index hops — no hashing, no
   string compare, no allocation — and makes cycle check at insert the same cheap walk.
3. **Removal semantics decide the design.** Options: (a) tombstone slots, never reuse IDs (tf2's
   implicit choice; unbounded growth only under frame churn, which is rare in robotics but real for
   e.g. per-detection frames); (b) free list + generation (slotmap model) so a stale handle fails
   loudly instead of aliasing a new frame. If `FrameId` stays **crate-private**, (a) or plain reuse is
   safe because no handle outlives a `&mut self` call. Generations are only needed if handles are
   exported.
4. **Whether to export `FrameId` is the real v3 question**, not how to intern. Exporting a handle
   (`registry.frame("lidar") -> Result<FrameId>`; `get_transform(id, id, t)`) removes two hash lookups
   per query and all `String` clones in errors, but: it is a primitive (AGENTS.md "Scope": exposing
   primitives is a larger commitment than the query), handles from registry A used on registry B are
   a silent-wrong-answer generator unless each registry carries a brand/ID checked at use (a
   non-atomic counter cannot mint globally unique registry IDs without a `static` atomic — on thumbv6m
   that means no CAS; an `AtomicU32` load/store is available but `fetch_add` is not), and removal
   requires generations. sguaba/slotmap/string-interner all leave cross-container misuse unchecked or
   solve it with distinct types (`new_key_type!`), which does not distinguish two instances of the
   same type. Recommendation to the brainstorm: intern internally first (pure win, zero API), measure,
   and treat exported handles as a separate proposal that must answer the cross-registry and
   stale-handle questions with an error, not documentation.
5. **Errors can borrow or carry names cheaply**: with internal interning, error payloads still need
   owned names (`String`/`Box<str>`) to be `'static`; that allocation happens only on the error path,
   which is acceptable. Do not put `FrameId` in public errors unless it is public API.

---

## Cross-cutting: what the prior art says v3 should and should not be

Supported by prior art:
- Internal dense frame IDs + `Vec` slots with parent links (tf2), no public change required.
- Unit-norm invariant carried by the rotation type with private fields (nalgebra `Unit`, Sophus SO3).
- Monogram naming made normative in docs/tests (Drake).
- One direction verb fixed by stored labels; no from/to method pairs (GTSAM, sguaba confusion).
- `checked_*` naming for fallible time ops (std, embassy); `Ord` totality as a stated `TimePoint`
  precondition (fugit's wrapping instants are the counter-example).
- Flat `#[non_exhaustive]` errors with structured payloads (beats tf2 strings and beats io's
  kind/payload split for this domain).

Not supported / do not copy:
- Compile-time typed frames in the core (sguaba) — different problem, heavy deps, `unsafe` marker.
- Scalar/dimension generics (nalgebra, Sophus) — Non-Goals.
- Debug-only or absent validation (glam), renormalize-everything (Sophus), `Option` constructors.
- In-band sentinels (tf2 time 0, ID 0), silent overwrite/kind change, per-sample parents.
- New dependencies for interning/slot maps/units (slotmap, string-interner, lasso, uom).
- `Arc<str>` names (no `alloc::sync` on thumbv6m).

Unverified items worth a second look before they enter a design doc: tf2 issue numbers (#181, #388,
#437) and their content; `CompactFrameID` width; whether nalgebra `Unit` deserialization validates;
Sophus SO3 per-op normalization; fugit "0.6.0" version string and wording; thiserror no_std mechanics;
existence/shape of `tf_rosrust`/r2r tf crates; the crate's own slerp behaviour at antipodal rotations.
