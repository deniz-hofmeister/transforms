# transforms 2.4.0: shipped state and standing constraints (fact-finding for v3)

Read-only pass at master `adccbdd` (= tag `v2.4.0`, published 2026-11-17). Tree clean.
Sources read in full: `analysis/v2.4-proposal/DESIGN.md` (rev 8), `analysis/v3-review/REPORT.md`
(round 2, amended 2026-11-11/12), `CHANGELOG.md` 2.4-2.2 sections (2.1/2.0 by heading only),
`MIGRATION.md`, `README.md`, `Cargo.toml`, `tests/test_all.sh`, `.github/workflows/tests.yml`,
AGENTS.md. Test files read by structure (names, module docs), not line by line.
Verified by running: `cargo test` (default features) passes; counts below come from that run.
The gate (`tests/test_all.sh`) was NOT run in this pass.

## 1. Maintainer rulings

### 1a. 2.4 design, §8 (`analysis/v2.4-proposal/DESIGN.md:897-927`)

| ID | Date | Ruling | Binds v3? |
|---|---|---|---|
| D1 | rev 3 | Deprecate `get_transform_at`: withdrawn (no replacement in 2.x) | No. Cross-time type moved to "v3 ledger" (§3) |
| D2 | 2026-11-12 | No scope exception for checked application to positions (`transform_positions`); rustdoc recipe on `Transformable` ships instead | Scoped to 2.4 ("No for 2.4 ... revisit on demonstrated need"). The reasoning (buildable outside => outside) is the AGENTS.md scope rule, which stands |
| D3 | 2026-11-12 | Name `transform_positions`: moot | n/a |
| D4 | rev 3 | Withdrawn | n/a |
| D5 | 2026-11-06 | Rejecting empty frame names is non-breaking; ships in 2.4 | Behaviour stands in v3 unless reopened |
| D6 | 2026-11-12 | Enforced at the insert boundary (`Buffer::check`, flat `RegistryError::EmptyFrameName`), not in `Transform::validate` | 2.x ruling. Rationale partly 2.x-specific (not changing constructors/Deserialize in a minor). v3 may revisit the site |
| D7 | 2026-11-12 | Unit payload. Explicitly "keeps both outcomes open": frame fields on insert errors are a v3 question (v3-review decision 5), to be decided together with this payload | Explicitly deferred to v3 |
| D8 | 2026-11-12 | No property test for the empty-name rule | 2.4 only |
| D9 | 2026-11-11 | Every example asserts its results | Convention now in AGENTS.md Style; stands |
| D10 | 2026-11-11 | Task-named, feature-agnostic examples replace the six | Stands (files exist) |
| D11 | 2026-11-12 | `Point` stays in the **2.x** examples | Explicitly 2.x; v3 decision 3 pending |
| D12 | 2026-11-11 | `ingest` example ships, last | Shipped |
| D13 | 2026-11-11/12 | `log`, `env_logger`, `tokio` dropped; `shared_latest` on std threads + `std::sync::RwLock` | Stands (dev-deps list in AGENTS.md) |
| D14 | - | Superseded by D10 | n/a |
| D15 | 2026-11-11/12 | Renames accepted; README example links are absolute URLs pinned to the release tag from 2.4 (crates.io resolves relative links against HEAD) | Process rule, in AGENTS.md release checklist; stands |
| D16 | 2026-11-12 | Clock-jump wording split: hazard on `with_max_age`, recovery on `remove_frame` / `remove_transforms_before` | 2.x wording; "strict cutoff is 2.x only" |
| D17 | 2026-11-12 | Of v3-review "Ships in 2.x": join 2.4 = frozen-bytes serde case, three doc sentences, internal deletions (`latest_timestamp`, `Buffer.child`, `InsertError::ChildFrameMismatch`), two doc recipes. NOT joining = Point-family removal notice, `#[diagnostic::on_unimplemented]` on `TimePoint` | 2.4 scoping only |

Ruled 2026-11-12 without ID (`DESIGN.md:917-927`): implementation authorized on feature branches
(merge/tag/publish stay with maintainer); release scope; MIGRATION opening text fixed; README
example links absolute; README feasibility-report sentence dropped. Also (`:818-824`): one extra
SciPy golden interpolation vector; `CARGO_BUILD_WARNINGS=deny` in the gate; `gate` and `commits`
are required status checks; **no notice to bridge maintainers and no issues on adapters "for now"**.

### 1b. v3-review rulings (`analysis/v3-review/REPORT.md`)

| ID | Date | Ruling | Status for v3 |
|---|---|---|---|
| Item 10 D1 | 2026-11-10 | Under `with_max_age`, insert strictly older than frame's newest stamp - `max_age` returns `RegistryError::SampleExpired { frame, stamp, newest }`, stores nothing; sample exactly on the boundary accepted (`:430`) | **Settled for 3.0** |
| D2 | 2026-11-10 | `add_transforms` fails whole and stays equal to the loop; each member measured against the frame's running newest (batch members included) | Settled |
| D3 | 2026-11-10 | No `max_age` => never reported; statics unaffected; unrepresentable subtraction => accepted; `reparent_frame` seed exempt | Settled |
| D4 | 2026-11-10 | Order-independent "batch newest" alternative rejected | Settled |
| D5 | 2026-11-10 | No `clear_history` | Settled |
| D6 | 2026-11-10 | 2.x keeps the silent drop; the error is not reset detection | Settled |
| Item 12 / decision 16 | 2026-11-11 | `remove_transforms_before(t)` removes the sample at `t` (inclusive), name kept, silent MIGRATION row (`:575-639`, `:956`) | **Settled for 3.0** |
| decision 13 | 2026-11-11 | Examples restructure in 2.4, port in 3.0 | Done for 2.4 |
| Hold | 2026-11-12 | "3.0 work is held until 2.4 has shipped. The open decisions below are taken then" (`:925-937`) | Condition met 2026-11-17. Memory note: maintainer had not explicitly opened v3 as of release; the current user request does |

Open, never ruled (v3-review decisions table `:939-956`): 1 (`inverse`: conjugate / keep
normalizing + delete variant / status quo; rec. b), 2 (normalize at constructors+Deserialize vs
`Buffer::insert`; rec. constructors), 3 (delete Point family; rec. delete all four), 4 (2.x notice
form), 5 (child frame field on structural insert errors; leaning yes), 6 (six vs two accessors on
`CrossTimeTransform`; rec. six), 7 (finiteness check 3.0 vs 2.x; rec. 3.0), 8 (delete
`SameFrameMultiplication`; yes, only with item 1), 9 (rename `ReparentingNotSupported`; optional),
10 (dev-dep on `transforms` 2.3 for a differential harness), 11 (policy sentence on what a minor
may change), 14 (`ingest` recovery vs guard), 15 (publish "expiry is the last insert check").

Everything in the v3 review other than item 10 and item 12 is a **recommendation**, not a ruling
(`REPORT.md:12-14`). The review's "What 3.0 leaves alone" list (`:641-656`: `Stamp`, flat
`RegistryError`, strict tree, `BTreeMap`, operators, Non-Goals, type names/paths, MSRV 1.85) is
also a recommendation and was written under a "minimal break" premise. The current user request
("re-imagining, no backwards compatibility") supersedes that premise; none of those is a ruling.

### 1c. Standing constraints not tied to a release (AGENTS.md)

- "Breaking changes ... only with explicit maintainer sign-off per release — a past approval does
  not carry forward."
- Stop-and-ask list: weakening the gate, widening public API, adding a dependency, touching
  Non-Goals. The f32 Non-Goal specifically: "any f32 work, in any release, starts with the
  maintainer amending it" (`DESIGN.md:771-776`).
- Non-negotiables (`forbid(unsafe_code)`, dependency list, no_std parity + thumb targets, additive
  and numerically identical features, no panics, checked time arithmetic) are written as
  release-independent.

## 2. v3-review items: landed in 2.4 vs remaining

**Landed in 2.4** (all from "Ships in 2.x", `REPORT.md:684-719`; verified by commit log
`v2.3.0..v2.4.0` and grep):
- Part D empty frame names: `e2fd9a5`; `src/core/buffer/mod.rs:325`, `src/core/registry/error.rs:54,252`.
- Part C rustdoc corrections incl. corrected item-5 clock-jump wording: `7caf816`.
- Three doc sentences (lossy `Display`, cross-time errors "may" name the fixed frame, software f64): `ee176c6`.
- Two doc recipes ("latest" lookup; checked ROS `(i32, u32)` stamp conversion): `f1c4809`.
- Internal deletions: `fe03dc4` (expiry reference derived from last key), `1d1ea4b` (`Buffer.child`,
  `ChildFrameMismatch`). grep finds no `latest_timestamp` field or `ChildFrameMismatch` in `src`.
- Frozen postcard bytes with a general rotation: `7cff392` (`tests/serde.rs:400`).
- Off-centre golden interpolation vector: `115d567` (`tests/golden_vectors.rs:293`).
- Examples E1-E4: `9328a04`, `e6d1c02`, `ed5d470`. `Point` still used in `basic` and README.
- Gate: `CARGO_BUILD_WARNINGS=deny` (`77d0274`).

**Not landed (still 2.x-shippable, additive):** Point-family removal notice; `TimePoint`
`#[diagnostic::on_unimplemented]` (grep: absent); "unbounded interpolation is the default, not a
guarantee" wording (never ruled; grep: absent).

**Remaining = all twelve 3.0 items.** None is implemented in `src` (grep: no `SampleExpired`, no
`CrossTimeTransform`; `Buffer::remove_before` still strict, `src/core/buffer/mod.rs:444-456`):

| # | Item | Status |
|---|---|---|
| 1 | `get_transform_at` returns `CrossTimeTransform` (+ cross-leg error rule) | recommended; decision 6 open |
| 2 | Rotations normalized where built (8 EPS dead band) | recommended; decision 2 open |
| 3 | `inverse` keeps normalizing; delete `TransformError::QuaternionError` variant | recommended; decision 1 open; one critic dissents |
| 4 | Finiteness check at lookup exit | recommended; decision 7 open |
| 5 | `Disconnected` before `NotFoundAt` | recommended |
| 6 | Error shapes: unit `TimestampMismatch`, `#[non_exhaustive]` struct variants, frame fields (open), rename (optional) | decisions 5, 9 open |
| 7 | Delete `Point`, `Transformable`, `Localized`, `get_transform_for` | decisions 3, 4 open; prototype builder leaned to keeping `Point`+`Transformable` |
| 8 | Delete `Transform::interpolate` (+ `StaticInterpolation`, `TimestampOutOfRange`) | recommended |
| 9 | Delete `Timestamp::as_seconds` | recommended |
| 10 | `SampleExpired` | **ruled** D1-D6 |
| 11 | Delete `SameFrameMultiplication` | decision 8; only with item 1 |
| 12 | Inclusive cleanup cutoff | **ruled** |

Prototypes exist under `analysis/v3-review/prototypes/` as library-only diffs against `7cc2a65`
(pre-2.4; stacked +416/-793). They predate Part D and the D17 internal deletions, so they will not
apply to `adccbdd` unchanged (unverified; inferred from both touching `Buffer::check`). None was
gate-clean; `cargo test` does not compile on the stack (`REPORT.md:1009-1014`).

The review's explicit "Not in 3.0" list (`:658-682`) rejected, under the minimal-break premise:
public `Rotation`/`Isometry`, operators -> named methods, `TransformError<T>`, required `max_age`,
`FrameId`/interning (hashing measured 7-8 ns/hop), `hashbrown` -> `BTreeMap`, signed `Timestamp`,
f32, removing `thiserror` (~128 lines), optional `approx`, `Resolved` result type, typed frames,
`can_transform`/`get_latest`. Max interpolation gap is "open, not rejected", core-or-nowhere, no
settled design. These are the natural reopen candidates for a re-imagining; the measurements cited
there (resolver rewrite 8-20% slower on 64-hop dynamic chains; conjugate changes bits of 46.5% of
lookups) were taken at 2.3.0 and not re-run here.

## 3. Test-suite inventory

`cargo test` (default features) at `adccbdd`: 196 unit + 4 + 5 + 14 + 0 (serde, feature off) + 2 +
18 + 9 integration + 39 doctests, all pass. `#[test]` counts by grep.

| Location | Tests | Pins |
|---|---|---|
| `src/core/registry/tests.rs` (4076 lines) | 98 | The bulk. By name prefix: `reparent_frame_*` 21, `get_transform_*` 11, `latest_common_*` 9, `time_travel_*` 7, `add_transform_*` 6, `a_batch_*` 6 (+ `add_transforms_*`), `remove_transforms_*` 5, `an_empty_*` 3, `remove_frame_*` 2; plus chain/branch/interpolation basics, identity for same frame, error precedence, overflow (`an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts`), `Send`/`Sync` assert. Two tests call private `get_transform_chain` |
| `src/core/buffer/tests.rs` (706) | 29 | Insert checks, static xor dynamic, parent pin, upsert, expiry on insert (incl. the silent drop D1 flips: `:639`, `:682` per review, line numbers pre-2.4), `remove_before`, interpolation bracket |
| `src/geometry/transform/tests.rs` (732) | 26 | Constructor validation, `Mul` frame/stamp rules, `inverse`, `interpolate`, empty-name acceptance at construction |
| `src/geometry/quaternion/tests.rs` (379) | 23 | Product, conjugate, normalize, slerp (bit-pinned), rotate_vector |
| `src/time/timestamp/tests.rs` (130) | 12 | Checked arithmetic, `as_seconds`, `now`/`try_now` smoke |
| `src/geometry/point/tests.rs`, `vector3/tests.rs` | 4 + 4 | `Point::transform` checks; vector ops |
| `tests/golden_vectors.rs` (320) | 5 | **External oracle**: SciPy literal digits for apply, 2-hop compose, inverse lookup, general rotation, slerp midpoint and quarter point. Catches a consistently flipped convention. Two cases apply through `Point` (`:99-106,205-212`) |
| `tests/properties.rs` (771) | 14 | inverse twice; transform/inverse roundtrip; slerp endpoints and unit norm; interpolate exact at endpoints + rejects outside; registry chain roundtrip to identity; `latest_common_time` exact on any pair; failed lookup names an edge on the connecting chain; failure diagnosis independent of geometry; random inserts + re-parents keep the tree walkable; `add_transforms` == loop or changes nothing (`:640`, `cases: 256`); constructor norm tolerance both sides; dynamic history vs reference map after upserts, expiry, cleanup (second `proptest!` block `:711`) |
| `tests/time_traits.rs` (659) | 18 | Custom `TimePoint` clocks (wide u128, picosecond, a clock that cannot report seconds): `latest_common_time` span checks, static leg served at any time, range extremes are ordinary, `SystemTime` (std), error formatting cannot be masked, cross-time with custom clock |
| `tests/serde.rs` (430) | 18 (feature `serde`) | JSON roundtrips, hand-written JSON field names, missing/null stamp are errors, invalid rotation/non-finite rejected on decode, `SystemTime` roundtrip, **frozen postcard bytes** for `Transform` (identity and general rotation) and `Point` |
| `tests/v2_compatibility.rs` (191) | 9 | "Public behavior that internal storage and traversal changes must preserve": sampling-failure precedence over `Disconnected`; failing edge on the chain; exact sample preserves signed zero and accepted rotation norm; attaching a root changes depth; unbounded default interpolates across long gaps; right identity keeps `SameFrameMultiplication`; cross-time result keeps only target stamp and is insertable; timestamp operators usable; constant dynamic reference as fixed frame |
| `tests/approx_comparisons.rs` (103) | 4 | `approx` impls check every component, exact metadata |
| `tests/tests.rs` (154) | 2 | Legacy tree smoke tests (`test_matching_tree`, `test_non_matching_tree`); names violate the Style rule on descriptive names; `:133` uses a struct-variant pattern |
| Doctests | 39 | README (via `mod readme` in `src/lib.rs:125`) + rustdoc examples |
| Examples | 4 x 2 modes | Asserting programs run by the gate |
| `benches/benchmarks.rs` (476) | smoke only | `cargo bench -- --test` |

**Natural acceptance gates for a rewrite** (API-independent or cheaply re-pointed):
1. `tests/golden_vectors.rs` - the only outside oracle; AGENTS.md forbids regenerating its digits.
   Needs only re-pointing of the two `Point` applications to accessor formulas.
2. `tests/properties.rs` - invariant-level; survives a rename-only port. Known weakness: the
   interpolate oracle is blind to swapped lerp weights (`REPORT.md:400-405`); the batch property
   rarely notices the item-10 atomicity trap (2 of 40 runs).
3. `tests/serde.rs` frozen postcard bytes - only if v3 keeps the wire format. A re-imagining that
   changes field order, `Stamp` tagging or drops `Point` invalidates them by design.
4. `tests/time_traits.rs` - pins the `TimePoint` trait contract with adversarial clocks; valid only
   as far as v3 keeps a generic time parameter.
5. `tests/v2_compatibility.rs` - explicitly a 2.x behaviour freeze. The review already flips 4 of 9
   (items 1, 2, 5, 11). For v3 it is a checklist of *declared differences*, not a gate.
6. The 278,880-line differential trace probe in `analysis/v2-feasibility/` (used for 2.2.0 and for
   every v3 prototype) - not part of the gate; must be ported to any new cross-time type; a
   dual-version harness needs a `transforms` 2.x dev-dependency (decision 10, unruled).

The 98 registry unit tests are the largest asset and the least portable: they are written against
the current public method names and error variants, and live in-crate (`mod registry_tests`).

## 4. Dependencies and feature matrix (`Cargo.toml`)

Runtime: `thiserror` 2.0.20 (no default features), `approx` 0.5.1 (no default features, **not
optional**), `hashbrown` 0.17.1 (default features on), `libm` 0.2.16, `serde` 1.0.229 (optional,
`alloc` + `derive`).
Dev: `proptest` 1.11.0, `serde_json` 1.0.151, `postcard` 1.1.3 (alloc), `criterion` 0.7.0 (held
below 0.8 for MSRV; dependabot ignore rule).
Features: `default = ["std"]`; `std = ["serde?/std"]`; `serde = ["dep:serde"]`. Four combinations,
all exercised by gate and CI. Feature-gated items: `Timestamp::now()`, `try_now()`, `TimePoint for
SystemTime` (`std`); serde derives (`serde`). Edition 2024, `rust-version = "1.85"`.
`[profile.release]`: `codegen-units = 1`, `lto = true` (repo builds only).
Package `include`: `/src`, manifest, README, CHANGELOG, MIGRATION, LICENSE (examples, tests,
analysis not shipped).

Gate (`tests/test_all.sh`, 61 lines, nightly required): 2 builds, 4 test runs, 4 clippy runs
(`--all-targets -D warnings`), fmt, 2 rustdoc builds, 8 example runs, 2 bench smoke runs, 6 thumb
builds (3 targets x {plain, serde}). CI (`tests.yml`, 257 lines, 13 jobs): `gate` (the script
verbatim), `commits` (AI trailer), `formatting`, `clippy` (stable, 4 combos), `tests` and `serde`
(x86_64 + ARM64), `doc`, `msrv` (1.85, `cargo check` 4 combos), `examples`, `benches`,
`bare-metal` (4 targets incl. riscv32imc), `semver` (cargo-semver-checks vs latest published, 4
combos), `security` (cargo audit). Required status checks: `gate`, `commits` only.

## 5. Line counts (`src/`, awk classification: blank / doc `///` `//!` / `//` comment / code)

| File | Total | Blank | Doc | Comment | Code |
|---|---|---|---|---|---|
| `core/registry/mod.rs` | 1365 | 53 | 674 | 88 | 550 |
| `core/registry/error.rs` | 289 | 20 | 155 | 0 | 114 |
| `core/buffer/mod.rs` | 495 | 30 | 201 | 19 | 245 |
| `core/buffer/error.rs` | 55 | 9 | 27 | 0 | 19 |
| `core/mod.rs` | 7 | 2 | 1 | 0 | 4 |
| `geometry/transform/mod.rs` | 655 | 36 | 315 | 3 | 301 |
| `geometry/transform/isometry.rs` | 111 | 11 | 36 | 0 | 64 |
| `geometry/transform/traits.rs` | 180 | 4 | 156 | 0 | 20 |
| `geometry/transform/error.rs` | 81 | 11 | 33 | 0 | 37 |
| `geometry/quaternion/mod.rs` | 376 | 28 | 162 | 1 | 185 |
| `geometry/quaternion/error.rs` | 13 | 1 | 3 | 0 | 9 |
| `geometry/vector3/mod.rs` | 182 | 21 | 36 | 0 | 125 |
| `geometry/point/mod.rs` | 204 | 13 | 63 | 4 | 124 |
| `geometry/mod.rs` | 11 | 2 | 1 | 0 | 8 |
| `time/timestamp/mod.rs` | 314 | 23 | 172 | 0 | 119 |
| `time/traits.rs` | 130 | 4 | 113 | 0 | 13 |
| `time/stamp.rs` | 72 | 4 | 42 | 0 | 26 |
| `time/system_time.rs` | 34 | 4 | 8 | 0 | 22 |
| `time/error.rs`, `time/mod.rs` | 16, 22 | 1, 2 | 4, 10 | 0 | 11, 10 |
| `errors/mod.rs`, `lib.rs` | 14, 125 | 1, 2 | 8, 93 | 0, 1 | 5, 29 |
| **Library total** | **4751** | 282 | **2313** | 116 | **2040** |

Unit tests in `src/**/tests.rs` (each `mod.rs` ends with a one-line `#[cfg(test)] mod tests;`):
registry 4076, transform 732, buffer 706, quaternion 379, timestamp 130, point 122, vector3 37 =
**6182** lines (4852 code). Integration `tests/*.rs`: **2628** lines. Benches 476, examples 609.

Ratios: library code 2040 lines; rustdoc 2313 lines (1.13 doc lines per code line; in
`registry/mod.rs` 674 doc vs 550 code); tests 8810 lines = 4.3x library code. The registry (mod +
error + its tests) is 5730 of 10933 `src` lines.

Public surface: review counted 104 named public items at 2.3.0, 87 after its deletions
(`REPORT.md:49-50`); 2.4 added one variant. Not recounted here (unverified). `Registry` has 11
public methods (`src/core/registry/mod.rs:118-827`): `new`, `with_max_age`, `add_transform`,
`add_transforms`, `get_transform`, `get_transform_for`, `get_transform_at`, `latest_common_time`,
`remove_transforms_before`, `remove_frame`, `reparent_frame`. Crate root re-exports `Registry`,
`Localized`, `Transform`, `Transformable` (`src/lib.rs:119-120`); `Vector3`, `Quaternion`, `Point`
only via `geometry::`, errors via `errors::`, time via `time::`. `#[allow]` sites: 12
`clippy::float_cmp` in tests, 3 cast allows + 1 scoped allow in `src/time/timestamp/mod.rs`.

## 6. What a clean-slate v3 would make false

**README.md**
- Quick start (`:41-72`) leads with `Point`, `Transformable`, `get_transform_for` (item 7 deletes all three).
- "Derived transforms are not re-validated ... Call `Transform::validate`" paragraph (item 4; item 2).
- `with_max_age` bullet describes eviction with no mention of an error (item 10).
- `get_transform_at` bullet: "stores only the target stamp ... keep both instants separately" (item 1).
- Install pins `2.4.0`; "What's new"; example links pinned to `blob/v2.4.0/`; tf2 comparison table rows
  for Latest data / Retention / Re-parenting if those semantics change.
- Non-Goals list is duplicated verbatim in `src/lib.rs` ("edit both or neither").
- "`Display` and `Debug` text are not [stable]" (`:118`) - stays true, useful freedom.

**MIGRATION.md** (260 lines, titled "Migrating to 2.4", three audiences: from 2.0.0, from 1.x, from
2.0 pre-releases, plus serde wire format)
- Opening "From 2.0.0, one behaviour change applies ... Nothing else needs migrating".
- Runtime bullets: "`get_transform_at` retains only the target time ... do not use ordinary
  `Transformable` application"; "Lookups and `Transform::interpolate` do not extrapolate";
  "An ancestor-directed lookup can return an infinite translation as `Ok`" (`:195` per review);
  "rejects equal child frames ... `SameFrameMultiplication`" (`:199`); `NotFoundAt`-over-`Disconnected`
  precedence (`:136-137`); the 1.x error table.
- Serde section: frozen field order, `Stamp` variant order, the 1.x `t = 0` postcard quirk. False if
  v3 changes the wire format; `Point` payloads stop decoding under item 7.
- A re-imagined v3 needs a new 2.x -> 3.0 guide; keeping the 1.x and pre-release sections is
  optional overhead (termviz2 and transforms-py still pin 1.x, `REPORT.md:920`).

**AGENTS.md** (483 lines; review counts 10 of 16 invariant bullets changing for the *minimal* 3.0,
`REPORT.md:912-914`)
- Architecture: "`Registry` — a `HashMap<String, Buffer>` keyed by child frame name", "`BTreeMap<T,
  Isometry>`", `Point` as "reference implementation of the `Transformable`/`Localized` traits",
  `TimePoint` method list.
- Invariants that encode current implementation rather than behaviour: `Buffer::check` "must never
  read the stored samples"; `Buffer::empty_like`; `InsertError`/`GetError<T>` split; "No registry
  path produces `TransformError::TimestampOutOfRange`; its only producer is the public
  `Transform::interpolate`"; `RegistryError::TransformError` arm canonicalisation; "Derived values
  are deliberately not re-validated"; `Transform::unvalidated` caller rule; "`Quaternion::from_wxyz`
  does not normalize"; expiry "removed on insert" with no error; cleanup strictness.
- Style: the constructor list; the standing-allow list with file paths; dev-dependency list.
- Non-negotiables naming the exact dependency set (`thiserror`, `approx`, `hashbrown`, `libm`,
  `serde`) and exact feature-gated items.
- API stability and Scope sections stay true, but Scope's worked examples (`latest_common_time` is
  core, `earliest_common_time` is not) assume the current query family.
- Behavioural invariants that a rewrite should keep as its spec (not implementation-bound):
  parent/child direction; composition rule; static xor dynamic; strict acyclic tree; batch
  atomicity == loop; lookup returns exactly the requested frames or an error decided on frames
  before geometry; no extrapolation; data-driven expiry, no wall clock; exact `==`;
  `#[non_exhaustive]` errors; single inversion per lookup and bit-exact single-hop.

**CHANGELOG.md** (738 lines): the 2.0.0 section alone is ~415 lines (`:254-670`). Nothing becomes
false; a 3.0 entry must list silent rows that `cargo semver-checks` cannot see.

**tests**: `tests/v2_compatibility.rs` by name and purpose; `tests/serde.rs:335`
`point_postcard_bytes_are_frozen`; `tests/tests.rs` legacy names.

## 7. Documentation / process overhead a v3 could simplify

- **Doc-to-code ratio.** 2313 rustdoc lines for 2040 code lines. Much of it documents hazards the
  3.0 items remove (cross-time application restrictions repeated in README, MIGRATION, crate docs,
  `Transform`, `Transformable`, `get_transform_at`; clock-jump wording split across three rustdocs;
  "not re-validated" caveats; `.at().unwrap()` warning). Closing a hazard in the type system deletes
  its prose in every location.
- **Triplicated normative text.** Non-Goals in README + crate root + AGENTS.md; examples loop in
  gate + CI `examples` job; clippy combos in gate + CI `clippy` job; MSRV in Cargo.toml + README +
  AGENTS.md + CI + dependabot comment.
- **CI redundancy.** `gate` already runs build/test/clippy/fmt/doc/examples/benches/thumb on
  nightly; separate jobs repeat them on stable. Only `gate` and `commits` are required checks. The
  stable-only value is: stable clippy, MSRV, ARM64 native, RISC-V, semver, audit.
- **`analysis/`**: 75 files, 1.3 MB, eight directories, several documents marked superseded
  (`v3-warrant/REPORT.md` header; three `v3-proposals` notes + four addenda; `v3-review/round1/`).
  The v3 design state is spread over: v3-review REPORT (1053 lines), `sample-expired-contract.md`,
  2.4 DESIGN §3/§6/§8, v3-warrant ledger, dependents study, seven `v3-proposals` files. A single v3
  design doc with one rulings table would replace the cross-references. Decision IDs collide across
  documents (2.4 D1-D17, item-10 D1-D6, v3-review decisions 1-16).
- **Release procedure**: 14-step checklist with prep branch, PR, fast-forward push, release branch,
  annotated tag, publish, GitHub release. Memory note: the maintainer questioned whether the
  fast-forward is worth it over an ordinary merge and did not rule.
- **AI-disclosure machinery**: `.githooks/agent-guard` (113 lines), `commit-msg` (57), wiring in
  `.claude/`, `.codex/`, `.cursor/`, a CI job. Orthogonal to v3; standing rule, not to be touched
  without the maintainer asking.
- **AGENTS.md invariants as scar tissue**: several bullets are multi-sentence narratives of a
  specific bug and name private functions. A rewrite can restate them as ~12 behavioural invariants
  (list in §6) and let tests carry the rest.
- **MIGRATION.md** serves three origin versions; v3 could ship 2.x -> 3.0 only and link the 2.4
  tag for older paths.
- **Legacy test file** `tests/tests.rs` (2 tests, `test_*` names) and the in-crate 4076-line
  registry test module are candidates for restructuring into behaviour-grouped integration tests
  that survive internal rewrites (only two tests need private access).

## Not verified in this pass

- The gate was not run; only `cargo test` in default features.
- Public item count (104/105) not recounted.
- CHANGELOG 2.0-2.1.3 bodies and MIGRATION middle section (truncated in my read at the 1.x error
  tables) were not read in full; line references into MIGRATION for `:136-137`, `:192`, `:195`,
  `:199` are the v3 review's, written against 2.3.0, and may have shifted by the 2.4 edits.
- Line references the review gives into `src` and tests are against `7cc2a65`, not `adccbdd`.
- Whether the v3-review prototypes apply to `adccbdd` was not tried.
- `analysis/dependents/*`, `v3-proposals/*`, `sample-expired-contract.md`, `round1/` not read.
