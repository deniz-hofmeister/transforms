# transforms 2.4: design proposal

Date: 2026-10-03. Draft revision: 3 (2026-11-03). Baseline: master
`aa29557` (released v2.2.0).

This document proposes the contents of a 2.4.0 minor release. It follows from
the verdict in [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md):
no v3 now, and the reachable value ships additively. Nothing here is
implemented. AGENTS.md applies throughout. Part B grows the public surface,
so this document is the maintainer note that AGENTS.md requires. It does not
authorize implementation, merge, or release.

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
([`REPORT.md`](../dimos-lessons/REPORT.md)). The part letters and decision
IDs are kept, so references from other reports stay valid.

## 1. Summary

| Part | What | Public surface | Status |
|---|---|---|---|
| A | A cross-time result type; `get_transform_at` deprecated | — | Moved to the v3 ledger (§3) |
| B | Checked application of a transform to plain positions | 1 method | Needs a scope decision (§4.1) |
| C | Documentation corrections | none | Proposed |

Everything is additive under SemVer. No signature, variant, trait bound, or
public path changes, and no declared behaviour changes. `cargo semver-checks`
should report a minor bump in all four feature combinations if Part B
ships. Without Part B, nothing here grows the public surface (§9).

## 2. Motivation

The parts that remain close one gap: **the checked path does not reach the
data callers actually transform, so they leave it.** For a crate whose worst
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
   2.x already documents it everywhere it is mentioned: the
   `get_transform_at` rustdoc ("Temporal metadata limitation"), the crate
   docs, the `Transform` and `Transformable` rustdoc, the README and
   MIGRATION.
3. **The hazard reaches a dependent.** `roslibrust_transforms` 0.26.0
   re-exports `get_transform_at` (`src/lib.rs:555`). Its rustdoc tells
   callers the result "converts points from `source_frame` at
   `source_time`". That is true of the geometry, but it does not say that
   the checked path rejects that use, or that the result must not be
   inserted.

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
  documented in every place listed in §2.2. Part C item 7 of revision 2
  existed only to repoint those references at the new method, so it is
  dropped.
- **Outreach, outside the crate.** Ask `roslibrust_transforms` to link
  `get_transform_at`'s application restrictions from its re-export. This
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
- The first non-adapter user, dimos, uses neither `Point` nor
  `Transformable`, and the dimos report recommends deferring Part B.

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
   answer to "what is the newest instant I can look up?".
3. **E0283.** Repeat the type page's `Registry::<Timestamp>::new()` note
   (`src/core/registry/mod.rs:87`) on `Registry::new`, `Registry::with_max_age`
   and `Transform::static_between`. These are where an all-static setup
   fails to infer.
4. **`Stamp` versus `T`.** Add one sentence to the `latest_common_time`
   rustdoc: on `Stamp::Static`, any instant serves, so pass the caller's own
   time. Do not copy the example's `.at().unwrap()` for chains that may be
   static.
5. **Wrapping clocks.** Add one paragraph to the `TimePoint` rustdoc: a
   wrapping hardware counter violates the total `Ord` the trait requires, and
   the crate cannot detect it. Widen it to a monotonic 64-bit count before
   implementing the trait.
6. **If Part B is rejected,** add a "Applying a transform to your own data"
   recipe to the `Transformable` rustdoc. It should spell out the check and
   the relabel that `Point` performs.
7. **Withdrawn in revision 3.** It repointed the cross-time references at
   Part A's method. They stay as they are (§3).

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
- **An f32 module.** It is additive and possible in a 2.x minor, but it
  waits on the hardware measurement and on a maintainer decision about the
  Non-Goal.

## 7. Tests

Every new test below must fail if its feature is removed or broken. Part B
fixes no bug in an existing method, so it adds no regression test for old
code. Part A's tests moved with it to the v3 addendum.

Part B:

- A frame mismatch, a time mismatch, and the `Static` exemption, each
  matching `Point::transform` on the same inputs.
- The relabel happens on success, and nothing changes on error.
- An empty slice still checks and relabels.
- Property: for random transforms and point sets, `transform_positions` and
  per-`Point` `transform` agree bit for bit.

All tests run in both feature modes. The serde suites are unchanged.

## 8. Open decisions for the maintainer

| ID | Decision | Recommendation |
|---|---|---|
| D1 | Deprecate `get_transform_at` with an attribute, or redirect in rustdoc only | Withdrawn in revision 3: no replacement ships in 2.x (§3) |
| D2 | Grant Part B a scope exception | Yes: the check-and-relabel rule is invariant logic the crate already owns once. The case against is in §4.1 |
| D3 | Name: `transform_positions` | As proposed. It mirrors `Transformable::transform` |
| D4 | Ship A without B if D2 is refused | Withdrawn in revision 3: Part A no longer ships in 2.x |

## 9. Release obligations

- If Part B ships, bump the version to 2.4.0 and add a `## [2.4.0]`
  CHANGELOG section. No MIGRATION note is needed: nothing is deprecated and
  no behaviour changes.
- If D2 is refused, Part C alone grows no public surface. It ships as a
  patch release, or rides with other 2.4 items such as the dimos report's
  empty-frame-name rejection.
- AGENTS.md needs no change for either part.
- Run the full gate. Run `cargo semver-checks check-release --baseline-rev v2.3.0`
  and confirm the result matches the declared bump.
