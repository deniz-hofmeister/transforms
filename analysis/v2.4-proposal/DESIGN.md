# transforms 2.4: design proposal

Date: 2026-10-03. Draft revision: 7 (2026-11-11). Baseline: master
`aa29557` (released v2.2.0) for the §2 probes; master `f9770d2` (released
v2.3.0 plus docs) for the dependent evidence and Part D; master `698d1b8`
for Part C item 10 and the f32 gate; master `7cc2a65` for Part E.

This document proposes the contents of a 2.4.0 minor release. It follows from
the verdict in [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md):
no v3 now, and the reachable value ships additively. The later
[v3 review](../v3-review/REPORT.md) (2026-11-10, amended 2026-11-11)
makes a 3.0 a given, after 2.4 and not urgent, and lists what it expects
2.x to ship ([Ships in 2.x](../v3-review/REPORT.md#ships-in-2x); §6,
D17). Nothing here is
implemented. AGENTS.md applies throughout. Parts B and D grow the public
surface, so this document is the maintainer note that AGENTS.md requires.
It does not authorize implementation, merge, or release.

Revision 2 retargeted the proposal from 2.3.0 to 2.4.0: 2.3.0 shipped
`Registry::add_transforms` alone (`76787c2`). The probes in §2 ran at
`aa29557` and were not re-run. `src`, the README, `Cargo.toml`,
MIGRATION, the gate script and the examples are unchanged since v2.3.0,
so references into them point into the v2.3.0 tree. AGENTS.md references
are to `f9770d2` (unchanged through `7cc2a65`), and the CI workflow
reference in §9 is to `7cc2a65`.

Revision 3 moves Part A, the cross-time result type, to the v3 ledger. An
additive type beside `get_transform_at` does not close the hazard it
targets, and it would commit 2.x to a second cross-time API designed ahead
of the v3 work it belongs with (§3). Its revision 2 design is kept in
[`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md);
the current design is item 1 of the v3 review (§3).
The dimos report's earlier recommendation to keep Part A is superseded;
its current text records the move
([`dimos.md`](../dependents/dimos.md#changes-since-2026-10-30)). The part letters and decision
IDs are kept, so references from other reports stay valid.

Revision 4 adds the dependent study
([`../dependents/README.md`](../dependents/README.md): three reports, with
probes under [`../dependents/probes/`](../dependents/probes/) run at
`f9770d2`; `src` is unchanged since v2.3.0). Each candidate was put to one
question: is it strictly necessary for 2.4?

- **In.** Part D, rejecting empty frame names (§5a). The maintainer ruled
  on 2026-11-06 that the rejection is non-breaking, so it ships in 2.4.
  Part C item 5 widens to backward clock jumps, and items 8 and 9 are new.
  Item 2 and §2.3 gain dependent evidence, and §4.1's survey of users
  turns D2's recommendation to No.
- **Not strictly necessary.** A rotation-tolerance sentence, negative
  seconds for pre-epoch `SystemTime`, and a maximum interpolation gap
  (§6). The study's v3 ledger rows and outreach items are not 2.4 work
  and stay in the index.
- The §2 probes were not re-run. Section numbers are kept: Part D is §5a.

Revision 5 adds Part C item 10 and states the f32 Non-Goal gate in §6.
Both were checked at `698d1b8`; `src` is unchanged since v2.3.0.

Revision 6 added Part E, the examples (§5b), with decisions D9 to D16. It
was prototyped at `7cc2a65`; `src` is unchanged since v2.3.0. It classed
the pieces as separate documentation changes outside the release, because
none is strictly necessary under the revision 4 question.

Revision 7 records the maintainer's decision of 2026-11-11 that Part E is
part of 2.4 (D9, D10, D12, D13, D15), and brings the document up to date
after an audit against the code and the v3 review:

- Part C item 5 now carries the corrected clock-jump sentences (D16).
  The strict cleanup cutoff they describe is 2.x only ([v3 item 12](../v3-review/REPORT.md#12-remove_transforms_before-removes-the-sample-at-the-cutoff)).
- Items 4 and 6 state what the rustdoc already says and what is missing.
  Item 1 is superseded by E2 (D14).
- §3 and §6 point at the v3 review where it replaced the plans they
  cited. The review's other 2.x items are listed in §6 and left to D17.
- §7 gains three Part D tests; §9 gains the Part C changelog line and
  the MIGRATION title.

## 1. Summary

| Part | What | Public surface | Status |
|---|---|---|---|
| A | A cross-time result type; `get_transform_at` deprecated | — | Moved to the v3 ledger (§3) |
| B | Checked application of a transform to plain positions | 1 method, if granted | Needs a scope decision; recommended No for 2.4, leaving the Part C item 6 recipe (§4.1, D2) |
| C | Documentation corrections | none | Proposed |
| D | Reject empty frame names at insert | 1 variant | Decided 2026-11-06: non-breaking; site open (§5a, D6) |
| E | Examples that assert, organised by task | none | Decided 2026-11-11: part of 2.4, as three commits (§5b) |

Everything is additive under SemVer. No signature, trait bound, or public
path changes. Part D adds one variant to the `#[non_exhaustive]`
`RegistryError` and declares one behaviour change, which the maintainer
ruled non-breaking: an insert naming an empty frame returns `Err` where
2.3.0 returned `Ok`. `cargo semver-checks` should find nothing breaking in
any of the four feature combinations; the release is a minor whether or
not Part B ships, because Part D adds a public variant (§9).

## 2. Motivation

The dependent evidence puts the reachable silent answers at input
boundaries and in time, not in geometry
([`../dependents/README.md`](../dependents/README.md#silent-wrong-answers)).
Part D and Part C items 2, 5 and 8 address those. The rest of this section
is Part B's case.

Part B closes one gap: **the checked path does not reach the data
callers actually transform, so they leave it.** For a crate whose worst
failure is a plausible wrong answer, a checked path that is inconvenient to
use protects nobody.

Probe-verified at `aa29557`:

1. **No checked route exists for raw positions.** Transforming a `Vector3`
   or a point cloud means writing `rotation().rotate_vector(v) +
   translation()` by hand, which skips the frame and time checks. The
   crate's one in-tree instance of this is the `get_transform_at` rustdoc
   example (`src/core/registry/mod.rs:416`). That is cross-time
   application, where hand-written arithmetic is the documented route, so
   it is evidence for the cross-time gap, not for single-time callers
   skipping checks.
2. **Cross-time application is inverted, and it stays that way in 2.x.**
   Take `x = get_transform_at(tgt, t2, src, t1, fixed)`.
   - Applying `x` to a point in `src` at `t1` (the correct input) fails with
     `TimestampMismatch`.
   - Applying `x` to a point in `src` at `t2` (the wrong input) returns `Ok`
     and mislabels the result.
   - Inserting `x` succeeds when the target is the source's pinned parent
     and that edge is dynamic, and then serves the t1 relationship as if
     it held at t2. Otherwise it fails: `ReparentingNotSupported` for
     another target, `StaticDynamicConflict` for a static edge,
     `CycleDetected` when the source is its tree's root. This is pinned by
     `cross_time_result_keeps_only_target_stamp_and_remains_insertable`.

   Only a type change on the method closes this, and that is a break (§3).
   2.x already documents it. The `get_transform_at` rustdoc ("Temporal
   metadata limitation"), the README and MIGRATION state the restriction;
   the crate docs and the `Transform` and `Transformable` rustdoc point to
   it.
3. **The hazard reaches a dependent.** `roslibrust_transforms` 0.26.0
   wraps `get_transform_at` (`src/lib.rs:555-573`). Its rustdoc tells
   callers the result "converts points from `source_frame` at
   `source_time`". That is true of the geometry, but it does not say that
   the checked path rejects that use, or that the result must not be
   inserted. Its `add_transform` inserts and then publishes on `/tf`
   (`lib.rs:518-547`), so a `get_transform_at` result for such a pair, passed to it,
   overwrites the true sample and is broadcast with the false stamp
   (`../dependents/probes/roslibrust_transforms.rs` P5,
   `roslibrust_transforms_verify.rs` V5;
   [Z4](../dependents/roslibrust_transforms.md)). This is reachability,
   not observed use.

## 3. Part A: moved to the v3 ledger

Revision 2 proposed `CrossTimeTransform<T>`, returned by a new
`Registry::get_cross_time_transform`, with `get_transform_at` deprecated
beside it. It is withdrawn from 2.4:

- **It does not close the hazard.** The old method keeps returning an
  insertable `Transform` with a false stamp until v3. A deprecation warns
  only callers who build against the new version and act on it. The
  v3 warrant report says the same: "Deprecation helps only callers who
  migrate. The cross-time hole stays open for everyone else."
- **It is a second spelling.** 2.x would carry two cross-time lookups that
  return different types for one query. That is the kind of duplication §6
  rejects for `checked_add` and an `unbounded()` alias.
- **It was designed ahead of its context.** Revision 3 expected the
  type's accessors and application to interact with a v3 `Rotation`
  newtype, a public `Isometry` and the move from operators to named
  methods (v3-3 §1, §2 and §4; v3-1 #7 for the operators). The v3 review
  takes none of those and still changed the type's shape: no application
  method, no fixed-frame field, and a cross-leg error rule
  ([item 1](../v3-review/REPORT.md#1-a-cross-time-result-type)). A type
  shipped in 2.4 would have been broken by that. Minimalism is measured
  in exported commitments.

**What 2.x does about it instead:**

- **Nothing more in the crate.** The application restrictions are already
  stated or linked everywhere §2.2 lists. Part C item 7 of revision 2
  existed only to repoint those references at the new method, so it is
  dropped.
- **Outreach, outside the crate.** Ask `roslibrust_transforms` to link
  `get_transform_at`'s application restrictions from its wrapper. This
  is not a change to this repository.

The design moves to
[`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md),
reworked so that `get_transform_at` itself returns the new type. The v3
ledger row in the warrant report points there.
[Item 1 of the v3 review](../v3-review/REPORT.md#1-a-cross-time-result-type)
supersedes the addendum's shape and is the current design.

## 4. Part B: checked application to positions

### 4.1 Scope decision required

This part conflicts with the letter of AGENTS.md's scope rule. A caller can
build it exactly on the public API: `parent()`, `child()`, `timestamp()`,
`rotation()`, `translation()`.

The case for an exception:

- **The check is the crate's own invariant logic.** That logic is: the frame
  equals `child`, and the stamp is `Static` or equal to the value's time.
- **The crate already ships that logic once,** as `Point`'s `Transformable`
  impl.
- **Every caller with non-`Point` data must copy it by hand or skip it.**
  That is the charter's "pushed-out invariant logic becomes hand-rolled
  wrong copies".
- **The relabel is the half callers forget.** The method writes the output
  frame, so the caller cannot forget it.

The case against:

- No user or dependent has asked for it, and the demonstrated-need rule is
  not met.
- With Part A gone, the in-tree evidence is gone too: the one hand-rolled
  application in the crate is cross-time, which Part B does not cover (§2.1).
- None of the three studied dependents (transforms_io, roslibrust_transforms,
  and dimos, which uses the crate only in an unmerged PR) uses `Point`,
  `Transformable` or `Localized`
  ([`../dependents/README.md`](../dependents/README.md#transforms-2x-code)).
  The one downstream point-cloud consumer, pelorus_syncio, receives a ROS
  `TransformStamped` from transforms_io and applies it as an `f32`
  nalgebra isometry (`pel@0.0.9:src/lib.rs:764-781`,
  [`transforms_io.md`](../dependents/transforms_io.md)), so
  `transform_positions` would not reach it.

If the maintainer rejects the exception, Part B reduces to a rustdoc recipe
on `Transformable` (§5).

### 4.2 API

```rust
impl<T: TimePoint> Transform<T> {
    /// Maps child-frame positions observed at `timestamp` into the parent
    /// frame, then sets `frame` to the parent.
    pub fn transform_positions(
        &self,
        frame: &mut String,
        timestamp: T,
        positions: &mut [Vector3],
    ) -> Result<(), TransformError>;
}
```

### 4.3 Semantics

- **The checks match `Point::transform`, and run before any mutation.** On
  error, the frame and positions are unchanged.
- **The frame is taken as `&mut String` deliberately.** The method that
  checks the input label also writes the output label, as `Point` does. A
  caller cannot leave a cloud labelled `camera` after mapping it into `base`.
- **The timestamp is taken by value.** A single-time transform preserves
  it, so there is nothing to write back.
- **Positions only.** Free vectors (normals, velocities) take only the
  rotation and are left out until a need appears.
- **One copy of the check.** `Point`'s `Transformable` impl and
  `Transform::transform_positions` share one private check-and-relabel
  helper, so the rule exists once.

### 4.4 Example

```rust
let tf = registry.get_transform("base", &cloud.frame, cloud.stamp)?;
tf.transform_positions(&mut cloud.frame, cloud.stamp, &mut cloud.points)?;
assert_eq!(cloud.frame, "base");
```

Disjoint field borrows make this compile as written.

## 5. Part C: documentation corrections

None of these changes behaviour. `Display` text is declared unstable
(`README.md:117`).

1. **Example direction comments.** Superseded by Part E's E2, which is
   part of 2.4 (D10, D14): those files are replaced, and the wording and
   names below are requirements on `basic.rs` (§5b). The original item,
   for the record: in `examples/std_minimal.rs:33,57` and
   `examples/no_std_full.rs:37,61`, comments say "transform from camera to
   base" above `Transform::new("base", "camera", …)` and "transform from
   base to map" above `Transform::new("map", "base", …)`. Rewrite them as
   "camera expressed in base (parent base, child camera)" and likewise for
   base in map. Rename `camera_to_base_*` to `base_camera_*`,
   `base_to_map` to `map_base` and `base_to_imu` (`no_std_full.rs:73`)
   to `base_imu` (the `t_a_b` style the tests use).
2. **`latest_common_time` discoverability.** Add one sentence to the
   `src/lib.rs` crate docs naming `Registry::latest_common_time` as the
   answer to "what is the newest instant I can look up?". Demonstrated:
   transforms_io 0.8.0 answers a time-zero lookup at `Timestamp::now()`,
   silent on post-dated or skewed hops
   ([H1](../dependents/transforms_io.md);
   `../dependents/probes/transforms_io_verify.rs` N4, V1), although
   `README.md:100-103` already names `latest_common_time`.
3. **E0283.** Repeat the type page's `Registry::<Timestamp>::new()` note
   (`src/core/registry/mod.rs:87`) on `Registry::new`, `Registry::with_max_age`
   and `Transform::static_between`. These are where an all-static setup
   fails to infer.
4. **`Stamp` versus `T`.** The `latest_common_time` rustdoc already says
   that on `Stamp::Static` "the caller then chooses an instant"
   (`src/core/registry/mod.rs:445-446`), but its example calls
   `stamp.at().unwrap()` (`:496`), which would panic on an all-static
   chain. Add one sentence after the existing one: do not copy the
   example's `.at().unwrap()` for chains that may be static; match on
   the `Stamp` and pass the caller's own time in the `Static` arm.
5. **Clocks that wrap or step backward.** Add one paragraph to the
   `TimePoint` rustdoc: the crate cannot detect a wrapping hardware
   counter. `Ord` stays total, but the stream's order disagrees with it.
   Widen it to a monotonic 64-bit count before implementing the trait.
   Add to the `Registry::with_max_age` rustdoc
   (`src/core/registry/mod.rs:122-130`), which states the eviction rule
   but not its consequence: "The registry does not detect a clock that
   steps backward. An insert older than the frame's newest timestamp
   minus `max_age` returns `Ok` and is evicted at once. A smaller jump
   keeps both runs, and lookups interpolate across them, as every jump
   does without `max_age`; a larger jump does the same from the moment
   the restarted stream reaches the newest timestamp minus `max_age`.
   Whoever owns the clock must detect the jump. `remove_frame` discards
   one frame's samples and its newest timestamp, for any time type, and
   releases the frame's parent and kind until its next insert.
   `remove_transforms_before` does it for every frame at once and keeps
   their pins, with a cutoff strictly later than every stored dynamic
   sample; a cutoff equal to the newest sample keeps it, and no such
   cutoff exists for a sample at the time type's maximum." The last
   sentences are the corrected form from
   [`sample-expired-contract.md` (f)](../v3-review/sample-expired-contract.md),
   with its probe record (D16); revision 6 said "a cutoff past every
   stored sample", which is wrong at the newest stamp. Link
   `remove_transforms_before`'s text on the expiry reset (`mod.rs:577-582`). The cutoff is
   strict in 2.x only; 3.0 makes it remove the sample at the cutoff
   ([v3 item 12](../v3-review/REPORT.md#12-remove_transforms_before-removes-the-sample-at-the-cutoff)), and these sentences change with it. Cause: the newest timestamp
   is a running max, and eviction runs after the insert is accepted
   (`src/core/buffer/mod.rs:316-321`). Evidence: transforms_io H7
   (`../dependents/probes/transforms_io.rs` P6,
   `transforms_io_verify.rs` N3), roslibrust_transforms Z3
   (`roslibrust_transforms.rs` P3a, P3b), dimos Hazard 1.
6. **If Part B is rejected** (D2's recommendation), extend the
   `Transformable` rustdoc. Its `# Contract` section
   (`src/geometry/transform/traits.rs:47-53`) already gives the position
   formula and the relabel to `tf.parent()`; `# Preconditions` says only
   that implementations "check frames and timestamps". Add an "Applying
   a transform to your own data" example, a compiled doctest, that
   spells out what `Point` performs (`src/geometry/point/mod.rs:112-129`):
   fail unless the data's frame equals `child()`; fail unless
   `timestamp()` is `Stamp::Static` or equals the data's time; then
   apply `rotation().rotate_vector(p) + translation()` to each position
   and set the frame to `parent()`. The code is not drafted here. The v3
   review deletes `Transformable` and moves this recipe to `Transform`'s
   rustdoc
   ([item 7](../v3-review/REPORT.md#7-delete-point-transformable-localized-get_transform_for),
   first condition). If v3 decision 3 is taken before 2.4, put the
   recipe on `Transform` and link it from `Transformable`.
7. **Withdrawn in revision 3.** It repointed the cross-time references at
   Part A's method. They stay as they are (§3).
8. **Dropping a `StaticDynamicConflict` transform.** Add one sentence to
   the `RegistryError::StaticDynamicConflict` rustdoc
   (`src/core/registry/error.rs:81-96`): "A caller that logs and drops
   the rejected transform leaves the stored static pose answering every
   instant; change the kind deliberately with `remove_frame`, not from an
   error handler." Today the doc states that consequence only for
   `reparent_frame`'s seed. Both shipped bridges log and drop: transforms_io
   H4 (`../dependents/probes/transforms_io.rs` P3, `Ok(0.0)` where the
   publisher said 3.0), roslibrust_transforms Z2
   (`roslibrust_transforms.rs` P2, `Ok(x=0.5)`).
9. **`try_now` wording.** In the `Timestamp::try_now` rustdoc
   (`src/time/timestamp/mod.rs:76`), replace "The panic-free counterpart
   of [`Timestamp::now`]." with "Reports as errors the out-of-range
   readings that make [`Timestamp::now`] panic." It calls std's
   `SystemTime::now` (`:95`), which traps on `wasm32-unknown-unknown`
   (roslibrust_transforms Z10, `../dependents/probes/wasm_try_now.rs`).
   No panic is added to the docs, so AGENTS.md is unchanged.
10. **`add_transforms` iterator consumption.** The rustdoc says "The
    iterator is consumed before anything is stored"
    (`src/core/registry/mod.rs:166-167`). That holds for an accepted
    batch. A rejected batch returns from inside the loop (`:227-247`), so
    the iterator is read only up to the first rejected transform. The
    argument is `impl IntoIterator`, which a `&mut` iterator satisfies, so
    a caller can observe it: the transforms after the rejected one are
    still in their iterator. Append "A rejected batch stops reading at the
    first rejected transform." The batch-insert record asked for this
    wording
    ([`DISCUSSION.md`](../atomic-batch-insert/DISCUSSION.md), "consumed up
    to the first error"), and the shipped sentence dropped it.

## 5a. Part D: reject empty frame names

Decided: the maintainer ruled on 2026-11-06 that rejecting `""` is
non-breaking, so it ships in 2.4 (D5). The enforcement site is open (D6).
References are to `f9770d2`.

**Hazard.** Chain resolution treats equal names as one frame
(`src/core/registry/mod.rs:871-910`). `""` is ROS 2's unset `frame_id`.
Two publishers that leave it unset share a fabricated root, and a lookup
between their unrelated trees returns `Ok`: transforms_io H5
(`../dependents/probes/transforms_io.rs` P4, `Ok(2.0)`) and
roslibrust_transforms Z1 (`roslibrust_transforms.rs` P1, `Ok(x=2)`). It is
the only tier-1 recommendation the study places in 2.x code
([rec 1](../dependents/README.md#transforms-2x-code)). tf2 rejects both
ids (`tf2/src/buffer_core.cpp:224-236`@`183399c`).

**Change.** One flat public variant with a unit payload:

```rust
/// The transform's parent or child frame name is empty. An empty name
/// is a missing name, not a frame: two publishers that both left it
/// unset would share one fabricated frame, and lookups between their
/// unrelated trees would succeed.
#[error("frame name is empty")]
EmptyFrameName,
```

`Buffer::check` (`src/core/buffer/mod.rs:350-376`) returns a crate-private
`InsertError::EmptyFrameName` when the parent or child is empty, after
`validate` and before the self-reference check, so `("", "")` reports
the empty name. `From<InsertError>` (`src/core/registry/error.rs:237-258`)
gains the matching arm and stays total. About six production lines.

**Enforcement points.** Every insert path runs `Buffer::check`:
`add_transform` (`registry/mod.rs:811-822`), `add_transforms` (`:229`,
`:232`, `:244`; the batch is rejected whole), and the `reparent_frame`
seed (`:782`). The sibling rule, "a frame cannot be its own parent",
lives in the same place (`buffer/mod.rs:356-358`). Constructors,
`Deserialize` and `Transform::validate` are unchanged.

**Lookup rule.** No lookup code. Nothing stores `""`, so it is an
unregistered name like any other: an empty endpoint reports
`UnknownFrame("")` (`frame_exists`, `registry/mod.rs:915-920`), and `""`
against itself is the identity, as documented for every unregistered
name (`:264-265`). With no carve-out, AGENTS.md's identity clause stays
true.

**Observable change.** `add_transform` and `add_transforms` return
`Err(RegistryError::EmptyFrameName)` for an empty parent or child that
2.3.0 accepted, and `reparent_frame` does for an empty new parent. An
empty child is never stored, so `reparent_frame` reports it as
`UnknownFrame("")`. Lookups that resolved through a stored `""` now
report `UnknownFrame` or `Disconnected`. Serde data naming `""` still
decodes; it cannot be inserted. The flat-error rule holds directly: no
`TransformError` twin exists.

**Not the dependent reports' shape.** They put `TransformError::EmptyFrameName`
in `Transform::validate`, with a `get_transform("", "", t)` carve-out
([`dimos.md`](../dependents/dimos.md#2x-reject-empty-frame-names)). That
also changes `new`, `static_between` and `Deserialize`, none of which
resolves a chain; stored data stops decoding; and two AGENTS.md
invariants change. The registry would then report a frame-tree rule
only through the wrapped `RegistryError::TransformError` arm, while its
sibling, `SelfReferentialFrame`, is flat.

**Docs.** The variant doc above; `EmptyFrameName` in the `# Errors` of
`add_transform` (`registry/mod.rs:146-154`) and among the seed's errors
in `reparent_frame` (`:695-698`); the internal `Buffer::insert` errors.
One README clause under conventions (`README.md:108-112`): "Registry
insertion also rejects an empty frame name." Tests are in §7, and the
CHANGELOG, MIGRATION and AGENTS.md obligations in §9.

## 5b. Part E: examples that assert, organised by task

Examples are not public surface and are not in the published package
(`Cargo.toml:19-26` lists `/src`, the manifest, README, CHANGELOG,
MIGRATION and LICENSE). Nothing here touches SemVer. The maintainer
decided on 2026-11-11 that all of Part E is part of 2.4: it lands in the
repository before the release is cut. References are to `7cc2a65`.

**What is wrong today.**

- **The gate cannot fail on a wrong example result.** No example asserts
  a value; expected answers live in comments. Eight failures (six
  lookups, two applications) are logged, not raised: `std_minimal.rs:80,83`, `no_std_minimal.rs:47,54`,
  `no_std_full.rs:91,101,104`, `std_full.rs:97`. In `std_full` the insert
  `.unwrap()` (`:69`) runs in a spawned task whose handle is only aborted
  (`:104-105`), so a panic there never reaches the exit code (by reading,
  not run). The gate's example step (`tests/test_all.sh:37-42`)
  therefore checks that six programs do not panic on the main thread. It
  claims no more than that they run; the gap is in what a reader assumes
  it proves.
- **No example names `RegistryError`** (grep: zero hits). Three of
  four users carry errors as strings, and all four have error-blind waits
  ([`../dependents/README.md`](../dependents/README.md#hand-rolled-around-the-crate)).
- **No example shows the ingest side.** Checked time conversion, a
  decision per insert error and clock-jump handling are where the
  adapters went wrong (H4, H6, H7, Z2, Z3, Z6 in the dependent reports).
- **The files are organised by feature flag.** The API is the same in
  both modes, so the split teaches nothing. By `diff`, `std_advanced` and
  `no_std_advanced` differ in 14 lines, and `std_minimal` and
  `no_std_full` in 50; `no_std_minimal` and `std_full` have no twin. The
  `no_std_*` files are hosted programs that log through `std`, so they do
  not show bare-metal use; the thumb builds and
  `cargo test --no-default-features` do.
- **The names say nothing.** `minimal`, `full` and `advanced` need the
  README table to be understood, and `no_std_full.rs:1` claims "the
  complete functionality", which it does not show.

**The four pieces.** No piece is strictly necessary for 2.4 under the
revision 4 question; they are in the release by the maintainer's
decision. The last column is the case for each.

| Piece | What | Case on merit |
|---|---|---|
| E1 | Every example asserts its results, so a wrong value fails the gate | Strong. The gate's example step can fail only on a main-thread panic |
| E2 | Task-named files that run in both feature modes replace the six | Moderate. A deletion and a clearer index, with no hazard behind it |
| E3 | A new `ingest` example | Moderate. It rests on the same dependent findings as Part C items 5 and 8. Whether an example changes what adapters do is untested: transforms_io ignored `latest_common_time` with `std_full` available |
| E4 | Drop `log` and `env_logger` from `[dev-dependencies]` | Follows from E1 and E2 |

E2, E3 and E4 are one purpose and one commit each; E1 is met inside E2 and E3 (below). None rides in Part D's commit.

### E1. Examples fail on a wrong result

Replace every logged outcome with an asserted one: `assert_eq!` for exact
values, `approx::assert_abs_diff_eq!` for computed ones, and a `match`
that panics on the unexpected arm for expected errors. The async example
joins its tasks instead of aborting them. This could be done in the
existing six files without renaming anything, but with E2 and E3 also
decided that would add asserts to files the next commit deletes. So E1
has no commit of its own: the E2 files (`basic`, `shared_latest`,
`cross_time`) and the E3 file (`ingest`) are written with asserts, and
E1 is the acceptance rule both commits must meet (§7). That makes three
commits: E2, E3, E4. This order is a recommendation, not part of the
maintainer's decision.

### E2. Four task-named, feature-agnostic examples

| New file | Replaces | Shows |
|---|---|---|
| `basic.rs` | `std_minimal`, `no_std_minimal`, `no_std_full` | Insert, a static edge, an interpolated lookup, checked application through `Point`, the opposite direction, and the no-extrapolation error with its `covered` range |
| `shared_latest.rs` | `std_full` | A tokio writer and reader on one `RwLock`; `latest_common_time` then `get_transform` under one read guard, with the `Stamp::Static` arm served by a lookup, never `.at().unwrap()` (Part C item 4) |
| `cross_time.rs` | `std_advanced`, `no_std_advanced` | `get_transform_at` on the conveyor case, with the warning not to re-insert the result |
| `ingest.rs` | new (E3) | A message loop |

- **One file, both modes.** Every example uses fixed
  `Timestamp::from_nanos` stamps and no `cfg`-gated `main`, so the same
  file runs with and without the library's `std` feature. No example
  needs that feature; `shared_latest` needs tokio, a dev-dependency in
  both modes. The six `[[example]]` blocks with their `required-features`
  go, and cargo discovers the files.
- **Runs.** E2's commit leaves three files and six runs; E3's commit
  adds `ingest` and makes it eight: four examples in two modes. The gate
  is edited in both commits, not weakened, and the CI `examples` job
  must be edited with it each time.
- **Manual cleanup leaves with the `no_std_*` files.** Today
  `remove_transforms_before` is shown in `no_std_minimal.rs:58`,
  `no_std_full.rs:110` and `no_std_advanced.rs:134`. Between
  the E2 and E3 commits no example shows it; from E3 on, `ingest` does.
- **`Timestamp::now()` leaves the examples.** It stays covered by
  `now_returns_a_positive_wall_clock_time` and
  `try_now_returns_the_current_time_without_panicking`
  (`src/time/timestamp/tests.rs:47,54`).
- **`Point` stays demonstrated.** In 2.x `Point::transform` is the
  checked way to apply a transform, and §2.1 records hand-written
  `rotate_vector(v) + translation()` as the route that skips the frame
  and time checks. `basic` therefore leads with the registry and applies
  through `Point` after its first lookup. Removing `Point` from the examples would
  pre-empt the v3 decision on it
  ([v3 review, item 7](../v3-review/REPORT.md#7-delete-point-transformable-localized-get_transform_for)).
  The README's compiled snippet (`README.md:41-72`) also keeps it. If
  the v3 review's removal notice for the Point family ships in 2.4, it
  goes in the changelog and README, not in the example.
- **Names.** `basic` uses the `t_a_b` names and the "camera expressed in
  base (parent base, child camera)" wording of Part C item 1.

### E3. The `ingest` example

A bridge loop over a plain struct (`sec: i32`, `nanosec: u32`, two frame
names, translation, rotation, a static flag) with one comment mapping it
to a middleware message. No middleware crate is added. The message
sequence is a fixed in-file fixture, and each message is asserted against
its expected outcome.

What it shows, and the evidence for each:

| Step | Shown | Evidence |
|---|---|---|
| Stamp | `sec < 0` and `nanosec >= 1e9` are refused, never clamped | H6, Z6 (negative seconds); `transforms_io.md:159` (`nanosec >= 1e9` carries into seconds) |
| Frame names | An empty name is refused through `RegistryError::EmptyFrameName` (under D6's recommended site; the `validate` site would raise it at construction) | Part D; H5, Z1 |
| Insert | Per-element `add_transform`, so a refused message costs only itself | "Do not steer bridges to `add_transforms`" ([`../dependents/README.md`](../dependents/README.md#do-not)) |
| Insert errors | One `match`, one stated decision per variant (below) | H3, H4, Z2, Z11; three of four users stringify |
| Clock jump | The bridge compares each stamp with the newest it stored and clears with `remove_transforms_before` | H7, Z3, dimos Hazard 1; Part C item 5 |
| Lookup errors | `UnknownFrame`, `Disconnected`, `NotFoundAt` with `covered: Some` and with `covered: None`, told apart | Z7; error-blind waits in all four users |

The decision per insert error. Each is the adapter's policy, stated as
such; the crate has no `is_retryable`, on purpose (§6).

| Variant | Decision shown | Source |
|---|---|---|
| `NonFiniteValues`, `NonUnitRotation` | Drop and report. No repair | `error.rs:26-44` says where they are caught; "no repair" is the example's policy, and normalizing within a stated bound is left to outreach (§6) |
| `SelfReferentialFrame`, `CycleDetected` | Drop and report | No guidance found in the rustdoc or the study beyond the tree rule |
| `EmptyFrameName` | Drop and report | §5a. Not in the prototype, which predates Part D |
| `ReparentingNotSupported` | Drop, name both parents, surface it. Never call `reparent_frame` from the handler | "Do not resolve this error mechanically" (`error.rs:63-66`) |
| `StaticDynamicConflict` | Drop and surface it. Never `remove_frame` from the handler | Part C item 8 |
| any other | Reported as unexpected; no fixture message expects it, so one fails the per-message assert | `#[non_exhaustive]` |

What the example must not teach:

- **That errors detect a clock jump.** In 2.x an insert the window evicts
  returns `Ok`, so the bridge has to track its own newest stamp. The
  threshold that separates a jump from a late sample is the adapter's
  assumption, and the example says so.
- **That the clear is safe to apply mechanically.** One valid-looking
  message with a stale stamp wipes every dynamic frame. The jump
  reference in the prototype is global, not per publisher, and the check
  runs before the insert, so a message that triggers the clear and is
  then refused has still cleared. The example states these limits.
- **A cutoff "past every stored sample" without the strictness.**
  `remove_transforms_before` is strict, so the cutoff is one nanosecond
  past the newest stored stamp. Part C item 5 carries the same
  correction
  ([`sample-expired-contract.md` (f)](../v3-review/sample-expired-contract.md)).
  This is a 2.x form: in 3.0 the cutoff removes the sample at it, and
  the newest stored stamp is enough
  ([v3 item 12](../v3-review/REPORT.md#12-remove_transforms_before-removes-the-sample-at-the-cutoff)).
- **That `NotFoundAt` always means "wait".** In 2.x it takes precedence
  over `Disconnected` (`error.rs:162-163`), so a pair no chain connects
  can report it. The prototype's `Disconnected` case needed a static
  second tree for that reason.
- **A wait loop, normalization, or stripping a leading `/`.** The study
  keeps the first two out of the core because they are exact outside,
  and calls `/`-stripping a silent guess. The example leaves all three
  out so that it carries no policy beyond the ones it states. The
  "latest" lookup lives in `shared_latest`, once.

**Order.** Within 2.4, E3 lands last: after Part D and Part C item 5,
so it matches `EmptyFrameName` by name and agrees with the rustdoc. The
prototype predates Part D and carries its own `is_empty()` check, which
is the copy the variant removes.

### E4. Dev-dependencies

With asserts and `println!`, nothing uses `log` or `env_logger` (no hits
in `src`, `tests` or `benches`). Removing them drops 20 packages from
`Cargo.lock`. It needs two AGENTS.md edits: the dev-dependency list
(`AGENTS.md:93`) and the Style sentence "logging belongs in `examples/`"
(`:307-308`), which would then describe examples that do not log. `tokio`
stays for `shared_latest`; if that example moved to `std::sync::RwLock`
and threads, as the v3 prototype did for determinism, `tokio` and its
mention at `AGENTS.md:93` would go too (D13).

### Prototype

In [`prototypes/examples/`](prototypes/examples/), with the manifest,
README, gate and CI edits and the first AGENTS.md edit (`:93`) in
[`prototypes/examples-manifest-gate-readme.diff`](prototypes/examples-manifest-gate-readme.diff).
The Style sentence edit and `Cargo.lock` are not in the diff. Written
against 2.3.0, so `ingest.rs` carries the pre-Part-D `is_empty()` check
and has no `EmptyFrameName` arm. It is one combined change, E1 to E4
together. No piece was built alone, and asserts in the six existing
files (E1 without E2) were not written.

| File | Lines |
|---|---|
| `basic.rs` | 94 |
| `ingest.rs` | 301 |
| `shared_latest.rs` | 110 |
| `cross_time.rs` | 71 |

576 lines replace 657. `ingest.rs` is over half of the total.

- **Verified in a scratch clone at `7cc2a65`:** `cargo fmt --check`;
  nightly clippy in all four feature combinations and stable clippy in
  both `std` modes, with no new `#[allow]`; `cargo check --examples` on
  1.85 in both modes; every example in both modes; and
  `tests/test_all.sh` to `GATE PASSED`. All of it ran on the files as
  committed here, as one combined change.
- **Break test.** Changing one expected value in `basic` makes the run
  exit 101 in both modes. The other three examples were not break-tested.
- **Not verified:** the CI workflow edit (not executed); the Part D form
  of `ingest` (no implementation exists); `shared_latest` under load (its
  assertions do not depend on timing, and it passed four runs); how
  crates.io resolves the README's relative example links for the already
  published 2.3.0 page after a rename.

### Not proposed

- **A prose integration guide.** `latest_common_time` is documented
  (`README.md:100-103`) and shown in `std_full`, and transforms_io still
  left it unused across seven releases. The same doubt applies to an
  example. The difference is drift: prose rots silently, and an asserted
  example fails the gate.
- **Recipes as `tests/` files.** They gate well and teach badly: nobody
  looks there, and they would duplicate the examples.
- **A long doctest for the loop.** Doctests ship and show on docs.rs, so
  they are the right home for the two short recipes the v3 review lists
  for 2.x (a "latest" lookup, a checked ROS time conversion). The style
  rule that doc examples use `.unwrap()` does not fit a match by variant.
- **Outreach alone.** It fixes the two adapters and nothing for the next
  one. It stays worth doing, and the `ingest` example is what such a pull
  request would cite.
- **An example as a substitute for any Part C item.** Rustdoc is what
  ships and what is normative. Items 2, 4, 5 and 8 stay required.

## 6. Considered and not proposed

- **A cross-time result type beside `get_transform_at`** (revision 2's
  Part A). Moved to the v3 ledger; see §3.
- **Axis-angle, roll/pitch/yaw, and `Vector3` norm/dot/cross.** These can be
  built exactly on the public API, so the scope rule places them in a
  companion crate. (The preceding chat discussion argued otherwise; that
  reading of the rule was wrong.)
- **An inherent `Timestamp::checked_add`.** It would be a second spelling
  beside the `Result`-returning `+`, without removing the unidiomatic
  operator. The v3 review does not take the idiom fix (operators → named methods)
  into 3.0; it proposes a diagnostic attribute on `TimePoint` for 2.x
  instead ([Not in 3.0](../v3-review/REPORT.md#not-in-30); D17).
- **A finiteness check at the end of every lookup** (to remove `Ok(inf)` on
  ancestor-ward lookups). It needs inputs around 1e308. It contradicts the
  documented "not re-validated" contract, which is pinned by
  `an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts`,
  and it carves an exception out of an AGENTS.md invariant. It is now
  [item 4 of the v3 review](../v3-review/REPORT.md#4-no-lookup-returns-a-non-finite-value),
  which drops the `Rotation` and bound route. Whether it is a 2.x fix
  instead is the maintainer's call (v3 decision 7); without that ruling
  it is not 2.4 work.
- **Root re-exports, a prelude, an `is_retryable` helper on errors, an
  `unbounded()` alias.** Each is speculative or a second spelling.
- **An f32 module.** f32 is a Non-Goal (README, crate root, AGENTS.md), so
  any f32 work, in any release, starts with the maintainer amending it.
  Only after that would a concrete f32 module be additive in a 2.x minor.
  A hardware measurement of `f64` cost on the target cores, which has
  not been made ([`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md),
  "Not done"), would inform that decision and does not replace it.
- **A sentence on ingesting rotations from looser sources** (dependents
  rec 12). The failure is loud: `NonUnitRotation` at construction
  (transforms_io H8, roslibrust_transforms Z5). The `UNIT_NORM_TOLERANCE`
  rustdoc already states the bound (`src/geometry/transform/mod.rs:18-30`).
  Outreach, not 2.4
  ([`../dependents/README.md`](../dependents/README.md#outreach)).
- **Negative seconds for pre-epoch `SystemTime`** (dependents rec 16). NaN
  is the documented contract (`src/time/system_time.rs:28-30`). It affects
  `Display` only, and its one user is the unmerged dimos PR.
- **A maximum interpolation gap.** Reachable in all three studied
  dependents (`../dependents/probes/transforms_io.rs` P9, `Ok(4.0)`;
  `roslibrust_transforms.rs` P10, `Ok(x=4)`; dimos Hazard 2) and not
  buildable outside, but nobody has asked, and it needs a
  `latest_common_time` redesign. It stays "design doc first", per frame
  ([`dimos.md`](../dependents/dimos.md#design-doc-maximum-interpolation-gap)).
  The v3 review keeps it open and out of 3.0 as well
  ([Not in 3.0](../v3-review/REPORT.md#not-in-30)).
- **The v3 review's other 2.x items**
  ([Ships in 2.x](../v3-review/REPORT.md#ships-in-2x)). Not put to the
  revision 4 question, and not part of this proposal until the
  maintainer rules on them (D17): the Point-family removal notice in the
  changelog and README (v3 decisions 3 and 4); a
  `#[diagnostic::on_unimplemented]` attribute on `TimePoint`; three doc
  sentences (`Display` of time-carrying errors is lossy and the payload
  exact; `get_transform_at` errors may name the fixed frame; `f64` is
  software on the core classes the gate builds); two doc recipes (a
  "latest" lookup, a checked ROS time conversion); a frozen-bytes serde
  case with a non-identity rotation; deleting the `latest_timestamp`
  field, `Buffer.child` and `InsertError::ChildFrameMismatch`; and, from
  [Not in 3.0](../v3-review/REPORT.md#not-in-30), wording unbounded
  interpolation as the default, not a guarantee.

## 7. Tests

Every new test below must fail if its feature is removed or broken. Part B
fixes no bug in an existing method, so it adds no regression test for old
code. Part D fixes one, so its first five tests fail on 2.3.0, as
AGENTS.md requires of a correctness fix. Part A's tests
moved with it to the v3 addendum.

Part D, in `src/core/registry/tests.rs` with the `translated` helper
(`:2165`):

- `an_empty_frame_name_cannot_join_unrelated_trees`: statics
  `("", "camera")` x=1 and `("", "lidar")` x=3 each return
  `EmptyFrameName`, and `camera`→`lidar` returns `UnknownFrame("camera")`
  (2.3.0: `Ok(2.0)`).
- `add_transform_rejects_an_empty_parent_or_child`, beside
  `add_transform_rejects_self_referential_frames` (`:1245`): `("", "x")`,
  `("x", "")` and `("", "")` each return `EmptyFrameName`, the last not
  `SelfReferentialFrame`, and the registry is unchanged.
- `add_transforms_rejects_a_batch_naming_an_empty_frame`:
  `[("map", "base"), ("base", "")]` returns `EmptyFrameName` and stores
  nothing.
- `an_empty_parent_is_reported_before_the_pinned_parent_and_kind`: with
  dynamic `("map", "base")` stored, `add_transform` of a dynamic and of
  a static `("", "base")` each return `EmptyFrameName`, not
  `ReparentingNotSupported` or `StaticDynamicConflict`;
  `add_transforms([("", "base")])` (stored-frame branch) and
  `add_transforms([("map", "arm"), ("", "arm")])` (created-frame branch)
  do the same and store nothing. A non-finite transform naming `""`,
  built with `Transform::unvalidated`, returns `NonFiniteValues`.
- `reparent_frame_rejects_a_self_referential_or_non_finite_seed`
  (`:3073`) gains a seed with parent `""`, which returns
  `EmptyFrameName`, and a seed with child `""`, which returns
  `UnknownFrame("")`: the unknown-child check runs before the seed's
  insert checks (§5a).
- `same_frame_lookup_returns_identity` (`:2062`) gains `""` in its frame
  list. It passes on 2.3.0 too; it pins that `""` is not special at
  lookup.

Part B:

- A frame mismatch, a time mismatch, and the `Static` exemption, each
  matching `Point::transform` on the same inputs.
- The relabel happens on success, and nothing changes on error.
- An empty slice still checks and relabels.
- Property: for random transforms and point sets, `transform_positions` and
  per-`Point` `transform` agree bit for bit.

All tests run in both feature modes. Two tests pin the site:
`construction_and_validation_still_accept_an_empty_frame_name` in the
transform tests (`Transform::new("", "x", ..)`, `static_between` and
`validate` return `Ok`), and, under the `serde` feature, a `serde_json`
roundtrip of a transform naming `""`, which decodes. Both pass on 2.3.0; they
fail if the check moves to `Transform::validate`. The existing serde
suites are otherwise unchanged.

Part E adds no test file. The examples are the check: each asserts its
results and runs in both feature modes in the gate (§5b). The acceptance
test for E1 is the break test, per example: one wrong expected value
must make it exit non-zero.

## 8. Decisions for the maintainer

| ID | Decision | Recommendation |
|---|---|---|
| D1 | Deprecate `get_transform_at` with an attribute, or redirect in rustdoc only | Withdrawn in revision 3: no replacement ships in 2.x (§3) |
| D2 | Grant Part B a scope exception | No for 2.4: no studied dependent shows the need (§4.1). Part C item 6 applies instead. Revisit on a demonstrated need |
| D3 | Name: `transform_positions` | As proposed. It mirrors `Transformable::transform` |
| D4 | Ship A without B if D2 is refused | Withdrawn in revision 3: Part A no longer ships in 2.x |
| D5 | Reject empty frame names in 2.x | Decided by the maintainer 2026-11-06: non-breaking, ships in 2.4 (§5a) |
| D6 | Enforcement site: the insert boundary (`Buffer::check`, flat `RegistryError::EmptyFrameName`) or `Transform::validate` (`TransformError::EmptyFrameName`) | The insert boundary. The hazard is registry-only, the sibling self-reference rule lives there, and decoding is untouched (§5a). The ruling covers rejection, not the site |
| D7 | Payload: unit, or name the empty side | Unit, as `SelfReferentialFrame` is. Frame fields on insert errors are a v3 ledger candidate for all of them at once (now a [ledger row](../v3-proposals/transforms-v3-addendum-dependents.md), 2026-11-06), since adding one to a shipped variant is a break. The v3 review reopened the question ([item 6](../v3-review/REPORT.md#6-error-shapes) and its decision 5: leaning yes, child frame only) and asks that this payload be decided with it. Unit in 2.4 keeps both outcomes open, since unit to struct needs the major either way |
| D8 | A property test for the rule | No: the check is a string comparison and §7 covers every insert branch. If wanted, map one name of the `f0..f6` alphabet in `tests/properties.rs` (`:123-165`) to `""` |
| D9 | E1: make every example fail on a wrong result | Decided 2026-11-11: yes, in 2.4. Recommended without a commit of its own: the E2 and E3 files are written with asserts (§5b, E1) |
| D10 | E2: task-named files run in both modes; delete the six; `Timestamp::now()` leaves the examples, and `remove_transforms_before` leaves them until E3 brings it back in `ingest`; edit the gate and the CI job together | Decided 2026-11-11: yes, in 2.4, as its own commit. The gate edit it implies (six runs stay six under new names; E3's commit makes them eight) is not a weakening, and is flagged for explicit sign-off when the change is reviewed |
| D11 | Remove `Point` from the examples in 2.x | No. `basic` leads with the registry and applies through `Point`. Revisit with the v3 decision on the Point family; a removal notice, if taken, goes in the changelog and README |
| D12 | E3: the `ingest` example in 2.x | Decided 2026-11-11: yes, in 2.4, last: after Part D and Part C item 5 |
| D13 | E4: drop `log` and `env_logger`; `tokio` or threads for `shared_latest` | Decided 2026-11-11 for `log` and `env_logger`: dropped in 2.4, with both AGENTS.md edits. `tokio` is still open, the maintainer's call: roslibrust_transforms uses `tokio::sync::RwLock` (the other users' lock types were not checked); threads are deterministic and would remove one more dev-dependency |
| D14 | Part C item 1 | Superseded: D10 is decided for 2.4 |
| D15 | Rename `std_full` to `shared_latest` and `std_advanced` to `cross_time` | Decided with D10. The released CHANGELOG entries that name the old files stay as written. The effect on the published 2.3.0 README links is unverified |
| D16 | Reword Part C item 5's recovery sentence ("a cutoff past every stored sample") | Yes, from [`sample-expired-contract.md` (f)](../v3-review/sample-expired-contract.md): the cutoff is strict and none exists for a sample at the time type's maximum, `remove_frame` is the per-frame route, and a jump larger than `max_age` has a tail. Applied to item 5 in revision 7, because D12 orders `ingest` after it; the maintainer can still veto the wording. The strict cutoff is 2.x only ([v3 item 12](../v3-review/REPORT.md#12-remove_transforms_before-removes-the-sample-at-the-cutoff), decided 2026-11-11) |
| D17 | Which of the v3 review's "Ships in 2.x" items join 2.4 | Maintainer's call; none is assessed here (§6) |

## 9. Release obligations

- Bump the version to 2.4.0 and add a `## [2.4.0]` CHANGELOG section:
  `RegistryError::EmptyFrameName` under Added, the insert rejection and
  its hazard under Changed. 2.4.0 is a minor release whether or not Part
  B ships, because Part D adds a public variant.
- Part C: one CHANGELOG line under Changed naming the corrected rustdoc
  (items 2, 3, 4, 5, 8, 9, 10, and 6 if D2 is refused), as earlier
  documentation corrections got (2.1.2, 2.1.3). If D2 is granted
  instead, `Transform::transform_positions` goes under Added, with a
  `# Errors` section naming the `TransformError` variants
  `Point::transform` returns, and item 6 is dropped.
- Add a MIGRATION note under "Coming from 2.0.0": since 2.4.0,
  `add_transform` and `add_transforms` reject a transform whose parent or
  child is `""` with `RegistryError::EmptyFrameName`, as does
  `reparent_frame` for an empty new parent; check names at ingest (an unset ROS
  `frame_id`); constructors and deserialization still accept `""`. Add the
  variant to the topology row of the 1.x error table (`MIGRATION.md:130`).
  Retitle the file "# Migrating to 2.4" (`MIGRATION.md:1`), and check
  its opening sentence, "From 2.0.0, no API migration is required"
  (`:3`), against the new note: an insert 2.3.0 accepted is now refused.
  Nothing is deprecated.
- Part D needs one AGENTS.md edit, flagged to the maintainer: the
  frame-tree invariant gains "no frame name is empty" beside "a frame
  cannot be its own parent" (`AGENTS.md:148`), as a tree rule checked at
  insert. The identity, flat-error and validation bullets stay true under
  D6's recommended site; the `validate` site would also edit the
  validation list and the identity clause. Parts B and C need none.
- Part E (decided for 2.4; examples are not packaged, so this is
  repository content): one CHANGELOG line; the
  README "Examples" table and run commands (`README.md:178-195`);
  `tests/test_all.sh:37-42` and `.github/workflows/tests.yml:176-187`,
  edited together, in the E2 and E3 commits; the `[[example]]` blocks (`Cargo.toml:60-88`). E4 also
  edits `[dev-dependencies]`, `Cargo.lock` and AGENTS.md (`:93`,
  `:307-308`), flagged to the maintainer. No MIGRATION entry: no API
  changes. Analysis documents that name the old files
  (`../v3-proposals/transforms-v3-addendum-cross-time.md:123`,
  `../v3-warrant/REPORT.md:113-114`) or count them
  (`../v2-feasibility/REPORT.md:142`) are historical and stay.
- Run the full gate. Run `cargo semver-checks check-release --baseline-rev v2.3.0`
  and confirm the result matches the declared bump. A variant added to a
  `#[non_exhaustive]` enum is no breaking lint; the `Ok` → `Err`
  tightening is invisible to the tool and rests on D5.
