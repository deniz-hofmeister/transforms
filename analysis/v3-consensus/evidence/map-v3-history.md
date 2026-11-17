# v3 idea history: ledger, dispositions, and what the compatibility stance suppressed

Read-only pass, 2026-11-17, tree at `adccbdd` (v2.4.0).

## Sources and verification status

Read in full: `analysis/v3-proposals/transforms-v3-1.md` (v3-1), `-2.md` (v3-2), `-3.md` (v3-3),
`-addendum-2.2.0.md` (A-220), `-addendum-corrections.md` (A-corr), `-addendum-cross-time.md` (A-xt),
`-addendum-dependents.md` (A-dep), `analysis/v3-warrant/REPORT.md` (W), `analysis/v3-review/round1/REPORT.md` (R1),
`analysis/v3-review/sample-expired-contract.md` (SEC), `analysis/v3-review/REPORT.md` (R2, read in full, not skimmed).

Not read: `analysis/dependents/*`, `analysis/v2.4-proposal/DESIGN.md`, `analysis/v2-feasibility/REPORT.md`, the
prototype diffs. Every number below (timings, percentages, download counts, line counts) is **quoted from the
documents and unverified by me**. No probe was re-run.

Verified by me against the current tree (grep only):

- No 3.0 item is implemented at v2.4.0. Still present: `Point::new` (`src/geometry/point/mod.rs:78`),
  `Registry::get_transform_for` (`src/core/registry/mod.rs:363`), `Transform::interpolate`
  (`src/geometry/transform/mod.rs:379`), `Timestamp::as_seconds` (`src/time/timestamp/mod.rs:213`),
  `TransformError::{TimestampMismatch{..}, StaticInterpolation, TimestampOutOfRange, SameFrameMultiplication,
  QuaternionError}` (`src/geometry/transform/error.rs:29,39,46,57,80`), the `walk_failure` fallback
  (`src/core/registry/mod.rs:1027,1046,1096`), `Result`-returning operators (`src/time/timestamp/mod.rs:249,263`,
  `src/geometry/transform/mod.rs:507`), the strict cleanup cutoff (`src/core/buffer/mod.rs:449-453`).
- Insert errors are still unit variants: `EmptyFrameName` (`src/core/registry/error.rs:54`, shipped in 2.4 as a unit),
  `SelfReferentialFrame` (`:58`), `CycleDetected` (`:88`), `StaticDynamicConflict` (`:107`);
  `ReparentingNotSupported { current_parent }` (`:81`) has no child.
- 2.4 did ship the examples restructure (`examples/{basic,cross_time,ingest,shared_latest}.rs`) and the
  `latest_timestamp` field deletion (no match in `src/core/buffer/mod.rs`).

## Stance of each document (decision-relevant)

| Doc | Date / baseline | Question it asked | Stance |
|---|---|---|---|
| v3-1 | 2.1.3 | What should v3 break? | Breaking items batched; f32 by measurement |
| v3-2 | 2.1.3, no toolchain, nothing re-run | Same | Selective; temporal correctness, geometry validity, f32 |
| v3-3 | 2.2.0 `feee3d0` | "is this the better design, and does it earn its place? — never does it justify a break?" (`transforms-v3-3.md:7-9`) | **Full breaking window. The only document written under the stance the new mandate restores** |
| W | 2026-10-03, `aa29557` | Is v3 warranted now? | No; ledger + triggers. Superseded 2026-11-12 (`v3-warrant/REPORT.md:3-25`) |
| A-dep | 2026-11-06 | Is a ledger row *strictly necessary*? Row needs a break **and** real-user evidence; additive fix can never be a row (`-addendum-dependents.md:10-13`) | Most restrictive filter of all |
| R1 | 2026-11-10, `7cc2a65` | What should 3.0 contain? | "Notes over-build the fixes"; smallest set closing hazards; names/paths frozen for the re-export chain (`round1/REPORT.md:338-342`) |
| R2 | 2026-11-10/11 | Same, from scratch | Same; criterion "3.0 changes no number that is right today" (`v3-review/REPORT.md:682,887`); wire frozen (`:903`) |

## Ledger

Disposition = final state in R2 unless noted. "Reason class": **M** merit argument, **C** compatibility / migration cost /
dependent breakage / frozen names or bytes, **N** "closes nothing" or "no demonstrated need", **A** "additive, so not a
v3 item", **D** depends on another unmade decision. Multiple letters = mixed, first dominates.

### A. Adopted in R2

| # | Idea | Proposed by | Evidence given | R2 disposition |
|---|---|---|---|---|
| A1 | Cross-time result type (`CrossTimeTransform`), not insertable, not composable | v3-1 #1, v3-2 §1, v3-3 §4, A-xt, W ledger | Pinned test; v3-3 finding 4 (checked path rejects the correct point); W: target-time point accepted with wrong answer; R2: live overwrite-and-broadcast in roslibrust_transforms | **Item 1**, changed shape: six accessors, no apply, no `fixed_frame`, no serde, no root re-export; cross-leg error rule added |
| A2 | Rotations normalized where built (constructor + `Deserialize`), 8 EPS dead band | v3-2 §2 ("normalize accepted near-unit input"); v3-3 §1 (as part of `Rotation`); R1 | Accepted band scales by n²: 0.108 m over 4 hops of 10 km; all f32-sourced quats are in tolerance, out of band | **Item 2** (site is open decision 2: constructors vs `Buffer::insert` only) |
| A3 | Delete `TransformError::QuaternionError` variant; `inverse` keeps normalizing | R2 | Variant reachable through public `*` then `inverse` | **Item 3(b)** |
| A4 | Lookups never return non-finite | v3-2 (as the "alternative"), v3-3 §1 alt., R1 | `Ok(inf)` on ancestor-ward, sibling, `get_transform_at`, `get_transform_for` | **Item 4** (exit check, not on `*`) |
| A5 | `UnknownFrame > Disconnected > NotFoundAt` | v3-1, v3-2 §4, v3-3 §3, W ledger | v3-3 finding 6; variant depends on time for a topology cause | **Item 5** (minimal: delete `walk_failure` fallback, +41/−66) |
| A6 | `TimestampMismatch` becomes unit; struct variants `#[non_exhaustive]` | R1 | f64 seconds prints equal for stamps 1 ns apart | **Item 6** |
| A7 | Delete `Point`, `Transformable`, `Localized`, `get_transform_for` | R1 (inverts v3-1 "seal or validate `Point`", v3-2 "`Point`→`Pose`", v3-3 "bound `Point`") | Zero call sites in seven public code bases; `Point` unvalidated | **Item 7**, with conditions; path is open decision 3 |
| A8 | Delete `Transform::interpolate` (+ `StaticInterpolation`, `TimestampOutOfRange`) | R1 | No registry caller, no user; current oracle blind to swapped lerp weights | **Item 8** |
| A9 | Delete `Timestamp::as_seconds` | R1 | `Err` above 2^53 ns (~104 days) | **Item 9** |
| A10 | Expired insert is an error (`SampleExpired`) | Dependents study → A-dep rejected 0/3 → R1 stress test rejected → **maintainer reinstated 2026-11-10** | One future-stamped sample evicts history; later inserts vanish with `Ok(())` | **Item 10**, D1-D6 settled |
| A11 | Delete `SameFrameMultiplication` | R2 (v3-2 §5 "identity composition" was the precursor; v3-3 left it "open") | `get(a,c) * get(c,c)` is `Err`; code ≠ AGENTS.md composition rule | **Item 11**, only with item 1 |
| A12 | Inclusive cutoff for `remove_transforms_before` | Maintainer, 2026-11-11, after review; not put to lenses | Sample at type max unremovable; full clear needs `newest + 1 ns` | **Item 12**, name kept, silent row |

Already shipped in 2.x (not v3): on-chain `NotFoundAt` attribution (PR #112), sample fold + crate-private `Isometry`
(PR #113), unconnectable lookups diagnosed before geometry (`6c8bb4e`), `add_transforms` (2.3), empty-name rejection,
examples restructure, internal field deletions (2.4).

### B. Open in R2

| # | Idea | Proposed by | State |
|---|---|---|---|
| B1 | Child-frame field on structural insert errors (`SelfReferentialFrame`, `CycleDetected`, `StaticDynamicConflict`, parent-pin variant; `EmptyFrameName` with it) | Dependents study, A-dep (2 of 3), W ledger row | R1: no. R2: open, leaning yes (decision 5). "Not settled by evidence" (`REPORT.md:1029`) |
| B2 | Rename `ReparentingNotSupported` → `ParentMismatch` | v3-3 vocabulary, W ledger, R1 | Optional (decision 9). Only stated cost: one match arm in transforms_io |
| B3 | Normalize at constructors vs at `Buffer::insert` only | R2 scope critic | Decision 2; constructors recommended |
| B4 | Finiteness check in 3.0 or 2.x | R1 | Decision 7 |
| B5 | Six vs two accessors on `CrossTimeTransform` | R2 | Decision 6; six |
| B6 | Publish "expiry is the last insert check" | SEC vs R2 item 10 | Decision 15, open; SEC marked "do not implement as written" |
| B7 | Maximum interpolation gap | v3-2 §5; dimos report | v3-3: not carried (no need). A-dep: 0/3 (additive → never a row). R1: rejected. **R2: "Open, not rejected", core-or-nowhere, per-frame, no design** |
| B8 | Policy sentence: what a minor may change | R1 | Decision 11, open |
| B9 | Dual-version differential harness (2.3 as dev-dependency) | R1 | Decision 10 |
| B10 | `ingest` example: recover vs guard | R2 | Decision 14 |

### C. Rejected or deferred in R2

| # | Idea | Proposed by | Evidence for it | Final disposition and stated reason | Reason class |
|---|---|---|---|---|---|
| C1 | Public `Rotation` newtype (unit by type, private fields) | v3-3 §1; W calls it "the cleanest end state", "centrepiece" | Retires insert re-validation, `From` canonicalization, fallible `inverse`, two lookup error variants | Rejected: "items 2-4 close the same hazards without a new type"; "breaks every construction site"; ~17 dependent sites (R1, count not re-verified) | N + C |
| C2 | Public `Isometry` / geometry split from metadata (`Sample`, `Resolved`) | v3-1 #1, v3-2 §1 (softer), v3-3 | 2.2.0 already has the crate-private type | Rejected with C1; `Resolved` "closes nothing" | N + C |
| C3 | Translation magnitude bound `B = 1e100` | v3-3 §1 | Makes `Isometry` ops total | Rejected: item 4 exact with no constant; narrows decode; **not sound** — `*` reaches inf from 1e100 inputs in 1,748 products (R2) | M |
| C4 | Newton renormalization per composition | v3-3 §1 | ≤2.2e-16 over 1e9 compositions | Rejected: not needed for lookups; changes bits of 28% (R1) / 45% (R2) of products; diverges above √3 (R1); 14-15% of a soft-float composition (R2) | M + C (bit change) |
| C5 | `inverse` = conjugate, infallible | v3-3 §1; R1 item 3 | R2: numerically sound; 22% faster inverting one-hop lookup (78.1→61.1 ns) | R1 adopted, **R2 reversed**: closes no hazard, changes last bits of 46.5% of lookups, makes one property test flaky (~10.6% of runs), `inverse` of a `*`-collapsed rotation becomes `Ok` | **C (bit change) + N**, one M point |
| C6 | One vocabulary: `target`/`source` on every query result; `parent`/`child` only for stored edges | v3-3 §2; v3-2 §5; W ledger | Four vocabularies today; `parent()`/`child()` on a sibling lookup mislabels | Rejected: needs a second result type that "closes nothing"; type names, paths, constructor names frozen for the transforms_io → pelorus re-export chain | **C + N** |
| C7 | Operators → `checked_*` / `compose` | v3-1 #7; W ergonomics #2 ("least idiomatic part of the API"); ergonomics lens dissent | Cannot chain `a * b * c`; `(t - d).unwrap()` in examples; no `checked_add` | Rejected: "closes no hazard"; ~60 sites in repo, 4 in roslibrust doctests; slip is always a compile error; `#[diagnostic::on_unimplemented]` suggested instead | **C + N** |
| C8 | `TransformError<T>` (typed payloads) | v3-1 #7, v3-2 §5, v3-3 | f64 seconds payload lossy | Rejected: one type parameter for one variant; breaks roslibrust's public conversion trait at three sites; replaced by unit variant | C + M |
| C9 | Required `max_age`; `Registry::unbounded()` opt-in | v3-1 #4, v3-3 (agreed) | Unbounded memory by default | Rejected: "every studied user already sets it". R1's reason ("bounds memory only") declared **wrong** by R2 — eviction does bound the gap at `max_age` | **N** (no merit argument left) |
| C10 | Topology-first resolver rewrite (one resolver) | v3-1, v3-2, v3-3 §3 | −241 lines (R1) / +75/−300; up to 3.7x faster on shared-trunk lookups | Deferred to any 3.x: 1.4x slower exact-stamp into inner ancestor, 8-20% slower 64-hop root-ward; take only as a deletion that does not slow root-ward | M (perf) |
| C11 | `RegistryError::TransformError` → `Time(TimeError)` | R1 | After items 3, 8, 11 only `TimestampError` reaches the arm | Deferred: `tests/time_traits.rs:317` pins the arm; total only after the resolver rewrite | **C (test pin) + D** |
| C12 | `FrameId` / interning (internal or public) | v3-1 #2 | Zero-alloc lookups | Rejected: hashing 7-8 ns per hop; `Arc` unavailable on thumbv6m and riscv32imc; v3-3: name-reuse hazard + generation counters, no demonstrated need | M + N |
| C13 | Un-swappable `Lookup { target, source, at }` / builder | v3-1 #3 | Swapped lookup silent through accessors (v3-3 finding 7) | Rejected by v3-3 §2: "checked application catches every swap it sees". R1/R2 never revisit | M — **but premise removed by item 7, see Contradictions** |
| C14 | f32 / scalar-generic registry | v3-1, v3-2 §3 | f64 is software on every gate target as built | Deferred: Non-Goal; hardware never measured; concrete f32 module stays additive | D (maintainer-only Non-Goal) |
| C15 | `clear_history` | R1 prototype; dimos report; whirl_tf hand-rolls one | Registry cannot enumerate frames; dimos lost reset recovery | D5: no. "Additive, so not a v3 item"; item 12 removed half the argument | **A + N** |
| C16 | Batch-newest (order-independent) rule for `SampleExpired` | R1 alternative | Refuses a batch with a future-stamped glitch after a normal sample | D4: rejected — batch would stop equalling the loop, which rustdoc promises and `tests/properties.rs:640` pins | **C (declared contract)**, also an AGENTS.md invariant |
| C17 | Rename `remove_transforms_before` to make item 12 loud | R2 notes the option | "Before" will mean "at or before" | Maintainer: keep name, silent MIGRATION row | **C** |
| C18 | Harden `Point` (private fields, fallible ctor); or keep `Point` + `Transformable` (option b); `Point`→`Pose` | v3-1 #7, v3-2 §5, v3-3 §1, R1 geometry-lens dissent, R2 deletions prototype leaned (b) | Only two outsiders who posted code used `Point` (#43, #65/#66); #43 is a real rejected wrong-direction apply | Rejected: adds surface for a type no dependent uses; scope rule (12-line outside function) | N + M (scope) |
| C19 | Checked apply to plain positions (`Transform::apply` / `transform_positions`) | W ergonomics #1 (top-ranked), 2.4 DESIGN Part B, A-xt `transform_point` | "The checked path does not cover the work people actually do" | Rejected: buildable outside; wrong for free vectors | M (scope) |
| C20 | Finiteness gate on `*` | R1 prototype | — | Rejected: fails a batch property; closes no lookup hazard; tests deliberately mint inf through `*` | N + C (tests) |
| C21 | Delete `get_transform_at` | R1/R2 considered | — | Rejected: outside rebuild is a wrong-answer generator (restamp defeats guard) | M |
| C22 | Delete `TimePoint::checked_sub` | R2 lens | — | Refuted: eviction silently stops for a custom clock | M |
| C23 | `remove_frame` with children: reject or cascade | v3-1 #6 | Finding 8: old subtree reattaches to a same-named new frame | Rejected by v3-2, v3-3 §5 "on the merits, not only for compatibility"; R1/R2 leave alone | M |
| C24 | Fixed-frame static-ancestry check | pre-v3-1 | — | v3-1 downgraded to opt-in; v3-3 dropped: constant dynamic edge is legitimate | M |
| C25 | Cached depth | pre-v3-1 | — | Dropped: depth mutable (roots gain parents, `reparent_frame`) | M |
| C26 | `VecDeque` history | pre-v3-1 | — | Retracted: 42x slower out-of-order at 60k samples, no lookup win | M (measured) |
| C27 | Sample / frame capacity limits | v3-2 §5 | Embedded | v3-3: no demonstrated need. **No row in R1 or R2** | N; undispositioned |
| C28 | Allocation-free / heap-free lookup (borrowed results, scratch storage) | v3-2 §3, §4 | A-220: 3 allocations up to 4 hops, 7 at 64 | **Never dispositioned.** R2 only rejects interning | — |
| C29 | Self-edge / empty-name checks in `Transform::new` | R1 | — | Rejected: `(x, x)` identity must stay constructible and serializable | M |
| C30 | `get_latest`, `can_transform`, bulk lookups, caches, typed frames, `is_transient` | R1/R2 lenses | — | Rejected: buildable outside or a new wrong-answer source | M (scope) |
| C31 | Euler/axis-angle ctors, `mint`/`nalgebra`/`glam` features, `Vector3` norm/dot/cross | W ergonomics #4 | Both ROS adapters hand-write ~30 lines of nalgebra conversion (not re-counted) | Rejected: scope rule; W withdrew its own claim | M (scope) |
| C32 | Flat crate root | R2 lens | — | "Cosmetic"; paths frozen for re-export chain; `CrossTimeTransform` gets no root re-export | C + N |
| C33 | Optional `approx` | R1/R2 | — | Keep: doubles feature matrix; approx 0.6 at rc2 | M |
| C34 | Remove `thiserror` | R1/R2 | ~60 (R1) / ~128 (R2) lines | "Unrelated to v3" | N |
| C35 | `hashbrown` → `BTreeMap` for the frame map; signed `Timestamp` | R2 lenses | Adapters alias/clamp pre-epoch stamps | "Deferred", **no reason stated** (`REPORT.md:677`) | unknown |
| C36 | `latest_common_time` returns `T` | dependents study | — | A-dep 0/3: `Stamp::Static` is the true answer for an all-static chain | M |
| C37 | Follow the newest input's kind (static↔dynamic) | dependents study | — | A-dep 0/3: breaks static-xor-dynamic | M |
| C38 | Remove `now`/`try_now` on wasm32 | dependents study | — | A-dep 0/3: API would differ by target | M |
| C39 | `E0283` on `Registry::new()` / static-only setups | W ergonomics #5 | Turbofish documented on the type page only | **No disposition**; R2 leaves constructors alone | — |
| C40 | 2.x `#[deprecated]` on the Point family; `TimePoint` diagnostic attribute | R1 / R2 | — | Did not join 2.4 (ruling 2026-11-12) | moot under the new mandate |
| C41 | Hardware measurement (DWT, per-core flags); cross-runner trace-hash comparison | v3-1, v3-3 §6, W | — | Never done as of R2; status at 2.4.0 not checked | — |
| C42 | Frozen wire format; 3.0 reads every 2.x `Transform` payload; serde on `CrossTimeTransform` withheld | R1/R2 release plan | — | Kept as a constraint, not argued | C |

## Rejections driven mainly by compatibility, migration cost, or "closes nothing"

Merit case = my reading of the documents' own evidence, not a new analysis.

| Item | What the rejection actually rested on | Merit case without the compat constraint |
|---|---|---|
| C7 operators → named methods | Site counts, "closes no hazard" | **Strong** as an idiom/ergonomics fix: W ranks it #2, ergonomics lens dissented in R1, chaining is impossible today. Not a safety fix |
| C17 rename `remove_transforms_before` | Avoiding a loud break | **Strong**: a silent boundary change under an unchanged name is the class W calls most dangerous (`v3-warrant/REPORT.md:184-185`); a rename converts it to a compile error |
| B2 rename `ReparentingNotSupported` | One transforms_io match arm | **Strong** (cheap, name is false since `reparent_frame` exists). R1 says its rustdoc defers the rename to 3.0 (not re-checked) |
| C6 one vocabulary | Frozen names for the re-export chain; "closes nothing" | **Moderate**: mislabelled sibling results and four vocabularies are documented in v3-3 §2 and W #3; gain is clarity, not a closed hazard |
| C1 `Rotation` newtype (+ C5 conjugate) | Construction-site breakage; items 2-4 cheaper | **Moderate to strong for strictness/minimalism of invariants**: R2's own residuals are what the type would remove — `Buffer::insert` validates but does not normalize, a re-published derived rotation within 1e-6 is stored as given, `inverse` stays fallible, a zero-norm rule is needed, insert re-validation and `From` canonicalization stay. Caveat: without renormalization the invariant holds only for built values, and Newton (C4) has merit objections |
| C5 conjugate `inverse` | "Changes bits of 46.5% of lookups", flaky existing test | **Moderate**: R2 states the numerics are sound and measures −22%. The bit-change objection is exactly the "no number that is right today" rule. The `Err`→`Ok` on a collapsed rotation is a real merit point that a `Rotation` invariant would dissolve |
| C2 public `Isometry` / metadata split | Bundled with C1; "closes nothing" | **Unknown**. v3-3 §2 itself warns it makes unchecked geometry easier to reach. Interacts with C28 (allocation-free results) |
| C9 required `max_age` | "Every studied user already sets it" | **Weak to moderate**: safe-by-default memory bound; R2 withdrew the only merit objection. Does not close the in-window gap hazard (8 s inside 10 s) |
| C11 `Time(TimeError)` arm | A pinning test; waits on resolver | **Moderate** for error exactness: a wrapper arm that can carry one inner variant is a false generality |
| C16 batch-newest rule | Declared batch = loop contract and its pinning property | **Unknown**: order-independence vs losing "batch equals loop" (also an AGENTS.md invariant). Worth one explicit decision rather than inheritance |
| C15 `clear_history` | "Additive, so not a v3 item" | **Weak to moderate**: recovery table in R2 shows "Everything" needs caller-tracked frame names because the registry cannot enumerate frames |
| B7 maximum gap | A-dep: additive → never a row; a default bound "has no defensible value" | **Strong hazard, no design**: R2 calls it core-or-nowhere and the only mechanism closing the in-window future stamp (lookups up to 3.47 m off, all `Ok`). The "additive" filter no longer applies; a required per-frame bound is now expressible |
| C8 `TransformError<T>` | roslibrust trait breakage | **Weak**: after items 7, 8, 11 the unit variant is defensible on merit |
| C32 flat crate root, C34 `thiserror`, C35 | Cosmetic / unrelated / no reason | **Unknown** — never argued |
| C42 frozen wire format | Unargued constraint | **Unknown** — no document weighs it |
| C20 gate on `*` | Existing tests mint inf through `*` | **Weak/unknown**: only matters if derived `Transform`s are meant to be valid by type (C1) |

Rejected on merit and unaffected by the new mandate: C3 (bound unsound), C12, C21, C22, C23, C24, C25, C26, C29, C30,
C31, C36, C37, C38. C4 and C10 are merit rejections on measured cost, re-openable only with new measurements.

Adopted items whose *shape* was compat-limited: item 1 (method name and argument order kept; no serde), item 5 (minimal
fallback deletion instead of one resolver), item 6 (`#[non_exhaustive]` struct variants as a future-compat device),
item 12 (name kept), "What 3.0 leaves alone" (`REPORT.md:641-656`).

## Internal contradictions

1. **Checked application.** W ranks "the checked path does not cover the work people actually do" as ergonomics
   finding #1 (`v3-warrant/REPORT.md:105-122`); v3-3 §2 makes checked apply the answer to swaps; R2 item 7 deletes the
   only checked path and declares "seven slips become silent" (`REPORT.md:358-363`). R2 item 1 then keeps four label
   accessors so "a checked application [can] be built outside" (`:143-147`).
2. **Un-swappable lookup.** v3-3 §2 rejected v3-1 #3 because checked application catches swaps
   (`transforms-v3-3.md:140-142`). Item 7 removes that mechanism; the rejection was never revisited.
3. **`inverse`.** Conjugate proposed (pre-v3-1) → retracted (v3-1) → re-proposed (v3-3, R1 item 3) → reversed (R2 item 3).
   R2 still calls the numerics sound.
4. **`Rotation` as centrepiece.** W: batch the ledger "with the `Rotation` invariant as the centrepiece"
   (`v3-warrant/REPORT.md:263`); R1/R2 reject it.
5. **Expired insert.** A-dep 0/3 against; R1 stress test against; maintainer for. A-dep's reason ("turns routine late
   arrival into `Err`") is kept by R2 only as an "accepted cost".
6. **`max_age` and gaps.** W and R1: "bounds memory only". R2: wrong, it bounds the gap at `max_age`. W's probe (8 s gap
   interpolated in a 10 s window) is consistent with both; the disagreement is wording.
7. **Warrant triggers.** W's hardware trigger contradicts v3-3 §6 (concrete f32 module is additive); the deprecation
   trigger cannot fire under the 2026-11-03 stance (R1 `:400-402`).
8. **v3-3 §2 vs §7.** "target/source for anything a query returns" implies a new `get_transform` return type that §7 does
   not carry (R1 correction).
9. **Check order.** SEC publishes "this is the last insert check"; R2 item 10 keeps the order internal; the recommended
   `remove_frame` recovery depends on it (decision 15).
10. **Row necessity.** A-dep: an additive fix "can never be a row"; R2 adopts additive-shaped deletions and treats the
    maximum gap as open.
11. **Line and speed figures between rounds.** Resolver: "net −241" vs "+75/−300" (R1), R1 9-17% vs R2 8-20% slower;
    conjugate 10-13% (R1) vs 22% (R2) faster; `thiserror` ~60 vs ~128 lines; AGENTS.md bullets six vs ten.

## Claims later refuted or corrected

| Claim | Source | Refuted by |
|---|---|---|
| "2 allocations per lookup regardless of hops" | v3-1 | A-220: 3 up to 4 hops, 7 at 64 |
| M7 runs f64 in hardware on the CI target | v3-1 table | v3-3 finding 9: software unless `target-cpu=cortex-m7` + DP FPU |
| f32 ≈ 6 cm at UTM scale | v3-1 | v3-3 finding 12: 0.5–1 m at northing |
| Infallible `Mul`/`inverse`/`interpolate` | v3-1 #1 | v3-3: conflicts with finiteness; A-corr: interpolation can fail on time |
| Feasibility table "classifies every one as breaking"; docs-cut line counts; "will not be considered" quote | v3-1 | A-corr |
| Topology-first resolver "straightforwardly compatible" | pre-v3-2 | v3-2 withdrew it |
| `Ok(inf)` is ancestor-ward only | v3-3 finding 2 | R1: sibling lookups, `get_transform_at`; R2: also `get_transform_for`, static and interpolated chains |
| Missing `alloc::sync` only on thumbv6m | v3-3 finding 10 | R1: also riscv32imc |
| Bound `B` needs k ≈ 1.8e208 to overflow | v3-3 §1 | R2: `*` reaches inf from 1e100 in 1,748 products |
| Newton step simply holds the norm | v3-3 §1 | R1: diverges above √3, changes bits |
| "Stored normalized" is a fixed point | v3-3 §1 | R1/R2: second normalization changes 38% of values; dead band required |
| `*` and interpolation cannot fail; `latest_common_time` must be made to agree | A-220 | A-corr |
| "None of the v3 notes records" the cross-time inversion; `add_transform` errors are all unit variants | W | A-corr |
| No dependent matches a `RegistryError`; dependents do not use the parts that change; ~385 downloads/month | W | A-dep (942 in 30 days) |
| `Rotation` payoff is small (drift 4e-17) | W | R1: the accepted band is the effect |
| Example comments show direction confusion | W ergonomics #3 | R1: reads consistently |
| "v3 is not warranted now" | W | Superseded 2026-11-12 |
| "An additive type helps only callers who migrate" | A-xt, W | R1: `#[deprecated]` warns direct callers |
| Per-edge clock-reset detection needs unexposed access; dimos hole is "loud" | dependents study | R1; R2 item 12 |
| 64 B per sample | v3-2 / perf notes | R1: 122 B live |
| Conjugate recommended; `QuaternionError` unreachable after item 2; disconnected "never will" connect; 7 EPS unproven; recovery wording; `newest` is stored; rejected-insert cost 41 vs 74 ns; three compat tests flip | R1 | R2 "Changes from round 1" |
| `Quaternion` struct literal counts 9 and 4 | dependents study | R2: 5 and 2 |

## Gaps no document covers

- No hardware number exists anywhere; every embedded cost is an estimate, QEMU count, or host timing.
- R2's gate ran on no prototype; the stacked diff builds but its tests do not compile.
- Not covered by any lens (R1 and R2 Limits): lock contention, allocator failure on small-RAM parts, `SystemTime` in
  benchmarks.
- Allocation-free lookup (C28), capacity limits (C27), `E0283` (C39), and the frozen wire format (C42) have no
  argued disposition.
- Item 12 was never reviewed by the lens or stress agents.
- Private users are invisible; "zero users" means zero in seven public code bases.
