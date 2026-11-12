# dimos: how it uses transforms and what it teaches

Analysis date: 2026-10-30. Updated: 2026-11-06.
Baseline: master `ea62c00` (released v2.3.0) on 2026-10-30; master
`f9770d2` on 2026-11-06. `src`, `tests`, `examples`, `README.md` and
`MIGRATION.md` are unchanged between the two; only `AGENTS.md` changed
(`dd92937`). Every code line reference below is valid at both.
Subject: [dimos PR #4722](https://github.com/dimensionalOS/dimos/pull/4722),
which replaces dimos's hand-rolled `tf.rs` with `transforms`, read at PR
head `92936a7` (unchanged since 2026-10-27). dimos main read at `abf36ce`.

**Refresh, 2026-11-12.** #4722 is active again: it was force-pushed on
2026-11-09 and its head is now `88e7d0c`, with commits through
2026-11-10. "Dormant" and "unchanged since 2026-10-27" below no longer
hold for the PR. What this report read is unchanged:
`native/rust/dimos-module/src/tf.rs` and
`native/rust/dimos-module/Cargo.toml` are byte-identical at `88e7d0c`
and `92936a7` (same blob hashes, by `gh api`), and the pin is still the
fork `TomCC7/transforms` at rev `5ae9563` with `=2.2.0`. Nothing else
was re-checked, and the text below is kept as written.

This report was "Lessons from dimos" at `analysis/dimos-lessons/REPORT.md`.
It asks what the first non-adapter user of the crate teaches about the 2.4
proposal ([`DESIGN.md`](../v2.4-proposal/DESIGN.md)), the v3 ledger
([`REPORT.md`](../v3-warrant/REPORT.md)) and adoption. Sibling reports:
[`transforms_io.md`](transforms_io.md),
[`roslibrust_transforms.md`](roslibrust_transforms.md), index and synthesis
in [`README.md`](README.md). Nothing here is implemented on master. Every
recommendation that grows the public surface is a maintainer note under
AGENTS.md, not an authorization.

### Changes since 2026-10-30

- Moved from `analysis/dimos-lessons/` and restructured into the shape of
  the other dependent reports. The 2026-10-30 review table, corrections,
  API shapes and probe results are kept.
- Status refreshed: #4722 open with no human review, the stack dormant
  since 2026-10-28, #4366 now conflicting, main still on its own
  nearest-sample `tf.rs`, fork rev `5ae9563` unchanged, no outreach done.
- Corrected: transforms_io 0.8.0 does match a `RegistryError` variant;
  dimos also calls `Transform::validate()` on every lookup result; DESIGN
  rev 3 moved Part A to the v3 ledger; 2.4 is now Part B plus Part C; the
  frame-names-in-insert-errors row was not in the v3 ledger table (added
  2026-11-06); the
  2.4 items "Document clock jumps" and "Reject empty frame names" were never
  carried into DESIGN.md; `src/time/system_time.rs` line reference; a
  large backward jump is not a permanent loss but ends in interleaved
  epochs (`probes/transforms_io_verify.rs` N3).
- Added: a hand-rolled table compared across dependents; the new probe
  `probes/dimos.rs` (P1–P9); the 2.3.0 swap narrowed by P1–P3; whirl_tf's
  `clear_dynamic` as weak evidence for a clear operation; pre-epoch stamps
  as a tested dimos path; an adoption assessment.
- The 2026-10-30 probes ran in a scratch copy and were not saved. Six of
  their scenarios are reproduced in behaviour, not always in numbers, by
  probes in this folder (see Probes).

## Method

2026-10-30:

- Read dimos's old `tf.rs` (main), the new `tf.rs` (PR head) and the PR's
  full file list (`gh api --paginate`).
- Took a third-party review of the same PR as input only, and checked each
  of its claims against source and probes.
- Ran probe programs against the public API at `ea62c00`. They were not
  saved.
- Prototyped the API shapes below: empty-name rejection in a worktree, the
  rest in a scratch copy of the crate.
- Re-checked the findings and the chat answer that preceded this report in
  independent verification passes.

2026-11-06:

- Re-read `tf.rs` at PR head `92936a7` (652 lines) and at main `abf36ce`
  (1205 lines). Re-checked PR, stack, fork and main status with `gh`.
- Grepped all 20 `Cargo.toml` files on dimos main for `transforms`.
- Read the crates.io sources of transforms_io 0.7.0 and 0.8.0,
  roslibrust_transforms 0.26.0 and whirl_tf 0.1.0-alpha.3 for the "Also
  in" column. Read only; nothing from them was built or run.
- Wrote and ran `probes/dimos.rs`, which mirrors dimos's call patterns
  against master `f9770d2`. It does not depend on dimos.
- Applied the cross-dependent scope rulings recorded for the
  transforms_io and roslibrust_transforms reports where they cover a dimos
  item. No scope ruling was made for dimos alone.
- **Not done:** the full `tests/test_all.sh` gate on any prototype;
  running dimos's own test suite against 2.3.0 (P1–P3 mirror three of its
  tests' assertions, nothing more); asking the dimos maintainers anything;
  checking where dimos callers apply the nalgebra isometries `tf.rs`
  exports; GitHub's "used by" graph. tf2 behaviour was read from
  `geometry2` source (rolling), not run.

## Verdict

- **The strict design survived a real integration, but only on a branch.**
  At the PR head dimos kept interpolation, no extrapolation, the strict
  tree and `latest_common_time`; `add_transforms` (2.3.0) came from its
  need. dimos main still runs its own nearest-sample `tf.rs`. #4722 has had
  no commit since 2026-10-27 and its stack none since 2026-10-28.
- **Real use exposes gaps in time handling across a run, not in
  geometry:** clock resets, interpolation across publisher dropouts, and
  diagnostics. transforms_io and roslibrust_transforms reproduce the same
  clock-reset and gap behaviour (`probes/transforms_io.rs` P6, P9;
  `probes/roslibrust_transforms.rs` P3a, P3b, P10). dimos gives no
  evidence for 2.4's remaining code item, Part B.
- **Empty frame names are the strongest core item.** dimos rejects `""`
  itself; both ROS adapters pass it through and return a wrong pose
  between unrelated frames (`probes/transforms_io.rs` P4,
  `probes/roslibrust_transforms.rs` P1). DESIGN.md mentions it only in
  passing (§9).
- **Frame names in insert errors affect all four users.** Three hand-roll
  them; dimos cannot, because `add_transforms` does not say which member
  failed. The row belongs in the v3 ledger table and is not there yet.
- **The cheapest adoption win is still outreach, and none has happened.**
  dimos's branch pins a git fork carrying the declined clone-per-call
  `add_transforms` of PR #123. Assessment below: a small standalone PR to
  dimos main beats waiting on #4722.

## Status

As of 2026-11-06.

| Item | State | Evidence |
|---|---|---|
| Project | dimensionalOS/dimos, a robotics framework with Python and native Rust modules. Not a crates.io crate, so no downloads or reverse dependencies | [`DISCUSSION.md`](../atomic-batch-insert/DISCUSSION.md) (not re-checked) |
| `transforms` on main | None. All 20 `Cargo.toml` files grepped | main `abf36ce` |
| TF on main | `native/rust/dimos-module/src/tf.rs`, the #2816 nalgebra design: nearest sample within a tolerance, no interpolation. Last changed `8a50665` (2026-10-02, #4382). Python TF (`dimos/protocol/tf/tf.py`, `dimos/memory/tf.py`) last changed 2026-08-26 (`a2d98ff`) | main `tf.rs:17-19, 141-172` |
| Requirement on the PR | `transforms = { version = "=2.2.0", git = "https://github.com/TomCC7/transforms", rev = "5ae9563…" }`; also in `examples/native-modules/rust/Cargo.lock` | `native/rust/dimos-module/Cargo.toml:12`@`92936a7` |
| The fork | `bugfix/atomic-batch-insertion` = v2.2.0 + 7 commits (ahead 7, behind 0). Last push 2026-10-22. Not moved | GitHub compare API |
| #4722 | OPEN, REVIEW_REQUIRED, MERGEABLE. +7199/−2936, 139 files. Last commit 2026-10-27T22:21Z. Reviews: bots and the author's own comments of 2026-10-21. No human review | `gh pr view 4722` |
| The stack | #4366 (base main) → #4721 → #4722 → #4735–#4740. #4366 is now CONFLICTING; paul-nechifor asked "Where is the CDR proposal?" (2026-10-13); their two reviews of 2026-10-21 are now DISMISSED. #4774 closed 2026-10-23, folded into #4722 | `gh pr view` |
| Activity | Dormant since 2026-10-28. The author's PRs since then are robot, teleop and control work | `gh` search, 2026-11-06 |
| Other mentions | None name the crate. #4681 (closed unmerged 2026-10-20) shows the motive: f64 seconds lose epoch nanoseconds | [#4681](https://github.com/dimensionalOS/dimos/pull/4681) |
| Outreach | None. A commenter search for deniz-hofmeister in dimos is empty | `gh search` |

Not checked: stars, contributor count, and whether dimos-module is
published anywhere.

## How dimos uses the crate

All at PR head `92936a7`, `native/rust/dimos-module/src/tf.rs`.

- **Types.** `Registry<SystemTime>`, `with_max_age(10 s)`
  (`DEFAULT_TF_WINDOW`, `:44`; `:372`). Signed ROS nanoseconds become
  `SystemTime`, pre-epoch allowed and checked (`:50-81`).
- **API used.** `add_transforms` (`:200`), `latest_common_time` (`:224`),
  `get_transform` (`:231`), `Transform::new` (`:145`) and
  `Transform::validate` on every lookup result (`:159-160`). The last was
  missed on 2026-10-30.
- **Routing.** `Tf::lookup_transform(target, source, time)` in ROS order
  (`:246-262`). ROS time 0 means "latest":
  `latest_common_time(..).at().unwrap_or(UNIX_EPOCH)` (`:220-228`).
  `get_latest` routes through the same path (`:272-273`).
- **Locking.** `RwLock<Registry>` plus tokio `Notify`; one write lock per
  TF message, then `notify_waiters` (`:191-204`). They built
  wait-with-timeout on the public API, which is what AGENTS.md's scope
  rule predicts.
- **Validation in front of the crate.** Frame names must be non-empty
  with no leading `/`, on insert (`:133-141`) and lookup (`:212-219`). A
  finite and norm check (`|q·q − 1| ≤ 1e-6` on the squared norm) runs
  before `Transform::new`, which already checks (`:114-131`, `:142`).
- **Error handling.** No `RegistryError` variant is matched. Insert
  errors become `io::Error`: returned from `publish` (`:308`), or logged
  by the subscription route as one `tracing::warn!("TF message
  rejected")` with only the error text (`:361`). That text names no
  frame, except `ReparentingNotSupported`'s current parent. Lookup errors
  become `io::Error` strings from `lookup_transform`, and `None` from
  `get_latest`, `Lookup::get` and `Lookup::within` (`:273`, `:326`,
  `:342`).
  Correction: on 2026-10-30 this report said the same held for "the three
  crates.io dependents". That was wrong since transforms_io 0.8.0
  (2026-10-27), which matches `RegistryError::ReparentingNotSupported` to
  log a conflict once per frame pair
  (`transforms_io-0.8.0 src/mt_pubsub_backend.rs:12, 342`). roslibrust
  0.26.0 matches none.
- **Static results.** A static result carries `UNIX_EPOCH` on the wire
  (`:165`, `:227`), which ROS reads as "latest".
- **Never touched.** `add_transform`, `get_transform_at`,
  `get_transform_for`, `reparent_frame`, `remove_frame`,
  `remove_transforms_before`, `Registry::new`, `Timestamp`, `Point`,
  `Transformable`, the `serde` feature.
- **Python/Rust split.** The same PR keeps Python TF on nearest-sample
  plus tolerance while Rust now interpolates. The PR body (line 5) calls
  the "interpolation/common-time behavior differences" "a separate review
  concern".

## History

### Before transforms

- main's `tf.rs` (#2816) buffers each edge in f64 seconds, answers with
  the nearest sample within a tolerance (default: the whole window), and
  never interpolates (main `tf.rs:141-172`).
- #4681 tried to keep exact source stamps in that design and was closed
  unmerged on 2026-10-20: "Generated TF timestamps cannot pass through f64
  seconds without losing epoch nanoseconds". `transforms`' u64-nanosecond
  `Timestamp` and `SystemTime` keys give that by construction. That is
  positive evidence for the current time design.

### The migration (#4722, 2.2.0 fork)

- TomCC7 opened transforms PR #123 (clone-per-call `add_transforms`) and
  pinned dimos to the fork at `5ae9563`. transforms declined #123 as
  written (closed 2026-10-26) and shipped a check-then-commit
  `add_transforms` with the same signature in 2.3.0 (2026-10-28)
  ([`DISCUSSION.md`](../atomic-batch-insert/DISCUSSION.md)). The fork still
  clones the whole registry on every call (fork `mod.rs:171`), so dimos's
  pinned build carries a Θ(N + F) cost per message.
- **Gained:** exact nanosecond stamps, interpolation, atomic batches,
  `latest_common_time` over the whole chain.
- **Lost:**
  - **Clock-reset handling,** and its test
    `a_clock_reset_drops_the_pre_jump_samples`. main cleared an edge when
    a stamp arrived a whole window behind its newest (main
    `tf.rs:112-120`). After a simulation reset, `get_latest` now keeps
    serving the pre-reset pose.
  - **Rate-limited warnings per frame pair on failed lookups** (main
    `tf.rs:296-320`). A failed `get_latest` is now a silent `None`.
  - **Any bound on staleness.** The old tolerance was loose (default: the
    whole window), but it existed. Interpolation now spans any retained
    gap.

### Swapping to crates.io 2.3.0

Narrowed, not closed. `probes/dimos.rs` P1–P3 reproduce the assertions of
three dimos tests against 2.3.0 and all hold: latest common time 1 s with
x = 11 (`tf.rs:457-477`), static plus eviction gives x = 5 at 30 s
(`:509-537`), and a cycle batch is rejected with nothing stored
(`:575-613`). dimos's own suite has not been run.

### The third-party review of #4722, checked (2026-10-30)

| Claim | Verdict | Evidence |
|---|---|---|
| 1. Maximum interpolation gap | Confirmed | 100 s / 108 s samples give 4.0 at 104 s. A caller cannot build the check outside: the public API exposes no samples or coverage. One registry-wide limit is the wrong shape; it cannot serve a 50 Hz edge and a 1 Hz edge together. |
| 2. Error on a backward clock jump | Partial | Confirmed on `max_age` registries: 3000 inserts return `Ok`, none are kept, `latest_common_time` stays at 1000 s. It misses the worse cases: the default registry mixes epochs (444.44 at 500 s), and jumps smaller than `max_age` interleave old and new samples. The proposed error would also reject a whole atomic batch. |
| 3. Reject empty frame names | Agree | Stronger than given: `""` is the default for unset string fields in ROS 2 messages. Two publishers that both forget `frame_id` share a `""` root, and lookups between unrelated frames succeed (camera→lidar = 2.0). |
| 4. `UnknownFrame` for equal unregistered names | Reject | Identity is documented (`src/core/registry/mod.rs:264-265`). dimos's own test asserts that `get_latest("map","map")` returns identity (`tf.rs:444-455`). Only `""` and `"/map"` must fail there. |
| Quaternion tolerance "level" | Wrong | Old dimos normalized any quaternion with norm above 1e-9. New dimos adopted a 1e-6 check on the squared norm, in front of the crate's own check. |
| nalgebra companion crate | Weak | dimos converts the ROS wire type to and from `transforms`, and the wire type to and from nalgebra. It never converts `transforms` ↔ nalgebra. |
| Adapter and reject lists | Agree | — |

### Corrections to the chat answer that preceded the 2026-10-30 report

- **The `SystemTime` NaN is not a bug.** `src/time/system_time.rs:28-33`
  and the `TimePoint` docs make NaN the documented result for instants
  that cannot be converted, and the typed payloads stay intact. Printing
  negative seconds changes that contract: a decision, not a patch fix.
- **tf2 clears statics too.** `tf2_ros` clears on any backward jump
  (`onTimeJump`), and `BufferCore::clear` also drops static transforms.
  ROS restores them from latched `/tf_static`. dimos has no such
  durability on LCM/Zenoh, so `remove_transforms_before`, which keeps
  statics, is the right recovery for them.
- **"Clearing is merely verbose outside the crate" was overstated.** It
  needs an instant after all stored data; neither `TimePoint` nor
  `SystemTime` has a maximum, though a far-future value works. Detecting a
  jump per edge needs each edge's newest stamp, which the API does not
  expose. Clearing the whole registry, triggered by whoever owns the
  clock, works today, with the cutoff hole described under API shapes.
- **C.4 (`Stamp` vs `T`) is consistent with dimos's usage, not confirmed
  by it.**
- **Swapping in 2.3.0 is a signature match, not proven to behave the
  same.** Narrowed on 2026-11-06 by P1–P3 (above).

## Hand-rolled around the crate

dimos at `92936a7`. "Also in": transforms_io 0.8.0, roslibrust_transforms
0.26.0, whirl_tf 0.1.0-alpha.3 (pinned to `=2.0.0-beta.4`, dormant since
2026-08-29).

| Feature | Where | Correct? | Exact on public API? | Also in |
|---|---|---|---|---|
| Wait-for-transform with timeout | `tf.rs:336-352`, `Notify` armed before resolve | Yes, race-free | Yes; AGENTS.md Scope names it as outside | roslibrust_transforms (`lib.rs:455-510`, `watch`); whirl_tf (`MessageFilter`, `message_filter.rs:123-145`, poll-based) |
| Frame-name check: non-empty, no leading `/` | `tf.rs:133-141` (insert), `:212-219` (lookup); test `:444-455` | Yes | Yes, but leaving it out returns wrong poses (P4 in `probes/transforms_io.rs`) | whirl_tf (`buffer.rs:377-392`, strips `/`, rejects empty). Not in transforms_io or roslibrust_transforms |
| Pre-insert finite and unit-norm check | `tf.rs:114-131` | Redundant and about twice as strict as the crate (P5) | Yes; `Transform::new` already checks | whirl_tf (`buffer.rs:309-337`, the crate's constant) |
| Re-validate every lookup result | `tf.rs:159-160` | Catches `Ok(inf)` (P6). Can reject a valid long chain (Hazard 5, not probed) | Yes | — |
| "Latest" lookup (ROS time 0) | `tf.rs:220-228`, `:254-261` | Yes (P1, P7) | Yes | roslibrust_transforms (passes `latest_common_time` through, `lib.rs:618`); transforms_io uses `Timestamp::now()` instead, which usually fails loudly (P8) and succeeds silently at `now()` with post-dated or skewed hops (`probes/transforms_io_verify.rs` V1, N4) |
| ROS `Time` ↔ `SystemTime` | `tf.rs:50-81`, signed, checked | Yes | Yes; ROS-specific, so outside | Both ROS adapters clamp silently: transforms_io clamps negative seconds to 0 and keeps the nanoseconds, roslibrust_transforms maps a pre-epoch stamp to `Timestamp::zero()`; whirl_tf rejects them |
| Static result → wire stamp | `UNIX_EPOCH` (`tf.rs:165`, `:227`) | Yes | Yes | roslibrust_transforms `(0,0)`; transforms_io echoes the request |
| Frame names in insert errors | None; `warn!` with only the error text (`tf.rs:361`) | Gap | Per element only; impossible for `add_transforms` | transforms_io (`mt_pubsub_backend.rs:335-355`), roslibrust_transforms (`lib.rs:324-331`), whirl_tf (`buffer.rs:40-46`) |
| Batch insertion | `add_transforms`, `tf.rs:191-204` | Yes, atomic (P3) | Shipped in 2.3.0 | transforms_io, roslibrust_transforms per element (skip and log); whirl_tf commits partial batches (`buffer.rs:78-86`) |
| Per-pair warning dedup | Lost (main `tf.rs:296-320`) | — | Yes | transforms_io, `ReparentingNotSupported` only (`mt_pubsub_backend.rs:342-352`) |
| Clock-reset handling | Lost (main `tf.rs:112-120`) | — | Registry-wide only, with the cutoff hole; not per edge | whirl_tf `clear_dynamic`, manual |
| Clear dynamic data | None | — | `remove_transforms_before(far)` keeps pins; a `remove_frame` loop releases them (P9) | whirl_tf (`buffer.rs:200-204`, `remove_frame` loop) |
| Staleness bound / maximum gap | Lost (main `tf.rs:141-172`) | — | No: no sample or coverage access | — (whirl_tf's `tolerance_ns` waits for coverage; it is not a gap bound) |

Notes:

- **The frame-name check splits along a telling line.** The two users that
  are not thin adapters (dimos, whirl_tf) reject `""`. The two ROS
  adapters, which ingest raw messages where `""` is the field default, do
  not.
- **Frame names in insert errors recur in all four.** Three wrap or log
  the names themselves. The fourth, dimos, uses the batch API and cannot.
- **Clock reset and the gap bound are hand-rolled by no current user.**
  dimos had both before the migration; nothing on the public API lets it
  rebuild them per edge.
- **The `remove_frame` loop is not equivalent to a clear.** It releases
  parent pins, so a re-parented edge is accepted afterwards (P9).

## Hazards

1. **A clock reset keeps serving the pre-reset pose.** Silent.
   - With `max_age` 10 s, inserts after a backward jump return `Ok` and
     are evicted on arrival; `latest_common_time` stays at the old end, so
     `get_latest` serves the old pose. That is not permanent: once the new
     stamps reach the old newest minus `max_age`, they are kept and
     interleave with the old epoch (`probes/transforms_io_verify.rs` N3).
     A jump smaller than `max_age` interleaves both epochs at once, and
     lookups return poses neither epoch contains. Without `max_age`,
     epochs mix.
   - Evidence: 2026-10-30 probes (O2, O4, O5); `probes/transforms_io.rs`
     P6 (`Ok(-2.4999)` where the epochs say 2.52 and −10);
     `probes/roslibrust_transforms.rs` P3a, P3b.
   - Cause: dimos dropped its detector. In transforms the behaviour is the
     documented expiry rule, but the consequence is not written down.
2. **Interpolation across any dropout.** Silent.
   - An 8 s gap inside `max_age` is interpolated (4.0 at 104 s).
   - Evidence: O1; `probes/transforms_io.rs` P9;
     `probes/roslibrust_transforms.rs` P10.
   - Cause: transforms' documented unbounded default
     (`src/core/registry/mod.rs:267`). dimos lost its tolerance.
3. **Empty frame names join unrelated trees.** Silent where unguarded.
   - `("", camera)` and `("", lidar)` give camera→lidar = 2.0.
   - Evidence: O6; `probes/transforms_io.rs` P4;
     `probes/roslibrust_transforms.rs` P1.
   - Cause: transforms accepts `""`. dimos is protected by its own check
     (`tf.rs:133-141`); the two ROS adapters are not.
4. **Failed lookups vanish.** Silent: no log, and the caller sees only
   `None`.
   - `get_latest`, `Lookup::get` and `Lookup::within` turn every error
     into `None` with no log (`tf.rs:273`, `:326`). main warned once per
     pair per interval.
   - Cause: dimos.
5. **Re-validating lookup results can reject a valid chain.** A refusal,
   not a wrong answer (`Err`, or `None` from `get_latest`). Not probed.
   - Composition does not renormalize, and a product's norm is the
     product of the norms. dimos admits rotations with `|q|` up to about
     1 + 5e-7. Three such hops compose to about 1 + 1.5e-6, which
     `validate()` at `tf.rs:159-160` rejects as `NonUnitRotation`. This is
     why AGENTS.md says derived values are deliberately not re-validated.
     Realistic inputs sit far closer to 1 (f32-widened: about 3e-8,
     `probes/roslibrust_transforms.rs` P4), so the risk is low.
   - Cause: dimos. The upside is real: `Ok(inf)` becomes an error (P6).
6. **Two TF implementations in one system answer differently.** Silent.
   - Python stays nearest-sample, Rust interpolates (PR body, line 5).
   - Cause: dimos.
7. **Pre-epoch diagnostics print NaN.** Loud, diagnostic only.
   - A lookup miss on pre-epoch data prints `covers [NaN, NaN]` (P4).
     Pre-epoch stamps are a tested dimos path (`tf.rs:521-546`).
   - Cause: transforms' documented `as_seconds_lossy` contract
     (`src/time/system_time.rs:28-33`).

## Friction

- **No batch insert in 2.2.** dimos needed all-or-nothing TF messages,
  which led to PR #123 and the fork. Fixed by 2.3.0; the pin remains.
- **Batch errors name no member.** `add_transforms` reports the first
  error without its transform (CHANGELOG 2.3.0), and the unit variants
  carry no frames. dimos logs "TF message rejected" with the error text,
  which names no member.
- **No per-edge newest stamp, samples or coverage.** dimos cannot port its
  per-edge clock-reset detector or its staleness tolerance. This is by
  design: AGENTS.md Scope forbids exposing raw state.
- **`latest_common_time` returns a `Stamp`.** dimos needs
  `.at().unwrap_or(UNIX_EPOCH)` to turn an all-static answer into an
  instant. It got this right; transforms_io did not (P8).
- **Clearing needs a far-future cutoff.** `TimePoint` has no maximum, and
  `UNIX_EPOCH + d` panics past the platform range, so a `SystemTime`
  registry needs the recipe under API shapes.

## Recommendations

Placement follows the cross-dependent scope rulings recorded for the
[transforms_io](transforms_io.md) and
[roslibrust_transforms](roslibrust_transforms.md) reports; no ruling was
made for dimos alone. Until DESIGN revision 4 (2026-11-06), two
2026-10-30 "2.4" items were not carried into
[`DESIGN.md`](../v2.4-proposal/DESIGN.md). Revision 4 adopts both:
empty-name rejection as Part D (at the insert boundary, not the shape
below) and the clock-jump text as Part C item 5. Neither has shipped (CHANGELOG has no
`[Unreleased]` section) or has a maintainer decision. Part A, which the
2026-10-30 report said to keep, moved to the v3 ledger in DESIGN rev 3
(`bad410a`, 2026-11-03). 2.4 is now Part B (one method, pending decision
D2) plus Part C (docs only).

### transforms 2.x code

- **Reject empty frame names.** A new `TransformError::EmptyFrameName`,
  enforced by `Transform::validate`, so by the constructors,
  `Deserialize` and insert. Lookups with an empty endpoint return
  `UnknownFrame("")`. The leading-`/` rule stays in adapters.
  - Invariant: a lookup must return the requested frames, and chain
    resolution treats equal names as one frame. `""` is ROS 2's unset
    `frame_id`, not a frame; two publishers that omit it share a
    fabricated root (Hazard 3).
  - Corroboration: dimos and whirl_tf hand-roll it; transforms_io and
    roslibrust_transforms omit it and return 2.0 between unrelated frames.
    tf2 rejects it (`buffer_core.cpp`, `validateFrameId`).
  - Strengthens the 2026-10-30 item; ruled core 2.x code in both sibling
    reports. Not a bandaid: no parallel API.
  - DESIGN revision 4 adopts it as Part D, but at the insert boundary
    (`Buffer::check`, a flat `RegistryError::EmptyFrameName`), which
    leaves constructors, `Deserialize` and the identity rule unchanged.
  - Needs: explicit sign-off (one variant), a MIGRATION note (stored `""`
    data stops decoding), an AGENTS.md edit for the identity carve-out.
    Shipping it makes 2.4 a minor even without Part B. If the maintainer
    judges the tightening a break, it becomes a plain v3 ledger row, with
    outreach to the two ROS adapters for 2.x.
- **Decide on negative seconds for pre-epoch `SystemTime`.** Changes the
  documented `as_seconds_lossy` contract from NaN to negative seconds.
  - New evidence: pre-epoch stamps are tested in dimos (`tf.rs:521-546`),
    so `covers [NaN, NaN]` is reachable in a real dependent (P4).
  - Not a parallel API; a contract change for a minor release. Low
    priority: the typed payloads are intact. Shape under API shapes.
- **Part B (`transform_positions`): no change from dimos.** dimos uses
  neither `Point` nor `Transformable`. DESIGN revision 4 changed D2 to
  recommend No for 2.4, citing every studied dependent; the two
  documents now agree. The maintainer has not decided.

### transforms docs

- **Document clock jumps.** On `with_max_age`: an insert older than the
  frame's newest minus `max_age` returns `Ok` and is evicted at once; a
  backward jump of any size ends in a window where two epochs interleave,
  because a large jump's new stamps are kept once they reach that bound
  (`probes/transforms_io_verify.rs` N3, found in the transforms_io
  verification); without `max_age`, epochs mix. Recovery is
  `remove_transforms_before(<after all data>)`, which keeps statics,
  topology, kind and `max_age`; "before" is strict, so a sample at the
  cutoff survives. Also state that a request older than
  `covered.end − max_age` can never be served. Detection belongs to
  whoever owns the clock (the `tf2_ros` precedent).
  - Corroboration: transforms_io (10 s default) and roslibrust_transforms
    sit in the same configuration (Hazard 1 probes). Ruled docs in both
    sibling reports.
  - Strengthens the 2026-10-30 item. Belongs in DESIGN Part C.
- **Part C items 2 and 4** (`latest_common_time` discoverability;
  `Stamp` vs `T`). dimos is the correct pattern; transforms_io's `now()`
  is the miss (P8). Strengthens the existing items; nothing new.

### Design doc first (unchanged)

- **Maximum interpolation gap, opt-in,** keeping the unbounded default.
  Recommended shape: a per-frame bound, which also changes how
  `latest_common_time` is computed (API shapes).
  - Evidence: dimos lost its tolerance; the gap is reachable in two more
    dependents (Hazard 2). Reachability is not demand: no user has asked.
    v3-2 §5 makes gap limits conditional on concrete requirements
    ([`transforms-v3-2.md:57`](../v3-proposals/transforms-v3-2.md)), and
    v3-3 records no demonstrated need
    ([`transforms-v3-3.md:229`](../v3-proposals/transforms-v3-3.md)). Both
    sibling rulings keep this status.
- **A registry clear operation, only on request.**
  - New weak evidence: whirl_tf's `clear_dynamic` (`buffer.rs:200-204`),
    which releases pins unlike the proposed operation (P9). whirl_tf is on
    beta.4 and dormant. Nobody has asked.

### v3 ledger

- **Frame names in insert errors** for `SelfReferentialFrame`,
  `CycleDetected` and `StaticDynamicConflict`. A break: unit variants gain
  fields; a parallel set of variants would break the flat-error rule and
  be a bandaid.
  - Correction: on 2026-10-30 this was recorded as added. It is not in
    the table "What genuinely requires a break"
    ([`REPORT.md`](../v3-warrant/REPORT.md)). That report lists
    "frame-less errors" only among its smaller findings (lines 141-142),
    and
    [`transforms-v3-addendum-corrections.md`](../v3-proposals/transforms-v3-addendum-corrections.md)
    (lines 84-86) says "that item stands". The action is to add the row.
  - Corroboration: all four users (Hand-rolled table). Low value; not a v3
    trigger on its own.
- **`Rotation` newtype and translation bound** (existing row). dimos's
  lookup re-validation is the first evidence that a user wants finite
  outputs. DESIGN §6 leaves the end-of-lookup check to this row.
  Strengthens it; no 2.x item.
- **Cross-time result type** (existing row, from former Part A). dimos
  does not call `get_transform_at`; no dimos evidence either way. See
  [`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md).

### Outreach to dimos

None of the 2026-10-30 outreach was done. #4722 has had no activity since
2026-10-27 and its stack is stalled at #4366, so a comment there reaches
few people.

**Assessment (2026-11-06, an estimate, not evidence):** about 15–25% odds
that #4722 lands as is. The better path is a small standalone PR to dimos
main that swaps `tf.rs` for crates.io `transforms = "2.3"`. Reasons: the
PR's size, the dormant stack, #4366's conflicts and reviewer pushback, and
main's nearest-sample semantics, which #4722 itself leaves as an open
concern. A standalone swap still has to justify interpolation against
Python's nearest-sample TF.

Contents for that PR, or for a comment if #4722 revives:

- Use crates.io 2.3 instead of the fork. It drops the clone-per-call cost
  (History).
- Restore clock-reset handling at the registry level: whoever owns the
  clock calls `remove_transforms_before(far)` on a backward jump.
- Restore per-pair warnings for failed lookups (Hazard 4).
- Drop the redundant pre-insert norm check, or match the crate's
  `|norm − 1| ≤ 1e-6` (P5). Keep the lookup re-validation only knowing
  Hazard 5.
- Ask which gap tolerances they would set per edge.
- Ask whether Python bindings would end the Python/Rust split. Two
  exist, both stale: art-e-fact/transforms-py binds 1.0.0 (0 stars, last
  push 2025-11-04), and whirl_tf's PyO3 package `whirl-tf` binds
  2.0.0-beta.4 (`README.md:10,19`@0.1.0-alpha.3).

### Do not

- **Return `UnknownFrame` for equal unregistered names.** Identity is
  documented and dimos tests it (`tf.rs:444-455`). Only `""` changes.
- **Make an `Expired`-on-insert error the default in 2.x.** A behaviour
  break. A 2.x opt-in knob is ruled out by the no-bandaid stance; the
  transforms_io report records it as a contested, low-priority v3
  candidate.
- **Move the leading-`/` rule into the core.** Stripping it is a silent
  guess (tf2 does it); it stays adapter policy.
- **Add a finiteness check at the end of every lookup in 2.x,** even
  though dimos does it. It contradicts the documented "not re-validated"
  contract (DESIGN §6).
- **Per-lookup gap bounds.** Rejected in API shapes.
- **Build nalgebra or async companion crates, or a core "get latest".**
  dimos never converts `transforms` ↔ nalgebra; wait and latest are exact
  on the public API, and two dependents built them correctly.

## API shapes

The exact public surface each recommendation would add, from 2026-10-30.
Empty-name rejection was prototyped in a worktree; the other shapes were
compiled or probed in a scratch copy of the crate. Nothing is committed.

### 2.x: reject empty frame names

```rust
// transforms::errors::TransformError — #[non_exhaustive], additive
/// The parent or child frame name is empty.
#[error("frame name is empty")]
EmptyFrameName,
```

- `Transform::validate` checks the names first, so `new`, `static_between`,
  `Deserialize` and insert-time validation all reject an empty name.
- Lookups with an empty endpoint (`get_transform`, `_for`, `_at`,
  `latest_common_time`) return the existing `UnknownFrame("")`. That
  includes `""` against itself: a carve-out from the identity rule that
  AGENTS.md must state.
- No flat `RegistryError` variant. The error cannot reach the registry
  except through the existing wrapped arm, and only in theory.
- Public surface: one variant. The prototype is about 26 lines outside
  tests. Tests pass with default features, `no_std` and `serde`; clippy is
  clean. The full gate was not run.

### Docs: clock-jump recipe

```rust
// Registry<Timestamp>
registry.remove_transforms_before(Timestamp::from_nanos(u64::MAX));

// Registry<SystemTime>: `UNIX_EPOCH + d` panics past the platform range
let far = [i64::MAX as u64, 1 << 39, 1 << 36]
    .into_iter()
    .find_map(|s| UNIX_EPOCH.checked_add(Duration::from_secs(s)))
    .unwrap();
registry.remove_transforms_before(far);
```

- No new API. The rustdoc must say that "before" is strict.
- **The hole:** a sample stamped exactly at the cutoff survives, keeps the
  `max_age` expiry anchor there, and evicts every sample of the restarted
  stream (probe O9). A garbage stamp of `u64::MAX` is reachable, for
  example from a cast of `-1`.
- `probes/transforms_io_verify.rs` V6 shows the recipe recovering
  (`@2.5s Ok(-10.0)` after the reset).

### On request: clear operation

```rust
impl<T: TimePoint> Registry<T> {
    /// Removes every dynamic sample. Static transforms, each frame's pinned
    /// parent and kind, and `max_age` remain; expiry anchors are reset.
    pub fn remove_dynamic_transforms(&mut self);
}
```

- Not the same as `remove_transforms_before(+∞)`: it also removes a sample
  at the maximum stamp and always resets the expiry anchor.
- Not the same as whirl_tf's `remove_frame` loop either, which releases
  pins (P9).
- It cannot be built generically outside the crate, because `TimePoint`
  has no maximum.
- The recipe's hole strengthens the case, but the hole fails loudly (the
  restarted stream is rejected, not served wrong). Ship only when someone
  asks. The name follows the existing `remove_*` methods.

### Design doc: maximum interpolation gap

```rust
impl<T: TimePoint> Registry<T> {
    /// Bounds the interpolation gap of a dynamic child frame's edge;
    /// `None` restores the unbounded default.
    pub fn set_max_gap(
        &mut self,
        child_frame: &str,
        max_gap: Option<Duration>,
    ) -> Result<(), RegistryError<T>>;
}

// RegistryError<T> — #[non_exhaustive], additive
#[error("transform from {source_frame} into {target_frame} at {} not found \
         ({frame} samples at {} and {} are further apart than the {} s limit)", ..)]
GapExceeded {
    target_frame: String,
    source_frame: String,
    frame: String,
    requested: T,
    before: T,          // newest sample before `requested`
    after: T,           // oldest sample after `requested`
    max_gap: Duration,
},
#[error("frame {0} has no sampled edge to bound")]
NoSampledEdge(String),  // root or static frame
```

- **Semantics.** A request strictly between samples `b < t < a` fails if
  and only if `a − b > max_gap`. A request exactly on a sample is always
  served, which keeps the bit-for-bit single-hop invariant. `ZERO` serves
  exact hits only. The bound is retroactive. `reparent_frame` keeps it;
  `remove_frame` drops it.
- **Errors.** `UnknownFrame` for a name that exists nowhere.
  `NoSampledEdge` for a root or static frame: a silent `Ok` there would
  make the caller believe a bound is active.
- **`latest_common_time` must change.** Min-of-ends can land inside a gap
  the bound vetoes. Example: hop A ends at 10 s; hop B has samples at 2 s
  and 12 s with a 5 s bound. Today's rule answers 10 s, which cannot be
  served; the correct answer is 2 s. Fix: start from the min of the ends,
  step each hop down to its newest servable instant at or before it, and
  repeat until nothing changes. About O(h log n) per call; keep the
  current fast path when no hop has a bound. The `NoCommonTime` payload is
  unchanged.
- **Setup window.** The frame must exist first, so `set_max_gap` follows
  its first insert. A single sample serves exact hits only, so the window
  does no harm.
- **Rejected: per-lookup bounds.** Four bounded twins (`get_transform`,
  `_for`, `_at`, `latest_common_time`) plus a variant. The unbounded
  siblings stay footguns, and the bound passed to `latest_common_time` can
  disagree with a later `get_transform`'s. Per frame costs one method and
  two variants.

### Decision: negative seconds for pre-epoch `SystemTime`

```rust
// src/time/system_time.rs — no signature change
fn as_seconds_lossy(self) -> f64 {
    match self.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_secs_f64(),
        Err(before) => -before.duration().as_secs_f64(),
    }
}
```

- The `TimePoint::as_seconds_lossy` docs change to: NaN only for a value
  with no seconds representation.
- Visible beyond `Display`: `TransformError`'s f64 payloads
  (`TimestampMismatch`, `TimestampOutOfRange`) go from NaN to negative
  numbers. One test changes (`tests/time_traits.rs:433`).
- A minor release with a changelog line.
- Reachable in dimos, which tests pre-epoch stamps (P4).

### v3: frame names in insert errors

```rust
// RegistryError<T> — breaking: unit variants gain fields
#[error("frame {frame} cannot be its own parent")]
SelfReferentialFrame { frame: String },
#[error("making {parent} the parent of {child} would create a cycle in the frame tree")]
CycleDetected { parent: String, child: String },
#[error("cannot mix static and dynamic transforms for child frame {frame}")]
StaticDynamicConflict { frame: String },
```

- Every construction site already holds the transform: `Buffer::check`,
  the insert fallthrough, and three `CycleDetected` sites. A scratch copy
  of the library builds with these variants.
- `NonUnitRotation` and `NonFiniteValues` stay as they are.
  `From<TransformError>` has no frame to add, and a lookup-side
  `NonFiniteValues` comes from a composed chain with no single frame to
  blame.

## Probes

`probes/dimos.rs`, run against master `f9770d2` (2026-11-06). Output from
the built binary; P8 depends on the wall clock.

| Label | Scenario | Result | File |
|---|---|---|---|
| P1 | dimos test `latest_resolves_the_whole_chain…` (`tf.rs:457-477`): map→odom at 1 s and 2 s, odom→base at 1 s; latest both ways | map←base x = 11 at 1 s; base←map x = −11. Matches the test | `probes/dimos.rs` |
| P2 | dimos test `static_edges_survive_eviction…` (`tf.rs:509-537`) | map←camera x = 5 at 30 s; base←camera at −1 ns x = 3, stamp −1 ns. Matches | `probes/dimos.rs` |
| P3 | dimos test `invalid_batches_do_not_change_the_graph…` (`tf.rs:575-613`), cycle half | `CycleDetected`; base←camera afterwards: `UnknownFrame`. Nothing stored | `probes/dimos.rs` |
| P3b | Rotation w = 0 (`tf.rs:584`) | `Transform::new` → `NonUnitRotation(0.0)` | `probes/dimos.rs` |
| P4 | Pre-epoch samples, lookup miss | `base covers [NaN, NaN]` | `probes/dimos.rs` |
| P5 | Norm 1 + 8e-7: dimos squared check vs `Transform::new` | dimos rejects; the crate accepts | `probes/dimos.rs` |
| P6 | Two static hops of 1.5e308: raw lookup vs dimos path | raw `Ok(inf)`; dimos `Err(non-finite)` | `probes/dimos.rs` |
| P7 | All-static chain, "latest" | stamp 0 ns; `latest_common_time` is `Static` | `probes/dimos.rs` |
| P8 | transforms_io's `Time{0,0}` → `Timestamp::now()` | `Err(NotFoundAt … covers [1, 2])` | `probes/dimos.rs` |
| P9 | whirl_tf's `remove_frame` clear vs `remove_transforms_before(MAX)`, then re-parent | after `remove_frame`: accepted; after the cutoff: `ReparentingNotSupported` | `probes/dimos.rs` |

2026-10-30 scratch probes against `ea62c00`. They were not saved; labels
added on 2026-11-06.

| Label | Scenario | Result | File |
|---|---|---|---|
| O1 | `max_age` 10 s; x=0 at 100 s, x=8 at 108 s; query 104 s | `Ok(4.0)` | not saved; reproduced by `probes/transforms_io.rs` P9, `probes/roslibrust_transforms.rs` P10 |
| O2 | `max_age` 10 s; history 990–1000 s; 3000 inserts near 100 s | 3000× `Ok`, 0 kept, `latest_common_time` = `At(1000 s)` | not saved; reproduced by `probes/transforms_io.rs` P6, `probes/roslibrust_transforms.rs` P3a |
| O3 | …then `remove_transforms_before(far)`; insert 100 s, 101 s; query 100.5 s | `Ok(1.5)` | not saved; reproduced by `probes/transforms_io_verify.rs` V6 |
| O4 | `new()`; 50 s, 1000 s, then x=0 at 100 s; query 500 s | `Ok(444.44)`, two epochs mixed | not saved |
| O5 | `max_age` 10 s; 100–105 s, then x=−1 at 102.5 s; query 102.4 s | `Ok(19.6)`, interleaved | not saved; reproduced by `probes/roslibrust_transforms.rs` P3b |
| O6 | `("", camera)` x=1, `("", lidar)` x=3; camera→lidar | `Ok(2.0)` | not saved; reproduced by `probes/transforms_io.rs` P4, `probes/roslibrust_transforms.rs` P1 |
| O7 | ghost→ghost on an empty registry | identity; `latest_common_time` = `Static` | not saved |
| O8 | `Registry<SystemTime>`, pre-epoch samples, failed lookup | `"covers [NaN, NaN]"` | not saved; reproduced by `probes/dimos.rs` P4 |
| O9 | Cutoff recipe with a sample exactly at the cutoff, then a restarted stream | sample survives, anchors expiry, restarted stream evicted | not saved |

## Sources

- dimos:
  [PR #4722](https://github.com/dimensionalOS/dimos/pull/4722),
  [new `tf.rs` (PR head `92936a7`)](https://github.com/dimensionalOS/dimos/blob/92936a710f4d87a6db4153db35d9db44236456da/native/rust/dimos-module/src/tf.rs),
  [old `tf.rs` (main `abf36ce`)](https://github.com/dimensionalOS/dimos/blob/abf36ce/native/rust/dimos-module/src/tf.rs),
  [#4366](https://github.com/dimensionalOS/dimos/pull/4366),
  [#4721](https://github.com/dimensionalOS/dimos/pull/4721),
  [#4774](https://github.com/dimensionalOS/dimos/pull/4774),
  [#4681](https://github.com/dimensionalOS/dimos/pull/4681). Status via
  `gh` on 2026-11-06.
- Fork: [TomCC7/transforms](https://github.com/TomCC7/transforms) at
  `5ae9563` (`bugfix/atomic-batch-insertion`);
  [transforms PR #123](https://github.com/deniz-hofmeister/transforms/pull/123).
- Other dependents, crates.io sources: transforms_io 0.7.0 and 0.8.0,
  roslibrust_transforms 0.26.0, whirl_tf 0.1.0-alpha.3
  ([IntelligentRoboticsLab/whirl_tf](https://github.com/IntelligentRoboticsLab/whirl_tf)
  `33dfd66`). Off crates.io:
  [art-e-fact/transforms-py](https://github.com/art-e-fact/transforms-py).
- tf2: [`tf2_ros/src/buffer.cpp`](https://github.com/ros2/geometry2/blob/rolling/tf2_ros/src/buffer.cpp)
  (`onTimeJump`), [`tf2/src/buffer_core.cpp`](https://github.com/ros2/geometry2/blob/rolling/tf2/src/buffer_core.cpp)
  (`clear`, `validateFrameId`).
- This repository: [`DESIGN.md`](../v2.4-proposal/DESIGN.md) (rev 3),
  [`v3-warrant/REPORT.md`](../v3-warrant/REPORT.md),
  [`transforms-v3-addendum-cross-time.md`](../v3-proposals/transforms-v3-addendum-cross-time.md),
  [`transforms-v3-addendum-corrections.md`](../v3-proposals/transforms-v3-addendum-corrections.md),
  [`atomic-batch-insert/DISCUSSION.md`](../atomic-batch-insert/DISCUSSION.md),
  `AGENTS.md`, `CHANGELOG.md`; probes in [`probes/`](probes/).
