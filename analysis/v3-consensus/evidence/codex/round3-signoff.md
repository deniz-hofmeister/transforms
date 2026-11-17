**SIGN OFF WITH THESE EDITS.** I support the design, including the newly proposed naming and module choices. I do not sign off on the current text: several evidence claims are overstated, and a finite normalization failure must not be called `NonFinite`.

I read the complete [DESIGN.md](analysis/v3-consensus/DESIGN.md), checked it against the archived positions, reports, prototype code and relevant 2.4 implementation, and reran the differential, allocation and boundary/wire probes. No repository files were changed. Line numbers below refer to the current document.

**1. Attribution**

- **M1 and M2 accurately represent my positions.**
- **M3–M10 and M12 accurately represent my recommendations**, subject to the factual corrections below.
- **M11 becomes agreed in this round.** It was not previously agreed, but I now accept it.
- I support the twelve headline “Review of the review” verdicts. I do **not** endorse all their supporting sentences.

These sentences misstate what I can endorse:

> “The review's objection to the conjugate (bit changes, a flaky tolerance pin) does not apply to this form.” — line 393

The corrected inverse still changes bits and still occasionally violates the old absolute tolerance. Its stronger sampled accuracy supports accepting those changes; it does not make them disappear.

> “It never questions the time generic, the mixed error enum, or the `std` feature.” — lines 414–415

Too broad. The review explicitly discusses `TransformError<T>`, changing the nested registry error, and deleting `TimePoint::checked_sub`. It retains generic time and the common registry error rather than undertaking our redesign. [Original review:673](analysis/v3-review/REPORT.md:673)

> “A correct floating-point result has no unique bit pattern.” — line 422

I do not hold that claim. A specified expression and rounding contract can determine a bit pattern. My position is that changing bits is not automatically a correctness regression.

> “no default protects an unconfigured user” / “Every shape is additive later.” — line 450

My round-2 answer explicitly identified exact-only lookup as a protective default, with a substantial usability cost. Changing defaults later is not automatically additive.

For the authorship sentence, replace lines 13–16 with:

> **Designers.** Claude and Codex wrote independent positions and exchanged them over two rounds. In round 3, Codex signed off subject to the corrections recorded in its response; this document incorporates those corrections.

Use the last clause only after incorporating them. I cannot independently authenticate the model/effort annotations.

**2. Rulings on the newly settled details**

| Item | Ruling | Qualification |
|---|---|---|
| **a. Flat root** | **ACCEPT** | Private implementation modules remain appropriate. No compatibility aliases or prelude are needed. |
| **b. Short names and `target()`/`source()`** | **ACCEPT** | Consistent and sufficiently clear on their owning types. |
| **c. Helper deletions; `MAX` without `MIN`** | **ACCEPT** | Keep `Stamp::at` for the demonstrated use. Delete the public norm-tolerance constant unconditionally; document the constructor tolerance. |
| **d. Shared `TransformError`** | **ACCEPT** | This is grouping by operation family, not a promise that every method produces every variant. |
| **e. Empty query → `UnknownFrame`** | **ACCEPT** | Check emptiness before any identity shortcut, including all three cross-time names. |
| **f. Error payloads** | **ACCEPT** | `Expired { frame, stamp, newest }` supplies the settled recovery information. `incoming: Stamp` identifies the conflicting kind; `{ a, b }` correctly describes the disconnected pair. I withdraw requiring `cutoff`. |
| **g. Five cross-time arguments** | **ACCEPT** | The adjacent frame/time pairs and explicit parameter names suffice. No argument-wrapper type is warranted. |
| **h. `Into<String>` constructor** | **ACCEPT** | Moving an existing `String` avoids copying that name. This does not establish allocation-free adapter conversion generally. |
| **i. Composition stamps** | **ACCEPT** | Preserve the 2.4 rule, including `Static ∘ Static → Static`. Specify it explicitly. |
| **j. `NonFinite` for normalization failure** | **CHANGE** | A finite value outside the norm band is not non-finite. Add `LookupError::NonUnitRotation { norm: f64 }`; retain `TransformError::NonUnitRotation` for the corresponding geometry failure. |

Exact addition to `LookupError`:

```rust
NonUnitRotation { norm: f64 },
```

Add this contract:

> `NonFinite` reports a non-finite numerical value. A finite rotation that fails the normalization postcondition reports `NonUnitRotation { norm }`. Construction and geometry operations use the corresponding `TransformError` variants. Neither failure may escape inside a successful transform.

**3. Factual corrections and exact replacement text**

**Surface count and API description**

The displayed API contains **78 named items**, using the inventory’s counting convention: 11 types, 39 functions, two constants, 21 enum variants and five public fields. Adding the lookup error above makes **79**, before optional M1/M2 additions or removal of the M3 pair. “Roughly 60” is incorrect.

Replace lines 43–45 with:

> **Public surface:** 105 named items in 2.4 versus 79 in this proposal, including M3 and the normalization-postcondition lookup variant, but excluding M1 and M2. The count includes types, functions, constants, enum variants and public fields; trait implementations are counted separately.

Delete the repeated “roughly 60” sentence at lines 526–527.

Other precise corrections:

- Line 208: replace **“six public types”** with **“five public value/error types and two traits”**.
- Line 209: replace **“15 variants”** with **“14 variants”**.
- Replace **“Each call's error type lists its own failures”** with **“Errors are grouped by operation family; each method documents the subset it can return.”**
- Replace **“No operators anywhere”** with **“No arithmetic operator implementations.”** Equality and ordering operators remain.
- Complete the impl list: `Clone` accompanies `Copy`; `Timestamp` also needs `Eq` and `PartialOrd`; `Stamp` retains `Eq` without ordering. State whether `Registry` retains `Debug`; my recommendation is yes.

The baseline inventory itself records 14 registry-error variants. [API inventory:64](analysis/v3-consensus/evidence/map-api.md:64)

**Rotation and numerical claims**

Replace the problematic part of the `Rotation` row at line 203 with:

> The invariant lives in one type, and `Transform::new` cannot fail on the rotation norm. The maintained band removes the admitted 1e-6 norm error of the old type; quaternion application still has rounding-scale error. Public quaternion normalization, norm, conjugation and interpolation helpers disappear.

Delete “five fewer public items”; the stated count is not supported. Also avoid “`Rotation::rotate` cannot [scale]”: an approximately unit quaternion used in the sandwich product still has a small norm-related scale error.

Replace the construction/maintenance evidence claims with:

> Without a band, reconstruction changed bits in 38–40% of the two constructed-input populations. With the 8-EPS band, those populations showed zero reconstruction changes in 1.6e8 samples each. This is empirical evidence, supplemented by a first-order bound sketch; the runtime postcondition enforces the invariant.
>
> An unconditional Newton step changed product bits in about 45% of the sampled pairs and broke bitwise right-identity multiplication in 28.8%. In the sampled raw five-product population, 0.0062% fell outside the 8-EPS band. That frequency is not a bound on normalization frequency for arbitrary inputs or long chains.

These qualifications follow the actual populations in the [numerics report:74](analysis/v3-consensus/evidence/stress-proto-numerics.md:74).

Replace the inverse measurement paragraph with:

> In the sampled comparison of two million chains against an f128 reference for the normalized rotation represented by the stored quaternion, the translation-corrected conjugate had relative RMS translation error of 0.66 EPS, versus 0.99 EPS for 2.4’s normalize-then-conjugate and 2.65 EPS for the plain conjugate over raw products. These are population-specific relative errors, not universal bounds. The measured product variants were raw products and thresholded Newton products, not an integrated implementation of this design.

Replace review item 3’s explanation with:

> Replace. Use the translation-corrected conjugate. It avoids square-root normalization during inversion and improves sampled relative RMS translation accuracy. Bit changes remain, and the old absolute-tolerance property still failed once in sixteen million sampled chains. Acceptance requires a justified, magnitude-aware error bound rather than retaining that marginal absolute pin.

The archived output directly records that failure and the bit changes. [Inverse results:18](analysis/v3-consensus/evidence/prototypes/numerics/out-inverse-x.txt:18)

Replace the overflow example at line 252 with:

> Unchecked doubling of an identity-rotation translation initially equal to 1.0 overflows on the 1024th doubling. The proposed fallible composition must return an error instead of producing an invalid transform.

`t = t.compose(&t)` is not the proposed Rust operation as written, and a general labelled transform cannot compose with itself.

Replace lines 253–259 with:

> **Interpolation ratio.** Divide integer nanosecond differences as `f64`. For spans below 2^53 ns, both integer conversions are exact and the division is correctly rounded. The sampled current seconds-based calculation differed from the reference in roughly 36% of cases, by at most two ulps in those samples.
>
> **Exactness kept.** A direct child-to-parent lookup at a stored instant returns the stored geometry bit for bit. Each same-time chain resolution inverts at most once. This is not a one-inversion bound for the entire cross-time operation. Floating-point transcendental operations use `libm`.

The direction restriction is explicit in the differential evidence. Cross-time currently resolves two legs and can invert the target leg again. [Hot-path report:35](analysis/v3-consensus/evidence/stress-proto-hotpath.md:35), [registry:1184](src/core/registry/mod.rs:1184)

Apply the same qualification to line 507.

**Time claims**

Replace lines 281–284 with:

> **What it fixes.** E0283 in all-static generic setups; lossy seconds in error payloads and formatting, where distinct nanosecond stamps can print identically; and an inferred obstacle to typed error handling in generic adapters. Every valid ROS `(i32, u32)` timestamp, with nanoseconds below 1,000,000,000, fits the signed representation.

The generic-error explanation is an inference, not an established cause of downstream authors’ choices.

Replace the 32,768-Hz sentence with:

> Converting non-decimal ticks to integer nanoseconds loses exactness. Nearest rounding at 32,768 Hz incurs at most 0.5 ns per stamp; floor conversion has a different bound. Quantizing absolute stamps is not equivalent to quantizing each elapsed duration. Codex’s floor-conversion example produced interpolation ratios `0.49999180797902842` and `0.50000819202097158`, respectively. The ratio error depends on the interpolation span; there is no general “eighth digit” bound.

I reproduced those values again. [Boundary probe:38](analysis/v3-consensus/evidence/codex/probes/round2-probe.rs:38)

In M3, change **“naive outbound `/ %`”** to **“naive outbound `/ %` followed by casting the remainder to `u32`”**. The cast is what produces `3794967296`.

**Internals, differential results and performance**

Replace the first two internals bullets with:

> **Interned frames.** A vector of slots plus a `hashbrown` name index. Every name referenced by a stored edge, including roots, has a slot. Name lookup is expected O(1) in frame count, excluding string hashing/comparison cost. IDs remain private; slots are reclaimed only when no edge or child references them. Index conversion must not truncate.
>
> **Topology-first resolution.** Index walks identify the connecting chain, and only its edges are sampled. Successful resolution needs no temporary heap allocation; materializing the labelled result allocates its names. Error construction can allocate. A cold diagnostic walk preserves target-side-first, lowest-failing-edge attribution on connected paths.

Replace lines 348–352 with:

> The differential run reproduced 372,996 successful comparisons with maximum component difference 0.0, 27,069/27,069 bit-identical direct stored-direction lookups, and zero named-frame disagreements among comparable errors. It also found 73,889 intended disconnected-precedence differences and 3,344 unknown-self differences: the prototype rejects those queries, while 2.4 and this design return identity. The prototype therefore does not yet implement the agreed identity contract.

I reran all four variants and reproduced those counts.

The timing table pools configurations that the design rejects. Prefer a table explicitly limited to **BTree history plus hash index**, using the report’s paired runs:

| Workload | Measured speed-up over 2.4 |
|---|---:|
| One hop, exact stamp | 1.51–2.19× |
| One hop, interpolated | 1.25–1.51× |
| Four dynamic hops, interpolated, either direction | 0.90–1.13× |
| Robot leaf-to-leaf | 3.31–3.71× |
| Siblings below 16 dynamic ancestors | 24.10–28.46× |
| Static climb, 1,000 deep | 2.25–3.26× |
| Unknown frame, 1,000 frames | 1,027–1,270× |

For the last row, the absolute ranges are approximately **50–95 µs versus 44–92 ns**, not one baseline time paired with a range from several runs. Keep the existing warning that labels and numerical changes are absent. [Timing evidence:75](analysis/v3-consensus/evidence/stress-proto-hotpath.md:75)

One correction belongs in the evidence record too: the past-newest allocation row at report line 61 should read:

```text
| one step past newest | 5 (100 B) | 2 (16 B) |
```

My rerun reports `2.002` allocations per operation before subtracting harness overhead. `sample` allocates an error, then `diagnose` samples again and allocates another. Consequently, the report’s “one small String on error” generalization also needs an erratum. [Prototype:310](analysis/v3-consensus/evidence/prototypes/hotpath/src/proto.rs:310)

**Serde**

Replace lines 380–382 with:

> Freeze the v3 serde schema. The signed timestamp encoding is not safely compatible with 2.x: an old postcard-encoded `u64` value of 1 successfully decodes as `i64` −1. Decoding failure cannot identify the old schema. Persisted or interchanged data must identify its schema externally or be migrated with the old decoder; no heuristic dual decoding.

Replace M10’s title with:

> A new serde schema with a silently incompatible timestamp representation.

“Cannot read” and “not readable” assert the opposite of the demonstrated hazard. I reproduced the postcard cases again.

**Review wording and deferred scope**

Use these replacements:

| Location | Replacement |
|---|---|
| Review item 6 | “Retain exact `lhs` and `rhs` timestamp payloads rather than replacing the current lossy `f64` payload with a unit variant.” |
| Lines 414–415 | “The review retains generic time and a common public registry error; it does not explore removing `TimePoint` or the `std` feature.” |
| Lines 416–418 | “2.4 samples shared ancestors that are later discarded and performs per-hop string-map work. Its unknown-frame failure path also scans parent references. The reported timing measures the whole query, not `frame_exists` alone.” |
| Lines 421–423 | “Changed bits are not by themselves a correctness regression. Preserve explicit bit-exact guarantees and assess other changes against independent references and justified error bounds.” |
| Gap-limit row | “The largest demonstrated interpolation-policy hazard left, and core-or-nowhere. No configuration model is settled. Exact-only lookup would protect an unconfigured user from dropout interpolation, at the cost of disabling ordinary interpolation until configured; no universal finite gap default is justified. Optional limits may be additive later, but changing defaults need not be. Record this as an unresolved priority, not a promised feature.” |

Also separate two misleading grouped exclusions:

> `clear_history` remains unnecessary because inclusive cleanup at `Timestamp::MAX` clears dynamic history. Automatic reset detection and capacity limits lack an agreed requirement and policy; D5 alone does not decide them.

> Frame enumeration and raw coverage introspection are rejected as additional state-exposing commitments, not because they are exactly implementable outside. `can_transform` and waiting compose on the public API. Caches require their own demonstrated need and invalidation design.

**Limits**

Replace lines 513–524 with:

> Treat the reported small timing differences as inconclusive on this noisy machine. No integrated v3 or embedded performance measurement exists.
>
> The 8-EPS band has empirical support and a first-order bound sketch. These probes do not establish an exhaustive normalization-closure proof or directly establish correct rounding of `libm::sqrt`; the runtime postcondition remains required.
>
> Downstream findings come from the repository’s studies of versions spanning 2.1–2.3 and the reviewed unmerged work. No dependent was built against this consensus API. The compiled adapter sketches used a signature-only stub of Claude’s first draft, including its subsequently rejected borrowed-edge design.

The stub limitation is explicit in its own report. [Dependents critique:3](analysis/v3-consensus/evidence/stress-critic-dependents.md:3)

**4. Contracts still needed to avoid implementation guesses**

Add the following text. These clarify the accepted design without adding public functions.

**Numerical algorithm**

> Compute the squared norm in the established component order. Preserve components unchanged inside the 8-EPS band. Outside it, normalize by multiplying each component by `1.0 / libm::sqrt(n2)`, then check finiteness and the band postcondition. Use this private maintenance rule for construction, products and interpolation; the public constructor additionally applies its 1e-6 input-acceptance tolerance.
>
> Inversion retains the conjugate quaternion. Following the measured operation order, compute `r = rotate_raw(conjugate(q), t)`, `s = -1.0 / n2(q)`, and inverse translation components `s * r[i]`. `rotate_raw` is the quaternion sandwich product, without another norm correction. Check the resulting translation.
>
> `Rotation::rotate(Vector3)` is raw numerical application: it provides no frame check and does not promise finite output for arbitrary input vectors or extreme magnitudes.

This explicitly chooses square-root normalization outside the band. An alternative Newton maintenance rule would need to be specified separately; “thresholded normalization” alone does not settle it.

**Composition and lookup**

> Composition requires `self.source() == rhs.target()`. Its result maps `rhs.source()` into `self.target()`. Two dynamic stamps must match; a static operand adopts the other operand’s stamp; two static operands produce `Static`. Inversion swaps the frame labels and preserves the stamp.
>
> Same-time lookup always returns `Stamp::At(requested)`, including static chains. A non-empty same-frame same-time query returns identity even when unregistered. `latest_common_time` returns `Static` for that case and for entirely static connecting chains.
>
> The same-frame shortcut does not bypass cross-time motion: equal endpoint names at different times still require the relevant legs. A leg whose endpoint equals the fixed frame is identity. The caller asserts that the fixed coordinate frame is stationary across the two times; this is not inferred from whether its stored edges are static.
>
> `NoCommonTime.latest_possible` is the minimum newest timestamp of the required dynamic edges. The named edge starts later than that candidate; `earliest` and `latest` describe that edge. Empty required history reports `NoData`.
>
> A numerical failure must not terminate diagnosis before all required coverage has been checked. Implementations may preflight coverage or retain a numerical failure while finishing diagnosis. Cross-time precedence applies across both legs.

**Mutation and time arithmetic**

> `Expired` applies only to bounded, non-empty dynamic histories. A sample exactly on the expiry boundary is accepted. An unrepresentable retention subtraction applies no cutoff. New or drained histories and a replacement reparent seed have no previous newest sample against which to expire.
>
> Duplicate dynamic stamps overwrite; static updates replace the timeless pose. Cleanup preserves topology and kind, but a fully drained history retains no hidden newest watermark. Batch errors report the zero-based input-slice index. Preflight leaves no logical error for commit; this is not an allocation-failure guarantee.
>
> Checked timestamp arithmetic must handle the full signed range and large `Duration` operands without prematurely narrowing them to `i64` nanoseconds.

Finally, **settle the serde layout explicitly**. The displayed `Stamp { At, Static }` declaration reverses 2.4’s enum order, which matters to derived binary encoding independently of signed timestamps. My recommendation is this addition:

> Preserve the existing serde field sequence and names for `Transform`: `translation`, `rotation`, `timestamp`, `parent`, `child`, with parent meaning target and child meaning source. Preserve quaternion component order `w,x,y,z`, vector order `x,y,z`, and `Stamp` variant indices `Static = 0`, `At = 1`. The timestamp scalar becomes `i64`. These choices must be explicit in the serde representation and frozen by tests rather than inherited accidentally from public declaration order.

That preservation is a deliberate reduction in gratuitous wire changes, not a backwards-compatibility claim.

**5. Verification and uncertainty**

This round I ran:

```text
…/scratchpad/proto-hotpath/target/release/diff
…/scratchpad/proto-hotpath/target/release/allocs
cargo run --release --offline --manifest-path …/scratch/round2/Cargo.toml
```

The first two use the supplied prototype binaries; the last reruns my boundary, signed-zero, wire and clock-quantization probe. I checked the timing figures against the archived tables, but did not rerun Criterion or the very large numerical populations. I did not build an integrated v3, run its verification gate, or measure embedded hardware.

My confidence is high in the architectural agreement and the concrete counterexamples. Performance and numerical guarantees for the integrated implementation remain acceptance work. **With the edits above incorporated, you may name me as a co-author who signed off on the design.**