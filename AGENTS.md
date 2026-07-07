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

## Architecture in five lines

- `Registry` — public entry point; a `HashMap<String, Buffer>` keyed by **child**
  frame name, plus chain resolution between arbitrary frames.
- `Buffer` — per-child-frame `BTreeMap<T, Transform<T>>` ordered by timestamp,
  with interpolation between stored samples.
- `geometry` — `Transform` (translation + rotation + timestamp + parent/child
  frames), `Vector3`, `Quaternion`, and `Point` as the reference implementation
  of the `Transformable`/`Localized` traits.
- `time` — the `TimePoint` trait and the default `Timestamp` (u128 nanoseconds);
  `std::time::SystemTime` is supported behind the `std` feature.

## Non-negotiables

- `#![forbid(unsafe_code)]`. No exceptions.
- No new dependencies without maintainer approval. Middleware independence is
  the crate's reason to exist; `thiserror`, `approx`, and `hashbrown` are the
  entire runtime dependency list. (The `[dev-dependencies]` — `log`/`env_logger`
  for examples, `tokio` for the async example, `criterion` for benches — are
  expected and do not contradict this.)
- `no_std` parity: every change must build and pass tests with
  `--no-default-features`. Feature-gated asymmetries (such as automatic buffer
  cleanup, `Timestamp::now()`) must be documented on both sides.
- The README **Non-Goals** section is load-bearing. Rigid-body transforms only:
  no scaling, skew, affine, or perspective transforms, no extrapolation, no
  non-linear interpolation, no tf2 API parity. Do not implement these even if an
  issue requests them; redirect to the maintainer.
- Library code must not panic on reachable paths. The only documented panic is
  `Timestamp::now()` on a pre-epoch system clock. Time arithmetic is checked,
  always.

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
  `BufferError::StaticDynamicConflict`. The static sentinel is
  `T::static_timestamp()` (`t = 0` for `Timestamp`).
- A lookup must return a transform whose `parent`/`child` match the requested
  frames exactly; a chain that resolves only partway (unknown frame, timestamp
  gap mid-chain) must return `NotFound`, never a partial result.
- Interpolation happens only between stored samples; a query outside the covered
  time range fails. There is no extrapolation.
- Buffer expiry is data-driven: entries older than
  (latest **inserted** timestamp − `max_age`) are removed on insert. Wall-clock
  time is never consulted.
- Rotations are expected to be unit quaternions; `Quaternion::new` does not
  normalize. Anything that inverts a rotation must normalize first (see
  `Transform::inverse`).

When you fix a correctness bug, ship the regression test that fails on the old
code in the same commit.

## Style

The gate below machine-checks lints, formatting, and docs; everything else in
this section is convention, enforced in review — follow it anyway.

- Edition 2024, `rust-version = "1.85"`. `#![warn(missing_docs)]` and
  `#![warn(clippy::pedantic)]` must stay at **zero warnings** in both feature
  modes. Never add a new `#[allow]` to get green; fix the cause or ask. The
  standing allowances are `clippy::similar_names` in tests (where `t_a_b`-style
  names are domain-correct) and a handful of narrowly-scoped
  `clippy::cast_*` allows on the numeric conversions in `src/time/timestamp/`
  — do not remove them, and do not treat them as precedent.
- Construction: `Vector3::new/zero/unit_*`, `Quaternion::new(w, x, y, z)` /
  `Quaternion::identity()`, `Timestamp::zero()` / `Timestamp::from_nanos()` —
  never struct literals in tests, examples, or docs. `Transform { .. }` and
  `Point { .. }` keep named-field literals (no full constructor by design).
- Float literals carry digits on both sides of the dot: `1.0`, never `1.`.
- Doc comments come first, then attributes (`#[cfg]`, `#[must_use]`,
  `#[inline]`). Constructors get bare `#[must_use]`; pure transforming
  operations get the std phrasing
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
  `examples/`), `assert_eq!`/`assert_ne!` over `assert!(a == b)`,
  behavior-descriptive snake_case names, and the dual-setup pattern with the
  `#[cfg(not(feature = "std"))]` block first. In `no_std` setups,
  `Timestamp::zero()` is fine for a single static sample, but never use it as
  the base of a *dynamic* time series — `t = 0` is the static sentinel, and a
  series starting there mixes kinds. Use `Timestamp::from_nanos(1_000_000_000)`
  or similar as the dynamic base.
- Strings into `String` fields: `"a".into()`. Format strings use inline
  captures: `{x}` / `{x:?}`.

## Definition of done — the verification gate

All of the following must pass before a change is complete
(`tests/test_all.sh` runs the whole gate):

```bash
cargo test
cargo test --no-default-features
cargo clippy --all-targets                          # zero warnings
cargo clippy --all-targets --no-default-features    # zero warnings
rustup run nightly cargo fmt --check                # repo uses nightly rustfmt options
cargo doc --no-deps                                 # zero warnings
cargo run --example std_minimal                     # and the other std examples
cargo run --example no_std_minimal --no-default-features   # and the other no_std examples
cargo bench -- --test
cargo bench --no-default-features -- --test         # CI also builds no_std benches
```

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
