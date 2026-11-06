# Dependents of transforms

This folder holds one report per real-world user of `transforms`. Each
report reads the user's code at a pinned version, reproduces its call
patterns in a probe against this checkout, and ends in recommendations for
`transforms`. This file is the index and the cross-dependent synthesis.
Date: 2026-11-06. Baseline: master `f9770d2` (released v2.3.0 plus
docs-only commits; `src`, `tests`, `examples`, `README.md` and
`MIGRATION.md` are unchanged since `v2.3.0`). Nothing here is implemented.
Every recommendation that grows the public surface or edits `AGENTS.md` is
a maintainer note, not an authorization.

Open questions, trade-offs and considerations left by this study are
collected in [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md).

| Report | Subject | Read at |
|---|---|---|
| [`dimos.md`](dimos.md) | dimensionalOS/dimos PR #4722, which swaps dimos's own `tf.rs` for `transforms` | PR head `92936a7`; main `abf36ce` |
| [`transforms_io.md`](transforms_io.md) | `transforms_io`, all 12 releases, and its dependent `pelorus_syncio` | 0.8.0 = Codeberg `2e6eb80`; pelorus 0.0.9 |
| [`roslibrust_transforms.md`](roslibrust_transforms.md) | `roslibrust_transforms`, all 5 releases | 0.26.0 = GitHub `a798361` |

How this synthesis was built:

- From the three reports, the dependent statements extracted from every
  older analysis document (the "corpus"), and the research notes behind
  each report. The notes are not kept in the repository.
- Every transforms_io and roslibrust_transforms research item went
  through an evidence skeptic. None was refuted. Items marked partial
  appear in the reports in corrected form, and items the skeptic found
  are marked "found in verification" there. The 2026-11-06 dimos refresh
  had no skeptic pass; `dimos.md` rests on its 2026-10-30 verification
  and on `probes/dimos.rs`.
- Placement of each transforms_io and roslibrust_transforms item follows
  a scope ruling made against `AGENTS.md`. No ruling was made for dimos
  alone; `dimos.md` applies the two sibling rulings where they overlap.
- Under the maintainer stance of 2026-11-03, no hazard gets an additive
  parallel 2.x API plus a deprecation. A fix that needs a break goes to
  the v3 ledger, with docs or outreach for 2.x.
- **Not done:** no dependent's code was built or run (probes copy their
  logic); no dependent maintainer was contacted; the `tests/test_all.sh`
  gate was not run on any prototype.

## Who uses the crate

As of 2026-11-06.

| User | Kind | `transforms` requirement | Status and activity | Downloads (all-time / 90-day) | Report |
|---|---|---|---|---|---|
| transforms_io | ROS 2 and Minot bridge library on crates.io (`stelzo`, Codeberg) | `^2.2` at 0.8.0, locked 2.2.0; fresh builds resolve 2.3.0 | Active: 12 releases 2026-04-13 to 2026-10-27; last commit `2e6eb80` (2026-10-27); no issues or PRs | 2,559 / 1,990 | [`transforms_io.md`](transforms_io.md) |
| roslibrust_transforms | ROS 1 and ROS 2 adapter in the RosLibRust workspace, on crates.io | `^2.1` at 0.26.0, locked 2.1.0; fresh builds resolve 2.3.0 | No crate change since 2026-09-25 (`a798361`). PR #352 (wasm) open since 2026-10-13, no reviews | 87 / 57 | [`roslibrust_transforms.md`](roslibrust_transforms.md) |
| whirl_tf | tf2-style buffer library on crates.io, also a PyO3 package `whirl-tf` on PyPI (`README.md:10,19`@0.1.0-alpha.3) | `=2.0.0-beta.4` | Dormant: last release and push 2026-08-29 (`33dfd66`); 0 stars | 133 / 133 (PyPI downloads not checked) | No report: cannot reach any 2.x release ([below](#whirl_tf)) |
| dimos | Robotics framework, Python plus native Rust; not on crates.io | `=2.2.0` from the git fork `TomCC7/transforms` rev `5ae9563`, on PR #4722 only (`native/rust/dimos-module/Cargo.toml:12`@`92936a7`). main has no `transforms` dependency | PR open, no human review; last commit 2026-10-27 | n/a | [`dimos.md`](dimos.md) |
| pelorus_syncio | Lidar processing IO layer; depends on `transforms_io = "0.8.0"` and re-exports five `transforms` types (`pel@0.0.9:src/tf.rs:6`) | Transitive, through transforms_io | One release, 0.0.9 (2026-10-28), read | 16 / 16 | Covered in [`transforms_io.md`](transforms_io.md) |
| termviz2 (`matijazigic`) | Rust application on GitHub, not on crates.io; `src/ros.rs` uses `Registry::new(Duration)`, `add_transform`, `get_transform` | `1.2` | 38 stars, last push 2026-03-23 | n/a | No report: 1.x, not studied |
| transforms-py (`art-e-fact`) | PyO3 Python bindings on GitHub; on PyPI as `transforms-py` 0.1.1 (2025-10-30), same authors | `1.0.0` | 0 stars, last push 2025-11-04 | n/a (PyPI downloads not checked) | No report: 1.x, not studied |

How the list was found:

- crates.io reverse-dependency API for `transforms`: `meta.total = 3`
  (transforms_io 0.8.0, roslibrust_transforms 0.26.0, whirl_tf
  0.1.0-alpha.3).
- crates.io reverse dependencies of transforms_io: `pelorus_syncio` 0.0.9.
- GitHub code search (`gh search code`), filtered by hand. Queries:
  `transforms = "2` (filename `Cargo.toml`), `transforms = { version =
  "2`, `transforms = "2.`, `transforms = "1`, `deniz-hofmeister/transforms`,
  `TomCC7/transforms`, `transforms::Registry`, `use transforms::`. Code
  search indexes default branches only. dimos was found through
  transforms PR #123 and
  [`DISCUSSION.md`](../atomic-batch-insert/DISCUSSION.md), not the search.
- README-only mentions, not users: robotics-rs/robotics.rs (an awesome
  list), arthurlovekin/rotation-visualizer (a reference link).
- **Not searched:** GitHub's "used by" graph (no API), GitLab, Codeberg
  beyond `stelzo/transforms_io` and minot `main` (`b394045`, no
  reference), PyPI dependents of `whirl-tf` and transforms-py, private
  code. The `stelzo/pelorus` Codeberg API returned an empty response.
- Downloads cannot be attributed to users. `transforms` has 28,979
  all-time, 3,799 in the 90-day window and 942 from 2026-10-07 to
  2026-11-06; 2.3.0 has 150 against 1,060 for 2.2.0 (crates.io API).

## whirl_tf

- **Dormant and pinned.** 0.1.0-alpha.3 (2026-08-29) is the latest;
  upstream IntelligentRoboticsLab/whirl_tf `main` is `33dfd66`, the same
  day, with no other branches or PRs. It pins `transforms =
  "=2.0.0-beta.4"` (`Cargo.toml:43-46`@0.1.0-alpha.3).
- **It does not compile on any 2.x release** (by reading; not built):
  - `Transform` is built as a struct literal (`buffer.rs:131-146`). The
    fields are private from v2.0.0 on
    (`src/geometry/transform/mod.rs:100-104`@`v2.0.0`).
  - It uses `Transform::<Timestamp>::UNIT_NORM_TOLERANCE`
    (`buffer.rs:332`); since v2.0.0 that is the module constant
    `geometry::UNIT_NORM_TOLERANCE` (`:28`@`v2.0.0`).
  - It names `errors::BufferError` (`buffer.rs:45`), which v2.0.0 does
    not have.
  - It uses `Timestamp::zero()` as the static sentinel and rejects a
    dynamic stamp of 0 (`buffer.rs:123-126`); `Stamp::Static` replaced
    the sentinel.
- **What it uses:** `Registry::with_max_age`, `add_transform`,
  `get_transform`, `remove_frame`, `TransformError`.
- **What it hand-rolls:**
  - `normalize_frame`: strips a leading `/`, rejects empty names and
    names with an inner `/` (`buffer.rs:377-391`).
  - Finite and unit-norm checks with the crate's constant
    (`buffer.rs:309-337`).
  - Insert and lookup errors wrapped with both frame names
    (`buffer.rs:40-53`, `147-153`).
  - `clear_dynamic` as `remove_frame` per dynamic child
    (`buffer.rs:200-204`). It releases parent pins, unlike
    `remove_transforms_before` (`probes/dimos.rs` P9).
  - Checked ROS time: rejects `sec < 0` and `nanosec >= 1e9`
    (`buffer.rs:395-416`).
  - `insert_message` commits a partial batch on the first failure
    (`buffer.rs:78-86`).
  - A tf2-style `MessageFilter` whose `tolerance_ns` waits for coverage;
    it is not a gap bound (`message_filter.rs:123-145`).
  - nalgebra conversion (`Cargo.toml:37-38`, `buffer.rs:14`).
- **Why no report of its own.** Its hazards are beta.4's, not those of
  any shipped 2.x release, so probing them would test code no one can
  build against today. What it teaches is recurrence, and that is
  already in the "Also in" columns of all three reports and in the matrix
  below. Its one standalone signal, `clear_dynamic`, is recorded in
  [`dimos.md`](dimos.md#on-request-clear-operation). Revisit if it moves
  to 2.x.

## What recurs

Users: dimos at PR head `92936a7`, transforms_io 0.8.0 (with pelorus
0.0.9 where noted), roslibrust_transforms 0.26.0, whirl_tf
0.1.0-alpha.3. Only what the reports establish; "not examined" means no
report looked.

### Hand-rolled around the crate

| Feature | dimos | transforms_io | roslibrust_transforms | whirl_tf | Users |
|---|---|---|---|---|---|
| Frame names in insert-error messages | Cannot: `add_transforms` names no member; `warn!` without names (`tf.rs:361`) | Yes (`mt_pubsub_backend.rs:343-363`) | Yes, clones names "so the warning below can still name them" (`lib.rs:324-331`) | Yes, typed wrapper (`buffer.rs:40-53`) | 4 (3 hand-roll, 1 cannot) |
| ROS time to `TimePoint` | Signed, checked, `SystemTime` (`tf.rs:50-81`) | `sec.max(0)`: clamps, aliases (P5) | Clamps to `zero()`, drops nsec (`lib.rs:90-98`; P6) | Checked, rejects (`buffer.rs:395-416`) | 4 (2 wrong) |
| "Latest" lookup | `latest_common_time`, `Static` → `UNIX_EPOCH`; correct (P1, P7) | `Timestamp::now()`; wrong (P1, N4) | `latest_common_time`, `Static` → `now()`; correct, but README and examples look up at `now()` (Z9) | None | 3 (1 wrong) |
| Wait for a transform | `Notify`, race-free; returns `None`, the error dropped (`tf.rs:336-349`) | None in the crate; pelorus polls 10 ms for 5 s on any error (`pel@0.0.9:src/lib.rs:317,717-731`) | `watch`; swallows the error (`lib.rs:451-510`; Z7) | `MessageFilter`, poll-based on `can_transform`, a `bool` (`message_filter.rs:123-143`) | 4 (all error-blind) |
| Errors carried as strings | Yes (`io::Error`; `None` from `get_latest`) | Yes (`TfError::Lookup(String)`) | Yes (`LookupError(String)`) | No, typed `source` | 3 |
| Frame-name check (reject `""`) | Yes, also leading `/` (`tf.rs:133-141`, `212-219`) | **Absent** | **Absent** | Yes, also strips `/` | 4 (2 hand-roll, 2 exposed) |
| Pre-insert finite and norm check | Yes, about twice as strict as the crate (P5) | No | No | Yes, crate constant | 2 |
| Batch insertion | Atomic `add_transforms` (P3) | Per element, skip and log | Per element, skip and log | Partial commit | 4 |
| Clock-reset handling | Lost in migration (main `tf.rs:112-120`) | Absent; users cannot add it (private registry) | Absent | Manual `clear_dynamic` | 4 (1 has it) |
| Gap or staleness bound | Lost in migration (main `tf.rs:141-172`) | Absent | Absent | Absent (`tolerance_ns` is not one) | 4 (none has it) |
| `/tf` vs `/tf_static` into one registry | Not examined | Yes, by topic | Yes, two tasks; arrival order picks the kind (Z2b) | Not examined | 2 |
| Static broadcaster, one transform per message | n/a | Yes, `KeepLast(1)` (H11) | Yes, ROS 1 not latched (Z12) | Not examined | 2 |
| Quaternion normalization within a looser bound | No (old main normalized above 1e-9) | Absent (P7) | Absent (P4) | Not examined | 2 exposed |
| Per-pair warning dedup | Lost (main `tf.rs:296-320`) | `ReparentingNotSupported` only | No | No | 1 (transforms_io, one variant; dimos lost it) |

### Silent wrong answers

| Hazard | dimos | transforms_io | roslibrust_transforms | whirl_tf |
|---|---|---|---|---|
| `""` frame id joins unrelated trees | Guarded | Exposed (H5; P4 `Ok(2.0)`) | Exposed (Z1; P1 `Ok(x=2)`) | Guarded |
| Clock reset: stale "latest", then mixed epochs | Exposed (Hazard 1) | Exposed (H7; P6, N3) | Exposed (Z3; P3a, P3b) | Mitigable by hand |
| Kind switch `/tf_static` → `/tf` freezes the static pose | Not examined | Exposed (H4; P3 `Ok(0.0)`, publisher said 3.0) | Exposed (Z2; P2 `Ok(x=0.5)`) | Not examined |
| Interpolation across a dropout inside `max_age` | Exposed (Hazard 2) | Exposed (H9; P9 `Ok(4.0)`) | Exposed (Z8; P10 `Ok(x=4)`) | Not probed |
| Pre-epoch stamps alias | No | Exposed (H6; P5) | Exposed (Z6; P6) | No |
| "Latest" served at a non-latest instant | No | Exposed on post-dated or skewed hops (H1; N4, V1) | No (examples fail loudly, Z9) | n/a |
| Cross-time result re-inserted and broadcast | Not reachable | Not reachable | Exposed by misuse (Z4; P5) | Not examined |
| Lookup parameters named backwards | No | Exposed for by-name callers in 3 of 4 backends (H2; P8) | No | No |

Notes:

- **Recurrence is the demand evidence, and it points at boundaries and
  time, not geometry.** Every recurring hazard is input the crate
  accepts (`""`), time across a run (resets, dropouts, pre-epoch
  stamps), or a rejected insert the bridge drops (the kind switch). No
  user hand-rolls geometry the crate lacks.
- **The frame-name check splits along a telling line.** The two users
  that are not thin ROS bridges reject `""`. The two bridges, which
  ingest raw messages where `""` is the field default, do not.
- **Frame names in insert errors is the only gap in transforms' own API
  that all four hit.** The other four-user rows are adapter work (ROS
  time, waits, batching) or absent everywhere (clock reset, gap bound).
  The workaround is exact per element and impossible for
  `add_transforms`.
- **Most hand-rolled pieces are exact on the public API** and stay
  outside under `AGENTS.md` Scope: waits, "latest", ROS time and message
  conversion, routing. The two that are not exact outside are the gap
  bound and per-edge clock-reset detection; both need sample or coverage
  access the API does not expose.
- **Single-user items** (not in the matrix): dimos re-validates every
  lookup result (`tf.rs:159-160`); transforms_io keeps a shadow log of
  statics (H10); roslibrust_transforms `add_transform` can fail after
  committing (Z13) and cannot reach `reparent_frame` at `^2.1` (Z11).
- **Not in the silent matrix:** the parent form of the static-stale trap
  (transforms_io H3, P2), where a static re-published under a new parent
  is rejected and the old pose keeps answering. The rejection is loud,
  and 2.2's `reparent_frame` closes it for a caller that decides to move
  the frame. rlt-4 adds the passthrough on the adapter side;
  `transforms_io.md` makes no H3 recommendation.

## Recommendations

Ranked by real-world impact. Tier 1 is a silent wrong answer reachable in
a shipped dependent (transforms_io, roslibrust_transforms, and through
transforms_io, pelorus). Tier 2 is silent only in the unmerged dimos
integration, or loud. Tier 3 is diagnostics and ledger bookkeeping.
Within a tier, silent comes before loud, then the reports' judgement of
likelihood decides; user count breaks ties. Details follow by placement.

**2.4 status (DESIGN revision 4, 2026-11-06).** The
[2.4 proposal](../v2.4-proposal/DESIGN.md) put every item here through an
"is this strictly necessary" test. In: item 1 as Part D, enforced at the
insert boundary rather than in `Transform::validate` (§5a, decision D6);
item 2 as Part C item 5; item 3 as Part C item 8; item 9 reduced to a
`try_now` rewording (Part C item 9); item 10's evidence on Part C item 2.
Out of 2.4: items 12 and 16 and the maximum interpolation gap (§6). D2
(Part B) is now recommended No. The text below is the study's own
proposal and is kept as written.

| # | Recommendation | Placement | Users | Tier |
|---|---|---|---|---|
| 1 | Reject empty frame names | 2.x code | all four | 1 |
| 2 | Document clock jumps under `with_max_age` | docs, plus outreach | dimos, tio, rlt (whirl_tf hand-rolls a clear) | 1 |
| 3 | State the consequence of dropping a `StaticDynamicConflict` transform | docs, plus outreach | tio, rlt | 1 |
| 4 | transforms_io: replace time zero → `now()` with `latest_common_time` | outreach | tio (rlt examples) | 1 |
| 5 | transforms_io: rename the backwards lookup parameters | outreach | tio | 1 |
| 6 | Cross-time result: record the re-broadcast; ask roslibrust to link the limitation | v3 ledger, plus outreach | rlt | 1 |
| 7 | Check ROS time instead of clamping | outreach | tio, rlt | 1 |
| 8 | dimos: a standalone PR to main on crates.io 2.3 | outreach | dimos | 2 |
| 9 | Correct `Timestamp::now`/`try_now` for clockless targets | docs | rlt | 2 |
| 10 | DESIGN Part C items 2 and 4, strengthened | docs | dimos, tio, rlt | 2 |
| 11 | Carry typed `RegistryError` in adapter errors; error-aware waits | outreach | tio (pelorus), rlt | 2 |
| 12 | Say how to ingest rotations from looser sources | docs, plus outreach | tio, rlt | 2 |
| 13 | Accumulate statics in adapter broadcasters | outreach | tio, rlt | 2 |
| 14 | roslibrust: `^2.2`, `reparent_frame` passthrough, stale `remove_frame` doc | outreach | rlt | 2 |
| 15 | Frame names in insert errors: add the ledger row | v3 ledger | all four | 3 |
| 16 | Decide negative seconds for pre-epoch `SystemTime` | 2.x code (contract) | dimos | 3 |
| 17 | `Rotation` newtype row: add dimos's lookup re-validation | v3 ledger | dimos | 3 |
| 18 | `Expired`-on-insert error: a contested candidate | v3 ledger | tio | 3 |
| 19 | Re-export chain: a cost line under "Cons" | v3 ledger | tio, pelorus | 3 |

### transforms 2.x code

**1. Reject empty frame names.**

- Shape: `TransformError::EmptyFrameName`, enforced by
  `Transform::validate`, so by `new`, `static_between`, `Deserialize` and
  insert. Lookups with an empty endpoint return `UnknownFrame("")`. The
  leading-`/` rule stays in adapters.
  ([`dimos.md`, API shape](dimos.md#2x-reject-empty-frame-names).)
- Invariant: a lookup must return a transform between the requested
  frames over a strict tree, and chain resolution treats equal names as
  one frame. `""` is ROS 2's unset `frame_id`, not a frame. Two
  publishers that omit it share a fabricated root, and the crate's own
  resolution returns a plausible wrong pose.
- Evidence: transforms_io H5 (P4, V4) in all four backends;
  roslibrust_transforms Z1 (P1, V1); dimos and whirl_tf hand-roll the
  check; tf2 rejects both ids
  (`tf2/src/buffer_core.cpp:224-236`@`183399c`). The strongest
  recurrence in this study, and the crate's own resolution produces the
  wrong answer.
- Existing proposals: [`dimos.md`](dimos.md#transforms-2x-code),
  [`transforms_io.md`](transforms_io.md#transforms-2x-code) item 1 and
  [`roslibrust_transforms.md`](roslibrust_transforms.md#transforms-2x-code)
  item 1 are the same item.
  Revision 4 of the 2.4 proposal adopts it as
  [Part D](../v2.4-proposal/DESIGN.md#5a-part-d-reject-empty-frame-names),
  at the insert boundary instead of the shape below.
- Needs: explicit maintainer sign-off (one public variant, so a 2.4 that
  carries it is a minor even without Part B); a MIGRATION note (stored
  `""` data stops decoding); an `AGENTS.md` edit for the identity
  carve-out (`get_transform("", "", t)`) and the `validate` list.
- Strongest objection (both rulings): it is buildable outside, and
  tightening accepted input in a minor turns `Ok` into `Err`. Kept as
  core 2.x code per the rulings. If the maintainer judges the tightening
  a break, it becomes a plain v3 ledger row, with outreach items tio-3 and
  rlt-7 below carrying 2.x.

**16. Decide negative seconds for pre-epoch `SystemTime`.**

- Change `as_seconds_lossy` from NaN to negative seconds before the
  epoch. A documented-contract change, not a parallel API
  ([`dimos.md`, shape](dimos.md#decision-negative-seconds-for-pre-epoch-systemtime)).
- Evidence: dimos tests pre-epoch stamps (`tf.rs:521-546`), and a lookup
  miss prints `covers [NaN, NaN]` (`probes/dimos.rs` P4). Diagnostic
  only: the typed payloads are intact. Only an unmerged integration.
- No scope ruling covers it; it stays where `dimos.md` placed it, at the
  bottom.

**No dependent evidence for DESIGN Part B.** None of dimos,
transforms_io or roslibrust_transforms uses `Point`, `Transformable` or
`Localized`. Dependents give no evidence for or against the scope
exception D2
([`DESIGN.md` §4.1](../v2.4-proposal/DESIGN.md#41-scope-decision-required));
`DESIGN.md` revision 4 now recommends No for 2.4, citing this study, so
it and `dimos.md` agree. The decision stays the maintainer's.

### transforms docs

Items 2, 3, 9 and 12 were proposed for
[`DESIGN.md` §5 Part C](../v2.4-proposal/DESIGN.md#5-part-c-documentation-corrections);
item 10 strengthens two items it already lists. None grows the public
surface. Revision 4 took 2, 3, 9 (reworded) and 10 (item 2 only), and
left 12 out.

**2. Document clock jumps under `with_max_age`.**

- Content, as corrected in verification: an insert older than the
  frame's newest minus `max_age` returns `Ok` and is evicted at once; a
  backward jump of any size ends in a window where two epochs
  interleave; without `max_age`, epochs mix. Recovery is
  `remove_transforms_before` with a cutoff past every sample, and
  "before" is strict, so a sample at the cutoff survives and anchors
  expiry (the hole). A request older than `covered.end − max_age` can
  never be served. Detection belongs to whoever owns the clock (the
  `tf2_ros` `onTimeJump` precedent).
- Evidence: transforms_io H7 (P6, V6, N3); roslibrust_transforms Z3
  (P3a, P3b); dimos Hazard 1 (it lost its detector). whirl_tf hand-rolls
  a clear. The `with_max_age` rustdoc (`src/core/registry/mod.rs:122-130`)
  states the rule, not its consequence under a jump.
- Existing: [`dimos.md` recipe](dimos.md#docs-clock-jump-recipe);
  [`transforms_io.md`](transforms_io.md#transforms-docs) item 2;
  [`roslibrust_transforms.md`](roslibrust_transforms.md#transforms-docs)
  item 2. Same item three times.
- Docs alone do not reach transforms_io users: its registry is private.
  Pair with outreach items tio-4, rlt-9 and dimos-2.

**3. State the consequence of dropping a `StaticDynamicConflict`
transform.**

- One sentence on the variant (`src/core/registry/error.rs:81-96`) and
  on `Registry::add_transform`: a caller that logs and drops the
  rejected dynamic transform keeps the static pose answering every
  instant. Mirror `ReparentingNotSupported`'s "Do not resolve this error
  mechanically": two publishers that disagree on kind would wipe each
  other's data under an automatic `remove_frame` handler.
- Optional: one descriptive row in the README tf2 table
  (`README.md:134-143`): tf2 follows the newest input's kind; transforms
  pins the first.
- The variant's rustdoc already says a frame flipped to static "would
  answer every instant", but only for `reparent_frame`'s cross-kind seed
  (`error.rs:87-91`), not for a caller that drops the rejected sample.
- Evidence: transforms_io H4 (P3, V3); roslibrust_transforms Z2 and Z2b
  (P2, V2). No existing proposal;
  [`TF2_RESEARCH.md:79`](../reparent-frame-value/TF2_RESEARCH.md) records
  the tf2 fact with no doc action.
- No code change: static-xor-dynamic is an `AGENTS.md` invariant.
  Adapter half: outreach items tio-5 and rlt-8.

**9. Correct `Timestamp::now` and `try_now` for clockless targets.**

- `try_now` is documented as "the panic-free counterpart of
  `Timestamp::now`" (`src/time/timestamp/mod.rs:76`). On
  `wasm32-unknown-unknown` std's `SystemTime::now()` panics before
  either can report anything. Say so; no `cfg` change.
- The matching `AGENTS.md:119-122` sentence ("The only documented panic
  is …") is a normative-file edit: flag it to the maintainer.
- Evidence: roslibrust_transforms Z10, a wasm probe run under node 24
  (`try_now trapped: RuntimeError unreachable`;
  [`probes/wasm_try_now.rs`](probes/wasm_try_now.rs)), and roslibrust
  PR #352, which works around it. One user. `AGENTS.md` treats
  documentation drift as a bug, which is why a single-user item ranks
  here.

**10. `latest_common_time` discoverability and `Stamp` versus `T`.**

- Nothing new: strengthens
  [`DESIGN.md` §5](../v2.4-proposal/DESIGN.md#5-part-c-documentation-corrections)
  items 2 and 4.
- Evidence: transforms_io's `now()` (H1) and roslibrust's README and
  examples (Z9) show what callers do without them; dimos shows the
  correct pattern.

**12. Say how to ingest rotations from looser sources.**

- One sentence on the `UNIT_NORM_TOLERANCE` rustdoc
  (`src/geometry/transform/mod.rs:18-30`): a source validated more
  loosely (tf2 accepts `|‖q‖² − 1| < 1e-2`) must normalize within a
  bound the adapter states before construction, never unconditionally.
  The tolerance does not change.
- Evidence: transforms_io H8 (P7, V7); roslibrust_transforms Z5 (P4,
  V4). Loud (the insert is rejected and logged), frequency unknown. The
  weakest docs item.

### v3 ledger

Rows go in the table "What genuinely requires a break" in
[`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md#what-genuinely-requires-a-break)
(`REPORT.md:162-172`). None is a v3 trigger on its own.

**6. Cross-time result type: add the re-broadcast evidence** to the
existing first row and to
[`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md).

- Through roslibrust's `add_transform`, a `get_transform_at` result
  upserts over the true sample and is published on `/tf`, so the wrong
  value leaves transforms' control (Z4; P5, V5).
- Keep the value at Medium: reachability, not occurrence. No 2.x type:
  [`DESIGN.md` §3](../v2.4-proposal/DESIGN.md#3-part-a-moved-to-the-v3-ledger)
  withdrew it as a bandaid. Adapter half: outreach item rlt-2.
- Context: `get_transform_at` began as a commit by `Carter12s`, the
  roslibrust_transforms author (`e87c11f`, merged through transforms PR
  #75, which the maintainer opened), so the v3 break lands first on the
  user that started the API.

**15. Frame names in insert errors: add the row.**
`SelfReferentialFrame`, `CycleDetected` and `StaticDynamicConflict` gain
frame fields. A break; a parallel set of framed variants would give one
condition two spellings, against the flat-error rule.

- Evidence: all four users (matrix). Three hand-roll it exactly per
  element; dimos cannot, because `add_transforms` does not say which
  member failed.
- It is recorded in [`dimos.md`](dimos.md#v3-frame-names-in-insert-errors)
  and as "that item stands" in
  [`transforms-v3-addendum-corrections.md:84-86`](../v3-proposals/transforms-v3-addendum-corrections.md).
  Added to the table on 2026-11-06
  ([addendum](../v3-proposals/transforms-v3-addendum-dependents.md)).

**17. `Rotation` newtype and translation bound: add evidence** to the
existing row. dimos re-validates every lookup result
(`tf.rs:159-160`), turning `Ok(inf)` into an error (`probes/dimos.rs`
P6), at the cost that a long valid chain could be rejected (dimos.md
Hazard 5, not probed). The first sign that a user wants finite outputs.

**18. An insert that its own `max_age` evicts reports an error.**
Contested, low. For: `Ok` for data that is not stored is a silent
guess, and tf2 refuses old data (`TF_OLD_DATA`). Against: it does not
fire for a jump smaller than `max_age`, which is the interleaving case;
it would fire on routine out-of-order arrival; nobody asked. A v3 design
must settle how `add_transforms` treats such a member
([`transforms_io.md`](transforms_io.md#v3-ledger) item 7).

**19. Cost line under "Cons".** transforms_io re-exports six
`transforms` types and uses them in its own `pub fn` signatures;
pelorus re-exports five again. A transforms major forces a
transforms_io major and a pelorus change. It also bears on "It will
never be cheaper" under "Pros": the chain grows with each downstream.
The adoption-lag signal is that `latest_common_time` (2.1.0) is unused
across seven transforms_io releases.

### Outreach

Every item is buildable on the public API and needs no transforms
change. Items are labelled `tio-N`, `rlt-N` and `dimos-N`; "Rank" gives
the row in the ranked table. Order within a dependent follows
impact.

**To transforms_io** (`tio-N`; one issue or PR set on
[Codeberg](https://codeberg.org/stelzo/transforms_io); full list in
[`transforms_io.md`](transforms_io.md#outreach-to-transforms_io)):

1. Replace time zero → `Timestamp::now()` with `latest_common_time` then
   `get_transform` under one read guard, and stamp the result header
   with the instant served instead of (0,0). H1: silent at the
   wall-clock instant on post-dated or skewed hops (N4, V1), loud
   otherwise (P1). Rank 4.
2. Rename the r2r, ros2-client and hiroz `lookup_transform` parameters
   to `(target_frame, source_frame)`, as `8388a61` did for mt-pubsub.
   Rename only; positional behaviour, the README and pelorus stay
   correct (H2; P8, V8). Rank 5.
3. Reject empty frame ids on ingest until item 1 ships (H5).
4. Own clock-jump detection and clear on a backward jump, as `tf2_ros`
   does, or expose a reset (H7).
5. Log `StaticDynamicConflict` once per edge at warn and decide whether
   to expose `remove_frame` (H4); keep the reparent warning visible for
   a dynamic hand-over (N1).
6. Check ROS time instead of `sec.max(0)`; flag the silent `i32` wrap in
   `timestamp_to_hiroz_time` (H6; P5). Rank 7.
7. Carry `RegistryError<Timestamp>` in `TfError::Lookup`, so pelorus can
   stop retrying failures that waiting cannot fix. Rank 11.
8. Normalize hand-typed quaternions within a stated bound, log beyond it
   (H8). Accumulate statics in `TfBroadcaster` (H11). Rank 12, 13.
9. If adopting 2.3, keep per-element `add_transform` for `TFMessage`;
   key a replayed `static_transforms()` by child (H10).

**To roslibrust_transforms** (`rlt-N`; an issue or small PR, then separate
questions; the transforms maintainer authored PRs #317 and #338; full
list in
[`roslibrust_transforms.md`](roslibrust_transforms.md#outreach-to-roslibrust_transforms)):

1. Carry a typed `RegistryError` in `TransformManagerError`; have
   `wait_for_transform` fail fast when the request is older than
   `covered.end − max_age` and report the last error on `Timeout` (Z7).
   Rank 11.
2. Link `get_transform_at`'s "Temporal metadata limitation" from the
   wrapper and say never to pass its result to `add_transform` (Z4).
   This is the outreach item of
   [`DESIGN.md` §3](../v2.4-proposal/DESIGN.md#3-part-a-moved-to-the-v3-ledger).
   Rank 6.
3. Stop teaching lookups at `Timestamp::now()` in the README, crate doc
   and examples; show the `latest_common_time` idiom (Z9). Rank 4.
4. Require `transforms = "2.2"`, add a `reparent_frame` passthrough
   (Z11; [`reparent-frame-value/REPORT.md:378-380`](../reparent-frame-value/REPORT.md)),
   and fix the `remove_frame` rustdoc that still says "The transforms
   crate does not support re-parenting" (`lib.rs:631`; found in the
   corpus status check). Rank 14.
5. Fix the time-conversion edges, first the `i32::MIN` negation panic in
   a detached ingest task (Z6). Rank 7.
6. Normalize quaternions within a stated bound (Z5). Rank 12.
7. Reject empty frame ids; decide on a leading `/` (Z1).
8. Question: decide the `/tf_static` → `/tf` kind switch loudly, never
   by an automatic `remove_frame` (Z2, Z2b).
9. Question: clear dynamic data on a backward time jump (Z3).
10. Observation: statics are not latched or accumulated on ROS 1 (Z12).
    Low: a one-guard latest in the wrapper (F4); `add_transform` can
    fail after committing (Z13).

**To dimos** (`dimos-N`, rank 8; a small standalone PR to dimos main
rather than a comment on the dormant #4722;
[`dimos.md`](dimos.md#outreach-to-dimos)):

1. Use crates.io `transforms = "2.3"` instead of the fork; it drops the
   fork's clone-per-call `add_transforms` cost.
2. Restore clock-reset handling at the registry level:
   `remove_transforms_before(far)` on a backward jump.
3. Restore per-pair warnings for failed lookups (dimos.md Hazard 4).
4. Drop the redundant pre-insert norm check or match the crate's
   (P5); keep the lookup re-validation only knowing Hazard 5.
5. Ask which per-edge gap tolerances they would set.
6. Ask whether Python bindings would end the Python/Rust TF split. Two
   exist, both stale: transforms-py on 1.0.0 and whirl_tf's `whirl-tf`
   on 2.0.0-beta.4.

The odds that #4722 lands as is are estimated at 15 to 25 percent in
`dimos.md`; that is an estimate, not evidence.

**whirl_tf and pelorus: none.** whirl_tf is dormant and cannot reach
2.x. pelorus reaches transforms only through transforms_io.

### Do not

- **A core `get_latest_transform`.** Three users, three conventions, two
  correct. Exact on the public API under one guard;
  `latest_common_time` is core for its max-of-starts guard, which does
  not license neighbours (`AGENTS.md` Scope). All three reports agree.
- **A core wait, or `is_retryable`.** `AGENTS.md` Scope names waiting as
  outside; `is_retryable` is already rejected as speculative
  ([`DESIGN.md` §6](../v2.4-proposal/DESIGN.md#6-considered-and-not-proposed)),
  and permanence depends on the caller's `max_age`, stream and clock.
  The cost of the hand-rolled waits is latency and lost diagnostics, not
  wrong answers.
- **Core ROS time conversion or `Timestamp::try_from_secs_nanos`.** Two
  of four users clamp wrongly, but the correct version needs no
  internals (whirl_tf shows it in about 20 lines). Outreach fixes the
  copies. Revisit if a third integration clamps and negative stamps are
  shown to occur.
- **Loosen `UNIT_NORM_TOLERANCE`, or add a normalizing constructor.**
  The first breaks the validation invariant; the second is exact
  outside (`Quaternion::normalize` then `Transform::new`).
- **Make a kind switch replace the frame's cache, as tf2 does.** It
  breaks static-xor-dynamic.
- **A registry read-out API for statics.** `AGENTS.md` Scope: never the
  raw state. transforms_io's shadow log keeps refused statics on purpose.
- **Typed frame handles or another 2.x direction API for H2.** The defect
  is transforms_io's parameter names, not `get_transform`'s.
- **A nalgebra companion crate.** transforms_io's transforms↔nalgebra
  converters are called nowhere, pelorus and dimos convert ROS↔nalgebra
  only, roslibrust has no nalgebra. whirl_tf converts both ways, on
  beta.4 only.
- **Steer bridges to `add_transforms`.** All-or-nothing drops a whole
  `TFMessage` on one bad member; tf2 accepts per transform.
- **A 2.x cross-time type or a 2.x `Expired`-on-insert knob.** Bandaids
  under the 2026-11-03 stance.
- **`UnknownFrame` for equal unregistered names; a core leading-`/` rule;
  a finiteness check at the end of every 2.x lookup.** Identity is
  documented and dimos tests it; stripping `/` is a silent guess; derived
  values are deliberately not re-validated.
- **Change the status of the maximum interpolation gap or the clear
  operation.** The gap is the largest silent hazard without a
  recommendation: reachable in all three studied users (dimos Hazard 2,
  transforms_io P9, roslibrust P10) and not buildable outside. But no
  user has asked, and reachability is not demand
  ([`transforms-v3-3.md:229-230`](../v3-proposals/transforms-v3-3.md)). It
  stays "design doc first", per frame
  ([`dimos.md` shape](dimos.md#design-doc-maximum-interpolation-gap)).
  The clear operation stays "on request"; whirl_tf's `clear_dynamic` is
  weak evidence ([`dimos.md`](dimos.md#on-request-clear-operation)).

### Placement disagreements and how they were resolved

| Item | Disagreement | Resolution |
|---|---|---|
| Kind switch (#3) | The tio-R3 ruling asks for a README tf2-table row; the rlt-REC2 ruling makes the row optional and adds the "do not resolve mechanically" warning | Rustdoc sentence and warning required; README row optional and descriptive. `AGENTS.md` says to keep the README brief and link to rustdoc for API contracts. Both reports now say this |
| Looser rotations (#12) | The tio-R8 ruling: a README row with both bounds; the rlt-REC4 ruling: one rustdoc sentence | The rustdoc sentence only, for the same `AGENTS.md` reason. Both reports now say this |
| Outreach bound for rotations (tio-8, rlt-6) | The rlt-REC6 ruling says "within tf2's bound"; transforms_io.md said "a bound the bridge owns" | "A bound the adapter states". Per the tio-R8 ruling's objection, a crate whose first concern is silent wrong answers should not recommend accepting rotations 0.5% off unit. Both reports now say this |
| `Expired`-on-insert (#18) | The 2026-10-30 dimos report: not a 2.x default, opt-in at most; transforms_io.md: a contested v3 candidate | No conflict after the 2026-11-03 stance removed the opt-in, and `dimos.md` now says so: nothing in 2.x, a contested ledger candidate |
| Empty names (#1) | Rulings place it in core 2.x code; their strongest objection is "buildable outside" | Kept as ruled: the outside version is a wrong-answer generator two shipped users already omit. Falls back to the v3 ledger only if the maintainer calls the tightening a break |
| Part B | `DESIGN.md` D2 recommends the exception; dimos.md defers | Not resolved here: no dependent gives evidence either way |

## Corrections to earlier analysis

Dependent claims in older reports that are now wrong or stale. This study
edits none of them; the v3 addendum of 2026-11-06 later corrected the
`v3-warrant/REPORT.md` claims at `:150-152`, `:176-177` and `:188-189` in
place. `dimos.md` (formerly
`analysis/dimos-lessons/REPORT.md`) carries its own corrections under
["Changes since 2026-10-30"](dimos.md#changes-since-2026-10-30).

| Where | Claim | Current statement | Evidence |
|---|---|---|---|
| [`v3-warrant/REPORT.md:22`](../v3-warrant/REPORT.md) | Read transforms_io 0.2.0/0.5.0/0.6.0 and roslibrust_transforms 0.1.0/0.2.0/0.26.0 | Incomplete. transforms_io has 12 releases up to 0.8.0 (2026-10-27). roslibrust_transforms also has 0.24.0 and 0.25.0, with the same `lib.rs` | transforms_io.md Status; roslibrust_transforms.md Status |
| `v3-warrant/REPORT.md:60-61` | "`transforms_io` 0.6.0 still carries a `Timestamp::now()` workaround for that trap" | Still present in 0.8.0, in all four backends, and it re-creates the trap: loud `NotFoundAt` on most dynamic chains, silent success at the wall-clock instant on post-dated or skewed hops, stamped (0,0) | transforms_io.md H1; `probes/transforms_io.rs` P1, `probes/transforms_io_verify.rs` N4, V1 |
| `v3-warrant/REPORT.md:123-125` | "Both ROS-adapter dependents hand-write roughly 30 lines of nalgebra conversion each" | roslibrust_transforms has no nalgebra in any release. The two nalgebra users are transforms_io (`convert.rs:18-50`@0.8.0, called by none of its backends) and whirl_tf | roslibrust_transforms.md "Earlier analysis claims" |
| `v3-warrant/REPORT.md:150-152` | All three are ROS adapters; "None matches a `RegistryError` variant, and none has comments complaining about the API" | transforms_io 0.8.0 matches `ReparentingNotSupported` (`mt_pubsub_backend.rs:12,342`). whirl_tf keeps the typed error. roslibrust comments that it clones names "so the warning below can still name them" (`lib.rs:324-327`). A fourth integration, dimos, is not an adapter | transforms_io.md History; roslibrust_transforms.md F1 |
| `v3-warrant/REPORT.md:155-156` | "the two active dependents absorbed it within weeks, and it fixed real silent-wrong-answer bugs in them" | True, with two qualifications. transforms_io migrated in one commit four days after 2.0.0 (`8fb3cf8`). roslibrust_transforms' migration (PR #338) was written by the transforms maintainer. The bugs are now named: the stamp-0 kind flip, unvalidated inserts and a `u128` sign-extension overflow in both; non-zero-stamped statics in transforms_io | transforms_io.md History; roslibrust_transforms.md History |
| `v3-warrant/REPORT.md:176-177` | "three dependents and about 385 downloads a month" | 942 downloads from 2026-10-07 to 2026-11-06. Still three crates.io reverse dependencies, plus dimos on a PR branch and pelorus through transforms_io | crates.io API; this file, "Who uses the crate" |
| `v3-warrant/REPORT.md:188-189` | "No user has asked for any item, and the dependents do not use the parts that would change" | The first half holds. The second changed: roslibrust_transforms re-exports `get_transform_at`, which the cross-time row changes; transforms_io 0.8.0 matches `ReparentingNotSupported`, which the vocabulary row renames (`v3-warrant/REPORT.md:170`); transforms_io and pelorus re-export `transforms` types. The corpus also cites dimos's use of `latest_common_time`'s `Stamp`, but no ledger row changes `Stamp` | corpus A3, corrected; recommendations 6, 19 |
| [`v3-proposals/transforms-v3-addendum-corrections.md:84-86`](../v3-proposals/transforms-v3-addendum-corrections.md) | The frame-names v3 item "stands" | It stood without a row in the ledger table; the row was added on 2026-11-06 ([addendum](../v3-proposals/transforms-v3-addendum-dependents.md)) | Recommendation 15 |
| [`reparent-frame-value/REPORT.md:66`](../reparent-frame-value/REPORT.md) | "transforms_io logs at error level and skips … None names the rejection variant" | Partly stale. Since 0.8.0 the default mt-pubsub backend matches `ReparentingNotSupported` and warns once per pair, then traces. r2r, ros2-client and hiroz still log an error per message | `mt_pubsub_backend.rs:337-354`@0.8.0 |
| `reparent-frame-value/REPORT.md:68` | 27,260 all-time and 2,943 recent downloads | Dated 2026-09-19. Today 28,979 and 3,799 | crates.io API |
| `reparent-frame-value/REPORT.md:118-119` | "Both bridges format errors through `Display` only" | Stale for mt-pubsub, which branches on the variant before printing `Display` | as above |
| `reparent-frame-value/REPORT.md:143-147` | PR #338 added `remove_frame` "two weeks after `reparent_frame` was implemented … the hatch was judged adequate" | Order reversed. `remove_frame` was written 2026-08-27 (`4759076`); `reparent_frame` was first committed 2026-08-31 (`2ce5b74`) and released 2026-09-27. The wrapper could not have been chosen over it | roslibrust_transforms.md "Earlier analysis claims" |
| `reparent-frame-value/REPORT.md:188` | "the crate's entire real user base is ROS bridges" | No longer true in spirit: dimos integrates as a framework on a PR branch | dimos.md Status |
| `reparent-frame-value/REPORT.md:378-380` | Checklist: a `reparent_frame` passthrough in roslibrust_transforms | Still not done; it needs `^2.2`. Its `remove_frame` rustdoc still says transforms does not support re-parenting (`lib.rs:631`, master and 0.26.0) | Outreach rlt-4 |
| [`atomic-batch-insert/DISCUSSION.md:38-39`](../atomic-batch-insert/DISCUSSION.md) | "#4722 is part of a 12-PR stack (#4799)" | #4799 returns 404 as a PR and as an issue. The stack seen on 2026-11-06 is #4366 (base main, now conflicting) → #4721 → #4722 → #4735–#4740 | `gh`, 2026-11-06; dimos.md Status |
| [`v2.4-proposal/DESIGN.md:275-277`](../v2.4-proposal/DESIGN.md#9-release-obligations) | Part C alone "ships as a patch release, or rides with other 2.4 items such as the dimos report's empty-frame-name rejection" | Correct, but riding with it makes 2.4 a minor: the rejection adds a public variant. Part C (§5) also lacked the clock-jump item the dimos report proposed on 2026-10-30. Both resolved in DESIGN revision 4 (Part D; Part C item 5) | Recommendations 1, 2 |

## Probes

One Cargo package, [`probes/`](probes/), with a path dependency on this
checkout (`../../..`). Each binary copies a dependent's call patterns,
with message types replaced by plain structs, and cites the copied lines
in its header. No dependent's code is built or run. From the repository
root:

```sh
cargo run --manifest-path analysis/dependents/probes/Cargo.toml --bin <name>
```

The package has its own `[workspace]`, so it builds into
`analysis/dependents/probes/target/`, which `.gitignore` covers. It also
writes an untracked `Cargo.lock` there; it is not committed.

| Binary | Mirrors | Labels | Covers | Report |
|---|---|---|---|---|
| `dimos` | dimos `tf.rs`@`92936a7` | P1–P9, P3b | Three dimos test assertions against 2.3.0 (P1–P3), pre-epoch NaN diagnostics (P4), dimos's stricter norm check (P5), lookup re-validation and `Ok(inf)` (P6), all-static latest (P7), transforms_io's `now()` latest (P8), whirl_tf's `remove_frame` clear vs the cutoff recipe (P9) | [`dimos.md`](dimos.md#probes) |
| `transforms_io` | transforms_io 0.8.0 conversion, insert and lookup | P1–P10, P2b | Time zero → `now()`, parent and kind traps, empty ids, negative seconds, clock reset, rounded quaternions, backwards parameter names, dropouts, static-log replay | [`transforms_io.md`](transforms_io.md#probes) |
| `transforms_io_verify` | The same, rewritten by the evidence skeptic | V1–V10, N1–N4 | P1–P10 independently; recovery by `remove_transforms_before` (V6); dynamic hand-over under the dedupe (N1); a (0,0) `/tf` sample (N2); the interleaving phase of a large jump (N3); AMCL-style post-dated hops (N4) | [`transforms_io.md`](transforms_io.md#probes) |
| `roslibrust_transforms` | roslibrust_transforms 0.26.0 `lib.rs` | P1–P10, P3a, P3b | Empty ids, kind switch, clock reset, rotation tolerance, cross-time re-insert and broadcast, ROS time edges, stringified errors, remove-then-add race, lookups at `now()`, dropouts | [`roslibrust_transforms.md`](roslibrust_transforms.md#probes) |
| `roslibrust_transforms_verify` | The same, rewritten by the evidence skeptic | V1–V10, V3a, V3b, VX | P1–P10 with other numbers; reverse kind order (V2); `Debug` of insert errors (VX) | [`roslibrust_transforms.md`](roslibrust_transforms.md#probes) |

Notes:

- Wall-clock-based numbers differ between runs; the outcomes do not:
  `dimos` P8, `transforms_io` P1, `transforms_io_verify` V1 and N4.
- [`probes/wasm_try_now.rs`](probes/wasm_try_now.rs) (roslibrust Z10) is
  not a binary of the package: it is a `cdylib` for
  `wasm32-unknown-unknown`, with build steps in its header.
- Not in this package: the 2026-10-30 dimos probes O1–O9, which were not
  saved. `dimos.md`
  lists which of those the probes here reproduce.
