# Critique of DESIGN.md revision 1

Date: 2026-09-20. Reviewer: Claude Fable 5.1 (model id `claude-fable-5-1`),
in the maintainer's Claude Code session, at Codex's request per
`/tmp/reparent-design-fable-brief.md`. Subject: `analysis/reparent-frame-value/DESIGN.md`
revision 1, byte-identical to the reference copy at
`/tmp/transforms-reparent-design-v1.md`. Implementation baseline:
`feature/reparenting` at `bb75fa9`, working tree unchanged apart from the
untracked analysis documents.

## Verdict

**Approve revision 1 with one blocking correction and a handful of
clarifications. No design change is needed.** Every behavioral claim the
document makes about the registry was checked against the branch and
held: the state transition, the error set and its precedence, the
cycle-check argument, failure preservation, the coverage and
`latest_common_time` consequences, static retroactivity, the expiry
reference, and the static-seed reconstruction guidance. The document does
not widen the feature and adds no release prerequisite. The three pending
documentation correction groups are recorded as pending, not as done.

The one blocking item is in the worked example: it does not compile as
written, because of one import path. With that path corrected, every
assertion in it passes on `bb75fa9`.

## What was inspected and run

Read in full: DESIGN.md; the current `src/core/registry/mod.rs`
(`reparent_frame`, `process_add_transform`, `creates_cycle`, `ancestry`,
`frame_exists`, `process_get_transform`, `diagnose_not_found`,
`combine_transforms`, `latest_common_time`, `get_transform_at`);
`src/core/buffer/mod.rs` (`empty_like`, `insert`, `get`, `get_nearest`,
`check_interpolation_time`, `remove_before`, `remove_expired`);
`src/core/registry/error.rs` and `src/core/buffer/error.rs` (both `From`
conversions); `Transform::validate`, `interpolate_to`, `Mul`, and
`restamped` in `src/geometry/transform/mod.rs`; the `TimePoint` impls for
`Timestamp` and `SystemTime`; and the crate-root re-exports in
`src/lib.rs`.

Run: the section 9 example, extracted verbatim into a scratch crate
outside the repository that depends on the branch by path. Verbatim it
fails to compile (E0432, below). With the import corrected it compiles,
runs to completion, and every assertion holds. No gate was re-run: nothing
implementation-facing changed, and Codex is running it.

## Blocking

**B1. Section 9: the example does not compile.** It imports
`RegistryError` from the crate root:

```rust
use transforms::{
    Registry, RegistryError,
    ...
```

`src/lib.rs` re-exports only `Registry`, `Transform`, `Localized`, and
`Transformable` at the root; the error types live in the public `errors`
module. rustc reports:

```text
error[E0432]: unresolved import `transforms::RegistryError`
  no `RegistryError` in the root
```

Replacement:

```rust
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};
```

With that single change the program compiles and all six assertions pass
on `bb75fa9`. The numbers in the example are right: the seed is
`-17` (`13 - 30`), the world pose of `item` at `t2` is `13` before and
after the move, the static `tip` edge answers at `t1`, the `t1` lookup
across the moved edge fails with `NotFoundAt { frame: "item", requested:
t1, covered: Some((t2, t2)) }`, and the `tip -> item` seed is refused with
`CycleDetected` leaving the world pose intact.

## Required clarifications

These are places where the text is either ambiguous on a point the brief
asks about, or silent on behavior a reader of a "detailed design" will
need. None changes the design.

**R1. Section 3 and section 5: say which commitments are stable.** The
document lists the two `Display` strings under "Public API" without
qualification, and section 5 calls the check order "intentional and
matches the current implementation", which leaves open whether it is a
contract. The branch answers both: the `ReparentingNotSupported` text
changes in this same minor release, so `Display` text is not a
compatibility surface; and the method's `# Errors` rustdoc says "Checked
in this order", so the precedence is documented behavior. Suggested
sentences, after the `Display` block in section 3: "The variant set and
payloads are the compatibility surface; `Display` text is not, and this
release changes one such text." And in section 5, replace "is intentional
and matches the current implementation" with "is documented in the
method's `# Errors` section and is part of the 2.2 contract". (My earlier
review noted that promising the order is a small commitment; either
promise it or soften the rustdoc, but the design should state which.)

**R2. Section 5: state that the error table is exhaustive.** Verified
against both `From` conversions: on a fresh unpinned buffer,
`Buffer::insert` cannot produce `ReparentingNotSupported` or
`ChildFrameMismatch`, and `Transform::validate` produces only the two
numeric errors, so the eight rows in the table are the complete set.
Suggested sentence at the end of the table: "This table is exhaustive:
`reparent_frame` returns no other variant, in particular never
`ReparentingNotSupported`, `NotFoundAt`, `Disconnected`, `NoCommonTime`,
or `TransformError`." Section 3 already says the first; the rest is worth
saying once.

**R3. Section 5 or 6: name the `TimePoint` operations the call performs,
and say any instant is accepted.** The brief asks about zero, negative,
and custom-clock instants, and the document is silent. Verified in
`Buffer::insert` and `remove_expired`: the call uses `T` only through
`Copy`, `Ord` (the map key and the expiry-reference comparison) and, for
a dynamic buffer with `max_age`, one `checked_sub` whose `Err` skips
expiry for that insert rather than failing the call. No `duration_since`
or seconds conversion runs. Suggested sentence: "Any `T` instant is
accepted as the seed stamp, including `Timestamp::zero()` and a
`SystemTime` before the Unix epoch. The call touches `T` only through
`Copy`, `Ord`, and, with `max_age`, one `checked_sub`; an `Err` from that
subtraction disables expiry for the insert and is not an error of this
call." (A pre-epoch `SystemTime` matters only if a later error prints it:
`as_seconds_lossy` yields NaN there, by design.)

**R4. Section 7, dynamic edges: state the lookup precedence in one line
rather than by reference.** Verified in `process_get_transform` and
`diagnose_not_found`: an unknown endpoint is reported first (target, then
source); otherwise the first buffer failure met by the walks, target-side
walk before source-side, names its frame in `NotFoundAt`; otherwise
`Disconnected`. Suggested replacement for "existing lookup error
precedence still apply": "the existing precedence applies: an unknown
endpoint first, then the first hop that fails during the target-side and
then the source-side walk, then `Disconnected`."

**R5. Section 11, witness table: two tests are missing.** Section 5's
self-reference and non-finite rows (5a, 5c) and the preservation claim
for them are witnessed by
`reparent_frame_rejects_a_self_referential_or_non_finite_seed`, which the
table does not list; add it to the "Validation precedence and flat
errors" row. Section 10's "Remove then add" alternative is witnessed by
`remove_frame_then_readding_root_reconnects_subtree`; add a row or a note.
The table is otherwise accurate: every named test exists on the branch
and covers what its row says.

## Optional editorial

**O1. Section 6, cycle argument.** Sound as written. Two words would make
it complete: the walk terminates because the pre-call graph is finite and
acyclic, and the `p == c` case is excluded from the walk's verdict and
caught by step 5c. Section 5 already says the second; the first is
implicit.

**O2. Section 7, `max_age` paragraph.** "Samples older than that reference
minus `max_age` expire" is right; the boundary is retained (verified:
`oldest >= threshold` keeps). Adding "strictly older; a sample exactly
`max_age` old is retained" matches the existing `with_max_age` rustdoc
("the boundary is inclusive").

**O3. Section 9, prose before the example.** "shows descendant history
surviving": the descendant edge `item -> tip` is static, so the example
shows the descendant edge surviving, not a history. "shows the descendant
edge and its data surviving" is exact. The dynamic-descendant history
case is covered by `reparent_frame_moves_the_whole_subtree`, which
section 11 cites.

**O4. Section 9, exactness note.** The `assert_eq!` on whole `Transform`
values relies on identity-rotation arithmetic (`inverse`,
`rotate_vector`, the quaternion product) and small-integer translation
sums being exact in the current implementation. That is true today and
the crate's bit-for-bit tests lean on it, but it is an arithmetic
property, not a contract. The existing sentence "General computed
geometry uses tolerant comparisons" covers the reader; a clause saying
"exact here because the operands are small integers and identity
rotations" would prevent anyone reading the example as a composition
exactness promise.

**O5. Section 8, static seed reconstruction.** Verified and sound:
`process_get_transform` restamps every result with `Stamp::At(requested)`
(the same holds for `get_transform_at`), so forwarding a lookup result to
a static frame fails with `StaticDynamicConflict`, and
`Transform::static_between` is the route. One clause could note that
`static_between` re-validates, so a rotation that has drifted past the
tolerance over a long chain is refused there with `NonUnitRotation`; the
text's "passing constructor validation" already implies it.

**O6. Section 4.** "A static lookup carries the requested timestamp ...
it does not return `Stamp::Static` merely because storage is static" is
verified (`restamped` at the end of `process_get_transform`). Acknowledged
as sound; nothing to change.

## Sections checked and found accurate without objection

- Section 1: the two consequences of remove-then-add are stated correctly,
  including that the re-add decides kind from the seed and so accepts a
  kind change silently.
- Section 2: scope and exclusions match the joint agreement; the Non-Goals
  need no change.
- Section 3: signature, ownership on error (the transform is consumed),
  no new bounds, both variants and their payloads, the revised
  `ReparentingNotSupported` text, no return payload.
- Section 4: every row of the transition table, including `Some(s)` as the
  new expiry reference (verified in `insert`: `None` becomes `Some(s)`),
  the unchanged buffer count, and the known-frame-set changes.
- Section 5: the precedence matches the code line by line, including that
  `ParentUnchanged` precedes the cycle check, that `validate` checks
  finiteness of all seven components before the norm (so 5a before 5b),
  and that self-reference reaches `Buffer::insert` because the walk from
  `c` never returns to `c` in an acyclic tree.
- Section 6: the six-step sequence is the code; `empty_like` copies only
  kind and `max_age`; preservation on `Err` holds because the only
  mutation is the final map insert; the complexity and memory statements
  are right, including the root-diagnosis scan.
- Section 7: `[s, s]` coverage; the `NotFoundAt` payload; no monotonicity
  guard; the expiry reference tracking; static retroactivity; the
  `latest_common_time` and two-time paragraphs; the identity rule.
- Section 8: pose responsibility, the lock guidance, the absence of an
  expected-old-parent parameter, and the ingestion warning.
- Section 10: every alternative row, and the cleanup and drained-buffer
  paragraph.
- Section 11: the additive claim (one method, two variants on a
  non-exhaustive enum, revised text), the serde statement, the honest
  note that the installed semver checker could not read this nightly's
  rustdoc output.
- Section 12: the three groups are exactly the three my review found,
  recorded as pending, with the `# Errors` bullet correctly excluded.

## Summary for Codex

One fix: the example's `RegistryError` import must be
`transforms::errors::RegistryError`. Five clarifications (R1 to R5), each
one or two sentences. Six optional edits. No disagreement with the design
itself; revision 1 describes the branch as it is.

## Final verdict on revision 2, 2026-09-20

Codex's response (`/tmp/reparent-design-fable-response.md`) and the
revised DESIGN.md were read in full. The verification was done on the
document's actual text: a `diff` of revision 2 against the preserved
revision 1 reference, each hunk checked against the branch, not against
the status table in section 13.

**No blocking or required findings remain. Revision 2 accurately
represents the agreed minimal contract and the implementation at
`bb75fa9`.**

### Resolution check, item by item

- **B1, resolved.** The import is now `transforms::errors::RegistryError`.
  I extracted the reformatted section 9 example verbatim from revision 2
  into the scratch crate and ran it: it compiles and exits 0 with every
  assertion holding. The four public paths added to section 3 all exist
  (`transforms::time::TimePoint` is re-exported from `time/traits.rs`).
- **R1, resolved, and more precisely than my suggestion.** Section 3 now
  separates variant names and payloads (compatibility surface) from
  `Display` text (diagnostic). Section 5 makes the topology checks and
  their precedence over seed insertion the contract, and rows 5a to 5d
  "the current order of the shared storage checks". That split matches
  the method's rustdoc exactly: its `# Errors` promises the order of the
  topology bullets and lists the four seed errors as one unordered group.
  (Observation, no action: that group's textual order puts
  `NonUnitRotation` before `NonFiniteValues`, the reverse of the actual
  check; since the rustdoc does not promise an order inside the group,
  nothing is wrong, and the pending documentation pass may leave it.)
- **R2, resolved.** Section 5 states the table is exhaustive and names the
  five variants the method never returns, with the non-exhaustive-enum
  reminder.
- **R3, resolved.** Section 6 now states that every representable `T`
  instant is admissible, including `Timestamp::zero()`, negative instants
  on a custom clock, and a pre-epoch `SystemTime` under `std`; that the
  operation touches `T` only through copying, ordering, and, when seeding
  a dynamic buffer with `max_age`, one `checked_sub` whose `Err` skips
  expiry rather than failing the call; and that no `duration_since` or
  `as_seconds_lossy` runs inside the operation. Verified against
  `Buffer::insert` and `remove_expired`.
- **R4, resolved; the refined wording is correct and better than mine.**
  Checked clause by clause against `process_get_transform`,
  `get_transform_chain`, and `diagnose_not_found`: identity returns before
  any diagnosis; unknown endpoints are checked target then source; the
  first buffer failure recorded during the target-side walk and then the
  source-side walk is the one reported; `NoTransformAvailable` and
  `OutOfRange` become `NotFoundAt`, while `GetError::Interpolation`
  propagates its cause through `From<TransformError>` (so a
  `TimestampError` arrives wrapped in `TransformError`, and the two
  numeric variants arrive flat); with nothing recorded the diagnosis is
  `Disconnected`. The clause "a successfully resolved connecting path can
  ignore a recorded failure above its common ancestor" is true: when both
  walks stop at the same frame because the hop above it fails, the
  `connected` check passes and the chains combine. The post-combine
  frame-mismatch branch also routes through `diagnose_not_found`, which
  the phrase "reaches topology/sampling diagnosis" covers. The closing
  sentence that this is not a global ordering over geometry and time
  errors is right, since `combine_transforms` failures return directly.
- **R5, resolved.** Both witnesses are in the section 11 table under
  accurate row labels; the names match the source.
- **O1 to O5, applied**: finite-and-acyclic termination; the inclusive
  retention boundary ("strictly older", boundary sample retained,
  matching `oldest >= threshold`); "the descendant edge and its data";
  exactness scoped to this example; constructor validation of a derived
  static pose. O6 needed no change.
- **Two additions not requested, both accurate:** a rejected proposal
  whose new parent was unregistered leaves that name unregistered, and no
  new frame-name rule or reserved root name is introduced.

### Scope and status

Revision 2 changes no behavior and no API. It adds no release
prerequisite. Section 12 still records the three branch documentation
correction groups as pending, which is correct: the branch at `bb75fa9`
has not been modified. Nothing in this critique or in the design
authorizes a merge, tag, or publication.

### What was run for this verdict

`diff -u` of revision 1 against revision 2; a verbatim extraction and
`cargo run` of the revision 2 example against the branch by path (exit
status 0); `grep` for the `TimePoint` re-export; a re-read of
`process_get_transform` lines 830 to 895 and `diagnose_not_found` for
the R4 wording. No source, test, or gate changes; no gate re-run, since
nothing implementation-facing changed.
