# transforms v3 — design notes (reviewed against 2.2.0)

Reviewed against master `feee3d0` (2.2.0, `Registry::reparent_frame` merged),
alongside `transforms-v3-1.md` and `transforms-v3-2.md`, which reviewed
2.1.3. Opinions, not decisions.

This note treats v3 as a full breaking window. The question asked of each
item is "is this the better design, and does it earn its place?" — never
"does it justify a break?". Every factual claim below was checked against the
tree, a probe run against the crate, or compiler output (host x86-64 Linux,
rustc 1.100.0-nightly 2026-09-22). Nothing was measured on hardware; claims
that depend on hardware are marked as estimates. Line references are to
`0cba503`, which adds PR #112 (§3) to that tree.

## What 2.2.0 changes for v3

- **Frames have a lifecycle.** The first insert pins a frame, `remove_frame`
  releases it, and `reparent_frame` replaces its pin. Depth is mutable, which
  confirms v3-1's reason for dropping cached depth. Any frame identity, string
  or ID, now has to be designed around frames that go away and come back
  (see §5).
- **Topology stability within a query is already guaranteed.** Lookups take
  `&self`, while `reparent_frame` and `remove_frame` take `&mut self`, so the
  borrow checker enforces it. Cross-call dependence
  (`latest_common_time` → `get_transform`) is documented as a one-guard rule
  (`README.md:99`, `src/core/registry/mod.rs:39`). v3 has nothing to add here.

## Verified findings

| # | Finding | Evidence |
|---|---|---|
| 1 | Every hop clones both frame names; a static hop clones a full `Transform`. | `src/core/buffer/mod.rs:389`, `:403-405` |
| 2 | An ancestor-ward lookup returns `Ok` with an infinite translation; the reverse direction rejects it. | `an_overflowing_lookup_reports_the_flat_variant_only_where_it_inverts` (`src/core/registry/tests.rs:2146`) |
| 3 | A cross-time result carries only the target stamp and can be inserted. | `cross_time_result_keeps_only_target_stamp_and_remains_insertable` (`tests/v2_compatibility.rs`) |
| 4 | A cross-time result **cannot be applied** to a source-time value through `Transformable`: `Point::transform` fails with `TimestampMismatch`. The only way to use `get_transform_at` is raw geometry, which skips every frame check. | probe; `src/geometry/point/mod.rs:112-127` |
| 5 | `NotFoundAt` could name an edge above the common ancestor, which the connecting chain never crosses. In the tree `map→odom` (t=0 only), `odom→{base,cam}` (t=0 and t=100), `base→lidar` (t=200), `get_transform("cam", "lidar", 50)` named `odom`; the real blocker is `lidar`. Rustdoc did not say `frame` may lie off the chain. Fixed in 2.2.0 by PR #112 (§3). | probe on `feee3d0`; pinned there by `failed_lookup_preserves_failure_above_common_ancestor`, which PR #112 renamed to `failed_lookup_names_the_failing_edge_on_the_connecting_chain` and flipped |
| 6 | For disconnected trees, the variant depends on time: `Disconnected` at t=0, `NotFoundAt` at t=50. | probe; pinned by `disconnected_lookup_preserves_sampling_failure_precedence`, documented on `NotFoundAt` |
| 7 | A swapped lookup is caught when applied to a `Point` (`IncompatibleFrames`) and is silent through `translation()`/`rotation()`. | probe |
| 8 | After `remove_frame`, the descendants' lookups fail loudly until a frame of the same name is re-added. They then reconnect with their pre-removal data, even if the new frame is physically a different one (probe: 0.5 → 100.5). This is the same mechanism as the documented remove-and-re-add route. | probe; `remove_frame` rustdoc |
| 9 | The gate's `thumbv7em-none-eabihf` build compiles f64 to **software** (`__aeabi_dmul`, `__aeabi_ddiv`); the target's default features include only `vfp2sp`. With `-C target-cpu=cortex-m7`, it emits `vmul.f64`/`vdiv.f64`. `thumbv8m.main-none-eabihf` is also software, and the Cortex-M33 FPU is single precision only. | `rustc --print cfg`, `--emit asm`; `tests/test_all.sh:45` passes no `target-cpu` |
| 10 | `alloc::sync` (and so `Arc<str>`) does not exist on `thumbv6m`: it has atomic load/store but no `target_has_atomic = "ptr"`. | compile error E0433 |
| 11 | Composition drift. Starting from rotations normalized in f64, repeating one constant increment (odometry style) drifts **linearly**, about 4.3e-17 per composition (4.3e-8 after 1e9). That reaches 1e-6 after about 2.3e10 compositions, roughly 270 days at 1 kHz. Random products drift like √n (4.5e-12 after 1e9). Inputs v2 accepts at the tolerance edge (w = 1 − 9e-7) leave the tolerance after **one** composition. | probe |
| 12 | Spacing between adjacent f32 values: 7.6e-6 at 100, 3.1e-2 at 5e5, 6.3e-2 at 8.3e5, 0.5 at 4.5e6, and 1.0 at 9.3e6–1e7. | probe (`f32::next_up`) |

## Corrections to the 2.1.3 notes

- **v3-1, embedded table.** "M7 (F7/H7): hardware f64" holds only for a build
  with `target-cpu=cortex-m7` (or `+fp64`), and only on parts that chose the
  Cortex-M7's double-precision FPU option. The FPU is single or double
  precision at the vendor's choice. The CI build for "M4F/M7" emits software
  f64 for both (finding 9). Any timing run must record its flags and FPU
  variant.
- **v3-1, precision.** "~6 cm at UTM scale" is true of easting (3–6 cm
  spacing). UTM northing reaches 1e7 m, where f32 spacing is 0.5–1 m
  (finding 12). f32 is out by an order of magnitude more than stated for
  geo-referenced frames.
- **v3-1, "infallible `Mul`/`inverse`/`interpolate`".** This conflicts with
  finiteness: two finite translations can overflow (finding 2). §1 resolves
  the conflict with a magnitude bound; without one, infallible composition and
  "never non-finite" cannot both hold.
- **v3-2 §2, "maintain normalization through composition".** The requirement
  is weaker for lookups than stated and stronger for user code. Once
  construction normalizes, a lookup chain of any realistic depth stays within
  about 1e-13 of unit. An unbounded user loop (integrating odometry with
  `Mul`) does not: its drift is linear (finding 11).

## v3 positions

### 1. Put the geometry invariants in the types

This is the one item neither note reaches, and v3 is where it becomes
possible.

**`Rotation`**: a unit quaternion with private fields.

- **Construction** rejects `|‖q‖ − 1| > UNIT_NORM_TOLERANCE`, then normalizes
  fully (one `sqrt`).
- **Every operation that produces a rotation** (composition, slerp) applies
  one sqrt-free Newton step, `q · (3 − ‖q‖²) / 2`. Measured: this holds
  `|‖q‖ − 1| ≤ 2.2e-16` across 1e9 constant-increment compositions. The step
  does not replace construction-time normalization: from `1 + 1e-6` it leaves
  1.5e-12. Static count: about 13 basic operations and no `sqrt`, against
  about 87 for a composition (28 for the Hamilton product, 56 for
  `rotate_vector`, 3 for the add). Hardware cost has not been measured.
- **`inverse` becomes the conjugate**: infallible, no normalization.
  2.1.3 kept normalization in `inverse` because accepted inputs sit at the
  tolerance edge. Under this invariant that input cannot be constructed, so
  the conjugate is the inverse to within rounding.

**Translation magnitude bound `B`**, unit-free.

- The crate assigns no units (`README.md:80`), so a physical bound would be
  wrong. `B = 1e100` exceeds the observable universe measured in Planck
  lengths (about 5.4e61), so it excludes no quantity in any unit system.
- Rotation preserves length. Composition therefore adds magnitudes, inverse
  preserves them, and interpolation never exceeds its endpoints. A derived
  value satisfies `|t| ≤ k·B·(1 + ε)`, where `k` counts the constructed values
  it derives from. Overflow needs `k ≈ 1.8e208`.
- Probe: 1e6 random hops at `|t| = 1e100` reach 9.0e102 (√n growth, bound
  1e106). `B²` is finite, so squared norms are safe.
- The invariant on derived values is `k·B`, not `B`. Insert must not
  re-check `B`, or it would reject a flattened chain. Deserializing a derived
  value beyond `B` fails, but loudly, and only for inputs that were already
  unphysical.
- Positions in `Point` need the same bound, or applying a transform to an
  unbounded point can still overflow. This is where v3-1's "seal or validate
  `Point`" earns its place.

**Result.**

- `Isometry` operations are total, and lookups cannot return non-finite
  values.
- `RegistryError::NonUnitRotation` and `NonFiniteValues` can arise only from
  construction and deserialization.
- The `From<TransformError>` canonicalization and the insert-time
  re-validation both lose their reason to exist: derived values escaping
  validity. AGENTS.md's "do not delete that check as redundant" clause is
  retired with its premise, not ignored.
- Contract changes: accepted near-unit input is stored normalized. A
  single-hop lookup still returns the stored value bit for bit, but that
  value is no longer the caller's input. Slerp bit pins are re-derived. The
  SciPy golden vectors (tolerance 1e-12) are unaffected, because the changes
  are about 1e-16 for inputs that are already unit.

**Alternative (v3-2):** accept unbounded input and check finiteness at the end
of each lookup. This also prevents `Ok(inf)`. The cost: `NonFiniteValues`
stays on the lookup error surface, and user-side `Mul` remains fallible or
unchecked. The bound is cleaner; the alternative is acceptable.

### 2. Frame-checked application, one vocabulary

Findings 4 and 7 share a cause. Frame and time checks exist only on the
`Transformable` path. Splitting geometry from metadata (v3-1 #1) makes raw
geometry easier to reach, not harder.

- **Apply results through checked methods.** A resolved result's apply
  checks the source frame and time. A cross-time result's apply checks the
  source frame and source time, then writes the target's. Taking out the
  `Isometry` is a deliberate, named step.
- **Not the un-swappable `Lookup { target, source, at }` struct (v3-1 #3).**
  Named fields make swaps rarer, not impossible, and add a type. Checked
  application catches every swap it sees.
- **One vocabulary.** Today there are four: lookup arguments say
  `target`/`source`, error fields `target_frame`/`source_frame`, the returned
  `Transform` says `parent()`/`child()`, and `Point` says `frame`. Use
  `parent`/`child` only for stored tree edges and `target`/`source` for
  anything a query returns. A lookup between siblings has no parent/child
  relation, so today's accessors mislabel it.

### 3. Resolver: topology first, attribution on the chain

Agree with v3-1 #5 and v3-2 §4. Precedence is
`UnknownFrame > Disconnected > NotFoundAt`, and `frame` always lies on the
connecting chain. Since PR #112, lookup diagnosis and `latest_common_time`
compute that chain with the same helper; with this precedence they would
also agree on which error comes first.

**The attribution half did not have to wait for v3.** No declaration said
which failing edge `frame` names: rustdoc, MIGRATION and the changelog only
promised "a sampled edge that could not serve the requested time", which both
the off-chain and the on-chain edge satisfied. Under SemVer clauses 1 and 6,
naming the on-chain edge was a compatible bug fix, and documenting it as a
guarantee made it a minor release. PR #112 fixed it in 2.2.0, itself a minor
release: diagnosis re-samples the connecting chain, the pinned test
(finding 5) asserts the on-chain frame, and a property test checks the
guarantee over random trees. The variant changes only with a custom clock
whose interpolation arithmetic can fail, where a failure off the chain used
to mask one on it.

**The precedence half does wait for v3.** `NotFoundAt` over `Disconnected`
is declared in rustdoc and MIGRATION, so flipping finding 6 is a
declared-API change.

### 4. Cross-time result as its own type

Agree with both notes. Finding 4 strengthens the case: today's design does
more than permit misuse. The checked path rejects the result outright, which
forces every correct caller onto the unchecked one. Proposed shape: frames,
both instants, the fixed frame and the geometry. It cannot be inserted,
composes with nothing, and has its own checked apply (§2).

### 5. Frame identity and lifecycle

- **Strings stay at the boundary.** Finding 8 is a name-reuse hazard strings
  already have: remove a frame, add an unrelated one under the same name, and
  the old subtree reattaches silently. That is also the documented feature.
  A public `FrameId` would inherit the hazard and add slot reuse, and would
  need generation counters. Nobody has demonstrated a need.
- **Internal interning only if measured.** `Arc<str>` is out on `thumbv6m`
  (finding 10), and `Rc<str>` gives up `Send`/`Sync`.
- **Keep descendant-preserving `remove_frame`.** It is loud until a name is
  reused, which is inherent to name identity. Rejecting would break the
  remove-and-re-add route; cascading destroys data. v3-2 §5 holds on the
  merits, not only for compatibility.

### 6. f32: measure first, with the right flags

- **Measurement protocol.** DWT cycle counts for 1-, 4- and 16-hop lookups,
  both interpolated and at an exact sample. Build with `target-cpu` set per
  core (`cortex-m4`, `cortex-m7`, `cortex-m33`) and record the FPU variant.
  Without the flag, an M7 run measures software f64 (finding 9).
- **Static operation count** from the code, as a cross-check on v3-1:
  - Per interpolated hop: about 40 basic operations and 4 transcendentals
    (`acos` plus 3 `sin`).
  - Per composition: about 87 basic operations.
  - A 4-hop interpolated lookup totals about 420 basic operations and 16
    transcendentals, plus about 70 if it inverts once. That is consistent
    with v3-1's "~550 + ~16".
  - Converting operations to time needs per-operation cycle costs nobody has
    measured, so the 0.5 ms figure (±3×, spanning both decision thresholds)
    remains an estimate.
- **Timing of the decision.** Only a registry *generic* over a scalar family
  must land in 3.0. Concrete per-scalar modules (glam-style
  `transforms::f32::…`) are additive and can arrive in 3.x. v3 should use
  concrete, non-generic scalar types anyway: E0283 defeats the default
  parameter, and a type parameter with one instantiation fails AGENTS.md. So
  the measurement informs f32 but does not block v3.
- **If f32 comes, it needs its own `B`.** f32's maximum is 3.4e38, so
  `B ≤ 1e19` keeps `B²` finite. Its documentation must say plainly that
  geo-referenced magnitudes lose metres (finding 12). The Non-Goal changes
  only by maintainer decision.

### 7. Not carried into v3

- The un-swappable `Lookup` struct (§2 covers it).
- A public `FrameId` (§5).
- A fixed-frame static-ancestry check. v3-1 already downgraded it: a constant
  dynamic edge is legitimate.
- Interpolation-gap limits and capacity limits (v3-2 §5): no demonstrated
  need yet.

Agreed with v3-1 without change: a required `max_age` argument with an
explicit `unbounded()` constructor, and `TransformError` payloads in `T`.

## `tests/v2_compatibility.rs` as the v3 contract diff

The nine tests pin the v2 behaviors a rewrite must preserve. v3 should flip
exactly the ones it intends to, and the rest stay as the acceptance gate:

| Test | Under these positions |
|---|---|
| `disconnected_lookup_preserves_sampling_failure_precedence` | flips (§3) |
| `failed_lookup_names_the_failing_edge_on_the_connecting_chain` | flipped in 2.2.0 by PR #112 (§3); stays |
| `exact_sample_preserves_signed_zero_and_accepted_rotation_norm` | rotation half flips (§1); signed zero stays |
| `attaching_an_existing_root_changes_descendant_depth` | stays |
| `unbounded_default_interpolates_across_long_gaps` | flips (required `max_age`) |
| `right_identity_retains_v2_composition_error` | open (v3-2 lists identity composition as secondary; no position here) |
| `cross_time_result_keeps_only_target_stamp_and_remains_insertable` | flips (§4) |
| `timestamp_operators_remain_usable_in_typed_downstream_code` | open (v3-1 #7; not examined here) |
| `constant_dynamic_reference_is_accepted_as_a_fixed_frame` | stays |

## Order of work

1. **2.x, non-breaking:**
   - On-chain `NotFoundAt` attribution (§3): done in 2.2.0 (PR #112).
   - Fold samples along the walk and build one `Transform` at the end (both
     notes agree; the differential trace verifies it).
   - Compare trace hashes across x86-64 and ARM64 runners (v3-1).
2. **Hardware measurement** with the §6 protocol. It informs f32 and the
   embedded claims; it does not block v3.
3. **v3:**
   - The types from §1, §2 and §4: `Rotation`, bounded translation,
     `Isometry`, resolved and cross-time results.
   - The topology-first resolver.
   - One vocabulary.
   - Required `max_age`.
   - `TransformError<T>`.
   - Acceptance: the differential trace, the golden vectors, and the
     compatibility table above.

## Keep

`latest_common_time` (no `get_latest`); validation at construction; the flat
`RegistryError`; a strict single-parent tree with explicit re-parenting; no
extrapolation; `libm` everywhere; `BTreeMap` history; caller-managed
concurrency; and the golden vectors, differential trace and compatibility
tests as the acceptance gate for the rewrite.

## Reproduction

The probes used the public API from a scratch crate with a path dependency:

- **Findings 4–8:** small registry fixtures, printed with `{:?}`.
- **Finding 11:** compose a fixed increment
  `from_wxyz(cos h, a·sin h, a·sin h, a·sin h).normalize()`, with
  `h = 5e-4` and `a = 1/√3`, 1e9 times, recording `|‖q‖ − 1|`. Repeat with
  the Newton step after each product.
- **Finding 9:** compile `pub fn dmul(a: f64, b: f64) -> f64 { a * b }` with
  `rustc -O --crate-type lib --emit asm --target thumbv7em-none-eabihf`, with
  and without `-C target-cpu=cortex-m7`, then search the output for
  `__aeabi_dmul` versus `vmul.f64`.
