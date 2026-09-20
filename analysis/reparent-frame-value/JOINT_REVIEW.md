# Reparenting: Codex and Claude Fable review agreement

Date: 2026-09-20. Branch reviewed: `feature/reparenting` at
`bb75fa9d515dac59109edd3718cde65c90922b44`.

## Decision

**Continue the existing narrow design. Correct three documentation issues
before merging; no architectural redirection is warranted by this review.**
Neither reviewer found a code defect. This is a recommendation about the
branch, not a merge or release action.

The goal is a capable, minimal foundation for Rust robotics. Historical
parent relationships, tf2 import/export, and compatibility are outside the
agreed scope. Dropping the moved edge's dynamic history is an intentional
contract, not unfinished compatibility work.

## How this agreement was reached

At the maintainer's request, Codex contacted the already-open Claude Code
session in this repository using Claude Code's cross-session messaging.
Claude Fable 5.1 (`claude-fable-5-1`) read the missing discussion, all four
analyses, repository instructions, branch history, implementation, tests,
and documentation. Its [full review](FABLE_REVIEW.md) records independent
verification and the subsequent discussion. Short-lived courier sessions
delivered messages; the existing interactive session performed the review.

Fable changed the earlier report's recommendation from deferral to
proceeding after documentation fixes. Codex challenged several parts of
the new rationale and checked the reported defects against the source.
The exchange resolved the substantive disagreements as follows.

| Question debated | Agreed result |
| --- | --- |
| Does demonstrated need require an existing external user of this exact crate? | No. AGENTS.md does not impose that requirement. Concrete domain workflows are evidence; selecting the accepted temporal model is maintainer judgment. This applies the existing rule without waiving it or establishing precedent for neighboring features. |
| What makes the operation belong in the core? | Sealed topology, kind, and stored samples prevent a helper over an arbitrary registry from reproducing the full validation and failure-preservation guarantee. Public numeric prevalidation can mitigate some failures; enforcing it at the boundary still helps. This does not justify a general transaction API. |
| What does remove/add actually do with an opposite-kind seed? | It accepts the new kind because removal releases the old pin. It does not discover a kind conflict after removal. Cycle or numeric rejection can instead destroy history before failing. These are distinct hazards. |
| How strong is the ROS evidence? | Editors, publishers, and visualizers establish real workflows. The scene manipulation service is the closest explicit frame-store mutation analogue. None establishes adoption of this exact crate or measures prevalence. |
| Should the method return the previous parent or expose a parent accessor? | Keep `Result<(), RegistryError<T>>`. Add neither capability on this branch. Any later proposal needs its own invariant and demonstrated need; semver additivity alone is insufficient. |
| Is a new example or external integration required? | No. The existing rustdoc example and tests suffice for this change. |
| Can recovery be described in publication periods? | No. The API has no publication cadence or wall-clock latency guarantee. Coverage depends on the stored samples of every edge crossed by the query. |

Fable also bounded its earlier claim about introducing no new hazard
classes to the mechanisms inspected. Neither review measured whether an
easier destructive operation changes misuse frequency. The original
[independent assessment](INDEPENDENT.md) and [earlier report](REPORT.md)
remain unchanged; Fable's addendum records the revisions to its position.

## Contract to retain

- Ordinary insertion rejects parent changes; reconfiguration is explicit.
- Every returned validation error leaves the old edge and samples intact.
- Successful replacement preserves kind and `max_age`.
- The moved dynamic edge starts with coverage at the seed instant; static
  replacement applies at all query times, including the past.
- Descendant buffers retain their state, and the caller supplies the new
  pose and owns the decision to move the frame.

## Three documentation fixes remain before merge

1. **Scope the README release claim correctly.** The
   [What's new paragraph](../../README.md#whats-new) carries forward
   2.1.3's claim that public APIs are unchanged into a description of
   2.2.0, which adds a method and error variants. Its tense also drifts.
2. **Correct the reason for `ParentUnchanged`.** The
   [error variant](../../src/core/registry/error.rs),
   [changelog](../../CHANGELOG.md), and
   [unit-test comment](../../src/core/registry/tests.rs) incorrectly
   explain it as preventing a mechanical failed-insert fallback from
   wiping history once. The refusal protects an unchanged or retried
   reconfiguration command from erasing history and keeps publication on
   `add_transform`. The method's existing `# Errors` bullet is already
   correct; the warning against conflicting publishers also remains valid.
3. **Correct the parent-pin lifetime descriptions.**
   [`add_transform` rustdoc](../../src/core/registry/mod.rs) and
   [migration guidance](../../MIGRATION.md#runtime-changes-and-limitations)
   imply that only removal releases the parent pin. Explicit reparenting
   replaces that pin while preserving kind.

These findings are recorded, not implemented in this review task. After
fixing them, run the repository gate on the fixed tree. Optional wording
and test suggestions in Fable's review are not additional merge conditions.

## Verification and limits

Fable independently ran the full `tests/test_all.sh` gate on the reviewed
commit and obtained `GATE PASSED (all steps completed)`. It also ran tests
with and without default features, clippy, rustdoc, 3,000 cases of the
topology property test, and eight scratch-crate probes. The full review
records the commands and results. Codex independently inspected the
implementation and confirmed the documentation findings.

The installed `cargo-semver-checks` could not process the nightly
compiler's rustdoc JSON format. Mechanical API compatibility verification
therefore remains incomplete; the additive-API conclusion is based on
source inspection. No source, test behavior, version, branch, commit, tag,
or remote state was changed by this review exchange.
