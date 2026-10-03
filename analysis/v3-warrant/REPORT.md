# Is a v3 warranted? Ergonomics review and verdict

Analysis date: 2026-10-03. Baseline: master `aa29557` (released v2.2.0 plus
the AI-disclosure hooks). This report judges the crate's ergonomics and
intuitiveness, then asks whether a breaking v3 release is warranted now.

The v3 notes in `analysis/v3-proposals/` were one input, not the frame. The
companion design for the recommended next step is
[`analysis/v2.3-proposal/DESIGN.md`](../v2.3-proposal/DESIGN.md).

## Method

- Five lenses wrote probe programs against the public API at `aa29557`:
  first-contact use, API surface and mental model, errors, time and numerics
  and `no_std`, and ecosystem. Each lens's findings went to an independent
  skeptic, which re-checked the evidence and downgraded or refuted overstated
  claims.
- Six clusters of v3 proposals were each evaluated against current master,
  then adversarially reviewed.
- The reverse dependencies were read from crates.io tarballs:
  `transforms_io` 0.2.0/0.5.0/0.6.0, `roslibrust_transforms` 0.1.0/0.2.0/0.26.0
  and `whirl_tf` 0.1.0-alpha.3. Two peers were read the same way:
  `schiebung` 0.5.0 and `cu-transform` 1.2.1.
- **Not done:** nothing was measured on hardware, and the tf2 documentation
  fetch was blocked, so tf2 comparisons rely on
  `analysis/reparent-frame-value/TF2_RESEARCH.md`. The peer comparisons were
  not re-verified.

## Verdict

**v3 is not warranted now.**

- The design is sound, and most friction is shallow. Nearly everything worth
  fixing can ship in 2.x as additive API or documentation.
- The items that genuinely require a break are small cleanups. None has a
  headline benefit for the users who exist, and none meets AGENTS.md's
  demonstrated-need rule.
- 2.0.0 shipped on 2026-08-22. Five 2.x releases followed in five weeks.
  Cutting a third major in about seven months would signal churn to a small
  ecosystem whose adopters already lag: `whirl_tf` is still pinned to a 2.0
  beta.

Keep a v3 ledger (below) and cut v3 when something forces a break. Batch
the whole ledger then.

## Ergonomics

### What works

- **One small mental model, applied consistently.** Edges are keyed by child,
  and `Transform(parent, child)` maps child coordinates into the parent.
  `Transform::new(parent, child, …)` and `get_transform(target, source, t)`
  both put the receiving frame first. A newcomer probe built a static mount, a
  dynamic odometry stream, an interpolated multi-hop lookup and a point
  transform in about three compile iterations. The result was correct on the
  first run.
- **The invariants live in the types, not just the prose.**
  - `Stamp` is unordered, and no timestamp value is reserved. That removes
    tf2's zero-means-latest trap. `transforms_io` 0.6.0 still carries a
    `Timestamp::now()` workaround for that trap.
  - Constructors validate, and transform fields are private.
  - Construction-only types are `#[non_exhaustive]`.
  - The tree is strict, and a frame is static xor dynamic.
- **The error design is unusually good.**
  - `RegistryError` is flat, and its payloads stay in the caller's time type.
  - `covered: None` versus `Some` separates "this frame holds nothing" from
    "this frame has a gap".
  - Topology is diagnosed before geometry, so a numeric failure cannot mask a
    missing frame.
  - Compiler diagnostics lead a newcomer to the right import, trait and
    constructor.
- **Custom clocks are cheap.** A `TimePoint` for an MCU tick took about 15
  lines and three methods. The `no_std` builds work on the real targets.

### Where it gets in the way

Ranked by consequence. Item 1 is the reviewer's own judgment; the lens
skeptics rated its parts low individually.

1. **The checked path does not cover the work people actually do.**
   - Frame and time checks exist only on `Transformable`, and `Point` is its
     only implementation. `Point` is a pose that carries a `String` frame and a
     quaternion.
   - Nothing applies a transform with checks to a bare `Vector3`, a batch, or
     a point cloud. Users write `rotation().rotate_vector(v) + translation()`
     by hand and skip every check. The `get_transform_at` doc example does
     exactly that.
   - The cross-time case is inverted (probe-verified). Applying a
     `get_transform_at` result to the correct point, in the source frame at
     the source time, fails with `TimestampMismatch`. Applying it to a point
     at the target time returns `Ok` with a wrong answer. The checked path
     rejects the right input and accepts the wrong one. None of the v3 notes
     records this.
   - The hazard reaches a real dependent. `roslibrust_transforms` 0.26.0
     re-exports `get_transform_at` (`src/lib.rs:555`). Its rustdoc tells
     callers that the result "converts points from `source_frame` at
     `source_time`". That is the use the checked path rejects.
2. **Operators return `Result`.**
   - `Timestamp ± Duration`, `Timestamp - Timestamp` and
     `Transform * Transform` all return `Result`.
   - Composition cannot chain (`a * b * c`), and the examples carry
     `(time - d).unwrap()`.
   - `checked_sub` exists only through the `TimePoint` trait (E0599 unless it
     is imported), and there is no `checked_add`.
   - Idiomatic Rust uses named `checked_*` methods for fallible arithmetic.
     For a crate that ranks "Rust-first" as its first priority, this is the
     least idiomatic part of the API.
3. **Argument direction with string frames.**
   - Swapped `Transform::new` arguments on a fresh tree, or a swapped
     `get_transform`, succeed silently with the inverse. This is inherent to
     string frames. `get_transform_for` avoids it for lookups.
   - The examples show the confusion. `examples/std_minimal.rs:33` and
     `examples/no_std_full.rs:37` say "transform from camera to base" above
     `Transform::new("base", "camera", …)`, and the variables are named
     `camera_to_base_*`.
   - `parent()`/`child()` on a sibling lookup result names a tree relation
     that does not exist. The accessor docs explain this, so it is learnable.
4. **Missing geometry vocabulary.**
   - `Quaternion` offers only `from_wxyz` and `identity`. It has no
     axis-angle or roll/pitch/yaw constructor, and `Vector3` has no norm,
     dot or cross product.
   - Both ROS-adapter dependents hand-write roughly 30 lines of nalgebra
     conversion each. That count was reported by the ecosystem lens, not
     re-counted.
   - AGENTS.md's scope rule places anything that can be built exactly on
     the public API outside the core, and these all can. So this is a
     companion-crate or documentation matter, not a core gap.
5. **Smaller items.**
   - **E0283 inference error.** `Registry::new()` and static-only setups hit
     E0283. The turbofish workaround is documented on the type page only,
     not on `new`.
   - **`Stamp` versus `T`.** `latest_common_time` returns a `Stamp`, but
     `get_transform` takes a `T`. On an all-static chain the caller must
     choose an instant, and the doc example's `.at().unwrap()` would panic if
     copied there.
   - **`latest_common_time` is hard to find.** It is absent from the
     crate-root docs and the minimal examples.
   - **Wrapping clocks.** `TimePoint` does not warn that a wrapping hardware
     counter violates `Ord`, which would silently misorder samples.
   - **Frame-less errors.** `add_transform` errors are unit variants, so
     their messages cannot name the frame.
   - **`max_age` is not a gap limit.** The unbounded default is documented.
     But note for v3: a probe with `with_max_age(10s)` still interpolated
     across an 8 s gap. A required `max_age` would bound memory, not the
     silent long-gap interpolation that v3-1 implies it fixes.

### What the dependents show

- **They are thin adapters.** All three are ROS adapters: convert messages,
  insert, look up, and turn errors into strings. None matches a
  `RegistryError` variant, and none has comments complaining about the API.
  Their friction is in ROS conversion, which no v3 item touches.
- **The 1.x → 2.0 migration was cheap and paid off.** It cost small diffs,
  the two active dependents absorbed it within weeks, and it fixed real
  silent-wrong-answer bugs in them.
- **The main 1.x → 2.0 trap compiled but changed behaviour:** `max_age`
  semantics. Any v3 change of that kind deserves the most caution.

## The v3 question

### What genuinely requires a break

| Item | Why it breaks | Value |
|---|---|---|
| Remove `get_transform_at`, or make its result non-insertable | Removal, and a pinned declared behaviour (`tests/v2_compatibility.rs`) | Medium. The hazard is real, but an additive result type covers callers who migrate |
| `Rotation` newtype + translation bound; retire insert re-validation and the `From` canonicalization | Signatures, serde, a pinned contract (stored rotations become normalized) | The cleanest end state. Small user payoff: drift is ~4e-17 per composition, and `Ok(inf)` needs ~1e308 inputs |
| `Disconnected` before `NotFoundAt` | Declared precedence (`error.rs:162`, MIGRATION) | Low to medium. It is a misleading diagnosis, not a wrong answer, and `latest_common_time` already tells the cases apart |
| Operators → `checked_*` / `compose` | Removal of operators | Low to medium. An idiom fix |
| One vocabulary (`target`/`source` on results, `Pose`, `ReparentingNotSupported`) | Renames | Low. Cosmetic unless result types split |
| `TransformError<T>`, sealed `Point`, required `max_age` | Signatures and defaults | Low |
| A registry generic over the scalar | Only if f32 shares the `Registry` type | None today. A concrete `f32` module would be additive |

### Pros of cutting v3

- **It will never be cheaper.** There are three dependents and about 385
  downloads a month, and the last migration was cheap for them.
- **Types close hazards for every caller.** Deprecation helps only callers
  who migrate. The cross-time hole stays open for everyone else.
- **The idiom warts go away.** The `Result`-returning operators otherwise
  stay forever.
- **The invariants get simpler.** Once derived values cannot escape validity,
  the defensive re-validation and the canonicalization can go.
- **One precedence rule and one vocabulary.**

### Cons

- **No demonstrated need.** No user has asked for any item, and the
  dependents do not use the parts that would change.
- **Timing.** A third major in about seven months costs trust, and trust is a
  feature for a crate that positions robots.
- **Most of the value is reachable additively.** v3 would mainly buy the
  deletion of deprecated paths.
- **Silent behaviour changes.** Several items compile unchanged but behave
  differently: normalized storage, the precedence flip, a bounded default.
  That is the class that nearly bit 1.x → 2.0.
- **The decision that could force a structural break is unmade.** Whether to
  support f32 waits on M4F/M33 numbers nobody has measured. The CI ARM
  builds compile f64 as software calls (`__aeabi_dmul`, probe-verified).
  Cutting v3 before measuring risks needing a v4.

## Recommendation

1. **Ship 2.3 with additive changes only.**
   [`DESIGN.md`](../v2.3-proposal/DESIGN.md) specifies three parts:
   - a cross-time result type with its own checked application, and
     `get_transform_at` deprecated;
   - checked application of a transform to plain positions, which needs a
     maintainer scope decision;
   - documentation fixes.
2. **Measure.** Time lookups on M4F, M7 and M33 with `target-cpu` set and
   the FPU variant recorded, following v3-3 §6. Compare differential-trace
   hashes across the x86-64 and ARM64 runners.
3. **Keep the v3 ledger** (the table above), and cut v3 only when a trigger
   appears:
   - hardware numbers make a scalar-generic registry necessary, or
   - a user hits a ledger hazard, or
   - deprecated surface accumulates enough to justify removal.

   Then batch the ledger, with the `Rotation` invariant as the centrepiece. If
   "invalid states unrepresentable" is valued for its own sake, prototype
   that invariant inside the crate-private `Isometry` first, and measure the
   Newton-step cost before committing to a break.

## Corrections to the chat discussion that preceded this report

- **Euler and axis-angle constructors.** The discussion claimed AGENTS.md's
  scope rule puts them in the core. It does not. The rule places anything
  buildable exactly on the public API outside the core, and the
  wrong-answer-generator clause applies only when the correct version needs
  sealed internals. They are withdrawn from the 2.3 proposal.
- **`checked_add`.** An inherent `checked_add` beside the `Result`-returning
  operators would be a second spelling of one operation, not an idiom fix.
  It moves to the v3 ledger (operators → named methods).
