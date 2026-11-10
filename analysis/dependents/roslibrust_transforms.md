# roslibrust_transforms: how it uses transforms and what it teaches

Analysis date: 2026-11-06. Baseline: master `f9770d2` (released v2.3.0
plus docs-only commits). Subject: `roslibrust_transforms` 0.1.0, 0.2.0,
0.24.0, 0.25.0 and 0.26.0 from crates.io, upstream
[RosLibRust/roslibrust](https://github.com/RosLibRust/roslibrust) master
`a798361` (the 0.26.0 release commit), PRs #317, #338 and #352, and
issues #311 and #350.

All `lib.rs:N` references point to `roslibrust_transforms/src/lib.rs` at
0.26.0. That file is byte-identical in 0.2.0, 0.24.0, 0.25.0 and master
(`diff -q`). Across those releases only the generated `messages.rs`, its
test, the manifest and the README's version pin differ. A
reference carries a version only where it points elsewhere.

This report is one of the dependent studies in this folder; the
cross-dependent synthesis is [`README.md`](README.md). Nothing here is
implemented. Every recommendation that grows the public surface is a
maintainer note under AGENTS.md, not an authorization.

## Method

- Read all five crates.io tarballs, the upstream clone, the PR #352 diff
  at head `2539d2d`, and PR and issue metadata (`gh`, 2026-11-06).
- Re-checked every earlier claim about this crate in `analysis/`
  (16 hits in 6 files).
- Read tf2 `buffer_core.cpp` and `static_cache.cpp` for ROS 2 rolling
  `183399c` and ROS 1 noetic `8fef92a`.
- Read `transforms_io` 0.8.0 and `whirl_tf` 0.1.0-alpha.3 only to see
  whether a pattern recurs.
- Probes (table at the end) copy the cited `lib.rs` lines verbatim and
  link only `transforms` at `f9770d2`. An independent verification pass
  rewrote them (`V` labels; V5 uses other numbers), built a
  `wasm32-unknown-unknown` probe, and ran it under node 24.
- Each recommendation got a placement ruling (core 2.x code, docs, v3
  ledger, outreach, reject) with its strongest objection.
- **Not done:**
  - No roslibrust code, example or mock backend was built or run.
  - ROS 2, zenoh and hiroz `/tf_static` QoS durability was not checked.
    Only ROS 1 non-latching was confirmed.
  - `tf2_ros` `onTimeJump` was not re-read here. It is cited from
    [`dimos.md`](dimos.md) and the transforms_io verification
    (`tf2_ros/src/buffer.cpp:111-122`@`183399c`).
  - Not checked whether `static_transform_publisher` normalises typed
    quaternions, whether roslibrust receives its own `/tf` publications,
    or what a remote tf2 listener does with a re-broadcast duplicate
    stamp.
  - The GitHub code search for downstream users was not re-run today.
  - No roslibrust maintainer was contacted. The gate was not run.

## Verdict

- **It is a thin adapter with no known users, and transforms points ROS
  users at it.** 87 downloads, no reverse dependencies, no crate change
  since 2026-09-25. The transforms maintainer wrote both of its transforms
  migrations (PRs #317 and #338). The transforms README links it as one
  of two ROS routes, with transforms_io (`README.md:146-149`), so its
  hazards sit on a path this crate recommends.
- **The 2.x migration removed real silent bugs:** the stamp-0 kind flip,
  unvalidated inserts, and a sign-extension overflow. It also introduced
  the warn-and-drop policy behind the kind-switch freeze (Z2).
- **Four silent wrong answers stand out** (all probe-verified; the /tf
  broadcast in P5 is read from `lib.rs:518-547`, not run):
  empty frame ids join unrelated trees (P1); a frame that moves from
  `/tf_static` to `/tf` freezes at its static pose (P2); a clock reset
  serves the pre-reset pose or mixes two runs (P3a, P3b); a
  `get_transform_at` result passed to the manager's `add_transform`
  overwrites the true sample and broadcasts it on `/tf` to every node
  (P5). Interpolation across dropouts (Z8) and pre-epoch stamp
  collisions (Z6) are silent too, but documented or rare.
- **Only the first has a 2.x code fix in transforms**, and it is already
  proposed. The cross-time one stays a v3 ledger item under the
  2026-11-03 stance. The rest is docs and adapter policy: one outreach
  issue covers most of it.
- **One transforms doc statement is wrong:** `Timestamp::try_now`, "the
  panic-free counterpart", traps on `wasm32-unknown-unknown`. PR #352
  hit it.
- **No new core API is warranted.** Recurrence with dimos and
  transforms_io strengthens existing items; it does not add new ones.

## Status

As of 2026-11-06 (crates.io API, `gh`).

| Release | Date | `transforms` req | Locked | Source commit | Downloads |
|---|---|---|---|---|---|
| 0.1.0 | 2026-02-12 | `^1.1` | 1.1.1 | `069db4a` | 31 |
| 0.2.0 | 2026-09-17 | `^2.1` | 2.1.0 | `56a2ac6` | 12 |
| 0.24.0 | 2026-09-18 | `^2.1` | 2.1.0 | `df53c2d` | 14 |
| 0.25.0 | 2026-09-19 | `^2.1` | 2.1.0 | `feeef29` | 13 |
| 0.26.0 | 2026-09-25 | `^2.1` | 2.1.0 | `a798361` | 17 |

- **Totals:** 87 downloads all-time, 57 in the last 90 days. Zero
  reverse dependencies. The research pass's GitHub code search found no
  user outside roslibrust and this repository (not re-run).
- **Requirement:** `transforms = "2.1"`. The published locks pin 2.1.0,
  but downstream builds resolve to 2.3.0, so users get the 2.1.2
  `latest_common_time` arithmetic fix and the 2.2.0 `NotFoundAt` edge
  correction. They cannot reach `reparent_frame` (2.2) or
  `add_transforms` (2.3) through the wrapper.
- **Activity:** the last commit touching the crate is `a798361`
  (2026-09-25). All releases are published by `Carter12s`.
- **Open work:** PR #352 (`JohnTGZ`, opened 2026-10-13, head `2539d2d`,
  no reviews) adds `wasm32-unknown-unknown` support to
  `roslibrust_rosbridge` and `roslibrust_transforms`. Its tracking issue
  #350 is open. On #350, `Carter12s` asked to keep wasm changes out of
  crates other than the rosbridge one (2026-10-11), so the transforms
  half of #352 may not land as written.
- **Origin:** issue #311 ("Add Crate that wraps transforms", `Carter12s`,
  2026-02-02, closed 2026-02-16).

## How roslibrust_transforms uses the crate

- **Registry.** One private `Registry<T>` built with
  `with_max_age(buffer_duration)` (`lib.rs:230`) behind
  `Arc<tokio::sync::RwLock<_>>` (`lib.rs:191`). Every doc and example
  passes 10 s. `buffer_duration` is also the default wait timeout
  (`lib.rs:458`).
- **Time type.** Generic over `T: TimePoint`, defaulting to `Timestamp`
  (`lib.rs:185-206`). Its own `RosTimestamp` trait has impls for
  `Timestamp` and `SystemTime` (`lib.rs:82-127`). Its tests implement
  `MockTimestamp` (`tests/mocked_tests.rs:14-63`), the only third-party
  `TimePoint` seen in a dependent.
- **Routing.** Ingest routes by topic: `/tf_static` becomes
  `Stamp::Static` and the header stamp is ignored; `/tf` becomes
  `Stamp::At` (`lib.rs:688-695`, `790-797`). Publishing routes by stamp:
  `Static` goes to `/tf_static` with wire stamp `(0, 0)`
  (`lib.rs:525`, `539-544`, `826-829`).
- **Ingest and locking.** Two detached tokio tasks, one per topic
  (`lib.rs:251-279`). Each takes one write guard per TFMessage and calls
  `add_transform` per element (`lib.rs:310-333`), so a message is
  accepted per transform, never all-or-nothing. Every lookup wrapper
  takes its own read guard. `add_transform` and `remove_frame` take
  their own write guards (`lib.rs:529-534`, `640-643`).
- **Construction.** `Transform::new` with a struct-literal `Quaternion`,
  never normalised (`lib.rs:697-712`, `799-814`). Accessors only on the
  way out (`lib.rs:720-757`, `822-854`).
- **Wrappers.** `get_transform`, `get_transform_at`,
  `latest_common_time` and `remove_frame` are passthroughs
  (`lib.rs:393-403`, `555-573`, `618-627`, `640-643`).
  `wait_for_transform` is hand-rolled (`lib.rs:451-511`).
  `add_transform` inserts, notifies waiters, then publishes
  (`lib.rs:518-547`).
- **Errors.** `TransformManagerError` holds strings only:
  `LookupError(String)`, `RejectedTransform(String)`,
  `Timeout(String, String, String)` (`lib.rs:67-79`). Every error is
  `e.to_string()` (`lib.rs:402`, `533`, `572`, `626`). `RegistryError`
  is not re-exported; `TransformError` is (`lib.rs:52-55`). Every ingest
  rejection, conversion or registry, is logged at `warn` and skipped
  (`lib.rs:317-332`).
- **Never touched:** `Registry::new`, `add_transforms`,
  `get_transform_for`, `remove_transforms_before`, `reparent_frame`
  (unreachable at `^2.1`), every `RegistryError` variant, `Point`,
  `Transformable`, `Localized`, `validate`, `interpolate`, `inverse`,
  `Mul`, `Quaternion::from_wxyz` and `normalize`, `UNIT_NORM_TOLERANCE`,
  the `serde` feature, and `no_std`. `Timestamp::now()` appears only in
  docs, examples and tests. The `Registry` re-export is vestigial: no
  accessor exposes the manager's registry.

## History

| Step | transforms | What changed | Evidence |
|---|---|---|---|
| 0.1.0 | 1.1 | First release, written by `Carter12s` | `069db4a` |
| PR #317 | 1.2 | Generic `T`, `RosTimestamp`, `get_transform_at`; merged, never published | `58c08ef`, merged 2026-03-05 |
| PR #338 → 0.2.0 | 2.1 | `Stamp::Static`, fallible construction, read locks, `latest_common_time`, `remove_frame`, warn-and-drop | `4759076`; opened 2026-08-30, merged 2026-09-17 |
| 0.24-0.26 | 2.1 | Generated messages only | `9dc3aa1`, `bf63800` |

PRs #317 and #338 were both written by `deniz-hofmeister`, the
transforms maintainer, and merged by `Carter12s`.

**What 0.1.0 got wrong on 1.x** (`v0.1.0 lib.rs`):

- **Stamp-0 kind flip (silent).** transforms' `Buffer::insert` set
  `is_static = (t == 0)` on every insert (`v1.1.0
  src/core/buffer/mod.rs:227`). A `/tf` sample with an unset stamp
  turned a dynamic frame static, served at every instant. It survived
  through 1.4.1 (`v1.4.1 src/core/buffer/mod.rs:243`). Only 2.x's
  `Stamp::Static` removed it.
- **No validation.** `add_transform` was infallible (`v0.1.0
  lib.rs:229-235`).
- **Sign extension.** `(secs as u128) * 1e9` with `secs: i32`
  (`v0.1.0 lib.rs:523-524`, `612-613`): a panic in debug, garbage in
  release.
- **Write-locked lookups and publish before insert** (`v0.1.0
  lib.rs:304`, `441-448`).
- **Wall-clock expiry, corrected in verification.** transforms 1.1
  expired entries against `Timestamp::now() - max_age` (`v1.1.0
  src/core/buffer/mod.rs:357`), which wiped sim-time and bag data. The
  research pass credited 0.2.0 with the fix. In fact transforms 1.2.0
  (2026-03-03) made expiry data-driven (`v1.2.0
  src/core/buffer/mod.rs:378`), and `^1.1` resolution delivered it to
  fresh 0.1.0 builds about six months before 0.2.0. Only builds locked
  to 1.1.x kept the bug.

**What the 2.1 migration introduced:**

- Clamp-to-zero for negative seconds (`lib.rs:91-98`). It replaced a
  panic with a silent collision (Z6).
- Warn-and-drop for every rejection (`lib.rs:317-332`). It makes the
  kind-switch freeze reachable (Z2).
- The `get_transform_at` wrapper rustdoc dates from PR #317 (March).
  transforms documented the result's temporal limitation only in 2.1.2
  (2026-09-12, `CHANGELOG.md`), after #338's content was written. That
  is why the wrapper does not link it (Z4).

**Since 0.2.0:** no transforms-facing change. 2.1.x, 2.2 and 2.3 reach
users only through `^2.1` resolution.

### Earlier analysis claims, re-checked

| Claim (where) | Status |
|---|---|
| "Both ROS-adapter dependents hand-write roughly 30 lines of nalgebra conversion" ([`v3-warrant/REPORT.md:123-125`](../v3-warrant/REPORT.md)) | Ambiguous, not wrong. roslibrust_transforms has no nalgebra in any release. The two nalgebra users are transforms_io (`convert.rs:3,19-50`@0.8.0) and whirl_tf (`Cargo.toml:37-38`, `buffer.rs:293-337`@0.1.0-alpha.3). Name the two crates. roslibrust hand-writes ROS message conversion instead (`lib.rs:658-855`, 198 lines with comments). |
| PR #338 "added `remove_frame` … two weeks after `reparent_frame` was implemented … the hatch was judged adequate" ([`reparent-frame-value/REPORT.md:143-147`](../reparent-frame-value/REPORT.md)) | Order reversed. `remove_frame` was written 2026-08-27 (`4759076`); `reparent_frame` was first committed 2026-08-31 (`2ce5b74`) and released 2026-09-27. The wrapper could not have been chosen over it. |
| "None matches a `RegistryError` variant" ([`v3-warrant/REPORT.md:150-152`](../v3-warrant/REPORT.md); `dimos-lessons/REPORT.md:54-56`@`f9770d2`) | Still true here. Stale for transforms_io 0.8.0, which matches `ReparentingNotSupported` (`mt_pubsub_backend.rs:342`); [`dimos.md`](dimos.md) now records the correction. |
| "the two active dependents absorbed it within weeks" ([`v3-warrant/REPORT.md:154-156`](../v3-warrant/REPORT.md)) | Needs qualifying: the transforms maintainer wrote this dependent's migration. |
| The `get_transform_at` re-export "justifies Part A" (`dimos-lessons/REPORT.md:122-123`@`f9770d2`) | Superseded by [`DESIGN.md`](../v2.4-proposal/DESIGN.md) revision 3; [`dimos.md`](dimos.md) now records it. |
| Releases read: 0.1.0/0.2.0/0.26.0 ([`v3-warrant/REPORT.md:22`](../v3-warrant/REPORT.md)) | Incomplete: 0.24.0 and 0.25.0 exist, with identical `lib.rs`. |
| `reparent_frame` passthrough ([`reparent-frame-value/REPORT.md:378-380`](../reparent-frame-value/REPORT.md)) | Still open; needs `^2.2`. The `remove_frame` rustdoc still says transforms does not support re-parenting (`lib.rs:631`). |

## Hand-rolled around the crate

| Feature | Where | Correct? | Exact on public API? | Also in |
|---|---|---|---|---|
| ROS message ↔ `Transform` | `lib.rs:658-855` | Yes; field order right, exact `f64` round trip | Yes; must stay outside (ROS types) | dimos, transforms_io, whirl_tf |
| ROS time ↔ `T` (`RosTimestamp`) | `lib.rs:82-127` | Normal stamps yes; edges wrong (Z6) | Yes (`from_nanos`, `as_nanos`) | transforms_io (`sec.max(0)`), whirl_tf (checked), dimos |
| `wait_for_transform` | `lib.rs:451-511` | Terminates and coalesces; swallows the error (Z7) | Yes; AGENTS.md Scope names it out | dimos |
| "Latest" lookup: `latest_common_time`, then `get_transform`, `Static => now()` | `lib.rs:575-627` docs; `examples/ros1.rs:74-80` | Yes; two guards, eviction between them fails loudly | Yes | dimos (`UNIX_EPOCH`), transforms_io (uses `now()`, broken) |
| `/tf` vs `/tf_static` split | `lib.rs:251-279`, `525`, `539-544` | Ingest yes; kind conflict dropped (Z2) | Yes | transforms_io |
| `remove_frame` passthrough | `lib.rs:629-643` | Yes; warns about stranded descendants | Passthrough | — |
| Frame names cloned for rejection logs | `lib.rs:324-327` | Yes | Yes | transforms_io, whirl_tf; dimos cannot (batch API, names nothing) |
| *Absent:* frame-name validation | — | Missing (Z1) | Yes, but forgetting it is silent | dimos and whirl_tf hand-roll it; transforms_io lacks it |
| *Absent:* quaternion normalisation within a bound | — | Missing (Z5) | Yes (`Quaternion::normalize`) | none |
| *Absent:* clock-jump handling | — | Missing (Z3) | Mostly; the recipe has a hole | dimos lost it; transforms_io lacks it; whirl_tf has `clear_dynamic` |
| *Absent:* gap or staleness bound | — | Missing (Z8) | Staleness yes; gap no | dimos lost its bound; transforms_io lacks it |

Notes:

- **Nothing here needs sealed internals,** so AGENTS.md Scope keeps all
  of it outside. The scope ruling rejects owning any of it in core.
- **ROS time conversion is the closest call.** It recurs in four
  integrations, and two share one wrong shape, a silent pre-epoch clamp:
  roslibrust clamps the whole stamp to zero, transforms_io clamps the
  seconds with `sec.max(0)` (`mt_pubsub_backend.rs:426`@0.8.0). whirl_tf
  shows the correct version in about 20 lines on the public API: reject
  negative seconds and `nanosec >= 1e9` (`buffer.rs:395-415`). It stays
  outside because the correct version needs no internals (Scope, first
  bullet); the wrong copies are adapter policy, fixed by outreach. The
  scope ruling named two copies of one wrong shape as half of its reopen
  condition; the other half, a middleware-free checked constructor that
  removes the bug class, is a maintainer call this report does not make.
- **The latest idiom is correct, not merely tolerable.** On an all-static
  chain any instant serves, so `Static => Timestamp::now()` works on
  native targets. It panics on wasm (Z10). transforms' advice to make
  both calls under one guard (`src/core/registry/mod.rs:448-450`) cannot
  be followed through the private lock; the wrapper doc argues the gap
  away (`lib.rs:581-583`), correctly for eviction.
- **whirl_tf** was read only for recurrence: frame-name checks
  (`buffer.rs:377-391`), typed error wrapping that adds the frames
  (`buffer.rs:147-152`), and `clear_dynamic` built on `remove_frame`
  (`buffer.rs:200-204`). It pins `transforms = "=2.0.0-beta.4"`.

## Hazards

Severity reflects the outcome, not the likelihood. "Cause" says where the
fix lives.

### Z1. Empty frame ids join unrelated trees (silent)

- **What goes wrong.** ROS 2 leaves an unset `frame_id` as `""`. Two
  publishers that both forget it share a `""` root, and a lookup between
  unrelated frames returns a plausible pose.
- **Evidence.** No name check in `Transform::validate`
  (`src/geometry/transform/mod.rs:301-321`) or in the adapter
  (`lib.rs:799-814`). tf2 rejects both cases with `TF_NO_CHILD_FRAME_ID`
  and `TF_NO_FRAME_ID` (ROS 2 `buffer_core.cpp:224-236`@`183399c`;
  ROS 1 `:226,232`@`8fef92a`).
- **Probe.** `probes/roslibrust_transforms.rs` P1, V1: `("" -> camera,
  x=1)`, `("" -> lidar, x=3)`, then camera←lidar returns `Ok(x=2)`.
- **Cause.** Both. transforms accepts `""`; the adapter does not check.

### Z2. A frame that moves from `/tf_static` to `/tf` freezes (silent)

- **What goes wrong.** Once a frame is static, every dynamic sample is
  rejected with `StaticDynamicConflict` and logged at `warn`. The stale
  static pose answers every instant. `latest_common_time` returns
  `Static`, and the crate's own latest idiom then serves that pose as
  "latest". A plausible trigger is a `robot_state_publisher` restart
  after a URDF joint changes from fixed to movable (not observed).
- **tf2 differs.** ROS 2 rolling re-allocates the frame's cache with the
  incoming kind ("Overwrite TimeCacheInterface type with a current
  input", `buffer_core.cpp:276-285`@`183399c`). ROS 1 keeps the first
  cache, but `StaticCache::insertData` overwrites its one sample
  (`buffer_core.cpp:266-268`, `static_cache.cpp:48-51`@`8fef92a`), so
  the newest sample answers.
- **transforms docs.** `StaticDynamicConflict` (`src/core/registry/
  error.rs:81-96`) states the rule and the `remove_frame` route, not the
  consequence of dropping the sample.
- **Naming.** This is the kind-switch variant. The "static-stale trap" in
  [`reparent-frame-value/REPORT.md`](../reparent-frame-value/REPORT.md)
  is the re-parenting variant, now closable with `reparent_frame`.
- **Probe.** P2, V2: `/tf_static` base→arm x=0.5, then `/tf` at 10 s
  and 11 s: two `WARN`s, base←arm at 10.5 s `Ok(x=0.5)`,
  `latest_common_time` `Ok(Static)`.
- **Cause.** Adapter policy (warn-and-drop) on a correct transforms rule.
  The undocumented consequence is transforms'.

### Z2b. Which kind wins depends on task scheduling (found in verification)

- **What goes wrong.** `/tf` and `/tf_static` are ingested by two
  independent tasks (`lib.rs:251-279`), each with its own write guard per
  message. For a frame on both topics, whichever message is processed
  first pins the kind. Static first freezes silently (Z2). Dynamic first
  rejects the static with a `warn` and fails loudly once `/tf` stops. The
  outcome can differ between restarts of the same system.
- **Probe.** V2 reverse order: dynamic at 10 s, then `/tf_static`, gives
  a `WARN` and `latest_common_time` `Ok(At(10 s))`. Scheduling
  nondeterminism itself was not exercised.
- **Cause.** Adapter.

### Z3. Clock reset: stale "latest" or mixed runs (silent)

- **What goes wrong.** There is no jump detection. A sim restart that
  rewinds by more than `max_age` is evicted on arrival with `Ok`;
  `latest_common_time` stays at the pre-reset instant and lookups there
  return the old pose. A smaller rewind interpolates between the old and
  new runs.
- **Later phase.** The transforms_io verification showed that a large
  rewind does not stay a loss: once the new run's stamps reach
  `old newest − max_age`, they are kept and interleave with the old run
  for up to `max_age` (see [`transforms_io.md`](transforms_io.md)). Same
  registry configuration; not probed here.
- **Evidence.** `latest_timestamp` is the maximum ever inserted
  (`src/core/buffer/mod.rs:316-321`). `tf2_ros` clears on a backward
  jump.
- **Probe.** P3a, V3a: 100–105 s, then 0–3 s, all `Ok`;
  `latest_common_time` `At(105 s)`, lookup there `Ok(x=105)`, lookup at
  2 s `Err(... covers [100, 105])`. P3b, V3b: run 1 at 0–8 s with
  x=10t, run 2 at 0.5/1.5/2.5 s with x=0; lookups give `Ok(x=20)` at
  2 s, `Ok(x=12)` at 2.2 s, `Ok(x=30)` at 3 s, while the robot is at the
  origin.
- **Cause.** Both. transforms offers only a recovery recipe with a hole;
  the adapter does not detect jumps.

### Z4. A cross-time result re-inserted overwrites the truth and broadcasts it (silent, propagates)

- **What goes wrong.** `get_transform_at` returns a `Transform` stamped
  only with `target_time`. Passed to the manager's `add_transform`, it
  upserts over the true sample at that instant
  (`src/core/registry/mod.rs:139`, "replacing any sample at the same
  timestamp") and is then published on `/tf` (`lib.rs:518-547`) with
  that stamp (`lib.rs:826-829`). Every other node receives a false edge
  as if it were measured.
- **Why it is reachable.** The wrapper rustdoc (`lib.rs:549-554`)
  describes the geometry correctly but does not link transforms'
  "Temporal metadata limitation" (`src/core/registry/mod.rs:368-377`),
  which forbids insertion. The crate doc's first example teaches
  lookup-then-`add_transform` (`lib.rs:31-45`), with `get_transform`.
- **Scope.** roslibrust_transforms is the only dependent that exposes
  `get_transform_at`. dimos and transforms_io do not call it. No user has
  been seen doing this; it is reachability, not occurrence.
- **Probe.** P5: odom←base at t2 is `Ok(x=5)`;
  `get_transform_at(odom, t2, base, t1, map)` gives stamp `At(t2)`,
  x=−1, wire stamp `(2, 0)`; `add_transform` `Ok(())`; odom←base at t2
  afterwards `Ok(x=-1)`. V5 reproduces with other numbers (x=−3).
- **Cause.** transforms (an insertable result type; documented, pinned
  by `cross_time_result_keeps_only_target_stamp_and_remains_insertable`)
  and the adapter (missing link, insert-and-publish in one call).

### Z5. Hand-typed quaternions that tf2 accepts are dropped (loud)

- **What goes wrong.** tf2 accepts `|(|q|² − 1)| < 10e-3` and stores the
  value as is (ROS 2 `buffer_core.cpp:66`, `250-255`; ROS 1 `:45`,
  `:250`). transforms requires `||q| − 1| ≤ 1e-6`
  (`src/geometry/transform/mod.rs:18-30`). A 4- or 3-digit quaternion
  fails `Transform::new`; the adapter logs a `warn` and drops it, and the
  frame later fails as `UnknownFrame`.
- **Probe.** P4, V4: `(0, 0, 0.7071, 0.7071)` has `|q|−1 = −9.59e-6`;
  tf2 accepts, transforms gives `Err(NonUnitRotation(0.99999041))`.
  `0.707` fails the same way. f32-rounded values pass.
  `Quaternion::normalize()` then `Transform::new` passes.
- **Cause.** A tolerance difference. transforms' invariant is correct;
  the adapter has no policy for looser sources. Frequency unknown.

### Z6. ROS time conversion edges (mostly silent, rare input)

All need malformed, pre-epoch, or post-2038 stamps.

- **Pre-epoch stamps collapse to `t = 0`** (`lib.rs:91-98`), where the
  last write wins. P6, V6: `(-1, 5e8)` and `(-7, 0)` both give `t = 0`;
  lookup `Ok(x=1)`.
- **ROS 1 negative `nsecs`** is cast `as u32` (`lib.rs:691-694`): `(10,
  -1)` becomes 14.294967295 s. roslibrust's ROS 1 `Time` has
  `nsecs: i32` (`roslibrust_codegen/src/integral_types.rs:16-26`).
- **`SystemTime` only** (`lib.rs:108-127`; the default `Timestamp` path
  clamps instead):
  - pre-epoch conversion subtracts `nsec` instead of adding it: `(-2,
    +0.5 s)` gives −2.5 s, not −1.5 s;
  - `sec = i32::MIN` panics on negation in debug builds. The ingest task
    is a detached `tokio::spawn` (`lib.rs:255`, `270`), so `/tf`
    ingestion would stop with nothing watching. In release the negation
    wraps (by reasoning, not probed);
  - pre-epoch `as_ros_time` goes out as `(0, 0)` via
    `unwrap_or_default` (`lib.rs:121-126`; found in verification).
- **`as_ros_time` wraps after 2038** (`lib.rs:100-105`): 2.2e9 s gives
  `(-2094967296, 0)`. That is the ROS 2 wire format's own `int32` limit.
- **Test code repeats the 0.1.0 bug.** `MockTimestamp::from_ros_time`
  computes `(sec as u128) * 1e9` (`tests/mocked_tests.rs:47-53`; found in
  verification). Test-only, but it shows `RosTimestamp` gives
  implementors no pre-epoch contract.
- **Cause.** Adapter.

### Z7. Errors are stringified; waits cannot tell permanent from transient (loud, causeless)

- **What goes wrong.** Every `RegistryError` becomes a `String`.
  `wait_for_transform` discards the error (`lib.rs:468`, `488`) and
  retries until the deadline, then returns `Timeout` with no cause. A
  request older than `newest − max_age` can never be served, because an
  older insert is evicted on arrival, so the wait can only time out.
- **Probe.** P7, V7: typed `NotFoundAt { frame: "base", requested: 50 s,
  covered: Some((101 s, 111 s)) }` reaches the caller as
  `LookupError("transform from base into odom at 50 not found (base
  covers [101, 111])")`. V7 inserts a late 50 s sample: `Ok(())`, and the
  lookup still fails with the same range.
- **Cause.** Adapter. The typed payload already holds what the decision
  needs. Recurs in all three studied integrations.

### Z8. Interpolation across publisher dropouts (silent)

- **What goes wrong.** No gap bound and no staleness check. An 8 s
  dropout inside the 10 s window is interpolated across.
- **Probe.** P10, V10: 100 s x=0, 108 s x=8, lookup at 104 s `Ok(x=4)`.
- **Cause.** transforms (no gap bound; documented). Same as the dimos
  finding. No roslibrust user has reported it.

### Z9. Docs teach `get_transform(.., Timestamp::now())` (loud)

- **What goes wrong.** The README (`README.md:31`@0.26.0) and the crate
  doc (`lib.rs:32`) `.unwrap()` a lookup at `now()`. On any dynamic chain
  the newest sample is older than now, and transforms does not
  extrapolate, so this panics. The shipped examples
  (`examples/ros1.rs:58`, `examples/ros2_rosbridge.rs:56`) look up at
  `now()` without unwrapping: they log a `warn` every tick and never
  succeed on a dynamic chain (correction from verification).
- **Probe.** P9, V9: newest sample 100 s, lookup at 100.02 s gives
  `Err(... base_link covers [100, 100])`.
- **Cause.** Adapter docs. transforms' README already says zero is an
  ordinary instant (`README.md:138`).

### Z10. `Timestamp::now()` and `try_now()` trap on `wasm32-unknown-unknown` (loud)

- **What goes wrong.** std's `SystemTime::now()` panics on that target
  ("time not implemented on this platform"). `try_now` calls it
  (`src/time/timestamp/mod.rs:94-101`) yet is documented as "the
  panic-free counterpart of `Timestamp::now`", with errors only for an
  out-of-range clock (`:73-82`). AGENTS.md:119-122 names only the
  out-of-range panic.
- **Who hit it.** PR #352 documents a workaround in roslibrust's
  `wait_for_transform` rustdoc ("pass a message stamp or the result of
  `latest_common_time`") and replaces `tokio::time::Instant::now()`. The
  examples' `Stamp::Static => Timestamp::now()` arm would still trap.
- **Probe.** `probes/wasm_try_now.rs` (not a binary of the probe
  package; build steps in its header): a cdylib exporting `Timestamp::try_now()`, built for
  `wasm32-unknown-unknown` against `f9770d2` and run under node 24. Output:
  `try_now trapped: RuntimeError unreachable`. `Timestamp::from_nanos`
  works.
- **Cause.** transforms docs (the statement is false on that target) and
  std.

### Z11. Remove-then-add is pre-empted by a late message (loud)

- **What goes wrong.** `remove_frame` and `add_transform` take separate
  write guards, so a late old-parent `/tf` message can re-pin the frame
  between them. The new parent's insert then fails with
  `ReparentingNotSupported`.
- **Probe.** P8, V8: `remove_frame(tool) = true`, late `world → tool`,
  then `add_transform(gripper → tool)` =
  `Err(ReparentingNotSupported { current_parent: "world" })`. In P8,
  `reparent_frame` succeeds, and the next old-parent message is rejected
  loudly.
- **Cause.** Adapter: no `reparent_frame` passthrough, and `^2.1` cannot
  reach it.

### Z12. Statics do not reach late joiners (loud, outside transforms)

- **What goes wrong.** `add_transform` publishes a one-transform
  TFMessage on `/tf_static` (`lib.rs:522-544`). ROS 1 `advertise` is not
  latched (`roslibrust_ros1/src/lib.rs:146-153`). Even latched, ROS 1
  replays only the last message, so accumulation is needed, as tf2's
  StaticTransformBroadcaster does. ROS 2 and zenoh QoS not checked.
- **Cause.** Adapter plumbing. transforms_io has the same shape on
  TransientLocal depth 1.

### Z13. `add_transform` can fail after committing (no wrong answer; found in verification)

The manager inserts, notifies, then publishes (`lib.rs:527-546`). A
publish failure returns `Err(RosError)` although the local buffer
changed. A retry is an idempotent upsert. Adapter only.

## Friction

- **F1. Insert errors do not name the frame.** `StaticDynamicConflict`,
  `CycleDetected` and `SelfReferentialFrame` are unit variants;
  `ReparentingNotSupported` names only the current parent (VX). So the
  adapter clones both names for every ingested transform, on the hot
  path, to log a rejection, with a comment saying why (`lib.rs:324-327`).
  transforms_io formats them by hand, and whirl_tf wraps the error with
  them.
- **F2. `Stamp` versus `T`.** Every latest call site needs a
  `Stamp::Static => <pick an instant>` arm (`lib.rs:609-613`;
  `examples/ros1.rs:74-79`). Correct, but the arm calls `now()`, which
  traps on wasm. Already [`DESIGN.md`](../v2.4-proposal/DESIGN.md) Part C
  item 4.
- **F3. E0283.** `Transform::static_between(..).map(..)` without a
  turbofish fails to infer `T`; the probe hit it while being written.
  roslibrust avoids it because `manager.add_transform` pins `T`. Already
  Part C item 3.
- **F4. One guard is impossible through the wrapper.** transforms says to
  call `latest_common_time` and `get_transform` under one read guard
  (`src/core/registry/mod.rs:448-450`). The private lock makes that
  impossible for users. An adapter matter; the failure between guards is
  loud.
- **F5. `RegistryError<T>` is generic.** Holding it typed would force
  `TransformManagerError<T>`; `TransformError`, which is not generic, is
  re-exported. Inference only: no comment says this drove the
  stringification. No action.
- **Not friction:** `add_transforms` (2.3) is all-or-nothing, the wrong
  contract for TFMessage, where tf2 accepts per transform. The adapter is
  right not to use it. No TODO or FIXME mentions transforms.

## Recommendations

### transforms 2.x code

1. **Reject empty frame names in `Transform::validate`** (a new
   `TransformError` variant, so constructors, `Deserialize` and insert
   all reject `""`).
   - **Invariant.** Chain resolution treats equal name strings as one
     frame. `""` is ROS 2's unset `frame_id`, not a frame, so two
     publishers that both omit it get a fabricated shared root, and the
     crate's own resolution returns a plausible wrong pose. This is
     boundary validation (AGENTS.md priority 1), the same class as the
     finite and unit-norm checks.
   - **Evidence.** Z1, P1, V1.
   - **Corroboration.** Two of four integrations forget it (this crate;
     transforms_io in all four backends). Two hand-roll it (dimos in an
     unmerged PR; whirl_tf).
   - **Existing proposal.** Duplicates and strengthens
     [`dimos.md`](dimos.md), "transforms 2.x code: Reject empty frame
     names" (shape under "API shapes", `TransformError::EmptyFrameName`).
     [`DESIGN.md`](../v2.4-proposal/DESIGN.md) revision 4 adopts it as
     Part D, at the insert boundary rather than in `Transform::validate`.
   - **Costs.** A MIGRATION note (stored data with `""` stops decoding)
     and an AGENTS.md carve-out from "a frame relative to itself is the
     identity" for `""`. The leading `/` stays an adapter concern.
   - **Strongest objection.** It is buildable outside, it tightens
     behaviour in a minor, and no user has been bitten.
   - **Fallback.** If the maintainer judges the tightening a break, it
     becomes a plain v3 ledger row, and outreach item 7 carries 2.x.

### transforms docs

1. **State the consequence of dropping a `StaticDynamicConflict`
   transform** (new). One sentence on the variant (`src/core/registry/
   error.rs:81-96`) and on `Registry::add_transform`: a caller that drops
   the rejected dynamic transform keeps the static pose answering every
   instant. Changing kind is `remove_frame` then a re-add, at the
   bridge's decision.
   - Mirror `ReparentingNotSupported`'s "Do not resolve this error
     mechanically" (`error.rs:63-66`): two publishers that disagree on
     kind would wipe each other's data under an automatic handler.
   - Optional README tf2-table row, descriptive only: tf2 follows the
     newest input's kind; transforms pins the first.
   - Evidence: Z2, P2. Corroboration: transforms_io (same placement in
     [`transforms_io.md`](transforms_io.md)). No existing proposal.
2. **Document clock jumps.** Both failure modes, now on a second
   dependent, plus the later interleaving phase of a large rewind. The
   recovery is `remove_transforms_before(<after all data>)`, with its
   strict-"before" hole stated. Detection belongs to whoever owns the
   clock.
   - Evidence: Z3, P3a, P3b. Corroboration: dimos, transforms_io.
   - Duplicates [`dimos.md`](dimos.md), "transforms docs: Document clock
     jumps" and "API shapes: Docs: clock-jump recipe".
3. **Correct `Timestamp::now` and `try_now` for clockless targets**
   (new). Say that on targets whose std has no clock
   (`wasm32-unknown-unknown`), std itself panics before either function
   can report anything. No `cfg` change; features stay additive.
   - The matching AGENTS.md sentence ("The only documented panic …",
     AGENTS.md:119-122) is a normative-file edit: flag it to the
     maintainer rather than make it.
   - Evidence: Z10, the wasm probe, PR #352. No corroboration from other
     dependents. No existing proposal.
4. **Say how to ingest from looser sources** (new, low priority). One
   sentence on `UNIT_NORM_TOLERANCE` or `Quaternion::normalize`: a source
   validated more loosely (tf2 accepts squared-norm deviations up to
   1e-2) must normalise within its own stated bound before construction,
   never unconditionally. The tolerance does not change.
   - Evidence: Z5, P4. Corroboration: transforms_io (same placement).
   - Weakest docs item: no user report, frequency unknown.

### v3 ledger

1. **Cross-time result type: add the re-broadcast evidence.** Annotate
   the existing first row of the ledger in
   [`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md) and
   [`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md):
   through this adapter a re-inserted result overwrites the true sample
   and is broadcast on `/tf`, so the wrong value leaves transforms'
   control (P5).
   - Keep the value at Medium: P5 shows reachability, not occurrence. An
     observed user doing this is a v3 trigger under the warrant report's
     Recommendation 3.
   - No 2.x parallel API: [`DESIGN.md`](../v2.4-proposal/DESIGN.md)
     revision 3 withdrew it, and the 2026-11-03 stance rules out a
     bandaid.
2. **Frame names in insert errors: add the row to the ledger table.**
   It exists only in the "v3 ledger" section of [`dimos.md`](dimos.md) and in
   [`transforms-v3-addendum-corrections.md`](../v3-proposals/transforms-v3-addendum-corrections.md)
   (lines 77-86), not in the warrant report's table.
   - Witnesses: dimos (batch member ambiguity), this crate's clone
     comment (`lib.rs:324-327`), transforms_io's hand formatting,
     whirl_tf's wrapper.
   - Only a break closes it: a parallel framed variant set would give one
     condition two spellings.

### Outreach to roslibrust_transforms

Everything here is adapter-side. The channel exists: the transforms
maintainer authored PRs #317 and #338. Split it: one issue or small PR
for the cheap items, separate questions for policy that roslibrust owns.

**Issue or small PR**, in order of impact:

1. **Carry a typed `RegistryError`** in `TransformManagerError` and
   re-export it. Have `wait_for_transform` fail fast when the request can
   never be served under `max_age` (`NotFoundAt` with `requested` older
   than `covered.end − max_age`; an older-than-`covered.start` request can
   still be backfilled inside the window), and report the last error on
   `Timeout` (Z7). Corroboration: dimos and transforms_io stringify too.
2. **Link `get_transform_at`'s "Temporal metadata limitation"** from the
   wrapper and say never to pass its result to `add_transform`, which
   overwrites and broadcasts (Z4). This is the outreach item in
   [`DESIGN.md`](../v2.4-proposal/DESIGN.md) §3.
3. **Stop teaching lookups at `Timestamp::now()`** in the README, crate
   doc and examples; show the `latest_common_time` idiom (Z9).
   transforms_io maps time zero to `now()` with the same failure.
4. **Require `transforms = "2.2"` or later and add a `reparent_frame`
   passthrough** (Z11). This is
   [`reparent-frame-value/REPORT.md:378-380`](../reparent-frame-value/REPORT.md).
   Also fix the `remove_frame` rustdoc, which still says "The transforms
   crate does not support re-parenting" (`lib.rs:631`, stale since 2.2.0;
   found in the corpus status check).
5. **Fix the time-conversion edges** (Z6), first the `i32::MIN` panic in
   an unwatched task. Reject negative seconds rather than clamp;
   whirl_tf's checked conversion is a model.
6. **Normalise quaternions within a bound the adapter states** before
   `Transform::new`, and log beyond it (Z5). tf2 accepts up to 1e-2 on the
   squared norm; the index does not recommend adopting that bound, since
   it accepts rotations 0.5% off unit
   ([`README.md`](README.md#placement-disagreements-and-how-they-were-resolved)).
7. **Reject empty frame ids now; strip or reject a leading `/`** as tf2
   does (Z1). If the 2.x code item ships, empty ids come free and only
   `/` remains.

**Questions, raised separately:**

8. **The `/tf_static` → `/tf` kind switch** (Z2, Z2b). Ask roslibrust
   to decide it rather than warn and drop: surface it loudly once per
   frame and offer an explicit switch. Do not suggest an automatic
   `remove_frame` and re-add on `StaticDynamicConflict`; transforms' own
   docs forbid that kind of mechanical handler.
9. **Clear dynamic data on a backward time jump** (Z3), on the `tf2_ros`
   precedent.
10. **Observation only:** `/tf_static` is not latched or accumulated on
    ROS 1 (Z12).
11. **Low:** a one-guard latest lookup in the wrapper would make
    transforms' advice followable (F4); `add_transform` can fail after
    committing (Z13).

PR #352 already documents the wasm clock workaround; nothing to add
there beyond the `Static => now()` arm in the examples.

### Do not

- **No core wait, combined get-latest, or `is_retryable`.** Each is
  exact on the public API and two dependents built the first two
  correctly. AGENTS.md Scope names async waiting as out; the two-call
  latest idiom fails loudly between guards; `is_retryable` is rejected
  in [`DESIGN.md`](../v2.4-proposal/DESIGN.md) §6 as speculative, and
  permanence depends on the caller's `max_age`, stream and clock.
- **No core ROS time or message conversion.** Middleware independence is
  the crate's reason to exist, and the correct version needs no
  internals.
- **No nalgebra companion.** This crate has no nalgebra; whirl_tf and
  transforms_io do, without a shared bug.
- **Do not steer bridges to `add_transforms`.** All-or-nothing turns one
  bad member into a dropped TFMessage.
- **Do not loosen `UNIT_NORM_TOLERANCE`.** A denormalised rotation
  corrupts lookups silently (AGENTS.md correctness invariants).
- **No gap bound yet.** It stays "design doc first" in
  [`dimos.md`](dimos.md): Z8 adds reachability in a second dependent,
  not demand.
- **No 2.x cross-time type.** Withdrawn as a bandaid in
  [`DESIGN.md`](../v2.4-proposal/DESIGN.md) revision 3.

## Probes

`P` labels are in `probes/roslibrust_transforms.rs`, `V` labels in
`probes/roslibrust_transforms_verify.rs`. Both were re-run on 2026-11-06
with identical output apart from thread ids in the expected panic line.

| Label | Scenario | Result | File |
|---|---|---|---|
| P1, V1 | `("" → camera, x=1)`, `("" → lidar, x=3)`; camera←lidar at 5 s | `Ok(x=2)` | both |
| P2, V2 | `/tf_static` base→arm x=0.5, then `/tf` at 10 s, 11 s | 2× `WARN`; 10.5 s `Ok(x=0.5)`; latest `Static` | both |
| V2 (reverse) | `/tf` at 10 s, then `/tf_static` | `WARN`; latest `At(10 s)` | verify |
| P3a, V3a | 100–105 s, then 0–3 s | all inserts `Ok`; latest `At(105 s)` → `Ok(x=105)`; 2 s `Err(covers [100, 105])` | both |
| P3b, V3b | run 1 0–8 s x=10t; run 2 0.5/1.5/2.5 s x=0 | 2 s `Ok(x=20)`, 2.2 s `Ok(x=12)`, 3 s `Ok(x=30)`; latest `At(8 s)` | both |
| P4, V4 | `(0,0,.7071,.7071)`, `(0,0,.707,.707)`, f32 yaw 45°, normalised | `Err(NonUnitRotation)`, `Err`, `Ok`, `Ok` | both |
| P5, V5 | `get_transform_at(odom,t2,base,t1,map)` re-inserted | stamp `At(t2)`, wire `(2, 0)`, insert `Ok`; odom←base at t2 5 → −1 (V5: → −3) | both |
| P6, V6 | ROS time edges | pre-epoch → 0, collide; ROS 1 `(10,-1)` → 14.29 s; `SystemTime(-2,+0.5)` → −2.5 s; `i32::MIN` panic; 2039 → negative secs | both |
| P7, V7 | lookup at 50 s, coverage 101–111 s; late 50 s insert | string-only error; late insert `Ok`, still `Err` | both |
| P8, V8 | `remove_frame`, late old-parent message, new-parent add | `Err(ReparentingNotSupported { current_parent: "world" })`; `reparent_frame` `Ok` (P8) | both |
| P9, V9 | lookup 20 ms after the newest sample | `Err(... covers [100, 100])` | both |
| P10, V10 | 100 s x=0, 108 s x=8; lookup 104 s | `Ok(x=4)` | both |
| VX | `Debug` of insert errors | `StaticDynamicConflict`, `ReparentingNotSupported { current_parent }`, `CycleDetected` | verify |
| wasm | `Timestamp::try_now()` in a cdylib on `wasm32-unknown-unknown`, node 24 | trap: `RuntimeError unreachable` | `probes/wasm_try_now.rs` |

## Sources

- **roslibrust_transforms on crates.io:**
  [crate page](https://crates.io/crates/roslibrust_transforms), tarballs
  0.1.0 (`069db4a`), 0.2.0 (`56a2ac6`), 0.24.0 (`df53c2d`), 0.25.0
  (`feeef29`), 0.26.0 (`a798361`).
- **Upstream:** [RosLibRust/roslibrust](https://github.com/RosLibRust/roslibrust)
  master `a798361` (2026-09-25);
  [`roslibrust_transforms/src/lib.rs`](https://github.com/RosLibRust/roslibrust/blob/a798361e8fe9ebbeaeac9e132f93a90fc018ac36/roslibrust_transforms/src/lib.rs).
- **PRs and issues:**
  [#311](https://github.com/RosLibRust/roslibrust/issues/311),
  [#317](https://github.com/RosLibRust/roslibrust/pull/317) (`58c08ef`),
  [#338](https://github.com/RosLibRust/roslibrust/pull/338) (`4759076`,
  merge `a62e832`),
  [#350](https://github.com/RosLibRust/roslibrust/issues/350),
  [#352](https://github.com/RosLibRust/roslibrust/pull/352) (head
  `2539d2d`).
- **tf2:** [`tf2/src/buffer_core.cpp`@`183399c`](https://github.com/ros2/geometry2/blob/183399c89a45d97f22aa32e0adee709c7af8fd0a/tf2/src/buffer_core.cpp),
  [`tf2/src/static_cache.cpp`@`183399c`](https://github.com/ros2/geometry2/blob/183399c89a45d97f22aa32e0adee709c7af8fd0a/tf2/src/static_cache.cpp),
  [`tf2/src/buffer_core.cpp`@`8fef92a`](https://github.com/ros/geometry2/blob/8fef92a6b56a26e2b5be58d4d5c84dc4de5ee696/tf2/src/buffer_core.cpp)
  (noetic).
- **Rust std on wasm:** `library/std/src/sys/time/mod.rs:39-41` and
  `unsupported.rs:34-35` in the local nightly sysroot; Rust 1.85.0
  `library/std/src/sys/pal/wasm/mod.rs:36-37`.
- **Peers read for recurrence:** `transforms_io` 0.8.0, `whirl_tf`
  0.1.0-alpha.3 (crates.io tarballs).
- **This repository:** `f9770d2`; tags `v1.1.0`, `v1.2.0`, `v1.4.1`;
  [`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md),
  [`v2.4-proposal/DESIGN.md`](../v2.4-proposal/DESIGN.md),
  [`reparent-frame-value/REPORT.md`](../reparent-frame-value/REPORT.md),
  [`v3-proposals/`](../v3-proposals/), [`dimos.md`](dimos.md),
  [`transforms_io.md`](transforms_io.md), `AGENTS.md`.
