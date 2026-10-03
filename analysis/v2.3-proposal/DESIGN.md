# transforms 2.3: design proposal

Date: 2026-10-03. Draft revision: 1. Baseline: master `aa29557`
(released v2.2.0).

This document proposes the contents of a 2.3.0 minor release. It follows from
the verdict in [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md):
no v3 now, and the reachable value ships additively. Nothing here is
implemented. AGENTS.md applies throughout. Every part grows the public
surface, so this document is the maintainer note that AGENTS.md requires.
It does not authorize implementation, merge, or release.

## 1. Summary

| Part | What | Public surface | Status |
|---|---|---|---|
| A | A cross-time result type with checked application; `get_transform_at` deprecated | 1 type, 1 method | Proposed |
| B | Checked application of a transform to plain positions | 2 methods | Needs a scope decision (§4.1) |
| C | Documentation corrections | none | Proposed |

Everything is additive under SemVer. No signature, variant, trait bound, or
public path changes, and no declared behaviour changes. `cargo semver-checks`
should report a minor bump in all four feature combinations.

## 2. Motivation

All three parts close the same gap: **the checked path rejects correct use
or does not reach it, so callers leave it.** For a crate whose worst failure
is a plausible wrong answer, a checked path that is inconvenient to use
protects nobody.

Probe-verified at `aa29557`:

1. **Cross-time application is inverted.** Take `x = get_transform_at(tgt,
   t2, src, t1, fixed)`.
   - Applying `x` to a point in `src` at `t1` (the correct input) fails with
     `TimestampMismatch`.
   - Applying `x` to a point in `src` at `t2` (the wrong input) returns `Ok`
     and mislabels the result.
   - Inserting `x` succeeds and serves the t1 relationship as if it held at
     t2. This is pinned by
     `cross_time_result_keeps_only_target_stamp_and_remains_insertable`.
2. **The hazard reaches a dependent.** `roslibrust_transforms` 0.26.0
   re-exports `get_transform_at` (`src/lib.rs:555`). Its rustdoc tells
   callers the result "converts points from `source_frame` at
   `source_time`", which is exactly the use that `Transformable` rejects.
3. **No checked route exists for raw positions.** Transforming a `Vector3`
   or a point cloud means writing `rotation().rotate_vector(v) +
   translation()` by hand, which skips the frame and time checks. The
   `get_transform_at` rustdoc example does exactly this
   (`src/core/registry/mod.rs:318`).

## 3. Part A: cross-time result type

### 3.1 Placement

The cross-time lookup is already a core capability. This part changes the
type of its answer and adds no new capability. The defect is in a value the
core itself returns: an insertable `Transform` that carries a false stamp. No
caller-side wrapper can make that value non-insertable. AGENTS.md's scope
rule therefore places the fix in the core.

### 3.2 API

```rust
// transforms::geometry
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTimeTransform<T = Timestamp>
where
    T: TimePoint,
{
    target_frame: String,
    target_time: T,
    source_frame: String,
    source_time: T,
    isometry: Isometry,
}

impl<T: TimePoint> CrossTimeTransform<T> {
    pub fn target_frame(&self) -> &str;
    pub fn target_time(&self) -> T;
    pub fn source_frame(&self) -> &str;
    pub fn source_time(&self) -> T;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Quaternion;

    /// Maps a source-frame point observed at the source time into the
    /// target frame at the target time.
    pub fn transform_point(&self, point: &mut Point<T>) -> Result<(), TransformError>;
}

impl<T: TimePoint> Registry<T> {
    pub fn get_cross_time_transform(
        &self,
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
    ) -> Result<CrossTimeTransform<T>, RegistryError<T>>;
}
```

### 3.3 Semantics

- **Lookup.** `get_cross_time_transform` takes the same arguments as
  `get_transform_at`, in the same order, with the same errors and the same
  fixed-frame short-circuits. `process_get_transform_at` changes to return
  the `Isometry`. `get_transform_at` wraps it as today, so the old method's
  results and trace lines stay byte-identical.
- **`transform_point` checks before it mutates.**
  - `point.frame` must equal `source_frame`, or it fails with
    `IncompatibleFrames { expected: source_frame, found }`.
  - `point.timestamp` must equal `source_time`, or it fails with
    `TimestampMismatch { lhs: point, rhs: source_time }`.
  - There is no `Stamp::Static` exemption: both instants are explicit.
  - On error, the point is unchanged.
- **On success**, `transform_point` maps the position as `R·p + t` and the
  orientation as `R·q`, then sets the frame to `target_frame` and the
  timestamp to `target_time`. This is the same arithmetic as `Point`'s
  `Transformable` impl.
- **What the type cannot do.** It has no `Mul`, `inverse`, `interpolate`,
  `validate`, or conversion into `Transform`, and it does not implement
  serde. It therefore cannot be inserted, composed as a single-time
  transform, or serialized with its provenance lost.
- **Numbers.** The result is derived and not re-validated, like every lookup
  (AGENTS.md). The existing `Transformable` precondition text applies
  unchanged.
- **Vocabulary.** The accessors say `target`/`source`, matching the query
  arguments. There is no `parent`/`child`, because no tree edge is implied.
- **No `fixed_frame` field.** It does not affect any check, and the caller
  passed it in. It is added only if a need appears.

### 3.4 Example

```rust
let x = registry.get_cross_time_transform("camera", t2, "object", t1, "map")?;
let mut detection = Point::new(p, q, t1, "object");
x.transform_point(&mut detection)?; // now in "camera" at t2
assert!(x.transform_point(&mut Point::new(p, q, t2, "object")).is_err());
```

### 3.5 Deprecating `get_transform_at`

The proposal is
`#[deprecated(since = "2.3.0", note = "use get_cross_time_transform, whose result keeps both instants")]`.

- A deprecation is a lint, not a SemVer break.
- The tests that pin the old method's behaviour keep it under test until v3.
  They need a scoped `#[expect(deprecated)]`, which is a new allowance.
  AGENTS.md requires the maintainer to approve that (§8, D1).
- The alternative is a rustdoc redirect with no attribute. It needs no new
  allowance but does not warn existing callers. One of those callers is a
  dependent that re-exports the method.

Removal is a v3-ledger item.

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
- **Every caller with non-`Point` data copies it by hand or skips it.** The
  evidence in §2 shows they skip it: the crate's own rustdoc example does.
  That is the charter's "pushed-out invariant logic becomes hand-rolled
  wrong copies".
- **The relabel is the half callers forget.** The method writes the output
  frame, so the caller cannot forget it.

The case against: no user or dependent has asked for it, and the
demonstrated-need rule is not met.

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

impl<T: TimePoint> CrossTimeTransform<T> {
    /// Same contract, but it also moves `timestamp` to the target time.
    pub fn transform_positions(
        &self,
        frame: &mut String,
        timestamp: &mut T,
        positions: &mut [Vector3],
    ) -> Result<(), TransformError>;
}
```

### 4.3 Semantics

- **The checks match `Point::transform`, and run before any mutation.** On
  error, the frame, timestamp, and positions are all unchanged.
- **The frame is taken as `&mut String` deliberately.** The method that
  checks the input label also writes the output label, as `Point` does. A
  caller cannot leave a cloud labelled `camera` after mapping it into `base`.
- **Why the `timestamp` parameters differ.** A single-time transform
  preserves the timestamp (by value). A cross-time result moves it
  (`&mut T`). The difference mirrors the semantics.
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
(`README.md:114`).

1. **Example direction comments.** In `examples/std_minimal.rs:33,57` and
   `examples/no_std_full.rs:37,61`, comments say "transform from camera to
   base" above `Transform::new("base", "camera", …)`. Rewrite them as
   "camera expressed in base (parent base, child camera)". Rename
   `camera_to_base_*` to `base_camera_*` (the `t_a_b` style the tests use).
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
7. **Cross-time references follow Part A.** Point these at the new method:
   - `README.md:101-103`;
   - `src/lib.rs:21-22`;
   - the `Transformable` cross-time paragraph;
   - `MIGRATION.md:181-186` (add a 2.3 note; do not rewrite 2.0 history);
   - `examples/std_advanced.rs:105-135` and
     `examples/no_std_advanced.rs:106-135`, which then apply the result to
     a `Point` instead of printing `translation()`.

## 6. Considered and not proposed

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

Every new test below must fail if its feature is removed or broken. Part A
adds no regression test for old code, because it fixes no bug in an existing
method: the old method keeps its pinned behaviour.

Part A:

- The happy path matches the manual formula on the `std_advanced` scenario,
  bit for bit, against `get_transform_at` geometry.
- `transform_point` rejects a target-time point when the instants differ.
  This is the inverted case from §2.
- `transform_point` rejects a wrong source frame.
- On error, the point is unchanged.
- The fixed-frame short-circuits: an endpoint equal to `fixed_frame`, and
  both endpoints equal to it.
- The errors match `get_transform_at`'s for the same arguments: unknown,
  disconnected, not found.
- `tests/properties.rs`: over random trees and instants, a source-time point
  mapped by `transform_point` equals the manual formula, and the result's
  accessors echo the request.
- A compile-fail check is not needed: `CrossTimeTransform` has no `Into`
  conversion into `Transform`, so insertion does not type-check.

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
| D1 | Deprecate `get_transform_at` with an attribute, which needs a scoped `#[expect(deprecated)]` on its pinned tests, or redirect in rustdoc only | Attribute: it warns the dependent that re-exports the method |
| D2 | Grant Part B a scope exception | Yes: the check-and-relabel rule is invariant logic the crate already owns once |
| D3 | Names: `CrossTimeTransform` / `get_cross_time_transform` / `transform_point` / `transform_positions` | As proposed. They mirror `Transformable::transform` and the query vocabulary |
| D4 | Ship A without B if D2 is refused | Yes. A stands on its own |

## 9. Release obligations

- Bump the version to 2.3.0. Add a `## [2.3.0]` CHANGELOG section and a
  MIGRATION note covering the deprecation and the new types.
- AGENTS.md:
  - mention `CrossTimeTransform` in the architecture's `geometry` line;
  - add one sentence under the correctness invariants: a cross-time result
    has its own type and is never a `Transform`.
- Run the full gate. Run `cargo semver-checks check-release --baseline-rev v2.2.0`
  and confirm the result is a minor bump.
