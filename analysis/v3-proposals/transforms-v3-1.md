# transforms v3 — design notes (updated for 2.1.3)

Reviewed against `v2.1.3` (2026-09-19) and `analysis/v2-feasibility/REPORT.md`.
Opinions, not decisions. Items retracted or revised since the 2.1.2 review
are marked.

## What 2.1.3 settled

- **Done, measured:** per-sample names removed (`BTreeMap<T, Sample>`, names
  pinned once per buffer), walk borrows pinned names. Host: −52% heap at
  8-char names, −75% at 128-char; 1-hop lookup 5→3 allocations, 4-hop 11→9.
  No public signature, serde byte, or numeric change; semver-checks and a
  278k-line differential trace agree.
- **Docs cut** (README −845 lines, MIGRATION −808). The unmeasured MCU
  envelope table is gone; CI is now described as proving compilation only.
- **Four of the earlier proposals were prototyped.** Findings below.

## Revised positions

- **`VecDeque` history — retracted.** At 1,000 samples: ordered insert equal
  to `BTreeMap`, out-of-order 1.7× slower (398 vs 240 ns), lookups unchanged
  (~180 ns). At 60,000 samples out-of-order it is 42× slower. No lookup win
  anywhere, because allocations were the name `String`s, not map nodes. Not
  worth the code.
- **`inverse()` normalization — retracted.** Accepted rotations sit within
  `UNIT_NORM_TOLERANCE` of unit; the regression test shows the conjugate is
  not the inverse there. Keep it.
- **Topology-first resolver — strengthened for v3.** Rejected for 2.x only
  because it changes error precedence in two cases. Both cases show v2's
  precedence is wrong by the crate's own philosophy: (1) two disconnected
  trees, both unavailable at t → v2 reports `NotFoundAt`, which tells the
  caller waiting may help when it never will; `Disconnected` is the truth.
  (2) v2 can name a failing edge *above* the common ancestor — a frame the
  connecting chain never crosses. `latest_common_time` already decides
  topology first; the two entry points disagree today. v3: one resolver,
  precedence `UnknownFrame > Disconnected > NotFoundAt`, diagnosis restricted
  to the chain actually crossed.
- **Scalar parameter — E0283 kills the shortcut, not the feature.** A
  defaulted `S = f64` does not infer `Vector3::zero()` in expression position
  (same limitation already documented for `Registry::new()`). The known
  answer is glam's: concrete per-scalar types (`Vec3`/`DVec3`), macro-stamped,
  no type parameter on geometry; the registry is generic over a sealed
  geometry-family trait. Cost is a duplicated API surface and test set, not
  inference.
- **Fixed-frame static-ancestry check — downgraded to opt-in.** The witness
  accepts a dynamic edge that happens to be constant; a static-only rule
  rejects that legitimate use.
- **Cached depth — dropped.** A root can later acquire a parent, so depth is
  not immutable. An LCA fold does not need it: two allocation-free walks to
  the root give both depths.

## Next 2.x step (non-breaking, not yet taken)

- **Fold samples along the walk, materialize once.** `Buffer::get` still
  clones both names per hop (`Transform::unvalidated(parent.clone(),
  child.clone(), …)`), and the static slot clones a full `Transform`. Walk
  with `(&str, &str, Sample)`, compose on samples (timestamps are trivially
  equal inside the registry), build one `Transform` at the end. Expected:
  2 allocations per lookup regardless of hops (4-hop 9→2, 64-hop 133→2).
  Walk order and error precedence unchanged, so the differential trace
  should still match.
- **Claim cross-device replay, or don't — but test it.** README 2.1.3
  retracted "bit for bit". For a fixed scalar type, IEEE basic ops are
  correctly rounded in hardware and soft-float alike, rustc does not contract
  to FMA, and `libm` is deterministic, so the claim should hold. ARM64 CI
  and the differential trace already exist: compare trace hashes across
  x86-64 and ARM64 runners and the claim is either verified or falsified.
- **Hardware measurement — still not done.** The README now says CI proves
  compilation only. That under-claims relative to "full embedded parity".
  Only a DWT-cycle-counter run on an M4F and an M7 decides whether f64 is a
  default or a blocker (rule below).

## v3 breaking changes

1. **Separate geometry from metadata.** `Isometry` (R, t; infallible `Mul`,
   `inverse`, `interpolate`). Insert `Sample { parent, child, at, iso }` or
   `add_static(…)`; lookup returns `Resolved { target, source, at, iso }`;
   time-travel returns a distinct two-instant type that cannot be inserted.
   The compatibility witness confirms the current cross-time result carries a
   false stamp and is insertable. Keep frame-checked composition as a method.
2. **Frame identity.** `&str` at the boundary, `FrameId(u32)` interned
   internally (conservative), or public `FrameId` (zero-alloc, only if an
   embedded user asks). `Arc<str>` does not build on `thumbv6m`.
3. **Un-swappable lookup.** `Lookup { target, source, at }` or a builder.
4. **Bounded by default.** `max_age` in the constructor; `unbounded()` opt-in.
5. **One resolver, topology-first precedence** (see above).
6. **`remove_frame` with children:** reject or cascade, not strand.
7. **Cleanups:** `TransformError<T>` instead of `f64` seconds;
   `checked_add`/`checked_sub` instead of `Result`-returning operators;
   seal or validate `Point`.

The report's disposition table classifies every one of these as breaking.
That is the point: they are v3 items, batched together.

## f64 vs embedded parity

Non-goal reads "will not be considered". It conflicts with full parity on
FPv4-SP / FPv5-SP cores.

| CI target | Core | f64 | f32 |
|---|---|---|---|
| `thumbv7em` | M4F | software | hardware |
| `thumbv7em` | M7 (F7/H7) | hardware | hardware |
| `thumbv8m.main` | M33 | software | hardware |
| `thumbv6m` | M0+ | software | software (~2–3× cheaper, est.) |
| `riscv32imc` | ESP32-C3/C6 | software | software |

One of five core classes runs f64 in hardware. Rough 4-hop dynamic lookup:
~550 basic ops + ~16 transcendentals; soft-f64 on M4F ≈ 0.5 ms (±3×),
hardware f32 ≈ 50 µs. f32 rescues CPU, not memory (payload 56→28 B; the
memory win was name dedup, now shipped). Precision: f32 is fine for local
trees (~12 µm at 100 m), not geo-referenced frames (~6 cm at UTM scale).
Tolerances must be per-scalar.

Coherent positions: (1) state the narrowed claim explicitly — "M7-class, or
≤100 Hz on FPv4/FPv5-SP" — or (2) add f32 the glam way. Rejected: a cargo
feature (non-additive); mixed precision.

**Decision rule:** measure first. M4F 4-hop under ~300 µs → f64 holds,
publish measured numbers; over ~1 ms → the non-goal blocks the listed
targets, add f32 in v3 alongside the other type changes.

## Keep

`latest_common_time` (no `get_latest`); validation at construction and
insert; flat `RegistryError`; strict single-parent tree; no extrapolation;
`libm` everywhere; the differential trace and golden vectors as the
acceptance gate for any internal rewrite — 2.1.3 showed that gate works.
