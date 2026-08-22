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

## Architecture in five lines

- `Registry` — public entry point; a `HashMap<String, Buffer>` keyed by **child**
  frame name, plus chain resolution between arbitrary frames.
- `Buffer` — crate-private, one per child frame: a `BTreeMap<T, Transform<T>>`
  ordered by timestamp, with interpolation between stored samples. Only
  `Registry` reaches it; it is not part of the public API.
- `geometry` — `Transform` (translation + rotation + timestamp + parent/child
  frames), `Vector3`, `Quaternion`, and `Point` as the reference implementation
  of the `Transformable`/`Localized` traits.
- `time` — the `TimePoint` trait (`Copy + Ord + Debug` plus `duration_since`,
  `checked_sub`, `as_seconds_lossy` — nothing the core does not call) and the
  default `Timestamp` (u64 nanoseconds, ~584 years);
  `std::time::SystemTime` is supported behind the `std` feature.

## Non-negotiables

- `#![forbid(unsafe_code)]`. No exceptions.
- No new dependencies without maintainer approval. Middleware independence is
  the crate's reason to exist; `thiserror`, `approx`, `hashbrown`, `libm`
  (all float math, in every feature mode), and the optional, default-off `serde`
  are the entire runtime dependency list. (The `[dev-dependencies]` —
  `log`/`env_logger` for examples, `tokio` for the async example, `criterion`
  for benches, `proptest` for property tests, `serde_json` for serde
  roundtrips, `postcard` for the frozen serde wire-format bytes — are
  expected and do not contradict this.)
- `no_std` parity: every change must build and pass tests with
  `--no-default-features`, and build for real bare-metal targets: the gate
  builds `thumbv7em-none-eabihf` (Cortex-M4F/M7 — STM32 F4/F7/H7 flight
  controllers), `thumbv6m-none-eabi` (Cortex-M0+ — RP2040; soft float, no
  compare-and-swap atomics), and `thumbv8m.main-none-eabihf` (Cortex-M33 —
  RP2350, STM32 H5/U5); CI additionally builds
  `riscv32imc-unknown-none-elf` (ESP32-C3/C6). `no_std` requires a heap
  allocator (`alloc`).
  Features must be additive: the same API exists in both modes; the only
  feature-gated items are `Timestamp::now()`, `Timestamp::try_now()`, the
  `SystemTime` time type (`std`), and the serde derives (`serde`,
  default-off). Additive also means numerically identical: `sqrt`, `sin`, and
  `acos` go through `libm` in both modes — never `f64`'s std methods — so
  enabling `std` changes which API exists, never a computed value. Slerp is
  pinned bit for bit in the tests to keep it that way.
- The README **Non-Goals** section is load-bearing, and the crate root carries
  the same list verbatim — edit both or neither. Rigid-body transforms only:
  no scaling, skew, affine, or perspective transforms, no extrapolation, no
  non-linear interpolation, no tf2 API parity, and no f32 or mixed-precision
  scalar — every coordinate and rotation is `f64`, on every target, which is
  why the README publishes a supported envelope instead of a rate claim. Do
  not implement these even if an issue requests them; redirect to the
  maintainer.
- Library code must not panic on reachable paths. The only documented panic is
  `Timestamp::now()` on a system clock outside the representable range (before
  the Unix epoch, or beyond the u64 nanosecond range in 2554); `try_now`
  returns those as errors. Time arithmetic is checked, always.

## Correctness invariants

Preserve these; every one of them exists because its violation once produced (or
would produce) a silent wrong answer:

- A `Transform` with frames `(parent, child)` maps child-frame coordinates into
  the parent frame.
- Composition `t_a_b * t_b_c = t_a_c` requires `lhs.child == rhs.parent` —
  no other pairing composes. Timestamps must be equal unless one operand is
  static.
- A child frame's buffer is static **xor** dynamic. The first insert fixes the
  kind; a mismatched later insert must fail with
  `RegistryError::StaticDynamicConflict`. Staticness is `Stamp::Static` on the
  transform — no timestamp value is reserved. `Stamp` is deliberately
  unordered (`PartialEq`/`Eq` only): `Static` denotes all time, so any
  ordering would rank an eternal transform against real instants and make
  `max_by_key(|tf| tf.timestamp)` silently pick the wrong sample. Its serde
  encoding is explicitly tagged (`{"At": t}` / `"Static"`) for the same
  reason: under an `Option`-shaped encoding a `null` *and* a dropped
  `timestamp` field both decode as `Static`, so a producer that lost a stamp
  minted a transform the registry then served at every instant. Both are
  decode errors; do not trade the derive back for an optional encoding.
- The frame tree is strict: the first insert also pins a child frame's parent
  (re-parenting fails with `ReparentingNotSupported`; `Registry::remove_frame`
  is the escape hatch), a frame cannot be its own parent, and inserts that
  would close a cycle fail with `CycleDetected`. Chain resolution relies on
  this — the topology is time-invariant and acyclic.
- A lookup must return a transform whose `parent`/`child` match the requested
  frames exactly; a chain that resolves only partway must return an error,
  never a partial result — `UnknownFrame` for a frame that exists nowhere,
  `Disconnected` for two known frames no chain connects, and `NotFoundAt`
  for a known frame that cannot serve the requested time — carrying that
  frame, the `requested: T` instant, and `covered: Option<(T, T)>`, which
  separates a gap in data the frame holds (`Some`, the covered range) from a
  frame holding nothing at all (`None`); a caller must not read the second
  as a timing problem. Results always carry the requested timestamp (also
  over static chains), and a frame relative to itself is the identity.
- Every `Registry` call reports `RegistryError<T>` and it stays **flat**:
  one `match` reaches every cause and every payload. `TransformError` is
  pure geometry and time, and the single `RegistryError::TransformError` arm
  that wraps it must never carry `NonUnitRotation` or `NonFiniteValues` —
  `From<TransformError> for RegistryError` canonicalizes those two into
  their flat variants, so a condition never has two spellings a caller could
  match one of and miss the other. Lookup payloads stay in the caller's time
  type `T`; the conversion to seconds happens in `Display`, nowhere else.
  The buffer's own error types (`InsertError`, `GetError<T>`) are internal
  and split by operation so that every conversion into `RegistryError` is
  total — one enum for both would force an unreachable arm on each.
- Interpolation happens only between stored samples; there is no
  extrapolation. A `Registry` lookup that falls outside a frame's covered
  range fails with `RegistryError::NotFoundAt` carrying
  `covered: Some(range)` — no registry path produces
  `TransformError::TimestampOutOfRange`; its only producer is
  `Transform::interpolate`, which the buffer never calls out of range.
- Error formatting goes through `TimePoint::as_seconds_lossy` and cannot fail;
  a conversion error must never mask the error being reported.
- Buffer expiry is data-driven: entries older than
  (latest **inserted** timestamp − `max_age`) are removed on insert (only for
  buffers built `dynamic_with_max_age`). Wall-clock time is never consulted.
  Manual cleanup (`Registry::remove_transforms_before`, and the internal
  `Buffer::remove_before` under it) never touches static buffers — a static
  transform is valid for all time — and never releases a frame: a drained
  buffer keeps its pinned parent and its static/dynamic kind, so cleanup cannot
  re-open a frame for re-parenting or a change of kind.
  `Registry::remove_frame` is the only release.
- Transforms are validated where they are built: `Transform::new`,
  `Transform::static_between` and the `Deserialize` impl all run
  `Transform::validate`, rejecting non-finite components and rotations whose
  norm deviates from 1 by more than `geometry::UNIT_NORM_TOLERANCE`. A
  denormalized rotation would silently corrupt every lookup it takes part in.
  The fields are private so a built transform cannot be edited back out of
  that guarantee; the crate-internal `Transform::unvalidated` exists only for
  values derived from already-valid ones, and every new caller of it must be
  able to name the validated transform its inputs came from.
- Values *derived* from valid transforms — `Mul`, `inverse`, `interpolate`,
  every registry lookup — are deliberately not re-validated: rotation norms
  drift a few ulps per composition, so re-checking a long chain would reject
  legitimate results. `Transform::validate` stays public for transforms of
  uncontrolled provenance, and `Transformable` documents that as its
  precondition.
- Because of the clause above, "a `Transform` is valid by construction" is
  true of built transforms only, never of derived ones — composing two
  rotations at the edge of `UNIT_NORM_TOLERANCE` walks past it, and extreme
  magnitudes overflow a translation to infinity. `Buffer::insert`, under
  `Registry::add_transform`, therefore runs `Transform::validate` on
  everything entering storage. That is the last boundary before a value
  starts answering lookups, and the ordinary "flatten a chain, re-publish it"
  pattern crosses it with a transform nothing else checked. Do not delete
  that check as redundant with the constructors; it is not.
- Rotations are expected to be unit quaternions; `Quaternion::from_wxyz` does
  not normalize. Anything that inverts a rotation must normalize first (see
  `Transform::inverse`, which also rejects a non-finite inverted translation).
- A lookup composes each half of the resolved chain in the direction the walk
  produced it and inverts at most once — never once per hop. A lookup toward
  an ancestor (`get_transform("map", "lidar", t)`, the documented direction)
  therefore inverts nothing, and a single-hop lookup at a stored timestamp
  returns that stored transform bit for bit. Reintroducing a per-element
  inversion is both a 2x cost and a loss of that exactness.
- `==` on geometry types is exact. Use `approx::assert_abs_diff_eq!` for
  tolerant comparison of computed results; never reintroduce epsilon-based
  `PartialEq`/`Eq` (it violates the trait contracts).
- All public error enums are `#[non_exhaustive]`; downstream matches need a
  wildcard arm, and new variants may be added in minor releases.

When you fix a correctness bug, ship the regression test that fails on the old
code in the same commit.

## Style

The gate below machine-checks lints, formatting, and docs; everything else in
this section is convention, enforced in review — follow it anyway.

- Edition 2024, `rust-version = "1.86"` (verified by a CI job). `#![warn(missing_docs)]` and
  `#![warn(clippy::pedantic)]` must stay at **zero warnings** in both feature
  modes. Never add a new `#[allow]` to get green; fix the cause or ask. The
  standing allowances are `clippy::similar_names` in tests (where `t_a_b`-style
  names are domain-correct), a handful of narrowly-scoped `clippy::cast_*`
  allows on the numeric conversions in `src/time/timestamp/`, the scoped
  `clippy::expect_used` allow on `Timestamp::now`'s documented panic, and the
  per-test `clippy::float_cmp` allows where exactness is the property under
  test (a reported error payload, a last-write-wins upsert — the compared
  values are exactly representable) — do not remove them, and do not treat
  them as precedent.
- Construction goes through constructors everywhere — tests, examples, docs:
  `Transform::new(parent, child, translation, rotation, stamp)` /
  `Transform::static_between(..)` (both fallible),
  `Point::new(position, orientation, timestamp, frame)`,
  `Vector3::new/zero`, `Quaternion::from_wxyz(w, x, y, z)` /
  `Quaternion::identity()`, `Timestamp::zero()` / `Timestamp::from_nanos()`.
  `Transform` and `Point` are `#[non_exhaustive]`; `Transform`'s fields are
  private, and a test that needs a deliberately invalid transform uses the
  crate-internal `Transform::unvalidated` rather than a struct literal.
  `Vector3` and `Quaternion` keep their public fields — they are plain
  numbers with no invariant to protect.
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
- Tests: no logging (no `env_logger`, no `debug!` — logging belongs in
  `examples/`), `assert_eq!`/`assert_ne!` over `assert!(a == b)`, and
  behavior-descriptive snake_case names. Tests are deterministic: fixed
  `Timestamp::from_nanos` fixtures, never `Timestamp::now()` (one dedicated
  std-only smoke test covers `now()` itself). `Timestamp::zero()` is fine for
  any fixture — `t = 0` is an ordinary dynamic instant; static transforms
  carry `Stamp::Static`. Invariants ideally get a property test in
  `tests/properties.rs` alongside the example-based ones.
- `tests/golden_vectors.rs` is the one place whose expected values do *not*
  come from this crate: they are literal digits computed with SciPy, and
  they are what would catch a convention flipped consistently — a
  transposed rotation, a swapped quaternion product, `(parent, child)` read
  backwards — which every self-referential assertion passes. Never
  regenerate those numbers from the crate's own arithmetic; re-derive them
  from outside, deliberately, or the layer stops existing.
- Strings into `String` fields: `"a".into()`. Format strings use inline
  captures: `{x}` / `{x:?}`.

## Definition of done — the verification gate

All of the following must pass before a change is complete
(`tests/test_all.sh` runs the whole gate). The gate requires a **nightly**
toolchain and crashes explicitly otherwise: rustfmt.toml uses nightly-only
options. No particular nightly is pinned — any recent one will do, however
Rust was installed (rustup, Nix, or a distro package), so two machines may
well be running different nightlies. Stable and MSRV verification is CI's
job. Nightly clippy usually anticipates stable's lints, but a lint can also
relax on nightly before stable follows — `float_cmp` stopped firing on
comparisons against `f64::INFINITY` there while stable 1.98 still flags them
— so a green local gate makes green CI clippy likely, not guaranteed; when
the two disagree, CI's stable clippy is the arbiter, and
`rustup run stable cargo clippy` reproduces it locally. Keep the script's
lint list and CI's in step regardless: the moment the script lints fewer
feature combinations than CI does, a lint can land in CI that nobody could
have seen locally.

```bash
cargo build                                         # both modes build first
cargo build --no-default-features
cargo test
cargo test --no-default-features
cargo test --features serde
cargo test --no-default-features --features serde
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --features serde -- -D warnings
cargo clippy --all-targets --no-default-features --features serde -- -D warnings
cargo fmt --check                                   # nightly rustfmt (see above)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo doc --no-deps --all-features   # the docs.rs configuration
cargo run --example std_minimal                     # and the other std examples
cargo run --example no_std_minimal --no-default-features   # and the other no_std examples
cargo bench -- --test
cargo bench --no-default-features -- --test         # CI also builds no_std benches
cargo build --no-default-features --target thumbv7em-none-eabihf   # real no_std proof
cargo build --no-default-features --target thumbv6m-none-eabi      # Cortex-M0+: soft float, no CAS
cargo build --no-default-features --target thumbv8m.main-none-eabihf
cargo build --no-default-features --features serde --target thumbv7em-none-eabihf   # serde stays std-free
cargo build --no-default-features --features serde --target thumbv6m-none-eabi
cargo build --no-default-features --features serde --target thumbv8m.main-none-eabihf
```

(On rustup machines: `rustup run nightly tests/test_all.sh`, and
`rustup target add <target>` once per target, if missing. CI also builds
`riscv32imc-unknown-none-elf`.)
CI runs this same script verbatim in its `gate` job, so the script is the
single source of truth for what the gate is — extend the script, not the
workflow.
CI additionally runs the test suite natively on ARM64 as well as x86_64
(the Raspberry Pi / Jetson deployment class), checks the MSRV
(`cargo check` on Rust 1.86), and runs `cargo audit` against the RustSec
advisory database.

Docs are part of the change: the README (API Reference, What's New, examples
table) and rustdoc must be updated in the same commit as the code they
describe. Documentation drift is treated as a bug.

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

  for example `Assisted-by: Claude:claude-fable-5`. This is assistance, not
  authorship: an AI agent must never add `Signed-off-by:` (only humans can
  certify the origin of a contribution). Harness-added trailers may coexist,
  but `Assisted-by:` must be present. The human maintainer reviews and takes
  responsibility for every merged line; see the "AI-Assisted Development"
  section of the README.

## When in doubt

- Prefer a loud error over a silent guess — in code and in your own workflow.
- If a change requires weakening the gate, widening the public API, adding a
  dependency, or touching the Non-Goals, stop and ask the maintainer.
- Read the git history of the code you are changing; several invariants above
  are scars from specific bugs, and the commit messages explain them.

## Releasing

Releases are cut by the maintainer. Release prep is ordinary branch work:
it lands on master through the usual branch-and-merge flow before anything
is tagged, and the tag goes on master — `cargo publish` then runs from the
tagged tree. The checklist, in order:

- Finalize `CHANGELOG.md`: replace the version's `Unreleased` marker with the
  release date and repoint its compare link to the tag. For 2.0.0 stable
  specifically, this step is also the consolidation, and it must happen
  here — before the tag and the publish, never after: `CHANGELOG.md` and
  `MIGRATION.md` ship inside the `.crate`, and a published crate is
  immutable. Fold the five published pre-release sections (alpha.1,
  beta.1–beta.4) and the never-published rc.2 section into a single
  `[2.0.0]` section organized by Keep-a-Changelog categories, give it the
  one compare link `v1.4.1...v2.0.0`, resolve the cross-references the
  fold orphans — entries pointing at per-pre-release sections, or at the
  never-published rc.2 — and verify `MIGRATION.md` against the result.
- Confirm the `version` in `Cargo.toml` matches the release, regenerate
  `Cargo.lock` so it records that version (any `cargo build` after the
  bump does), and bump the version pins in the README installation
  snippets — all committed together: `cargo publish` refuses a dirty
  tree.
- Run the full verification gate (`tests/test_all.sh`).
- Run `cargo semver-checks check-release --baseline-rev <previous tag>` and
  confirm the diff is exactly the changelogged one. Against a baseline the
  release already majors over it enumerates nothing — every breaking lint is
  skipped as permitted, and the run proves only that the tooling works and
  that the declared bump covers whatever changed. Measured for 2.0.0 against
  `v1.4.1`: 254 checks, all skipped. To see the diff itself before such a
  release, run it once with the version temporarily set to a patch bump on
  the baseline.
- `cargo publish --dry-run` and inspect the file list — nothing missing,
  nothing that should not ship.
- Merge the release-prep branch to master. If the merge is not a
  fast-forward, re-run the gate on it — the tag must point at a tree the
  gate has seen.
- Tag `vX.Y.Z` on the merge and push the tag.
- `cargo publish`.
- Create a GitHub release for the tag (pre-releases marked as such). For
  2.0.0 stable specifically: mark it as latest.
- After 2.0.0 is published: add a `cargo-semver-checks` CI job so accidental
  breaking changes are caught against the published baseline. This is
  deliberately not added pre-release — everything is breaking against 1.4.1.
