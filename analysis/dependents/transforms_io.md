# transforms_io: how it uses transforms and what it teaches

Analysis date: 2026-11-06. Baseline: transforms master `f9770d2` (released
v2.3.0 plus docs-only commits).
Subject: crates.io `transforms_io`, all 12 published versions (0.1.0 to
0.8.0), and the upstream repo
[codeberg.org/stelzo/transforms_io](https://codeberg.org/stelzo/transforms_io)
at `2e6eb80` (tag v0.8.0, HEAD of `main`). Also its one crates.io
dependent, `pelorus_syncio` 0.0.9.

Citation shorthand:

- `tio@0.8.0:src/x.rs:N` is a line in the crates.io tarball of that
  version. `tio@<commit>` is the upstream git repo.
- `pel@0.0.9:` is `pelorus_syncio` 0.0.9, `rt@0.26.0:` is
  `roslibrust_transforms` 0.26.0, `wtf@0.1.0-alpha.3:` is `whirl_tf`.
- `tf2@183399c:` is ros2/geometry2 `rolling`, fetched 2026-11-06.
  `nav2@35d9203:` is ros-navigation/navigation2.
- `P1`–`P10` are labels in
  [`probes/transforms_io.rs`](probes/transforms_io.rs). `V1`–`V10` and
  `N1`–`N4` are labels in
  [`probes/transforms_io_verify.rs`](probes/transforms_io_verify.rs).

## Method

- Read every published tarball (0.1.0 to 0.8.0) and diffed consecutive
  versions. Read the upstream history (34 commits, 2026-04-14 to
  2026-10-27, one branch, one author). The 1.x to 2.0 migration was read
  as the commit diff `8fb3cf8`.
- Read `pelorus_syncio` 0.0.9 for how the one downstream crate calls
  transforms_io. Shallow-cloned minot `main` (`b394045`): it does not
  reference transforms.
- Read `roslibrust_transforms` 0.26.0 and `whirl_tf` 0.1.0-alpha.3 only
  for the "Also in" column. Their full studies are
  [roslibrust_transforms.md](roslibrust_transforms.md) and the corpus
  notes behind [README.md](README.md).
- Read tf2 and tf2_ros behaviour from source (`tf2@183399c`), not by
  running them.
- Two probe programs copy transforms_io 0.8.0's conversion, insert and
  lookup logic, with ROS message types replaced by plain structs, and run
  it against this checkout of transforms (`f9770d2`). Each probe header
  cites the copied lines.
- An independent evidence skeptic re-checked every research item, wrote the
  second probe, and marked each item confirmed, partial or refuted.
  Nothing was refuted. Partial items appear here in corrected form, and
  items the skeptic found are marked "found in verification". A scope
  judge ruled the placement of every recommendation against AGENTS.md.
- **Not done:**
  - transforms_io and pelorus were not built or run. All behaviour comes
    from copied logic in the probes.
  - H11 (static broadcaster) depends on DDS or zenoh late-joiner delivery
    and was not probed. The mt-pubsub static publisher depth comes from
    `qos.transient_local()` and was not checked.
  - No consumer of `TfListener::static_transforms()` was found. Checked:
    pelorus 0.0.9, minot `main`, and a GitHub code search. The
    stelzo/pelorus Codeberg API returned an empty response, so that repo
    was not read.
  - Whether minot replays loop or rewrite stamps was not checked, so H7
    through minot replay is plausible, not shown. H7 is shown for a
    generic simulation reset.
  - How often real ROS data carries 4-decimal quaternions (H8) was not
    measured.
  - The unpublished in-repo `transforms_io_hiroz` subcrate (merged in
    `95ef7dc`) was not read file by file.
  - Nobody at transforms_io was contacted. The repo has no issues or
    pull requests.
  - pelorus download counts were not fetched.

## Verdict

- **transforms_io is a thin bridge, and its users can reach only one
  lookup.** Each listener holds a private `Registry<Timestamp>` built with
  `with_max_age` (10 s by default), fed by `/tf` and `/tf_static`. The
  only forwarded method is `lookup_transform`. A transforms improvement
  reaches its users only through a transforms_io release.
- **Three of four backends name their lookup parameters backwards.** The
  r2r, ros2-client and hiroz `lookup_transform(source_frame, target_frame,
  time)` pass the arguments to `get_transform(target, source)` in that
  order (P8, V8). A caller who follows the declared names gets the inverse
  transform. The result's header names the inverted frames, so only a
  caller who ignores the header is misled silently. Callers who write tf2
  order, as the README and pelorus do, get the right answer. This is an
  adapter bug, not a transforms bug. mt-pubsub fixed it in `8388a61`.
- **"Latest" is emulated with `Timestamp::now()`, in all four backends.**
  That fails loudly when a dynamic hop ends before `now()` (P1). When
  every dynamic hop is post-dated or clock-skewed past `now()`, it succeeds
  silently at the wall-clock instant, stamped (0,0) (V1, N4; found in
  verification). `latest_common_time` shipped in
  2.1.0, and seven transforms_io releases since have not used it.
- **Three silent wrong answers pass through the bridge where transforms
  could help.** An empty `frame_id` joins unrelated trees (P4). A frame
  that starts on `/tf_static` and later moves to `/tf` is frozen at the
  static pose (P3). A backward clock jump interleaves two epochs (P6, N3).
  The first gets a 2.x code change that is already proposed. The other two
  get docs.
- **The 1.4 to 2.0 migration was one mechanical commit, and it fixed four
  silent bugs.** It also added a lossy clamp of negative seconds. 0.8.0 is
  the first dependent to match a `RegistryError` variant. Earlier analysis
  statements that no dependent does are stale.
- **Re-exports raise the cost of a transforms major.** transforms types are
  part of transforms_io's public API and are re-exported again by pelorus.

## Status

As of 2026-11-06 (crates.io API; upstream git):

| Item | Value |
|---|---|
| Versions | 12, 0.1.0 (2026-04-13) to 0.8.0 (2026-10-27), none yanked |
| transforms requirement | `^2.2` at 0.8.0, locked 2.2.0. A fresh build resolves 2.3.0 |
| Downloads | 2,559 all time, 1,990 recent (90-day window). 0.8.0: 285 |
| Backends | `mt-pubsub` (Minot, default since 0.2.0), `r2r`, `ros2-client`, `hiroz` (since 0.6.0) |
| MSRV | 1.89.0 (`tio@0.8.0:Cargo.toml.orig:25`) |
| Activity | 34 commits by `stelzo`. Last commit `2e6eb80`, 2026-10-27. No issues, no PRs, no unreleased changes |
| Downstream | `pelorus_syncio` 0.0.9, `transforms_io = "0.8.0"` with default features off (`pel@0.0.9:Cargo.toml:286-288`), all four backends behind its own features (`Cargo.toml.orig:67-88`). It is "IO layer for pelorus_sync" (lidar processing), homepage `pelorus.steado.tech` |

Per-version requirements (each tarball's `Cargo.toml` and `Cargo.lock`):

| transforms_io | Date | Requires | Locked | Upstream |
|---|---|---|---|---|
| 0.1.0 | 2026-04-13 | `1.4` | 1.4.1 | `2b85c56` |
| 0.2.0 | 2026-06-27 | `1.4` | 1.4.1 | `3357fdb` |
| 0.3.0 | 2026-08-22 | `1.4` | 1.4.1 | `59e5a53` |
| 0.4.0 | 2026-08-23 | `1.4` | 1.4.1 | `3494470` |
| 0.5.0 | 2026-08-27 | `2.0` | 2.0.0 | `1bd5b23` (migration `8fb3cf8`) |
| 0.5.1 | 2026-09-05 | `2.0` | 2.1.0 | `d07a2df` |
| 0.5.2 | 2026-09-12 | `2.0` | 2.1.1 | `f26b76c` |
| 0.5.3 | 2026-09-13 | `2.0` | 2.1.1 | `6ed1e7e` |
| 0.5.4 | 2026-09-16 | `2.0` | 2.1.1 | `93b4ee9` |
| 0.6.0 | 2026-09-29 | `2.0` | 2.1.1 | `123022f` (hiroz merged in `95ef7dc`) |
| 0.7.0 | 2026-10-04 | `2.0` | 2.2.0 | `5bc6cc7` |
| 0.8.0 | 2026-10-27 | `2.2` | 2.2.0 | `2e6eb80` |

- The 0.8.0 bump to `^2.2` buys no 2.2 API. It only guarantees 2.2's
  behaviour, which 0.7.0 already locked: the reworded
  `ReparentingNotSupported` message and the 2.2 lookup-error fixes
  (CHANGELOG.md:23-98). The variant's shape is unchanged since 2.0.0
  (`git show v2.0.0:src/core/registry/error.rs`, lines 69-73).
- transforms 2.0.0 shipped 2026-08-22. transforms_io migrated four days
  later (`8fb3cf8`, 2026-08-26).
- `latest_common_time` (2.1.0, 2026-08-27) is unused in 0.5.1 to 0.8.0.
  `add_transforms` (2.3.0) shipped the day after 0.8.0 and is unused.

## How transforms_io uses the crate

All four backends do the same things. Lines are for 0.8.0.

- **Types.** `Registry`, `geometry::{Quaternion, Transform, Vector3}`,
  `time::{Stamp, Timestamp}`, and `errors::RegistryError` (mt-pubsub only,
  `mt_pubsub_backend.rs:12`). `Timestamp::as_nanos` appears once
  (`hiroz_backend.rs:287`).
- **Time type.** `Registry<Timestamp>` everywhere. No `SystemTime`, no
  custom `TimePoint`.
- **ROS time to `Timestamp`.**
  `(time.sec.max(0) as u64) * 1_000_000_000 + (time.nanosec as u64)`
  (`mt_pubsub_backend.rs:426`, `r2r_backend.rs:347`,
  `ros2_client_backend.rs:292`, `hiroz_backend.rs:281`, `pub` in hiroz).
  Negative seconds clamp to 0 and keep their nanoseconds (P5).
  `nanosec >= 1e9` carries into seconds.
- **Construction.** `Transform::new(parent, child, Vector3::new(..),
  Quaternion { w, x, y, z }, stamp)` with a struct-literal quaternion and
  no normalization (`mt_pubsub_backend.rs:370-391` and siblings).
  `Transform::static_between` is never used.
- **Registry.** `Registry::with_max_age(buffer_duration)`, default 10 s
  (`lib.rs:46-48`, duplicated per backend), commented "matches the default
  used by ROS2 tf2". tf2 also prunes 10 s behind each frame's newest
  sample (`tf2@183399c:tf2/src/cache.cpp:365-372`).
- **Routing.** The topic decides the kind: `/tf_static` becomes
  `Stamp::Static`, `/tf` becomes `Stamp::At(header stamp)`. Both feed one
  registry (`mt_pubsub_backend.rs:188-209,221-268`; `r2r_backend.rs:143-177`;
  `ros2_client_backend.rs:105-167`; `hiroz_backend.rs:99-123`).
- **Insertion and locking.** `Arc<RwLock<Registry<Timestamp>>>`. One write
  guard per `TFMessage`, then `add_transform` per element. A rejected
  element is logged and skipped, and the rest of the message is inserted.
  The private helper is named `add_transforms`, the same name as the 2.3
  API with the opposite atomicity (`mt_pubsub_backend.rs:310-367`). A
  poisoned lock drops the whole message with one log line. Readers take
  one read guard per lookup.
- **Lookup.** One method, `lookup_transform(a, b, ros_time)`, which calls
  `reg.get_transform(a, b, ts)`. Time (0,0) is replaced by
  `Timestamp::now()`. The result becomes a ROS `TransformStamped` whose
  header stamp is the *requested* time, so (0,0) stays (0,0)
  (`mt_pubsub_backend.rs:285-307`, `r2r_backend.rs:97-119`,
  `ros2_client_backend.rs:171-193`, `hiroz_backend.rs:127-149`).
- **Errors.** `TfError` is a flat enum of `String`-carrying variants
  (`error.rs:4-25`). Lookup failures become
  `TfError::Lookup(e.to_string())`; construction failures become
  `InvalidData(e.to_string())`. Insert failures are only logged.
  mt-pubsub matches `RegistryError::ReparentingNotSupported { .. }`, warns
  once per (parent, child) and traces after that
  (`mt_pubsub_backend.rs:337-354`, `19fbbe0`). The other three backends
  log every rejection at error level (`r2r_backend.rs:277-282`,
  `ros2_client_backend.rs:220-225`, `hiroz_backend.rs:197-202`).
- **Introspection.** mt-pubsub alone keeps a shadow
  `BTreeMap<(parent, child), TransformStamped>` of every `/tf_static`
  message "as published", including ones the registry refused. It is
  exposed as `TfListener::static_transforms()`
  (`mt_pubsub_backend.rs:124-147,271-279`, `19c2cbb`).
- **Public surface.** `lib.rs:41-44` re-exports `Registry`, `Quaternion`,
  `Transform`, `Vector3`, `Stamp` and `Timestamp`. transforms types also
  appear in transforms_io's own `pub fn` signatures:
  `hiroz_to_transform` and `hiroz_to_static_transform` return `Transform`,
  `transform_to_hiroz` takes `&Transform`, the hiroz time functions use
  `Timestamp`, and `convert::isometry_to_transform` takes a `Stamp`
  (`hiroz_backend.rs:219,229,254,280,286`; `convert.rs:18-37`; found in
  verification). pelorus re-exports `Quaternion`, `Registry`, `Timestamp`,
  `Transform` and `Vector3` again, but not `Stamp`
  (`pel@0.0.9:src/tf.rs:6`, `tf_lh.rs:6`, `tf_r2r.rs:6`, `tf_hiroz.rs:4`).

**Never touched** (grep over all 12 versions): `get_transform_for`,
`get_transform_at`, `latest_common_time`, `Registry::add_transforms`,
`reparent_frame`, `remove_frame`, `remove_transforms_before`,
`Registry::new` (2.x), `Transform::static_between`, `validate`,
`inverse`, `interpolate`, `*`, `Point`, `Transformable`, `Localized`,
`Quaternion::normalize`/`slerp`, `Timestamp::try_now`, `SystemTime`,
`serde`. The registry field is private with no accessor
(`mt_pubsub_backend.rs:125-131`, `r2r_backend.rs:80-82`,
`ros2_client_backend.rs:88-90`, `hiroz_backend.rs:82-86`), so users
cannot reach any of these either.

**pelorus** calls only `lookup_transform`, positionally in tf2 order, at
the point cloud's header stamp, never a literal (0,0)
(`pel@0.0.9:src/lib.rs:720-731,759-763,907-911`). It reads only
`.transform` from the result, converts it to `f32` nalgebra by hand, and
inverts it (`lib.rs:764-781,912-933`). It never uses `Transformable`.

## History

**1.x era (0.1.0 to 0.4.0, transforms 1.4.1).**

- Transforms were struct literals and inserts were unchecked:
  `reg.add_transform(transform);` returned nothing
  (`tio@0.4.0:src/mt_pubsub_backend.rs:161,186,226-241`).
- Time was `(time.sec as u128) * 1_000_000_000 + nanosec`
  (`tio@0.4.0:src/mt_pubsub_backend.rs:275`). A negative `i32` second
  sign-extends to about 2^128, and the multiply overflows: a panic in debug
  builds, a garbage stamp in release builds.
- `/tf_static` was inserted at its header stamp. A static stamped (0,0)
  was a true 1.x static. A static with a non-zero stamp became a
  single-instant dynamic sample, and 1.4.1 lookups through it failed
  except at that exact stamp (`v1.4.1:src/core/buffer/mod.rs:314-331`).
- Any `/tf` sample stamped (0,0) flipped its buffer to static
  (`v1.4.1:src/core/buffer/mod.rs:243`). It then served that pose at
  every instant until the next non-zero insert. Silent.
- The time-zero substitution and the `(source_frame, target_frame)`
  parameter names are present from 0.1.0 in every backend
  (`tio@0.1.0:src/mt_pubsub_backend.rs:186-208`, `r2r_backend.rs:85-103`).
  The names map one to one onto 1.x's `get_transform(from, to)`
  (`v1.4.1:src/core/registry/mod.rs:344-348`).

**1.4 to 2.0 (`8fb3cf8`, released as 0.5.0).** One mechanical commit.

- Struct literals became `Transform::new(..)?`. `Registry::new(d)` became
  `Registry::with_max_age(d)`, which avoids the trap MIGRATION.md warns
  about. `/tf_static` became `Stamp::Static`. Inserts are checked and
  logged. `as u128` became `sec.max(0) as u64`.
- **Fixed:** statics answer every instant; the zero-stamp static flip is
  gone (a (0,0) `/tf` sample is now an ordinary instant, N2); the
  `u128` overflow is gone; denormalized and non-finite inputs are rejected
  instead of stored; rejected inserts are visible.
- **Introduced:** the negative-seconds clamp aliases instants (H6), and
  each rejected insert logs one error line per message.
- **Not fixed:** time zero to `now()` (H1) and the backwards parameter
  names (H2). transforms 2.0 renamed its own `from`/`to` to
  `target`/`source` because the old names "read backwards and silently
  produced the inverse for plain-English callers" (CHANGELOG.md:407-411).
  The rename kept positional behaviour, so transforms_io compiled and
  behaved the same, and its names stayed backwards.

**2.0 to 2.2 (0.5.1 to 0.7.0).** No change in how transforms is used.
The lockfile moved to 2.1.0, 2.1.1 and 2.2.0 under a `2.0` requirement.
Other changes were transport-only: hiroz merged in (`95ef7dc`), static
listener QoS depth 1 to 100 (`93b4ee9`).

**0.8.0 (transforms 2.2).**

- `19fbbe0` "conflicts": mt-pubsub matches `ReparentingNotSupported` and
  dedupes it. Its comment reads "The registry keeps the first one on
  purpose, and lookups through it still work, so this is worth one line,
  not one per message" (`mt_pubsub_backend.rs:337-341`).
- `19c2cbb` "fix static tf": the shadow static log.
- `8388a61` "spawn": an async constructor, a `TfError::Subscribe`
  variant, and the mt-pubsub `lookup_transform` renamed to
  `(target_frame, source_frame)` with the call swapped to match, so
  positional behaviour is unchanged. It adds a doc line tying the order to
  tf2. The other three backends were not touched.
- `2e6eb80`: requirement `^2.2`.

**Earlier analysis claims, re-checked on 0.8.0.**

| Claim | Status | Evidence |
|---|---|---|
| "transforms_io logs at error level and skips" ([reparent REPORT.md:66](../reparent-frame-value/REPORT.md)) | Partly stale. True for r2r, ros2-client, hiroz. mt-pubsub warns once per edge, then traces | `mt_pubsub_backend.rs:342-354` |
| "None names the rejection variant" (same row); "None matches a `RegistryError` variant" ([v3-warrant REPORT.md:151-152](../v3-warrant/REPORT.md)); the same in the 2026-10-30 dimos report | Stale. transforms_io 0.8.0 matches one variant. Lookup errors are still strings. [dimos.md](dimos.md) already carries this correction ("How dimos uses the crate") | `mt_pubsub_backend.rs:12,342` |
| "Both bridges format errors through `Display` only" ([reparent REPORT.md:118-119](../reparent-frame-value/REPORT.md)) | Stale for mt-pubsub. It branches on the variant, then prints `Display` | as above |
| /tf and /tf_static feed one registry, so the static-stale trap is reachable (reparent REPORT.md:66) | Still true, in two forms (H3, H4) | see Hazards |
| "transforms_io keeps its registry private with no mutator" (reparent REPORT.md:136) | Still true | private fields, all backends |
| "0.6.0 still carries a `Timestamp::now()` workaround" ([v3-warrant REPORT.md:60-61](../v3-warrant/REPORT.md)) | Still true in 0.8.0, and worse than described (H1) | `mt_pubsub_backend.rs:295-300` and siblings |
| "Both ROS-adapter dependents hand-write roughly 30 lines of nalgebra conversion" (v3-warrant REPORT.md:123-125) | Partly true. transforms_io exports transforms-to-nalgebra converters but calls them nowhere. The second nalgebra user is whirl_tf, not roslibrust | `convert.rs:18-50`; `rt@0.26.0` has no nalgebra dependency |

## Hand-rolled around the crate

| Feature | Where | Correct? | Exact on public API? | Also in |
|---|---|---|---|---|
| ROS time to `Timestamp` | 4 backends, e.g. `mt_pubsub_backend.rs:423-428` | Lossy: `sec < 0` clamps and aliases (P5) | Yes; a checked version is exact | roslibrust clamps to `zero()` and drops nsec (`rt@0.26.0:src/lib.rs:90-98`); whirl_tf rejects `sec < 0` and `nsec >= 1e9` (`wtf:src/buffer.rs:395-416`); dimos uses `SystemTime` |
| "Latest" as time zero to `now()` | 4 backends, e.g. `mt_pubsub_backend.rs:295-300` | No (H1) | Yes: `latest_common_time` then `get_transform` under one guard | roslibrust examples look up at `Timestamp::now()` (`rt@0.26.0:examples/ros1.rs:58`, `ros2_rosbridge.rs:56`) and its doc example maps `Static` to `now()` (`src/lib.rs:609-613`); dimos maps `Static` to `UNIX_EPOCH` ([dimos.md](dimos.md)) |
| /tf versus /tf_static routing | 4 backends | Yes, as tf2 | Yes | roslibrust (`rt@0.26.0:src/lib.rs:252-280`) |
| "First publisher wins", deduped warning | `mt_pubsub_backend.rs:337-354` | Matches the registry invariant. Hides the cause on a dynamic hand-over (N1) | Yes | roslibrust warns per message (`rt@0.26.0:src/lib.rs:328-331`) |
| Shadow log of statics "as published" | `mt_pubsub_backend.rs:124-147,271-279` | Correct for its purpose. A replay picks a different parent than the live registry (P10) | Yes | none seen |
| Message to `Transform` | 4 backends | Yes. Validation is left to `Transform::new` | Yes | roslibrust the same (`rt@0.26.0:src/lib.rs:687-713`); dimos repeats the norm check |
| nalgebra glue | `convert.rs:18-50` (unused); `isometry_from_transform` per backend | `isometry_from_transform` normalizes raw ROS quaternions unchecked; all-zero gives NaN. It bypasses transforms | Yes | whirl_tf (nalgebra API); dimos converts ROS to nalgebra only |
| Errors to `TfError(String)` | `error.rs:4-25`; `mt_pubsub_backend.rs:304` | Loses `frame`, `requested`, `covered` | Yes | roslibrust `LookupError(String)` (`rt@0.26.0:src/lib.rs:402,572,626`); dimos `to_string` then `io::Error`. whirl_tf keeps the typed error as `source` (`wtf:src/buffer.rs:147-153,180-185`) |
| Wait for a transform | pelorus: 10 ms poll, 5 s deadline, any `Err`, managed replay only (`pel@0.0.9:src/lib.rs:317,717-731`) | Works. Retries permanent failures, drops the reason | Yes (AGENTS.md Scope: outside) | roslibrust `wait_for_transform` (`rt@0.26.0:src/lib.rs:451-510`); dimos `Notify` |
| Frame names in insert-error logs | `mt_pubsub_backend.rs:343-363` and siblings | Yes | Yes | roslibrust clones names "so the warning below can still name them" (`rt@0.26.0:src/lib.rs:324-331`); whirl_tf wraps with `parent`/`child` (`wtf:src/buffer.rs:147-153`) |
| Frame-name checks | **absent** | Empty names pass (P4) | Yes | dimos validates; whirl_tf `normalize_frame` strips `/`, rejects empty (`wtf:src/buffer.rs:377-391`); roslibrust absent |
| Quaternion tolerance for tf2 data | **absent** | Data tf2 accepts is dropped (P7) | Yes (`Quaternion::normalize`) | roslibrust absent (`rt@0.26.0:src/lib.rs:705-710`) |
| Clock-jump handling | **absent**; users cannot add it (private registry) | — | Partly (dimos.md clock-jump recipe) | dimos lost it in migration; whirl_tf `clear_dynamic` via `remove_frame` (`wtf:src/buffer.rs:200-204`); roslibrust absent |
| Static broadcaster | 3 backends: one transform per `TFMessage`, `KeepLast(1)` | No (H11) | Not a transforms matter | roslibrust the same ([roslibrust_transforms.md](roslibrust_transforms.md)) |

Notes:

- **"Latest" recurs in every integration, with three conventions.**
  roslibrust's now()-on-`Static` and dimos's `UNIX_EPOCH`-on-`Static` are
  correct, because any instant serves an all-static chain. Only the
  transforms_io pattern, looking up dynamic chains at `now()`, is broken.
  roslibrust's shipped examples repeat it (found in verification).
- **Every ROS adapter stringifies errors, and every wait loop is
  error-blind.** whirl_tf is the one integration that keeps the typed
  error. The crate already provides a flat typed `RegistryError<T>`; the
  adapters discard it.
- **Frame names around frame-less insert errors recur three times.**
  `StaticDynamicConflict`, `CycleDetected` and `SelfReferentialFrame`
  carry no frames (`src/core/registry/error.rs:47-96`). The workaround is
  exact for per-element inserts, because the caller holds the transform.
- **Ingest checks that tf2 performs are mostly absent in both ROS
  adapters.** tf2 strips a leading `/`, rejects empty ids and accepts
  `|‖q‖² − 1| < 1e-2` (`tf2@183399c:tf2/src/buffer_core.cpp:66,212-236,
  250-262`). tf2_ros clears the buffer on a backward ROS-clock jump
  (`tf2_ros/src/buffer.cpp:111-122`). The other two integrations
  (dimos, whirl_tf) hand-roll the name checks.

## Hazards

**H1. Time zero is answered at wall-clock `now()`.**

- What goes wrong: a tf2-style "latest" lookup asks transforms for the
  pose at the listener's wall clock. transforms does not extrapolate.
  - If any dynamic hop ends before `now()`, the lookup fails with
    `NotFoundAt` (P1, V1). This holds in real time with a sample 10 ms old
    and in sim time (`covers [100, 101]`). Static-only chains succeed.
  - If every dynamic hop covers `now()`, the lookup succeeds silently at
    `now()`, not at the newest sample. Two common causes: nav2 AMCL stamps
    `map→odom` at scan time plus `transform_tolerance` (default 1.0 s,
    `nav2@35d9203:nav2_amcl/src/amcl_node.cpp:607-611,990`), and a
    publisher clock ahead of the listener's. N4: `map<-odom` returns
    x ≈ 79.2 while the newest sample is x = 99. V1: a +50 ms skew returns
    x ≈ 5.0 while the newest is 10. In N4 the full chain
    `map<-base_link` still fails, because its `odom→base_link` hop ends
    before `now()`. Found in verification; it corrects the research claim
    that the failure is always loud.
  - On success the result header carries (0,0), so the caller cannot tell
    which instant was served.
- Silent or loud: loud when a dynamic hop ends before `now()`; silent
  when every dynamic hop is post-dated or skewed past it.
- Fix that works: `latest_common_time` then `get_transform` under one read
  guard returns x = 3.0 in P1.
- Cause: the dependent. transforms documents the right path (README.md:100-
  103 and the tf2 table row at README.md:138).

**H2. Three backends name their lookup parameters backwards.**

- What goes wrong: r2r, ros2-client and hiroz declare
  `lookup_transform(source_frame, target_frame, time)` and call
  `get_transform(source_frame, target_frame, ts)`
  (`tio@0.8.0:src/r2r_backend.rs:97-119`,
  `ros2_client_backend.rs:171-193`, `hiroz_backend.rs:127-149`).
  `get_transform`'s first parameter is the target. So the parameter named
  `source_frame` is used as the target, and the one named `target_frame`
  as the source.
- Who is hit: Rust has no named arguments. "By name" here means a caller
  who binds arguments to the declared names, as shown on docs.rs and in IDE
  hints, instead of writing tf2 order from habit. P8 and V8: asking for
  `source_frame = "laser"`, `target_frame = "base_link"` returns x = −0.3
  with frames `laser<-base_link`. The intended answer, which mt-pubsub
  gives for the same request, is x = 0.3, `base_link<-laser`.
- Who is not hit: a caller who writes tf2 order positionally. The
  README examples do (`lookup_transform("base_link", "sensor", time)`,
  `tio@0.8.0:README.md:47,70,79,91,108`), and so does pelorus
  (`pel@0.0.9:src/lib.rs:723,759-763,907-911`).
- Silent or loud: the inverted result is `Ok`. The returned
  `TransformStamped` header names the inverted frames (`frame_id =
  "laser"`, `child_frame_id = "base_link"`; `r2r_backend.rs:317-343`), so
  a caller who checks the header can notice. A caller who reads only
  `.transform`, as pelorus does with its results, cannot.
- Scope of the evidence: the names have been reversed in r2r and
  ros2-client since 0.1.0, in hiroz since 0.6.0, and in mt-pubsub until
  0.7.0. Positional behaviour is the same in all four backends and all 12
  releases. Switching backends never changes a result; only the declared
  names differ. The three backends' doc comments say only "Look up a
  transform at a specific time." No caller who follows the names was
  found, and no wrong result in a deployment is known.
- Cause: the dependent. `get_transform(target, source)` is named and
  documented correctly. The origin is transforms 1.x's backwards
  `from`/`to` names, which transforms_io's names copy.

**H3. Static-stale trap, parent form.**

- What goes wrong: a static mount re-published under a new parent is
  rejected with `ReparentingNotSupported`. Lookups through the old parent
  keep returning the old pose at any time. The new parent is unknown
  (P2, V2).
- Silent or loud: the old-parent answer is silently stale; the new-parent
  lookup fails loudly. mt-pubsub logs one WARN, then TRACE. The other
  three backends log an ERROR per message.
- A same-parent static re-publish replaces the pose retroactively (P2b:
  0.3 becomes 0.35 at the same t = 10 s). That is documented and matches
  tf2.
- Cause: transforms by design (first insert pins the parent). The
  rejection is loud; the bridge chooses to log and continue.

**H4. Kind-switch trap: `/tf_static` first, then `/tf` on the same edge.**

- What goes wrong: an edge first arrives on `/tf_static` (for example a
  default `map→odom`) and is later animated on `/tf`. Every `/tf` insert
  fails with `StaticDynamicConflict`, and lookups keep returning the
  static pose at every instant. P3: `map<-odom` at 102 s returns 0.0 while
  the publisher said 3.0, with one ERROR line per `/tf` message in every
  backend (no dedupe for this variant).
- In the reverse order the static is rejected and lookups past the stream
  fail loudly (`covers [100, 101]`).
- tf2 does the opposite: a kind switch re-allocates the frame's cache, so
  the newest kind wins (`tf2@183399c:tf2/src/buffer_core.cpp:276-286`,
  "Overwrite TimeCacheInterface type with a current input").
- Silent or loud: silent wrong answer at every instant, plus log noise.
- Cause: both. Static-xor-dynamic is an AGENTS.md invariant. The tf2
  difference is undocumented in transforms. The bridge logs and continues.
  roslibrust feeds both topics into one registry too
  (`rt@0.26.0:src/lib.rs:230,252-280`).

**H5. An empty `frame_id` makes a shared root.**

- What goes wrong: `""` is the default of an unset ROS 2 string field.
  P4, V4: statics `("", camera)` x = 1 and `("", lidar)` x = 3 are
  accepted with no log line, and `camera<-lidar` returns 2.0. An empty
  child on `/tf` is accepted too.
- tf2 rejects both (`TF_NO_FRAME_ID`, `TF_NO_CHILD_FRAME_ID`,
  `tf2@183399c:tf2/src/buffer_core.cpp:224-236`).
- Silent or loud: silent wrong answer between unrelated frames.
- Cause: transforms accepts `""`, and the bridge does not check. The same
  holds for roslibrust; dimos and whirl_tf check by hand.

**H6. Negative ROS seconds alias onto [0, 1) s.**

- P5, V5: `(-1, 0.5e9)`, `(0, 0.5e9)` and `(-7, 0.5e9)` all map to
  500000000 ns. After inserting x = 1 at the first and x = 2 at the second,
  a lookup at `(-1, 0.5e9)` returns 2.0 with no log.
- Silent, and rare: it needs pre-epoch or garbage stamps.
- Cause: the dependent (`sec.max(0)`). The reverse function
  `timestamp_to_hiroz_time` casts seconds `as i32` and wraps silently
  after 2038-01-19 (`hiroz_backend.rs:286-292`). That is a ROS wire
  limit, but the wrap is unchecked.

**H7. A backward clock jump under the 10 s `max_age`.**

- What goes wrong (P6, V6, N3):
  - After 30 s of data, a reset to 0–5 s produces inserts that all return
    `Ok` and are evicted on arrival. Lookups at the new times fail with
    `covers [20, 30]`; `latest_common_time` stays at 30 s.
  - That loss is not permanent, which corrects the research claim. Once
    the new stamps reach the old newest minus `max_age` (20 s), new
    samples are kept and interleave with the old epoch. N3: at 25.05 s the
    lookup returns 7.55, a blend of the old epoch's 25.1 and the new −10.
    The old epoch drains after up to `max_age` more seconds. Found in
    verification.
  - A jump smaller than `max_age` interleaves at once: P6 returns −2.5 at
    2.52 s, where the old epoch says 2.52 and the new says −10.
- Silent or loud: silent. No insert reports anything.
- Recovery: `remove_transforms_before(Timestamp::from_nanos(u64::MAX))`
  restores correct answers (V6: −10 at 2.5 s). transforms_io users cannot
  call it, because the registry is private.
- tf2_ros clears its buffer on any backward ROS-clock jump
  (`tf2@183399c:tf2_ros/src/buffer.cpp:111-122`). tf2's `TF_OLD_DATA`
  (`tf2/src/cache.cpp:290-293`) covers old stamps without a clock jump.
- Cause: both. transforms' rule is documented (`with_max_age`,
  `src/core/registry/mod.rs:122-130`), but its consequences under a jump
  are not. Jump detection belongs to the clock owner, the bridge.

**H8. ROS quaternions that tf2 accepts are dropped.**

- P7, V7: `(0.7071, 0.7071)` has norm − 1 = −9.59e-6. tf2 accepts it;
  transforms rejects it (`UNIT_NORM_TOLERANCE` = 1e-6,
  `src/geometry/transform/mod.rs:30`). Three decimals (−1.51e-4) are also
  rejected. 8-decimal and `f32`-rounded values pass.
- `static_transform_publisher` passes typed components through
  unnormalized
  (`tf2@183399c:tf2_ros/src/static_transform_broadcaster_program.cpp:132-154`).
- Silent or loud: loud. The bridge logs and skips; later lookups fail.
- Prevalence was not measured.
- Cause: transforms is strict by design; the bridge does not normalize.

**H9. Dropouts inside `max_age` are interpolated.**

- P9, V9: samples at 100 s and 108 s, a lookup at 104 s returns 4.0.
- Silent. transforms_io adds reachability, but no comment, TODO or issue
  shows a need.
- Cause: transforms has no gap limit
  ([dimos.md](dimos.md), "Design doc: maximum interpolation gap").

**H10. The static shadow log replays differently from the live registry.**

- P10, V10: `laser` arrives under `z_mount`, then `a_mount`. The live
  registry keeps `z_mount` (`base<-laser` = 1.0). The log holds 4 entries
  for 3 children. Replaying it in iteration order keeps `a_mount`
  (`base<-laser` = 2.0, with a WARN).
- Matters only if someone replays `static_transforms()`. No consumer was
  found.
- Cause: the dependent.

**H11. Late joiners see only the last static the bridge published.**
Found in verification.

- `TfBroadcaster::send_static_transform` publishes a `TFMessage` holding
  one transform on a TransientLocal topic with `KeepLast(1)`
  (`r2r_backend.rs:35-44,65-76`; `ros2_client_backend.rs:37-42,74-84`;
  `hiroz_backend.rs:71-78,161-168`). tf2_ros's broadcaster accumulates all
  statics by child and republishes the full set
  (`tf2@183399c:tf2_ros/src/static_transform_broadcaster.cpp:51-71`).
- A node that publishes two mounts through transforms_io leaves only the
  last for listeners that join later. Their lookups fail with
  `UnknownFrame`.
- Loud. Not probed. Transport-side; no transforms involvement.
- Cause: the dependent. roslibrust has the same pattern.

**Checked and not reachable through transforms_io:** re-inserting a
`get_transform_at` result (never called), cross-time application (not
used), `Transformable` misuse (not used), and the `Timestamp::now()`
panic (needs a clock before 1970 or after 2554).

## Friction

- **The registry is private, and only `lookup_transform` is forwarded.**
  Users cannot reach `latest_common_time`, the clock-jump recovery,
  `remove_frame` or `reparent_frame`. This is transforms_io's design
  choice, and it decides what any transforms change can reach.
- **The shadow static log exists because the registry refuses a second
  parent and offers no read-out.** Its doc comment says so: "Unlike the
  registry it keeps one the registry refused"
  (`mt_pubsub_backend.rs:127-130`). It also keeps entries that fail
  `Transform::new`. It is a message-log need, not a registry-state need.
- **Frame names are formatted by hand in every insert log** because three
  insert errors are frame-less. Cheap and exact, but repeated in three
  integrations.
- **The 2.3 name `add_transforms` collides** with the bridge's private
  per-element helper of the opposite atomicity. A naive switch would let
  one conflicting edge (H3, H4) drop a whole `TFMessage`, such as
  robot_state_publisher's joint batch. transforms already says
  `add_transform` is the way to keep the valid transforms of a batch
  (README.md:89-91, CHANGELOG.md:19-21).
- **The 0.8.0 dedupe assumes "lookups through it still work".** That holds
  for statics. For a dynamic hand-over, where the first parent's publisher
  stops and a second continues under another parent, both lookups fail
  and the cause is logged once, then at trace level only (N1: 1 WARN,
  49 TRACE; `map_a<-odom` fails `covers [100, 100.9]`, `map_b<-odom` is
  unknown). Found in verification. Low severity.
- **No E0283 friction.** The `Arc<RwLock<Registry<Timestamp>>>`
  annotation fixes `T`.
- **No TODO, FIXME or `unwrap` touches the transforms API.** All
  `expect`/`unwrap` calls are transport setup
  (`mt_pubsub_backend.rs:232,253`; `r2r_backend.rs:33,44,148,
  164,223,237`; `ros2_client_backend.rs:29-52,110-152`;
  `hiroz_backend.rs:156,165,172`).

## Recommendations

Placement follows the scope judge's ruling in every case. Items 3 and 4
stay docs, but their text moves from the README to rustdoc, as the index
resolves. Under the maintainer stance of 2026-11-03, no hazard gets an
additive parallel 2.x API; a fix that needs a break goes to the v3
ledger, with docs or outreach for 2.x.

### transforms 2.x code

1. **Reject empty frame names.** Already proposed in
   [dimos.md](dimos.md) ("Reject empty frame names" under "transforms 2.x
   code"; shape under "2.x: reject empty frame names":
   `TransformError::EmptyFrameName`, enforced by `Transform::validate`;
   `UnknownFrame("")` on lookups). Not on master (grep). DESIGN
   revision 4 adopts it as Part D, at the insert boundary instead.
   - Evidence: H5 (P4, V4), reachable unfiltered through all four
     transforms_io backends.
   - Corroboration: roslibrust passes `""` through too. dimos and whirl_tf
     check by hand (`wtf:src/buffer.rs:377-391`). tf2 rejects it. Two
     integrations are exposed and two hand-roll the check: the strongest
     recurrence in this study.
   - Invariant: a lookup must return a transform between the requested
     frames over a strict tree. `""` marks a missing name, and two
     publishers that forget it share a root, so unrelated frames resolve
     (AGENTS.md "Correctness invariants"; "Transforms are validated where
     they are built").
   - Conditions from the ruling: one additive variant on a
     `#[non_exhaustive]` enum; a MIGRATION note that stored `""` data
     stops decoding; an AGENTS.md edit for the identity carve-out
     (`get_transform("", "", t)`) and the `validate` list; explicit
     maintainer sign-off because it grows the public surface. If the
     maintainer rules that rejecting previously accepted input is a
     break, it moves to the v3 ledger as a plain row, with outreach for
     2.x.
   - Leading-`/` stripping stays in adapters: tf2 rewrites names silently.

### transforms docs

2. **Clock jumps under `with_max_age`.** Already proposed in
   [dimos.md](dimos.md) ("Document clock jumps"; shape under "Docs:
   clock-jump recipe"). Not
   shipped: the `with_max_age` rustdoc
   (`src/core/registry/mod.rs:122-130`) has no clock-jump text.
   - Evidence: H7 (P6, V6, N3) through transforms_io's 10 s default.
   - Content, corrected by verification: an insert older than the frame's
     newest minus `max_age` returns `Ok` and is evicted at once; a
     backward jump, large or small, ends in a window where two epochs
     interleave; the recovery is `remove_transforms_before` with a cutoff
     past every sample, and "before" is strict. Also state once that a
     request older than `covered.end − max_age` can never be served. That
     absorbs the doc half of item 11.
   - Corroboration: dimos uses a 10 s window, roslibrust's docs pass
     10 s, and neither has jump handling.
3. **The kind-switch difference from tf2.** New; no proposal covers it.
   Add one sentence to the `StaticDynamicConflict` rustdoc
   (`src/core/registry/error.rs:81-96`) and to `Registry::add_transform`:
   a caller that logs and drops the rejected dynamic transform keeps the
   static pose answering every instant. Mirror `ReparentingNotSupported`'s
   "Do not resolve this error mechanically" (`error.rs:63-66`). A README
   tf2-table row (tf2 replaces a frame's cache when it moves between `/tf`
   and `/tf_static`; transforms keeps the first kind) is optional and
   descriptive. The ruling asked for the row; the index makes it optional
   because AGENTS.md says to keep the README brief and link to rustdoc for
   API contracts
   ([README.md](README.md#placement-disagreements-and-how-they-were-resolved)).
   - Evidence: H4 (P3, V3), a silent wrong answer at every instant.
   - Corroboration: roslibrust is exposed the same way
     ([roslibrust_transforms.md](roslibrust_transforms.md)).
     [TF2_RESEARCH.md:79](../reparent-frame-value/TF2_RESEARCH.md)
     records the tf2 fact with no doc action. It is the kind sibling of
     the README "Re-parenting" row (README.md:143).
   - No code change: static-xor-dynamic is an AGENTS.md invariant. A
     bridge that wants tf2's behaviour must expose `remove_frame` on
     purpose.
4. **The quaternion tolerance against tf2.** Low priority. Add one
   sentence to the `UNIT_NORM_TOLERANCE` rustdoc
   (`src/geometry/transform/mod.rs:18-30`): a source validated more
   loosely (tf2 accepts `|‖q‖² − 1| < 1e-2`) must normalize within a
   bound the adapter states before construction, never unconditionally.
   Do not recommend a bound. The ruling placed this in a README tf2-table
   row; the index moves it to rustdoc for the same AGENTS.md reason as
   item 3.
   - Evidence: H8 (P7, V7). Neither ROS adapter normalizes.
   - Do not loosen `UNIT_NORM_TOLERANCE` (see "Do not").
5. **`latest_common_time` discoverability and `Stamp` versus `T`.**
   Nothing new. This strengthens
   [v2.4 DESIGN.md](../v2.4-proposal/DESIGN.md) §5 Part C items 2 and 4:
   H1 shows what callers do without them.

### v3 ledger

6. **Frame names in insert errors.** Already recommended in
   [dimos.md](dimos.md) ("v3: frame names in insert errors") and noted in
   [v3-warrant REPORT.md](../v3-warrant/REPORT.md) ("Frame-less errors"),
   but it is not yet a row in the ledger table ("What genuinely requires a
   break"). Add the row when the ledger is next edited.
   - New recurrence: transforms_io (`mt_pubsub_backend.rs:343-363`),
     roslibrust (explicit comment, `rt@0.26.0:src/lib.rs:324-331`) and
     whirl_tf (`wtf:src/buffer.rs:147-153`) all work around it.
   - Low value, not a trigger: the per-element workaround is exact.
7. **An insert that its own `max_age` window evicts reports an error.**
   Contested, low priority.
   - For: `Ok` for data that is not stored is a plausible success, against
     "prefer a loud error over a silent guess". tf2 refuses old data with
     `TF_OLD_DATA`.
   - Against (from the ruling): it does not fix the silent case. A small
     jump keeps and interleaves the new samples, so no error fires. It
     would also fire on routine out-of-order arrival. No dependent asked.
   - [dimos.md](dimos.md) "Do not" rules out an `Expired`-on-insert
     error as the 2.x default, and its former opt-in fallback is ruled out
     by the 2026-11-03 stance, so the ledger is the only home. The
     2026-10-30 dimos report's atomic-batch objection does not apply to
     the per-element `add_transform` that every bridge uses, but a v3
     design must still settle how `add_transforms` treats such a member.
8. **Cost line for "Cons": re-exports deepen the migration chain.**
   transforms_io re-exports six transforms types and uses them in its own
   `pub fn` signatures; pelorus re-exports five again. A transforms major
   forces a transforms_io major and a pelorus change.
   - Corrected lag evidence: `latest_common_time` (2.1.0) is unused across
     seven releases, 0.5.1 to 0.8.0. "2.3 not adopted" spans only nine
     days and is weak.
   - Note the counter-reading under "Pros" ("It will never be cheaper"):
     the chain grows with each downstream.

### Outreach to transforms_io

One issue or PR set on Codeberg. Every item is buildable on the public
API, and none needs a transforms change.

9. **Rename the r2r, ros2-client and hiroz `lookup_transform` parameters
   to `(target_frame, source_frame)`**, as `8388a61` did for mt-pubsub,
   with the same doc line. Rename only: positional behaviour, and so the
   README and pelorus, stay correct. Evidence: H2 (P8, V8). No transforms
   action: argument direction is inherent to string frames
   ([v3-warrant REPORT.md](../v3-warrant/REPORT.md) item 3), and
   `get_transform_for` exists for callers who want the direction carried
   by the value.
10. **Replace time zero to `now()` with `latest_common_time`.** Call
    `latest_common_time(target, source)` and `get_transform` under one read
    guard; map `Static` to any instant; stamp the returned header with the
    instant actually served instead of (0,0). Evidence: H1 (P1, V1, N4).
    Also to roslibrust, whose examples repeat the `now()` lookup
    (`rt@0.26.0:examples/ros1.rs:58`, `ros2_rosbridge.rs:56`).
11. **Carry `RegistryError<Timestamp>` in `TfError::Lookup`** (`#[from]`)
    instead of a `String`, so pelorus can stop retrying failures that
    waiting cannot fix. Evidence: the error and wait rows of the
    hand-rolled table. Same suggestion to roslibrust (`LookupError`).
    whirl_tf already keeps the typed error.
12. **Validate frame names on ingest** until transforms rejects `""`
    (item 1): reject empty ids; decide deliberately about a leading `/`.
    Evidence: H5.
13. **Own clock-jump handling**, as tf2_ros does, or expose a reset.
    Evidence: H7. The private registry means users cannot do it.
14. **Escalate and dedupe `StaticDynamicConflict`.** On `/tf` for a static
    frame it means lookups are stale; log it once per edge at warn, as
    `ReparentingNotSupported` already is, and decide whether to expose
    `remove_frame`. Evidence: H4. Also: for a dynamic frame, keep the
    reparent warning visible after the first line (N1).
15. **Normalize hand-typed quaternions within a bound the bridge states,
    and log beyond it.** Evidence: H8. Also: `isometry_from_transform`
    normalizes raw ROS quaternions unchecked, so an all-zero message
    gives NaN (hand-rolled table; optional, per the scope ruling).
16. **Check ROS time instead of clamping.** Reject or log `sec < 0`; do not
    alias. Flag the silent `i32` wrap in `timestamp_to_hiroz_time`.
    Evidence: H6 (P5). whirl_tf shows the checked version
    (`wtf:src/buffer.rs:395-416`). Same suggestion to roslibrust.
17. **Accumulate statics in `TfBroadcaster`** and republish the full set,
    as tf2_ros does. Evidence: H11. Same in roslibrust.
18. **If `static_transforms()` is ever replayed, key it by child and keep
    arrival order.** Evidence: H10 (P10).
19. **If adopting 2.3, keep per-element `add_transform` for `TFMessage`
    ingestion.** `Registry::add_transforms` would drop a whole message on
    one bad edge.

### Do not

- **A core `get_latest_transform`.** The composition is exact on the
  public API under one guard, and `latest_common_time` is core because of
  its max-of-starts guard, which "does not license" neighbours (AGENTS.md
  Scope). Of three conventions, two are correct. H1's silent variant does
  not change this: the pose served is correct for the instant `now()`, the
  adapter's (0,0) header hides that instant, and a core query would not
  stop an adapter from calling `get_transform(.., now())`.
- **Typed frame handles or another 2.x direction API for H2.** The defect
  is in transforms_io's parameter names. Revisit only if callers misuse
  transforms' own `get_transform` or `Transform::new` direction
  ([v3-warrant REPORT.md](../v3-warrant/REPORT.md) item 3).
- **Loosen `UNIT_NORM_TOLERANCE`, or add a normalizing constructor.** The
  first breaks the AGENTS.md validation invariant. The second is exact
  outside (`Quaternion::normalize` then `Transform::new`).
- **Make a kind switch replace the cache, as tf2 does.** It breaks the
  static-xor-dynamic invariant, and marking a static stale after a
  contradicting insert turns `Ok` into `Err`, a v3-class change with no
  demand.
- **An `is_retryable` helper or a core wait.** Already rejected
  ([v2.4 DESIGN.md](../v2.4-proposal/DESIGN.md) §6). The classification is
  exact from the typed payload plus the caller's `max_age`, and AGENTS.md
  Scope names waiting as outside. The adapters' cost is latency and lost
  diagnostics, not wrong answers.
- **`Timestamp::try_from_secs_nanos` in the core.** Buildable exactly
  outside, and whirl_tf does. Revisit only if a third integration clamps
  and negative stamps are shown to occur.
- **A registry read-out API for statics.** The shadow log keeps refused
  statics on purpose, and AGENTS.md Scope forbids exposing raw state.
- **Change the maximum-gap item's status.** It stays "design doc first"
  ([dimos.md](dimos.md)). transforms_io makes H9 reachable but shows no
  need.
- **A nalgebra companion crate.** transforms_io's transforms-to-nalgebra
  converters are called nowhere, and pelorus imports only the
  ROS-to-nalgebra one. The dependency rule bars nalgebra from the core
  anyway.
- **An `Expired`-on-insert error as a 2.x opt-in.** A knob with one setting
  beside an unchanged default is the bandaid the 2026-11-03 stance rules
  out (item 7 holds the ledger entry).

## Probes

All probes build against this checkout (`f9770d2`). Wall-clock-based
numbers (P1, V1, N4) differ between runs; the outcome does not.

| Label | Scenario | Result | File |
|---|---|---|---|
| P1 | Time zero to `now()`: real time, sim time, static chain | `Err` "covers […]" on dynamic chains; `latest_common_time` path `Ok(3.0)`; static `Ok(0.3)` | `probes/transforms_io.rs` |
| P2 | Static re-published under a new parent | 1 WARN then TRACE; old parent `Ok(0.3)` at 50 s; `mast<-laser` unknown | `probes/transforms_io.rs` |
| P2b | Same parent, new static pose | 0.3 becomes 0.35 at the same t = 10 s | `probes/transforms_io.rs` |
| P3 | `/tf_static` then `/tf` on one edge | 5 ERROR lines; `map<-odom` at 102 s `Ok(0.0)`, publisher said 3.0; reverse order fails loudly | `probes/transforms_io.rs` |
| P4 | Empty `frame_id` | No log; `camera<-lidar` `Ok(2.0)` | `probes/transforms_io.rs` |
| P5 | Negative ROS seconds | Three stamps alias to 500000000 ns; lookup `Ok(2.0)` (last write wins) | `probes/transforms_io.rs` |
| P6 | Sim reset with 10 s `max_age` | 0 log lines; lookup `Err` "covers [20, 30]"; small jump `Ok(-2.4999…)` | `probes/transforms_io.rs` |
| P7 | Rounded quaternions | 4 and 3 decimals rejected (tf2 accepts); 8 decimals and `f32` pass | `probes/transforms_io.rs` |
| P8 | r2r names versus mt-pubsub names | r2r by name `Ok((-0.3, "laser<-base_link"))`; mt-pubsub `Ok((0.3, "base_link<-laser"))` | `probes/transforms_io.rs` |
| P9 | 8 s dropout inside 10 s window | `Ok(4.0)` at 104 s | `probes/transforms_io.rs` |
| P10 | Static log replay | 4 entries, 3 children; live `Ok(1.0)`, replay `Ok(2.0)` | `probes/transforms_io.rs` |
| V1 | P1 independently, plus a +50 ms publisher skew | Skew: `Ok(5.005…)` stamped (0,0) while newest is 10 | `probes/transforms_io_verify.rs` |
| V2–V10 | P2–P10 independently | Same outcomes; V6 adds recovery via `remove_transforms_before`: `Ok(-10.0)` | `probes/transforms_io_verify.rs` |
| N1 | Dynamic parent hand-over under the dedupe | 1 WARN, 49 TRACE; both lookups fail | `probes/transforms_io_verify.rs` |
| N2 | (0,0)-stamped `/tf` sample in a live stream | No log; evicted on arrival; no static flip | `probes/transforms_io_verify.rs` |
| N3 | Large backward jump, new epoch keeps running | Loss until 20 s, then 25.05 s gives 7.55 (mixed epochs), clean by 41 s; 0 log lines | `probes/transforms_io_verify.rs` |
| N4 | AMCL-style post-dated `map→odom`, time zero | `map<-odom` `Ok(79.2…)` stamped (0,0), newest is 99; `map<-base_link` `Err` | `probes/transforms_io_verify.rs` |

Run with
`cargo run --manifest-path analysis/dependents/probes/Cargo.toml --bin
transforms_io` (or `--bin transforms_io_verify`).

## Sources

- transforms_io crates.io tarballs 0.1.0, 0.2.0, 0.3.0, 0.4.0, 0.5.0,
  0.5.1, 0.5.2, 0.5.3, 0.5.4, 0.6.0, 0.7.0, 0.8.0;
  [crates.io/crates/transforms_io](https://crates.io/crates/transforms_io)
  API, read 2026-11-06.
- Upstream
  [codeberg.org/stelzo/transforms_io](https://codeberg.org/stelzo/transforms_io)
  `main` at `2e6eb80`. Commits cited: `2b85c56`, `8fb3cf8`, `95ef7dc`,
  `93b4ee9`, `8388a61`, `19fbbe0`, `19c2cbb`, `2e6eb80`.
- `pelorus_syncio` 0.0.9 (crates.io tarball, vcs `c7ac1d9`).
- minot `main` at `b394045` (Codeberg, shallow clone).
- `roslibrust_transforms` 0.26.0 and `whirl_tf` 0.1.0-alpha.3 (crates.io
  tarballs).
- transforms: master `f9770d2`; tags `v1.4.1`, `v2.0.0`; README.md,
  CHANGELOG.md, `src/core/registry/{mod,error}.rs`,
  `src/geometry/transform/mod.rs`.
- [ros2/geometry2](https://github.com/ros2/geometry2) `rolling` at
  `183399c`: `tf2/src/buffer_core.cpp`, `tf2/src/cache.cpp`,
  `tf2_ros/src/buffer.cpp`, `tf2_ros/src/static_transform_broadcaster.cpp`,
  `tf2_ros/src/static_transform_broadcaster_program.cpp`,
  `tf2_ros/include/tf2_ros/qos.hpp`.
- [ros-navigation/navigation2](https://github.com/ros-navigation/navigation2)
  at `35d9203`: `nav2_amcl/src/amcl_node.cpp`.
- Related analysis: [dimos.md](dimos.md),
  [roslibrust_transforms.md](roslibrust_transforms.md),
  [README.md](README.md),
  [v3-warrant REPORT.md](../v3-warrant/REPORT.md),
  [v2.4 DESIGN.md](../v2.4-proposal/DESIGN.md),
  [reparent-frame-value REPORT.md](../reparent-frame-value/REPORT.md),
  [TF2_RESEARCH.md](../reparent-frame-value/TF2_RESEARCH.md).
