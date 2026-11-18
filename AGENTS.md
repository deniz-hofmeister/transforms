# AGENTS.md

Guidance for AI agents (and new human contributors) working on this repository.
This file is normative: follow it unless the maintainer explicitly overrides it.

## What this crate is

`transforms` is a coordinate transform library for robotics and computer vision.
Its priorities, in this order:

1. **A Rust-first library.** Idiomatic, modern Rust and its design ethics come
   first: `Result` over panics, constructors over field-poking, invariants
   enforced at the API boundary rather than promised in documentation, invalid
   states made unrepresentable — or rejected with an error where they cannot be.
2. **A safety-critical mindset.** This code positions robots. The worst failure
   mode is not an error and not a panic — it is a plausible-looking wrong answer
   returned silently. Every design decision is weighed against that failure mode
   first.
3. **Only lastly, a spiritual mirror of ROS2 tf2.** Familiar concepts for
   robotics developers, zero desire to match tf2's API or feature set. Never
   justify a design with "that is how tf2 does it."

## Mindset: minimal and intentional

Every line must earn its place; the default answer to "should this exist?" is
no. Concretely:

- No speculative API. Nothing is added because a downstream user *might* want
  it — additions follow a demonstrated, concrete need.
- Prefer deletion. A fix that removes code beats one that adds code; treat a
  net-negative diff as evidence of a good change.
- Write the version without the new abstraction first. Keep a trait, wrapper,
  or type parameter only if the concrete version is demonstrably worse.
- One purpose per change. No "while I'm here" refactors, helpers, or options
  riding along with a fix.
- Question single-use generality: a helper with one caller, a type parameter
  with one instantiation, a config knob with one setting.

## Scope: what belongs in the core

The crate owns exactly the computations its invariants make dangerous to
perform anywhere else, and its interface exposes validated answers about its
state — never the raw state. Non-Goals bounds the domain; this bounds the
altitude. To place a proposed capability:

- If it can be built correctly and exactly on the public API, it belongs
  outside — the caller's code or a companion crate, never here. (An async
  wait-for-transform layer composes on the public API; out.)
- If the correct version needs sealed internals, the choice is core or
  nowhere. Never resolve it by exposing the internals instead: primitives are
  a larger commitment than the narrow query they would enable, and they hand
  every downstream the composition mistakes the query exists to prevent.
  Minimalism is measured in exported commitments, not exported functions.
- Between core and nowhere: if the substitute users would build outside is
  merely verbose, wait for demonstrated need. If it is a wrong-answer
  generator, ship the correct version — this crate cannot assume a
  well-maintained utility layer will materialize around it, and pushed-out
  invariant logic becomes hand-rolled wrong copies. (`latest_common_time` is
  core because its max-of-starts guard is precisely what external
  implementations omit.)
- No addition is precedent for its neighbors. Each point of a query family
  is argued from an invariant and its own demonstrated need —
  `latest_common_time` does not license `earliest_common_time` or coverage
  introspection.

## Architecture in five lines

- `Registry` — public entry point. Frame names are interned into slots
  behind a `hashbrown` name index; a child frame's slot holds its edge (the
  parent's slot and the edge's history). Ids never leave the registry.
  Resolution is topology-first: index walks find where two frames' chains
  meet, and only the edges between are sampled.
- `History` — crate-private, one per edge: `Static(Isometry)`, one pose
  outside the time path, or `Dynamic(BTreeMap<Timestamp, Isometry>)`. The
  kind is structural and fixed by the edge's first transform; the newest
  stamp is the map's last key, never remembered beside it.
- `geometry` — `Vector3` (plain numbers), `Rotation` (a unit quaternion by
  construction), `Transform` (translation + rotation + stamp +
  target/source frames, always valid) and `CrossTimeTransform` (the result
  of a lookup between two instants). All transform arithmetic —
  composition, inversion, interpolation — lives once, on the crate-private
  `Isometry`; `Transform` checks frames and stamps and delegates, and a
  lookup composes isometries along its walk, materializing one labelled
  result for the answer.
- `time` — one concrete `Timestamp` (a signed `i64` nanosecond newtype,
  about ±292 years around a caller-chosen epoch) and `Stamp`
  (`Static` | `At(Timestamp)`). There is no time generic.
- The crate root is flat: `geometry`, `registry` and `time` are private
  modules, and every public item is re-exported from the root.

## Non-negotiables

- `#![forbid(unsafe_code)]`. No exceptions.
- No new dependencies without maintainer approval. Middleware independence is
  the crate's reason to exist; `thiserror`, `hashbrown`, `libm` (all float
  math, on every target), and the optional, default-off `serde` are the
  entire runtime dependency list. (The `[dev-dependencies]` — `approx` for
  tolerant comparison in tests, `criterion` for benches, `proptest` for
  property tests, `serde_json` for serde roundtrips, `postcard` for the
  frozen serde wire-format bytes — are expected and do not contradict this.)
- One mode, plus `serde`: the crate is unconditionally `#![no_std]` and
  requires a heap allocator (`alloc`). There is no `std` feature and nothing
  that reads a clock. Every change must build and pass tests with and
  without `--features serde`, and build for real bare-metal targets: the
  gate builds `thumbv7em-none-eabihf` (Cortex-M4F/M7 — STM32 F4/F7/H7 flight
  controllers), `thumbv6m-none-eabi` (Cortex-M0+ — RP2040; soft float, no
  compare-and-swap atomics), and `thumbv8m.main-none-eabihf` (Cortex-M33 —
  RP2350, STM32 H5/U5); CI additionally builds
  `riscv32imc-unknown-none-elf` (ESP32-C3/C6). The only feature-gated items
  are the serde impls. `sqrt`, `sin`, and `acos` go through `libm` — never
  `f64`'s std methods — so a computed value does not depend on the target's
  math library. Slerp is pinned bit for bit in the tests as a regression
  pin.
- The README **Non-Goals** section is load-bearing, and the crate root carries
  the same list verbatim — edit both or neither. Rigid-body transforms only:
  no scaling, skew, affine, or perspective transforms, no extrapolation, no
  non-linear interpolation, no tf2 API parity, and no f32 or mixed-precision
  scalar — every coordinate and rotation is `f64`, on every target. Do
  not implement these even if an issue requests them; redirect to the
  maintainer.
- Library code must not panic on reachable paths, and the crate has no
  documented panic. Time arithmetic is checked, always: instants are
  compared with `Ord`, differenced with `abs_diff` on ordered operands, and
  moved with `Timestamp::checked_add` / `checked_sub`, which are exact over
  the whole signed range and for every `Duration`.

## Correctness invariants

Preserve these; every one of them exists because its violation once produced (or
would produce) a silent wrong answer:

- A `Transform` with frames `(target, source)` maps source-frame coordinates
  into the target frame: `p_target = R p_source + t`. Inserted into a
  registry it is the edge whose parent is `target` and whose child is
  `source`; `parent`/`child` are words for topology calls and their errors
  only, because a lookup between siblings has no parent-child relation.
- Validity is the type's, and holds for derived values as much as for built
  ones. Every `Rotation` is finite with a squared norm within
  `8 * f64::EPSILON` of 1, measured as `w*w + x*x + y*y + z*z`; every
  `Isometry`, and so every `Transform` and `CrossTimeTransform`, has a
  finite translation and a `Rotation`; every `Transform` has non-empty
  frame names. Every constructor, decoder and operation establishes this or
  returns an error, so nothing re-validates: there is no `validate`, no
  unvalidated constructor, and no insert-time numeric check. The one place
  a rotation is computed is the private `Quaternion::into_rotation` —
  components inside the band are kept bit for bit, anything else is scaled
  by `1 / sqrt(n²)` and must then be finite and inside the band.
  Construction, products and slerp all end in it; a new producer of a
  rotation that skips it breaks the invariant for everything downstream.
  The two exceptions preserve the invariant exactly and need no check: the
  `IDENTITY` constant, and the conjugate, whose squared norm is its
  operand's bit for bit. The one place an isometry is computed is
  `Isometry::new`, which checks all three translation components; the
  `IDENTITY` constant is the exception. `Transform::from_parts` is crate-internal and unchecked: its
  isometry is already valid and its names are the ones a query was answered
  for, which the registry knows and are therefore not empty.
- `Rotation::from_wxyz` rejects non-finite components and a norm further
  than `1e-6` from 1, then applies the rule above. The band is what makes
  rebuilding a rotation from its own components a bitwise fixed point — and
  with it `decode(encode(t)) == t`. Do not replace it with an unconditional
  renormalization or a Newton step on every product: both change bits of
  values that were already valid.
- Numerical failures are errors, never values: `NonFinite` for a non-finite
  result, `NonUnitRotation { norm }` for a finite rotation that fails the
  postcondition. Neither may escape inside a successful result. There is no
  magnitude bound on translations — a bound is not closed under
  composition.
- Composition `a.compose(&b)` requires `a.source() == b.target()` — no other
  pairing composes — and reports `FrameMismatch` otherwise. Two dynamic
  stamps must then be equal (`StampMismatch`, carrying both exact
  instants); a static operand adopts the other's stamp. The frame check
  comes first.
- Inversion is the conjugate rotation with a corrected translation:
  `(-1 / n²) * rotate(conjugate(q), t)`. It needs no square root and its
  only failure is `NonFinite`. Do not divide the quaternion itself by `n²`:
  that is a different algorithm and leaves the band.
- Equal frame names are legal on a `Transform`, with any geometry: a pose
  error `est.inverse()?.compose(&truth)?` is a `base`-to-`base` transform
  that is not the identity, so "self-named implies identity" cannot be a
  type invariant and no constructor or decoder enforces it. The registry
  rejects every self-named transform as `InsertError::SelfEdge`.
- A child frame's edge is static **xor** dynamic. The first insert fixes the
  kind; a mismatched later insert must fail with
  `InsertError::KindMismatch`. Staticness is `Stamp::Static` on the
  transform — no timestamp value is reserved. `Stamp` is deliberately
  unordered (`PartialEq`/`Eq`/`Hash` only): `Static` denotes all time, so
  any ordering would rank an eternal transform against real instants and
  make `max_by_key(|tf| tf.stamp())` silently pick the wrong sample. Its
  serde encoding is explicitly tagged (`"Static"` / `{"At": t}`), variants
  in that order, for the same reason: under an `Option`-shaped encoding a
  `null` *and* a dropped `stamp` field both decode as `Static`, so a
  producer that lost a stamp minted a transform the registry then served at
  every instant. Both are decode errors; do not trade the derive back for
  an optional encoding, and do not reorder the variants.
- The frame tree is strict: the first insert also pins a child frame's parent
  (`insert` never changes it — that fails with `ParentMismatch`), a frame
  cannot be its own parent (`SelfEdge`), and every edge entering the tree is
  cycle-checked before it commits, failing with `Cycle`. Empty frame names
  never reach the registry: `Transform::new` and decoding reject them
  (`TransformError::EmptyFrameName`), and an empty name in a query is an
  unknown frame. Re-parenting is a deliberate act with its own call:
  `Registry::reparent` validates everything — unknown/root split
  (`UnknownFrame`, `NoParent`), unchanged parent, self edge, cycle, kind —
  before mutating anything, preserves the frame's static-xor-dynamic kind
  (a cross-kind seed fails with `KindMismatch`), and drops the frame's
  stored history — loudly for a dynamic frame (coverage collapses to the
  seed), retroactively and silently for a static one (the replaced pose
  answers every instant, as any static re-publish does).
  `Registry::remove_frame` plus re-adding remains the route for kind
  changes and keep-the-history moves. Chain resolution relies on the
  result — the topology is acyclic at every instant and time-invariant for
  the duration of any query. Every structural insert error names its frame,
  and every `frame` field of `InsertError` is the child.
- Known names are defined by stored edges and nothing else: a name is known
  iff a stored edge names it as child or as parent when the call returns. A
  drained edge still counts. Slots implement that rule and do not get to
  bend it: a slot is occupied iff its frame has an edge or at least one
  child edge names it, the name index holds exactly the occupied slots, and
  `remove_frame` and `reparent` release a slot in the same call that removes
  its last reference. A removed child stays known while it has children; a
  parent named by no other edge becomes unknown. A failed insert, batch or
  `reparent` makes no name known. Reclamation never renumbers an occupied
  slot. An empty registry therefore answers every query with
  `UnknownFrame`, equal names included, and a lone frame cannot be made
  known.
- `Registry::insert_all` is all-or-nothing. On error the registry is
  unchanged: nothing from the batch is stored, no sample is evicted and no
  name becomes known. On success it holds what inserting the batch in order
  with `insert` would leave, and the error is the first one that loop would
  report, with its zero-based slice index in `BatchError`. Every transform
  is checked before any is stored — against the stored frames, and against
  the pins the batch itself has staged (parent, kind and running newest
  stamp per child frame), so later members see earlier members' edges, the
  cycle walk sees them too, and expiry is measured against the stamp the
  loop would have reached. Checking first and storing after is sound only
  because a check reads pins and nothing else: `Registry::check` must never
  read stored samples beyond a frame's newest stamp, or an earlier member of
  the batch could invalidate a check already passed and the batch would
  fail half-stored. `insert` is the same check with nothing staged, then the
  same commit.
- A lookup must return a transform whose `target`/`source` match the
  requested frames exactly; a chain that resolves only partway must return
  an error, never a partial result. Diagnosis has one order for both
  lookups, decided before any geometry can mask it: an unknown name
  (`UnknownFrame`, checked on the names in the order target, source, fixed,
  before any leg is formed), then disconnected topology (`Disconnected`,
  naming the pair that is actually disconnected — in a cross-time lookup an
  endpoint and the fixed frame), then coverage, then numerical failure.
  Coverage is `NoData` for an edge holding nothing and `OutOfRange` —
  carrying the frame, the requested instant and the covered range — for an
  edge holding samples elsewhere; they are two variants so that a caller
  cannot read the first as a timing problem. A coverage error names an edge
  the connecting chain crosses, the lowest failing one on the target's side,
  else on the source's: edges above the meeting point are never sampled and
  never reported. A numerical failure must not end diagnosis before all
  required coverage is checked — the cold `diagnose` path re-reads the
  whole chain for coverage first. Results always carry
  `Stamp::At(requested)` (also over static chains), and a *known* frame
  relative to itself is the identity.
- `Registry::lookup_across` always resolves both legs against the fixed
  frame — also when the endpoint names and the two times coincide — with the
  target leg first within each diagnosis class; only a leg whose endpoint
  is the fixed frame is the identity, and then the result is the other
  leg alone: as resolved when the target is the fixed frame, inverted once
  when the source is. Its result is a `CrossTimeTransform`, not a
  `Transform`: it carries both instants and can be neither composed,
  inserted nor serialized. The fixed frame is the caller's assertion and is
  not recorded.
- `latest_common_time` diagnoses with the same `UnknownFrame` and
  `Disconnected`, returns `Static` when the connecting chain has no dynamic
  edge (a known frame relative to itself included), reports `NoData` for a
  dynamic edge holding nothing, and refuses with `NoCommonTime` — naming an
  edge whose coverage starts after `latest_possible`, the minimum newest
  stamp of the chain's dynamic edges. It must never return an instant some
  edge cannot serve — min of the dynamic edges' newest samples, guarded by
  max of their starts, over only the edges the connecting chain crosses. It
  evaluates no geometry and never returns `OutOfRange`, `NonFinite` or
  `NonUnitRotation`; the lookups never return `NoCommonTime`.
- Errors are grouped by operation family: `TransformError` (building,
  composing, inverting), `InsertError` with `BatchError` (changing a
  registry), `LookupError` (querying one). Each enum stays **flat** — one
  `match` reaches every cause and every payload of that family — and a
  condition has one spelling per family. Each method's rustdoc lists the
  subset it can return. Struct variants are `#[non_exhaustive]`. Time
  payloads are exact `Timestamp`s, formatted by `Timestamp`'s `Display`,
  which cannot fail; nothing converts a stamp to floating-point seconds.
  The private `NumericError`, `Gap`, `SampleError`, `Fault` and `Failure`
  are internal steps toward those enums and every conversion out of them
  is total.
- Interpolation happens only between stored samples; there is no
  extrapolation. A lookup outside an edge's covered range fails with
  `LookupError::OutOfRange`. The ratio is computed from integer nanosecond
  differences (`Timestamp::ratio_between`), never from seconds, and a
  request at a stored instant returns the stored isometry without
  interpolating.
- Buffer expiry is data-driven and has one boundary: the frame's newest
  stored stamp − `max_age`, computed exactly by `Timestamp::checked_sub`
  (only under `Registry::with_max_age`). On insert, stored samples older
  than it are removed, and an insert that is itself older is refused with
  `InsertError::Expired` and stores nothing — `Ok(())` for a sample its own
  insert discards is a false return value. A sample on the boundary is
  kept; where the subtraction is not representable, nothing is removed or
  refused. Expiry is the last insert check, so the error implies the sample
  has the frame's parent and kind. `insert_all` measures each member against
  its frame's running newest stamp, so one expired member rejects the whole
  batch. The `reparent` seed is exempt — it replaces the history it would
  be measured against. The error is not clock-reset detection: the sample
  that moves the newest stamp ahead is accepted, and a backward step
  smaller than `max_age` is stored silently. Wall-clock time is never
  consulted.
- Manual cleanup (`Registry::remove_samples_through`, and
  `History::remove_through` under it) removes every dynamic sample at or
  before the cutoff, in every frame: the cutoff is inclusive, so a frame's
  newest stamp is always removable and `Timestamp::MAX` clears all dynamic
  history. Expiry is the opposite on purpose: it keeps the sample exactly on
  its boundary. Cleanup never touches static edges — a static transform is
  valid for all time — and never releases a frame: a drained edge keeps its
  pinned parent, its kind and its name, so cleanup cannot re-open a frame
  for re-parenting or a change of kind, and it keeps no hidden newest
  stamp. `Registry::remove_frame` is the only release on the cleanup path;
  `Registry::reparent` also releases a pin — by replacing the edge
  wholesale, never by draining it.
- A lookup composes each half of the resolved chain in the direction the walk
  produced it and inverts at most once — never once per hop; a cross-time
  lookup resolves two such legs. A lookup toward an ancestor
  (`lookup("map", "lidar", t)`, the documented direction) therefore inverts
  nothing, and a single-hop lookup at a stored instant returns that stored
  geometry bit for bit. Reintroducing a per-element inversion is both a 2x
  cost and a loss of that exactness.
- `==` on geometry types is exact. Use `approx::assert_abs_diff_eq!` on the
  `f64` components for tolerant comparison of computed results in tests;
  never introduce epsilon-based `PartialEq`/`Eq` (it violates the trait
  contracts), and do not implement `approx` traits on the public types —
  `approx` is a dev-dependency.
- The serde layout is written out explicitly and frozen by tests, never
  inherited from declaration order: `Transform` is `target`, `source`,
  `translation`, `rotation`, `stamp`; `Rotation` is `w`, `x`, `y`, `z`;
  `Timestamp` is the bare integer. `Rotation` and `Transform` decode
  through `from_wxyz` and `Transform::new`; unknown fields are an error.
  The name-first order makes 2.x postcard data misalign, so that it
  usually fails the rotation check instead of decoding with wrong frames
  and stamps; `tests/serde.rs` pins that for the frozen 2.4 fixtures. It is
  a regression pin, not a schema detector: a 2.x payload whose bytes form
  a valid transform still decodes. Persisted 2.x data is migrated with the
  2.x decoder; do not add heuristic dual decoding.
- All public error enums are `#[non_exhaustive]`; downstream matches need a
  wildcard arm, and new variants may be added in minor releases.
- No arithmetic operator impls and no `Default` on any public type. Only the
  functions marked `const` are promised const.

When you fix a correctness bug, ship the regression test that fails on the old
code in the same commit.

## Style

The gate below machine-checks lints, formatting, and docs; everything else in
this section is convention, enforced in review — follow it anyway.

- Edition 2024, `rust-version = "1.85"` — the edition floor, verified by a
  CI job; criterion is held at 0.7 to keep it true (see Cargo.toml and the
  dependabot ignore rule). `#![warn(missing_docs)]` and
  `#![warn(clippy::pedantic)]` must stay at **zero warnings** in both feature
  modes. Never add a new `#[allow]` to get green; fix the cause or ask. The
  one standing allowance is `clippy::similar_names` in tests (where
  `t_a_b`-style names are domain-correct), set once at the crate root under
  `cfg(test)` — do not remove it, and do not treat it as precedent. The
  tree carries no other `allow`: tests that assert exactness compare bit
  patterns or whole values, and the one integer-to-float conversion
  (`Timestamp::ratio_between`) is written without a lossy cast.
- Construction goes through constructors everywhere — tests, examples, docs:
  `Transform::new(target, source, translation, rotation, stamp)` (fallible),
  `Vector3::new(x, y, z)`, `Rotation::from_wxyz(w, x, y, z)` (fallible) /
  `Rotation::IDENTITY`, `Timestamp::from_nanos(n)`,
  `Registry::with_max_age(d)` / `Registry::unbounded()`. `Transform`,
  `Rotation` and `CrossTimeTransform` have private fields and no unchecked
  public constructor; a test cannot build an invalid one, and must not be
  given a way to. `Vector3` keeps its public fields — it is plain numbers
  with no invariant to protect.
- Float literals carry digits on both sides of the dot: `1.0`, never `1.`.
- Doc comments come first, then attributes (`#[cfg]`, `#[must_use]`,
  `#[inline]`). Constructors get bare `#[must_use]` — except the fallible
  ones, where `Result` already carries it and clippy's `double_must_use`
  fires; pure transforming operations get the std phrasing
  `#[must_use = "this returns the result of the operation, without modifying the original"]`.
- Rustdoc: no `# Arguments` / `# Returns` / `# Fields` sections — fold anything
  non-obvious into prose. Keep `# Errors` and `# Panics`; `# Examples` comes
  last. No hand-maintained inventories of a module's contents (rustdoc generates
  those). Doc statements must describe actual behavior, not intent; doc examples
  use `.unwrap()` and must compile (they run as doc tests).
- Errors: `Display` messages are lowercase, single-clause, no trailing period
  (Rust API guideline C-GOOD-ERR). Every variant carries a doc comment. Error
  types live in a private `mod error;` re-exported via `pub use`.
- Tests: no logging (the crate has no logging dependency; examples assert
  and print with `println!`), `assert_eq!`/`assert_ne!` over
  `assert!(a == b)`, and
  behavior-descriptive snake_case names. Tests are deterministic: fixed
  `Timestamp::from_nanos` fixtures and nothing that reads a clock.
  `Timestamp::from_nanos(0)` is fine for any fixture — `t = 0` is an
  ordinary dynamic instant; static transforms carry `Stamp::Static`.
  Invariants ideally get a property test in `tests/properties.rs` alongside
  the example-based ones; the property tests run on one pinned seed, so
  every gate run exercises the same cases.
- `tests/golden_vectors.rs` is the one place whose expected values do *not*
  come from this crate: they are literal digits computed with SciPy, and
  they are what would catch a convention flipped consistently — a
  transposed rotation, a swapped quaternion product, `(target, source)` read
  backwards — which every self-referential assertion passes. Never
  regenerate those numbers from the crate's own arithmetic; re-derive them
  from outside, deliberately, or the layer stops existing.
- Strings into `String` fields: `"a".into()`. Format strings use inline
  captures: `{x}` / `{x:?}`.

## Definition of done — the verification gate

Run `tests/test_all.sh` to completion before a change is complete. It requires
an unpinned recent nightly toolchain with rustfmt, clippy, and the three ARM
targets below. On rustup installations:

```sh
rustup target add --toolchain nightly thumbv7em-none-eabihf thumbv6m-none-eabi thumbv8m.main-none-eabihf
rustup run nightly tests/test_all.sh
```

With a Nix or distro nightly on PATH, run the script directly. It checks builds,
tests and clippy with and without `serde`; formatting; rustdoc
including docs.rs settings; all examples; benchmark smoke tests; and ARM
bare-metal builds with and without serde. Any compiler warning fails it. It
prints `GATE PASSED` only after all checks finish. Never weaken or bypass it.

CI runs the same script. Change the gate in the script, and keep its clippy
feature combinations aligned with CI. CI additionally checks stable clippy,
native x86_64/ARM64 tests, Rust 1.85, RISC-V compilation, `cargo audit`, and
published-API compatibility in both feature combinations. Stable clippy
is the arbiter when it differs from nightly — a lint can relax on nightly
before stable follows (`float_cmp` against `f64::INFINITY` did), so a green
local gate makes green CI likely, not guaranteed; reproduce with
`rustup run stable cargo clippy` and the relevant feature flags.

Update user-facing docs with the behavior they describe. Keep the README
brief, link to rustdoc for API contracts, and update examples and the changelog
when relevant. Documentation drift is a bug.

## API stability

- Breaking changes (signatures, enum variants, trait bounds, public paths) land
  only at major versions and only with explicit maintainer sign-off per release
  — a past approval does not carry forward.
- Additive API (new methods, new trait impls, adding `const` or `#[must_use]`)
  is acceptable, but anything that grows the public surface deserves a note to
  the maintainer.

## Commits and disclosure

- Branch names: `bugfix/<topic>`, `feature/<topic>`, `docs/<topic>`
  (kebab-case). Release branches (`release/vX.Y.Z`) are cut by the maintainer.
- Commit messages: imperative summary line, then a body explaining *why*.
- **AI disclosure (required):** every commit authored with AI assistance must
  carry a Linux-kernel-style trailer identifying the agent and model:

  ```
  Assisted-by: <AgentName>:<model-version>
  ```

  for example `Assisted-by: Claude:claude-fable-5`. The model is the exact
  model ID you are running as — never one copied from this file, an earlier
  commit, or a config. This is assistance, not authorship: an AI agent must
  never add `Signed-off-by:` (only humans can certify the origin of a
  contribution). Harness-added trailers may coexist, but `Assisted-by:` must
  be present. The human maintainer reviews assisted changes and decides what
  is merged; that review is not a warranty (see `LICENSE` and the
  "AI-Assisted Development" section of the README).
- The rule is enforced, not requested. Enable the repository's hooks once
  per clone with `git config core.hooksPath .githooks`. `.githooks/commit-msg`
  checks the final message of every commit an AI agent makes (detected from
  the environment variables agents set) or that carries an AI mark: it needs
  a well-formed `Assisted-by:` trailer, and an agent's commit must not carry
  `Signed-off-by:`. CI runs the same script on every new commit. In Claude
  Code, Codex, and Cursor, `.githooks/agent-guard` also runs before every
  shell command (wired in `.claude/settings.json`, `.codex/config.toml`, and
  `.cursor/hooks.json`): it blocks a commit unless its command line carries
  the trailer naming the running model, which it reads from the harness, and
  blocks commits while the hooks are disabled, `--no-verify`, and any other
  change to `core.hooksPath`. Give the message with `-m` or a heredoc; a
  message the guard cannot see (`-F`, `--amend --no-edit`) is blocked. Never
  disable, edit, or route around these hooks unless the maintainer explicitly
  asks.

## When in doubt

- Prefer a loud error over a silent guess — in code and in your own workflow.
- If a change requires weakening the gate, widening the public API, adding a
  dependency, or touching the Non-Goals, stop and ask the maintainer.
- Read the git history of the code you are changing; several invariants above
  are scars from specific bugs, and the commit messages explain them.

## Releasing

Releases are cut by the maintainer. Release prep is one commit on a prep
branch, `docs/vX.Y.Z-release-prep`, cut from master's head and put through a
pull request, so the `master` ruleset's required checks pass on it before
master moves. Master then takes that commit by fast-forward — never through
the merge button, whose three methods each put a commit on master that no
check ran on: a merge commit, or a squashed or rebased copy under a new
hash. The release branch is then cut from that commit and never receives a
commit of its own: the branch head, the tag, the published crate, and the
GitHub release all name the commit master holds, and that is the commit the
gate and the pull request's CI passed. The checklist, in order:

- Finalize `CHANGELOG.md`: rename `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD`
  with the release date, add the `[X.Y.Z]` comparison link against the
  previous tag, repoint `[Unreleased]` to compare from `vX.Y.Z`, and verify
  `MIGRATION.md`. Both ship in the immutable crate package.
- Set the `version` in `Cargo.toml`, regenerate `Cargo.lock` so it records
  that version (any `cargo build` after the bump does), bump the version
  pins in the README installation snippets, and update the README's
  "What's new" to lead with the release.
- Point the README's example links at the release tag, as absolute
  `https://github.com/deniz-hofmeister/transforms/blob/vX.Y.Z/examples/<name>.rs`
  URLs: crates.io resolves relative README links against the repository's
  default branch, so a later rename breaks every published page (verified
  for 2.3.0 on 2026-11-12).
- Commit all of it together on the prep branch as
  `Finalize the changelog and release notes for vX.Y.Z`, with a body that
  says what the commit changed. Do not push yet. `cargo publish` refuses a
  dirty tree, so every check below runs on that commit.
- Run the full verification gate (`tests/test_all.sh`).
- Run `cargo semver-checks check-release --baseline-rev <previous tag>` in
  each of the two feature combinations the `semver` CI job runs, and
  confirm the diff is exactly the changelogged one. Against a baseline the
  release already majors over it enumerates nothing — every breaking lint is
  skipped as permitted, and the run proves only that the tooling works and
  that the declared bump covers whatever changed. To inspect the breaking
  diff, run once with the version temporarily set to a patch bump on the
  baseline.
- `cargo publish --dry-run` and inspect the file list — nothing missing,
  nothing that should not ship.
- Push the prep branch, open a pull request against master, and wait for
  every CI job on it to pass. If a check fails, fix it on the prep branch
  and restart from the gate; if master moves, rebase onto it and restart
  from the gate likewise.
- Fast-forward master to the pull request's head:
  `git push origin docs/vX.Y.Z-release-prep:master`. Wait for the CI run on
  that push as well — the pull request's jobs ran on GitHub's merge ref,
  this one runs on the commit itself; never tag a commit CI has not passed.
- Cut `release/vX.Y.Z` from that commit and push it. Nothing is ever
  committed to it.
- Tag `vX.Y.Z` on the branch head as an annotated tag whose message is
  `vX.Y.Z`, and push the tag.
- `cargo publish` from a clean checkout of `release/vX.Y.Z`. It runs before
  the GitHub release because it is the one step that cannot be undone: a
  GitHub release announces the version, and must not announce one crates.io
  does not serve.
- Create the GitHub release for the tag, titled `vX.Y.Z`, with that
  version's `CHANGELOG.md` section, minus its heading, as the notes; mark
  pre-releases as such and the current stable release as latest.
- The `semver` CI job checks against the latest published release; its
  baseline advances automatically after publication.
