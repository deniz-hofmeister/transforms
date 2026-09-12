# Validated v2 feasibility

Analysis date: 2026-09-12. Baseline: `56482d1` (2.1.2 unreleased).
Implementation branch: `feature/v2-feasibility`, based on released v2.1.2
(`71a263c`) for pull request review. This report does not constitute a release.

Release follow-up: fetched `origin/master` at `71a263c` and the `v2.1.2`
tag after publication. Its library code, manifest, lockfile, tests, examples,
and benchmarks are identical to the analysis baseline. The retained changes
also pass patch-level semver checks against `v2.1.2` in all four feature
combinations (223 checks pass, 31 skip per run). The appropriate next version
for these changes is **2.1.3**. A minor release is permitted for substantial
private improvements but is not required by these changes.
[SemVer specification, clauses 6–7](https://semver.org/spec/v2.0.0.html).
PR preparation updated the implementation branch to that release base and
placed its changelog notes under v2.1.3 (Unreleased). The package version
remains 2.1.2 until release preparation.

**Two implementation changes are validated for v2:** compact dynamic history
and borrowing pinned names during the existing lookup walk. Their supporting
tests and documentation are included. No public API, dependency, feature,
scalar type, or serde representation was added or replaced.

This is a behavioral compatibility assessment as well as an API comparison.
Cargo classifies public renames, signature changes, and tightened bounds as
breaking, while private representation changes can be compatible. It also
notes that runtime compatibility requires judgment beyond compilation. Here,
explicitly documented v2 numerical behavior and error payloads were treated
as contracts. [Cargo's SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html).

## Fully implemented and verified locally

| Change | Implementation and compatibility boundary |
|---|---|
| Compact dynamic samples | `BTreeMap<T, Sample>` stores translation and rotation only. The existing buffer pins supply the frame names; the key supplies the timestamp. Static storage remains one complete transform. Public `Transform<T>` is unchanged. |
| Shared interpolation arithmetic | A crate-private `Transform::interpolate_to` serves public interpolation and buffer reconstruction. It preserves duration operations, ratio calculation, zero-span behavior, quaternion interpolation, and arithmetic order. No public geometry type was introduced. |
| Borrow names during traversal | The current walk follows borrowed pinned parents instead of copying names into scratch strings. Early exits, sampling order, error precedence, shared-tail handling, composition order, and inversion count remain unchanged. |
| Verification and accurate documentation | Nine compatibility tests, a reference-model property test, reproducible differential/measurement clients, and updated README, changelog, rustdoc, and the buffer architecture sentence in AGENTS.md. Obsolete sample-memory figures and unmeasured MCU rate estimates were removed. |

The new reconstruction through `Transform::unvalidated` is justified by the
validated transform entering `Buffer::insert`: its numeric fields are copied
unchanged, its frames are checked against permanent pins, and only dynamic
timestamps become map keys. Validation at insertion remains in place.
The map, expiry implementation, static storage, and removal semantics remain
the existing ones. The numerical operation sequence is shared with public
interpolation rather than duplicated in storage code.

There is no migration required for these changes. They fit a patch release
under the compatibility checks below; following the publication of 2.1.2,
that is 2.1.3. Publishing a version remains the maintainer's release decision.

## Implementations tried and excluded

| Experiment | Observed result | Conclusion for this change |
|---|---|---|
| Shared topology-first resolver | Implemented ancestry resolution shared by ordinary lookup and `latest_common_time`, followed by sampling only the connecting edges. Two of eight compatibility tests failed. | Excluded: this implementation changes v2 error precedence and the named failing frame. |
| Normalize constructed rotations and validate every composition | Implemented both changes in a separate tree. Three existing unit tests and one compatibility test failed. | Excluded: accepted rotation components change, and a documented successful overflowing composition/lookup becomes an error. |
| `VecDeque` history with binary search | Implemented insertion, upsert, neighbor search, expiry, and manual cleanup. Its 278,880-line `Timestamp` trace matched the baseline. Out-of-order insertion into a 60,000-sample history was about 42 times slower than compact `BTreeMap`. | Excluded on measured workload cost. The prototype was not promoted to full gate acceptance. |
| Defaulted scalar parameter with generic constructors | A standalone language witness compiled before generalization and failed with E0283 afterward, on nightly and Rust 1.85.1. | A default `S = f64` does not by itself preserve inference for calls such as `Vector::zero()`. This was a language experiment, not an f32 implementation of the crate. |

The path counterexamples are concrete:

- Two known disconnected trees, both unavailable at the requested instant:
  v2 reports `NotFoundAt` from the target walk; the prototype reports
  `Disconnected`.
- A target leg reaches a common ancestor whose own parent edge is unavailable,
  while the source leg is also unavailable: v2 can report that first failure
  above the common ancestor; the prototype names the source leg instead.

Thus, “private implementation” alone does not establish compatibility. A
different resolver could preserve those diagnostics, but no such complete
replacement was validated here. The retained borrowing change avoids this
semantic change.

The numerical failures also reject the earlier claim that inverse normalization
is redundant. The existing inverse regression deliberately constructs an
accepted near-unit rotation and requires normalization on inversion. Removing
that protection was not implemented.

## Disposition of the other proposed v3 items

These are compatibility findings, not recommendations to implement untested
alternatives. A proposed replacement and an additional opt-in API are different
changes: adding an alternative does not remove the old behavior or enforce the
new invariant throughout v2.

| Proposal | What the evidence establishes |
|---|---|
| Replace `Transform` with geometry, sample, resolved, and cross-time types | Replacing existing argument/return types breaks callers. The compatibility test confirms a cross-time result is currently a `Transform`, carries the target stamp, and can be inserted. An additional typed API would leave that existing route available; it was not implemented or approved here. |
| Guarantee every successful derived transform is numerically valid | Incompatible with the documented v2 contract and failed numerical prototype. A new opt-in type/API was not tested. |
| Replace `Mul` with `try_compose` | Removing or changing the existing trait implementation breaks callers. Adding another method does not replace `Mul`; no such public addition was tested. |
| Accept right-hand identity composition | The executable compatibility witness pins the current `SameFrameMultiplication` error. Changing that documented behavior was excluded under this strict assessment; it is not a signature-only decision. |
| Replace `Point` with `Pose`, split points/vectors/poses, seal and validate `Point` | Renaming the public type or removing mutable public fields breaks existing uses. An alias or additional types would not establish the proposed new invariants on existing `Point` values. Not implemented. |
| Rename result accessors to target/source; replace positional lookup with named arguments | Replacing existing names or signatures is breaking. Additional names/builders would require their own API decision and testing; none were added. |
| Change existing `TransformError` timestamp payloads from `f64` to `T` | Changes public field types and downstream error handling. `#[non_exhaustive]` does not make replacement of existing fields compatible. Not implemented. |
| Replace timestamp operators with checked methods | The typed downstream witness uses the existing `Result`-returning operators. Removing them breaks that code. Additional methods were not implemented; `TimePoint::checked_sub` already exists. |
| Require retention at construction; change `Default` | Changes the constructor or its documented behavior. The compatibility witness verifies default storage still interpolates across a 30-second gap. An additional explicit unbounded constructor would leave the existing default in place. |
| Capacity limits, maximum interpolation gaps, and hole-aware latest-common-time queries | Enabling limits for existing constructors changes accepted data/lookups. Opt-in additions need a fully tested coverage and eviction design; no capacity or gap implementation was validated here. |
| Freshness relative to a caller-supplied time | `latest_common_time` and checked clock arithmetic already support caller-side age decisions. No additional core freshness API was implemented or validated. |
| Reject dynamic ancestry for the fixed frame | The new witness accepts a constant dynamic reference frame. A static-only ancestry rule would reject that existing use. Physical stationarity is not proven merely by reading the edge kind. |
| Reject/cascade removal, explicit root persistence, immutable cached depths | The removal and root-attachment witnesses show descendants survive removal, implicit roots can disappear, and an existing root can later acquire a parent. Depth is not immutable. Preserving those semantics with a different internal graph needs additional implementation and tests. |
| Private frame interning and O(1) frame existence | Not necessary for the validated memory reduction. No complete ID/reference-count design was implemented; this assessment neither recommends it nor declares it incompatible with v2. |
| Public frame handles, borrowed results, and zero-allocation lookups | No such API was implemented. Changing the existing owned return type or adding a borrow from the registry changes how callers can retain results across mutation. The measured improvement here is fewer allocations, not zero. |
| Full f32/f64 parity | No scalar-generic library was implemented. The inference counterexample defeats the proposed default-parameter shortcut as a blanket compatibility argument. Separate opt-in designs remain unverified; the f32 Non-Goal was not changed. |
| Hardware latency/energy bounds and universal host/MCU replay claims | No hardware measurements were made. The existing embedded build matrix was retained and exercised; no timing or replay guarantee was inferred from compilation. |
| Shorten AGENTS.md and relocate its rationale | Not performed. Only its storage description changed to match the implementation; contributor-policy restructuring was not needed for these v2 changes. |

## Measurements

Host: Intel Core i7-1065G7, x86-64. Library: release profile. Standalone client:
`rustc -O`. Heap tools: Valgrind 3.27.1 Massif and Memcheck, with safe Rust
throughout. These fixtures measure requested heap and process allocation
deltas, not physical MCU SRAM use. Massif's estimated allocator overhead is
recorded separately in the JSON; actual allocators may differ.

| Fixture | Starting tree | Retained implementation |
|---|---:|---:|
| 10,000 sequential dynamic samples, 8-character names | 2,556,806 B | 1,220,983 B |
| Same, 128-character names | 4,957,408 B | 1,221,825 B |
| Single-hop ancestor lookup, exact or interpolated | 5 allocations | 3 allocations |
| Single-hop reverse lookup | 6 allocations | 5 allocations |
| Four-hop interpolated ancestor lookup | 11 allocations | 9 allocations |
| 64-hop ancestor lookup | 135 allocations | 133 allocations |

The 10,000-sample requested-heap reduction is approximately 52% for short
names and 75% for long names. The numeric payload plus default timestamp key
is 64 B; it is not the total resident cost. Map occupancy, insertion pattern,
timestamp size, transient allocations, and allocator overhead still matter.

Seven timing repetitions alternated implementation order. For 1,000
out-of-order inserts into a prefilled 60,000-sample history, median cost was
about 0.53 microseconds per insert for compact `BTreeMap`, versus 22.30
microseconds for `VecDeque`. This establishes a substantial regression in
that tested workload, not a universal crossover point or an MCU estimate.
The complete timing ranges are in [host-timings.json](host-timings.json).

## Verification performed

- Baseline and retained implementation: full `tests/test_all.sh` gate.
  This includes four feature combinations, clippy, rustfmt, rustdoc including
  docs.rs settings, six examples, benchmark smoke runs in both std modes,
  and the three ARM bare-metal targets with and without serde.
- Stable 1.98.0 clippy: all targets, all four feature combinations, with
  warnings denied. Rust 1.85.1 checks: all four feature combinations.
- Extra `riscv32imc-unknown-none-elf` builds: with and without serde.
- `cargo-semver-checks` 0.50.0: `--release-type patch` against `56482d1`
  in all four feature combinations, plus all features against released
  `v2.1.1`. Each run reported 223 passing checks, 31 skipped, and no required
  semver update. This does not prove behavior; the tests below cover that.
- Nine new public compatibility witnesses and a 256-case reference-model
  property test pass on both the baseline and retained implementation.
  The model exercises upserts, out-of-order insertion, bounded/unbounded
  expiry, cleanup, refill, interpolation, and exact error ranges.
- Identical 278,880-line differential traces for baseline, compact storage
  alone, the retained implementation, and baseline/retained `no_std` builds.
  Traces include numeric `to_bits()` values, frames, stamps, error variants,
  payloads and formatting, static/dynamic conflicts, removal, cross-time
  queries, and latest-common-time queries. The same deterministic fixture
  also matched the deque prototype using `Timestamp`.
- Existing independent SciPy golden vectors, exact slerp expectations, custom
  clock tests, validation regressions, and frozen serde bytes remain unchanged
  and pass the gate.

The local gate ran with Nix Rust 1.100.0-nightly. It emits the same pre-existing
Cargo manifest advisory about explicit `package.readme` as the baseline;
compiler/clippy/rustdoc checks passed. Native ARM64 execution, physical MCU
timing, cross-device replay, and allocation-free operation were not verified
by this work and are not claimed.

## Reproduction and retained evidence

[verification.json](verification.json) records tool versions, source hashes,
trace hashes, and check summaries. [baseline-measurements.json](baseline-measurements.json)
and [candidate-measurements.json](candidate-measurements.json) retain the heap
and allocation measurements.

Use one selected Rust toolchain consistently on PATH. `run.py` explicitly
passes that compiler to Cargo to avoid mixing a Nix compiler with rustup's
compiler. From the repository root, with a separate baseline checkout:

```sh
git worktree add --detach /tmp/transforms-baseline 56482d1
python3 analysis/v2-feasibility/run.py /tmp/transforms-baseline /tmp/v2-baseline
python3 analysis/v2-feasibility/run.py . /tmp/v2-candidate
cmp /tmp/v2-baseline/trace.txt /tmp/v2-candidate/trace.txt
python3 analysis/v2-feasibility/measure.py /tmp/v2-baseline
python3 analysis/v2-feasibility/measure.py /tmp/v2-candidate
python3 analysis/v2-feasibility/bench.py /tmp/v2-baseline /tmp/v2-candidate
rustup run nightly tests/test_all.sh
cargo semver-checks check-release --baseline-rev 56482d1 --release-type patch --all-features
```

Repeat `run.py` with `--no-default-features` to check the no_std trace.
Copy `tests/v2_compatibility.rs` and `tests/properties.rs` into the baseline
checkout to execute the added tests against the original library.

Rejected experimental patches are retained as evidence, not active code:
[path-prototype.patch](path-prototype.patch) and
[numeric-prototype.patch](numeric-prototype.patch) apply to separate clean
`56482d1` checkouts. Copy the compatibility tests there and run them to
reproduce the failures. [deque-prototype.patch](deque-prototype.patch)
applies to the retained implementation in a separate checkout. Build its
probe and include that output directory in `bench.py` to compare timings.
Compile [scalar-default.rs](scalar-default.rs) normally and then with
`--cfg generic`; only the latter must fail with E0283.
