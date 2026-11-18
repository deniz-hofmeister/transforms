# Dependents map for the v3 brainstorm

Source: `analysis/dependents/` (README.md, OPEN_QUESTIONS.md, dimos.md,
transforms_io.md, roslibrust_transforms.md, probes/). Citations of the form
`file.md:N` are to that folder. Downstream line numbers (`tf.rs:N`,
`lib.rs:N`, `mt_pubsub_backend.rs:N`) are **as cited by the reports, not
re-verified by me** — no dependent source is in this repository.

Verification status of this pass:

- Read in full: the five `.md` files, `probes/Cargo.toml`, `probes/wasm_try_now.rs`.
- Probes `dimos.rs`, `transforms_io*.rs`, `roslibrust_transforms*.rs`: headers
  read and API calls tallied by grep; **not read line by line, not run**.
- Checked against the working tree (v2.4.0): which findings 2.4 already
  closed (`CHANGELOG.md` 2.4.0 section, `src/core/registry/error.rs:54`,
  `src/core/buffer/mod.rs:325`, `src/core/registry/mod.rs:129-141`).
- The study itself built or ran no dependent code (README.md:39-41); its
  probes copy call patterns against master `f9770d2` (= v2.3.0 source).
- Study dates: 2026-10-30 / 2026-11-06, with a 2026-11-12 status note. All
  dependents were studied against 2.1–2.3, none against 2.4.

## 1. Who uses the crate (README.md:47-55)

| User | Kind | Requirement | State |
|---|---|---|---|
| transforms_io 0.8.0 | ROS 2 / Minot bridge, 4 backends | `^2.2` | Active; 2,559 downloads; no issues/PRs |
| pelorus_syncio 0.0.9 | lidar IO layer, via transforms_io | transitive | 1 release |
| roslibrust_transforms 0.26.0 | ROS 1/2 adapter | `^2.1` | Idle since 2026-09-25; 87 downloads; 2.x migration written by the transforms maintainer |
| dimos (PR #4722) | robotics framework, the only non-adapter | `=2.2.0` git fork `TomCC7/transforms@5ae9563` | Unmerged PR; active again 2026-11-09 (dimos.md:12-20) |
| whirl_tf 0.1.0-alpha.3 | tf2-style buffer + PyO3 | `=2.0.0-beta.4` | Dormant; cannot compile on any 2.x (by reading) |
| termviz2, transforms-py | 1.x users | `1.2`, `1.0.0` | Not studied |

Total crates.io reverse deps: 3. Downloads are not attributable to users.

## 2. Per-dependent API usage

### dimos (dimos.md:138-184)

- **Uses:** `Registry<SystemTime>`, `Registry::with_max_age(10 s)`,
  `add_transforms` (the API exists because of dimos), `latest_common_time`,
  `get_transform`, `Transform::new`, `Transform::validate` on **every lookup
  result** (`tf.rs:159-160`), `Stamp::at()`.
- **Never touches:** `add_transform`, `get_transform_at`, `get_transform_for`,
  `reparent_frame`, `remove_frame`, `remove_transforms_before`,
  `Registry::new`, `Timestamp`, `Point`, `Transformable`, `serde`.
- **Errors:** no variant matched. Insert errors → `io::Error` or one
  `warn!("TF message rejected")`; lookup errors → `io::Error` string or `None`.
- **Concurrency:** `RwLock<Registry>` + tokio `Notify`; one write lock per TF
  message then `notify_waiters`; `Notify` armed before resolve (race-free).
- **Conversions:** signed ROS ns ↔ `SystemTime`, checked, pre-epoch allowed
  (`tf.rs:50-81`); ROS wire ↔ `Transform`; ROS wire ↔ nalgebra. **Never
  `transforms` ↔ nalgebra.** Static results stamped `UNIX_EPOCH` on the wire.

### transforms_io (transforms_io.md:144-225)

- **Uses:** `Registry<Timestamp>`, `with_max_age(10 s)`, `add_transform` per
  element, `get_transform` (the one forwarded method, as `lookup_transform`),
  `Transform::new` with struct-literal `Quaternion { w, x, y, z }`,
  `Vector3::new`, `Stamp::{At, Static}`, `Timestamp::now`,
  `Timestamp::as_nanos` (once), `RegistryError::ReparentingNotSupported`
  (mt-pubsub only — the only variant any dependent matches).
- **Never touches** (grep over 12 releases): `get_transform_for`,
  `get_transform_at`, `latest_common_time`, `add_transforms`,
  `reparent_frame`, `remove_frame`, `remove_transforms_before`,
  `Registry::new`, `static_between`, `validate`, `inverse`, `interpolate`,
  `Mul`, `Point`, `Transformable`, `Localized`, `Quaternion::normalize`/`slerp`,
  `try_now`, `SystemTime`, `serde`.
- **Registry is private with no accessor:** its users can reach nothing but
  one lookup. A transforms improvement reaches them only through a
  transforms_io release.
- **Errors:** `TfError::Lookup(String)` / `InvalidData(String)`; insert
  failures only logged.
- **Concurrency:** `Arc<RwLock<Registry<Timestamp>>>`; one write guard per
  `TFMessage`; one read guard per lookup; poisoned lock drops the message.
- **Conversions:** ROS time via `sec.max(0) as u64 * 1e9 + nanosec` (wrong);
  ROS msg ↔ `Transform`; `convert.rs` transforms↔nalgebra converters exist
  and are **called nowhere**; `isometry_from_transform` goes ROS → nalgebra
  directly, normalizing unchecked.
- **Public re-exports:** `Registry`, `Quaternion`, `Transform`, `Vector3`,
  `Stamp`, `Timestamp`, and transforms types in its own `pub fn` signatures.
  pelorus re-exports five of them again.
- **pelorus:** calls only `lookup_transform`, positionally in tf2 order, at
  the cloud's header stamp; reads `.transform` only; converts to **f32
  nalgebra by hand and inverts it**; never uses `Transformable`.

### roslibrust_transforms (roslibrust_transforms.md:112-158)

- **Uses:** `Registry<T>` generic over `T: TimePoint` (default `Timestamp`),
  `with_max_age`, `add_transform`, `get_transform`, `get_transform_at`
  (the only dependent exposing it; the API originated from this crate's
  author), `latest_common_time`, `remove_frame`, `Transform::new` with
  struct-literal `Quaternion`, accessors on the way out,
  `Timestamp::from_nanos`/`as_nanos`. Implements a third-party `TimePoint`
  (`MockTimestamp`, tests only) — the only one seen.
- **Never touches:** `Registry::new`, `add_transforms`, `get_transform_for`,
  `remove_transforms_before`, `reparent_frame` (unreachable at `^2.1`), every
  `RegistryError` variant, `Point`, `Transformable`, `Localized`, `validate`,
  `interpolate`, `inverse`, `Mul`, `Quaternion::from_wxyz`/`normalize`,
  `UNIT_NORM_TOLERANCE`, `serde`, `no_std`.
- **Errors:** `LookupError(String)`, `RejectedTransform(String)`,
  `Timeout(String, String, String)`. Re-exports `TransformError` but not
  `RegistryError`. Inferred (not stated) reason: `RegistryError<T>` being
  generic would force `TransformManagerError<T>` (F5).
- **Concurrency:** `Arc<tokio::sync::RwLock<Registry<T>>>`; two detached
  tokio ingest tasks (`/tf`, `/tf_static`), one write guard per message;
  each lookup wrapper its own read guard; `tokio::sync::watch` for waits.
  The private lock makes transforms' "one guard for `latest_common_time` +
  `get_transform`" advice impossible for its users (F4).
- **Conversions:** ROS 1/ROS 2 msg ↔ `Transform` (198 lines), own
  `RosTimestamp` trait for `Timestamp` and `SystemTime`. **No nalgebra.**

### whirl_tf (README.md:80-123; beta.4 only)

- **Uses:** `Registry::with_max_age`, `add_transform`, `get_transform`,
  `remove_frame`, `TransformError`. Keeps the typed error as `source`.
- nalgebra both ways. The only dependent that converts transforms ↔ nalgebra
  on a live path.

## 3. Wrappers / helpers written around the crate

From README.md:134-149 (matrix) and the per-report tables.

| Helper | dimos | transforms_io | roslibrust | whirl_tf | Exact on public API? |
|---|---|---|---|---|---|
| Frame names added to insert errors | **Cannot** (`add_transforms` names no member) | hand-formats | clones both names per ingested transform on the hot path, with a comment | typed wrapper | Per element yes; batch **no** |
| ROS time → time type | checked, signed | clamps (wrong) | clamps to zero, drops nsec (wrong) | checked, rejects | Yes |
| "Latest" lookup | `latest_common_time` + `Static → UNIX_EPOCH` (correct) | `Timestamp::now()` (wrong) | `latest_common_time` + `Static → now()` (correct; README/examples teach `now()`) | none | Yes, under one guard |
| Wait for transform | `Notify`, returns `None` | pelorus polls 10 ms / 5 s on any `Err` | `watch`, swallows error | poll on `can_transform` bool | Yes; **all four error-blind** |
| Errors → strings | yes | yes | yes | no | — |
| Reject `""` frame | yes (+ leading `/`) | absent | absent | yes (+ strips `/`) | Yes (closed in 2.4 at insert) |
| Pre-insert finite/norm check | yes, ~2× stricter than crate (squared-norm 1e-6) | no | no | yes, crate's constant | Redundant |
| Re-validate every lookup result | yes | — | — | — | Yes |
| Batch insertion | atomic `add_transforms` | per element, skip+log; private helper also named `add_transforms` with opposite atomicity | per element, skip+log | partial commit | — |
| Clock-reset handling | **lost in migration** | absent, users cannot add | absent | manual `clear_dynamic` = `remove_frame` loop (releases pins) | Registry-wide only, with a hole; per edge **no** |
| Gap / staleness bound | **lost in migration** | absent | absent | absent | **No** (needs sample/coverage access) |
| `/tf` vs `/tf_static` → kind | not examined | by topic | by topic, two tasks, arrival order pins kind | not examined | Yes |
| Shadow log of statics "as published" | — | yes (mt-pubsub), incl. ones the registry refused | — | — | Yes |
| Quaternion normalization within a bound | no | absent | absent | — | Yes |
| Per-pair warning dedup | lost | `ReparentingNotSupported` only | no | no | Yes |
| `Stamp::Static => <pick an instant>` arm at every latest call site | yes | n/a | yes | — | Yes |

The reports' own reading: recurrence "points at boundaries and time, not
geometry"; "no user hand-rolls geometry the crate lacks" (README.md:166-170).
Only two helpers are *not* buildable exactly outside: the gap bound and
per-edge clock-reset detection.

## 4. Bugs / wrong-answer generators found downstream

"2.4" column: closed by v2.4.0 as verified in this tree. "Shape" is my
inference of the crate-side API shape that would make it impossible or loud;
the reports' own placement is noted where it differs.

| # | Hazard | Silent? | Where | Probe result | Cause per report | 2.4 | Crate-side shape that would prevent it |
|---|---|---|---|---|---|---|---|
| 1 | `""` frame id joins unrelated trees | Silent | tio H5, rlt Z1 (dimos, whirl_tf guard by hand) | camera←lidar `Ok(2.0)` | both | **Closed at insert** (`RegistryError::EmptyFrameName`; constructors/serde still accept `""`) | A validated frame-name type (non-empty by construction) would close it at construction rather than insert. Reports' T1 notes `Mul` also matches names, so hand-composition outside the registry still joins on `""` |
| 2 | Clock reset: stale "latest", then two epochs interleaved | Silent | dimos H1, tio H7, rlt Z3 | 3000 inserts `Ok`, 0 kept; later `Ok(7.55)` blend of 25.1 and −10 | both | Docs only (`mod.rs:135-141`) | (a) an insert that its own `max_age` evicts is not `Ok(())` — contested in the reports: misses jumps < `max_age`, fires on out-of-order arrival; (b) a first-class clear-dynamic operation (the cutoff recipe has a hole: a sample exactly at the cutoff survives and anchors expiry; `TimePoint` has no max); (c) per-edge newest stamp or a monotonicity policy so the owner can detect it. None of these is settled |
| 3 | Kind switch `/tf_static` → `/tf` freezes at the static pose | Silent | tio H4, rlt Z2/Z2b | `Ok(0.0)`, publisher said 3.0 | adapter drops the loud `StaticDynamicConflict` | Docs only | Loud already; the adapter drops it. Frame names in the error + an explicit, non-mechanical kind-change call would make the decision reachable. Reports forbid tf2's "newest kind wins" |
| 4 | Interpolation across a dropout inside `max_age` | Silent | dimos H2, tio H9, rlt Z8 | 100 s/108 s → `Ok(4.0)` at 104 s | transforms (unbounded by design) | Open | Per-frame max gap (`set_max_gap`, `GapExceeded`), which also forces a `latest_common_time` change (dimos.md:598-650). "Largest silent hazard without a recommendation", not buildable outside, **no user asked** |
| 5 | Pre-epoch / negative ROS stamps alias | Silent, rare | tio H6, rlt Z6 | three stamps → 500000000 ns; lookup `Ok(2.0)` | adapter | Doc recipe only | A checked `(secs, nanos)` constructor on `Timestamp`. Reports say "Do not" in core (exact outside, whirl_tf ~20 lines) with a reopen condition: a third clamping integration |
| 6 | "Latest" answered at wall-clock `now()` | Loud usually; silent on post-dated/skewed hops, stamped (0,0) | tio H1; rlt docs/examples Z9 | AMCL-style: `Ok(79.2)`, newest is 99 | adapter | Doc recipe | The reports reject a core `get_latest_transform` (3 users, 3 conventions, 2 correct). What made it hard: `latest_common_time` returns `Stamp`, forcing a `Static => pick an instant` arm; and a lookup result for an all-static chain needs a stamp from somewhere |
| 7 | Cross-time result re-inserted overwrites the true sample and is broadcast on `/tf` | Silent, propagates | rlt Z4 (reachability, not observed) | odom←base at t2: 5 → −1 | transforms (result type is an insertable `Transform`) + adapter | Open, v3 ledger | `get_transform_at` returns a distinct non-insertable type |
| 8 | Lookup parameters named backwards (`source, target` passed as `target, source`) | Silent for by-name callers | tio H2, 3 of 4 backends | `Ok(-0.3, laser<-base_link)` | adapter; origin is transforms 1.x's `from`/`to` | n/a | Reports: "Do not" add typed frame handles; `get_transform_for` exists. Direction is still carried only by position of two `&str` |
| 9 | Static re-published under a new parent keeps old pose | Stale old-parent answer; loud rejection | tio H3 | old parent `Ok(0.3)` | by design | `reparent_frame` (2.2) | Same as 3: names in the error |
| 10 | Hand-typed quaternions tf2 accepts are dropped (`0.7071`) | Loud | tio H8, rlt Z5 | `NonUnitRotation(0.99999041)` | tolerance difference (1e-6 vs tf2 1e-2 on squared norm) | Not documented (rec 12 left out) | Reports: "Do not" loosen or add a normalizing constructor. An explicit "normalize within stated bound" entry point is the alternative; frequency unmeasured |
| 11 | Lookup `Ok(inf)` from overflowing composition | Silent | dimos P6 (two static hops of 1.5e308) | raw `Ok(inf)` | transforms: derived values not re-validated | Open, v3 ledger ("`Rotation` newtype and translation bound") | Finite check on lookup output (dimos does it; cost: can reject a valid long chain if norms drift — Hazard 5, **not probed**) |
| 12 | Pre-epoch `SystemTime` diagnostics print `covers [NaN, NaN]` | Loud, diagnostic | dimos P4 | — | `as_seconds_lossy` contract | Open (rec 16 out of 2.4) | Negative seconds instead of NaN |
| 13 | `Timestamp::try_now()` traps on `wasm32-unknown-unknown` | Loud | rlt Z10, roslibrust PR #352 | `RuntimeError unreachable` | std + docs | Doc corrected | — |
| 14 | Remove-then-add pre-empted by a late old-parent message | Loud | rlt Z11 | `ReparentingNotSupported` | adapter (`^2.1`, two guards) | `reparent_frame` exists | — |
| 15 | Wait loops retry permanent failures; request older than `covered.end − max_age` can never be served | Loud, causeless | rlt Z7, pelorus, dimos | — | adapter stringifies | Open | Typed errors are available and discarded; reports reject `is_retryable` |
| 16 | Insert returns `Ok` for data that is evicted on arrival | Silent | tio H7, rlt V7 | late 50 s insert `Ok`, lookup still fails | transforms | Open, contested v3 row 18 | See #2(a) |
| 17 | Static shadow-log replay picks a different parent than live registry | Only if replayed | tio H10 | live `Ok(1.0)`, replay `Ok(2.0)` | adapter | — | Exists because the registry refuses a second parent and offers no read-out; reports rule a read-out API out |
| 18 | Two TF implementations (Python nearest-sample, Rust interpolating) answer differently | Silent | dimos H6 | — | dimos | — | Outside the crate |

Fixed by the 1.x → 2.0 migration in both adapters (transforms_io.md:251-268,
roslibrust_transforms.md:172-194): the stamp-0 static flip, unvalidated
inserts, a `u128` sign-extension overflow, non-zero-stamped statics. That is
evidence that `Stamp::Static`, fallible constructors and checked inserts paid
off. Introduced by the same migration: the negative-seconds clamp and the
warn-and-drop policy behind #3.

## 5. What nobody uses

No studied dependent (dimos, transforms_io, pelorus, roslibrust_transforms)
uses:

- `Point`, `Transformable`, `Localized` — zero users (README.md:284-290).
- `get_transform_for` — zero users.
- `Transform::inverse`, `Transform::interpolate`, `Mul` — zero users
  (dimos and "other users were not checked" for `Mul`, OPEN_QUESTIONS.md:79-80).
- `Quaternion::slerp`, `Quaternion::normalize` — zero users.
- `Registry::new` (no `max_age`) — zero 2.x users; all use `with_max_age(10 s)`.
- `remove_transforms_before` — zero users.
- `reparent_frame` — zero users (transforms_io private registry; roslibrust
  at `^2.1`).
- `Transform::static_between` — zero users; all write `Transform::new(.., Stamp::Static)`.
- `serde` feature — zero users. `no_std` — zero users.
- `Timestamp::try_now` — zero users.

Used by exactly one: `add_transforms` (dimos), `get_transform_at`
(roslibrust), `remove_frame` (roslibrust; whirl_tf on beta.4),
`Transform::validate` (dimos), `SystemTime` as `T` (dimos; roslibrust
supports it), a custom `TimePoint` (roslibrust tests), matching a
`RegistryError` variant (transforms_io, one variant).

Used by all: `Registry::with_max_age`, `get_transform`, `Transform::new`,
`Stamp`, an insert call. `latest_common_time`: dimos and roslibrust; unused
across seven transforms_io releases after it shipped.

Caveat: "unused by three thin adapters and one unmerged PR" is weak evidence
about `no_std`/`serde`/`Point` users in private code; the study searched
crates.io reverse deps and GitHub default branches only (README.md:72-75).

## 6. Open questions (OPEN_QUESTIONS.md)

Ruled 2026-11-12: Q1–Q10, Q17 (no outreach for now). Moved to the v3 review:
Q11, Q12 (frame fields on insert errors), Q16 (max interpolation gap, "Not in
3.0"). Not ruled: Q13, Q14, Q18, Q19 (bookkeeping).

Trade-offs still live for v3:

- **T1** empty names at insert vs in the type. 2.4 chose insert. Tipping
  evidence named: a caller composing `Transform`s by hand with unset names.
  None found; not checked for dimos.
- **T3** v3 ledger as record vs wishlist; no entry rule for Low rows.
- **T4** reachability vs occurrence. No user has reported any hazard.
- **T5** outreach vs core. transforms_io has never had an issue or PR;
  roslibrust is idle; nothing was filed (Q17: none for now). Hazards left to
  outreach stay live.
- **T6** re-export chain: transforms_io re-exports six types, pelorus five; a
  transforms major forces both.
- **T7** each dependent that starts matching variants raises the cost of
  reshaping them; today almost none match.
- **T8** max interpolation gap: the largest silent hazard, not buildable
  outside, nobody asked; a default bound "has no defensible value".

Unverified in the study (OPEN_QUESTIONS.md:198-242): dimos Hazard 5; saved
probes do not reproduce O4, O7, O9 (the cutoff hole); tf2 read not run;
prevalence of rounded quaternions; whirl_tf's non-compilation; QoS behaviour.

## 7. Ranked: what real users needed that the API made hard

1. **Diagnosable insert rejections.** All four users. Unit variants
   `SelfReferentialFrame`, `CycleDetected`, `StaticDynamicConflict` carry no
   frames; `ReparentingNotSupported` names only the current parent;
   `add_transforms` does not say which member failed. Three hand-roll; dimos
   cannot. The only gap in transforms' own API hit by all four.
2. **Surviving a clock reset.** Three exposed, one mitigates by a
   pin-releasing loop. No clear operation; the documented recipe has a hole
   and needs a far-future instant `TimePoint` cannot name; per-edge detection
   needs state the API does not expose; evicted-on-arrival inserts return `Ok`.
3. **Bounding interpolation across dropouts / staleness.** Reachable in three,
   dimos lost a feature it had. Not buildable outside. No request.
4. **"Latest".** Three users, three conventions, one wrong. Friction is
   `Stamp` vs `T` and discoverability, not a missing query.
5. **Telling permanent from transient lookup failures.** Every wait loop is
   error-blind because every adapter stringifies. Partly the adapters'
   choice; `RegistryError<T>` being generic is the inferred obstacle for
   roslibrust.
6. **Ingesting middleware time.** Two of four clamp wrongly.
7. **Ingesting middleware frame names.** Closed for `""` in 2.4; leading `/`
   remains adapter policy.
8. **Ingesting looser rotations.** Both ROS adapters drop data tf2 accepts.
   Loud; frequency unknown.
9. **Finite lookup outputs.** One user (dimos) re-validates every result.
10. **Kind change / re-parent as a decision.** Adapters warn-and-drop; one
    keeps a shadow log because there is no read-out.
11. **Type inference (E0283)** on `Transform::static_between` / all-static
    setups — hit while writing probes, avoided downstream by accident.
12. **Lookup direction.** One adapter carried 1.x's backwards names for 12
    releases; the crate's own names are correct.

## 8. Ranked: what the API offers that nobody uses

1. `Point` / `Transformable` / `Localized` — zero.
2. `get_transform_for` — zero (it depends on 1).
3. `Transform::interpolate`, `Transform::inverse`, `Mul` as public API — zero
   observed. pelorus inverts, but in nalgebra after conversion.
4. `Registry::new` without `max_age` — zero.
5. `remove_transforms_before` — zero (it is the documented clock-jump
   recovery nobody calls).
6. `Transform::static_between` — zero.
7. `serde`, `no_std` — zero among studied users.
8. `reparent_frame` — zero (reachability, not rejection).
9. `Quaternion::slerp` / `normalize` — zero.
10. `get_transform_at` — one, the crate whose author requested it; also the
    source of hazard #7.
11. `add_transforms` — one (dimos, on a fork predating the shipped version);
    the bridges are right not to use it for `TFMessage`.
12. Typed `RegistryError` payloads — one variant matched by one backend.

## 9. Things the reports rule out (context for a v3 that may reopen them)

README.md:525-572: a core `get_latest_transform`; a core wait or
`is_retryable`; core ROS time conversion; loosening `UNIT_NORM_TOLERANCE` or a
normalizing constructor; tf2-style kind replacement; a registry read-out for
statics; typed frame handles; a nalgebra companion crate; steering bridges to
`add_transforms`; a finiteness check on every lookup; a default gap bound.
These rulings were made under 2.x constraints (no breaks, no parallel API) and
the AGENTS.md Scope rule; several rest on "exact on the public API", which
this study also shows two of four users got wrong (time clamp, `now()` latest).
