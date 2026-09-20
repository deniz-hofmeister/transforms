# transforms v3 — updated critique

Reviewed released **v2.1.3**, [commit 808169b](https://github.com/deniz-hofmeister/transforms/commit/808169b2dfe350e9c60d49ad85142c88269e0e27), including implementation, regression tests, documentation, and retained feasibility evidence. Tests and benchmarks were not rerun: this environment has no Rust toolchain. This is a design proposal, not a final specification.

**Recommendation:** focus v3 on temporal correctness, geometry guarantees, and embedded precision choice. v2.1.3 substantially reduces the case for a storage redesign.

## What v2.1.3 already resolves

Dynamic history now stores geometry under timestamp keys, with frame names pinned per buffer. Traversal borrows those names. Remove duplicated sample metadata from the outstanding recommendations.

The repository's [host measurements](https://github.com/deniz-hofmeister/transforms/blob/808169b2dfe350e9c60d49ad85142c88269e0e27/analysis/v2-feasibility/REPORT.md) report:

| Fixture | Before | v2.1.3 implementation |
| --- | ---: | ---: |
| 10,000 samples, 8-character names: requested heap | 2,556,806 B | 1,220,983 B |
| 10,000 samples, 128-character names: requested heap | 4,957,408 B | 1,221,825 B |
| One-hop ancestor lookup: allocations | 5 | 3 |
| 64-hop ancestor lookup: allocations | 135 | 133 |

These are reported host results, not independently reproduced measurements or MCU budgets. The 64-byte geometry-plus-timestamp payload is not total sample memory.

The README now distinguishes compilation coverage from hardware performance and cross-device reproducibility. Removal and cross-time restrictions are clearer. Withdraw the earlier documentation criticism where those corrections apply.

## 1. Preserve cross-time provenance

`get_transform_at` still returns an ordinary `Transform` carrying only the target timestamp. A regression test confirms that this result remains insertable as a single-time sample; documentation warns against it but cannot enforce the restriction.

Introduce a dedicated cross-time result retaining both frames and timestamps. Its application should check source metadata and produce target metadata. Prevent implicit insertion or ordinary single-time composition.

Separate pure geometry from frame/time metadata. The earlier four-type proposal was more prescriptive than necessary: add distinct sample and resolved wrappers only where they enforce an additional invariant. Physical stationarity of the fixed frame remains a caller responsibility.

## 2. Make successful geometry operations preserve validity

The existing tests still demonstrate accepted rotations composing outside the validation tolerance, and ancestor-directed lookup returning infinite translation while reverse lookup rejects it.

Use a normalized rotation invariant and consistent rejection of non-finite derived geometry. Normalize accepted near-unit constructor input; maintain precision-appropriate normalization through composition and interpolation.

Simply validating every result would reject accumulated drift without fixing its cause. Preserve the protection provided by inverse normalization. This changes v2's numerical contract and belongs in a deliberate v3 migration.

## 3. Offer full `f32`/`f64` functionality

The crate remains `f64`-only. For the owner's embedded goals, support homogeneous `f32` and `f64` registries, retaining `f64` as default. Both should support the complete transform API under `std` and `no_std + alloc`, with precision-appropriate math and tolerances. Conversions should be explicit.

The feasibility report exposes an API concern: a default scalar parameter does not automatically preserve inference for calls such as `Vector3::zero()`. Prototype constructor ergonomics before choosing the public generic design; do not describe it as automatically compatible.

Parity means functionality and invariants, not identical results between precisions. Preserve timestamp precision before scalar conversion. Hardware performance remains unmeasured; heap-free operation would require separate storage work.

## 4. Improve lookup without assuming frame interning

The main remaining allocation cost is materializing owned transforms at each hop. First pursue internal geometry accumulation and allocate frame names only for the final owned result. Strictly allocation-free lookup requires a separate ownership and scratch-storage decision.

Retain `BTreeMap`: the reported deque experiment substantially regressed large out-of-order insertion workloads. Frame IDs, persistent root objects, public handles, and borrowed results need independent justification; compact history no longer depends on them.

A shared topology-first resolver remains attractive, but the prototype changed error precedence and the reported failing frame. Withdraw the earlier suggestion that it is straightforwardly compatible with v2. For v3, define topology errors first and report temporal failures only on connecting edges. Compatibility-test failures establish a contract change, not that this design is wrong.

## 5. Keep further scope selective

- Consider sample/frame capacity and maximum interpolation gaps for concrete embedded or outage requirements. Gap limits require hole-aware latest-common-time queries.
- Leave freshness decisions to callers using `latest_common_time` and checked clock arithmetic; another core policy is not yet justified.
- Retain descendant-preserving removal, now clearly documented. A clearer edge-removal name is reasonable; automatic subtree deletion is not an established need.
- Keep identity composition, `Point` → `Pose`, source/target vocabulary, and typed `TransformError` timestamps as secondary API improvements. `RegistryError<T>` already preserves lookup timestamps.
- Retain existing numerical, property, compatibility, and serialization tests. Add focused tests for changed contracts; avoid duplicating the new storage reference-model coverage.

Keep rigid-transform scope, middleware independence, private internals, and caller-managed concurrency. Contributor-document restructuring is optional maintenance, not a v3 prerequisite.
