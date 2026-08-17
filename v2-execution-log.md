# v2 Execution Log

Companion to `v2-decision-record.md` (the authoritative stage plan). This file is updated by the
executing agent as each stage progresses, and is committed together with each stage. **Git history
on `feature/v2-audit-decisions` plus this table are the ground truth for resuming.**

## Resume protocol (cold start, any session)

1. `git log --oneline feature/v2-stable-gate..HEAD` and this table tell you what is done.
2. If the working tree is **dirty**, a stage was interrupted mid-flight: the row marked
   `IN PROGRESS` says which one. Inspect the diff against that stage's spec in
   `v2-decision-record.md`; either finish the stage, or `git reset --hard` to the last commit
   and redo it from scratch. Never salvage a half-understood diff — redoing a stage is cheap,
   a silently wrong merge is not.
3. Continue with the first row not marked `DONE`. Stages run strictly in order.
4. The full gate (`rustup run nightly tests/test_all.sh`) must be green before every stage commit.
5. Commits: imperative summary, why-body, trailer `Assisted-by: Claude:claude-fable-5`.
   Never push, tag, or publish from an execution session — the maintainer does that.

## Stage table

| Stage | Scope | Status | Commit(s) | Gate | Notes / deviations |
|---|---|---|---|---|---|
| 1 | Correctness semantics (D1, D3, D4) | DONE | a7e7cd3 + follow-up | GREEN | No source drift: the retain, the test, the `Stamp` derive and the contradicted rustdoc were all where the record said. `Buffer`'s pin-doc needed no edit — it already promised what D1 now makes true, so only the registry side was corrected. No CHANGELOG entry for D4: `Stamp` ships first in this unreleased rc.1, so the ordering derives never reached a user; the rationale is pinned as `Stamp` rustdoc and an AGENTS.md invariant instead. AGENTS.md gained two invariant clauses (pins survive cleanup; `Stamp` is deliberately unordered) — beyond the stage bullets, but required by the normative-docs rule. **Review follow-up:** both blockers verified real and are one defect — a7e7cd3 corrected `delete_transforms_before`'s own rustdoc but left `NotFoundAt`'s contract saying the frame "holds data" and carries "the covered range", which D1 falsified by making an empty-but-present buffer reachable (`NoTransformAvailable`, no range). Fixed at all five rustdoc sites (`transform/error.rs` variant doc, `Registry::get_transform` `# Errors`, `diagnose_not_found`, `process_get_transform`, `process_get_transform_at`); the docs now separate the two `BufferError` causes and state that the drained one is not a timing problem. Same false claim also fixed in MIGRATION.md item 3 (which told migrating users to size a retry window from `TimestampOutOfRange`) and behavior-change 5, in the AGENTS.md lookup invariant, and in the D1 CHANGELOG bullet. Doc-only: behavior already pinned by `delete_transforms_before_leaves_drained_frames_diagnosable`, so no new test. Stage 6 (D6) inherits corrected wording for `RegistryError`'s `covered: Option<(T, T)>`. |
| 2 | Time types (D11, D12) | DONE | f49c6f0 + follow-up | GREEN | No source drift: the trait, both impls, the adapter doctest and the proptest cap were all where the record said. Deviations: (a) `Timestamp::now()`'s panic wording is *not* kept verbatim — the u64 narrowing adds a second panic cause (a clock past 2554), and "time went backwards" would have misdiagnosed it, so the message and the rustdoc now name the representable range; AGENTS.md's panic invariant follows. (b) `Sub<Timestamp>` is simplified further than "drop the `seconds > u64::MAX` branch": under u64 nanos every difference is a valid `Duration`, so the whole `Ordering` match collapses to one `checked_sub().map(Duration::from_nanos)`, deleting the seconds/nanos split, both casts and their `cast_possible_truncation` allow. (c) `Add<Duration>`/`Sub<Duration>` were retyped too (not named in the bullet, but they carried the u128 arithmetic); a `Duration` wider than the u64 nanosecond range now reports overflow/underflow instead of being silently representable. (d) The inherent accessors keep `&self`: clippy's `trivially_copy_pass_by_ref` does not fire on `self` receivers at 8 bytes, so no signature churn was warranted. (e) The proptest strategy keeps the original sub-1e15 band and adds three (2^53 straddle, ~1.7e18 wall clock, top of u64), parameterized by a `headroom` the interpolation test uses so `start + span + outside` cannot overflow; band coverage was verified empirically (~25% each over 1000 draws) with a throwaway check that was then removed. Wire format confirmed unchanged as the record predicted — postcard goldens pass byte-identically (u64 varint ≡ u128 varint in range), so `tests/serde.rs` needed only a comment fix. Docs updated beyond the stage's named files where the change falsified them: README serde note (the `u128`/MessagePack-blob caveat is now false), core-types table and the `TimePoint` vs `Timestamp` section; `lib.rs` feature bullet; `time` module doc; AGENTS.md architecture + panic lines; MIGRATION items 5 and the `TimePoint` bullet; CHANGELOG, including deletion of the now-false "`checked_add` stays in the trait by decision" entry. **Review follow-up:** the two blockers filed are one defect, re-verified real — `try_now` grew a `DurationOverflow` arm and `now()` panics through it, so the crate-root Reliability bullet in `src/lib.rs` ("library code does not panic … the single documented exception is `Timestamp::now()` … before the Unix epoch") was falsified by this stage. The same contract was correctly restated in the other two places it appears (`Timestamp::now`'s rustdoc, AGENTS.md's panic invariant), so the crate shipped two contradictory panic contracts and the false one was the front-page rustdoc a docs.rs reader audits. Fixed by naming both causes in that bullet, in the wording already used by AGENTS.md and `try_now`'s `# Errors`. A sweep for the claim elsewhere (README, MIGRATION, CHANGELOG, rustdoc, the `SystemTime` impl) found no other stale site — CHANGELOG already recorded the second panic cause. Doc-only, so no new test: the overflow arm needs a system clock past 2554, which no deterministic fixture can produce. This makes the "docs updated beyond the stage's named files" list above complete — `lib.rs` needed the panic bullet as well as the feature bullet. **Gate repair (separate commit, unrelated to this stage):** the gate could not be run at all this session. `rustup run nightly` no longer selects rustup's pinned nightly (1.95, 2026-02-27) — a Nix `rust-minimal` 1.99.0-nightly-2026-08-10 precedes it in `PATH`, so `rustc`, `cargo` and `clippy` all resolve to it — and 1.99 hard-deprecates the `core::f64` module constants. `use core::f64;` in `src/geometry/quaternion/tests.rs` shadows the primitive, so 36 `f64::EPSILON`/`NAN`/`INFINITY` uses became errors under both `-D warnings` clippy steps. Fixed by dropping that import and spelling the six `f64::consts::PI` uses `core::f64::consts::PI`, as every other test file here already does — same constants, test-only, no behavior change. Pre-existing and not caused by Stage 2: it reproduces at `79a169e` with this stage's fix stashed. **For the maintainer:** the gate's "one pinned toolchain on every machine" premise does not hold on this box — rustup's nightly has no clippy component installed, so `cargo clippy` falls through to whatever toolchain `PATH` offers, and the gate silently ran on a compiler six months newer than the pinned one. |
| 3 | Numerics: libm everywhere (D10) | DONE | this commit | GREEN | No source drift: the `math` cfg fork was exactly where the record said, and `libm` was already non-optional. Deviations: (a) the `math` wrapper module is **deleted**, not kept with libm-only bodies — with the fork gone it was single-use indirection, so the five call sites now spell `libm::sqrt`/`sin`/`acos` and a file-level comment says why they are not `f64`'s. (b) The pinned tests take decimal-literal operands rather than computed rotations: an operand built with `.cos()`/`.sin()` would come from the *platform's* trig and make the pin platform-dependent, defeating its purpose. (c) Discrimination is uneven and worth knowing: only the **interior** pin fails against the pre-change `std` path (verified by temporarily restoring the intrinsics — two ulps in `w` and one in `x` on x86-64 glibc; the "one ulp per component" first recorded here was wrong, see the review follow-up). The near-antipodal pin does not, because glibc and libm agree there, and the near-identity pin *cannot*, because its branch uses only `sqrt`, which is correctly rounded in every implementation. All three still pin their distinct branch (trig / flip+trig / normalized-lerp) and run in both feature modes, which is what D10 asked for. (d) Constants are anchored to mathematics, not to the implementation's output: the interior pins sit one ulp from the exact `cos(0.305)`/`sin(0.305)`, and the near-antipodal pins are the correctly rounded `cos(0.005)`/`sin(0.005)` exactly (high-precision reference computed outside the crate). (e) Docs beyond the stage's named AGENTS.md clause, where this change falsified them: README (the "float math falls back to libm" line, plus a Performance bullet stating the both-modes guarantee), `lib.rs` Reliability bullet, `slerp` rustdoc, CHANGELOG behavior-change entry, and MIGRATION runtime item 8 — a 1.x user's slerped rotations can move by a few ulps, which "compile clean, behave differently" is exactly the section for. (f) Commit column reads "this commit" per the Stage 2 precedent (a row cannot carry the hash of the commit that contains it). Environment unchanged from the Stage 2 note: `rustup run nightly` supplies cargo/rustc 1.95, while clippy and rustfmt resolve to the Nix 1.99 nightly on `PATH`; the gate is green under that mix. **Review follow-up:** all three blockers verified real and independently reproduced; the first two are one defect (MIGRATION item 8), the third is a magnitude that had spread to four sites. (1)+(2) MIGRATION item 8 invented 1.x history and scoped the change to `std`. `v1.4.1:Cargo.toml` has no `libm` at all and `v1.4.1:src/geometry/quaternion/mod.rs` has no `cfg(feature)` — `.sqrt()`/`.acos()`/`.sin()` are unconditional — so 1.x ran the platform's math in *both* feature modes; the fork this stage deleted first appeared in 2.0.0-alpha.1. Reproduced by extracting the v1.4.1 tree and running this stage's interior fixture through it: `--no-default-features` and `std` both yield `3FEE85EA0B555D65 3FD33800DDAF16F5`, identical to each other and two/one ulps from the shipped pin. Item 8 now states the real baseline (1.x had no libm; both modes were the platform's), drops the `std` scoping in headline and advice, and quotes the measured bound — a 1.4.1 `default-features = false` user (the configuration 1.4.1's own README documents) was previously told their numbers could not move. (3) "up to one ulp per component" is false: an independent sweep on this box (identical operands through the platform path and the libm path, 1999 arcs over (0, pi) x 101 factors x 2 rotation axes, ~1.6M component comparisons) reproduced the reviewer's numbers exactly — worst **4 ulps** at 1.165531 rad, t = 0.39; distribution 49201 at 1 ulp, 12052 at 2, 493 at 3, 13 at 4. Corrected in CHANGELOG (four ulps, interior fixture moves two) and in the `bits()` doc comment in `src/geometry/quaternion/tests.rs`, which carried the same false "one ulp per component" and was not named by either reviewer. Deliberate non-edits: the CHANGELOG's historical clause ("called `f64`'s own … under `std` and `libm`'s without it") is *correct* in its rc.1 entry, which describes the delta from beta.4 — but it reads as 1.x history out of context, so the Final-stage 2.0.0 consolidation must keep it scoped to the 2.0 series; and `v2-fitness-audit.md:186`'s pre-implementation "~1 ulp" estimate stays as the dated record of what was known then (its conclusion, negligible against `UNIT_NORM_TOLERANCE`, survives at 4 ulps). Doc-only: the numbers themselves are already pinned by the three bit tests, so no new test. |
| 4 | Surface trims + renames (D5, D7, D8) | PENDING | — | — | — |
| 5 | Full enforcement (D2, D15) + double-inversion rework | PENDING | — | — | — |
| 6 | Error overhaul (D6) | PENDING | — | — | — |
| 7 | Wire format (D13, D14) | PENDING | — | — | — |
| 8 | Docs true-up (D9 + Part-2 doc items) | PENDING | — | — | — |
| 9 | Test & bench hardening | PENDING | — | — | — |
| Final | Full gate + semver-checks vs v1.4.1 + bump 2.0.0-rc.2 | PENDING | — | — | — |

Status values: `PENDING` → `IN PROGRESS` → `DONE` (or `BLOCKED` with a reason in Notes).
Review findings that led to a follow-up commit are noted in the same row.

## Interruption record — 2026-08-15, after Stage 2

Deliberate clean termination by the maintainer. Working tree clean at `f49c6f0`; Stages 1–2 are
implemented, gated, and committed. Stage 2's **post-commit review was cut short**:

- The adversarial reviewer filed one blocker, **verified real before shutdown**: the crate-root
  panic-policy rustdoc (`src/lib.rs:192-195`) still claims the *single* documented panic cause is a
  pre-epoch clock, but the u64 narrowing added a second reachable cause (`Timestamp::now()` on a
  clock past `u64::MAX` ns, mid-2554). The commit updated `Timestamp::now`'s rustdoc and AGENTS.md
  but missed this crate-root site. **Resolved** on resume by the "Address Stage 2 review findings"
  follow-up commit; see the Stage 2 row.
- The fidelity reviewer was terminated mid-run; its verdict is unknown. Optionally re-review
  `f49c6f0` (spec-fidelity against the record's Stage 2 + sub-decisions) before moving on; its
  last observed activity (leftover-`u128`/`Duration` sweeps, cross-target builds) had surfaced
  nothing at termination time.

Then proceed with Stage 3 per the protocol above. (Both items were resolved on resume — see the
Stage 2 row and commits f1a401b/b3a98f9; f1a401b also absorbed a toolchain drift, the newer nightly
deprecating the `std::f64` module constants the quaternion tests used.)

## Interruption record — 2026-08-15 evening, during Stage 3 review

Second deliberate clean termination by the maintainer. Working tree clean at `b2026be`; Stages 1–3
are implemented, gated, and committed (Stage 3 = libm everywhere, D10). Stage 3's **post-commit
review had filed its verdicts but the fix agent was stopped before editing anything.** Both
reviewers verified their findings with reproduced evidence (extracting and building v1.4.1,
ulp sweeps on both math paths). Two real defects, both documentation-accuracy in `b2026be`:

1. **MIGRATION.md runtime item 8 (~:213-220) invents false 1.x history** — it says 1.x used
   `f64`'s methods under `std` and "libm's without it". Verified false: `v1.4.1:Cargo.toml` has no
   libm dependency and its quaternion code calls `.sqrt()`/`.acos()`/`.sin()` unconditionally —
   1.x used platform math in both modes; the libm fallback only appeared in 2.0.0-alpha.1. The
   item must describe the real baseline (and not scope the change to `std` builds only).
2. **CHANGELOG.md (~:115-116) says the numeric shift is "up to one ulp per component"** — the
   commit's own pinned fixture already moves 2 ulps in w, and a reviewer sweep (2000 arcs ×
   101 factors, identical operands on glibc vs libm paths) measured up to **4 ulps** (worst at
   ~1.1655 rad, t = 0.39). Restate the bound honestly (e.g. "a few ulps, measured ≤ 4") so
   migrating users don't size exact-comparison windows at 1 ulp.

**First action on resume:** fix both doc sites, run the full gate, commit as "Address Stage 3
review findings", append the outcome to the Stage 3 row. Then proceed with Stage 4.

(Both were resolved on resume by the "Address Stage 3 review findings" commit, which also fixed a
third site carrying the same wrong ulp figure — the `bits()` doc comment in
`src/geometry/quaternion/tests.rs`. Findings re-verified independently first: the v1.4.1 tree was
extracted and run in both feature modes, and the glibc-vs-libm sweep was reproduced. See the
Stage 3 row.)
