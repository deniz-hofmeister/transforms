# transforms v3 — addendum: what the dependents add to the ledger

A companion to `transforms-v3-1.md`, `-2.md`, `-3.md`,
`transforms-v3-addendum-2.2.0.md`, `-cross-time.md`, `-corrections.md`,
[`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md) and the
dependent study [`analysis/dependents/README.md`](../dependents/README.md).
Written 2026-11-06. Baseline: master `f9770d2` (released v2.3.0 plus
docs-only commits). Nothing here is implemented.

Each v3 candidate the dependent study raised was put to one question: is
a ledger row strictly necessary? A row must need a break, and it must have
evidence from a real user. An additive fix can never be a row, and
reachability alone does not raise a row's value. Three judges ruled on
each candidate independently, and a change needed two of the three votes.
Counts below are votes for a change; the ballots are not kept in the
repository. Every claim below was re-checked against the code at
`f9770d2`, the dependents' sources at the versions the study pins, and the
probes, re-run from their prebuilt binaries. No dependent's code was built
or run.

## Rows added

### Insert errors name the rejected transform's frames

Two of three judges voted for the row. Added to the ledger table as:

> Insert errors name the rejected transform's frames:
> `SelfReferentialFrame`, `CycleDetected` and `StaticDynamicConflict`
> gain frame fields (and `EmptyFrameName`, if 2.4 ships it);
> `ReparentingNotSupported` gains its child

- **What changes.** `SelfReferentialFrame`, `CycleDetected` and
  `StaticDynamicConflict` are unit variants
  (`src/core/registry/error.rs:49`, `:79`, `:96`).
  `ReparentingNotSupported { current_parent }` names the pinned parent
  but not the child (`:71-75`). DESIGN.md Part D would add
  `EmptyFrameName` as a unit variant too.
- **Why it needs a break.**
  - Unit variants that gain fields break every pattern written today.
    That includes the `add_transforms` doc example,
    `matches!(result, Err(RegistryError::CycleDetected))`
    (`src/core/registry/mod.rs:211`).
  - `ReparentingNotSupported` is not `#[non_exhaustive]`, so a new field
    breaks its construction and every pattern without `..`.
  - The additive routes fail. A parallel set of framed variants gives one
    condition two spellings, against the flat-error rule in `AGENTS.md`.
    An `add_transforms` that returns a member index stops returning
    `RegistryError<T>`. A dry run cannot see conflicts between members of
    one batch, and `Registry` derives only `Debug` (`mod.rs:94`), so a
    caller cannot replay the batch on a copy. Deriving `Clone` would be
    additive, but cloning the registry to diagnose a rejected message is
    a workaround, not a fix.
- **Evidence.** All four studied users hit it
  ([matrix row](../dependents/README.md#hand-rolled-around-the-crate)).
  - dimos inserts each TF message with `add_transforms`
    (`tf.rs:200`@`92936a7`) and logs `"TF message rejected"` with only
    the error text (`tf.rs:361`). It cannot name the member: the 2.3.0
    changelog says the error does not identify it (`CHANGELOG.md:20`).
    [`probes/dimos.rs`](../dependents/probes/dimos.rs) P3:
    `cycle batch err=CycleDetected`.
  - roslibrust_transforms clones both names "so the warning below can
    still name them" (`lib.rs:324-331`@0.26.0).
    [`probes/roslibrust_transforms_verify.rs`](../dependents/probes/roslibrust_transforms_verify.rs)
    VX prints `StaticDynamicConflict | ReparentingNotSupported {
    current_parent: "a" } | CycleDetected`.
  - transforms_io formats both names by hand
    (`mt_pubsub_backend.rs:343-363`@0.8.0).
  - whirl_tf wraps its errors with both names
    (`buffer.rs:40-53`@0.1.0-alpha.3), pinned to `=2.0.0-beta.4`.
- **Cost to the studied users.** About zero. None of them matches the
  three unit variants. transforms_io's one match,
  `ReparentingNotSupported { .. }` (`mt_pubsub_backend.rs:342`@0.8.0),
  survives an added field. The others stringify insert errors, so they
  see only new `Display` text.
- **Value.** Low. It is diagnostics, not a wrong answer. Per element the
  workaround is exact, and three users write it; for `add_transforms` it
  is impossible. Not a trigger.
- **Why the table needs it now.** DESIGN.md D7 ships `EmptyFrameName`
  with a unit payload because frame fields are a ledger candidate for all
  insert errors at once (`DESIGN.md:458`). The corrections addendum said
  the item "stands"
  ([`-corrections.md:84-86`](transforms-v3-addendum-corrections.md)),
  but it was never in the table. Without the row, D7 defers to nothing.
- **Left out.** `NonUnitRotation` and `NonFiniteValues`. A caller can
  check each member exactly with `Transform::validate`, as dimos does,
  and a lookup-side `NonFiniteValues` comes from a composed chain with no
  single frame to blame
  ([`dimos.md` shape](../dependents/dimos.md#v3-frame-names-in-insert-errors)).
- **Added in judging.** `ReparentingNotSupported` gaining its child. The
  study's shape covers only the three unit variants, but a batch member
  that fails with `ReparentingNotSupported` is just as ambiguous. The
  vocabulary row already renames this variant, so the two share the
  break.
- **Dissent.** One judge voted against: every instance fails loudly, and
  a break that buys only diagnostics does not meet the bar for a row.

## Evidence on existing rows

### `get_transform_at` returns a cross-time result type

Two of three judges voted for evidence. Appended to the Value cell:

> roslibrust_transforms 0.26.0 puts a misuse one call from a wrong edge
> on the network: its `add_transform` upserts the result over the true
> sample and publishes it on `/tf`. Reachable, not observed

- roslibrust_transforms wraps `get_transform_at` as a public passthrough
  (`lib.rs:555-573`@0.26.0). Its `add_transform` inserts into the
  registry, then publishes on `/tf` for a non-static stamp
  (`lib.rs:518-547`). The broadcast was read, not run.
- The insert replaces "any sample at the same timestamp"
  (`src/core/registry/mod.rs:139`).
- [`probes/roslibrust_transforms.rs`](../dependents/probes/roslibrust_transforms.rs)
  P5: odom←base at t2 is `Ok(x=5)` before; the result carries stamp
  `At(2 s)` and x=−1; `add_transform(result)` is `Ok(())`; odom←base at
  t2 is `Ok(x=-1)` after.
- Value stays Medium. No user has been seen doing this
  ([`roslibrust_transforms.md` Z4](../dependents/roslibrust_transforms.md#z4-a-cross-time-result-re-inserted-overwrites-the-truth-and-broadcasts-it-silent-propagates)).
- Dissent: one judge voted for no change. DESIGN.md §2 item 3 already
  records the re-broadcast (`DESIGN.md:100-110`), and neither cell
  changes.

### `TransformError<T>`, sealed `Point`, required `max_age`

Two of three judges voted for evidence. Appended to the Value cell:

> Every studied user already builds with `with_max_age` (10 s in their
> defaults or docs), so requiring it changes none of them, and an 8 s
> dropout inside it is still interpolated

- `Registry::with_max_age` is how every studied user builds the
  registry: transforms_io in all four backends
  (`hiroz_backend.rs:99`, `mt_pubsub_backend.rs:188` and `:221`,
  `r2r_backend.rs:143` and `:218`, `ros2_client_backend.rs:105`,
  @0.8.0; 10 s default at `lib.rs:48`); roslibrust_transforms
  (`lib.rs:230`@0.26.0, a caller-supplied duration, 10 s in its docs at
  `:29` and `:217`); dimos (`tf.rs:372`@`92936a7`, 10 s at `tf.rs:44`);
  whirl_tf (`buffer.rs:72`@0.1.0-alpha.3, caller-supplied).
- [`probes/transforms_io.rs`](../dependents/probes/transforms_io.rs) P9:
  an 8 s gap, lookup at 104 s, `Ok((4.0, "odom<-base"))`.
  [`probes/roslibrust_transforms.rs`](../dependents/probes/roslibrust_transforms.rs)
  P10: `Ok(x=4)` with a 10 s `max_age`.
- Why in the table: v3-1 item 4 ("Bounded by default",
  `transforms-v3-1.md:85`) and v3-3's compatibility table, which flips
  `unbounded_default_interpolates_across_long_gaps` for a required
  `max_age` (`transforms-v3-3.md:246`), read as if the requirement
  closed the gap hazard. The report says it does not
  (`REPORT.md:143-146`), but from a reviewer probe, and a batched v3
  would be cut from the table. The Value stays Low.
- Dissent: one judge voted for no change, because `REPORT.md:143-146`
  already says it.

## Corrections to the warrant report

Edited in place in
[`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md), each with a dated note
pointing here. Line numbers are before the edit. None changes the
verdict: together they make v3 costlier, not more needed.

- **"What the dependents show", lines 151-153.** Three of three judges.
  Old text:

  > None matches a `RegistryError` variant, and none has comments
  > complaining about the API. Their friction is in ROS conversion, which
  > no v3 item touches.

  - transforms_io 0.8.0 (2026-10-27) matches
    `RegistryError::ReparentingNotSupported { .. }`
    (`mt_pubsub_backend.rs:342`@0.8.0).
  - Frame names in insert errors, which all four studied users write by
    hand or cannot add, is now a ledger row (above).
  - New text keeps "none has comments complaining about the API", which
    no majority flagged, and says the rest as above.
- **"It will never be cheaper", lines 176-177.** Two of three judges.
  Old text:

  > There are three dependents and about 385 downloads a month, and the
  > last migration was cheap for them.

  - crates.io gives 942 downloads from 2026-10-07 to 2026-11-06. There
    are still three crates.io reverse dependencies
    ([dependents, "Who uses the crate"](../dependents/README.md#who-uses-the-crate)).
    The 385 figure is kept as the figure at writing; it was not shown
    wrong.
  - The transforms maintainer wrote roslibrust_transforms' 2.x migration
    (PR #338; [`roslibrust_transforms.md`](../dependents/roslibrust_transforms.md#verdict)),
    so its cost fell on this crate's side. transforms_io migrated on
    its own, in one commit four days after 2.0.0
    ([`transforms_io.md`](../dependents/transforms_io.md#history)).
- **Cons, "No demonstrated need", lines 188-189.** Three of three judges.
  Old text:

  > No user has asked for any item, and the dependents do not use the
  > parts that would change.

  - The first half holds. The second was already false for roslibrust
    when written; the report's own lines 95-98 say it re-exports
    `get_transform_at`.
  - roslibrust_transforms wraps `get_transform_at` (cross-time row) and
    re-exports `TransformError` (`TransformError<T>` row)
    (`lib.rs:52`, `555-573`@0.26.0).
  - transforms_io 0.8.0 matches `ReparentingNotSupported` (vocabulary
    row).
  - transforms_io re-exports six `transforms` types and uses them in its
    own `pub fn` signatures; pelorus_syncio re-exports five again
    (`lib.rs:41-44`@0.8.0; `pel@0.0.9:src/tf.rs:6`;
    [`transforms_io.md`](../dependents/transforms_io.md#how-transforms_io-uses-the-crate)).
    Each needs a release to move to a major.

Not corrected here, because no majority flagged them and none bears on
the ledger or the verdict: line 22 (the release list), lines 60-61
(transforms_io's `now()` workaround), lines 123-125 (nalgebra line
counts) and lines 155-156 (the 2.0 migration). They are recorded in the
study's
[corrections table](../dependents/README.md#corrections-to-earlier-analysis).

## Considered and not added

- **`Rotation` row: dimos's lookup re-validation** (0 of 3). Defensive
  code in an unmerged PR; `Ok(inf)` needs ~1e308
  inputs ([`probes/dimos.rs`](../dependents/probes/dimos.rs) P6). The
  Value cell stays true.
- **`Expired` error when an insert's own `max_age` evicts it** (0 of 3).
  The break does not close the hazard: a jump smaller than `max_age`
  interleaves silently
  ([`probes/transforms_io.rs`](../dependents/probes/transforms_io.rs) P6
  `Ok(-2.4999…)`, `roslibrust_transforms.rs` P3b), and it would turn
  routine late arrival into `Err`. DESIGN.md Part C item 5 documents it.
- **Re-export chain as its own Cons line** (0 of 3 for a line). The fact
  is carried in the Cons correction above.
- **Vocabulary row: `ReparentingNotSupported` match and `parent()`/
  `child()` on results** (1 of 3). Cost, not value; the match is in the
  Cons correction. That every user reads `parent()`/`child()` on results
  (`mt_pubsub_backend.rs:403-405`@0.8.0, dimos `tf.rs:165-168`) had one
  vote.
- **A maximum interpolation gap** (0 of 3). The designed fix, a
  per-frame bound
  ([`dimos.md`](../dependents/dimos.md#design-doc-maximum-interpolation-gap)),
  is additive in full, so it can never be a row. A default bound would
  break, but has no defensible value. It stays "design doc first".
- **Negative seconds for pre-epoch `SystemTime`** (0 of 3). Diagnostics
  only (`probes/dimos.rs` P4, `covers [NaN, NaN]`); the
  `TransformError<T>` row removes its typed exposure (the `f64` seconds
  in `TransformError`).
- **Following the newest input's kind** (0 of 3). It breaks
  static-xor-dynamic, and bridges that log and drop the error would start
  switching kinds silently. DESIGN.md Part C item 8 documents the
  consequence.
- **`latest_common_time` returning `T`** (0 of 3). `Stamp::Static` is the
  true answer for an all-static chain; both callers handle it
  (`probes/dimos.rs` P7). Docs: DESIGN.md Part C items 2 and 4.
- **`now`/`try_now` removed on `wasm32-unknown-unknown`** (0 of 3). Makes
  the API differ by target for one user that already works around it
  ([`probes/wasm_try_now.rs`](../dependents/probes/wasm_try_now.rs)).
  DESIGN.md Part C item 9 fixes the wording.
- **Precedence row: no studied user matches a lookup variant** (1 of 3).
  True (all stringify), but the "Low to medium" cell already spans it.
- **Cross-time row: the change forces a roslibrust_transforms release**
  (1 of 3 as a cell note). The wrapper is in the Cons correction.
- **`TransformError<T>` row: `TransformError` in roslibrust's public
  conversion trait** (`lib.rs:143`@0.26.0; 1 of 3 as a cell note). The
  re-export is in the Cons correction.
- **Empty frame names as a row** (0 of 3). Moot: the maintainer ruled
  rejection non-breaking (DESIGN.md D5). Only its unit payload reaches
  the ledger, through the new row.
- **Zero operator cost to users; slow adoption of additive API;
  backwards lookup parameter names; pre-epoch aliasing and a `now()`
  "latest"** (0 of 3 each). The first two change no cell. The rest are
  adapter code that a break in transforms cannot fix; they are outreach
  (tio-1, tio-2, tio-6, rlt-5).

## Does any trigger fire?

No. The warrant report's three triggers (`REPORT.md:233-237`, after
the edit):

- **Hardware numbers.** Nothing was measured.
- **A user hits a ledger hazard.** None has been reported or observed.
  - The cross-time re-insert is reachable in a shipped dependent
    (roslibrust_transforms, P5), but no user has been seen doing it.
    roslibrust_transforms has 87 downloads, no reverse dependencies and
    no issue about it.
  - `Ok(inf)` is not reached by any user: dimos P6 builds it by hand.
  - The precedence flip has no consumer: no studied user matches a
    lookup variant.
  - The new row is diagnostics, not a hazard.
  - What would fire it: an observed roslibrust_transforms caller passing
    a `get_transform_at` result to `add_transform`
    ([`roslibrust_transforms.md`](../dependents/roslibrust_transforms.md#v3-ledger)).
- **Deprecated surface accumulates.** 2.4 deprecates nothing
  (`DESIGN.md:473`). Under the 2026-11-03 stance, break-only fixes go
  to this ledger instead of a deprecated 2.x path, so this trigger is
  unlikely to fire. That is a note for the maintainer, not a ledger
  edit.

The verdict stands: v3 is not warranted now. The study raises v3's cost
(the corrections above) and adds one Low row. It makes nothing more
urgent.
