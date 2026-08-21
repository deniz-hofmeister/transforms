# transforms v2.0.0-rc.1 — Fitness-for-Purpose Audit

**Date:** 2026-08-02 · **Tree:** `feature/v2-stable-gate` @ a6bbf6c · **Method:** 8 parallel deep audits
(correctness, ergonomics, performance, style, documentation, concurrency, API design, testing), each
finding anchored to file:line with empirical reproduction where possible; the 24 highest-severity
findings independently re-derived by adversarial verifiers instructed to refute them; a completeness
critic over the assembled results. ~2.6M tokens of analysis, 33 agents. Severities below are
**post-verification** — every verified finding was adjusted, mostly downward, after the verifiers
re-ran reproductions and checked claims against git history and the AGENTS.md charter.

---

## Verdict

**Fit for purpose for its primary deployment class — Linux-class robots (SBC/Jetson/desktop) at
realistic rates — with a genuinely correct mathematical core, and the cleanest error/documentation
discipline the panel has seen in a crate this size. Not yet fit for the high-rate MCU story the
charter and README currently advertise, and carrying roughly a dozen one-way-door decisions that
should be made consciously before the 2.0 tag, because they are free today and breaking forever
after.**

No dimension graded "unfit"; all eight graded "fit-with-reservations". Zero fabricated-pose defects
were found in the registry pipeline (insert → validate → chain-resolve → interpolate → return): the
panel attacked slerp double-cover, equal-timestamp division, chain junction logic, partial-chain
returns, static/dynamic mixing, and eviction underflow — and all held, several verified empirically
(e.g. a 200-hop chain returning zero rotation-norm deviation).

The two structural reservations:

1. **The validation boundary is narrower than the marketing.** Everything outside the registry —
   `Transform { .. }` literal → `Transformable::transform` / `Transform * Transform` /
   `static_between` → apply — is deliberately unvalidated, and that is where users hand-build sensor
   mounts. A norm-1.01 quaternion silently scales poses by 2% through `Ok(())`-returning APIs. This
   is documented and deliberate (AGENTS.md concedes it), but it is a per-call documented promise
   where the charter's own priority #1 demands boundary enforcement.
2. **The MCU claim is not currently supported by the numbers.** Measured 327 B of resident heap per
   stored sample and 11–23 heap allocations per lookup, plus f64 math that is software-emulated on
   *all four* named MCU targets (M4F/M33 FPUs are single-precision). ≥1 kHz multi-hop dynamic use
   fails on RAM before CPU on every named board; ≤100 Hz with mostly-static trees is genuinely fine.
   The gap is between the numbers and the advertising, and can be closed from either side.

---

## Scorecard

| Dimension | Grade | One-line summary |
|---|---|---|
| Correctness | fit-with-reservations | Registry pipeline verified sound under adversarial reading; unvalidated geometry layer + `delete_transforms_before` pin-revocation are the residue |
| Ergonomics | fit-with-reservations | Direction contract documented consistently everywhere; missing latest-lookup/introspection is the real hole |
| Performance | fit-with-reservations | Algorithmic claims hold exactly; memory (327 B/sample) and the 2× documented-direction inversion overhead are the findings |
| Style | fit-with-reservations | Machine-checked half is spotless; naming seams (`delete`/`remove`, `pub mod core`) freeze at 2.0 |
| Documentation | fit-with-reservations | Unusually accurate overall; docs.rs rustdoc carries stale 1.x prose the README already fixed |
| Concurrency | fit-with-reservations | `&self` reads + zero interior mutability verified; serde `null`→`Static` is a wire-freeze decision |
| API design | fit-with-reservations | Small, mostly well-considered surface; ~8 one-way doors need explicit decisions before the tag |
| Testing | fit-with-reservations | Example-based suite unusually disciplined (verified by mutation); property layer is a facade — 9/19 targeted mutations survived |

---

## Part 1 — The release-gating list

Per the completeness critic's key insight: **severity does not decide what blocks the tag —
"is the fix additive after 2.0?" does.** These items are breaking or behavior-changing and are
therefore now-or-never. Each needs a conscious maintainer decision, even if the decision is "keep
as-is".

### 1.1 `delete_transforms_before` silently revokes the parent pin and the static/dynamic kind
`src/core/registry/mod.rs:493` · **major, CONFIRMED empirically by finder and verifier** · behavior-changing

Dropping emptied buffers un-pins parent and kind, so the next insert re-derives both. Reproduced:
`ReparentingNotSupported` → routine cleanup → same insert returns `Ok` and topology changed
silently; `StaticDynamicConflict` → cleanup → a moving frame becomes an eternal static one serving
confidently wrong historical poses (`Ok` at t=999 ns for a frame whose data started at t=1 s).
The pruning is deliberate and test-locked (commit 15fb526, `delete_transforms_before_prunes_empty_frames`),
but its safety consequence is documented nowhere, and `Buffer`'s own rustdoc promises the opposite
("drop the whole buffer (`Registry::remove_frame`) to release it"). **Fix is a one-line deletion**
(stop the `retain`) plus inverting one test; flips some `Ok` inserts to `Err`, so it is free now and
a semver break later. This is the panel's clearest "fix before tag" item.

### 1.2 The validation-boundary decision
`src/geometry/point/mod.rs:124`, `src/geometry/transform/mod.rs:385` · **major** (deduped from 2 findings incl. api-design's critical) · Ok→Err behavior change

`Point::transform` and the public `Mul` reject frame/timestamp errors loudly but apply geometric
garbage silently (norm-1.01 rotation → 2% scale error → `Ok(())`; NaN translation → `Ok(())`).
`static_between` — the crate's only real constructor and the README's recommended mount-builder —
accepts a norm-2 quaternion without complaint. Verifier corrections that matter: the blessed
insert→lookup→apply pipeline is *not* affected (slerp and composition preserve validated norms);
the README's "lookups either answer or error" claim is scoped to lookups and is true; and the
minimum fix is *not* a private-fields rewrite (which would contradict the charter's explicit
named-field-literal decision). **Minimum decisive fix:** `transform.validate()?` at the top of the
`Point::transform` impl and of the public `Mul` (not `compose_ignoring_time` — the registry path is
already validated at insert), document it as the `Transformable` trait contract, regression tests
for both. Both already return `Result` with existing variants; the Ok→Err flip is why it belongs at
2.0. Do **not** add `validate()` to `inverse()` — verifier measured that norm tolerances multiply
across composition, so validating the composed chain inside `inverse()` would reject legitimate
lookups on accepted data. The unambiguous `inverse()` fix is a translation-finiteness check only.

### 1.3 `Stamp` derives `Ord`: `Static` sorts below every real instant
`src/time/stamp.rs:32` · **major, self-verified** (derive confirmed; zero internal uses) · removing a derive is breaking

`max_by_key(|tf| tf.timestamp)` — the natural "freshest transform" idiom — silently ranks an eternal
transform as *older than the epoch*. The ordering has zero uses inside the crate. Drop
`PartialOrd, Ord` from the derive before they freeze.

### 1.4 Public-surface trims (all deletions, all breaking, all free today)
- **`pub mod core`** (`src/lib.rs:240`): shadows `::core` in the crate root (reproduced: a
  `use core::time::Duration;` in lib.rs fails with a misleading E0432), duplicates the `Registry`
  path, and is the sole path to `Buffer`. Verifier scoped the shadowing to lib.rs's six declaration
  lines (downstream users unaffected) — the durable reason is the duplicate frozen path. Flatten to
  the crate root.
- **`Buffer` is public with zero demonstrated consumers**: appears in no test, example, bench, or
  README snippet (only `BufferError` is imported anywhere). Publishing it freezes the BTreeMap
  storage model and forecloses the fixed-capacity-ring rework the MCU story most plausibly needs,
  and makes the error-enum recursion (below) unfixable. Make it `pub(crate)`; if standalone use is
  real, write the example first — if the example cannot be written, the type does not need to be pub.
- **Error taxonomy**: `BufferError::TransformError(TransformError)` and
  `TransformError::NotFoundAt { source: Box<BufferError> }` are mutually recursive;
  `add_transform` and `get_transform` speak two different error languages; learning "your rotation
  is not unit" from an insert requires matching a validation error through the lookup error type
  through the storage error type. A flat `RegistryError` for the Registry surface (keeping
  `TransformError` for pure geometry) untangles it — multi-version migration if attempted post-2.0.
- **`Quaternion::new(w,x,y,z)`**: introduced *in this release cycle* (absent from v1.4.1 — verifier
  checked), so removing it is nominally breaking with zero stable users. Four unlabeled positional
  floats where a transposed `w` is norm-preserving and invisible to `validate()`. Charter-aligned
  options: delete it (the self-labeling `Quaternion { w, x, y, z }` literal is mis-order-proof and
  matches the `Transform { .. }` precedent) or rename to `from_wxyz`. Do not ship `from_xyzw`
  alongside — two ways to do one thing, justified only by tf2 ordering, which AGENTS.md forbids as
  a rationale.
- **`delete_` vs `remove_`**: `Registry` spells removal two ways (`delete_transforms_before`,
  `remove_frame`); std never uses `delete`. Verifier narrowed the original four-way-inconsistency
  claim to this one real seam (and defended `static_edge`: `static` is a reserved keyword and "edge"
  is accurate). Rename `delete_transforms_before` → `remove_transforms_before` (and
  `Buffer::delete_before` → `remove_before` if Buffer stays public).

### 1.5 `TimePoint` trait shape
`src/time/traits.rs:100` · **major, unverified but CHANGELOG-corroborated** · breaking

`checked_add` has zero call sites (CHANGELOG admits it stays "by decision ... although the crate
itself only calls `checked_sub`" — textbook speculative API by the charter's own standard);
`as_seconds` exists only to feed the default `as_seconds_lossy`, whose own docs tell implementors to
override it. An MCU implementor writes four methods, two for nothing — on a Cortex-M0+ the unused
`as_seconds` is a soft-float divide. Also: no `Debug` supertrait, so a user's `McuTicks(u64)`
without a derive makes `Transform<McuTicks>` silently non-Debug. Slim to
`duration_since` + `checked_sub` + required `as_seconds_lossy`, add `Debug` bound.

### 1.6 The f64 decision
`src/geometry/quaternion/mod.rs:58` · **major** · parameterizing later is breaking

rustc target specs confirmed: `+vfp4d16sp` / `+fp-armv8d16sp` — single-precision FPUs on
thumbv7em/thumbv8m; an M33 staticlib links `__aeabi_dmul/dadd/ddiv` + double-precision libm
`sqrt/sin/acos`. Every f64 op is software on all four named MCU targets; the hardware FPU on the
M4F/M33 boards the charter names is 100% unused. Decide explicitly: (a) parameterize the scalar now
(`Vector3<S = f64>` — costly, one instantiation today, contradicts minimal-and-intentional), or
(b) **the minimal answer**: add f32/mixed-precision to the Non-Goals, state the envelope (below) in
the README, and stop implying flight-controller-rate fitness. What is not acceptable is shipping
2.0 with the door quietly closed while the charter names M4F flight controllers first-class.

### 1.7 Wire-format freeze items (serde)
- **`Stamp` as `Option<T>`**: an out-of-contract producer emitting `"timestamp": null` as a frame's
  *first-ever* sample mints an eternal static edge; lookups then answer every timestamp with the
  stale pose. Verifier demolished the "critical" framing — subsequent inserts fail loudly forever
  (`StaticDynamicConflict`), `remove_frame` recovers, mid-stream nulls are rejected, the JS
  `undefined` vector doesn't exist, and the behavior is documented in four places as deliberate.
  Also: a tagged enum encodes byte-identically to `Option` in postcard, so tagging buys hardening
  *only* for JSON/CBOR and the golden-byte pins would not change. Residue: a hardening decision to
  make before the wire freeze, not a defect.
- **`Timestamp` serializes as `{"t": <u128>}`**: a private one-letter field name frozen as a public
  JSON key. `#[serde(transparent)]` emits the bare integer; postcard pins unchanged. Now-or-never.
- **`TimestampOutOfRange { requested, start, end }` payloads are lossy `f64` seconds**: at 2026
  wall-clock magnitudes an f64 ulp is ~256 ns; measured: the reported `end` overshoots the true
  newest sample by 107 ns, so retrying at the reported bound fails again. The payload *type* freezes
  at 2.0 even though the introspection accessors that fix the underlying need (below) are additive.
  Decide: retype in `T` now, or explicitly declare the payloads Display-only diagnostics.
- **`u128` → `u64` nanoseconds** (minor): u64 spans 584 years; halves stamp storage and 32-bit
  multi-word arithmetic. `from_nanos`/`as_nanos` are public → now-or-never. Weaker case; maintainer
  taste.

### 1.8 Feature-additivity violation: slerp forks numerically on `std`
`src/geometry/quaternion/mod.rs:27` · **major, unverified** (fork confirmed in source; measured
divergence: libm vs system `sin` differ on ~2.7% of inputs at 1 ulp) · behavior-changing at bit level

AGENTS.md's non-negotiable says features are additive with three named gated items; the `math`
module is an undisclosed fourth that forks *behavior*: the same registry, same samples, returns
bit-different rotations under `--no-default-features`. For a crate whose no_std story is "the host
build validates the target build", host-vs-MCU bit non-reproducibility undermines HIL/SIL replay.
Fix is a net-negative diff: call libm unconditionally, pin slerp bit patterns in both feature modes.
(Cost: ~1 ulp of precision vs platform libm on std targets — negligible against UNIT_NORM_TOLERANCE.)

### 1.9 Evolvability defaults (critic)
`#[non_exhaustive]` exists on exactly the four error enums — no struct, and `Stamp` is exhaustively
matchable. After 2.0, adding any field to `Transform`/`Point` or any variant to `Stamp` is a major
bump — and several recommended fixes above want exactly that headroom. Decide per type before the
tag. Also run `cargo semver-checks --baseline-rev v1.4.1` once *now*, purely to prove the tooling
works before it becomes load-bearing post-2.0.

---

## Part 2 — Fix before tag, non-breaking

1. **Lookup-direction double inversion** (`combine_transforms` + `reverse_and_invert_transforms`) —
   **major, CONFIRMED digit-for-digit by verifier.** The documented direction
   (`get_transform("map", "lidar", t)`) does k+1 inversions where 0–1 suffice: 11 vs 6 allocs and
   ~2× time at depth 1; 271 vs 136 allocs at depth 64. The benches measure almost exclusively the
   penalized direction. Commit history (81366b2) shows an optimization added for one direction and
   never mirrored — oversight, not design. Fix: compose each chain naturally, invert at most once;
   *deletes* `reverse_and_invert_transforms` entirely. Purely internal, big MCU win, makes
   single-hop lookups bit-exact returns of stored data.
2. **docs.rs rustdoc still teaches the 1.x API** — `src/lib.rs:165,167` and
   `src/core/registry/mod.rs:22-26` carry verbatim v1.4.1 prose: `Registry::new(...)` with
   arguments, plus the `Registry::new()` ≡ `Registry::<Timestamp>::new()` equivalence this very
   release's CHANGELOG records as retracted (it's false Rust — defaults don't apply in expression
   position). The README was fixed; rustdoc — what docs.rs serves — was not. Verifier confirmed
   these four lines are the complete set.
3. **MIGRATION.md omits the module-path privatization** — every 1.x deep import
   (`transforms::geometry::transform::Transform` etc.) fails E0603 with no bullet in the guide that
   promises "the full list". Mitigations verified: rustc suggests the correct fix inline, and 1.4.1's
   own docs never used deep paths. One bullet, stated as a rule not an inventory.
4. **`examples/std_full.rs` future-stamps transforms** (+1 s) to make its own reader's `now()`
   lookups resolve — teaching a pattern that mis-stamps real sensor data. Fix the example to query
   at covered stamps regardless of any API addition.
5. **Benchmark blind spots**: every rotation in the bench suite is `Quaternion::identity()`, so
   slerp's trig path — the dominant float cost on soft-float targets (measured 8.2× the fast branch)
   — is 0% measured; no bench covers the timestamp-past-newest failure (the steady-state failure
   shape) or compares lookup directions.
6. **Test-suite mutation gaps** (all demonstrated by surviving mutations):
   - Removing `normalize()` from `inverse()` — the sole renormalization point on every lookup — left
     202 tests green. Add the denormalized-inverse regression test.
   - `truncate_at_common_parent` deleted entirely → green (its named test asserts only `is_ok()`);
     a silent ~1000× latency regression class. Assert post-truncation chain lengths.
   - Proptest timestamps cap at 1e15 ns — below the 2^53 `as_seconds` cliff (a historical-bug
     regime) and 1000× below wall clock. Add magnitude arms.
   - `UNIT_NORM_TOLERANCE` can drift from 1e-6 to 1.9e-6 undetected (it is `pub`, hence API).
     Exact-boundary tests keyed off the constant.
   - slerp's lerp/slerp switchover widened from `1-ε` to `0.999` → green; equal-stamp `interpolate`
     returning `to` instead of `from` → green. Pin both.
   - `normalize()`'s rustdoc threshold is wrong by 8 orders of magnitude (documents ~1e-8, actual
     `f64::EPSILON` ≈ 2.2e-16 — text copied from `Div`). Fix doc, pin boundary.
7. **Gate drift**: `tests/test_all.sh` runs 2 clippy feature combos; CI runs 4 (serde added in
   ecedc32) — the AGENTS.md "green local gate implies green CI clippy" claim is already false. Add
   the serde combos to the script; have CI run the script.
8. **Release bookkeeping** (correct for an rc, must land at stable): consolidated `[2.0.0]`
   CHANGELOG section, compare links, README version pins; also a CHANGELOG entry referencing a
   "beta.5" correction that never shipped as described.
9. **Memory constant into the docs**: lib.rs tells no_std users to "size the heap for max_age ×
   insert rate" but never states the coefficient. Measured: **~327 B resident per stored sample**
   (144 B Transform + 160 B BTreeMap entry at ~50% node fill + two frame-name Strings). One
   sentence makes the formula actionable. (The verifier killed the "critical SRAM exhaustion"
   framing — `with_max_age` *does* bound the interpolation gap and the window, `delete_transforms_before`
   exists, and the 19.6 MB figure came from scoring the *std* Quick Start against a 192 KB MCU —
   but the missing constant is real.)

---

## Part 3 — The MCU question, answered with numbers

Measured on x86-64 (release+LTO, counting allocator), estimated for MCU by first-principles cycle
counting from linked soft-float symbols (`__aeabi_d*`, double-precision libm) — **not executed on
target** (see caveats):

| Operation | x86-64 measured | 168 MHz M4F estimate | 133 MHz M0+ estimate |
|---|---|---|---|
| Insert (steady-state, with eviction) | 383 ns, 2 allocs | ~25–30 µs | worse |
| 1-hop lookup (exact) | 767 ns, 11 allocs, 1736 B churn | ~130 µs | — |
| 4-hop lookup (documented direction) | 2364 ns, 23 allocs | ~375 µs | ~1.1–1.5 ms |
| Failed lookup @1000 frames | 5569 ns, 8 allocs | — | — |
| Resident memory | 327 B/sample | same | same |

- **Linux SBC (Pi/Jetson class): comfortably fit.** 1 kHz loop with 6 inserts + 3 four-hop lookups
  ≈ 32 µs ≈ 3.2% CPU.
- **M4F at 1 kHz, 3 dynamic streams + 2 four-hop lookups/tick: ~83% CPU** before the control law
  runs, needing ~1 MB RAM (6 edges × 1 s × 1 kHz × 327 B ≈ 1.9 MB) against 192 KB SRAM. **Fails on
  RAM before CPU.**
- **M0+: one 4-hop lookup > 1 ms — cannot fit a 1 kHz tick at all.**
- **≤100 Hz, mostly-static tree, one or two dynamic edges: genuinely fine** (~8% CPU, ~98 KB on M4F).

So "will this fail for high-rate MCU targets" = **workload-dependent, and the current docs let users
self-select into the failure region**. The performance chain that compounds: f64-everywhere ×
two heap Strings per stored sample × BTreeMap of fat values × allocate-per-lookup × the
double-inversion on the documented direction. Items 1 (Part 2) and the Sample-split below attack
the last three without breaking anything; the first two are Part 1 decisions.

**Recommendation (also the critic's GAP 2/3):** publish a supported-envelope table (platform ×
rate × hops × memory) built from these numbers, and re-word the README's embedded positioning to
match it. That single table converts the "fails on MCU" reservation into a documented, defensible
scope.

---

## Part 4 — Post-2.0 additive roadmap (ranked)

1. **`get_latest(target, source)`** — resolve at the newest instant covered by every dynamic hop.
   The highest-value single addition: without extrapolation, a `now()` query fails on essentially
   every tick against sensor-stamped publishers; the only current escape is the error's lossy-f64
   range (measured non-round-trippable) — or the flagship example's future-stamping trick. The
   crate's own example is the demonstrated caller. (Verifier: one method, not a pair; keep it
   fallible.)
2. **Frame introspection** — `frames()` / `parent_of()`: the documented `remove_frame` subtree
   procedure is currently only executable by users who mirror the tree externally
   (MIGRATION.md says so verbatim), and `view_frames`-style diagnosis is impossible.
3. **`Transform::apply(Vector3) -> Vector3`** — the rigid-body primitive every custom
   `Transformable` impl must currently re-derive; a wrong-order impl passes identity-rotation tests
   and diverges only under real rotation. One line, one canonical definition, reimplement
   `Point::transform` with it.
4. **`Quaternion::from_axis_angle`** — the crate hand-writes the half-angle idiom 11 times across 5
   files and implements axis-angle inline in its own proptest strategy; that is the demonstrated
   concrete need. (`from_rpy` is a separate, weaker decision — convention-choice hazards.)
5. **Buffer internals rework** (needs Buffer private first): store `Sample { translation, rotation,
   stamp }` keyed by `T`, reconstruct `Transform` at the boundary — removes ~48 B + 2 heap blocks
   per sample (~halves resident memory) and both per-hop String clones; consider a fixed-capacity
   ring for dynamic buffers (append-at-end/evict-at-front/binary-search is exactly the access
   pattern), which would give the design-time memory ceiling embedded users need.
6. **Allocation-free lookups**: borrow `&str` during the walk, carry light samples, materialize the
   result once — target 2 allocs per lookup at any depth, then machine-check the README's
   allocation profile with a regression test.
7. **Testing rigor**: stateful proptest with an independent oracle (fold stored links with `*`
   directly — the current registry property is its own oracle); a cargo-fuzz target over operation
   sequences; **3–5 external golden vectors** (scipy/tf2-derived, hard-coded numbers) — the panel
   confirmed the suite contains *zero* externally-derived expected values, so a globally flipped
   convention would pass everything; a QEMU semihosting smoke test with DWT cycle + heap
   high-water measurement (one data point would confirm or destroy the entire embedded analysis).
8. **Smaller items**: out-of-order-expired insert returns `Ok(())` while discarding the sample
   (document or reject); `get_transform_at` could cheaply verify the fixed frame's chain is static;
   same-frame lookups succeed for frames the registry has never seen (documented; consider
   `frame_exists` gate); `Registry`/`Buffer` are not `Clone` (forecloses snapshot patterns);
   `Transformable` has no atomicity-on-`Err` contract; README's concurrency snippet is an
   uncompiled `rust` block recommending a `Mutex` where `&self` + `RwLock` is the design's point;
   no concurrency guidance for no_std targets (`critical-section` pattern).

---

## What is genuinely good (verified, not assumed)

- **The math.** Slerp's antipodal handling structurally eliminates the sin(θ)≈0 blow-up; nlerp
  fallback renormalizes; `dot.clamp` keeps `acos` NaN-free; Hamilton product, inverse, composition
  order, rotate-then-translate all independently re-derived as correct; equal-timestamp
  division-by-zero impossible by construction; no extrapolation anywhere; all time arithmetic
  checked; error formatting infallible by design.
- **The registry chain algebra** held under adversarial reading: two-walk + truncate +
  junction-connectivity check only composes chains that genuinely join; partial chains are caught by
  the exact parent/child guard, never returned.
- **Concurrency architecture**: zero interior mutability, all read paths `&self`, real
  `assert_send_sync` test; externalized locking is the right call for a crate that must run where
  `std::sync` does not exist. Coarse-lock contention at realistic robot loads: ~0.1% write duty —
  a non-problem, honestly assessed.
- **Error/documentation discipline**: Display conventions hold without exception across all four
  enums; 48/48 doctests pass; README Non-Goals is character-identical to lib.rs; the examples table
  matches `examples/` one-for-one; CI genuinely runs everything AGENTS.md claims (ARM64, MSRV,
  audit, all four MCU targets, serde combos).
- **The serde golden-byte pins** (postcard, frozen bytes) are best-in-class for a robotics wire
  format, and the missing-`timestamp`-field hard error shows deliberate wire-safety thinking.
- **Example-based test discipline**, verified *by mutation*: every AGENTS.md correctness invariant
  except the `inverse()`-normalization one has at least one test that fails when it breaks;
  eviction boundaries tested to the nanosecond; scar-comments carry the bugs they pin.

## Caveats on this audit itself (from the completeness critic)

- **No external oracle**: all correctness verification is self-consistency plus code reading. The
  golden-vector gap (Part 4, item 7) applies to this audit too — a globally mirrored convention
  would have passed every check the panel ran.
- **No bare-metal execution**: the MCU cycle/RAM numbers are first-principles estimates from linked
  symbols and x86 measurements; treat them as ±2× until a QEMU/DWT data point exists.
- **Panic-freedom is enforced by lint + review, not machine-proof**: spot-check ran clean for
  `indexing_slicing`/`panic_in_result_fn` (0 hits); 17 `arithmetic_side_effects` hits in `src/` are
  all structurally bounded integer ops, but debug-profile overflow is not gated.
- `api-design` and `testing` findings were not adversarially verified (verification budget was spent
  on the other six dimensions); their claims above are marked where I re-checked them myself.
- Verification systematically *downgraded* severities (4 criticals → 0 after dedup and refutation of
  framing), which should raise confidence in the surviving list: what remains survived an attack.
