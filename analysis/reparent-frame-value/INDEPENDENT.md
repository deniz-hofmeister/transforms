# Independent assessment of `Registry::reparent_frame`

Date: 2026-09-20. Author: Codex (GPT-6).
Repository examined: `feature/reparenting`, commit `bb75fa9`.

## Method and independence

This assessment was written and saved before opening the existing
`analysis/reparent-frame-value/REPORT.md` or the feature's explanatory
documentation. Its inputs were the supplied repository rules, implementation,
selected tests, and earlier history of frame pinning and removal. The supplied
rules already describe reparenting, so this is an independent evaluation of its
value, not a blind invention of the design. No downstream interviews, issue
survey, or market research were performed. Use cases below are hypotheses,
not evidence of adoption or demand.

The question is whether this particular operation earns a permanent place in
this crate. Reparenting in a general scene graph, time-dependent topology, and
automatic preservation of a frame's world pose are different capabilities.

## Conclusion

The feature has a defensible, narrow value: replacing an existing frame's
incoming edge with a validated seed, while preserving the original registry
on any returned error. That is a stronger contract than deleting the edge and
then trying an insertion. It fits the crate's safety priorities and can belong
in the core because an arbitrary borrowed registry does not expose enough
state to implement the same transaction externally.

That establishes architectural legitimacy, not sufficient user demand. My
recommendation is conditional inclusion: add it for an identified workflow
that needs this failure guarantee and accepts discarding the moved edge's
history. Without that evidence, defer publication of the API. The current
implementation's existence and the general usefulness of mutable trees do not
by themselves meet the repository's demonstrated-need rule.

## What users actually gain

`remove_frame(child)` already removes just that child's incoming edge.
Descendants remain attached to the child. Inserting a new edge for the same
child therefore reconnects the subtree without rebuilding descendants. The
new method does not introduce the ability to move subtrees.

The meaningful difference is failure behavior. With removal followed by
`add_transform`, an invalid replacement can fail after the old edge and its
history have been discarded. A transform constructed successfully can still
be unsuitable: its destination can create a cycle, its kind can conflict with
the intended policy, and a derived transform can fail storage validation.
The proposed method checks against the existing topology, builds a fresh
buffer of the old kind, inserts the seed into it, and replaces the old buffer
only after success. On `Err`, the original edge and samples remain available.

This is valuable even without concurrent readers. An application can already
hold its lock or exclusive borrow across two public calls. The additional
guarantee is preservation on failure, not a special form of thread safety.

Other useful constraints follow from the operation being explicit:

- Routine `add_transform` still rejects accidental parent changes.
- A request for the same parent fails instead of unexpectedly clearing history.
- Static/dynamic kind and retention policy survive the replacement.
- Cycles are rejected using topology, including edges whose sample buffers
  have been drained.
- Descendant buffers remain intact; no descendant traversal or rewriting is
  needed to move the subtree.

An application that owns all original samples and topology can build a
replacement registry and swap it after successful validation. That is a real
alternative, particularly for configuration reloads. It does not give a small
helper the same behavior on an arbitrary existing registry: stored samples
and pins are sealed, and there is no public snapshot/clone transaction API.
Adding broad state access merely to enable this helper would commit the core
to more public surface than the narrow operation itself.

## Where the contract could be useful

| Candidate workflow | Value | Required qualification |
| --- | --- | --- |
| Reconnect an existing sensor or robot subtree during controlled reconfiguration | Keeps a working registry intact if the replacement is rejected | The moved edge's old samples may be discarded; the application supplies a correct new pose |
| Correct an incorrectly configured parent | Changes topology without rebuilding unaffected descendants | Static corrections intentionally reinterpret all times; dynamic corrections restart coverage |
| Attach an existing disconnected subtree to a new reference frame | Encapsulates validation and replacement of its existing incoming edge | A parentless root instead needs ordinary insertion; reparenting requires an edge to replace |
| Switch an object's attachment during live manipulation | May simplify the current-state update | Often needs historical event semantics or a static/dynamic kind change, which this operation does not provide |

The first two are the strongest candidates. The last is an attractive example
that can oversell the feature: a physical attachment event is not necessarily
a good match for a history-discarding registry edit.

Many users with fixed calibrated trees, append-only transform streams, or
registries rebuilt from configuration would gain little. Their existing
workflows need neither an edge replacement transaction nor its error cases.

## Costs and limits that matter

### Time is the principal semantic cost

For a dynamic edge with samples at 10 and 20, replacing it with a seed at 30
leaves only coverage `[30, 30]` on that edge. Historical queries crossing it
lose access to the discarded samples. Future queries beyond 30 also fail
until sufficient new samples arrive; there is no extrapolation. Queries
entirely within the retained subtree can still work when their connecting
chain does not cross the replaced edge. A seed earlier than the old newest
sample is allowed, so latest common time can move backwards.

A static replacement is different: the new edge answers at every time. It
does not record “the attachment changed at 30.” Historical queries can succeed
with the replacement geometry. That can be correct for a calibration
correction and misleading for an event log. The library validates structure
and numbers; it cannot establish that the caller's historical interpretation
matches the physical system.

Neither form adds versioned topology. Old samples subsequently published
under the old parent are rejected while the new parent remains pinned, but
the method does not provide general epoch or stale-message protection. The
application remains responsible for publishers and event ordering.

### It accepts a pose; it does not compute a physical move

The supplied transform must express the child in the new parent's coordinates.
Changing a parent name while reusing the old translation and rotation is
generally wrong. Where both frames are connected and covered at the desired
instant, the caller can obtain the new relationship through an ordinary
lookup before replacing the edge. Otherwise it needs an external measurement
or calibration. Reparenting guarantees neither continuity of world pose nor
physical correctness of the supplied measurement.

### Successful replacement is deliberately destructive

The moved edge's stored history is discarded; no undo record is returned.
Preserving static/dynamic kind is useful protection, but it rules out some
attachment workflows. The method changes one edge, not several edges as one
transaction. Those limitations should stay explicit rather than creating
pressure for unrelated transaction, snapshot, or topology-history APIs.

### Small runtime logic still creates lasting maintenance work

The implementation reuses ordinary insertion validation and does not require
new dependencies or a new lookup algorithm. That is favorable. Its cost
nevertheless includes a public method, error variants, documentation, and
tests for interactions with cleanup, coverage, static transforms, and time
types. The existing `ReparentingNotSupported` error name also becomes less
literal: it describes rejection by ordinary insertion, not the absence of
all reparenting support.

The operation walks ancestry to check cycles and drops the replaced history.
It should not be described as constant-time or hard real-time merely because
the final map replacement is small. I have not measured its performance.

## The strongest case against inclusion

The crate deliberately prefers absence to speculative convenience. It already
allows explicit removal and reinsertion, and applications that own their
configuration can validate a candidate registry before replacing the active
one. If reconfiguration is rare, fully controlled, or followed by rebuilding
state anyway, the new transaction can be marginal value.

Moreover, the capability most easily suggested by the name—tracking a moving
object as it changes attachment through time—is not supplied. Shipping a
carefully implemented operation that users naturally overinterpret can still
increase wrong-answer risk, especially for static replacements. Extensive
documentation is not a substitute for identifying users whose intended
semantics actually match the contract.

The case against inclusion is therefore credible even if the implementation
is correct. “Can be made safe at the storage boundary” and “belongs in the
published product now” are separate judgments.

## Decision criteria

Inclusion is justified when a concrete consumer can answer all of these:

1. It must replace an existing incoming edge while continuing to use the
   existing registry and unaffected history.
2. A rejected replacement must preserve the old edge and all its samples.
3. Losing the moved edge's history, retaining its kind, and supplying the new
   pose are acceptable.
4. Static all-time replacement or dynamic coverage restart matches its time
   semantics; historical topology is unnecessary.

If those conditions hold, the narrow method is preferable to exposing buffer
internals or teaching partial rollback. If they do not, defer the addition
and use the existing public operations or an application-owned rebuild. This
assessment does not recommend expanding the method to satisfy other cases.

## Evidence consulted before reading the other report

- [Repository rules](../../AGENTS.md): scope, safety priorities, demand
  threshold, and the supplied feature contract.
- [Registry implementation](../../src/core/registry/mod.rs): `remove_frame`,
  `reparent_frame`, insertion, cycle checking, and chain resolution.
- [Buffer implementation](../../src/core/buffer/mod.rs): `empty_like`,
  insertion validation, kind, coverage, and retention.
- [Property tests](../../tests/properties.rs): mixed insert/reparent operations
  and consistency between latest common time and lookup. Tests demonstrate
  intended invariants, not user demand.
- Git commits `15fb526` (strict tree and original failure modes), `a7e7cd3`
  (pins surviving cleanup), and `80242d6` (subtree moves already possible by
  replacing only their incoming edge).

This document is the preserved first assessment. Later comparison and any
revised recommendation belong in `CONSOLIDATED.md`.
