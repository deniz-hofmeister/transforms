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
| 3 | Numerics: libm everywhere (D10) | PENDING | — | — | — |
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

Then proceed with Stage 3 per the protocol above.
