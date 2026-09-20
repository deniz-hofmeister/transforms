# Reparenting: consolidated value assessment

Date: 2026-09-20. Repository: `feature/reparenting` at `bb75fa9`.

This summary combines the [independent assessment](INDEPENDENT.md), saved
before reading the earlier analysis, with the [2026-09-19 report](REPORT.md).
The independent document remains unchanged. A subsequent
[ROS/tf2 investigation](TF2_RESEARCH.md), requested by the maintainer,
materially strengthened the evidence of real use and revised the initial
recommendation below. Implementation claims were checked against this
branch and external claims against primary sources. The earlier report's
download counts, dependency census, and exhaustive-search claims were not
independently reproduced.

The maintainer subsequently requested a full review from the open Claude
Fable session. The [joint review agreement](JOINT_REVIEW.md) records the
result and debate; [Fable's full review](FABLE_REVIEW.md) includes its
independent checks. Both reviewers recommend continuing the design, with
three documentation corrections before merge and no code defect found.

## Recommendation

**Continue the branch's narrow, explicit reconfiguration design.** The
maintainer has clarified that the goal is a capable, minimal foundation
for Rust robotics and has accepted the exclusion of historical parent
relationships. tf2 provides inspiration and evidence of robotics workflows;
API parity, feature parity, import/export, and cross-compatibility are not
requirements. This resolves the principal scope question left open by the
earlier conditional recommendation.

The strongest core benefit remains preserving a working registry when a
proposed replacement fails. That guarantee depends on sealed state and
addresses a concrete robotics operation. Minimalism therefore favors one
validated operation over requiring callers to reconstruct the invariant
logic or exposing buffers to make that possible.

The initial merged assessment favored deferral pending a concrete need.
The research established real use of the underlying operation; the
maintainer's scope decision now establishes which temporal model this
crate intends to serve. Adoption of this exact API remains unmeasured,
but waiting for tf2 compatibility or a named external consumer is not a
completion condition. The feature still earns its place through its own
guarantee, not through a goal of broad feature completeness.

This applies the unchanged demonstrated-need rule to concrete domain
evidence; the rule does not require an existing external adopter. The
maintainer's scope decision selects the temporal model rather than waiving
minimalism. This addition supplies no precedent for adjacent capabilities.

## What the broader ROS search adds

- **Deliberate upstream support:** in 2013, tf2's maintainer explicitly
  specified behavior enabling reparenting, followed by a fix and test.
  [Maintainer comment](https://github.com/ros/geometry2/issues/28#issuecomment-26234399),
  [merged PR](https://github.com/ros/geometry2/pull/29)
- **ROS 2 application functionality:** RQT Frame Editor exposes parent
  changes; its command computes a new pose and supports undo. A separate
  ROS 2 Scene Manipulation Service implements reparenting for item and tool
  handling using its own current-state store. These are implementations,
  not proof of broad deployment or consumers of `transforms`.
  [Editor command](https://github.com/ipa320/rqt_frame_editor_plugin/blob/d9aef75fad759b7d41c03cd093499a52000ecc60/frame_editor_py/commands.py#L331),
  [scene service](https://github.com/sequenceplanner/scene_manipulation_service/blob/a315fa67236d905dd42b9285f549b707deb06d06/scene_manipulation_service/src/core/sms.rs#L469)
- **Reported use and recent demand:** a tf2 user reports reparenting static
  frames in their project; a 2025 ROS-based manipulation request asks Rerun
  to support object-to-gripper parent changes.
  [Firsthand use](https://github.com/ros/geometry2/issues/370#issuecomment-825582343),
  [2025 request](https://github.com/rerun-io/rerun/issues/1533#issuecomment-2927587600)

These sources have different weight: publisher, editor, and visualizer
behavior establishes workflows and application decisions. The scene
manipulation service is the closest explicit frame-store mutation analogue;
its latest-state model still differs from this crate's history contract.

Current ROS 2 tf2 changes parents through ordinary transform updates. It
retains dynamic samples across those changes and avoids interpolating
between different parents. Static replacements apply at all times.
This differs intentionally from the proposed crate method and is not a
compatibility gap the branch needs to close.
[Dynamic cache](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/cache.cpp),
[static cache](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/static_cache.cpp)

## What the two analyses establish together

| Question | Consolidated finding |
| --- | --- |
| Does it add a useful guarantee? | Yes. Every returned error preserves the old edge and samples; remove-then-add can destroy them before rejecting the replacement. |
| Does it newly enable subtree moves? | No. Removing and replacing only the subtree's incoming edge already reconnects its descendants. The added value is the stronger replacement contract. |
| Does it belong in the core if needed? | Yes. A helper operating on an arbitrary existing registry lacks the stored samples and topology needed for equivalent validation and restoration. Applications owning all source state can instead build and swap a candidate registry. |
| Which safeguards matter? | Cycle checking, ordinary storage validation, preservation of static/dynamic kind and retention policy, and refusal of same-parent calls that would otherwise erase history. |
| Who benefits if the time contract fits? | Explicit frame editors, scene reconfiguration commands, and wrappers that must preserve existing state on rejection. Fixed trees and configuration-driven full rebuilds benefit less. |
| What does it not supply? | Historical topology, automatic preservation of world pose, kind changes, undo, or transactions involving several edges. |

These findings follow from the [registry](../../src/core/registry/mod.rs)
and [buffer](../../src/core/buffer/mod.rs) implementations. The
[registry tests](../../src/core/registry/tests.rs) cover rejection without
mutation, descendant preservation, static replacement, coverage restart,
stale seeds, and retention; the [property tests](../../tests/properties.rs)
exercise mixed topology changes and lookup consistency.

## Time semantics are an intentional boundary

A successful dynamic move replaces the moved edge's history with one sample.
Queries crossing that edge initially work only at the seed instant, subject
to the rest of the chain's coverage. Descendant-only queries can retain their
old coverage. A static move replaces geometry for every instant, including
the past. Neither records a transition from one parent to another over time.

Consequently, correcting a static configuration can fit well. An attachment
event, map transition, or replay workload fits only if the application
accepts these exact consequences. The seed must contain coordinates relative
to the new parent; the method does not derive them. A stale seed can move
coverage backwards, and a far-future seed can cause subsequent live samples
to expire under `max_age`. Those outcomes do not invalidate
`latest_common_time`: it still describes the stored data, which may no longer
represent the freshness the application intended.

The earlier report adds useful domain evidence: REP-105 explicitly assigns
map-transition reparenting to the localization authority. That demonstrates
a legitimate domain operation, not demand for this crate's particular
history policy. [REP-105, transitions between maps](https://raw.githubusercontent.com/ros-infrastructure/rep/master/rep-0105.rst)

## The earlier report's strongest additional finding

The inspected `roslibrust_transforms` implementation separately acquires its
private registry write lock for removal, insertion, and incoming messages.
**Inference from that code:** an old-parent message can arrive between a
remove and an add, restore the old pin, and cause the intended insertion to
fail. A wrapper exposing one deliberate `reparent_frame` call under one
guard would remove that gap. This is a plausible failure sequence, not an
observed incident or evidence of its frequency. [Wrapper implementation,
checked 2026-09-20](https://raw.githubusercontent.com/RosLibRust/roslibrust/master/roslibrust_transforms/src/lib.rs)

The earlier report correctly identifies a practical integration opportunity.
Under the clarified scope, that wrapper is supporting evidence for the
replacement guarantee, not a requirement to add ROS integration here.
It overstates the case when it treats this as the only setting where atomicity
matters. Preserving state on rejection is valuable in single-threaded callers
too. Applications controlling their own lock can already prevent interleaving
between two calls, but that alone does not restore history after a failed add.

The downstream PR explicitly added removal as a reparenting escape hatch.
That is evidence the operation was considered useful; it does not establish
that the author compared both designs and judged the weaker guarantee
sufficient. [RosLibRust PR #338](https://github.com/RosLibRust/roslibrust/pull/338)

## Claims that need qualification in the merged conclusion

- **Underlying demand is demonstrated; exact API demand remains unproven.**
  The broader search found implementations, reported use, and requests that
  the earlier report missed. Claims that the use cases were all constructed
  or that public interest amounted to roughly one question per decade should
  be withdrawn. The evidence does not measure prevalence or prove that
  these users would choose this crate's history policy. Multiple agent
  perspectives remain arguments, not independent user research.
- **The method reduces specific hazards; universal safety is not established.**
  Retroactive static answers and incorrect caller-supplied poses are already
  possible through existing operations. Making a destructive operation easier
  to invoke can nevertheless change misuse frequency. Neither analysis
  measures that effect.
- **Bridge use is legitimate when the application makes the decision.** An
  explicit wrapper operation is appropriate. Automatically reparenting on
  every `ReparentingNotSupported` error lets competing publishers repeatedly
  replace each other's edge and history. The relevant distinction is who
  authorizes a topology change, not whether the caller is a bridge.
- **Lost history remains a meaningful limitation.** Both replacement routes
  lose the registry's moved-edge samples unless the caller retained its own
  data. Sharing that limitation does not make it irrelevant to a user's
  decision to adopt the feature. Retaining and re-expressing samples also
  does not provide historical parent changes.
- **Keep the unit return and sealed state.** After discussion, both reviewers
  recommend `Result<(), RegistryError<T>>` and no parent accessor for this
  branch. The earlier report's proposed old-parent return is withdrawn.
  Any future audit capability needs its own invariant and demonstrated need;
  an additive signature alone would not justify it.
- **Deferral and implementation both have costs.** A small generic method
  need not burden the hot lookup path, but documentation, compatibility,
  tests, and branch maintenance persist. Claims of zero firmware cost or
  cost-free deferral are stronger than the evidence collected here.

The [feature rustdoc](../../src/core/registry/mod.rs) and
[error guidance](../../src/core/registry/error.rs) already explain many of
these limits. The research supplies concrete workflows; the maintainer's
clarification selects the intended scope. Documentation should explain
that scope directly as part of the Rust API contract.

## Direction for completing the branch

Retain the current design commitments:

1. Parent changes require an explicit operation; ordinary insertion keeps
   rejecting them.
2. Validation precedes mutation, so a returned error preserves the old
   edge and its samples.
3. Replacement preserves kind and retention policy, while restarting the
   moved dynamic edge's coverage at the seed or replacing the static edge
   for every instant.
4. The caller supplies the new pose and owns the decision to reconfigure;
   unaffected buffers retain their state.

The full review found three documentation issues: an outdated README
release claim, an incorrect rationale for `ParentUnchanged`, and two stale
descriptions of the parent pin's lifetime. The
[joint review](JOINT_REVIEW.md) identifies the exact corrections. Fix them
and run the gate on the fixed tree. The existing rustdoc example and tests
suffice; a new example program or external integration is not a prerequisite.
Pose preservation can be supplied by the application using a lookup where
the frames are connected. Historical topology, multi-edge transactions,
and integration features are outside this change.

The enduring costs are a public API commitment and the need to make its
destructive time semantics clear. Those deserve review, but the research
and clarified goals provide no reason to redirect the branch's architecture
or defer it for lack of a tf2 compatibility story.

## Verification

`tests/test_all.sh` completed successfully on 2026-09-20 with
`rustc 1.100.0-nightly (0dfb098f3 2026-08-31)` and printed
`GATE PASSED (all steps completed)`. This covers the repository's builds,
tests, clippy feature combinations, formatting, rustdoc, examples, benchmark
smoke tests, and ARM builds. It validates the current branch against its
gate; it does not establish user demand or constitute release approval.
