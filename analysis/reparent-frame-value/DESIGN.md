# Explicit frame reparenting: detailed design

Date: 2026-09-20. Draft revision: 2, reviewed by Claude Fable 5.1.
Implementation baseline: `feature/reparenting` at
`bb75fa9d515dac59109edd3718cde65c90922b44`.

This document specifies the intended behavior of the existing branch. It
incorporates the maintainer's scope decision and the
[joint review agreement](JOINT_REVIEW.md). The method is already implemented;
the remaining implementation-facing work identified so far is documentation
correction, not a new algorithm. Repository-wide requirements in
[AGENTS.md](../../AGENTS.md) continue to apply. This draft does not authorize
merging, tagging, or publication.

## 1. Purpose and placement

Provide one explicit operation for replacing a child frame's incoming edge
without destroying the current registry if the replacement is rejected.
The crate is a Rust-first foundation for robotics: it owns computations
whose correctness depends on its sealed invariants and keeps the public
surface narrow.

The underlying operation has concrete frame-editing and scene-management
uses documented in [the research](TF2_RESEARCH.md). These sources establish
real workflows, not measured adoption of this exact API. A helper over an
arbitrary existing registry cannot reproduce the complete guarantee: the
registry does not expose pinned topology, kind, or stored samples for
equivalent validation and restoration. Applications owning all input state
can instead construct and swap a replacement registry.

The current remove-then-add alternative has two distinct consequences:

- A cycle or invalid seed can be rejected after the old buffer is gone.
- Removing the buffer releases its kind. An opposite-kind seed can then
  be accepted, changing time semantics without a kind-conflict error.

The explicit operation closes those gaps while preserving the existing
strict-tree architecture. This is a case-specific application of the scope
rules; it does not establish precedent for other topology operations.

## 2. Scope and settled exclusions

The operation replaces exactly one existing incoming edge, supplies its
initial geometry, and preserves all other buffers. Descendants move with
their subtree because their immediate parent names remain unchanged.

This design includes no historical parent relationships, tf2 API or feature
parity, tf2 import/export, or cross-compatibility requirement. It also adds
no automatic reparenting during ingestion, pose-preservation option,
kind-changing mode, old-parent return, parent accessor, history export,
undo facility, multi-edge transaction, internal lock, or callback.

The caller owns the decision to reconfigure and the new pose. The seed's
timestamp is data, not an effective-from declaration or an epoch boundary.
Existing rigid-body, `f64`, no-extrapolation, dependency, MSRV, and `no_std`
requirements remain in force. The README and crate-root Non-Goals need no
change for this feature.

## 3. Public API

The public paths are `transforms::Registry`,
`transforms::geometry::Transform`, `transforms::time::TimePoint`, and
`transforms::errors::RegistryError`. Inside the existing
`impl<T: TimePoint> Registry<T>`:

```rust
pub fn reparent_frame(
    &mut self,
    t: Transform<T>,
) -> Result<(), RegistryError<T>>;
```

This is a signature excerpt, not a standalone Rust program.

| Input or output | Meaning |
| --- | --- |
| `t.child()` | Existing frame whose incoming edge is replaced |
| `t.parent()` | Requested new parent, different from the current parent |
| Translation and rotation | Geometry mapping child-frame coordinates into the new parent frame |
| `Stamp::At(s)` | Dynamic seed at instant `s`; requires an existing dynamic edge |
| `Stamp::Static` | Static seed valid at every query time; requires an existing static edge |
| `Ok(())` | Replacement committed; no displaced state is returned |
| `Err(RegistryError<T>)` | Replacement refused; the registry remains unchanged |

The transform is consumed on success or error. Registry preservation does
not imply returning the supplied transform to the caller. A caller needing
to retain it can clone it before the call. No additional trait bounds are
introduced beyond the existing `TimePoint` contract.

Add two variants to the existing `#[non_exhaustive] RegistryError<T>`:

```rust
NoParentToReplace(String),
ParentUnchanged(String),
```

These are enum-variant excerpts. Each payload is the child frame name.
Their `Display` strings are respectively:

```text
frame {child} has no parent to replace
frame {child} already has the requested parent
```

Keep `ReparentingNotSupported { current_parent: String }` for ordinary
insertion. Its variant name remains compatible; its message becomes:

```text
add_transform cannot change the child frame's parent ({current_parent})
```

`reparent_frame` does not return that variant. There is no new feature flag,
dependency, public type, serialization representation, or successful return
payload.

Existing variant names and payloads are part of the compatibility surface;
the non-exhaustive enum permits additional variants. `Display` text is
diagnostic, not a stable machine-readable interface. The changed message
above is not an error-variant rename.

## 4. State model and successful transition

Let `R` be the registry's child-keyed buffer map. Let `c = t.child()`,
`p = t.parent()`, and `B = R[c]` be the existing buffer. For a dynamic buffer,
write `H` for stored samples, `l` for its expiry reference, and `a` for its
optional `max_age` policy.

On success, only `R[c]` changes:

| State | Before | After |
| --- | --- | --- |
| Child pin | `c` | `c` |
| Parent pin | Old parent `q`, with `q != p` | `p` |
| Dynamic kind | Dynamic | Dynamic |
| Dynamic samples | `H`, possibly empty after cleanup | Exactly the seed sample at `s` |
| Dynamic expiry reference | `l`, possibly reset by cleanup | `Some(s)` |
| Dynamic retention policy | `a` | The same `a` |
| Static kind and value | Static, holding the old transform | Static, holding `t` |
| Every buffer keyed by a child other than `c` | Existing state | Unchanged |
| Registry-level retention configuration | Existing value | Unchanged |

Only the row for the existing kind applies. A kind change is rejected.
Replacing an edge does not normalize, invert, interpolate, or otherwise
re-express its geometry. A direct child-to-new-parent lookup of a dynamic
seed at its stored timestamp retains the existing exact stored-sample
behavior. A static lookup carries the requested timestamp, as all registry
lookups do; it does not return `Stamp::Static` merely because storage is static.

The number of child buffers is unchanged. The set of known frame names can
change: a previously unregistered new parent becomes a known root, and an
old parent mentioned nowhere else disappears. If the old parent has its own
incoming edge or remains another child's parent, it remains known.

No old sample is retained or transformed into the new parent frame. Kind
and retention policy belong to the frame; expiry state and sample history
belong to the replaced buffer contents.

## 5. Validation order and errors

All checks finish before the registry map is mutated. The topology checks
and their precedence over seed insertion are documented in the method's
`# Errors` section and are part of this feature's contract. Rows 5a through
5d spell out the current order of the shared storage checks; the operation
delegates to that boundary rather than defining a second validator. Earlier
failures take precedence when the seed violates multiple conditions.

| Order | Check | Error |
| --- | --- | --- |
| 1 | `c` has no buffer and occurs nowhere as a parent | `UnknownFrame(c)` |
| 1 | `c` has no buffer but is known as a parent/root | `NoParentToReplace(c)` |
| 2 | Existing buffer has no parent pin | `NoParentToReplace(c)`; defensive, unreachable through the public registry API |
| 3 | Pinned parent already equals `p` | `ParentUnchanged(c)` |
| 4 | Attaching `c` under `p` would close a non-self cycle | `CycleDetected` |
| 5a | Seed has any non-finite translation or quaternion component | `NonFiniteValues` |
| 5b | Seed's quaternion norm fails the existing unit-norm tolerance | `NonUnitRotation(norm)` |
| 5c | `p == c` | `SelfReferentialFrame` |
| 5d | Seed stamp's kind differs from the existing buffer's kind | `StaticDynamicConflict` |

The listed variants are exhaustive for this design. This method returns
none of `ReparentingNotSupported`, `NotFoundAt`, `Disconnected`,
`NoCommonTime`, or the wrapping `TransformError` variant. Downstream matches
still require a wildcard because the public error enum is non-exhaustive.

The two cases in step 1 are mutually exclusive. Step 5 uses ordinary
`Buffer::insert` on a fresh buffer, including `Transform::validate`;
the design must not duplicate or weaken those checks. Numeric failures
are canonicalized to the flat `RegistryError` variants, not wrapped in
`RegistryError::TransformError`.

Self-reference reaches the storage check: the cycle walker checks whether
an ancestor reached from the proposed parent points to `c`, and does not
treat its starting node being `c` as the cycle result. Consequently an
otherwise valid existing frame moved under itself reports
`SelfReferentialFrame`, while a non-finite self-referential seed first
reports `NonFiniteValues`.

The fresh buffer has no parent or child pin before insertion. Its insert
therefore cannot reject this seed for a conflicting pinned parent or child.
The moved frame was already registered, so this operation never creates an
empty registered buffer. No lookup, interpolation, timestamp conversion,
or comparison with the old history is part of validation.

Unknown/new parents are allowed, consistently with `add_transform`.
Frame names follow the existing insertion rules; no new naming restriction
or reserved root name is introduced.
Attaching a known root for the first time uses `add_transform`; it is not
an implicit special case of reparenting. Reapplying an unchanged move is
an error rather than a successful no-op or upsert: this prevents a repeated
configuration command from clearing history and keeps sample publication
on `add_transform`.

## 6. Failure preservation and implementation

The implementation sequence is:

1. Borrow the existing buffer and diagnose the child, parent, and cycle
   conditions in section 5 against the current tree.
2. Create `fresh = old.empty_like()` without changing `old`.
3. Copy the child name for the map key and insert the supplied seed into
   `fresh` through the ordinary storage boundary.
4. If insertion fails, return its converted error and drop `fresh`.
5. Replace the existing map entry with the validated, populated `fresh`.
   Dropping the displaced buffer releases its old samples.
6. Return `Ok(())`.

`Buffer::empty_like` is crate-private. It returns an unpinned empty buffer,
copying only the kind and, for a dynamic buffer, its `max_age` value. A
dynamic buffer starts with an empty map and no expiry reference; a static
buffer starts with an empty slot. The seed's successful insertion
establishes the new pins and populated storage. Its full match over the
kind's fields requires an explicit carry-or-reset decision if those fields
change later.

For every returned error, all registered buffers, samples, frame pins,
retention policies, and expiry references equal their pre-call state.
If another check rejects a proposal whose new parent was unregistered, that
name is not registered as a side effect.
This is a guarantee about the registry on a returned `Err`, not a promise
to recover from allocator failure or arbitrary failures inside a caller's
custom trait implementations. The implementation introduces no panic path
or unchecked time arithmetic.

There is no additional timestamp admissibility rule. Every representable
instant of `T` can seed an otherwise valid dynamic move, including
`Timestamp::zero()`, negative instants on a custom clock that supports them,
and a `SystemTime` before the Unix epoch when the `std` feature is enabled.
No value means "now", "latest", or static.

The operation uses timestamps through copying and ordering and, when
seeding a dynamic buffer with `max_age`, one `TimePoint::checked_sub`
call. An error from that subtraction skips the expiry sweep for this insert
and is not returned as a reparenting error. No `duration_since` or
`as_seconds_lossy` call occurs inside the operation. Later queries and
their error formatting retain their own time-operation requirements.

### Why checking the old topology is correct

Represent a stored parent relationship as an upward edge `child -> parent`.
Before the call the graph is finite and acyclic, so an upward walk
terminates. Apart from the self-reference case,
a proposed edge `c -> p` creates a cycle exactly when following existing
parents from `p` reaches `c`. That path does not require following `c`'s
old outgoing edge: the walk has already found the cycle when it reaches
`c`. Therefore the cycle decision is the same with that old edge present
or removed. Checking the current tree is sufficient, and no temporary
destructive removal is needed.

All untouched edges remain acyclic, and any newly introduced cycle would
have to contain the replacement edge. The check therefore preserves
acyclicity for every successful transition, including moves between
disconnected trees and moves past drained buffers whose pins remain.

### Work and memory

A successful move walks the proposed parent's ancestor chain, creates one
replacement buffer, and destroys the displaced samples. String hashing,
copying, and allocator work also apply. Diagnosing an absent child as a
known root can scan the registry. No descendant buffer is walked to carry
the subtree, and the whole registry is not cloned.

The old buffer and one seeded replacement coexist briefly. Releasing a
large history takes work proportional to that history's storage; one public
call does not imply constant-time execution or bounded real-time latency.
There is no new work on ordinary lookup paths.

## 7. Time semantics and query consequences

### Dynamic edges

Immediately after a move seeded at `s`, the moved edge's covered interval
is exactly `[s, s]`. At that instant it has data; elsewhere it cannot serve
a sample. A query crossing that edge can therefore fail with
`NotFoundAt { frame: c, requested, covered: Some((s, s)), .. }`.

That payload is not promised for every failing query. When a failed
non-identity lookup reaches topology/sampling diagnosis, unknown endpoints
take precedence, checked target first and then source. Otherwise, the first
recorded buffer failure from the target-side walk and then the source-side
walk is reported: empty or out-of-range data becomes `NotFoundAt`, while
an interpolation failure propagates its underlying error. With no recorded
failure, the diagnosis is `Disconnected`. A successfully resolved connecting
path can ignore a recorded failure above its common ancestor; geometry and
time failures otherwise retain their existing propagation. This is not a
new global ordering over every possible lookup error.
Queries whose connecting path does not cross the moved edge can continue
using their untouched history. Their endpoints still must resolve through
the current topology according to the existing query contract.

Later `add_transform` calls must name the new parent and retain the dynamic
kind. Accepted retained samples extend coverage, and interpolation remains
confined to samples expressed in that one pinned parent frame. Old-parent
messages are rejected by ordinary insertion instead of switching the frame
back. No automatic fallback invokes `reparent_frame`.

No monotonicity or freshness guard compares `s` with dropped samples. A
stale seed is accepted and can move coverage backwards. A later insertion
with a timestamp before `s` is also allowed under ordinary insertion and
retention rules. The seed establishes no permanent lower timestamp bound.
There is no required publication period, wall-clock consultation, minimum
recovery latency, or guarantee that a chain ever gains further coverage.

For a buffer with `max_age`, its newest-inserted-timestamp reference resets
to the seed and subsequently tracks the maximum timestamp inserted since
the replacement, subject to existing cleanup behavior. Samples strictly older than
that reference minus `max_age` expire on insertion under the existing
checked-arithmetic rules; a sample exactly on the boundary is retained.
A far-future seed may thus cause subsequently
arriving older samples to be immediately discarded. Reparenting adds no
freshness policy to compensate for a caller's choice of timestamp.

### Static edges

A successful static move stores one static seed that answers every instant,
including times before the call. The old geometry is replaced retroactively,
as with an ordinary same-parent static republish. There is no transition
timestamp or interval where the old parent is still selected.

Static reconfiguration asserts the replacement configuration at all query
times. A timed attach/detach event with historical before/after requirements
is not represented by this operation. Making the edge dynamic supplies
sampled coverage, but this design still does not store historical parents.

### `latest_common_time` and two-time queries

`latest_common_time` examines the current connecting path. A moved dynamic
edge initially constrains that path to `s`; if other dynamic ranges exclude
`s`, the existing `NoCommonTime` behavior applies. Unknown and disconnected
endpoints retain their own diagnoses. A path excluding the moved edge is
not constrained by its lost history. All-static and identity paths retain
their existing timeless result. Existing arithmetic checks still apply.

`get_transform_at` resolves its two instants against one current topology.
It does not select an old parent for its earlier leg. A dynamic leg crossing
the moved edge may lose coverage; a static replacement can change answers
at either instant. The caller remains responsible for a physically
stationary fixed frame and for preserving the source/target time provenance
of a two-time result. Such a result is not a valid single-time reparent seed
merely because its numeric validation succeeds.

The registry's existing identity rule is unchanged: a frame queried relative
to itself yields identity even when its name is unregistered. Thus a
disappearing old root produces `UnknownFrame` only where the ordinary
non-identity query contract requires it.

## 8. Caller responsibilities and pose construction

The seed must express the child's geometry in the new parent's coordinates.
Reusing the old numeric translation and rotation with a new parent name
does not generally preserve world pose. The registry checks structural and
numeric validity, not whether the geometry matches a real robot.

Where the current tree connects the new parent to the child at a suitable
instant, the application can obtain that relative pose before replacing
the edge. Across disconnected trees, the application needs an independently
known relationship, such as a measured or configured pose.

**Lookup results always carry `Stamp::At(requested_time)`.** For a dynamic
edge, a suitable ordinary `get_transform` result can serve as the seed;
storage revalidates it because derived transforms can drift or overflow.
For a static edge, forwarding that result would fail with
`StaticDynamicConflict`. The application must construct a static seed using
`Transform::static_between(new_parent, child, pose.translation(),
pose.rotation())`, asserting all-time validity and passing constructor
validation, which can reject numeric drift in a derived pose. Kind
knowledge comes from the application's configuration;
this design does not introduce a registry kind accessor.

With a shared registry, acquire the application's exclusive/write guard
before dependent registry reads and retain it through the decision and
replacement. `&mut self` guarantees exclusive access for this call; it
does not reserve state across separate lock acquisitions. The method has no
expected-old-parent parameter and can replace a parent installed by another
writer between a stale decision and the eventual call.

Applications coordinate publishers, calibration, physical transitions, and
logging. The registry supplies no distributed transaction. Reparent on an
authorized reconfiguration decision; do not turn every
`ReparentingNotSupported` error into a move. Two competing publishers could
otherwise alternate parents and repeatedly destroy history.

## 9. Worked dynamic reconfiguration

This complete example uses one-dimensional translations and a static tool
frame for clarity. It preserves the item's world pose at the chosen instant,
shows the descendant edge and its data surviving, verifies lost incoming-edge history,
and checks that a rejected cycle preserves the successful replacement.
The application explicitly knows that `item` is dynamic.

```rust
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

fn main() {
    let mut registry = Registry::<Timestamp>::new();
    let t1 = Timestamp::from_nanos(1);
    let t2 = Timestamp::from_nanos(2);

    for (parent, child, x) in [
        ("world", "bin", 10.0),
        ("world", "tool", 30.0),
        ("item", "tip", 0.5),
    ] {
        registry
            .add_transform(
                Transform::static_between(
                    parent,
                    child,
                    Vector3::new(x, 0.0, 0.0),
                    Quaternion::identity(),
                )
                .unwrap(),
            )
            .unwrap();
    }
    for (stamp, x) in [(t1, 1.0), (t2, 3.0)] {
        registry
            .add_transform(
                Transform::new(
                    "bin",
                    "item",
                    Vector3::new(x, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(stamp),
                )
                .unwrap(),
            )
            .unwrap();
    }

    let before = registry.get_transform("world", "item", t2).unwrap();
    let seed = registry.get_transform("tool", "item", t2).unwrap();
    assert_eq!(seed.translation(), Vector3::new(-17.0, 0.0, 0.0));
    registry.reparent_frame(seed).unwrap();
    assert_eq!(registry.get_transform("world", "item", t2).unwrap(), before);
    assert_eq!(
        registry
            .get_transform("item", "tip", t1)
            .unwrap()
            .translation(),
        Vector3::new(0.5, 0.0, 0.0),
    );
    assert!(matches!(
        registry.get_transform("tool", "item", t1),
        Err(RegistryError::NotFoundAt {
            frame, requested, covered: Some((start, end)), ..
        }) if frame == "item" && requested == t1 && start == t2 && end == t2
    ));

    let cyclic_seed = Transform::new(
        "tip",
        "item",
        Vector3::zero(),
        Quaternion::identity(),
        Stamp::At(t2),
    )
    .unwrap();
    assert!(matches!(
        registry.reparent_frame(cyclic_seed),
        Err(RegistryError::CycleDetected),
    ));
    assert_eq!(registry.get_transform("world", "item", t2).unwrap(), before);
}
```

The small values are exactly representable and identity-rotation arithmetic
is exact in this example, so exact assertions are appropriate here. This
does not promise exact general composition; general computed geometry uses
tolerant comparisons. This design-document example supplements the existing rustdoc
example; it is not a requirement to add another program to `examples/`.

## 10. Alternatives and interactions

| Alternative or adjacent operation | Decision |
| --- | --- |
| Remove then add | Remains available for deliberate kind changes or rebuilding from caller-retained samples; it does not retain registry-owned history or provide this failure guarantee |
| Rebuild and swap the registry | Appropriate when the application owns complete source configuration/data and wants a larger atomic application-level change |
| Implicit parent changes in `add_transform` | Rejected; routine samples must not silently reconfigure the tree |
| Empty replacement without a seed | Rejected; an edge should be populated through the same validated first-insert boundary, including static edges |
| Historical parent samples | Excluded by the maintainer; would change the storage and resolution model |
| Preserve-pose option | Derivable through a suitable ordinary lookup or supplied pose; reference time and physical interpretation belong to the caller |
| Return the old parent, expose topology, or export samples | Not justified for this change; keep `()` and sealed buffers |
| Same-parent no-op or upsert | Rejected; a repeated move should reveal the unchanged configuration and retain history |
| Reject stale seeds or enforce a new epoch floor | Not introduced; ordinary out-of-order insertion semantics remain |
| Atomic edge reversal or multi-edge edits | Separate operation family; not provided by this method |

Manual cleanup never releases pins. A drained dynamic frame can still be
reparented and is seeded while preserving its kind and retention policy.
After a move, cleanup retains the new pins; only removal releases the frame
on the cleanup path. Static buffers remain exempt from time-based cleanup.

## 11. Compatibility and verification criteria

The intended release is additive relative to v2.1.3: one public method,
two variants on a non-exhaustive error enum, and revised error text. The
crate-private helper adds no exported commitment. Existing insertion,
lookup, geometry, and serialization behavior are otherwise unchanged.
Retain edition 2024, Rust 1.85, the existing dependencies, `forbid(unsafe_code)`,
and identical availability and computed behavior across `std`/`no_std`.
There is no serde representation change to `Transform` or `Stamp`.

The existing [unit tests](../../src/core/registry/tests.rs) and
[property test](../../tests/properties.rs) are the primary witnesses:

| Required property | Existing representative coverage |
| --- | --- |
| Subtree moves without rewriting descendants | `reparent_frame_moves_the_whole_subtree`, `reparent_frame_accepts_a_sibling_move` |
| Cycle rejection, including drained topology | `reparent_frame_rejects_cycles` |
| History loss and later new-parent publication | `reparent_frame_drops_the_frames_history_loudly`, `reparent_frame_refuses_the_old_edges_publisher_and_grows_coverage_back` |
| Kind and expiry-policy preservation | `reparent_frame_rejects_a_kind_change_in_both_directions`, `reparent_frame_preserves_the_max_age_expiry_policy` |
| Root/unknown distinction and same-parent refusal | `reparent_frame_diagnoses_a_root_and_an_unknown_frame_differently`, `reparent_frame_rejects_an_unchanged_parent` |
| Validation precedence, self-reference, and flat numeric errors | `reparent_frame_checks_the_topology_before_the_seed`, `reparent_frame_diagnoses_the_frame_before_a_broken_seed`, `reparent_frame_rejects_a_self_referential_or_non_finite_seed`, `reparent_frame_rejects_a_non_unit_rotation_seed_as_the_flat_variant` |
| Static replacement and retained static behavior | `reparent_frame_moves_a_static_frame_and_keeps_it_static`, `reparent_frame_moves_a_static_frame_in_a_max_age_registry` |
| Seed exactness, stale stamps, unknown parents | `reparent_frame_seed_survives_a_lookup_bit_for_bit`, `reparent_frame_accepts_a_stale_seed_and_moves_coverage_backwards`, `reparent_frame_accepts_an_unknown_new_parent` |
| Coverage and two-time-query interactions | `reparent_frame_collapses_latest_common_time_to_the_seed`, `get_transform_at_across_a_reparented_hop_depends_on_the_fixed_frame` |
| Cross-tree moves, successive moves, cleanup pins | `reparent_frame_joins_two_disconnected_trees`, `successive_reparent_frames_pay_per_move_and_can_disjoin_a_chain`, `reparent_frame_then_cleanup_keeps_the_new_pin_and_kind` |
| Mixed accepted/rejected operations remain walkable, with valid promised coverage | `random_inserts_and_re_parents_keep_the_tree_walkable` |
| Remove/add reconnects a subtree while retaining descendant buffers | `remove_frame_then_readding_root_reconnects_subtree` |

The failure cases also check preservation of prior observable data. Keep
meaningful regressions with any correctness fix; no new tests are required
merely to duplicate existing witnesses. Compile and run the complete example
in section 9 when verifying this document; analysis Markdown is not collected
automatically as a rustdoc test.

Run `tests/test_all.sh` to completion on any changed implementation-facing
tree. The full gate covers feature combinations, lint/docs, examples,
benchmark smoke tests, and the required ARM builds. Published API checking
is an additional release check: the earlier review could not run the
installed semver checker against this nightly's rustdoc format. A passing
gate does not silently fill that verification gap.

## 12. Required documentation corrections before merge

The behavioral implementation already follows this design. Apply these
previously agreed corrections in a later implementation-facing change:

1. In README's What's new, scope the unchanged-public-API statement to
   2.1.3 or remove it. The 2.2.0 method and variants are additions.
2. In `RegistryError::ParentUnchanged` rustdoc, the 2.2.0 changelog entry,
   and the corresponding unit-test comment, explain unchanged/retried
   configuration commands and history preservation. Remove the inaccurate
   mechanical-failed-insert rationale. The method's Errors bullet already
   states the correct non-upsert behavior.
3. In `add_transform` rustdoc and MIGRATION.md, describe parent replacement
   through this explicit operation as well as pin release through removal;
   the kind still survives reparenting.

Those three correction groups are pending, not implemented by this design
document. Existing rustdoc examples suffice; no external adopter, integration
layer, new accessor, or new example program is a merge prerequisite.

## 13. Review status

Claude Fable 5.1 reviewed revision 1 against the implementation and wrote
[a separate critique](DESIGN_REVIEW_FABLE.md). It accepted the design with
one blocking example error and five required clarifications. Revision 2
addresses them:

| Finding | Resolution |
| --- | --- |
| B1: invalid root-level `RegistryError` import | Section 9 imports it from `transforms::errors`; the corrected example compiles and runs |
| R1: compatibility and ordering commitments | Sections 3 and 5 distinguish stable variants/payloads, diagnostic text, documented topology precedence, and the shared seed validator |
| R2: completeness of the error table | Section 5 explicitly identifies the complete error set for this design |
| R3: custom clocks and time operations | Section 6 covers zero, negative/custom and pre-epoch instants, checked subtraction, and the absence of duration/seconds conversion |
| R4: lookup failure precedence | Section 7 states the diagnosis order while preserving interpolation-error propagation and successful-path handling |
| R5: missing witnesses | Section 11 adds the self-reference/non-finite test and the remove/add subtree test |

The revision also clarifies termination of the cycle walk, inclusive
retention boundaries, the static descendant in the example, and the scope
of exact arithmetic. It makes no change to the feature's behavior or API.
Fable reviewed revision 2 against the branch and confirmed that no blocking
or required findings remain. Its final verdict is appended to the critique.
Both reviewers independently extracted, compiled, and ran the complete
section 9 example successfully. Codex also ran `tests/test_all.sh` to
completion on 2026-09-20: `GATE PASSED (all steps completed)`. The published
API check limitation described in section 11 remains.

The three branch documentation corrections in section 12 remain pending
separately. This review changed the design document, not the implementation.
