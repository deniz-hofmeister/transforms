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
| 2 | Time types (D11, D12) | DONE | this commit | GREEN | No source drift: the trait, both impls, the adapter doctest and the proptest cap were all where the record said. Deviations: (a) `Timestamp::now()`'s panic wording is *not* kept verbatim — the u64 narrowing adds a second panic cause (a clock past 2554), and "time went backwards" would have misdiagnosed it, so the message and the rustdoc now name the representable range; AGENTS.md's panic invariant follows. (b) `Sub<Timestamp>` is simplified further than "drop the `seconds > u64::MAX` branch": under u64 nanos every difference is a valid `Duration`, so the whole `Ordering` match collapses to one `checked_sub().map(Duration::from_nanos)`, deleting the seconds/nanos split, both casts and their `cast_possible_truncation` allow. (c) `Add<Duration>`/`Sub<Duration>` were retyped too (not named in the bullet, but they carried the u128 arithmetic); a `Duration` wider than the u64 nanosecond range now reports overflow/underflow instead of being silently representable. (d) The inherent accessors keep `&self`: clippy's `trivially_copy_pass_by_ref` does not fire on `self` receivers at 8 bytes, so no signature churn was warranted. (e) The proptest strategy keeps the original sub-1e15 band and adds three (2^53 straddle, ~1.7e18 wall clock, top of u64), parameterized by a `headroom` the interpolation test uses so `start + span + outside` cannot overflow; band coverage was verified empirically (~25% each over 1000 draws) with a throwaway check that was then removed. Wire format confirmed unchanged as the record predicted — postcard goldens pass byte-identically (u64 varint ≡ u128 varint in range), so `tests/serde.rs` needed only a comment fix. Docs updated beyond the stage's named files where the change falsified them: README serde note (the `u128`/MessagePack-blob caveat is now false), core-types table and the `TimePoint` vs `Timestamp` section; `lib.rs` feature bullet; `time` module doc; AGENTS.md architecture + panic lines; MIGRATION items 5 and the `TimePoint` bullet; CHANGELOG, including deletion of the now-false "`checked_add` stays in the trait by decision" entry. |
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
