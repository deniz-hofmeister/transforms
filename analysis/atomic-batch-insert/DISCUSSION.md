# Atomic batch insertion (PR #123): discussion record

Date: 2026-10-26. Baseline: master `0fce128` (v2.2.0 plus docs). Subject:
[PR #123](https://github.com/deniz-hofmeister/transforms/pull/123),
"Add atomic insertion of transform batches" (TomCC7, draft, written with Codex).

This record is a handover. It covers the analysis, the prototypes, and where
the discussion stands. **No decision has been made and nothing has been
posted on the PR.** It does not authorize implementation or merge.
AGENTS.md applies throughout.

## 1. Summary

| Option | Verdict |
|---|---|
| Merge PR #123 as written | **Never.** It clones the whole registry on every call. |
| **Decline the feature for now** | **Current lean, about 60/40.** Reply with reasons and conditions. Put frame names in insert errors on the v3 ledger. |
| A: `Registry::add_transforms`, check then commit | **Ship this if implementing**, built on a crate-private B (§6). |
| B: `Transaction` handle | Keep it crate-private. Make it public only for a demonstrated need for multi-step tree edits (§7). |
| C: read-only dry run `check_transforms` | No. A does everything C does, and C's atomicity depends on the caller holding one lock. |
| D: raw `parent` / `frame_kind` queries | No. They expose raw state, and the natural caller validator was wrong (§5). |

What would flip the lean to A:
- a concrete case where partial application produced a wrong pose, or
- a second, independent request.

## 2. The PR and its context

- **What it does.** `add_transforms(impl IntoIterator<Item = Transform<T>>) -> Result<(), RegistryError<T>>`
  is all-or-nothing. It clones `self.data`, applies the batch to the clone,
  and swaps the clone in on success. It adds `#[derive(Clone)]` on the private
  `Buffer` and `Kind`. Its one test covers a cyclic batch with an eviction
  rolled back.
- **Who needs it.** DimOS
  [#4722](https://github.com/dimensionalOS/dimos/pull/4722) (open, same
  author) pins the fork at `5ae9563`. It does so in Cargo
  (`version = "=2.2.0", git = …, rev = …`) and in a Nix lock with a sha256.
  #4774 was closed as superseded by #4722, and #4722 is part of a 12-PR stack
  (#4799).
- **Verified from the #4722 diff only.** This is high confidence for that
  PR as it stands, not for DimOS as a whole.
  - `Graph { buffer: RwLock<Registry<SystemTime>> }`, built with `with_max_age(window)`.
  - **Receive:** decode, then `to_frame` per edge, which builds each edge
    with `Transform::new` and so validates the numbers. The whole message is
    collected into a `Result` first, then `add_transforms`. A rejection is
    logged once with `warn!`.
  - **Publish:** reserve the queue permit, then `graph.insert`, then send. The
    docs promise "Invalid batches do not change the graph or enqueue a packet."
  - **The old receive path** skipped and logged each bad edge (per-edge `continue`).
  - **Not on crates.io.** `dimos-module`, `dimos-tf-messages` and `dimos` do
    not exist there, so the git dependency is not a publishing blocker.
- **Inferred, not verified.** That `Graph` is the only writer to its
  registry, that DimOS never calls `remove_frame` or `reparent_frame`, and how
  their deployment looks. These rest on a text search of one diff in a stack.
- **tf2.** `tf2_ros/src/transform_listener.cpp` `subscription_callback`
  inserts each transform separately, catching and logging every failure.
  This is context only; AGENTS.md bars it as a justification.

## 3. Is the concept useful?

**Facts that hold:**
- **It cannot be built outside the crate, for a general caller.**
  `Registry` derives only `Debug` (not `Clone`), there is no way to remove a
  single sample, eviction cannot be undone, and the stateful checks
  (parent, kind, cycle) need sealed internals.
- **Correction:** a *single-writer wrapper* can build exact all-or-nothing
  outside the crate. It tracks each child's parent and kind as inserts
  succeed, plus an overlay for frames the batch itself creates, then checks
  before inserting. Cost: ~50 lines that duplicate the crate's rules (see
  Design D: three iterations to get right). Divergence after a future crate
  rule change fails loudly, not silently.
- **Visibility to readers is not an argument.** `&mut self`, or one write
  guard, already hides a half-applied loop.
- **After numeric validation, the only rejections left are tree-configuration
  errors.** That means re-parenting, a static/dynamic conflict, a cycle, or a
  frame named as its own parent. All-or-nothing does not prevent these; it
  decides whether valid sibling transforms are dropped too.

**For:**
1. It is a "core or nowhere" capability with a real call site.
2. It is cheap when done right (§4).
3. It closes two partial-application hazards:
   - **A static group ending up half old, half new.** If a recalibration
     publishes several static edges and only some are accepted, the chain
     mixes old and new values silently, for all time.
   - **Frames pinned by a rejected message.**
4. Adoption: DimOS is a large project.

**Against, and why it weighs more:**
1. **It is a policy preference, not an invariant.** AGENTS.md's test is
   whether the outside substitute is "merely verbose" or a "wrong-answer
   generator". The substitute here is the per-item loop, and it is neither:
   every edge it applies is individually valid, and missing samples fail
   loudly.
2. **It hurts availability.** A persistent configuration error on one edge
   turns into a permanent freeze of everything that producer sends, logged as
   one warning that names no frame.
3. **It is a permanent commitment.** It freezes "first error, no culprit".
   Internally, checks on an existing frame must never depend on state that
   inserts change, which constrains future rules such as rejecting
   out-of-order stamps.
4. **The evidence is one consumer**, the PR author's own project. No
   comparable library found offers it. schiebung's batch API is documented as
   non-atomic; re_tf, cu-transform and tf_rosrust are unverified.

**Arguments overweighted earlier:**
- **Static groups:** these need the producer to change its configuration
  between messages. Real, but narrow.
- **Sticky pins:** all-or-nothing only protects against a *rejected*
  message. A valid but wrong message pins a frame just as permanently.
- **"Eviction loses data":** the samples evicted were pushed out by a newer
  sample that was itself valid. That counts as harm only if the whole
  message is assumed untrustworthy, which is the policy under debate.

**Note on framing.** "This is a problem with their own tree construction" is
mostly true. The remaining failures are configuration errors, and on the
publish path the data is entirely their own. The nuance: in a system with
many modules, a receiver does need a policy for bad input from other
producers. That policy belongs in the application. The library already
rejects each bad edge with an error.

## 4. Cost

Notation:
- b: transforms in the batch (b_new of them create a new frame)
- F: frames in the registry
- n: samples in the buffer being touched
- N: samples stored across all buffers
- d: depth of the frame tree
- e: samples evicted by `max_age` (amortized O(1) per insert)

| | Time per call | Extra memory | Grows with stored history? |
|---|---|---|---|
| `add_transform` loop | O(b·log n + b_new·d + e) | O(1) | No |
| A | same | O(b) | No |
| B | same | O(b) | No |
| PR #123 | **Θ(N + F)** + the loop's cost | **Θ(N + F)**, about twice the registry at peak | **Yes** |

- **PR #123:** the clone is paid on every call, even for an empty batch or a
  single transform. Freeing the old registry, or the clone on failure, is
  another Θ(N + F). It all happens under the caller's write lock, so readers
  stall.
- **On microcontrollers:** the PR still compiles for the bare-metal targets.
  The transient doubling of heap use is what makes it unsuitable there, not
  broken.

Measured, release build, x86. Batch of all F frames at a new timestamp, with
`max_age` set so samples are evicted (`prototypes/batch_bench.rs`):

| F × samples | Loop | PR #123 | Check-then-commit prototype |
|---|---|---|---|
| 10 × 100 | 0.84 µs | 8.9 µs | 1.05 µs |
| 30 × 1,000 | 3.3 µs | 341 µs | 4.1 µs |
| 60 × 2,000 | 6.7 µs | 2,333 µs | 8.4 µs |

- The PR is 10–350× slower than the loop; the prototype about 1.2×. An earlier
  independent run gave the same picture: 10× / 156× / 201×.
- Single `add_transform` calls were not measurably slower with `Buffer::insert`
  split into check and commit. That was measured on a noisy machine.

## 5. Designs prototyped and reviewed

The designs were checked in two rounds:
- **Prototype round:** each design built and tested in its own worktree, with
  `cargo test` and clippy (`-D warnings`) passing in both feature modes and
  `thumbv6m` or `thumbv7em` building.
- **Review round:** a separate adversarial reviewer per design.
- **Not run for any design:** the full `tests/test_all.sh` gate.

Diffs are in `prototypes/`.

**Shared mechanism** (`check-commit-prototype.diff`, +102/−34):
- `Buffer::insert` becomes `check(&self)` followed by `commit(&mut self)`.
- Existing frames are checked against the live buffer. A frame's parent and
  kind never change on insert, so that check holds for the whole batch.
- New frames are staged as real `Buffer`s in a side `HashMap`, through the
  ordinary `insert`.
- The cycle walk goes through a `parent_of` closure that looks at the live
  registry and then the staged frames.
- Commit applies the existing-frame transforms in order, then
  `extend(staged)`.
- **Equivalence property test** (`batch_equivalence.rs`, 20,000 cases): on
  success, same state as the loop; on failure, same first error and unchanged
  state.
  - It covers random topologies, static/dynamic mixes, `max_age` 0–3,
    self-reference, cycles, duplicate children, and overflowed derived
    transforms.
  - It passes on both the PR and the prototype, so their behaviour is
    identical.
  - Removing the staged-frame lookup from the cycle walk made it fail at once.

### A: `add_transforms` (`design-a.diff`)

```rust
registry.add_transforms(edges)?; // on Err nothing was applied or evicted
```

- **Public surface:** one method.
- **Atomicity cannot be broken by callers.**
- **Review findings (sound with fixes):**
  - **Panic hazard in the hardened variant.** It moved buffers out of the map
    during staging, so a panic in the caller's iterator *deleted frames*
    (reproduced with `catch_unwind`). Staging must not modify the registry.
  - **Silent drop.** The prototype's `commit` mismatch arm `_ => {}` and the
    `get_mut` miss drop a transform silently. Return an error instead: loud,
    though no longer atomic, and unreachable today.
  - **Doc wording.** The iterator is consumed up to the first error, not fully.

### B: `Transaction` (`design-b.diff`, +395/−36)

```rust
let mut tx = registry.transaction();
for (i, t) in batch.into_iter().enumerate() {
    if let Err(e) = tx.add_transform(t) { warn!(index = i, %e); return; } // drop = discard
}
tx.commit();
```

- **Public surface:** one type with a lifetime, plus three methods.
- **Gain:** callers know which item failed, without changing `RegistryError`.
- **Misuse risks:** atomicity relies on caller discipline.
  - `let _ = tx.add_transform(..)` followed by `commit()` applies part of a batch.
  - A forgotten `commit` silently discards the batch. `#[must_use]` misses `let _tx`.
- **Speculative extra:** it also allows a "skip bad items" mode that nobody
  asked for.
- **Two check paths.** Routing `add_transform` through the transaction was
  about 35% slower for single inserts (noisy measurement), so two check paths
  remain, held in sync by property tests.

### C: `check_transforms(&self, &[Transform])` dry run (`design-c.diff`, +371/−34)

- **It needs the same internals as A.**
- **Atomicity is only a documented promise.** A test demonstrated the race:
  check under one guard, a writer modifies the registry, add under another
  guard, and the batch is partially applied.
- **Conflicts with AGENTS.md:** "invariants enforced at the API boundary
  rather than promised in documentation".

### D: `parent()` / `frame_kind()` + `FrameKind` (`design-d.diff`, library +55)

- **The test:** a caller-side all-or-nothing validator, written from the docs
  only, checked against the loop.
- **Iterations:**
  - v1 failed both properties: a duplicate child (dynamic then static) was
    accepted, and a batch was partially applied.
  - v2 made the right accept/reject decision but returned the wrong error. For
    a new frame the cycle check precedes numeric validation, and that order is
    undocumented.
  - v3 was correct.
- **Other problems:**
  - It exposes raw state, contrary to AGENTS.md: "never the raw state", and
    "never resolve it by exposing the internals instead".
  - `parent() == None` means either a root or an unknown frame.
- **Conclusion:** the hard part is the state that builds up inside the batch.
  More introspection does not help with it.

## 6. If implementing: recommended shape

```rust
pub fn add_transforms(&mut self, transforms: impl IntoIterator<Item = Transform<T>>)
    -> Result<(), RegistryError<T>>
{
    let mut tx = self.transaction(); // crate-private Transaction (B)
    for t in transforms { tx.add_transform(t)?; } // drop on error = nothing applied
    tx.commit();
    Ok(())
}
```

Why A over B as the public API:
- **Callers cannot get it wrong.**
- **Smallest commitment.**
- **It matches the demonstrated need exactly.**
- **AGENTS.md: write the version without the new abstraction first.**
- **The scope rule rules out shipping both.** With B public, A is five lines
  any caller can write, so it would belong outside the crate.
- **B's real advantage**, naming the failing transform, also comes from frame
  names in insert errors (v3).

Requirements:
- **Staging never modifies the registry.**
- **`commit` returns an error rather than silently dropping** on an
  impossible mismatch.
- **Commit the equivalence property test** in `tests/properties.rs`.
- **Rustdoc states:**
  - the failure semantics: the first error in order, and the registry
    unchanged on failure;
  - the availability trade-off: one bad item drops the whole batch;
  - that the error does not name the failing item;
  - that the iterator is consumed up to the first error.
- **Release work:** CHANGELOG entry, maintainer note for the public surface
  growth, full gate.

What freezes:
- **A's contract:** first-error order, no index or culprit, equivalence with
  the loop.
- **Robust for add-only batches.** Adds never remove edges or change an
  existing frame's parent or kind, so a batch of adds is valid as a whole
  exactly when it is valid item by item. Only which error is reported can
  differ.

## 7. B's possible future

Keeping B private blocks nothing. Making it public later and adding methods
are minor-release changes. Expansions, by value:

1. **Edge reversal.** `tx.remove_frame("b"); tx.add_transform(b → a)`. This
   is impossible today without a window: `reparent_frame` fails because `a`
   is a root, and adding first creates a cycle.
   `reparent-frame-value/DESIGN.md:508` already lists "atomic edge reversal or
   multi-edge edits" as a separate family.
2. **Several re-parents at once**, including moves that are acyclic only as a
   whole.
3. **Subtree replacement:** map switch, relocalization, URDF reload.
4. **Kind change with rollback.**

Little or no value:
- reads inside a transaction (costly, because interpolation would have to work
  across committed and pending samples);
- `remove_transforms_before` (it cannot fail);
- savepoints or nesting (that is a database).

Cost of expanding: removes or re-parents inside a transaction break the "a
frame's parent and kind never change while it is open" assumption. The
staging area then becomes a full overlay, with each frame live, removed
(tombstone), replaced, or new. Do not build that overlay speculatively. Each
expansion needs its own demonstrated need: "No addition is precedent for its
neighbors."

## 8. Database analogy

The registry behaves like an in-memory time-series store with integrity
constraints:

| Database concept | In this crate |
|---|---|
| Schema / types | A frame's static/dynamic kind, fixed by the first insert |
| Integrity constraints | Fixed parent, acyclicity, no self-parenting, valid numbers |
| Statement atomicity | Option A |
| Explicit transactions | Option B |

Atomicity techniques used by databases:

| Technique | Example | In this crate |
|---|---|---|
| Shadow copy | LMDB's copy-on-write B-tree, copying only touched pages | PR #123, but naively copying everything |
| Undo log | SQLite rollback journal, InnoDB undo logs | Considered, rejected as more complex |
| Deferred update | Collect, check, apply at commit | Options A and B |

Telemetry-ingest systems tend to the opposite of all-or-nothing:
- **InfluxDB 3:** `write_lp` defaults to `accept_partial=true` and reports
  errors per line. That `false` rejects the whole batch is stated only in an
  AWS Timestream guide.
- **Elasticsearch `_bulk`:** reports per-item results and is non-atomic. Only
  third-party sources were found.

Lessons:
- **Per-item acceptance is the norm**, and all-or-nothing is opt-in.
- **Good ingest APIs name the failing item.**
- **Non-Goals:** they do not say "not a database". Whether to add it is open.

## 9. Impact on DimOS of declining

**Nothing breaks.** Their fork is pinned and checksummed, they do not
publish to crates.io, and #4722 has not merged.

Their options:
1. **Stay on the fork.** The cost is the Θ(N + F) implementation (they could
   swap in check-then-commit) and rebasing on every upstream release.
2. **Per-edge loop.** This is their previous behaviour. Numeric errors are
   still rejected whole. The publish-path promise would need weakening for
   parent, kind and cycle errors.
3. **Single-writer wrapper with their own pre-check.** This is exact (§3),
   assuming `Graph` really is the only writer, which is not verified.

## 10. Draft PR reply (not posted)

The first draft asserted DimOS internals and was withdrawn as overreaching.
Current draft:

> Thanks for this PR, and for considering transforms for DimOS. I've decided
> not to merge it in this form, and I'd rather explain why than leave it
> sitting as a draft.
>
> The clone-and-swap copies the whole registry on every call, so its cost
> grows with the stored history rather than the batch size. That part is
> fixable. The harder question for me is the semantics. In this crate, a
> transform that gets past construction can only be rejected for
> tree-structure reasons: a changed parent, a static/dynamic conflict, or a
> cycle. All-or-nothing doesn't prevent those. It decides whether the valid
> transforms in the same batch are dropped along with the bad one. I see that
> as a policy for the application to choose, and I'd like the library to stay
> at validated per-transform insertion.
>
> I may be missing context about your use case, though. Could you describe
> the failure you saw, or expect to see, where partial application caused a
> problem? A concrete case like that would change my view. I'm also happy to
> help work out how to get the behaviour you need on top of the existing API.
> Separately, adding frame names to insertion errors is on my list for the
> next major release, which should make rejected transforms easier to
> diagnose.

## 11. Open decisions

1. Decline the feature, or ship A on a private B.
2. Whether to offer active help to DimOS, such as a PR to their repo with the
   validator. Only offer it if you mean it.
3. v3 ledger: frame names in insert errors (`CycleDetected`,
   `StaticDynamicConflict`, …). This is breaking, because unit variants become
   variants with fields. Worth doing whatever is decided here.
4. If shipping A: the exact error-order wording in the rustdoc, which
   becomes a semver contract.
5. Whether "not a database" belongs in the Non-Goals.

## 12. Artifacts

- `prototypes/`:
  - `pr123.diff`;
  - `check-commit-prototype.diff`;
  - `design-{a,b,c,d}.diff`;
  - `batch_equivalence.rs` (place it in `tests/` to run);
  - `batch_bench.rs` (place it in `examples/` to run).
- Local branches `pr-123` and `scratch/batch-check-first`, with worktrees in
  the session scratchpad. Eight workflow worktrees under `.claude/worktrees/`.
  All are disposable.

Sources:
- [PR #123](https://github.com/deniz-hofmeister/transforms/pull/123)
- [DimOS #4722](https://github.com/dimensionalOS/dimos/pull/4722)
- [tf2_ros transform_listener.cpp](https://github.com/ros2/geometry2/blob/rolling/tf2_ros/src/transform_listener.cpp)
- [PostgreSQL SET CONSTRAINTS](https://postgresql.org/docs/18/sql-set-constraints.html)
- [InfluxDB 3 write_lp](https://docs.influxdata.com/influxdb3/core/write-data/http-api/v3-write-lp/)
- [Timestream for InfluxDB 3](https://docs.aws.amazon.com/de_de/timestream/latest/developerguide/writing-data-to-your-influxdb-3-cluster.html)
- [Elasticsearch bulk partial failure (third-party)](https://pulse.support/kb/elasticsearch-bulkrequestexception-bulk-request-partially-failed)
