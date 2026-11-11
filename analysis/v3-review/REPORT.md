# transforms v3: review and proposal, round 2

Date: 2026-11-10. Amended 2026-11-11: [Examples in 3.0](#examples-in-30),
one condition of item 7, one bullet under "Ships in 2.x", decisions 13 to
15 and two Prototypes rows; then
[item 12](#12-remove_transforms_before-removes-the-sample-at-the-cutoff),
which the maintainer decided that day, with its effect on item 10's
recovery text. The PDF and `report-source.html` carry the
amendment in summary form. Baseline: master `7cc2a65` (released v2.3.0 plus docs-only
commits). Nothing here is implemented. Everything is a recommendation,
except [item 10](#10-an-insert-the-window-discards-is-an-error), whose
semantics the maintainer settled in discussion the same day.

This is the second full pass. The first is kept in
[`round1/`](round1/REPORT.md), unchanged apart from its relative links.
Round 2 repeated the review from scratch
with reviewers that could not see round 1, took the item 10 decisions as
fixed input, and then tested a new draft against prototypes and critics
that could see both rounds. [Changes from round 1](#changes-from-round-1)
lists every difference.

Inputs: the v3 notes in [`analysis/v3-proposals/`](../v3-proposals/), the
ledger in [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md), the
dependent study in [`analysis/dependents/`](../dependents/README.md) and the
[2.4 proposal](../v2.4-proposal/DESIGN.md).

## Verdict

- **Round 2 confirms round 1's ten items in substance and changes one
  recommendation.** `inverse` should keep normalizing (item 3). The
  conjugate closes no hazard, changes the last bits of about half of all
  lookup results, and makes one existing property test fail in about one
  run in ten.
- **One item is new:** delete `TransformError::SameFrameMultiplication`
  (item 11). It is safe only together with item 1. A twelfth item, the
  inclusive cleanup cutoff, was decided by the maintainer after the
  review (2026-11-11).
- **Item 10 is settled and has an implementation trap.** All ten round 2
  reviewers found it independently: the obvious implementation breaks the
  atomicity of `add_transforms`. Round 1's prototype did not have the bug.
  Round 1's recovery wording and its description of the `newest` field
  were wrong or incomplete, and are corrected here.
- **Item 1 has live evidence now.** In roslibrust_transforms a cross-time
  result passed to its `add_transform` is accepted, overwrites an honest
  sample and is broadcast (probe on 2.3.0).
- **The size is unchanged:** one new public type and one new error
  variant, while the deletions alone take the public surface from 104
  named items to 87. The four prototypes together are roughly +420/−680
  library lines; stacked and cleaned of the files the deletions orphan,
  +416/−793 ([Examples in 3.0](#examples-in-30)).
- **Known compile cost downstream:** one return type, one re-export and
  three test lines in roslibrust_transforms; one match arm in transforms_io
  if the optional rename is taken; nothing in dimos, pelorus or whirl_tf.
  The real cost is behavioural and is almost all item 10; item 12 adds
  one more silent row, which no studied dependent reaches.
- **Four questions are open and are the maintainer's** (three more, on
  the examples, were added on 2026-11-11 as decisions 13 to 15): the Point family's
  removal path, frame fields on insert errors, where rotations are
  normalized, and whether the finiteness check is a 2.x fix. See
  [Decisions](#decisions-for-the-maintainer).

## Method

- **Lens review (20 agents).** Ten lenses, one reviewer and one skeptic
  each: host performance, embedded and scalar, geometry safety, temporal
  safety, ergonomics, errors, v2 defects, dependents, prior art, migration
  and release. Reviewers were told not to read `analysis/v3-review/` and
  were given decisions D1-D6 of item 10 as settled. Each ran probes
  against the crate. Statuses used here are the skeptics'.
- **Stress test (10 agents).** A draft built from the lens review was
  given to four prototype builders (item 10; items 2-4; items 1 and 5; the
  deletions) and six critics (scope and minimalism, alternative designs,
  dependent cost from real sources, the item 10 contract, consistency
  against code and round 1, public demand). These could read round 1 and
  were asked to rule on every point where the rounds differ.
- **Models.** Every round 2 agent ran `claude-fable-5-2` at medium effort
  (verified from the transcripts). Round 1 ran on Sonnet 5.5.
- **Not done.** See [Limits](#limits). The gate ran on no prototype.

## The proposed 3.0

Kinds: **loud** means the compiler reports it. **Silent** means code
compiles and behaves differently. **Wire** means serialized data is
affected.

| # | Change | Kind | Status against round 1 |
|---|---|---|---|
| 1 | `get_transform_at` returns `CrossTimeTransform` | loud | Kept; error payload rule added |
| 2 | Rotations normalized where built | silent, wire | Kept; bound now proven conditionally |
| 3 | `inverse` keeps normalizing; the `TransformError::QuaternionError` variant deleted | loud | **Changed** (round 1: conjugate) |
| 4 | Lookups never return a non-finite value | silent | Kept |
| 5 | `Disconnected` before `NotFoundAt` | silent | Kept; two error classes, not one |
| 6 | Error shapes | loud | Kept; frame fields reopened |
| 7 | Delete `Point`, `Transformable`, `Localized`, `get_transform_for` | loud, wire | Kept; cost now stated |
| 8 | Delete `Transform::interpolate` | loud | Kept |
| 9 | Delete `Timestamp::as_seconds` | loud | Kept |
| 10 | Expired insert returns `SampleExpired` | silent | Settled; contract added |
| 11 | Delete `SameFrameMultiplication` | silent | **New** |
| 12 | `remove_transforms_before` removes the sample at the cutoff | silent | **New**, decided 2026-11-11 after the review |

### 1. A cross-time result type

`get_transform_at` returns `transforms::geometry::CrossTimeTransform<T>`
instead of a `Transform`. No crate-root re-export.

- **Shape.** Private fields. Accessors `target_frame()`, `target_time()`,
  `source_frame()`, `source_time()`, `translation()`, `rotation()`. No
  `Mul`, `inverse`, serde, conversion into `Transform`, `fixed_frame`
  field or apply method. `Clone` and `PartialEq` (exact over all six
  fields) are optional and additive later. The method keeps its name and
  argument order.
- **Evidence (new).** roslibrust_transforms wraps `get_transform_at` and
  also inserts then publishes. On 2.3.0,
  `add_transform(get_transform_at("odom", t2, "base", t1, "map"))` returns
  `Ok` and replaces the honest `odom <- base @ t2` sample (x: 1 to −4).
  The re-insert succeeds only when the target is the source's pinned
  parent; any other target fails with `ReparentingNotSupported`.
- **What the type does and does not close.** It removes the accident: the
  result no longer inserts or composes. It does not remove the capability:
  one `Transform::new` expression rebuilds an insertable transform from
  the accessors. Today that takes zero lines.
- **Why not delete the method.** The outside replacement is two
  single-time lookups plus a restamp of one leg through a public
  constructor, which defeats the equal-stamp guard. That is a wrong-answer
  generator, so the scope rule keeps the method in the core.
- **Error rule (new; round 1 left it open).** Both legs are resolved. On
  failure the reported error is chosen by rank `UnknownFrame` >
  `Disconnected` > other, target leg first within a rank. `NotFoundAt`
  carries the caller's `target_frame` and `source_frame`, with the failing
  leg's frame, that endpoint's requested time and its covered range.
  `Disconnected` names (endpoint, fixed frame), the endpoint in the field
  of its own role. About 40 code lines in the prototype.
  - Without this rule, item 5 does not reach this method: the source leg
    is resolved first and its error returned, so an unknown target is
    reported as `NotFoundAt` or `Disconnected` depending on `source_time`.
  - It changes which leg is reported when both fail (source today, target
    after).
  - One blemish: in `Disconnected` one field holds the fixed frame under a
    target or source field name. Rewriting to (target, source) would be
    false when those two are connected and the fixed frame is not.
- **Four label accessors or two.** `translation()` and `rotation()` alone
  are sufficient for conversion, and the other four return the caller's
  own arguments. They are kept because they let a checked application be
  built outside once the value leaves the call site, which is what keeps
  application out of the core (item 7). This is a judgement call.
- **Cost.** roslibrust_transforms changes one return type (`lib.rs:562`),
  adds one re-export and edits three test lines; `cargo check --lib`
  against the prototype gave exactly that one error. Its `FromTransform`
  trait cannot convert the result to a ROS message. In this repository 12
  lines in five tests and one doctest stop compiling.
  `cross_time_result_keeps_only_target_stamp_and_remains_insertable` is
  retired, not migrated.
- **Measured.** 43,427 `get_transform_at` calls: all 2,060 `Ok` results
  are bit-identical to 2.3.0.

### 2. Rotations are unit where they are built

`Transform::new`, `static_between` and `Deserialize` normalize the rotation
after the existing tolerance check.

- **The rule.** Acceptance stays on the norm (within
  `UNIT_NORM_TOLERANCE`). Then, if the squared norm differs from 1 by more
  than `8 * f64::EPSILON`, every component is multiplied by
  `1 / libm::sqrt(n²)`. The step is written as plain arithmetic: no `?`,
  no fallback arm. It is bit-identical to `Quaternion::normalize` on
  3.4e9 accepted values.
- **The hazard is the accepted band, not drift.** A rotation up to 1e-6
  off unit norm is stored as given, and applying it scales by the squared
  norm, up to 2e-6 per hop.
  - Four hops of 10 km at `w = 1 + 0.9e-6`: 0.108 m forward error.
  - Ten hops of 10 m at norm 1 ± 9e-7: 0.81 mm at 100 m.
  - The same frame answers with different scale at a stored instant and
    between samples, because slerp changes the norm (`w = 1 − 9e-7`,
    identical samples: norm − 1 is −9.0e-7 at a sample and −4.5e-7
    midway).
- **Reach.** A unit quaternion rounded to `f32` is off by at most 5.0e-8
  in norm (200,000 random samples). That is inside the tolerance and
  outside the band: the rule changes 200,000 of 200,000 `f32`-sourced
  rotations and 0 of 200,000 `f64`-normalized ones. dimos shows the effect
  today: it admits |n² − 1| ≤ 1e-6 at ingest and re-validates every lookup
  result, and a three-hop forward lookup at n² = 1 ± 9.9e-7 fails that
  re-check.
- **The band is required and is now bounded.** A second normalization
  changes the bits of 38% of once-normalized random quaternions, so an
  unconditional rule is not a fixed point. After one normalization the
  squared norm is within 7 EPS of 1, proven on the assumption that
  `libm::sqrt` is correctly rounded, and within 8 EPS if it is only
  faithfully rounded; the test is `> 8 EPS`, so both suffice. Correct
  rounding is tested, not proven (0 mismatches against hardware `sqrtsd`
  in 6e9 random values and every float within 2^23 ulps of 1). Observed
  maximum: 4 EPS over 3.4e9 samples. Rebuilding a built value changed 0
  bits in 3.4e9 values.
- **Where to normalize (open).** The scope critic prototyped the same
  normalization at `Buffer::insert` only. It closes the lookup hazard,
  flips one test instead of three, and needs no constructor or decode
  change. It leaves a built, never stored transform off-unit (1.8 mm per
  km when applied by hand) and stores something other than what a caller
  inserted. The constructor site is recommended: with item 7 application
  happens in caller code on built values, and "stored equals built" keeps
  the single-hop bit-for-bit guarantee simple.
- **Declared silent changes.**
  - For a rotation outside the band, `tf.rotation()` no longer equals the
    quaternion passed in, and lookups through it change by up to 2e-6
    relative per hop. In-band inputs change nothing: 0 result bits in
    100,000 forest lookups.
  - A flattened product of two band-edge transforms is now accepted on
    insert, where 2.3 returns `NonUnitRotation`.
  - `Deserialize` re-normalizes a derived result whose norm drifted past
    the band (0.2% of 10-hop and 23% of 64-hop lookup results), so
    serialize-then-deserialize is not bit-exact for those.
- **Residual.** `Buffer::insert` validates and does not normalize. A
  re-published derived rotation within 1e-6 is stored as given.
- **Test notes.** Three existing tests flip, all intended. A rewritten
  fixture must not pin `1.0`: `new(w = 1 − 9e-7)` stores
  `0.9999999999999999`. Pin the fixed point over postcard or through
  `Transform::new`, not JSON: `serde_json` with default features parses
  `0.9999999999999999` as `1.0`, on master too.
- **Alternatives rejected** (unchanged from round 1): normalizing only in
  composition, a scale-invariant `rotate_vector`, a public `Rotation`
  newtype, a tighter tolerance, a Newton step per composition.

### 3. `inverse` keeps normalizing, and the `QuaternionError` variant goes

Round 1 proposed the conjugate. Round 2 reverses that.

- **Recommendation (b).** Keep the normalization in `Isometry::inverse`,
  written without `?`. Report a non-finite norm as `NonFiniteValues` and a
  norm below EPSILON as `NonUnitRotation(norm)`. Delete
  `TransformError::QuaternionError`. `Transform::inverse` stays fallible.
  The prototype reports both cases as `NonUnitRotation(norm)`; decide the
  mapping when it is built. `NonFiniteValues` is the existing spelling of
  a non-finite rotation everywhere else.
- **Why not the conjugate (a).**

  | | Master | (a) conjugate | (b) keep normalizing |
  |---|---|---|---|
  | Lookup results with changed bits, unit inputs | n/a | 46.5% | 0 |
  | `registry_chain_roundtrip_composes_to_identity` fails | 1 in 3e6 cases | 0.044% of cases, about 10.6% of runs | as master |
  | `inverse()` of a rotation collapsed by `*` | `Err` | `Ok` | `Err` |
  | One-hop inverting lookup, host | 78.1 ns | 61.1 ns | 79.6 ns |

  (a) closes no hazard. It saves about 17 ns per inverting lookup on host.
- **The numeric argument for (a) is sound.** A lens claimed the conjugate
  needs a Newton step in composition. It does not: for a squared norm of
  1 + d the conjugate's error is d·|p − t|, the same order as the forward
  error d·|p| the crate already has (ratio 1.0 to 2.4 for d from 1e-15 to
  1e-6). (a) stays available later as a measured performance change.
- **The variant is reachable today**, through public `*` followed by
  `inverse()`: 72 multiplications from `w = 1 + 1e-6` with a
  norm-cubing pattern, or about 3.7e7 plain products of band-edge
  operands. It is not reachable through constructors, `Deserialize` or
  registry lookups over stored data. So "delete it because it is
  unreachable" is not available; (b) gives its two cases a defined result
  instead.
- **Dissent.** The consistency critic ranks (b) lowest because it needs
  that rule for a zero norm, and treats (a) against (c), the status quo,
  as a maintainer call.

### 4. No lookup returns a non-finite value

- One finiteness check at the exit of `get_transform` and
  `get_transform_at`, reporting the existing `NonFiniteValues`. +19/−0
  lines; cost not distinguishable from zero on host.
- On master these return `Ok` with `inf` or `NaN`: ancestor-ward two hops
  of 1e308, sibling lookups, three `get_transform_at` cases,
  `get_transform_for`, interpolated, rotated (−inf, inf, NaN) and static
  chains, and a chain whose true result is 5e307. Each is
  `Err(NonFiniteValues)` under the check. A single hop at `f64::MAX`
  stays `Ok`; a disconnected pair with overflowing geometry stays
  `Disconnected`.
- **Not on `*`.** Round 1's prototype gated `*` as well. That gate fails
  `add_transforms_matches_the_add_transform_loop_or_changes_nothing` at
  `tests/properties.rs:141` and closes no lookup hazard.
- One more existing test flips
  (`an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts`).
  Docs that become false: `src/core/registry/mod.rs:281-285`,
  `src/core/registry/error.rs:37-43`, `MIGRATION.md:195`.
- It changes a documented behaviour, so it is listed for 3.0. It changes
  no number. A maintainer ruling could move it to 2.x, as D5 did for
  empty names.

### 5. `Disconnected` before `NotFoundAt`

- Precedence becomes `UnknownFrame` > `Disconnected` > `NotFoundAt` for
  lookups. `latest_common_time` already decides this way.
- **Reason, corrected.** The error variant must not depend on the
  requested time when the cause is topology. Round 1 wrote that waiting
  "never will" help a disconnected pair; that is wrong, since a later
  insert can connect two trees.
- **The change** is deleting the `walk_failure` fallback and its plumbing:
  +41/−66 library lines. One test flips. Two in-crate unit tests that call
  the private `get_transform_chain` need a mechanical edit.
- **Two error classes change, not one.** In 144,000 queries over two
  clocks, 30,294 differ, all for disconnected pairs: `NotFoundAt` to
  `Disconnected`, and with a custom clock
  `TransformError(TimestampError(..))` to `Disconnected` (3,160 cases).
  Zero differences for connected, same-frame and unknown-frame lookups and
  for `latest_common_time`.
- **`get_transform_at` needs the cross-leg rule of item 1** to get the
  same property.
- Stale docs: `src/core/registry/mod.rs:276-278` and `:456`,
  `src/core/registry/error.rs:137-140` and `:162-163`,
  `MIGRATION.md:136-137`.
- **The resolver rewrite is not a release item.** It is internal and can
  land in any 3.x. Round 2 measured the same regression round 1 did
  (64-hop dynamic chains 8-20% slower). If taken: after this item, as two
  commits (the fold gated on identical output including errors, then the
  deletion), and only if root-ward lookups do not get slower.

### 6. Error shapes

- **`TimestampMismatch` becomes a unit variant.** Its `f64` seconds
  payload prints equal values for stamps 1 ns apart. `Mul` takes its
  operands by value, so a caller that wants the stamps in its report reads
  them before the call. `TransformError<T>` stays rejected: it breaks
  roslibrust's public conversion trait at three sites.
- **`#[non_exhaustive]` on struct variants**, so a field can be added in
  a minor. No dependent breaks (the one struct pattern in any user already
  has `..`). `tests/tests.rs:133` stops compiling. The scope critic would
  cut this because no field is waiting; it is kept because it is the
  crate's existing convention and costs users nothing.
- **Frame field on the four structural insert errors (open).**
  `SelfReferentialFrame { frame }`, `CycleDetected { frame }`,
  `StaticDynamicConflict { frame }` and the parent-pin variant
  `{ frame, current_parent }`, child frame only, printed by `Display`.
  - For: after item 10 one insert error names its frame and four do not.
    The failing member of a batch cannot be identified outside without
    giving up atomicity (the loop finds it only by committing the others;
    `Registry` has no clone or dry run). Unit to struct needs a major.
    Zero compile errors in dependents.
  - Against: a child frame names an edge, not a member. The two value
    errors stay frame-less. The only batch caller is dimos (unmerged PR,
    its own `add_transforms`), and it gains only through `Display`.
  - Critics split two for, one against, one undecidable. Round 1 said no.
    Leaning yes, child frame only. `EmptyFrameName`'s payload (2.4 D7)
    must be decided with it.
- **Optional: rename `ReparentingNotSupported` to `ParentMismatch`.** The
  one known compile break in transforms_io
  (`mt_pubsub_backend.rs:342`). Cosmetic.

### 7. Delete `Point`, `Transformable`, `Localized`, `get_transform_for`

- **Evidence.** Zero call sites in seven public code bases
  (transforms_io, roslibrust_transforms, pelorus, whirl_tf, dimos PR
  #4722, termviz2, transforms-py), by three independent search methods.
  Both end applications apply transforms in nalgebra.
- **Evidence against.** The only two outsiders who ever posted code in
  the tracker used `Point` (#43, #65/#66). The README leads with it.
  Issue #43 is a public case of `Point::transform` rejecting a
  wrong-direction application. The same threads show `Point` confusing
  its users. Private users are invisible.
- **Scope rule.** `Point::transform` reads only public accessors and
  validates no numbers. A 12-line outside function gave the same errors
  on a swapped lookup, a wrong instant and a wrong frame. Scope bullet 1
  puts it outside.
- **Declared cost.** For a caller that applies by hand, seven slips
  become silent for a position (swapped lookup, other instant, other
  frame, applied twice, three hand-math slips), plus orientation order
  for a pose. For a pose written as a `Transform`, `*` still checks
  frames and stamps (two allocations, a fallible constructor, operands
  consumed). Round 1 did not state this cost.
- **Conditions.**
  - Move the application contract (position `R p + t`, orientation
    `R * q`, free vectors take only the rotation) from the
    `Transformable` rustdoc, its only normative statement
    (`src/geometry/transform/traits.rs:46-53`), to `Transform`'s rustdoc,
    with a recipe that asserts `child()` and the stamp.
  - Re-point `tests/golden_vectors.rs:99-106,205-212` at the accessor
    formulas with the same literal digits, never regenerated. The layer
    then pins `rotate_vector`, the quaternion product and lookup
    direction, and no longer a crate application path.
  - Rewrite `README.md:44-68` and two examples (one, `basic.rs`, if the
    2.4 examples restructure lands first; see
    [Examples in 3.0](#examples-in-30)).
  - Give 2.x users notice. Either `#[deprecated]` with a feedback link, a
    cut-off date and an adoption threshold (about 17 in-repo files need
    `#[allow(deprecated)]`, which needs maintainer sign-off), or a removal
    notice in the 2.4 changelog and README. Reach is small either way:
    2.3.0 has 181 downloads in 13 days, and 1.x pins never see it.
- **Options not taken.** (b) delete only `get_transform_for` and
  `Localized`: −79 library lines against −369, keeps the checked route,
  but `Point` is unvalidated with public fields, so "the checked path"
  would need hardening, and a cross-time result cannot be applied through
  `Transformable` correctly. (c) a positions-only `apply` method:
  buildable outside. The deletions prototype leans to (b) unless the cost
  above is accepted, and the demand critic wants a 2.x deprecation before
  `Point` and `Transformable` go; the scope, alternatives and consistency
  critics rule (a).
- `Point`'s serde wire format stops decoding. 10 tests are deleted and 6
  re-pointed.

### 8. Delete `Transform::interpolate`

- No registry path calls it and no user was found. It delegates to
  `Isometry::interpolate`, so it is not a second arithmetic path. Library
  +13/−127; two error variants go (`StaticInterpolation`,
  `TimestampOutOfRange`).
- **The test oracle becomes an in-test reference** (12 lines: linear in
  translation over exact nanosecond spans). A crate-internal route cannot
  be reached from `tests/properties.rs`. Today's oracle is blind to
  arithmetic mutations: with the lerp weights swapped in
  `Isometry::interpolate`, the existing property still passes; the
  in-test reference fails.
- The `AGENTS.md` `TimestampOutOfRange` sentence and `MIGRATION.md:192`
  become false.

### 9. Delete `Timestamp::as_seconds`

- `Err` for every stamp above 2^53 ns, about 104 days. `as_seconds_lossy`
  stays. Library +1/−40.
- `TimeError::AccuracyLoss` stays for custom clocks and loses its only
  crate producer; reword its doc in the same commit. No public clock was
  found producing it.

### 10. An insert the window discards is an error

Three round 1 lenses ranked the silent drop as the most serious defect
outside the ledger; round 1's stress test turned the fix down; the
maintainer reinstated it and settled its semantics in discussion on
2026-11-10. Round 2 took those decisions as input and worked out the
contract. Draft rustdoc, the `MIGRATION` row and the `AGENTS.md` bullet
are in [`sample-expired-contract.md`](sample-expired-contract.md).

**Settled by the maintainer**

| | Decision |
|---|---|
| D1 | On a `with_max_age` registry, an insert strictly older than the frame's newest stamp minus `max_age` returns `RegistryError::SampleExpired { frame, stamp, newest }` and stores nothing. The boundary is the eviction boundary: a sample exactly on it is accepted |
| D2 | `add_transforms` fails as a whole and stays equal to the insert loop. Each member is measured against its frame's running newest stamp, earlier members of the batch included. With a 10 s window, `[50 s, 100 s]` is accepted and `[100 s, 50 s]` is rejected |
| D3 | No `max_age`: never reported. Statics unaffected. Where the subtraction is not representable, the sample is accepted, as in eviction. The `reparent_frame` seed is exempt |
| D4 | The batch-newest alternative (independent of order) is rejected: the batch would stop equalling the loop, which the rustdoc promises and `tests/properties.rs:640` pins |
| D5 | No `clear_history` |
| D6 | 2.x keeps the silent drop. The sample that moves the window ahead is still accepted, and a jump smaller than `max_age` still interleaves silently; the error is not reset detection |

**What round 2 adds**

- **The trap.** Putting the D1 test inside `Buffer::check` against the
  live buffer breaks D2 on frames the registry already holds:
  `[at(100), at(50)]` stores 100 and then fails on 50. The check phase
  needs a per-frame running newest stamp. The existing batch property
  rarely notices (2 of 40 runs; it pins `cases: 256` in its config, so
  `PROPTEST_CASES` does not raise it). A biased property caught it in 40
  of 40 runs.
- **Round 1's prototype did not have this bug.** It passes all 15 new
  tests and 100,000 property cases. It differs from the round 2 prototype
  in other ways: it adds `clear_history`, changes `Buffer::check`'s
  signature, and documents `newest` as stored.
- **Check order.** Expiry is the last insert check: cycle (new frames),
  numeric, self-reference, parent pin, kind, expiry. An expired sample
  with the wrong parent reports `ReparentingNotSupported`. No batch
  property catches a swapped order; only deterministic tests do. Because
  expiry is last, a sample that reports it has the frame's parent and
  kind. Keep the order as an internal rule and tests; publishing a
  precedence among insert errors is a new commitment with no requester.
- **Placement.** `InsertError<T>` at buffer level, with a separate
  `Buffer::check_expiry(transform, newest)` called last by
  `Buffer::insert` and by the batch check phase. `Buffer::check` keeps its
  signature and its rustdoc ("none of the checks reads the stored
  samples"). +118/−6 library lines. A registry-level placement is smaller
  (+77/−1) and leaves `Buffer::insert` a silent dropper behind three
  paired call sites.
- **One threshold.** Use the eviction's own `checked_sub` form. A
  `duration_since` form silently stops rejecting for a clock whose
  `duration_since` fails on ordinary spans (the Picos test clock).
- **What the payload means.** From a batch, `stamp` belongs to the first
  failing member in order, which need not be the culprit, and `newest`
  can be the stamp of an earlier member that the registry does not hold
  after the error (`[poison, legit]` reports `stamp: legit, newest:
  poison`). More than one member can carry the same `(frame, stamp)`.
  The error does not say which stamp is wrong: a late sample and a frame
  whose newest stamp is ahead of its stream look the same.
- **`Ok` from a batch** means every member was stored at its turn; a
  member that a later member evicts or replaces is not in the registry
  when the call returns, as after the same sequence of `add_transform`
  calls.
- **Do not advise "drop the named member and resubmit".** It ends in the
  same state as the loop (poison committed), costs a batch clone per
  attempt, and repairs nothing.

**Recovery, by cause**

Recover only when `newest` is judged to be the bad stamp, which needs a
clock the crate does not have. Applied mechanically, one late sample wipes
a healthy frame. The error does not return the transform.

| Cause | Route | Notes |
|---|---|---|
| One frame's newest stamp is wrong | `remove_frame(frame)`, then re-add a sample the caller kept | Works for every time type and every stamp. Releases the parent and kind pins until the re-add; the rejected sample already has both. Lookups through the removed edge report `Disconnected` or `UnknownFrame` until then; the subtree below stays intact |
| The clock stepped back (bag loop, sim restart) | `remove_transforms_before(c)` with `c` at or after every stored dynamic sample: the newest stamp the caller tracked | Drains every dynamic frame, keeps statics and pins. The cutoff removes the sample at `c` (item 12), so this works for every time type and at the type's maximum. With `c` taken from a `SampleExpired` payload it drains that frame and removes everything at or before `c` in the others; their later samples stay. In 2.x the cutoff is strict and `c` must be later than every sample |
| Everything | Replace the `Registry`, or loop `remove_frame` over child names the caller tracked | The registry cannot enumerate frames. A new registry also drops statics. whirl_tf already does the loop |

Two `reparent_frame` calls through a throwaway parent also clear one
frame. Keep that out of the rustdoc: it needs a fabricated transform,
contradicts the method's own guidance, and its second seed is unchecked.
Round 1 named it as a recovery.

**Consequences to document**

- A zero window rejects every out-of-order sample
  (`5,4,5,6,5,6,7` gives `Ok,Err,Ok,Ok,Err,Ok,Ok`).
- After a backward jump larger than `max_age`, the errors stop when the
  stream reaches newest minus `max_age`, and lookups then interpolate
  across the two epochs (jump from 1000 s to 100 s with a 10 s window: 890
  errors, first accepted sample at 990 s, a lookup at 990.5 s blends
  epochs).
- A sample stamped ahead by less than `max_age` is never reported: later
  samples are accepted and lookups interpolate toward it (10 s window,
  sample 7 s ahead: lookups up to 3.47 m off, all `Ok`). A maximum
  interpolation gap is the mechanism that would close this; see
  [Not in 3.0](#not-in-30).
- A `reparent_frame` seed stamped ahead of the live stream makes later
  samples fail until the stream catches up.
- `Display` prints both stamps as `f64` seconds and inherits the 1 ns
  ambiguity of the other time-carrying errors.

**Dependents**

- Zero compile errors anywhere.
- transforms_io and dimos expose no recovery call; roslibrust exposes
  only `remove_frame`. Without per-adapter recipes in `MIGRATION` the
  practical result is a permanent error-level log stream where 2.x was
  silent.
- transforms_io and roslibrust log each rejection while holding the
  registry write lock. An insert-then-publish wrapper stops publishing
  late samples. In roslibrust the window is also the caller's wait
  timeout, so a short timeout makes ordinary late arrival an error.
- One expired member rejects a whole TF message in dimos, where 2.3
  stores the rest.

**Evidence**

- Exactly three existing tests fail, all pins of the old contract
  (`src/core/buffer/tests.rs:639`, `:682`, `tests/properties.rs:715`). 36
  doctests and all nine `v2_compatibility` tests pass.
- Differential trace: 122 of 1,911 insert lines flip from `Ok` to
  `SampleExpired`; 0 other differences in 278,880 lines.
- Cost: a rejected insert 63.6 ns against 64.3 ns; the batch scratch map
  adds 0.1-0.35 µs per 16-member batch under `max_age`, nothing without.
- No public report of dropped late samples exists for this crate or its
  dependents. tf2's `TF_OLD_DATA` uses the same boundary. tf2 users ask
  for a whole-buffer reset on a backward jump and for fewer warnings,
  never for the late sample to be accepted; automatic clearing has its
  own failure record (ros/geometry2#341).

### 11. Delete `SameFrameMultiplication`

New in round 2.

- `ensure_composable` rejects equal children before it tests
  `lhs.child == rhs.parent`. The guard's only effect on `Ok`/`Err` is to
  reject a right-hand self edge, including the registry's own
  `get_transform(x, x)` identity: `get(a, a) * get(a, c)` is `Ok`,
  `get(a, c) * get(c, c)` is `Err`. Removing it makes the code equal the
  `AGENTS.md` composition rule. Library +7/−23.
- **Only together with item 1.** Without it a silent wrong answer becomes
  reachable on the right: `world_camera @ t2 * get_transform_at(camera,
  t2, camera, t1, world)` is `Err` today and would be `Ok` with the t1
  pose stamped t2. The same product already composes on the left today.
  `CrossTimeTransform` has no `Mul`, which removes the only library
  producer of a non-identity self edge.
- Over 324 frame pairings: 64 change from `SameFrameMultiplication` to
  `Ok` (right-hand self edges and self × self), 52 change variant to
  `IncompatibleFrames` (`Err` stays `Err`; the forgot-to-invert slip), 208
  are unchanged. No registry lookup changes (72,000-line trace
  byte-identical).
- Four tests flip, and `MIGRATION.md:199` declares the old behaviour.
- `Transform::new` does not gain a `parent == child` rule: the registry's
  identity is rebuilt through it, and `t * t.inverse()` is a valid self
  edge that is not exactly the identity. Insert already rejects self
  edges.
- No dependent multiplies transforms or names the variant.

### 12. `remove_transforms_before` removes the sample at the cutoff

Decided by the maintainer on 2026-11-11, after the review: the cutoff is
inclusive, the name stays, and the change is a MIGRATION note. This item
was not put to the lens or stress agents; one prototype and one text
inventory back it.

- **Change.** `remove_transforms_before(t)` removes every dynamic sample
  stamped at or before `t`. Today it removes those strictly before.
- **What it closes.**
  - A sample at the time type's maximum can never be removed by this
    call today. Under `with_max_age` it then anchors expiry, and every
    later sample is accepted and evicted on arrival. With the inclusive
    cutoff, three honest samples inserted after a cleanup at `u64::MAX`
    are stored and served.
  - `remove_transforms_before(newest)` keeps the newest sample today, so
    a full clear needs `newest + 1 ns`. That instant does not exist at
    the maximum and cannot be formed for a generic `TimePoint`.
  - The dimos report named this hole (`../dependents/dimos.md:561-564`)
    and counted it toward an on-request clear call (`:568-586`). That
    argument is gone; D5 (no `clear_history`) is not reopened. That
    report calls the hole loud ("the restarted stream is rejected"); it
    is not: the inserts return `Ok` and are evicted, and lookups report
    `NotFoundAt`.
- **Implementation.** One added line in `Buffer::remove_before` (and a
  `mut`): `split_off(&cutoff)` keeps keys at or after the cutoff, and
  `kept.remove(&cutoff)` then removes the one key equal to it. It needs only `Ord`, so no successor
  operation and no new bound.
- **Cost: a silent change.** Code compiles unchanged and removes one more
  sample. A caller that passes a stamp it means to keep loses it:
  - with samples at 1, 2 and 4 s and a cutoff of 2 s, lookups at 2 s and
    3 s turn from `Ok` into `NotFoundAt`;
  - a cutoff at a frame's only sample drains the frame.

  Three in-repo examples pass a stamp that a frame stores as its
  newest sample. `no_std_minimal.rs:58` and `no_std_advanced.rs:134`
  pass the newest stamp in the registry and would drain every dynamic
  frame. `no_std_full.rs:110` passes 2 s, the only sample of one frame,
  and would drain that frame; another frame's sample at 3 s survives.
  None shows a visible sign. No studied
  dependent calls the method.
- **Two boundaries, opposite on purpose.** Expiry keeps the sample
  exactly on its boundary (D1, unchanged). Cleanup removes the sample
  exactly at its cutoff. Expiry measures back from the newest sample and
  must keep it; cleanup must be able to remove it. The public rustdoc and
  the AGENTS.md bullet say "kept" and "removed" and avoid "inclusive"
  for both. Only the method's first sentence and the AGENTS.md bullet are
  drafted; a sentence contrasting the cutoff with expiry is still to
  write. `remove_transforms_before(newest − max_age)` is therefore not a
  replay of expiry: it removes one sample more.
- **The name.** "Before" now means "at or before". A rename would make
  the break loud; the maintainer chose the MIGRATION note instead. The
  rustdoc's first sentence carries the boundary.
- **Tests.** Two existing tests fail, both pins of the strict cutoff:
  `reparent_frame_then_cleanup_keeps_the_new_pin_and_kind`
  (`src/core/registry/tests.rs:3391`) and the cleanup reference model of
  the property
  `dynamic_history_matches_a_reference_map_after_upserts_expiry_and_cleanup`
  (`tests/properties.rs:729`, `>=` becomes `>`). All nine
  `v2_compatibility` tests and all 36 doctests pass. No existing test
  pins the new boundary; three are needed: a cutoff equal to a sample
  removes it, a sample at the maximum is removable, and a stream
  restarted after that is stored under `max_age`.
- **D1 to D6.** None mentions the manual cutoff, and none is
  contradicted.

### What 3.0 leaves alone

`Stamp` and its serde tags, the flat `RegistryError`, the strict tree and
`reparent_frame`, descendant-preserving `remove_frame`, no extrapolation,
`libm` everywhere, `BTreeMap` history, `latest_common_time`, the
constructors `Registry::new` and `with_max_age`, the operators, the
Non-Goals, the Non-negotiables, MSRV 1.85 and edition 2024.

The type names, module paths and constructor names of `Registry`,
`Transform`, `Vector3`, `Quaternion`, `Stamp`, `Timestamp`, `TimePoint`,
`RegistryError`, `TransformError` and `TimeError` do not change; they are
re-exported down a chain (transforms_io, then pelorus_syncio). Three
things that sentence does not cover: `TransformError` loses four variants
and one changes shape, and roslibrust returns that type from a public
trait; `get_transform_at` returns a type nobody re-exports yet; `Transform`
loses `interpolate`.

## Not in 3.0

| Item | Reason |
|---|---|
| Conjugate `inverse` | Item 3. Sound, closes nothing, changes bits. Available later as a performance change |
| Public `Rotation`, public `Isometry` | Items 2-4 close the same hazards without a new type |
| Translation bound `B` | Item 4 is exact and needs no constant. `*` reaches infinity from 1e100 inputs in 1,748 products, so the bound is not sound either |
| Newton renormalization | Not needed for lookups or for item 3. Changes the bits of 45% of two-hop products; 14-15% of a soft-float composition |
| A maximum interpolation gap | **Open, not rejected.** It is core-or-nowhere (callers cannot see sample spacing) and it is what would close the in-window future stamp of item 10. It is additive in every shape that leaves the default alone, and has no settled design. A registry-wide bound is the wrong shape: the smallest bound that serves a 1 Hz edge still passes a 0.245 m error on a 100 Hz edge. Reserve wording only: describe unbounded interpolation as the default, not a guarantee. On `with_max_age` registries the gap is already bounded by `max_age` |
| Required `max_age` | Every studied user already sets it. Round 1's reason ("bounds memory only") was wrong: eviction bounds the gap between retained neighbours at `max_age` |
| `clear_history` | Settled (D5) |
| Operators to named methods | The slip is always a compile error, never a wrong value (one error when `T` is fixed, four when the slip fixes `T`). `#[diagnostic::on_unimplemented]` on `TimePoint` puts the fix in the first error; additive, for 2.x |
| Deleting `get_transform_at` | Rebuilt outside in one expression with a restamp. Item 1 |
| `Transform::apply` on a slice | Buildable outside; wrong for free vectors |
| `Resolved` result type | Closes nothing |
| `TransformError<T>` | Item 6 |
| `RegistryError::TransformError` to `Time(TimeError)` | After items 3, 8 and 11 only `TimestampError` reaches the arm from a registry call, apart from two defensive paths. `tests/time_traits.rs:317` pins the arm. Revisit with the resolver rewrite |
| Deleting `TimePoint::checked_sub` | Refuted: eviction silently stops for a custom clock |
| Optional approx | Doubles the feature matrix. approx 0.6 is still at rc2 |
| `hashbrown` to `BTreeMap`, signed `Timestamp`, `f32` | Deferred. A concrete `f32` module stays additive |
| `FrameId`, interning | Hashing is 7-8 ns per hop |
| `get_latest`, `can_transform`, caches, typed frames, `is_transient` | Buildable on the public API, or a new source of wrong answers |
| Euler constructors, math-crate features, flat crate root, `Point` to `Pose` | Buildable outside or cosmetic |
| Removing `thiserror` | About 128 lines; unrelated to v3 |
| "3.0 changes no byte and no number" | Rejected as a rule: a major is the only place a value change can land, and items 2 and 4 close silent wrong answers. Adopted in the form "3.0 changes no number that is right today", which item 3(a) fails |

## Ships in 2.x

None of these needs a break.

- The 2.4 proposal: reject empty frame names (Part D) and the
  documentation corrections (Part C).
- Reword 2.4 Part C item 5 (2.x keeps the strict cutoff; item 12 changes
  it in 3.0 only). It prescribes a cutoff "past every stored
  sample", which has the same holes as round 1's recovery text, and it
  omits the tail of a jump larger than `max_age` (906 inserts all `Ok`, 6
  stored). Corrected sentences are in
  [`sample-expired-contract.md`](sample-expired-contract.md); the 2.4
  proposal carries them since its revision 7.
- The examples work (2.4 Part E), which the maintainer decided on
  2026-11-11 is part of 2.4: examples that assert, four task-named
  files, an `ingest` example, and `log` and `env_logger` dropped. `Point` stays in the 2.x
  examples until decision 3 is taken.
- The removal notice for the Point family (item 7).
- The `TimePoint` diagnostic attribute (stable since Rust 1.78 from
  memory; built on nightly only here).
- Docs: `Display` of time-carrying errors is lossy (a 1 ns miss prints as
  inside the covered range) and the payload is exact. `get_transform_at`
  errors may name the fixed frame; word it as "may", so 3.0 can change the
  payload. The `latest_common_time` example unwraps `.at()`, which panics
  when copied onto an all-static chain. `f64` is software on all five
  named core classes as the gate builds them; hardware only on an M7 with
  the double-precision FPU built with `target-cpu=cortex-m7`.
- Doc recipes that are exact on the public API and that adapters get
  wrong today: a "latest" lookup (transforms_io's fails on any chain with
  a dynamic hop) and a checked ROS `(i32 s, u32 ns)` to `Timestamp`
  conversion (two adapters alias or clamp pre-epoch stamps).
- Tests: add a frozen-bytes case with a non-identity rotation inside the
  8 EPS band. All three frozen rotations are `(1, 0, 0, 0)`, so a
  reordered quaternion is not caught.
- Internal: delete the `latest_timestamp` field (always the map's last
  key); drop `Buffer.child` and `InsertError::ChildFrameMismatch`.

## Examples in 3.0

Added 2026-11-11, after the two review rounds, by four agents on
`claude-fable-5-2` at low effort (two prototypes, an evidence pass, a
scope critic) and three checkers. The 2.x half is Part E of the
[2.4 proposal](../v2.4-proposal/DESIGN.md#5b-part-e-examples-that-assert-organised-by-task):
four task-named examples (`basic`, `shared_latest`, `cross_time`,
`ingest`) that assert their results and run in both feature modes. This
section is what those four look like on the proposed 3.0, written and run
against the stacked prototypes.

**Recommendation.** Restructure in 2.x, then port in 3.0. The maintainer
decided on 2026-11-11 that all of Part E is part of 2.4 (the 2.4
proposal's D9, D10, D12 and D13): asserts (E1), the four task-named
files (E2) and `ingest` (E3).

- **For.** The gate hole (no example can fail on a wrong value) closes
  now, and the dependents are on 2.x. That argument supports E1 alone;
  E2 and E3 rest on their own merit.
- **Against.** `basic` and `cross_time` are rewritten twice, and
  `ingest` too if E3 ships in 2.x. The 2.4 `basic` and `cross_time` do
  not compile on the stack; the 2.4 `ingest` and `shared_latest` do.
- **A migration record, if the port is an edit.** If the 3.0 files are
  made by editing the 2.4 files, their diff is an asserted record of the
  migration. The prototypes here were written independently, with
  different fixtures, so their diff is a rewrite. Examples are not in
  the published package, so MIGRATION would have to link the repository.
- **Effect on item 7.** Its condition "rewrite `README.md:44-68` and two
  examples" becomes "rewrite `README.md:44-68` and `basic.rs`".

**What changes in kind, not only in names.**

| Example | 2.4 form | 3.0 form | Item |
|---|---|---|---|
| `basic` | Applies through `Point::transform`; the library checks frame and instant | Applies by hand in a helper that asserts `tf.child()` and the stamp, then `rotation().rotate_vector(p) + translation()`. The checks are the caller's own | 7 |
| `cross_time` | The result is a `Transform` stamped with the target time; a comment says not to insert it | The result is a `CrossTimeTransform`. The four label accessors and `translation()` are asserted (`rotation()` is used but every fixture rotation is the identity), and the three misuses are named with their compile errors | 1, 5 |
| `ingest` | A late insert returns `Ok`, so there is no arm for it; `NotFoundAt` can hide a disconnected pair | A `SampleExpired` arm with recovery by cause; `Disconnected` no longer depends on the instant, so the lookup `match` can say what each variant asks of the caller | 10, 5 |
| `shared_latest` | `latest_common_time` then `get_transform` under one read guard | Needs no change: the 2.4 file builds and runs unchanged on the stack. The prototype here is a threads variant (see below) | none |

**Requirements on each 3.0 example.**

- **`basic`: one normative source for application.** The example's helper
  must mirror the recipe on `Transform`'s rustdoc that item 7's first
  condition asks for; a doctest is the form that cannot drift. `deletions.diff` deletes `traits.rs`, the contract's only
  home, and adds it nowhere, so on the stack the contract (`R p + t`,
  `R * q` for an orientation, rotation only for a free vector) exists
  only in the example's own comment. The helper is ten lines as formatted,
  three of them body; the formula is one.
- **`cross_time`: apply through the accessors, never rebuild.** The
  example asserts `source_frame()` and `source_time()` before applying.
  It names, in a comment, what no longer compiles: `add_transform(x)`
  (E0308), `x * y` (E0369) and `x.inverse()` (E0599). It must not build a
  `Transform` from the accessors, which would reopen the hole item 1
  closes.
- **`ingest`: `SampleExpired` is not reset detection (D6).** The arm
  separates two causes with the bridge's own clock:
  - `newest` is plausible: the sample is late. Drop and count. Resetting
    the frame here would let one straggler wipe a healthy history.
  - `newest` is ahead of the clock: that stored stamp is the bad one.
    `remove_frame`, rebuild the sample from the message and insert again.
  A clock that stepped back is detected by the bridge, not by the error,
  as in 2.x, and cleared with `remove_transforms_before` at the newest
  stored stamp (item 12; the 2.4 form passes one nanosecond past it). The two prototypes detect it
  differently, and should be made the same: the 3.0 one compares a clock
  reading carried with each message, the 2.4 one compares each stamp with
  the newest stored one against a threshold. A step smaller than the window raises no
  error, so an example that waits for `SampleExpired` would miss it.
- **`ingest`: the rebuild is a visible cost of item 10.** `add_transform`
  consumes the sample and the error does not return it, so the recovery
  converts the message a second time. The contract's route A says the
  same.
- **`ingest`: one ordering fact is relied on, and that is open.** The
  `match` arms imply no order. The `remove_frame` recovery does: its
  comment says expiry is the last insert check, so a sample that reports
  `SampleExpired` already has the frame's parent and kind. Item 10 keeps
  the check order internal, while the contract draft's rustdoc publishes
  exactly this fact. Either it is published, or the example handles a
  failed re-insert without stating why it cannot happen (decision 15).
- **`ingest`: empty names.** The 3.0 example matches `EmptyFrameName`
  (2.4 Part D). The prototype has neither the arm nor a check, because
  the stack is built on `7cc2a65`, without Part D.
- **`ingest`: no `latest` helper.** The prototype wraps
  `latest_common_time` and `get_transform` in one; the 2.4 design keeps
  that lookup in `shared_latest` only. Remove it from the 3.0 file.
- **Struct patterns use `..`,** so the examples survive
  `#[non_exhaustive]` on the struct variants (item 6). If the frame
  fields are taken, the conflict arms print the error's `frame` instead
  of a name kept from the message. For a per-element loop that only makes
  `Display` self-sufficient; the real gain is for `add_transforms`
  callers.

**Open for the example, not settled here.**

- **Guard or recover.** A bridge with a clock can refuse a stamp ahead of
  it before inserting, which makes the `remove_frame` route unreachable.
  The prototype shows the recovery and calls the guard "the better guard
  where it applies" in a comment. The recovery is what the item 10
  contract documents; the example bridge does have the clock in hand. A guard is
  wrong for a bridge whose stamps are not on its own clock (simulation
  time).
- **Threads or tokio.** The prototype's `shared_latest` uses
  `std::sync::RwLock` and threads for determinism (four readers assert
  every answer; 10 of 10 runs passed). The 2.4 form uses tokio. The two
  should not differ; the choice is 2.4's D13. With threads, `tokio`
  leaves the dev-dependencies.
- **Length.** The 3.0 set is 838 lines against 576 for 2.4 and 657 today
  (829 with the 2.4 `shared_latest`). `ingest.rs` is 479 of them. It
  should be cut before it ships; the fixture and the counters are the
  bulk.

**What the stack showed.** All four round 2 library diffs were applied
together for the first time
([`stacked-library.diff`](prototypes/stacked-library.diff), 14 files,
+416/−793).

- **"Each applies cleanly" holds alone, not together.** `sample-expired`
  and `normalize-finite-check-inverse-b` stack cleanly. One hunk of
  `cross-time-type-and-precedence` meets item 4's exit check in the
  cross-time resolve and was merged by hand, keeping both. Seven hunks of
  `deletions` reject: six against lines the cross-time diff changed
  (including the whole-file deletion of `traits.rs`), one against the
  import line `normalize-finite-check-inverse-b` changed. No
  semantic conflict was found.
- **`deletions.diff` leaves debris:** a zero-byte
  `src/geometry/point/mod.rs` and an untouched, orphaned
  `point/tests.rs`. On the stack two doc links to `Transformable`, added
  by the cross-time diff, also survive, because the hunks that remove
  them reject. The stack removes all of these.
- **`sample-expired.diff` is behind its contract draft:** no
  `#[non_exhaustive]` on the variant, a different `Display` text, and a
  three-line rustdoc.
- **Item 1:** the prototype already derives `Clone` and `PartialEq` on
  `CrossTimeTransform`, which the item text calls optional and later.
- **Item 3:** the heading and the summary row said `QuaternionError`
  goes. Only the `TransformError::QuaternionError` variant goes; the type
  stays public, because `Quaternion::normalize` returns it. Both are
  corrected above.
- **Behaviour confirmed on the stack, against 2.3.0:**
  - a late insert under a 10 s window: `Ok(())` becomes `SampleExpired`;
  - a disconnected pair at an uncovered instant: `NotFoundAt` becomes
    `Disconnected`;
  - `get_transform_at` with an unknown target: `NotFoundAt` naming the
    source leg becomes `UnknownFrame`;
  - on 2.3.0, re-inserting a cross-time result moved a stored pose from
    x = 1 to x = −4.

**Verified.** The stacked library builds in all four feature
combinations and for `thumbv7em-none-eabihf`. The four examples build,
pass clippy with `-D warnings` and run with identical output in all four
combinations. `ingest.rs` was changed afterwards for item 12 and needs
`inclusive-cutoff.diff` on top of the stack; that combination was re-run
with the same result. On the stack alone, or on 2.x, its cleanup keeps
the newest sample and the restarted stream is refused. **Not verified:** the gate (the existing tests do not
compile on the stack, as expected of library-only diffs), `cargo test`,
`cargo doc`, the README doctest, stable clippy, MSRV 1.85, `thumbv6m`,
`thumbv8m.main` and RISC-V. Items 2, 3, 4, 8, 9 and 11 are in the stack
but no example exercises them.

## Release plan

- **Order.** 2.4, then the 3.0 prototype on a branch through the full
  gate, then one `3.0.0-rc` built against transforms_io and
  roslibrust_transforms, then the release.
- **Warrant.** D1 cannot ship in a minor: the `with_max_age` rustdoc
  describes the drop and three tests pin it. That makes a 3.0 a given
  rather than a question. It is still not urgent.
- **Acceptance.** State the criterion as "3.0 changes no number that is
  right today" and list the allowed differences against 2.3:
  - lookups: item 2 (off-band inputs, up to 2e-6 relative per hop; in-band
    inputs byte-identical), item 4 (`Ok(non-finite)` to `Err`), item 5
    (two error classes), item 1 (payload of cross-time errors);
  - cleanup: `remove_transforms_before(t)` also removes a sample stamped
    exactly `t`, so a later lookup at `t` on that frame changes from `Ok`
    to `NotFoundAt`, and a stream restarted after a full cleanup is
    stored where 2.3 evicted it (item 12);
  - inserts: `Ok` to `SampleExpired`; `Err(NonUnitRotation)` to `Ok` for
    products of band inputs; the text of insert errors if frame fields are
    taken.
  A dual-version differential harness needs `transforms` 2.3 as a
  dev-dependency, which needs maintainer approval; otherwise keep it
  outside the crate. The existing trace probe must be ported to the
  cross-time type. `cargo semver-checks` sees none of the silent rows.
- **Wire.** Postcard layouts and JSON names stay frozen. 3.0 reads every
  2.x `Transform` payload. Off-band rotations are normalized on decode,
  and `Point` payloads stop decoding.
- **`tests/v2_compatibility.rs`.** Four tests flip (items 1, 2, 5, 11),
  five stay.
- **Other tests that flip or go.** Item 12: two. Item 2: two unit tests. Item 4: one.
  Item 10: three. Item 11: three more. Items 7-9: 17 tests deleted, 13
  re-pointed. `cargo test --no-default-features` stops at the first
  failing target; acceptance runs need `--no-fail-fast`.
- **`AGENTS.md`.** Ten of the sixteen invariant bullets change text
  (round 1 counted six), plus the `geometry` line in Architecture and the
  constructor list in Style. Non-negotiables and Non-Goals are untouched.
- **MIGRATION.** One row per item, tagged loud, silent or wire. Silent:
  items 2, 4, 5, 10, 11, 12. Add per-adapter recovery recipes for item 10 and
  "do not rebuild a `Transform` from the cross-time accessors".
- **2.x after 3.0.** One sentence in `SECURITY.md` on whether a 2.x line
  is maintained; the `semver` CI job assumes a single line.
- **Lead time.** termviz2 and transforms-py still pin 1.x eight months
  after 2.0. No 2.x signal reaches them.

## Decisions for the maintainer

| # | Question | Recommendation |
|---|---|---|
| 1 | `inverse`: conjugate (a), keep normalizing and delete the variant (b), or status quo (c)? | (b). Changed from round 1 |
| 2 | Normalize at the constructors and `Deserialize`, or at `Buffer::insert` only? | Constructors and `Deserialize` |
| 3 | Point family: delete all four, or keep `Point` and `Transformable`? | Delete all four, with the stated cost accepted and a 2.x notice |
| 4 | 2.x notice: `#[deprecated]` or changelog and README? | Changelog and README, unless hearing from unknown users is worth the `#[allow(deprecated)]` sites |
| 5 | Child frame field on structural insert errors? | Leaning yes; the door closes for a major cycle. Decide `EmptyFrameName` with it |
| 6 | Six accessors on `CrossTimeTransform` or two? | Six |
| 7 | Finiteness check: 3.0 or a 2.x fix? | 3.0, unless ruled a bug fix like D5 |
| 8 | Delete `SameFrameMultiplication`? | Yes, only with item 1 |
| 9 | Rename `ReparentingNotSupported`? | Optional |
| 10 | Dev-dependency on `transforms` 2.3 for a differential harness? | Maintainer's call; otherwise outside the crate |
| 11 | The policy sentence from round 1 (what a minor may change) | Still open; unchanged |
| 12 | Item 10 | Decided 2026-11-10 (D1-D6) |
| 13 | Examples in 2.x: asserts only (E1), also the four task-named files (E2), also `ingest` (E3), or wait for 3.0? | Decided 2026-11-11: all three in 2.4. Port them in 3.0 ([Examples in 3.0](#examples-in-30)) |
| 14 | The 3.0 `ingest` example: show the `remove_frame` recovery, or a guard that refuses stamps ahead of the bridge's clock? | No firm recommendation. The recovery is what the contract documents; the guard is simpler where stamps are on the bridge's clock |
| 15 | Publish "expiry is the last insert check"? The `remove_frame` recovery and the contract draft rely on it; item 10 keeps the order internal | Open. If not published, the example must not state it |
| 16 | Item 12: inclusive cutoff for `remove_transforms_before`, name kept | Decided 2026-11-11: inclusive, no rename, a silent MIGRATION row |

## Changes from round 1

| Topic | Round 1 | Round 2 |
|---|---|---|
| `inverse` | Conjugate; 28% of lookups change last bits; 10-13% faster | Keep normalizing. 46.5% of lookups change bits under the conjugate (a different forest); 22% faster; one property test becomes flaky; one `Err` becomes `Ok` |
| `QuaternionError` | Unreachable once item 2 lands | Reachable today through public `*` |
| Dead-band bound | 7 EPS "unproven" | Proven given a correctly rounded `sqrt`; 8 EPS given a faithful one |
| Non-idempotent normalization | 15.6% | 38% for a second pass. 15.6% is the third pass; both figures reproduce |
| Item 2 hazard | Scale per hop | Also a scale step in time between a stored instant and between samples |
| Item 5 scope | One error class; "it never will" | Two classes; a later insert can connect trees; `get_transform_at` needs a cross-leg rule |
| Cross-time error payload | Open | Rule given and prototyped |
| Item 1 evidence | A pinned test | A live overwrite-and-broadcast path in roslibrust |
| Point family | Delete; cost not stated | Delete; seven silent slips for positions stated; conditions listed |
| `SameFrameMultiplication` | No position; assumed the test stays | Delete, with item 1 |
| Frame fields on insert errors | Not in 3.0 | Open, leaning yes |
| `interpolate` oracle | In-test reference | Same; and today's oracle is shown blind to arithmetic mutations |
| Item 10 recovery | "`remove_transforms_before` past the newest sample"; two `reparent_frame` calls | By cause; `remove_frame` first; strict cutoff with its limits, which item 12 then removes for 3.0; reparent route kept out of docs; guard against mechanical use |
| Item 10 `newest` | "The frame's newest stored instant" | Can be an unstored batch member |
| Item 10 prototype | With `clear_history`; changes `Buffer::check` | Without; `check_expiry` beside an unchanged `Buffer::check`. Round 1's was correct on atomicity and order |
| Item 10 cost | 41 ns against 74 ns for a rejected insert | Not reproduced: 63.6 against 64.3 ns |
| Required `max_age` | "Bounds memory only" | Wrong: it bounds the gap at `max_age`. Still rejected |
| Maximum gap | Rejected | Open, additive, design first |
| `v2_compatibility` | Three flip, six stay | Four flip, five stay |
| `AGENTS.md` bullets | Six | Ten of sixteen |
| Diff sizes | Mixed non-blank and all-line counts | All-line counts throughout |
| 2.x deprecation probe | Recommended | Kept as a decision, with its limited reach stated |

Round 1 was right where round 2's draft was wrong on three points: the
embedded sentence (software `f64` on every gate target by default), the
condition on the resolver rewrite, and the remedy for the `interpolate`
oracle.

## Corrections to earlier documents

Round 1's list stands (see
[`round1/REPORT.md`](round1/REPORT.md#corrections-to-earlier-documents)).
Added by round 2:

- **2.4 proposal, Part C item 5.** See [Ships in 2.x](#ships-in-2x).
- **`add_transforms` rustdoc** (`src/core/registry/mod.rs:171-172`): "The
  error does not identify the rejected transform" becomes false for
  `SampleExpired` under item 10.
- **`tests/serde.rs:269-283`.** The comment claims any `Quaternion`
  reorder is caught; with an identity rotation it is not.
- **Dependents study.** The count of `transforms::Quaternion` struct
  literals is 5 in transforms_io and 2 in roslibrust, not 9 and 4; the
  rest are ROS message structs.
- **Round 1 report.** The rows of the table above.

## Limits

- `tests/test_all.sh` ran on no prototype. The single-item prototypes
  ran `cargo test` in default and no-default-features modes, most with
  `serde`; clippy and fmt on the library; one `thumbv6m` build for items
  2-4. No stable clippy, MSRV 1.85 or RISC-V build. The stacked diff
  builds but `cargo test` does not compile on it; its limits and those of
  the examples are under [Examples in 3.0](#examples-in-30).
- No measurement on hardware. Embedded instruction counts come from QEMU
  in the lens review and were not re-run. Host timings were taken on a
  shared machine; differences under about 3 ns are noise.
- Only roslibrust_transforms was type-checked against a prototype
  (`cargo check --lib`, item 1). Every other downstream count is from
  grep and reading sources. The dimos PR was read at head `fc941c1`.
- `libm::sqrt` correct rounding is tested, not proven. The band's fixed
  point is empirical for bands narrower than 8 EPS.
- The 10.6% flake rate under the conjugate comes from a replica of the
  property's generators, not from proptest's own distributions.
- The differential probes used `Timestamp` and one custom clock.
- Private users are invisible. crates.io lists three reverse
  dependencies; "zero users" of a deleted item means zero in seven public
  code bases.
- The frame-field question was not settled by evidence.
- Not covered by any lens: lock contention, allocator failure on
  small-RAM parts, `SystemTime` in benchmarks.

## Prototypes

Library-only diffs against `7cc2a65`; each applies cleanly on its own.
Together they need the hand merges listed under
[Examples in 3.0](#examples-in-30). They are
evidence, not patches: none is gate-clean and none updates docs or
existing tests (the stack deletes one orphaned unit-test file and two
dead doc links). Round 1's are in [`round1/prototypes/`](round1/prototypes/).

| File | Content | Lines | Existing tests that fail |
|---|---|---|---|
| [`sample-expired.diff`](prototypes/sample-expired.diff) | Item 10 | +118/−6 | 3, pins of the old contract |
| [`sample_expired_tests.rs`](prototypes/sample_expired_tests.rs) | 15 new tests for item 10, including the atomicity and order traps | 445 | n/a |
| [`normalize-finite-check-inverse-b.diff`](prototypes/normalize-finite-check-inverse-b.diff) | Items 2, 4 and 3 as recommended | +59/−11 | 4, all intended |
| [`normalize-finite-check-inverse-a-conjugate.diff`](prototypes/normalize-finite-check-inverse-a-conjugate.diff) | Items 2 and 4 with the conjugate, for comparison | +56/−19 | 4, plus one flaky property |
| [`cross-time-type-and-precedence.diff`](prototypes/cross-time-type-and-precedence.diff) | Items 1 and 5 with the cross-leg error rule | +214/−118 | 1; 12 lines in 5 tests stop compiling |
| [`precedence-only.diff`](prototypes/precedence-only.diff) | Item 5 alone | +41/−66 | 1 |
| [`deletions.diff`](prototypes/deletions.diff) | Items 7, 8, 9 and 11 | +29/−546 | 4 flip (item 11); the rest are deleted with their subjects |
| [`stacked-library.diff`](prototypes/stacked-library.diff) | The four recommended diffs applied together, hand-merged (added 2026-11-11) | +416/−793 | `cargo test` does not compile |
| [`examples/`](prototypes/examples/) | The four examples on the stack plus the item 12 diff: `basic`, `shared_latest`, `cross_time`, `ingest` (added 2026-11-11) | 838 | n/a |
| [`inclusive-cutoff.diff`](prototypes/inclusive-cutoff.diff) | Item 12. Applies to `7cc2a65` and, with offsets, on the stack (added 2026-11-11) | +12/−8 | 2, pins of the strict cutoff |
