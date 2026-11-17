I now support a v3 built around owned, labelled results, a validated `Rotation`, concrete signed timestamps, operation-specific errors, and internal frame interning. The main remaining disagreements concern scope; the numerical evidence also requires two corrections before it becomes a specification.

**A. Your concessions**

| Point | Verdict | Position |
|---|---|---|
| **A1** | **ACCEPT** | Owned `Transform` and distinct `CrossTimeTransform`; inserts borrow `&Transform`, batches borrow `&[Transform]`. |
| **A2** | **ACCEPT** | Named, fallible transform composition and inversion. Every successful public `Transform` must have finite translation and a valid rotation. |
| **A3** | **ACCEPT** | Keep `BTreeMap` history. The out-of-order regression is substantial, and the proposed lookup benefit did not reproduce. |
| **A4** | **ACCEPT** | Keep `hashbrown`. Qualify the measurement: the reported large-map workload supports this choice; it does not establish that hashing wins for every registry size. |
| **A5** | **ACCEPT** | `target`/`source` describe mappings; `parent`/`child` describe topology. |
| **A6** | **ACCEPT** | An unknown, non-empty name relative to itself yields identity. Reject empty query names before this shortcut. |
| **A7** | **ACCEPT WITH CHANGE** | Specify the computed squared-norm band, `abs(n2 - 1) <= 8 * EPSILON`, and preserve constructor fixed points. Do **not** claim this alone guarantees bitwise identity multiplication: signed zeros are a counterexample. |

For A7, my probe computes `[1.0, -0.0, 0.0, 0.0] * identity` as `[1.0, +0.0, 0.0, 0.0]`, without any normalization. Preserve the exact stored-sample guarantee; a universal bitwise identity law would require explicit shortcuts. The random-population result does not establish that law. [Probe:13](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/round2/src/main.rs:13)

**B. Your retained positions**

**B1 — ACCEPT WITH CHANGE.** Replace `Quaternion` with `Rotation`. My “another type” objection was wrong when the open quaternion disappears. Private components, validating construction, named component accessors, `IDENTITY`, and private interpolation make a smaller and stronger contract.

However, the inverse described in your message is **not the algorithm your measurement evaluates**. The prototype computes:

```rust
let inverse_rotation = conjugate(q);
let inverse_translation = -rotate_raw(inverse_rotation, translation) / norm_squared(q);
```

It does **not** compute `inverse_rotation = conjugate(q) / norm_squared(q)`. The f128 reference also measures the inverse of the *normalized rotation represented by q*. [Numerics implementation:228](evidence/proto-numerics/src/main.rs:228)

I accept the reported **translation-corrected conjugate** formulation as the proposed inverse, retaining the translation finiteness check. I reject substituting quaternion division under the same evidence:

- In my boundary-focused sample, literal `conjugate(q) / n2` escaped the 8-EPS band for **17,426 of 528,047 accepted inputs**.
- One input had computed deviation `−7.5 EPS`; its inverse had `+9 EPS`.
- Recanonicalizing that result would be another algorithm, requiring its own evaluation.

These frequencies describe my deliberately boundary-heavy population, not expected production frequency. [Probe:18](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/round2/src/main.rs:18)

Use the same thresholded normalization rule throughout construction, composition and interpolation. The type invariant must hold at every escape point; sampling alone is not its enforcement mechanism.

**B2 — ACCEPT WITH CHANGE.** I concede concrete `Timestamp(i64 nanoseconds)` and removal of `TimePoint` and the public time generics. The downstream simplification and removal of artificial lookup failure paths now outweigh the demonstrated use of generic clocks.

The contract needs these details:

- The epoch is caller-selected; `Timestamp` does not imply Unix time.
- Ordered differences use `i64::abs_diff`, yielding `u64`. `MAX − MIN` is representable there.
- Interpolation divides integer nanosecond differences directly. I reproduced the ratio probe: no rounding mismatches in its four regimes below `2^53 ns`; roughly 36% mismatches for the current seconds-based calculation.
- `from_secs_nanos` rejects `nanos >= 1_000_000_000`; `as_secs_nanos` uses floor semantics.
- Calculate the combined seconds/nanoseconds value in a wider intermediate. A checked `i64` multiplication followed by addition wrongly rejects the valid representation of `i64::MIN`: `(-9223372037, 145224192)`.
- Keep checked duration arithmetic and exact integer-based `Display`.

Two arguments should be weakened:

1. Nanosecond-quantized **differences** and independently quantized **absolute stamps** are not equivalent. My 32,768-Hz example gives ratios `0.49999180797902842` and `0.50000819202097158`, respectively. The existing `Duration` boundary therefore does not erase every cost of this change.
2. Returning to generics later would still require a breaking release. Reversibility is not a safety net.

I support the seconds/nanoseconds conversion pair, but flag it as a **scope ruling to reopen explicitly**: both are implementable outside the crate. Their justification is the demonstrated conversion errors and the new signed representation, not an assertion that sealed registry state is necessary.

**B3 — ACCEPT WITH CHANGE.** Group errors by operation family, and replace my repeated optional batch indices with:

```rust
pub struct BatchError {
    pub index: usize,
    pub error: InsertError,
}
```

That is strictly clearer. Retain `Clone + PartialEq`, `Display`, `core::error::Error`, and non-exhaustive enums. `BatchError::source()` should expose its `InsertError`.

Further requirements:

- Split empty history from out-of-range history.
- Specify `NoCommonTime` completely:

```rust
NoCommonTime {
    frame: String,
    earliest: Timestamp,
    latest: Timestamp,
    latest_possible: Timestamp,
}
```

Here the named edge starts after the candidate `latest_possible`. Empty history instead reports `NoData`; no optional coverage payload is needed.

- Keep diagnostic context even when it echoes an argument. Errors routinely outlive the call site.
- Define the reparent error surface explicitly. Sharing `InsertError` with reparenting is acceptable as an operation-family compromise; do not describe it as “every variant is possible from every method.”
- Remove insert-time numeric validation **only after every constructor, decoder and derived-result path establishes the stronger invariant**.

Two supporting claims are overstated: a non-exhaustive lookup match already need not mention `CycleDetected`, and retaining the last error does not require `Clone`—an error can be moved. Neither correction undermines the proposed design.

**B4 — ACCEPT WITH CHANGE.** Internal interning and allocation-free topology resolution belong in v3. I withdraw my earlier deferral.

I reran the differential executable and reproduced, for each implementation:

- **372,996** successful comparisons with zero maximum component difference;
- **27,069/27,069** bit-identical exact single-hop results;
- zero named-frame disagreements;
- the documented disconnected-precedence and unknown-self differences.

The remaining requirements are implementation obligations, not objections to adopting the design:

- Reclaim/reuse slots after frame churn; retain a slot when descendants still reference it.
- Failed insertion and failed batches must not change name visibility.
- Keep IDs entirely private; no exported handles or generations are needed merely for internal reuse.
- Avoid unchecked narrowing of slot indices.
- Preserve diagnostic precedence when numerical operations become fallible.

The prototype omits removal, reparenting, batches, latest-common-time and cross-time lookup. Its timings also exclude the restored owned result labels and the proposed numerical changes. Consequently, the reported speedups support the architectural choice, **not a finished-v3 performance claim**. [Prototype limitations:18](evidence/stress-proto-hotpath.md:18)

**B5 — HOLD.** With public `Rotation::rotate`, this remains exactly implementable outside:

```rust
let rotated = tf.rotation().rotate(point);
let translation = tf.translation();
// componentwise addition
```

Removing vector operators makes that more verbose; it does not make it inaccessible or introduce a sealed invariant. The scope critic’s argument for admitting `transform_point` depended on **also removing public rotation application**, which your retained design does not do.

The convention-pinning argument is useful, but the listed hand-math slips are not new evidence of downstream use. Under the standing scope rule, I still leave this outside. A maintainer exception could reasonably admit your single method; it must be identified as an exception. Its raw `Vector3 -> Vector3` signature would also retain ordinary floating-point overflow behavior and provide no frame/time check.

This follows [AGENTS.md:46](AGENTS.md:46), and the previous application discussion explicitly recognized that conflict. [DESIGN.md:224](analysis/v2.4-proposal/DESIGN.md:224)

**B6 — ACCEPT.** `with_max_age` and `unbounded`, without `new` or `Default`. The names expose the retention decision without inventing a default duration. Do not imply that a retention window establishes an adequate interpolation-gap policy.

**B7 — ACCEPT WITH CHANGE.** Prefer `remove_samples_through`.

But the existing “name stays” decision was explicitly a **v3 ruling**, not merely a 2.x compatibility restriction. The record says the maintainer considered a loud rename and chose otherwise. Reopen it on clarity and migration safety; do not describe it as automatically voided by the new mandate. [REPORT.md:625](analysis/v3-review/REPORT.md:625)

**B8 — ACCEPT.** Drop public `approx` implementations; keep the dependency for tests. I withdraw my preference for preserving them. Keep `thiserror` and `hashbrown`.

Flag the runtime dependency-list amendment for the maintainer. No replacement runtime dependency is warranted.

**B9 — ACCEPT WITH CHANGE.** Unconditional `no_std + alloc`, no `std` feature, no clock-reading functions.

The strongest argument is the cleaner responsibility boundary after B2. Removing `now()` removes a misleading convenience; it does **not** prevent wall-clock/simulation-clock mixing. Keep the existing target, MSRV and numerical-parity requirements, testing both serde configurations.

**C. Remaining disagreements**

**C1 — ACCEPT WITH CHANGE.** Defer maximum-gap configuration from 3.0. I withdraw my insistence that `set_max_gap` ship immediately.

I have no defensible universal finite default. Exact-only lookup would protect an unconfigured user from interpolating across dropouts, but it would also disable ordinary interpolation until configuration; that is a substantial policy choice.

An optional limit can still be valuable without protecting every unconfigured user, so I reject that as a necessary admission criterion. Nevertheless, the unsettled configuration model and hole-aware latest-time algorithm justify deferral. Record it as an unresolved priority, **not a promised first 3.x feature**.

**C2 — ACCEPT WITH CHANGE.** List `lookup_latest` as a maintainer scope decision, with my recommendation still **outside under the current rule**.

The private-lock argument does not establish a core limitation. An adapter can hold one guard across the two existing calls. If it hides that capability today, an adapter release is required whether it implements the two-call helper or forwards a new core method.

The historical wrong implementations are a legitimate argument for an exception. If the maintainer broadens scope to admit such helpers, apply the same stated criterion to timestamp conversion and application helpers; do not call some of them “core-or-nowhere.”

**C3 — ACCEPT.** Under B2/B9, both the trait formatting hook and `try_now` disappear. Exact `Timestamp::Display` remains.

**D. Your questions**

**D1 — ACCEPT.** With owned labels restored, I have no remaining architectural objection to B4. Slot lifecycle, transactional interning and the omitted mutation operations need verification before implementation acceptance.

**D2 — ACCEPT WITH CHANGE.** Keep named-field `Vector3`, without arithmetic operators. Named components improve adapter construction, readout and diagnostics; `[f64; 3]` saves a type name but makes component meaning positional. Do not add array conversions merely for symmetry.

**D3 — ACCEPT WITH CHANGE.** Keep default-off serde on the five listed types, with validated decoding where those types have invariants. No serde on `CrossTimeTransform`.

Preserve mandatory, explicitly tagged `Stamp`; missing/null stamps must not become static. Freeze the **new** binary schema and test constructor/decode fixed points.

The signed timestamp change is a real silent wire incompatibility, not merely changed bytes. I verified with postcard:

```text
old u64 1   -> decoded i64 -1
old u64 100 -> decoded i64 50
```

Document that persisted/interchanged data must identify its schema version externally or be migrated with the old decoder. Do not attempt heuristic dual decoding. [Wire probe:30](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/round2/src/main.rs:30)

**D4 — ACCEPT.** I retract these parts of my earlier position:

- retaining the open quaternion alongside stronger transform invariants;
- deferring internal interning after the requested evidence now exists;
- retaining generic time primarily for hypothetical users;
- optional batch-index fields inside ordinary insert errors;
- preserving `new`/`Default`, public `approx` implementations and the old cleanup name;
- insisting on maximum-gap configuration in 3.0.

I retain the objections to unchecked transform arithmetic and unlabelled results—which you have already accepted—and the application-scope objection.

**My earlier requirements that remain outstanding**

1. **Reject empty names at construction and decoding**, as well as query boundaries. Non-empty self transforms remain constructible; insertion rejects self-edges.
2. **Keep cross-time provenance explicit:** six read-only accessors, no implicit conversion into a same-time transform, no ordinary composition or serialization shortcut.
3. **Specify error precedence across both cross-time legs:** topology before coverage before numerical failure; deterministic tie-breaking. A numerical failure encountered early must not mask a later missing required edge.
4. **Batch preflight must include running-newest expiry state and staged topology.** Commit must not contain a remaining logical failure path. Preserve ordered-loop equivalence and the settled expiry boundary/exemption rules.
5. **Keep reparenting atomic**, preserving kind and retention policy, replacing history only after validation. Reject the scope critic’s remove-then-insert substitute: failure-induced topology loss is a meaningful regression.
6. **Keep one private geometry implementation**, bare geometry for static storage, established fold order, and exact single-hop retrieval.
7. **Delete the Point/Transformable/Localized family, public transform interpolation and redundant validation surface.** Do not replace them with public isometries, frame handles, caches, enumeration or retry policy.
8. **Preserve independent golden vectors.** Port their invocation paths; do not regenerate expected values from the rewritten crate.

I read all supplied reports and maps, inspected the relevant prototype code, reran `proto-hotpath/target/release/diff` and `critic-time/target/release/ratio`, and ran the additional [boundary/wire probe](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/round2/src/main.rs). I did not rerun the timing suite or billion-iteration numerical runs, build an integrated revised v3, or measure embedded hardware. Confidence is high in the architectural conclusions and the reproduced counterexamples; finished-v3 performance remains unverified. The repository is unchanged.