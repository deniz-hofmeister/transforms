# v2.0.0 Decision Record & Implementation Plan

**Date:** 2026-08-02 · **Status:** DECIDED, NOT YET EXECUTED · **Base:** `feature/v2-stable-gate` @ a6bbf6c
**Source:** `v2-fitness-audit.md` (full findings register). All sixteen gating decisions below were made
interactively by the maintainer on 2026-08-02. Execution mode chosen: **implement all, staged commits,
autonomous run** — deferred to a later session. To resume: create branch
`feature/v2-audit-decisions` off `feature/v2-stable-gate` and execute the stages in order.

---

## The sixteen decisions

| # | Item | Decision |
|---|---|---|
| D1 | `delete_transforms_before` pin revocation | **Keep pins** — stop dropping emptied buffers |
| D2 | Validation boundary | **Full enforcement** — private `translation`/`rotation`, validating constructors, `static_between` → `Result` |
| D3 | `Registry::new()` unbounded default | **Keep, document** prominently |
| D4 | `Stamp: Ord` derive | **Drop** `PartialOrd`/`Ord`, keep `PartialEq`/`Eq` |
| D5 | `pub mod core` + `Buffer` visibility | **Both private** — Buffer `pub(crate)`, `core` module gone, `Registry` at root only |
| D6 | Error taxonomy | **Flat `RegistryError<T>` with payloads typed in `T`**; `TransformError` stays for pure geometry; `BufferError` leaves the public surface |
| D7 | `Quaternion::new(w,x,y,z)` | **Rename to `from_wxyz`**, delete `new` |
| D8 | `delete_` vs `remove_` | **Unify on `remove_`** (`remove_transforms_before`; internal `Buffer::remove_before`) |
| D9 | f64 scalar | **Commit to f64**: Non-Goals entry + published supported-envelope table |
| D10 | slerp std/no_std fork | **libm everywhere**, bit-pinned in both feature modes |
| D11 | `TimePoint` shape | **Slim + Debug**: drop `checked_add` + `as_seconds`; require `duration_since`, `checked_sub`, `as_seconds_lossy`; supertraits `Copy + Ord + Debug` |
| D12 | `Timestamp` width | **Narrow to u64** nanoseconds |
| D13 | `Stamp` wire encoding | **Tag explicitly** (`{"At": t}` / `"Static"`); `null` becomes a hard error |
| D14 | `Timestamp` JSON shape | **`#[serde(transparent)]`** — bare integer |
| D15 | `#[non_exhaustive]` | **Structs only** (`Transform`, `Point`); `Stamp` stays exhaustively matchable |
| D16 | Execution | **Implement all** (staged commits, full gate per stage), plus the Part-2 non-breaking fixes |

## Sub-decisions taken by default during planning (flag in review if wrong)

- **`Point`**: gets an infallible `Point::new(position, orientation, timestamp, frame)`; fields stay
  `pub` (it is a data record, not an invariant carrier — its `Transformable` impl now validates the
  *transform*, not the point); `#[non_exhaustive]` added (blocks downstream literals, hence the
  constructor). Point's literal dies alongside Transform's — AGENTS.md style section must change.
- **`Transform` accessor shape**: getters `translation()`, `rotation()`, `timestamp()` (Copy values),
  `parent()`/`child()` → `&str`. No setters initially — rebuild via constructor. A `pub(crate)`
  unchecked constructor for registry/geometry internals (interpolate, inverse, compose) so validated
  data is never revalidated.
- **Serde now validates `Transform`**: with private fields, the `Deserialize` impl must go through
  validation (manual impl or `try_from` shadow struct). This *closes* the documented
  "deserialization does not validate" gap — README serde note flips, MIGRATION notes it. `Point`
  stays raw-data deserialize.
- **`Buffer::insert`'s `validate()` call becomes dead** once `Transform` is valid-by-construction
  (constructor + serde both enforce). Delete it per prefer-deletion, with a test proving invalid
  transforms are unrepresentable; keep nothing "just in case".
- **`Timestamp` inherent methods**: keep both `as_seconds` (checked, 2^53 cliff — still real inside
  u64) and `as_seconds_lossy`, documented; only the *trait* loses `as_seconds`.
- **`Timestamp::now()` under u64**: `as_nanos()` from `SystemTime` is u128 → `try_into` u64 overflows
  in year 2554; route the failure into `try_now`'s error, keep `now()`'s documented panic wording.
- **`RegistryError<T>` sketch** (refine at implementation): insert-side variants
  `NonUnitRotation(f64)`, `NonFiniteValues`, `SelfReferentialFrame`,
  `ReparentingNotSupported { current_parent }`, `CycleDetected`, `StaticDynamicConflict`;
  lookup-side `UnknownFrame(String)`, `Disconnected { target, source }`,
  `NotFoundAt { target, source, frame, requested: T, covered: Option<(T, T)> }` (typed, no
  `Box<BufferError>`); `#[non_exhaustive]`; `Display` via `as_seconds_lossy` (hence `T: TimePoint`
  bound on the impl). `ChildFrameMismatch` is unreachable through `Registry` (buffers keyed by
  child) — internal only.
- **Buffer doctests** (buffer/mod.rs:157, 183, 210) become unit tests when `Buffer` goes private
  (doctests on non-public items don't run). Intra-doc links to fix: `point/mod.rs:133`,
  `traits.rs:11`, test path `registry/tests.rs:1693`.

---

## Stage plan

Branch: `feature/v2-audit-decisions` off `feature/v2-stable-gate`. One commit per stage (imperative
summary + why-body + `Assisted-by: Claude:claude-fable-5`). **Full gate green after every stage**
(`rustup run nightly tests/test_all.sh`). Each stage updates the docs it invalidates in the same
commit (README / MIGRATION.md / CHANGELOG.md / AGENTS.md) — stage 8 is only the final sweep.

### Stage 1 — Correctness semantics (D1, D3, D4) — small, independent
- Delete the `retain` in `delete_transforms_before` (`src/core/registry/mod.rs:493`); pins/kind now
  survive cleanup; `remove_frame` is the sole release.
- Invert `delete_transforms_before_prunes_empty_frames` (`src/core/registry/tests.rs:1098`); add
  regression tests: (a) re-parent still rejected after cleanup, (b) dynamic→static flip still
  rejected after cleanup, (c) drained-frame lookup now diagnoses
  `NotFoundAt { .. NoTransformAvailable }` instead of `UnknownFrame`.
- Fix the now-contradicted rustdoc (`registry/mod.rs:484-485`) and the Buffer pin-doc it now agrees
  with; MIGRATION.md item 5 and CHANGELOG updated.
- Add prominent rustdoc on `Registry::new`/`Default`: unbounded retention AND unbounded
  interpolation gap; point at `with_max_age` (which bounds both — verified in audit).
- Drop `PartialOrd, Ord` from `Stamp`'s derive (`src/time/stamp.rs:32`); fix any fallout (none
  expected — zero internal uses).

### Stage 2 — Time types (D11, D12) — before errors, which need `as_seconds_lossy` in `T`
- `TimePoint`: supertraits `Copy + Ord + core::fmt::Debug`; methods `duration_since`, `checked_sub`,
  `as_seconds_lossy` (now required, documented as the infallible formatter). Delete `checked_add`
  and `as_seconds` from the trait. Update `Timestamp`/`SystemTime` impls, `tests/time_traits.rs`,
  and the adapter example in `src/time/traits.rs` (it currently implements all four).
- `Timestamp { t: u64 }`: `from_nanos(u64)`, `as_nanos() -> u64`; simplify `Sub` (drop the
  `seconds > u64::MAX` branch); keep the 2^53 `as_seconds` cliff logic; `try_now` grows the
  overflow arm (see sub-decisions). Update `src/time/timestamp/tests.rs` extremes to u64.
- Widen `tests/properties.rs` timestamp strategy while here (audit testing finding): arms at
  wall-clock magnitude (~1.7e18), straddling 2^53, and near `u64::MAX` (this also discharges the
  "proptest caps at 1e15" gap).

### Stage 3 — Numerics (D10)
- Delete the cfg fork in the `math` module (`src/geometry/quaternion/mod.rs:14-50`); call
  `libm::sqrt/sin/acos` unconditionally (libm is already a non-optional dependency).
- New tests pinning exact slerp bit patterns (interior + near-antipodal + near-identity cases), run
  under both `cargo test` and `--no-default-features`.
- AGENTS.md additivity clause: state float math is libm in all modes (removes the undisclosed
  fourth feature fork).

### Stage 4 — Surface trims + renames (D5, D7, D8)
- `Buffer` → `pub(crate)`; delete the public `core` module (private `mod`, `pub use ...::Registry`
  at the crate root). Convert Buffer's three doctests to unit tests; fix the intra-doc links and
  test path listed in sub-decisions; `assert_send_sync` keeps covering Registry.
- `Quaternion::new` → `Quaternion::from_wxyz` (~75 call sites); update AGENTS.md construction line,
  README:164/218, MIGRATION, CHANGELOG.
- `delete_transforms_before` → `remove_transforms_before`; `Buffer::delete_before` →
  `remove_before` (internal). README API table + MIGRATION in the same commit.

### Stage 5 — Full enforcement (D2, D15) + the double-inversion rework
- `Transform`: private `translation`/`rotation`/`timestamp`/`parent`/`child`; public API =
  `Transform::new(parent, child, translation, rotation, stamp) -> Result<Self, TransformError>`
  (validates), `static_between(..) -> Result<Self, TransformError>`, getters per sub-decisions,
  `#[non_exhaustive]`. `pub(crate)` unchecked constructor for internals. Delete `identity()` (its
  blank/unusable semantics were a separate audit finding; registry synthesizes its own identity).
- `Point`: `Point::new` (infallible), fields stay `pub`, `#[non_exhaustive]`.
- Serde: manual/try-from `Deserialize` for `Transform` that validates; drop the
  `deserialize_with` missing-field detour only in Stage 7 (it interacts with D13).
- `Point::transform` (the crate's `Transformable` impl) relies on transforms being
  valid-by-construction — document that as the trait's stated precondition for third-party
  transforms obtained elsewhere.
- Registry internals: rework `combine_transforms` to compose each chain in natural order and invert
  at most once (**deletes `reverse_and_invert_transforms`** — audit-confirmed 2× win in the
  documented direction); delete `Buffer::insert`'s now-dead `validate()`; delete the two
  unreachable defensive backstops (`remaining` counter, `visited` set) per the testing finding, or
  keep with a written reason.
- Regression tests: norm-1.01 rotation unrepresentable (constructor + serde both reject);
  single-hop exact-timestamp lookup returns the stored transform bit-identically (new fast path);
  `inverse()` translation-finiteness (NaN translation → `Err(NonFiniteValues)`) — do **not** add
  full `validate()` inside `inverse()` (audit-measured composition-drift regression).
- Rewrite every example, doctest, README snippet, and the benches (`Transform::identity()`
  field-poking sites) to the constructors. AGENTS.md: replace the named-field-literal style rule
  with the constructor story. This is the largest stage; expect most of the diff here.

### Stage 6 — Error overhaul (D6)
- Introduce `RegistryError<T>` per the sketch; `add_transform` and all lookups return it.
  `TransformError` shrinks to pure-geometry variants (no `BufferError` arm, no `NotFoundAt`);
  `BufferError` deleted from `errors::*`. Display strings keep the C-GOOD-ERR conventions.
- `tests/tests.rs:119-143` (the 25-line triple-nested match) collapses — keep it as the
  demonstration test of the new one-level match.
- MIGRATION.md gets a dedicated error-overhaul section with before/after.

### Stage 7 — Wire format (D13, D14)
- `Stamp`: delete the custom Option-shaped `Serialize`/`Deserialize`; derive both (externally
  tagged: `{"At": t}` / `"Static"`). `null` and missing field both now hard-error via the derive —
  delete the `deserialize_with` detour on `Transform.timestamp` (net-negative).
- `Timestamp`: `#[serde(transparent)]` (well — with validation-Deserialize from Stage 5, keep the
  transparent *shape*: bare integer).
- `tests/serde.rs`: JSON goldens updated (tagged stamp, bare-int timestamp); postcard golden bytes
  expected **unchanged** (tagged enum ≡ Option, u64 varint ≡ u128 varint for in-range values —
  verify, and treat any diff as a stop-and-look); add negative wire tests (null stamp rejected,
  NaN rotation rejected at deserialize now, norm-2 rotation rejected).
- README serde note + MIGRATION wire section rewritten (wire breaks once, in this stage).

### Stage 8 — Docs true-up (D9 + audit Part-2 doc items)
- Non-Goals: add "f32 / mixed-precision arithmetic" to README + lib.rs lists (keep them identical).
- README performance section: supported-envelope table (platform × rate × hops × memory) from the
  audit's measured numbers; state ~330 B/sample resident (remeasure after Stage 5's changes — the
  Sample split was NOT part of this program, so expect similar) next to the heap-sizing formula in
  lib.rs; restate the allocation profile post-double-inversion-fix; reword embedded positioning.
- Fix stale 1.x rustdoc: `src/lib.rs:165,167` and `src/core/registry/mod.rs:22-26` (the audit
  verified these four lines are the complete set).
- MIGRATION.md: add the module-privatization bullet (rule + "rustc's E0603 suggestion is correct
  here"); fix the bullet attributing 2.0-beta constructor names to 1.x.
- `Quaternion::normalize` rustdoc: fix the threshold claim (actual: norm < `f64::EPSILON`, not
  ~1e-8) + boundary doctest.
- CHANGELOG: fix the phantom "corrected in beta.5" cross-reference; entries for every stage.
- README concurrency snippet: make it compile (doc-tested or example-backed), use `RwLock` to match
  the `&self` design; fix `examples/std_full.rs` to stop future-stamping (+1 s) — query at covered
  stamps.

### Stage 9 — Test & bench hardening (audit Part-2 items not already absorbed)
- Mutation-gap tests: denormalized-inverse regression (norm-2 → now unrepresentable, so the test
  becomes "constructor rejects" + inverse NaN-translation check); `truncate_at_common_parent`
  asserts post-truncation chain lengths (plus the deep-trunk case); `UNIT_NORM_TOLERANCE`
  exact-boundary tests keyed off the constant; slerp switchover + equal-stamp `interpolate` pinned;
  `as_seconds_lossy` NaN-fallback test via an always-failing custom `TimePoint`.
- Benches: rotate samples about a non-axis-aligned axis so slerp's trig path is measured; add
  both-directions 4-hop bench, timestamp-past-newest failure bench, insert+evict steady state.
- Gate: add the two serde clippy combos to `tests/test_all.sh`; add a CI job that runs
  `tests/test_all.sh` so the script is the single source of truth; update the AGENTS.md superset
  claim once parity holds.
- **External golden vectors** (audit's deepest gap): 3–5 hard-coded expected poses derived from an
  outside source (scipy `Rotation` / tf2 worked example) — 90° yaw + offset applied to a known
  point, one two-hop chain — asserted against literal numbers computed *outside* the crate.

### Final
- Full gate; `cargo semver-checks check-release --baseline-rev v1.4.1` once (prove the tooling);
  bump to `2.0.0-rc.2`; MIGRATION.md read end-to-end as a 1.4.1 user; then the maintainer's release
  checklist in AGENTS.md.

## Explicitly deferred (post-2.0, additive — NOT in this program)
`get_latest` + coverage/frames introspection, `Transform::apply(Vector3)`,
`Quaternion::from_axis_angle`, Sample-split/ring-buffer storage rework (now unlocked by D5),
allocation-free lookups, QEMU semihosting smoke test, stateful proptest with independent oracle +
cargo-fuzz. See `v2-fitness-audit.md` Part 4 for the ranked list and rationale.
