# transforms v2.4.0 — internals and lookup hot path (read-only fact-finding)

Commit `adccbdd`. Paths relative to `the repository root`. `R:` = `src/core/registry/mod.rs`,
`B:` = `src/core/buffer/mod.rs`, `I:` = `src/geometry/transform/isometry.rs`, `T:` = `src/geometry/transform/mod.rs`.
Everything below was read from source unless marked **unverified**. Timings are from this machine (caveat in section 9).

## 1. Storage

- `Registry<T> { data: HashMap<String, Buffer<T>>, max_age: Option<Duration> }` (R:95-102). `hashbrown` 0.17.1 with
  its default hasher (foldhash, per Cargo.lock; not overridden). Key = **child** frame name. Roots have no entry;
  a root exists only as some buffer's `parent` string.
- `Buffer<T> { parent: Option<String>, kind: Kind<T> }` (B:50-56).
  - `Kind::Static(Option<Transform<T>>)` (B:121) — stores a whole `Transform`, i.e. translation, rotation, stamp
    **plus two more `String`s** (parent, child; T:84-92). A static frame therefore holds its parent name twice
    (pin + transform) and its child name twice (map key + transform): 4 heap strings per static frame.
  - `Kind::Dynamic { data: BTreeMap<T, Isometry>, max_age }` (B:125-128). `Isometry` = `Vector3` + `Quaternion`
    = 7 f64 = 56 B (I:22-25); with `Timestamp` (u64) key, 64 B per sample in B-tree leaves of 11.
  - `max_age` is copied into every dynamic buffer at creation (R:911-920); the registry-level field is only the
    template.
- No frame index, no depth, no child list, no root set, no interning. Every topology question is answered by
  string-keyed hash lookups walking `parent` strings upward.

## 2. Insert path (`add_transform`, R:171-176 -> `process_add_transform` R:878-907)

Steady-state dynamic insert into an existing frame:
1. Caller: `Transform::new(&str, &str, ..)` allocates 2 `String`s and validates (T:145-161). Measured 122 ns
   for construct+drop alone.
2. `data.contains_key(child)` — hash #1 (R:886).
3. `data.get_mut(child)` — hash #2 (R:894). (Two lookups for one entry; `entry`/single `get_mut` would do.)
4. `Buffer::insert` -> `check` (B:277, 318-339): `validate` again (finite checks + `q.norm()` via libm sqrt,
   T:304-325), empty-name, self-reference, parent pin compare (string eq), kind match.
5. `data.insert(timestamp, transform.isometry())` (B:289) — B-tree insert; node alloc amortized 1 per ~6-11 inserts.
6. `remove_expired` (B:476-492): `last_key_value`, `checked_sub`, loop `first_key_value`/`pop_first`.
7. The `Transform` is dropped: both `String`s freed. Net: the 2 frame-name allocations per sample exist only
   to be compared and freed.

New frame additionally: `creates_cycle` (R:887, section 6), `empty_buffer_for`, `child` String clone (R:903),
parent pin String clone (B:282), `data.insert` (hash #3, possible table growth). The buffer is filled before it
is registered so a failed first insert leaves nothing behind (R:898-905).

Static insert: stores the `Transform` itself (B:285-287); last write wins; no eviction.

Validation runs twice per sample on the common path (constructor + `Buffer::check`) — deliberate per AGENTS.md
(derived transforms cross this boundary unchecked), but the cost is paid by constructor-built ones too.

## 3. `add_transforms` atomicity (R:231-275)

Check-all-then-store, with new frames built aside:
- `checked: Vec<Transform>` and `created: HashMap<String, Buffer<T>>` (R:242-243).
- Per item: `self.data.get(child)` hit -> `buffer.check(&t)` (no storage read, B:318) and push to `checked`.
  Else `created.get_mut(child)` hit -> real `insert` into the aside buffer. Else cycle walk over
  `data ∪ created` (two hash lookups per step, R:251-256), build buffer, insert, `created.insert`.
- Commit: `for t in checked { process_add_transform(..)? }` then `self.data.extend(created)` (R:270-273).
- Soundness rests on one invariant: `Buffer::check` reads only pin and kind, which no insert changes after the
  first (B:304-313). The commit loop still uses `?`; if that invariant were ever broken the batch would be
  half-stored (the comment at R:266-269 accepts this in exchange for a single way into a buffer).
- Cost per existing-frame member: 3 hashes of the child (get, contains_key, get_mut) and `validate` + pin checks
  **twice** (check pass, then insert). The `Vec<Transform>` holds full transforms (2 Strings each) until commit.
- The error does not identify which batch member failed (documented, R:188-189).

## 4. Lookup path (`get_transform`, R:334-352 -> `resolve`, R:1080-1142)

1. `target == source` -> identity, known or not (R:1089). Still allocates the 2 result Strings (measured 51 ns).
2. **Target walk first** `get_transform_chain(target, source, ..)` (R:1097, 1216-1259): loop
   `data.get(current)` -> `buffer.parent()` -> `buffer.get(timestamp)` -> push `Hop{parent,child,isometry}`
   (88 B: two `&str` + 56 B) into a fresh `Vec`; stop early only if the walk reaches `source` (R:1253), else
   continue **to the root**, sampling every edge.
3. If the target walk did not reach `source`: source walk the same way (R:1104), stopping early only at `target`.
   If it reaches `target`, the target chain is discarded (`clear`, R:1109-1111).
4. `truncate_at_common_parent` (R:1266-1282): compare the two chains from the root end by child-name string
   equality, drop the shared suffix. No depth information exists, so the common ancestor is only discoverable
   by walking both sides to the root.
5. Tops compared by string (R:1124-1126). Unequal -> `diagnose_not_found` (section 7).
6. `combine_transforms` (R:1306-1330): fold each half front-to-back with `Hop * Hop` (each multiply runs
   `ensure_composable`: 2 string comparisons that never fail on a walk, R:68-77, T:486), invert the target half
   once (`Isometry::inverse` normalizes: libm sqrt, I:47-59), multiply. Ancestor-direction lookups invert nothing.
7. Materialize: `Transform::unvalidated(target.into(), source.into(), ..)` — 2 `String` allocations (R:345-351).

Per-hop sampling (`Buffer::get`, B:367-393): static -> copy isometry out of the stored `Transform`. Dynamic ->
`get_nearest` (B:418-436): `range(..=t).next_back()`, and unless that is an exact hit, a **second** tree descent
`range(t..).next()`; then `Isometry::interpolate` (I:72-94): two `duration_since`, `as_secs_f64` division, lerp,
slerp (libm acos/sin). Exact hit short-circuits on a zero span after one `duration_since`.

### Allocations and hashing per successful lookup
| Item | Count |
|---|---|
| Hash lookups (`&str` hashed + compared) | (hops walked on target side + 1) + (hops walked on source side + 1), counting hops **walked**, not hops on the chain |
| `Vec<Hop>` allocations | 0-2 (one per non-empty walk; first push allocates cap 4 = 352 B, doubles after) |
| Result `String`s | 2, always (also for identity) |
| Discarded `String` | 1 if any walk hit a sampling failure, even when the lookup then succeeds (R:1244-1246) |
| B-tree descents | 1 per dynamic hop on exact hit, 2 when interpolating |

### Wasted work — confirmed by reading and by measurement
- **Edges above the common ancestor are sampled and thrown away.** Sibling query `laser <- camera_optical`
  needs 3 edges (2 static, 1 dynamic) but samples 7 (4 dynamic interpolations above `base_link`, all discarded).
  Measured: 689 ns exact / 1206 ns interpolated, versus 408 / 732 ns for the 4-edge `map <- camera_optical`.
- **The documented direction pays for the target's own ancestry unless the target is the root.**
  `get_transform("camera", "camera_optical", t)` is one static hop, but the target walk first climbs
  `camera -> base_link -> odom -> map` sampling 3 dynamic edges, then is cleared (R:1109-1111). Measured 329 ns,
  versus 155 ns for a one-hop *dynamic* exact lookup whose target is the root.
- A sampling failure above the common ancestor allocates a `String` for `walk_failure` that a successful lookup
  then drops.

`get_transform_for` (R:363-372) is a pure forwarder. `get_transform_at` (R:1162-1206): 0-2 `resolve` calls
against `fixed_frame`, one `inverse`, one bare `Isometry` multiply, 2 result Strings; the walks toward
`fixed_frame` repeat any shared ancestry twice.

## 5. `latest_common_time` (R:575-650)

- Equal names -> `Stamp::Static` (R:580).
- `connecting_chain` (R:974-993): two `ancestry` walks to the root (R:954-968; one hash per step, one `Vec` each
  of `(&str, &Buffer)`), root names compared, shared suffix truncated. Topology-first, unlike `get_transform`.
- `None` -> `frame_exists` on both endpoints (O(N) scan each, section 7) -> `UnknownFrame` / `Disconnected`.
- One pass over the chain's `coverage()` (B:227-236): min of ends, max of starts with the owning frame;
  empty hop -> `NoCommonTime{covered: None}`; `max_start > min_end` -> `NoCommonTime{covered: Some}`.
- Second pass `check_interpolation_time(end)` per hop (B:398-410): 1-2 more B-tree descents each, time
  arithmetic only.
- Success allocates 2 `Vec`s, no `String`s. Measured 324 ns (4 hops, scratch) / 716 ns (criterion, 4 dynamic hops).
- This function already is the "resolve topology first, then touch data" shape that `resolve` is not; the two
  paths duplicate the walk logic (`ancestry` vs `get_transform_chain`, `connecting_chain` vs
  `truncate_at_common_parent`).

## 6. Cycle check (`creates_cycle`, R:934-947)

- Runs only when an edge enters the map: first insert of a child (R:886-887), a batch-created frame (R:251),
  `reparent_frame` (R:852). Walks upward from the new `parent`, comparing each ancestor to `child`.
- Cost: O(depth of new parent) hash lookups. Building a depth-N chain root-first is O(N^2) hash lookups in
  total (the `deep_static_chain` bench fixture, benches/benchmarks.rs:390-398, does exactly this); leaf-first
  is O(N). Not benchmarked by the suite.
- It can only fire when `child` is currently a root that is an ancestor of `parent` (a child with a buffer skips
  the check; its pin cannot change).
- Termination relies on acyclicity being maintained at every entry point; there is no hop bound.

## 7. Error diagnosis — a second pass (`diagnose_not_found`, R:1022-1065)

Triggered when the two half-chains do not meet. In order:
1. `frame_exists` for target then source (R:998-1003): `contains_key || data.values().any(parent == frame)` —
   an **O(N) scan of every buffer for any frame that is a root or unknown**. Measured:
   `not_found_unknown_frame_1k` 94 µs (1000-frame registry).
2. `connecting_chain` again (two more full ancestry walks, 2 `Vec`s), then `buffer.get(timestamp)` re-sampled on
   every chain edge until the first failure (full interpolation for the edges that succeed).
3. Fallback to the first-pass `walk_failure` only for disconnected endpoints; sampling failure takes precedence
   over `Disconnected`.
4. `GetError::Interpolation` is returned as the flat/wrapped transform error rather than `NotFoundAt`.
- Error payload allocations: `UnknownFrame` 1 String; `Disconnected` 2; `NotFoundAt` / `NoCommonTime` 3
  (R error.rs:146-214). `not_found_past_newest_1k`: 744 ns versus 240 ns for the matching success.
- The second pass exists because the first pass conflates topology and data: it cannot tell "failed above the
  common ancestor" from "failed on the chain" without knowing the common ancestor, which it only learns by
  walking past the failure. A topology-first resolve makes the first pass sufficient.
- Only the first failing edge is reported; a caller waiting on several stale edges learns of them one per call.

## 8. Mutations

- `remove_transforms_before` (R:663-670): `for buffer in data.values_mut()`; dynamic -> `split_off(&t)` and
  drop the prefix (B:444-456); static untouched; pins and kind survive a full drain. O(frames + log n + evicted).
- `remove_frame` (R:720-725): `data.remove(child).is_some()`. Descendants keep buffers whose `parent` string
  still names the removed child, which becomes a root. No cascade, no report of orphaned descendants.
- `reparent_frame` (R:827-868): `get(child)` -> root/unknown split via `frame_exists` (O(N) on that error
  path) -> `ParentUnchanged` -> `creates_cycle` against the pre-commit tree -> `old.empty_like()` (B:192-207)
  -> seed `insert` (all numeric/name/kind checks) -> one `HashMap::insert` commits and drops the old buffer
  (O(history) free). 2 String allocs (key, pin) + 3-4 hashes + cycle walk.
- Expiry (B:476-492) is per-buffer, on insert, relative to that buffer's newest key. A frame whose stream stops
  keeps its last `max_age` window forever; nothing ages it out.

## 9. Benchmarks

Machine: i7-12850HX (24 threads, hybrid P/E cores), Linux 7.0.0, rustc 1.101.0-nightly (2026-10-18),
governor `powersave`, load average 3.4 during the run, no core pinning. Treat absolute numbers as ±30% and
ratios within one table as the useful signal. `cargo bench --bench benchmarks -- --quick`, release profile
(LTO, codegen-units=1).

| Criterion bench | time (mid) |
|---|---|
| add_transform_prewarmed_1k / 60k / 100k | 314 ns / 407 ns / 382 ns |
| get_transform_1k (1 dyn hop, newest sample) | 240 ns |
| get_transform_interpolated_1k | 523 ns (range 427-547, noisy) |
| robot_tree_map_to_camera_optical (4 hops, interpolated) | 1.58 µs |
| robot_tree_leaf_to_leaf (3-edge chain, 7 edges sampled) | 2.48 µs |
| dynamic_chain_3hop_interpolated_1k | 1.42 µs |
| dynamic_chain_4hop_toward_ancestor / descendant | 1.91 µs / 2.09 µs |
| latest_common_time_4hop | 716 ns |
| get_transform_at_1k | 873 ns |
| tree_climb_1k (999 static hops) | 180 µs (~180 ns/hop) |
| tree_climb_1k_common_parent_elim | 652 µs |
| not_found_past_newest_1k | 744 ns |
| not_found_unknown_frame_1k | 94 µs |

Scratch microbench (best of 7 x 2M iterations, same machine, source at
`evidence/micro/src/main.rs`;
component rows are stand-ins for the private internals, crate rows call the public API on the robot tree):

| Component | ns |
|---|---|
| hashbrown `get(&str)` hit, 7-entry map | 17-19 |
| hash walk, 4 hops (5 dependent gets) | 162 |
| index walk, 4 hops (`Vec<u32>` parents) | 5.6 |
| 2x `String::from` + drop (result frames) | 60 |
| `Vec<[u8;88]>` push x4 + drop (hop chain) | 54 |
| `[[u8;88];8]` on stack, fill x4 | 68 (no win as written; zero-init of 704 B dominates) |
| `Transform::new` + drop | 122 |
| nearest-pair, exact newest, n=1k: BTreeMap / VecDeque | 60 / 48 |
| nearest-pair, bracketing, n=1k: BTreeMap / VecDeque | 112 / 46 |
| nearest-pair, bracketing, n=60k: BTreeMap / VecDeque | 125 / 54 |
| append newest + evict oldest, n=1k: BTreeMap / VecDeque | 129 / 11 |
| append newest + evict oldest, n=60k: BTreeMap / VecDeque | 209 / 16 |

| Crate call (robot tree, 1k samples per dynamic edge) | ns |
|---|---|
| same frame (identity) | 51 |
| 1 dynamic hop, exact, target is root | 155 |
| 1 dynamic hop, interpolated | 285 |
| 1 **static** hop, target not root (`camera <- camera_optical`) | 329 |
| `map <- camera_optical`, exact / interpolated (4 hops) | 408 / 732 |
| `laser <- camera_optical`, exact / interpolated (3-edge chain) | 689 / 1206 |
| `latest_common_time(map, camera_optical)` | 324 |

Reading: on a one-hop exact lookup (~155 ns), the 2 result Strings (~60 ns) and the hop `Vec` are the largest
fixed costs; slerp adds ~130 ns per interpolated dynamic hop; hashing is ~17-40 ns per hop. Why `tree_climb_1k`
costs ~180 ns per static hop is **unverified** (candidates: cache misses across 1000 scattered buffers and their
heap strings, `Vec<Hop>` regrowth, per-hop `ensure_composable`); not profiled.

## 10. Internal redesign candidates

Effect estimates are for this machine; "measured" means a number above supports it, not that the redesign was
built.

1. **Topology-first resolve** (walk frames to find the common ancestor, then sample only the chain's edges).
   Removes sampling above the common ancestor and of the target's ancestry, removes the discarded
   `walk_failure` String, makes `diagnose_not_found`'s second pass unnecessary, and lets `get_transform` and
   `latest_common_time` share one chain resolver. *Measured waste*: 329 ns vs ~100 ns expected for the static
   hop; 689 vs 408 ns for a shorter chain. Needs no new data structure. Behavior change to settle: today a
   sampling failure outranks `Disconnected` (R:1012-1017); topology-first would naturally report topology first,
   as `latest_common_time` already does.
2. **Interned frame ids with the parent id stored inline** (`Vec<Frame>` + `HashMap<String, FrameId>` used once
   per endpoint). Walks become index chases; roots become real entries, so `frame_exists` is O(1) instead of an
   O(N) scan and `UnknownFrame` vs root needs no scan. *Measured*: 162 ns -> 5.6 ns for a 4-hop walk; the 94 µs
   unknown-frame diagnosis disappears. Open design points: `remove_frame` needs tombstones or generation
   counters; whether ids are public (a handle API) or purely internal.
3. **Depth stored per frame.** Gives the common ancestor by the level-then-climb-together walk in O(chain) with
   no allocation and no root walk. `reparent_frame`/`remove_frame`/attaching a parent to an existing root must
   update the depths of the whole subtree — O(subtree), which needs child lists or a scan. *Unmeasured.*
   Alternative without depth: walk both ancestries as ids into two fixed buffers (what `ancestry` does today
   with `Vec`s); costs O(depth) but ids are cheap. Depth is an optimization on top of 2, not a prerequisite.
4. **Sorted `VecDeque<(T, Isometry)>` instead of `BTreeMap`.** *Measured*: bracketing lookup 112 -> 46 ns
   (one binary search yields both neighbors; BTreeMap needs two descents), in-order append + evict 129 -> 11 ns
   (n=1k), contiguous memory, no per-node allocation. Costs: out-of-order insert is O(n) memmove (rare in
   practice, pathological for a backfilling publisher); `remove_before` becomes `drain(..i)`; upsert at an
   existing stamp is a binary search + overwrite. Unmeasured: the out-of-order worst case.
5. **No heap for the hop chain.** With 1, composition can fold while walking: accumulate one `Isometry` per
   side, so no `Vec<Hop>` at all and no `Hop` type. *Partly measured*: the `Vec` costs ~54 ns per walk; a naive
   fixed stack array did not beat it (68 ns), so the win comes from not materializing hops, not from moving
   them to the stack. Also drops the per-hop `ensure_composable` string comparisons, which guard nothing a
   walk can violate.
6. **Result without `String` allocation.** *Measured*: ~60 ns of every lookup, 39% of a one-hop exact lookup,
   100% of the identity case. Options: return bare geometry plus the stamp and let the caller hold the frame
   names it already passed in; a result type borrowing the caller's `&str`; or frame ids in results. This is an
   API decision (what a `Transform` is), not only an internal one. The same applies to inserts: 2 Strings are
   allocated per sample and freed after two comparisons (122 ns measured for construct+drop of ~314 ns insert).
7. **Prepared chain handle for repeated lookups** (resolve endpoints once, re-sample per call). Removes endpoint
   hashing and the ancestor search per call. *Unmeasured*; after 1+2+5 the remaining per-call topology cost is
   ~2 endpoint hashes (~35 ns) plus an index walk (~6 ns), so the handle saves little unless it also caches
   composed static runs. It adds an invalidation problem (reparent/remove need a topology generation counter)
   and public surface. Weakest case on the list by the crate's own "demonstrated need" rule.
8. **Statics out of the time path.** Store a static edge as a bare `Isometry` (drops 2 redundant Strings and
   the dead stamp per static frame — currently a full `Transform`, B:121); optionally make kind a field of the
   frame record so the sampler branches once. *Unmeasured*; small per-hop gain, real memory/clarity gain.
   Pre-composing runs of consecutive static edges is a further step with invalidation cost; unmeasured.
9. **Single hash on insert.** `contains_key` + `get_mut` (+ `get` in batches) -> one lookup. *Measured unit cost*
   ~17 ns per avoided hash; with 2, an insert by id needs none.
10. **Validate once.** Constructor-built transforms are validated twice per insert (three times in a batch).
    A type-level "validated" marker would remove it but AGENTS.md states the storage check must stay for derived
    transforms; only worth doing if derived and built transforms become distinguishable in the type system.
    *Unmeasured* (one libm sqrt + 7 finite checks; expected < 10 ns).
11. **Cycle check cost.** With 2, still O(depth) per new edge, but index chases. With 3, a new leaf under any
    parent needs no walk at all (a frame with no children cannot be an ancestor of anything). *Unmeasured.*
12. **Report every failing edge / the failing batch index.** Diagnostics, not speed: `NotFoundAt` names one edge
    per call and `add_transforms` does not say which member failed (R:188-189). With 1, collecting all failing
    chain edges is one pass. *Unmeasured*; payload-shape decision.

Dependencies between candidates: 1 is independent and removes the measured waste alone. 2 enables 3, 9, 11 and
most of 7. 5 presupposes 1. 6 and 10 are public-API questions. 4 and 8 are local to `Buffer`.
