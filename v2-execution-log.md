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
| 1 | Correctness semantics (D1, D3, D4) | DONE | this commit | GREEN | No source drift: the retain, the test, the `Stamp` derive and the contradicted rustdoc were all where the record said. `Buffer`'s pin-doc needed no edit — it already promised what D1 now makes true, so only the registry side was corrected. No CHANGELOG entry for D4: `Stamp` ships first in this unreleased rc.1, so the ordering derives never reached a user; the rationale is pinned as `Stamp` rustdoc and an AGENTS.md invariant instead. AGENTS.md gained two invariant clauses (pins survive cleanup; `Stamp` is deliberately unordered) — beyond the stage bullets, but required by the normative-docs rule. |
| 2 | Time types (D11, D12) | PENDING | — | — | — |
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
