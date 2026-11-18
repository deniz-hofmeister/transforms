# Differential harness: transforms 2.4.0 against v3

DESIGN.md, "Acceptance", requires "a differential harness against 2.4, kept
outside the crate, with the declared differences listed". This is that
harness. It is a standalone, unpublished cargo package: it depends on the
published `transforms` 2.4.0 (renamed `transforms_v2`) and on the v3 working
tree by path, and it is not part of the crate, its tests or its gate.

## Running it

```sh
cd analysis/v3-consensus/differential
cargo run --release                          # default seed, 300,000 operations
cargo run --release -- --seed 7 --ops 3000000
cargo run --release -- --seed 7 --scenario 1143   # replay one scenario
```

The first build downloads `transforms` 2.4.0 from crates.io. A default run
takes about one second. The run is deterministic: the generator is a
splitmix64 stream, each scenario is seeded from `(seed, scenario index)`,
and nothing depends on hash-map iteration order.

The exit code is 0 only if every compared outcome is EQUAL or falls in
exactly one of the twelve declared classes. Any other difference makes it
1. A usage error is 2. `cargo test --release` runs the unit
tests of the numeric comparator. A difference that fits no predicate at all prints the scenario: its
seed and index, every operation up to the failing one, and the tree held at
that point. The scenario is then abandoned, because the two registries may
no longer hold the same state.

## Method

A scenario builds one 2.4 `Registry<Timestamp>` and one v3 `Registry`, both
unbounded (35% of scenarios) or both with the same `max_age` (0, 50, 200 or
5,000 clock ticks; a tick is 1 ns, 1,000,003 ns or 1 s). It then applies 60
to 199 generated operations to both and compares every outcome.

- **Names.** Inserts draw from `a` to `h` and, 3% of the time, the empty
  name. Queries also draw `yy` and `zz`, which no insert uses. Equal names
  arise by chance.
- **Stamps** are non-negative `i64` nanoseconds, given to 2.4 as the same
  `u64`. They follow a scenario clock, step behind it (inside and outside
  the window), repeat stored instants, and occasionally sit at `i64::MAX`.
- **Geometry.** Translations: ordinary, integral, zero, large, extreme
  finite values up to `f64::MAX` (so that sums, differences and rotations
  overflow), and non-finite. Rotations: random unit, identity, exact axis
  rotations, unit quaternions rounded through `f32`, unit quaternions
  scaled to a norm within 1e-6 of 1, scaled outside it, and non-finite.
- **Operations.** Insert (30%), batch of 2 to 6 (8%; 40% of batches are
  one frame's samples in ascending, descending or mixed order), reparent
  (4%), `remove_frame` (2%), cleanup (4%), lookup (30%), cross-time lookup
  (12%), `latest_common_time` (10%). Slightly over half of the inserts aim
  at an existing edge, 8% of those with the other kind.
- **Construction** is compared in two stages, because v3 builds a
  `Rotation` before a `Transform`: `Rotation::from_wxyz` against a 2.4
  `Transform::new` with a zero translation, then the two `Transform::new`
  calls. A transform neither version constructs is dropped from the
  operation (a batch member is dropped from both batches).
- **Cleanup.** 2.4 `remove_transforms_before(t)` is paired with v3
  `remove_samples_through(t - 1 ns)` for `t > 0`, and
  `remove_transforms_before(u64::MAX)` with
  `remove_samples_through(Timestamp::MAX)`; both pairs must leave equal
  states. A third operation gives the same `t` to both calls (class 3).
- **A reference model** (`src/model.rs`) mirrors the tree v3 accepted:
  parent, kind, the stored v3 geometry per sample and the rotation as the
  scenario gave it (which is what 2.4 stores). It never decides
  whether an operation succeeds. The comparison uses it to evaluate the
  class predicates below, and the generator uses it to aim at existing
  frames.

### What EQUAL means

- Both `Ok`: the same frames and stamp, and bit-identical geometry (seven
  `f64`, compared with `to_bits`). `remove_frame` returns the same `bool`.
- Both `Err` with corresponding variants and the same named frame and
  payload where both carry one:

  | 2.4 | v3 | Compared |
  |---|---|---|
  | `SelfReferentialFrame` | `InsertError::SelfEdge` | frame is the child |
  | `ReparentingNotSupported` | `ParentMismatch` | frame, pinned parent, offered parent |
  | `StaticDynamicConflict` | `KindMismatch` | frame, incoming stamp |
  | `CycleDetected` | `Cycle` | child and parent are the rejected edge |
  | `UnknownFrame` (reparent) | `InsertError::UnknownFrame` | frame |
  | `NoParentToReplace` | `NoParent` | frame |
  | `ParentUnchanged` | `ParentUnchanged` | frame, parent |
  | `UnknownFrame` | `LookupError::UnknownFrame` | frame |
  | `Disconnected` | `Disconnected` | the pair; see below |
  | `NotFoundAt { covered: None }` | `NoData` | frame |
  | `NotFoundAt { covered: Some }` | `OutOfRange` | frame, requested, earliest, latest |
  | `NoCommonTime { covered: None }` | `NoData` | frame |
  | `NoCommonTime { covered: Some }` | `NoCommonTime` | frame, earliest, latest; v3 `latest_possible < earliest` |
  | `NonFiniteValues` | `NonFinite` | |
  | `NonUnitRotation` | `NonUnitRotation` | norm, at construction |

- A batch error is compared by variant and payload against the member at
  v3's `BatchError::index`; 2.4 reports no index.
- `Disconnected` in `lookup` and `latest_common_time` must name
  `(target, source)` in that order on both sides. In a cross-time lookup
  2.4 names the pair `(fixed, endpoint)` and v3 `(endpoint, fixed)`; the
  same pair in those positions counts as EQUAL.

In addition, v3 is checked against itself where the model allows it: a
successful v3 result must be finite; a one-hop stored-direction lookup at a
stored instant must return the stored v3 geometry bit for bit; and a failed
`lookup_across` must report the error DESIGN.md's diagnosis order
prescribes, derived from v3's own `lookup` answers for the two legs
(unknown names in the order target, source, fixed; then a disconnected leg;
then coverage; then numerics; the target's leg first within each step).

## Declared difference classes

Each class is a predicate. An outcome pair that is not EQUAL must satisfy
exactly one of them. They are tested in an order that makes them exclusive.

1. **`Disconnected` precedence.** 2.4 returns `NotFoundAt`, v3 returns
   `Disconnected { a, b }`, and the model confirms that `a` and `b` are
   both known and that no chain connects them. In `lookup`, `(a, b)` must
   be `(target, source)`; in `lookup_across`, `b` must be the fixed frame
   and `a` the target or the source.
2. **`Expired`.** Single insert: 2.4 returns `Ok`, v3 returns
   `Expired { frame, stamp, newest }`, the registries have a `max_age`, the
   payload names the transform's child and stamp, `newest` is the model's
   newest stamp for that frame, and `stamp < newest - max_age`. The model
   is not changed, and later queries verify that 2.4's insert-then-evict
   left the same state.
   Batch: v3's `insert_all` reports `Expired` at an index. The payload is
   checked the same way, with `newest` taken as the maximum of the model's
   newest stamp and the stamps of earlier members for that frame. The
   member is dropped and the batch is resubmitted, the recovery the
   `insert_all` rustdoc documents, until it reports something else. That
   final result must be EQUAL to 2.4's result for the whole batch (`Ok`
   and `Ok`, or corresponding errors). Later queries verify that the
   stored state answers identically.
3. **Inclusive cutoff.** After the same `t` went to
   `remove_transforms_before` and `remove_samples_through`, every edge
   that held a sample at exactly `t` (per the model) is probed with a
   direct lookup at `t`. 2.4 must return `Ok`; v3 must return `NoData`
   naming the child with the model showing the edge drained, or
   `OutOfRange` naming the child with `requested == t` and `earliest > t`.
   One count per probed sample. 2.4 then gets
   `remove_transforms_before(t + 1 ns)`, so the states agree again.
4. **Normalization of off-band rotations.** Both `Ok`, geometry not
   bit-identical, and at least one edge on the chain read a sample whose
   rotation v3 changed at construction (its stored components differ in
   bits from the input; "read" means the stored sample at the instant, or
   the two samples bracketing it). Every component must agree within the
   class 5 bound plus, per such edge, `4 * 1e-6` on the rotation and
   `4 * 1e-6 * (M + 1)` on the translation. 1e-6 is the norm tolerance
   both versions accept. 2.4 applies the raw quaternion in its sandwich
   product, which scales a vector by the squared norm, so the first-order
   effect of one off-band edge is `2 * 1e-6`; the factor 4 leaves a margin
   of two. Largest observed: 0.495 of the bound.
5. **Last-bit changes.** Both `Ok`, geometry not bit-identical, no edge on
   the chain read a normalized rotation, and the lookup is not one that
   must be exact. Every rotation component must agree within
   `8 * EPSILON * (hops + 1)` and every translation component within
   `8 * EPSILON * (hops + 1) * (M + 1)`. `hops` is the number of edges
   the chain crosses (both legs for a cross-time lookup). `M` is the sum
   over those edges of the largest `|x| + |y| + |z|` among the samples
   read. Justification: each interpolation, rotation application, product
   and inversion contributes a few ulp relative to the magnitudes that
   enter it, so the error is linear in the number of edges and in the
   translation magnitude carried along the chain; `+ 1` covers results
   that cancel to near zero and the unit-scale rotation terms. Largest
   observed over 21 million operations: 0.172 of the bound.
   The translation side of this bound and of the class 4 bound is
   evaluated in scaled coordinates: every translation component, every
   magnitude term and the `1` are multiplied by 2^-16 before they are
   summed or differenced. `M` can exceed `f64::MAX` (two edges at
   `f64::MAX` suffice), and unscaled it would make the bound infinite,
   which accepts any difference. Scaled, the bound is finite and positive
   for every finite input, and for inputs away from the overflow and
   underflow edges the ratio of difference to bound is unchanged, because
   a power of two scales exactly. A ratio that is NaN, or anything else
   that is not at most 1, fails.
   **Exact lookups** are excluded from this class and must be
   bit-identical: a known frame relative to itself, and a one-hop lookup
   in the stored direction (`target` is the parent of `source`) on a
   static edge or at a stored instant, unless that sample is a normalized
   one (class 4).
6. **`Ok(non-finite)` to `Err`.** 2.4 returns `Ok` with at least one
   non-finite component and v3 returns `LookupError::NonFinite`. (2.4
   `NonFiniteValues` against v3 `NonFinite` is EQUAL.)
7. **Equal unregistered names.** The same non-empty name, unknown to the
   model, is given for every frame argument. 2.4 returns the identity
   (`lookup`, and `lookup_across` with target, source and fixed all equal)
   or `Stamp::Static` (`latest_common_time`); v3 returns `UnknownFrame`
   naming it.
8. **Order of reported unknown names.** `lookup_across` only. Both return
   `UnknownFrame` with different names; 2.4's name is unknown to the
   model; v3's name is the first unknown one in the order target, source,
   fixed (2.4 reports in the order fixed, source, target).
9. **Empty names.** v3 `Transform::new` returns `EmptyFrameName` for a
   transform 2.4 constructs. Insert and reparent: the transform is given
   to 2.4 alone, which must refuse it (with `EmptyFrameName`, or, in
   `reparent_frame`, with an error it checks earlier). Batch: 2.4 must
   refuse the batch; if its error is `EmptyFrameName` the outcome is in
   this class, and if it is another member's error, v3 must report the
   corresponding error for the members before the empty name (submitted
   with a self-edge sentinel appended, so nothing is stored), which
   counts as EQUAL. Also in this class: v3 reports `EmptyFrameName` where
   2.4 reports the non-finite translation of the same transform, and the
   class 7 situation with the empty name as the equal name.
10. **Cross-time result type.** Both `Ok` and bit-identical geometry in
    `lookup_across`. 2.4 returns a `Transform` stamped with the target
    time; v3 returns a `CrossTimeTransform`. Compared through accessors:
    target, source, `target_time` against 2.4's stamp and the request,
    `source_time` against the request. A cross-time pair whose geometry
    differs is counted under class 4 or 5 instead, with the same label
    checks.
11. **Cross-time leg order.** Prescribed by DESIGN.md's registry
    semantics; missing from the Acceptance list's enumeration, which names
    only the order of unknown names. Both legs of a `lookup_across` fail.
    2.4 resolves the source's leg first and returns its failure; v3
    follows the diagnosis order (unknown names, then `Disconnected`, then
    coverage, then numerics, the target's leg first within each).
    Predicate: both return errors that are neither EQUAL nor class 1 or 8,
    v3's error is the one the diagnosis order prescribes from its own
    legs, and 2.4's error is verbatim the failure of one of its legs
    looked up alone.

12. **Finite against non-finite at the overflow edge.** One version returns a
    finite result and the other reports or returns a non-finite one. The
    explanation offered: both versions use the same sandwich product, whose
    intermediates overflow before the result does, and last-bit differences
    decide the side. Observed in all three directions: 2.4 `Ok(finite)`
    against v3 `NonFinite`, 2.4 `NonFiniteValues` against v3 `Ok(finite)`,
    and 2.4 `Ok(inf)` against v3 `Ok(f64::MAX)`.
    The predicate tests that explanation instead of assuming it. First, the
    chain must be able to overflow: `M >= f64::MAX / 4`. Second, the query
    is replayed where it cannot. The model's stored samples of every edge
    the chain crosses (both legs of a cross-time lookup) go into a fresh
    unbounded 2.4 registry and a fresh unbounded v3 registry, with the same
    stamps, the same parents, the rotation inputs the scenario gave, and
    every translation component multiplied by 2^-64. That factor is exact
    for components of magnitude at least 2^-958 and leaves a factor of 2^64
    between any intermediate and `f64::MAX`. Both registries must answer
    the same query with `Ok`, and the two answers must be bit-identical or
    agree within the class 4 / class 5 bound taken in the scaled
    coordinates (`M` and the `1` multiplied by 2^-64). An outcome that
    fails either step is UNCLASSIFIED and printed with its scenario. Every
    case observed in the two runs below passed the replay.
    Declared by the maintainer on 2026-11-18. The Acceptance list in
    DESIGN.md did not name it as first written.

## Not covered

- Re-publishing a lookup result (2.4 validates at insert; v3 results are
  valid by type).
- Negative stamps, serde, `compose` and `inverse` called directly,
  `Rotation::rotate`.
- The razor edge of the 1e-6 norm tolerance: the nearest generated norms
  are `1 ± 9.9e-7` and `1 ± 1.01e-6`.

## Counts

Default run on 2026-11-18 (seed 8390876182755045234, 2,321 scenarios,
300,111 operations), against the v3 working tree of that day:

| Class | Outcomes |
|---|---:|
| EQUAL | 443,332 |
| 1 `Disconnected` precedence | 11,871 |
| 2 `Expired` (insert and batch) | 6,458 |
| 3 inclusive cleanup cutoff | 929 |
| 4 off-band rotation normalized | 1,698 |
| 5 last-bit numeric change | 1,129 |
| 6 `Ok(non-finite)` to `Err(NonFinite)` | 52 |
| 7 equal unregistered names | 3,527 |
| 8 unknown-name report order | 8,839 |
| 9 empty name | 3,966 |
| 10 cross-time result type | 639 |
| 11 cross-time leg order | 6,308 |
| 12 finite/NonFinite at overflow edge | 2 |
| UNCLASSIFIED | 0 |

7,729 lookups had to be bit-identical and were. The run exits 0. Both
class 12 outcomes passed the scaled replay.

Outcomes outnumber operations: every constructed transform is one compared
outcome (batch members included), and each sample probed after a
same-cutoff cleanup is one.

A longer run the same day (`--seed 20261118 --ops 3000000`: 23,190
scenarios, 3,000,141 operations): EQUAL 4,435,364; class 1 119,704; class 2
64,449; class 3 9,242; class 4 17,657; class 5 11,324; class 6 432; class 7
34,879; class 8 87,323; class 9 39,953; class 10 5,941; class 11 63,582;
class 12 5, all of which passed the scaled replay; unclassified 0; 76,098
exact lookups bit-identical. It exits 0.
