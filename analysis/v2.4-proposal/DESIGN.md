# transforms 2.4: design proposal

Date: 2026-10-03. Draft revision: 5 (2026-11-10). Baseline: master
`aa29557` (released v2.2.0) for the §2 probes; master `f9770d2` (released
v2.3.0 plus docs) for the dependent evidence and Part D.

This document proposes the contents of a 2.4.0 minor release. It follows from
the verdict in [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md):
no v3 now, and the reachable value ships additively. Nothing here is
implemented. AGENTS.md applies throughout. Parts B and D grow the public
surface, so this document is the maintainer note that AGENTS.md requires.
It does not authorize implementation, merge, or release.

Revision 2 retargeted the proposal from 2.3.0 to 2.4.0: 2.3.0 shipped
`Registry::add_transforms` alone (`76787c2`). The probes in §2 ran at
`aa29557` and were not re-run; the file and line references below point
into the v2.3.0 tree.

Revision 3 moves Part A, the cross-time result type, to the v3 ledger. An
additive type beside `get_transform_at` does not close the hazard it
targets, and it would commit 2.x to a second cross-time API designed ahead
of the v3 work it belongs with (§3). Its design now lives in
[`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md).
This supersedes the dimos report's "keep Parts A and C"
([`dimos.md`](../dependents/dimos.md)). The part letters and decision
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

## 1. Summary

| Part | What | Public surface | Status |
|---|---|---|---|
| A | A cross-time result type; `get_transform_at` deprecated | — | Moved to the v3 ledger (§3) |
| B | Checked application of a transform to plain positions | 1 method | Needs a scope decision (§4.1) |
| C | Documentation corrections | none | Proposed |
| D | Reject empty frame names at insert | 1 variant | Decided 2026-11-06: non-breaking; site open (§5a, D6) |

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
   - Inserting `x` succeeds and serves the t1 relationship as if it held at
     t2. This is pinned by
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
   (`lib.rs:518-547`), so a `get_transform_at` result passed to it
   overwrites the true sample and is broadcast with the false stamp
   (`../dependents/probes/roslibrust_transforms.rs` P5, V5;
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
- **It is designed ahead of its context.** The result type's accessors and
  application interact with the v3 `Rotation` newtype, a public
  `Isometry`, the move from operators to named methods, and the single
  target/source vocabulary (v3-3 §1, §2 and §4; v3-1 #7 for the
  operators). A 2.4 type would either
  constrain those choices or be broken again by them. Minimalism is
  measured in exported commitments.

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

1. **Example direction comments.** In `examples/std_minimal.rs:33,57` and
   `examples/no_std_full.rs:37,61`, comments say "transform from camera to
   base" above `Transform::new("base", "camera", …)` and "transform from
   base to map" above `Transform::new("map", "base", …)`. Rewrite them as
   "camera expressed in base (parent base, child camera)" and likewise for
   base in map. Rename `camera_to_base_*` to `base_camera_*` and
   `base_to_map` to `map_base` (the `t_a_b` style the tests use).
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
4. **`Stamp` versus `T`.** Add one sentence to the `latest_common_time`
   rustdoc: on `Stamp::Static`, any instant serves, so pass the caller's own
   time. Do not copy the example's `.at().unwrap()` for chains that may be
   static.
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
   does without `max_age`. Whoever owns the clock must detect the jump
   and call `remove_transforms_before` with a cutoff past every stored
   sample." Link, do not repeat, that method's text on the strict cutoff
   and the expiry reset (`mod.rs:577-582`). Cause: the newest timestamp
   is a running max, and eviction runs after the insert is accepted
   (`src/core/buffer/mod.rs:316-321`). Evidence: transforms_io H7
   (`../dependents/probes/transforms_io.rs` P6,
   `transforms_io_verify.rs` N3), roslibrust_transforms Z3
   (`roslibrust_transforms.rs` P3a, P3b), dimos Hazard 1.
6. **If Part B is rejected,** add a "Applying a transform to your own data"
   recipe to the `Transformable` rustdoc. It should spell out the check and
   the relabel that `Point` performs.
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

## 6. Considered and not proposed

- **A cross-time result type beside `get_transform_at`** (revision 2's
  Part A). Moved to the v3 ledger; see §3.
- **Axis-angle, roll/pitch/yaw, and `Vector3` norm/dot/cross.** These can be
  built exactly on the public API, so the scope rule places them in a
  companion crate. (The preceding chat discussion argued otherwise; that
  reading of the rule was wrong.)
- **An inherent `Timestamp::checked_add`.** It would be a second spelling
  beside the `Result`-returning `+`, without removing the unidiomatic
  operator. The idiom fix (operators → named methods) is a v3-ledger item.
- **A finiteness check at the end of every lookup** (to remove `Ok(inf)` on
  ancestor-ward lookups). It needs inputs around 1e308. It contradicts the
  documented "not re-validated" contract, which is pinned by
  `an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts`,
  and it carves an exception out of an AGENTS.md invariant. Left to the
  v3 `Rotation`/bound work.
- **Root re-exports, a prelude, an `is_retryable` helper on errors, an
  `unbounded()` alias.** Each is speculative or a second spelling.
- **An f32 module.** f32 is a Non-Goal (README, crate root, AGENTS.md), so
  any f32 work, in any release, starts with the maintainer amending it.
  Only after that would a concrete f32 module be additive in a 2.x minor.
  The hardware measurement informs that decision and does not replace it.
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

## 7. Tests

Every new test below must fail if its feature is removed or broken. Part B
fixes no bug in an existing method, so it adds no regression test for old
code. Part D fixes one, so its first four tests fail on 2.3.0, as
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
- `reparent_frame_rejects_a_self_referential_or_non_finite_seed`
  (`:3073`) gains a seed with parent `""`, which returns `EmptyFrameName`.
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

All tests run in both feature modes. The serde suites are unchanged:
under D6's recommended site, decoding still accepts `""`.

## 8. Open decisions for the maintainer

| ID | Decision | Recommendation |
|---|---|---|
| D1 | Deprecate `get_transform_at` with an attribute, or redirect in rustdoc only | Withdrawn in revision 3: no replacement ships in 2.x (§3) |
| D2 | Grant Part B a scope exception | No for 2.4: no studied dependent shows the need (§4.1). Part C item 6 applies instead. Revisit on a demonstrated need |
| D3 | Name: `transform_positions` | As proposed. It mirrors `Transformable::transform` |
| D4 | Ship A without B if D2 is refused | Withdrawn in revision 3: Part A no longer ships in 2.x |
| D5 | Reject empty frame names in 2.x | Decided by the maintainer 2026-11-06: non-breaking, ships in 2.4 (§5a) |
| D6 | Enforcement site: the insert boundary (`Buffer::check`, flat `RegistryError::EmptyFrameName`) or `Transform::validate` (`TransformError::EmptyFrameName`) | The insert boundary. The hazard is registry-only, the sibling self-reference rule lives there, and decoding is untouched (§5a). The ruling covers rejection, not the site |
| D7 | Payload: unit, or name the empty side | Unit, as `SelfReferentialFrame` is. Frame fields on insert errors are a v3 ledger candidate for all of them at once (now a [ledger row](../v3-proposals/transforms-v3-addendum-dependents.md), 2026-11-06), since adding one to a shipped variant is a break |
| D8 | A property test for the rule | No: the check is a string comparison and §7 covers every insert path. If wanted, map one name of the `f0..f6` alphabet in `tests/properties.rs` (`:123-165`) to `""` |

## 9. Release obligations

- Bump the version to 2.4.0 and add a `## [2.4.0]` CHANGELOG section:
  `RegistryError::EmptyFrameName` under Added, the insert rejection and
  its hazard under Changed. 2.4.0 is a minor release whether or not Part
  B ships, because Part D adds a public variant.
- Add a MIGRATION note under "Coming from 2.0.0": since 2.4.0,
  `add_transform` and `add_transforms` reject a transform whose parent or
  child is `""` with `RegistryError::EmptyFrameName`, as does
  `reparent_frame` for an empty new parent; check names at ingest (an unset ROS
  `frame_id`); constructors and deserialization still accept `""`. Add the
  variant to the topology row of the 1.x error table (`MIGRATION.md:130`).
  Nothing is deprecated.
- Part D needs one AGENTS.md edit, flagged to the maintainer: the
  frame-tree invariant gains "no frame name is empty" beside "a frame
  cannot be its own parent" (`AGENTS.md:148`), as a tree rule checked at
  insert. The identity, flat-error and validation bullets stay true under
  D6's recommended site; the `validate` site would also edit the
  validation list and the identity clause. Parts B and C need none.
- Run the full gate. Run `cargo semver-checks check-release --baseline-rev v2.3.0`
  and confirm the result matches the declared bump. A variant added to a
  `#[non_exhaustive]` enum is no breaking lint; the `Ok` → `Err`
  tightening is invisible to the tool and rests on D5.
