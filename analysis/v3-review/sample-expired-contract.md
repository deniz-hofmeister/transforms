# Item 10: draft contract text for `SampleExpired`

**Note, 2026-11-12: do not implement this draft as written.** Its draft
rustdoc states "This is the last insert check", and the `remove_frame`
recovery relies on it, while [`REPORT.md`](REPORT.md) item 10 keeps the
check order internal. Whether to publish the order is decision 15 of the
report, which is open and is taken with the other v3 decisions after 2.4
ships. Until then the sentences that state the order are not settled.

Companion to [`REPORT.md`](REPORT.md), item 10. Draft wording for the
places that state the contract, written by the contract critic of round 2
and attacked with probes on the unmodified crate (a public-API emulator of
D1) before it was accepted. Nothing here is applied. The semantics are the
maintainer's decisions D1-D6; this file only words them.

## Recovery routes

| route | generic `T` | at the type's maximum | parent pin | kind | max_age | other frames | transform in hand | can fail |
|---|---|---|---|---|---|---|---|---|
| A. `remove_frame(frame)`, then the next insert | yes | yes | released; the next insert pins it | released; the next insert decides it | kept (registry-level) | chains through the removed edge fail (`Disconnected`, or `UnknownFrame` if the parent was named only by it) until the insert; the subtree below stays intact | none for the removal; the re-insert needs the sample (the error does not return it) | removal: no (`false` only for a frame with no edge). Re-insert: any first-insert error; `CycleDetected` only under a descendant, never under the old parent |
| B. two `reparent_frame` calls (throwaway parent, then back) | yes | yes | ends on the second seed's parent | kept | kept | between the calls the subtree hangs under the throwaway parent (`Disconnected` from the old ancestors); none after | two: a throwaway seed of the frame's kind, and the real sample; plus an unused parent name | `ParentUnchanged`, `CycleDetected` (throwaway is a descendant), `StaticDynamicConflict`, validation. Second seed's stamp is unchecked and becomes `newest` |
| C. `remove_transforms_before(c)`, `c` at or after `newest` (3.0: the cutoff removes the sample at `c`, REPORT item 12) | yes (`c = newest`) | yes | kept | kept | kept | every dynamic frame loses every sample at or before `c`; their later samples and all statics are untouched | none | cannot fail; `c < newest` silently keeps the sample (and still empties the other frames up to `c`) |
| D. wait (stream reaches `newest − max_age`) | yes | yes | kept | kept | kept | none | none | not a recovery: `SampleExpired` until then, afterwards lookups interpolate between the stream and the sample ahead until the stream passes it by more than max_age |
| E. a new `Registry` | yes | yes | all released | all released | as constructed | everything is lost, statics included | all | — |

Not routes (probed): static insert into the frame (`StaticDynamicConflict`); upsert at the poisoned stamp (replaces the
geometry, `newest` unchanged, error persists); `remove_frame` or double `reparent_frame` of an **ancestor** (the frame's
own buffer is untouched, error persists); a newer sample (moves `newest` further); `add_transforms` (same rule).
One `reparent_frame` to a new permanent parent clears the window but is a real move, not a recovery.

Name first: **A, `remove_frame`**. It is the only route named in the rustdoc that touches one frame; like C it works for every `T` and
every stamp, needs no fabricated transform or name, and its own call cannot fail. Its cost (pins released until the next
insert) is bounded by the check order: `SampleExpired` is the last check, so the rejected sample already carries the
frame's parent and kind. Name C second, as the all-frames form (a clock that stepped back affects every frame). The
inclusive cutoff of item 12 removed its two limits (no cutoff for a generic `T`, none at the maximum); its
remaining cost is its reach: with `c = newest` from one frame's error it also empties every other frame up to `c`. Leave B out of the rustdoc: it abuses a call documented as "on a decision to re-parent", needs a
fabricated transform, and its second seed is an unchecked poisoning route; keep it in the table above only.

## Text

### (a) `RegistryError::SampleExpired`

```rust
/// The sample is older than its frame's retention window, so storing it
/// would evict it in the same call. Nothing is stored.
///
/// Reported only by a registry built with
/// [`Registry::with_max_age`](crate::Registry::with_max_age), only for a
/// dynamic sample, and only when `stamp` is strictly older than `newest`
/// minus `max_age` — the boundary eviction uses. A sample exactly on the
/// boundary is stored. Where `newest` minus `max_age` is not representable
/// in the time type, nothing is older and no sample is rejected. This is
/// the last insert check: a sample that reports it has the frame's parent
/// and kind.
///
/// The error does not say which stamp is wrong. A sample that arrives late
/// and a frame whose newest timestamp is ahead of its stream report it the
/// same way, and the sample that moved `newest` ahead was accepted. Once
/// the stream reaches `newest` minus `max_age` its samples are stored
/// again, beside the ones ahead of it, and lookups interpolate between the
/// two.
///
/// From [`Registry::add_transforms`](crate::Registry::add_transforms),
/// `stamp` belongs to the first member, in order, that its frame's window
/// discards, and `newest` can be the timestamp of an earlier member of the
/// same batch, which the registry does not hold after the error.
///
/// # Recovery
///
/// [`Registry::remove_frame`](crate::Registry::remove_frame) discards the
/// frame's samples together with `newest`, for every time type and every
/// timestamp. The frame's next insert is a first insert: no timestamp
/// expires it, and it pins the parent and kind again. Until then, lookups
/// through the removed edge report
/// [`Disconnected`](Self::Disconnected) or
/// [`UnknownFrame`](Self::UnknownFrame).
///
/// [`Registry::remove_transforms_before`](crate::Registry::remove_transforms_before)
/// with `newest` as the cutoff drains the frame and keeps its parent and
/// kind, for every time type and every timestamp. It also removes every
/// dynamic sample at or before `newest` from every other frame. When
/// `newest` came from an earlier member of a rejected batch, the registry
/// never held it: draining then removes the frame's older samples and
/// does not make that batch insertable.
#[error(
    "sample for {frame} at {} is older than the retention window behind {}",
    .stamp.as_seconds_lossy(),
    .newest.as_seconds_lossy()
)]
#[non_exhaustive]
SampleExpired {
    /// The child frame of the rejected sample.
    frame: String,
    /// The rejected sample's timestamp.
    stamp: T,
    /// The frame's newest timestamp when the sample was checked.
    newest: T,
},
```

### (b) `Registry::with_max_age`

```rust
/// Creates a registry whose dynamic frames each keep a window of `max_age`
/// behind their newest timestamp.
///
/// An insert removes the frame's samples older than its newest timestamp
/// minus `max_age`. An insert that is itself older than that boundary
/// stores nothing and fails with [`RegistryError::SampleExpired`]. A
/// sample exactly on the boundary is kept, and
/// `Duration::ZERO` keeps only the newest sample and rejects every older
/// one. Where the subtraction is not representable in the time type,
/// nothing is removed and nothing is rejected. Wall-clock time is never
/// consulted, and static transforms never expire.
///
/// The window follows the timestamps inserted, and
/// [`Registry::add_transform`] never moves a frame's newest timestamp
/// back. A sample stamped ahead of its frame's stream is accepted and
/// takes the window with it; the stream's next samples fail with
/// `SampleExpired` until the stream reaches the boundary. A stream that
/// steps back by less than `max_age` is stored without an error beside the
/// samples ahead of it, and lookups interpolate between the two. See
/// [`RegistryError::SampleExpired`] for discarding a frame's newest
/// timestamp.
///
/// This bounds retained history duration, not sample or frame count.
/// Available with and without `std`.
```

### (c) `Registry::add_transforms` — delta

Replace the sentence "The error does not identify the rejected transform." with:

```rust
/// Only [`RegistryError::SampleExpired`] names the frame and timestamp of
/// the rejected transform, and more than one member can carry that pair.
```

Add after the second paragraph:

```rust
/// Under [`Registry::with_max_age`] the order of a batch decides its
/// result, as it does for the loop. Each dynamic member is measured
/// against its frame's newest timestamp at its turn, earlier members of
/// the batch included. With a 10 s window, one frame's samples at
/// `[50 s, 100 s]` are accepted, and the sample at 50 s is evicted by the
/// one at 100 s before the call returns; `[100 s, 50 s]` fails with
/// `SampleExpired` and stores neither. `Ok` means every member was stored
/// at its turn in the order; a member that a later member evicts or
/// replaces is not in the registry when the call returns, as after the
/// same sequence of [`Registry::add_transform`] calls.
///
/// On `SampleExpired`, `newest` is the frame's newest timestamp at the
/// rejected member's turn. It can be the timestamp of an earlier member,
/// which the registry does not hold after the error. Inserting one at a
/// time stores every member its frame's window does not discard and
/// reports each discarded one; it accepts a sample stamped ahead and
/// rejects the samples behind it, exactly as the batch measures them.
```

### (d) MIGRATION row

| Tag | 2.x | 3.0 |
|---|---|---|
| silent | On a `with_max_age` registry, an insert older than the frame's newest timestamp minus `max_age` returned `Ok(())` and was evicted in the same call | The insert returns `RegistryError::SampleExpired { frame, stamp, newest }` and stores nothing. Stored state after an accepted insert is unchanged. Nothing stops compiling. An insert loop that uses `?` now stops at the first late sample, and `add_transforms` rejects the whole batch for one such member, where 2.3 stored the rest. The sample that moves a frame's newest timestamp ahead is still accepted; `remove_frame` discards that timestamp |

### (e) AGENTS.md — buffer-expiry bullet

```
- Buffer expiry is data-driven and has one boundary: the frame's newest
  stored timestamp − `max_age`, computed by one `checked_sub` call (only for
  buffers built `dynamic_with_max_age`). On insert, stored entries older
  than it are removed, and an insert that is itself older is refused with
  `RegistryError::SampleExpired` and stores nothing — `Ok(())` for a sample
  its own insert discards is a false return value. A sample on the boundary
  is kept; where the subtraction is not representable, nothing is removed
  or refused. Do not restate the test through `duration_since`: a clock
  whose `duration_since` fails on ordinary spans would be told `Ok` for
  samples eviction then removes. Expiry is the last insert check, so the
  error implies the sample has the frame's parent and kind.
  `add_transforms` measures each member against its frame's running newest
  timestamp in the check phase, before anything is stored: the batch stays
  all-or-nothing and equal to the insert loop, order included. The
  `reparent_frame` seed is exempt — it replaces the history it would be
  measured against. The error is not clock-reset detection: the sample
  that moves the newest timestamp ahead is accepted, and a backward step
  smaller than `max_age` is stored silently. Wall-clock time is never
  consulted.
  Manual cleanup (`Registry::remove_transforms_before`, and the internal
  `Buffer::remove_before` under it) never touches static buffers — a static
  transform is valid for all time — and never releases a frame: a drained
  buffer keeps its pinned parent and its static/dynamic kind, so cleanup cannot
  re-open a frame for re-parenting or a change of kind. Cleanup removes
  every dynamic sample at or before the cutoff: the sample at the cutoff
  goes too, so a frame's newest timestamp is always removable, in every
  time type. A strict cutoff left a sample at the type's maximum
  unremovable, and under `max_age` it then evicted every later sample on
  arrival. Expiry is the opposite on purpose: it keeps the sample exactly
  on its boundary. A drained buffer has no newest timestamp; the cutoff
  applies to every frame.
  `Registry::remove_frame` is the only release on the cleanup path, and the
  cleanup-path way to discard one frame's newest timestamp alone;
  `Registry::reparent_frame` also discards it and releases a pin — by replacing the
  buffer wholesale, never by draining it.
```

### (f) 2.4 Part C item 5 — corrected sentences (2.x keeps the silent drop and the strict cutoff)

These sentences describe 2.x only. In 3.0 the cutoff removes the sample at it (REPORT item 12), and the
probe record below is of the unmodified crate.

Replace "A smaller jump keeps both runs, and lookups interpolate across them, as every jump does without
`max_age`. Whoever owns the clock must detect the jump and call `remove_transforms_before` with a cutoff past
every stored sample." with:

> "A smaller jump keeps both runs, and lookups interpolate across them, as every jump does without `max_age`; a
> larger jump does the same from the moment the restarted stream reaches the newest timestamp minus `max_age`.
> Whoever owns the clock must detect the jump. `remove_frame` discards one frame's samples and its newest
> timestamp, for any time type, and releases the frame's parent and kind until its next insert.
> `remove_transforms_before` does it for every frame at once and keeps their pins, with a cutoff strictly later
> than every stored dynamic sample; a cutoff equal to the newest sample keeps it, and no such cutoff exists for a
> sample at the time type's maximum."

Probes on the unmodified crate: larger jump, no cleanup: 906 inserts all `Ok`, 6 stored, first stored
stamp 1000 = newest − max_age, lookup @1005.5 = x 0 (cross-epoch); cutoff == newest: restarted sample not stored,
healthy frame keeps its newest; cutoff strictly later: stored, `other` -> `covered: None`, static kept, parent pin kept;
`remove_frame`: stored, `other` untouched; sample at `u64::MAX`: cutoff = max fails, `remove_frame` works; without
`max_age` a jump back interpolates (@500 = x 500).

Also: the `reparent_frame` rustdoc (`src/core/registry/mod.rs:663-665`) becomes "under `with_max_age`, a seed ahead of
the live stream makes later samples older than `max_age` relative to it fail with `SampleExpired`".
