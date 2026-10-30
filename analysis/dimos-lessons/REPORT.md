# Lessons from dimos: what a real integration means for 2.4 and v3

Analysis date: 2026-10-30. Baseline: master `ea62c00` (released v2.3.0).
Subject: [dimos PR #4722](https://github.com/dimensionalOS/dimos/pull/4722),
which replaces dimos's hand-rolled `tf.rs` with `transforms`, read at PR
head `92936a7`.

This report asks what the first non-adapter user of the crate teaches about
the 2.4 proposal ([`DESIGN.md`](../v2.4-proposal/DESIGN.md)), the v3 ledger
([`REPORT.md`](../v3-warrant/REPORT.md)) and adoption. Nothing here is
implemented on master. Every recommendation that grows the public surface
is a maintainer note under AGENTS.md, not an authorization.

## Method

- Read dimos's old `tf.rs` (main), the new `tf.rs` (PR head) and the PR's
  full file list (`gh api --paginate`).
- Took a third-party review of the same PR as input only, and checked each
  of its claims against source and probes.
- Ran probe programs against the public API at `ea62c00` (table at the end).
- Prototyped the API shapes below: empty-name rejection in a worktree, the
  rest in a scratch copy of the crate.
- Re-checked the findings and the chat answer that preceded this report in
  independent verification passes. Corrections are listed below.
- **Not done:** the full `tests/test_all.sh` gate on any prototype; running
  dimos's own test suite against 2.3.0; asking the dimos maintainers anything.
  tf2 behaviour was read from `geometry2` source (rolling), not run.

## Verdict

- **The strict design survived a real integration.** dimos kept
  interpolation, no extrapolation, the strict tree and
  `latest_common_time`. Its `get_latest` now routes through
  `latest_common_time`. `add_transforms` (2.3.0) came from their need.
- **Real use exposes gaps in time handling across a run, not in geometry:**
  clock resets, interpolation across publisher dropouts, and diagnostics.
  The 2.4 proposal is mostly about applying transforms to data; dimos gives
  no evidence either way on that.
- **The cheapest adoption win is outreach.** dimos pins a git fork of 2.2.0
  (`TomCC7/transforms` rev `5ae9563`, 2.2.0 + 7 commits). The fork's
  `add_transforms` has the same signature as 2.3.0's.

## How dimos uses the crate

- `Registry<SystemTime>`, `with_max_age(10 s)`, behind `RwLock` + tokio
  `Notify`. They built wait-with-timeout on the public API, which is what
  AGENTS.md's scope rule predicts.
- Calls only `add_transforms`, `latest_common_time` and `get_transform`. No
  `get_transform_at`, `get_transform_for`, `reparent_frame`, `Point` or
  `Transformable`.
- Validates frame names itself (non-empty, no leading `/`) on insert and
  lookup. Also repeats the finite and norm check that `Transform::new`
  already runs (1e-6 on the squared norm).
- Errors become `io::Error` via `to_string`, then `Option` via `.ok()`. No
  `RegistryError` variant is ever matched, the same as the three crates.io
  dependents.
- On an all-static chain, `latest_common_time` returns `Static` and they pick
  `UNIX_EPOCH` as the lookup instant
  (`latest_common_time(..).at().unwrap_or(UNIX_EPOCH)`).
- The same PR keeps Python TF on nearest-sample + tolerance while Rust now
  interpolates. The PR body acknowledges the divergence.

### Lost in the migration

- **Clock-reset handling**, and its test
  `a_clock_reset_drops_the_pre_jump_samples`. After a simulation reset,
  `get_latest` keeps serving the pre-reset pose.
- **Rate-limited warnings per frame pair on failed lookups.** A failed
  lookup is now a silent `None`.
- **Any bound on staleness.** The old tolerance was loose (default: the whole
  window), but it existed. Interpolation now spans any retained gap.

## The third-party review, checked

| Claim | Verdict | Evidence |
|---|---|---|
| 1. Maximum interpolation gap | Confirmed | 100 s / 108 s samples give 4.0 at 104 s. A caller cannot build the check outside: the public API exposes no samples or coverage. One registry-wide limit is the wrong shape; it cannot serve a 50 Hz edge and a 1 Hz edge together. |
| 2. Error on a backward clock jump | Partial | Confirmed on `max_age` registries: 3000 inserts return `Ok`, none are kept, `latest_common_time` stays at 1000 s. It misses the worse cases: the default registry mixes epochs (444.44 at 500 s), and jumps smaller than `max_age` interleave old and new samples. The proposed error would also reject a whole atomic batch. |
| 3. Reject empty frame names | Agree | Stronger than given: `""` is the default for unset string fields in ROS 2 messages. Two publishers that both forget `frame_id` share a `""` root, and lookups between unrelated frames succeed (camera→lidar = 2.0). |
| 4. `UnknownFrame` for equal unregistered names | Reject | Identity is documented (`src/core/registry/mod.rs:264-265`). dimos's own test asserts that `get_latest("map","map")` returns identity. Only `""` and `"/map"` must fail there. |
| Quaternion tolerance "level" | Wrong | Old dimos normalized any quaternion with norm above 1e-9. New dimos adopted a 1e-6 check on the squared norm, in front of the crate's own check. |
| nalgebra companion crate | Weak | dimos converts the ROS wire type to and from `transforms`, and the wire type to and from nalgebra. It never converts `transforms` ↔ nalgebra. |
| Adapter and reject lists | Agree | — |

## Corrections to the chat answer that preceded this report

- **The `SystemTime` NaN is not a bug.** `src/time/system_time.rs:27-33` and
  the `TimePoint` docs make NaN the documented result for instants that
  cannot be converted, and the typed payloads stay intact. Printing negative
  seconds changes that contract: a 2.4 decision, not a 2.3.1 fix.
- **tf2 clears statics too.** `tf2_ros` clears on any backward jump
  (`onTimeJump`), and `BufferCore::clear` also drops static transforms. ROS
  restores them from latched `/tf_static`. dimos has no such durability on
  LCM/Zenoh, so `remove_transforms_before`, which keeps statics, is the
  right recovery for them.
- **"Clearing is merely verbose outside the crate" was overstated.** It needs
  an instant after all stored data; neither `TimePoint` nor `SystemTime` has
  a maximum, though a far-future value works. Detecting a jump per edge
  needs each edge's newest stamp, which the API does not expose. Clearing the
  whole registry, triggered by whoever owns the clock, works today, with the
  cutoff hole described under API shapes.
- **C.4 (`Stamp` vs `T`) is consistent with dimos's usage, not confirmed by
  it.**
- **Swapping in 2.3.0 is a signature match, not proven to behave the same.**
  The fork stages a batch in a cloned registry. dimos's tests should confirm
  the swap.

## Recommendations

### 2.4

- **Reject empty frame names.** A new `TransformError` variant, enforced by
  `Transform::validate`, so by the constructors, `Deserialize` and insert. A
  MIGRATION note: stored data with `""` stops decoding. The leading-`/` rule
  stays in adapters.
- **Document clock jumps.** Two failure modes: samples silently dropped with
  `max_age`, epochs silently mixed without it. Recovery is
  `remove_transforms_before(<after all data>)`, which keeps statics,
  topology, kind and `max_age`. A sample stamped exactly at the cutoff
  survives. Detection belongs to whoever owns the clock (the `tf2_ros`
  precedent).
- **Keep Parts A and C** of DESIGN.md. The `roslibrust_transforms`
  re-export of `get_transform_at` justifies Part A.
- **Defer Part B.** No evidence from dimos.

### Design doc first

- **Maximum interpolation gap, opt-in**, keeping the unbounded default.
  Recommended shape: a per-frame bound. It also changes how
  `latest_common_time` is computed.
- **A registry clear operation**, only if a user asks.
- **Negative seconds for pre-epoch `SystemTime`**, which needs a contract
  change.

### v3 ledger (add)

- **Frame names in insert errors** for `SelfReferentialFrame`,
  `CycleDetected` and `StaticDynamicConflict`. New evidence: in a batch,
  the caller cannot tell which member failed. There is no additive path.

### Do not

- Return `UnknownFrame` for equal unregistered names.
- Make an `Expired`-on-insert error the default. At most opt-in, at low
  priority.
- Build nalgebra or async companion crates before anyone asks.

### Outreach on PR #4722

- Suggest `transforms = "2.3"` from crates.io instead of the fork.
- Report the lost clock-reset handling and the silent lookup failures.
- Ask which gap tolerances they would set per edge.
- Ask whether Python bindings would end their Python/Rust split.

## API shapes

The exact public surface each recommendation would add. Empty-name rejection
was prototyped in a worktree; the other shapes were compiled or probed in a
scratch copy of the crate. Nothing is committed.

### 2.4: reject empty frame names

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

### 2.4: clock-jump recipe (docs only)

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
  stream (probe-verified). A garbage stamp of `u64::MAX` is reachable, for
  example from a cast of `-1`.

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
- It cannot be built generically outside the crate, because `TimePoint` has
  no maximum.
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
- **Errors.** `UnknownFrame` for a name that exists nowhere. `NoSampledEdge`
  for a root or static frame: a silent `Ok` there would make the caller
  believe a bound is active.
- **`latest_common_time` must change.** Min-of-ends can land inside a gap
  the bound vetoes. Example: hop A ends at 10 s; hop B has samples at 2 s
  and 12 s with a 5 s bound. Today's rule answers 10 s, which cannot be
  served; the correct answer is 2 s. Fix: start from the min of the ends,
  step each hop down to its newest servable instant at or before it, and
  repeat until nothing changes. About O(h log n) per call; keep the current
  fast path when no hop has a bound. The `NoCommonTime` payload is
  unchanged.
- **Setup window.** The frame must exist first, so `set_max_gap` follows its
  first insert. A single sample serves exact hits only, so the window does
  no harm.
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

- Every construction site already holds the transform: `Buffer::check`, the
  insert fallthrough, and three `CycleDetected` sites. A scratch copy of the
  library builds with these variants.
- `NonUnitRotation` and `NonFiniteValues` stay as they are.
  `From<TransformError>` has no frame to add, and a lookup-side
  `NonFiniteValues` comes from a composed chain with no single frame to
  blame.

## Probes

| Setup | Result |
|---|---|
| `max_age` 10 s; x=0 at 100 s, x=8 at 108 s; query 104 s | `Ok(4.0)` |
| `max_age` 10 s; history 990–1000 s; 3000 inserts near 100 s | 3000× `Ok`, 0 kept, `latest_common_time` = `At(1000 s)` |
| …then `remove_transforms_before(far)`; insert 100 s, 101 s; query 100.5 s | `Ok(1.5)` |
| `new()`; 50 s, 1000 s, then x=0 at 100 s; query 500 s | `Ok(444.44)`, two epochs mixed |
| `max_age` 10 s; 100–105 s, then x=−1 at 102.5 s; query 102.4 s | `Ok(19.6)`, interleaved |
| `("", camera)` x=1, `("", lidar)` x=3; camera→lidar | `Ok(2.0)` |
| ghost→ghost on an empty registry | identity; `latest_common_time` = `Static` |
| `Registry<SystemTime>`, pre-epoch samples, failed lookup | `"covers [NaN, NaN]"` |

## Sources

- dimos: [old `tf.rs` (main)](https://github.com/dimensionalOS/dimos/blob/main/native/rust/dimos-module/src/tf.rs),
  [new `tf.rs` (PR head `92936a7`)](https://github.com/dimensionalOS/dimos/blob/92936a710f4d87a6db4153db35d9db44236456da/native/rust/dimos-module/src/tf.rs).
- tf2: [`tf2_ros/src/buffer.cpp`](https://github.com/ros2/geometry2/blob/rolling/tf2_ros/src/buffer.cpp)
  (`onTimeJump`), [`tf2/src/buffer_core.cpp`](https://github.com/ros2/geometry2/blob/rolling/tf2/src/buffer_core.cpp)
  (`clear`, `validateFrameId`).
- This repository: [`analysis/v2.4-proposal/DESIGN.md`](../v2.4-proposal/DESIGN.md),
  [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md), `AGENTS.md`.
