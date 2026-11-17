# Stress test: hot-path prototype of draft section 2 vs transforms 2.4.0

Scratch crate: `evidence/proto-hotpath`
(`src/proto.rs` 359 lines = the prototype; `src/lib.rs` = adapters + workloads; `src/bin/diff.rs`, `src/bin/allocs.rs`, `benches/hot.rs`; raw criterion output `run1.txt`, `run2.txt`, `run3.txt`).
Baseline: transforms 2.4.0 as a path dependency (master `adccbdd`), default features. Repository working tree untouched (`git status --short` empty afterwards).

## 1. What was built

- `Vec<Slot { name: Box<str>, parent: u32, edge: None | Static(Iso) | Dynamic(H) }>` + name index.
- Lookup: two index probes, depth of both by walking to the root, `Disconnected` if roots differ, lift the deeper side, step both until they meet; **only connecting edges are sampled**, folded as 2.4 does (`hop * acc`, first hop taken verbatim), at most one inversion, one finiteness check at exit. Returns a `Copy` 56-byte `Iso`, no labels.
- History `H`: `BTreeMap<i64, Iso>` (same two `range` probes as 2.4's `get_nearest`) or sorted `VecDeque<(i64, Iso)>` (`partition_point`; insert = push_back fast path, else `binary_search` + `VecDeque::insert`).
- Index `I`: `hashbrown::HashMap<Box<str>, u32>` (default foldhash, as 2.4) or `alloc::BTreeMap<Box<str>, u32>`.
- Geometry copied from 2.4 `src/geometry` (quaternion product, `rotate_vector` as two full products, slerp incl. nlerp shortcut, libm `sqrt/sin/acos`, ratio via `Duration::as_secs_f64`). **Inverse is 2.4's normalize-then-conjugate, not the draft's bare conjugate**, and there is no Newton renormalization in `*` — the numerics of draft section 3 are NOT what was measured; only section 2 structure is.

### Semantics the prototype has
Insert: finiteness + unit-norm check (1e-6), empty name, self-parent, parent pin (string compare against the pinned slot's name), static-xor-dynamic, cycle check when an existing root gains a parent, `max_age` eviction (data-driven). Lookup: `UnknownFrame` (target first) > `Disconnected` > `NoData`/`OutOfRange`, non-finite exit check.

### Semantics the prototype OMITS (do not over-read the numbers)
- `InsertError::Expired` (draft) / any stale-insert rule; errors carry no frame payload on insert.
- Batch insert and its atomicity; `reparent`; `remove_frame`; `remove_samples_through`; slot reclamation and the `children` count; `latest_common_time`; `lookup_across`.
- Unit-by-construction `Rotation`, `Edge<'a>`, `Timestamp` newtype, `Stamp` — inputs are raw `(&str, &str, Iso, Option<i64>)`.
- 2.4's `TimeError` paths (span not representable as `Duration`): the prototype casts `i64` differences to `u64` unchecked; fixtures are non-negative and in range.
- serde, `no_std` build, generic `T: TimePoint` (the prototype is monomorphic `i64`; 2.4 is measured with its default `Timestamp(u64)`).
None of the omitted items sits on the lookup path except the time-span checks (2 checked subtractions per interpolated hop in 2.4). On insert, `Expired` would add one compare; slot `children` bookkeeping only touches first inserts.

## 2. Differential test

Command: `CARGO_TARGET_DIR=$P/target cargo build --release --offline --bins && ./target/release/diff` (seed `0xC0FFEE`, splitmix64; 2000 random forests of 2..41 frames, 10% of frames start a new root, 30% static edges, dynamic edges hold 1..20 samples at random stamps in [0, 1e9) ns, translations in [-10, 10], uniform-ish unit quaternions; 500 queries per forest = 1,000,000 queries, incl. unknown names and out-of-range times). Identical output for all four variants:

| | count |
|---|---|
| both `Ok` | 372,996 |
| max abs translation difference | **0.0** |
| max abs rotation-component difference | **0.0** |
| single-hop child->parent lookups at a stored stamp or static edge | 27,069 |
| ...bit-identical (`to_bits` on all 7 components) | **27,069 / 27,069** |
| both `Err`, same category and same named frame | 549,771 |
| named-frame disagreements | 0 |
| 2.4 `NotFoundAt{covered: Some}` vs prototype `Disconnected` | 73,889 |
| 2.4 `Ok(identity)` vs prototype `UnknownFrame` (target == source, name stored nowhere) | 3,344 |

Reading:
- Numerically the prototype is not merely within 1e-12: every successful result has zero absolute difference (the `Ok` comparison is `abs` difference, so `-0.0` vs `0.0` would pass; the single-hop subset is checked bitwise). This is expected, since arithmetic and fold order were copied; it says the *restructuring* (topology-first, interning, deque) costs no bits. It says nothing about the draft's conjugate inverse / Newton step, which were not implemented.
- The 73,889 category differences are all frames in different trees where 2.4's data-first walk hits a time gap before it learns the trees are disjoint. That is the intended `Disconnected > OutOfRange` precedence; it is a behaviour change relative to 2.4, verified here to be the only one in error category.
- The first cut of the prototype reported the first failure in *lift order* (deeper side first) and disagreed with 2.4 on the named frame in 19,740 queries. Getting "target side first, lowest edge" needs a **second walk on the error path** (`diagnose`, `#[cold]`, allocation only for the error's `String`). So the draft's "one pass ... the second diagnosis pass disappears" is true for the success path only; with the stated precedence a re-walk on failure remains (cheap: index chases + re-sampling connecting edges). With it, 0 frame disagreements.
- Same-name query on an unknown frame: 2.4 returns identity, the prototype (following the draft's precedence) returns `UnknownFrame`. The draft does not state this case; it needs a ruling.

## 3. Allocations

Command: `./target/release/allocs` (counting `GlobalAlloc`, 1000 calls per workload; a constant 2 allocations per 1000 from the label `format!` inside the measured window has been subtracted below).

| workload | 2.4 allocs/op (bytes) | prototype, all variants |
|---|---|---|
| 1-hop exact / interp, 4-hop both directions | 3 (354 B) | 0 |
| robot map->camera_optical | 3 (369 B) | 0 |
| robot leaf-to-leaf | 4 (723 B) | 0 |
| siblings under 16-deep dynamic chain | 10 (10.6 kB) | 0 |
| static siblings, 1k-deep elimination | 22 (720 kB) | 0 |
| static climb 1k | 11 (180 kB) | 0 |
| unknown frame, 1k frames | 10 (180 kB) | 1 (7 B, the error `String`) |
| one step past newest | 5 (100 B) | 1 (8 B) |
| steady insert, sample pre-built | 0.167 (122 B) | BTree history 0.167 (122 B); deque 0 |
| steady insert incl. building the sample | 2.17 | BTree 0.17; deque 0 |

(2.4's 3 per successful lookup = 1 hop `Vec` + 2 result `String`s. Deque steady state allocates nothing once capacity is reached; it would reallocate-and-copy on growth, which this run does not exercise.)

## 4. Timings

Command: `CARGO_TARGET_DIR=$P/target taskset -c <6|2|6> cargo bench --offline --bench hot` (criterion 0.7.0, resolved offline; `lto = true`, `codegen-units = 1`; read benches 100 samples x 1.5 s, out-of-order benches 20 samples x 0.6 s via `iter_custom`). rustc 1.101.0-nightly (b6eeb93c9 2026-10-18).

**Machine caveat:** i7-12850HX laptop, `powersave` governor, turbo on, other workloads running (load average 13 at run 1, 3.3 at runs 2-3). Absolute numbers moved by up to 2x between runs (see the three columns); only within-run ratios are usable, and ratios below ~1.3x are inside the noise. Three full runs are given as r1/r2/r3 (criterion median, ns); bold = 2.4 time / prototype time per run (>1 = prototype faster).

| workload | v2.4 ns (r1/r2/r3) | proto-btree-hash ns (r1/r2/r3) | proto-btree-btreeidx ns (r1/r2/r3) | proto-deque-hash ns (r1/r2/r3) | proto-deque-btreeidx ns (r1/r2/r3) |
|---|---|---|---|---|---|
| 1hop_exact_1k | 351.2/349.0/181.4 | 160.7/212.6/120.2 → **2.19/1.64/1.51x** | 124.5/165.9/76.8 → **2.82/2.10/2.36x** | 167.5/282.1/156.5 → **2.10/1.24/1.16x** | 123.3/219.6/114.2 → **2.85/1.59/1.59x** |
| 1hop_interp_1k | 497.6/633.8/295.7 | 330.5/442.9/237.3 → **1.51/1.43/1.25x** | 279.2/435.5/220.5 → **1.78/1.46/1.34x** | 280.0/510.0/250.8 → **1.78/1.24/1.18x** | 251.4/428.5/246.3 → **1.98/1.48/1.20x** |
| 4hop_dyn_to_ancestor | 1029/1781/995.0 | 1140/1679/886.9 → **0.90/1.06/1.12x** | 1069/1677/835.4 → **0.96/1.06/1.19x** | 1007/1775/903.4 → **1.02/1.00/1.10x** | 1002/1754/943.6 → **1.03/1.02/1.05x** |
| 4hop_dyn_to_descendant | 1280/2163/1120 | 1316/1978/986.5 → **0.97/1.09/1.13x** | 1193/1858/931.2 → **1.07/1.16/1.20x** | 1081/1862/997.0 → **1.18/1.16/1.12x** | 1065/1997/1051 → **1.20/1.08/1.06x** |
| robot_map_to_optical | 1067/1537/766.5 | 905.3/1363/722.7 → **1.18/1.13/1.06x** | 882.1/1375/688.0 → **1.21/1.12/1.11x** | 768.7/1397/729.7 → **1.39/1.10/1.05x** | 812.3/1478/745.5 → **1.31/1.04/1.03x** |
| robot_leaf_to_leaf | 1744/2699/1376 | 469.8/726.8/415.7 → **3.71/3.71/3.31x** | 446.8/756.7/507.1 → **3.90/3.57/2.71x** | 406.5/713.7/375.2 → **4.29/3.78/3.67x** | 417.5/773.9/391.3 → **4.18/3.49/3.52x** |
| siblings_under_16deep_dyn | 9055/13876/7246 | 375.7/487.6/261.9 → **24.10/28.46/27.67x** | 389.7/539.5/368.8 → **23.23/25.72/19.65x** | 277.0/502.1/258.7 → **32.69/27.64/28.01x** | 304.3/541.2/285.9 → **29.75/25.64/25.34x** |
| static_siblings_1k_elim | 448390/667110/379920 | 127840/164850/86863 → **3.51/4.05/4.37x** | 119280/169920/87907 → **3.76/3.93/4.32x** | 94699/171400/87319 → **4.73/3.89/4.35x** | 93627/164500/88386 → **4.79/4.06/4.30x** |
| static_climb_1k | 114000/176400/78992 | 35013/60303/35051 → **3.26/2.93/2.25x** | 39236/62167/33199 → **2.91/2.84/2.38x** | 37338/61848/33807 → **3.05/2.85/2.34x** | 37026/60971/32351 → **3.08/2.89/2.44x** |
| unknown_frame_1k_frames | 66062/94676/49618 | 52.0/92.2/44.3 → **1269.98/1027.13/1120.63x** | 314.4/501.8/265.1 → **210.12/188.66/187.13x** | 47.2/91.2/44.2 → **1400.03/1038.16/1122.35x** | 301.3/537.3/267.7 → **219.23/176.20/185.34x** |
| past_newest_1k | 618.9/903.3/458.5 | 475.4/630.7/305.5 → **1.30/1.43/1.50x** | 336.1/584.2/296.1 → **1.84/1.55/1.55x** | 231.8/401.7/247.0 → **2.67/2.25/1.86x** | 239.9/389.9/203.6 → **2.58/2.32/2.25x** |
| insert_steady_maxage10s_1k | 342.8/444.9/241.3 | 242.2/391.1/190.2 → **1.42/1.14/1.27x** | 220.8/373.9/192.6 → **1.55/1.19/1.25x** | 75.0/135.3/67.6 → **4.57/3.29/3.57x** | 71.6/121.9/63.5 → **4.79/3.65/3.80x** |
| ooo_insert_1k_uniform | 343.5/492.8/265.9 | 271.5/409.5/245.0 → **1.27/1.20/1.09x** | - | 556.7/1114/501.2 → **0.62/0.44/0.53x** | - |
| ooo_insert_1k_newest1pct | 287.1/496.7/289.3 | 259.9/441.6/220.4 → **1.10/1.12/1.31x** | - | 263.0/441.2/241.5 → **1.09/1.13/1.20x** | - |
| ooo_insert_10k_uniform | 391.5/529.8/348.7 | 317.5/780.1/287.1 → **1.23/0.68/1.21x** | - | 6859/12776/6518 → **0.06/0.04/0.05x** | - |
| ooo_insert_10k_newest1pct | 372.7/689.8/302.7 | 283.4/407.1/258.6 → **1.32/1.69/1.17x** | - | 576.2/1047/651.0 → **0.65/0.66/0.46x** | - |
| ooo_insert_60k_uniform | 604.1/584.8/475.9 | 443.1/505.4/403.1 → **1.36/1.16/1.18x** | - | 77666/134550/69921 → **0.01/0.00/0.01x** | - |
| ooo_insert_60k_newest1pct | 341.2/311.7/305.4 | 317.7/318.3/295.1 → **1.07/0.98/1.03x** | - | 4375/2909/3838 → **0.08/0.11/0.08x** | - |

Workloads (all in `src/lib.rs::reads` / `benches/hot.rs`, modelled on `benches/benchmarks.rs`; 1 ms sample spacing, rotation 1 rad/s about (1,1,1) so slerp takes the sin branch):
- `1hop_exact_1k` / `1hop_interp_1k`: `a<-b`, 1k samples, newest stamp / midpoint between samples 500 and 501.
- `4hop_dyn_*`: `a..e`, 1k samples per edge, every hop interpolates.
- `robot_*`: the bench's map->odom->base_link->{laser, imu, camera->camera_optical}; 3 dynamic edges x 1k, 3 static mounts.
- `siblings_under_16deep_dyn`: 16 dynamic edges (1k samples each) above two static leaves; leaf-to-leaf. My addition, to isolate topology-first.
- `static_siblings_1k_elim` = bench `tree_climb_1k_common_parent_elim`; `static_climb_1k` = `tree_climb_1k`; `unknown_frame_1k_frames` = `not_found_unknown_frame_1k`; `past_newest_1k` = one step past the newest sample (error path).
- `insert_steady_maxage10s_1k`: `max_age` 10 s, 10 ms spacing, 1000 resident samples, sample built in untimed setup (as the crate's bench does). For 2.4 the timed region includes dropping the `Transform`'s two `String`s; building them (2 more allocations) is NOT timed, so the caller-visible gap is larger than shown.
- `ooo_insert_{1k,10k,60k}_{uniform,newest1pct}`: unbounded history of n samples; fresh keys (odd nanoseconds, off the 1 ms grid) at a uniformly random position, or uniformly within the newest 1% of the covered span. History rebuilt untimed every n/10 inserts (grows <= 10%). Only run for 2.4, BTree+hash, deque+hash (the index variant does not matter here).

### Where the prototype is faster, and why
- **Topology-first** is the big structural win: `robot_leaf_to_leaf` 2.7-4.3x, siblings under a 16-deep dynamic chain 20-33x (2.4 samples and slerps all 16 shared ancestors twice, then discards them), `static_siblings_1k_elim` 3.5-4.8x.
- **Interning**: `unknown_frame` with 1k frames 66-95 us -> 44-92 ns with the hash index (>1000x), 265-540 ns with the BTree index (~200x). `static_climb_1k` 2.3-3.3x (the remainder is 1000 quaternion compositions, ~33-60 ns each on this machine).
- **No labels / no Vec**: `1hop_exact` 1.2-2.9x, `1hop_interp` 1.2-2.0x.
- **Deque append+evict**: steady insert 3.3-4.8x vs 2.4; the BTree-history prototype gets only 1.1-1.6x (that part is the two saved `String` drops and one hash probe).

### Where the prototype is NOT faster or is SLOWER
- **Multi-hop dynamic chains are a wash**: `4hop_dyn_to_ancestor` 0.90-1.19x, `to_descendant` 0.97-1.20x, `robot_map_to_optical` 1.03-1.39x. The cost is 3-4 slerps (2 x `sin`, 1 x `acos` through libm each) plus compositions; removing a hash per hop and three allocations is within measurement noise here. The draft's "162 ns -> 5.6 ns for 4 hops" walk figure may be true of the walk in isolation but does not move the end-to-end number on an interpolating chain. Any real gain for this (the most common) query shape has to come from the arithmetic, not the structure.
- **Deque vs BTree on lookup: no reproducible win.** At n = 1k the deque is 1.0-1.3x *slower* than the BTree prototype on `1hop_exact` with the hash index (167/282/157 vs 161/213/120 ns) and indistinguishable on `1hop_interp` and the chains. The draft's "bracketing lookup 112 -> 46 ns" is not reproduced end to end. Plausible cause (unverified): 64-byte entries put each binary-search probe on its own cache line, while BTree nodes pack 11 `i64` keys contiguously. A split layout (stamps in one deque, isometries in another) was not tried.
- **Out-of-order insert, deque** (ratios are 2.4 / deque):
  - uniform position: 1k 0.44-0.62x (0.5-1.1 us), 10k 0.04-0.06x (6.5-12.8 us), 60k ~0.005-0.008x (**70-135 us per insert, 115-230x slower than 2.4**). The draft quotes 22 us / 42x for this case from the 2.1.3 experiment; on this machine, today, it is 3-6x worse than quoted.
  - newest 1%: 1k parity (1.1-1.2x), 10k 0.46-0.66x (0.6-1.0 us), 60k 0.08-0.11x (**2.9-4.4 us, 9-13x slower than 2.4**; mean shift ~300 entries x 64 B).
  - So "realistic late samples land near the back" holds only if "near" means tens of samples, not 1% of a 60 s / 1 kHz window. A late sample 0.6 s old in a 60k history costs microseconds. Not measured: a "last few samples" pattern, where the deque should be at parity (the 1k/newest-1% case, mean shift ~5 entries, is the closest data point: parity).
  - BTree-history prototype stays at 2.4's level or slightly better (0.98-1.7x; one 0.68x outlier in run 2 for 10k uniform, not reproduced in runs 1 and 3).
- **BTree name index**: with 2-7 frames it is *faster* than hashbrown (1hop_exact 77-166 ns vs 120-213 ns: two tiny-map probes beat two foldhash computations). With 1k frames a miss costs ~220-450 ns more than the hash index per probed name (265-540 vs 44-92 ns on `unknown_frame`, which probes one known and one unknown name and allocates the error). Not measured: a successful lookup in a 1k-frame registry with a short chain, which would isolate the per-query index cost (expected, unverified: +0.2-0.4 us per lookup on this machine for two probes among ~1k names with long shared prefixes like `a_999`). The draft's "dependency-free BTree index" is therefore a real trade against large trees, a win for typical (<50 frame) robots.

## 5. Verdict on draft section 2 claims

| claim | result |
|---|---|
| interned slots make `frame_exists` O(1) (94 us at 1k frames today) | confirmed: 50-95 us -> <0.1 us (hash) / ~0.3-0.5 us (BTree idx) |
| topology-first samples only connecting edges | confirmed, 3-30x where shared ancestors are dynamic; zero effect on straight chains |
| one pass, diagnosis pass disappears | only on success; target-first error naming needs a cold re-walk (19,740 of 1M random queries named a different frame without it) |
| lookups allocate nothing | confirmed (0; 1 small `String` on error) |
| single-hop exact stays bit-exact | confirmed 27,069/27,069; all 372,996 `Ok` results have zero difference to 2.4 (with 2.4's inverse, not the draft's) |
| deque bracketing lookup 112 -> 46 ns | **not reproduced**; no measurable end-to-end gain at 1k, possibly a small loss |
| deque append+evict 129 -> 11 ns | direction confirmed end to end: steady insert 3.3-4.8x faster, zero allocations |
| out-of-order cost "22 us at 60k uniform" | **understated here**: 70-135 us; and 2.9-4.4 us for the newest 1% |
| BTree name index "to be measured" | faster for small trees, ~5-6x slower per probe at 1k frames |
| hash per hop is a hot-path waste | true but immaterial on interpolating chains (slerp dominates) |

Open risks not covered: deque reallocation spikes on growth (unbounded registries), behaviour on soft-float / no-CAS targets (nothing was built for thumb targets), the draft's conjugate inverse and Newton step (would change bits; not implemented), a quiet machine (all ratios under ~1.3x should be re-measured before being quoted).
