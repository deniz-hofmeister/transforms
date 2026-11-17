# Stress test of the draft's section 3 (numerics)

Subject: `claude-position.md` §3, against `transforms` 2.4.0 (master `adccbdd`).
Nothing in the repository working tree was touched (`git status --short` is empty after the runs).

## Verdict in one paragraph

The Newton step does what §3 says for the norm, but §3 is wrong or incomplete in five places:

1. **The cited bound is slightly optimistic.** Measured max `| |q|-1 |` is 2.9e-16, not 2.2e-16.
2. **Construction-time normalization as written is not a fixed point.** `from_wxyz(r.w, r.x, r.y, r.z) != r` for 38-50% of values, so a serde roundtrip is not bit-exact. A dead band is required, and §3 does not mention one.
3. **The unconditional Newton step breaks the bit-level identity law.** `a * identity != a` in 28.8% of cases. Banding the step fixes that but forfeits the accuracy gain that justifies the conjugate inverse. The draft has to choose; §3 presents neither side.
4. **"Valid by construction is true of every value" is false for translations** under either overflow option, because the public `Transform * Transform` is total and can return `inf`.
5. **Renormalizing per product is not free in accuracy.** It trades linear norm drift for a (smaller) linear direction drift on constant-increment chains.

The conjugate inverse without a Newton step is a real accuracy loss (2.7x rms), not only a test artefact. With the unconditional Newton step it is more accurate than 2.4.

## Method and caveats

- Scratch crate: `evidence/proto-numerics` (`src/main.rs`, `runall.sh`, raw outputs `out-*.txt`).
- It uses `libm` 0.2.16 and path-depends on the crate. Quaternion and isometry arithmetic is copied expression for expression from `src/geometry/quaternion/mod.rs` and `src/geometry/transform/isometry.rs`.
- The replica was checked against the real `Registry`: 20000 of 20000 chains bit-equal, forward and backward lookups. The registry composes `hop * composed` from the leaf upward (`compose_chain`, `src/core/registry/mod.rs:1347`).
- Slerp numbers call the crate's own `Quaternion::slerp`.
- Norms are measured exactly: `|q|^2 - 1` via error-free products (FMA) and two-sum, so the measurement adds no rounding of its own.
- Build: `RUSTFLAGS="-C target-cpu=native" cargo build --release` (rustc 1.101.0-nightly), binary `target/release/proto-numerics <mode> <N>`. Rust never contracts to FMA, so results are bit-identical to a default build (bench checksums match across both builds).
- Machine: i7-12850HX laptop, shared with other jobs (load average 3-13 during the runs), clock reported at 0.8-1.5 GHz. **Timings are indicative only**; ratios are more trustworthy than absolutes. No embedded or soft-float measurement was made.
- RNG: xoshiro256++, fixed seeds, 16 threads. "Constructed" means the crate's `normalize` (`q * (1/sqrt(n2))`) applied to a random 4-vector.
- The property-test numbers come from a replica of the generators (uniform ranges, chain length uniform in 2..=5), not from proptest's own distributions.
- "EPS" is `f64::EPSILON` (2.22e-16). "n2" is the float-computed squared norm `w*w + x*x + y*y + z*z`.

## (a) Norm drift and cost

Commands: `proto-numerics drift 1e8`, `proto-numerics drift9 1e9`. Max over the whole run of `| |q|-1 |`, worst of the increments tested (1e-3 rad about z; 1 rad about (1,2,3); 0.1 rad about (-3,1,2); a random unit quaternion).

| Variant after each product | const-increment, 1e8 | const-increment, 1e9 | random walk, 4 x 1e8 | single product, 1e8 |
|---|---|---|---|---|
| none (2.4) | 5.5e-9 | 5.5e-8 | 1.4e-12 | 6.5e-16 |
| Newton `q*(3-n2)/2` | 2.8e-16 | 2.85e-16 | 2.9e-16 | 2.9e-16 |
| Newton, skipped when `abs(n2-1) <= 8 EPS` | 1.04e-15 | 1.04e-15 | 1.04e-15 | 6.5e-16 |
| full sqrt normalization | 3.3e-16 | 3.4e-16 | 3.4e-16 | 3.4e-16 |
| sqrt, skipped inside the 8-EPS band | 1.04e-15 | n/a | 1.04e-15 | 6.5e-16 |

- Without renormalization the constant-increment drift is linear, about 4e-17 to 5.5e-17 per product, with a fixed sign per increment.
- The draft's "2.2e-16 over 1e9" holds for one of the increments (1.8e-16) but not in general: the measured ceiling is 2.9e-16 (1.3 EPS). The claim's substance stands.
- Newton is marginally tighter than sqrt normalization here.

**Direction error is the cost nobody listed.** `proto-numerics dir 3e6`: angle between the f64 chain and an f128 reference chain after 3e6 constant-increment products.

| Increment | none | Newton | Newton + band 8 | sqrt |
|---|---|---|---|---|
| 1e-3 rad about z | 8.6e-14 | 1.9e-13 | 9.0e-14 | 1.4e-13 |
| 1 rad about (1,2,3) | 1.0e-12 | 5.3e-12 | 2.4e-12 | 3.4e-12 |
| random unit | 1.8e-13 | 1.4e-11 | 3.6e-13 | 8.9e-12 |

- Rescaling by a factor like `1 - EPS/2` rounds each component by a different relative amount, and with a constant increment the factor has a constant sign, so the bias accumulates: up to about 5e-18 rad per product here, 75x the un-renormalized chain in the worst row.
- It is physically negligible (about 5e-10 rad after 1e8 products, extrapolated, not measured) and irrelevant to registry lookups, whose chains are short. It does mean no variant is "exact"; only three increments and one N were measured.

**Cost.** `taskset -c 4 target-default/release/proto-numerics bench` (default x86-64 build, variants interleaved, min of 40 rounds).

| Workload | none | Newton | Newton + band 8 | sqrt |
|---|---|---|---|---|
| Rotation product, independent pairs | 7.2 ns | 10.3 ns (+3.0) | 9.8 ns | 10.8 ns (+3.6) |
| Isometry product, independent pairs | 18.5 ns | 21.2 ns (+2.7, x1.15) | 20.8 ns | 24.1 ns (+5.7) |
| Isometry chain in registry order (latency-bound) | 21.7 ns | 32.7 ns (+11.0, x1.50) | 26.1 ns | 45.8 ns (+24) |
| Rotation chain `q = q*d` (latency-bound) | 26.3 ns | 42.1 ns (+15.8, x1.60) | 36.4 ns | 56.4 ns (+30) |

- The draft's "13 flops on 87" is arithmetically right (7 for n2, 2 for the factor, 4 to scale; 28 + 59 for the isometry product) and matches the throughput row (+15%).
- It understates the cost inside a lookup. There the step sits on the dependency chain of the accumulated rotation, and the measured overhead is +50% per hop of pure arithmetic. Relative to a whole lookup (hashing, tree search, slerp) this was not measured.
- In builds with `+fma` or `target-cpu=native`, `libm::sqrt` took about 280 ns per call on this host (default build: a few ns). Cause not investigated; the sqrt column above is from the default build.
- An alternative the draft does not consider: keep internal chain products raw and apply one step where a rotation becomes observable (lookup exit, public `*`, slerp result). Raw drift over a chain of depth n is about n EPS, and one step from a deviation up to 1e-9 lands on the floor (table in (f)). Not prototyped inside the registry.

## (b) Fixed point of reconstruction

Command: `proto-numerics fixed 1.6e8` (1.6e8 samples per population). Fraction of values `r` for which the candidate constructor rule changes at least one bit of `r`.

| Population of values | always normalize | band 1 EPS | band 2 EPS | band 3 EPS | band 4 EPS | band 8 EPS | max float `abs(n2-1)` |
|---|---|---|---|---|---|---|---|
| Constructed (sqrt-normalized random vector) | 38.27% | 7.80% | 0.093% | 5.1e-5% | 0 | 0 | 4 EPS |
| Constructed from 2.4-acceptable input (norm 1 ± 1e-6) | 39.55% | 9.24% | 0.109% | 2.1e-5% | 0 | 0 | 3.5 EPS |
| Newton-stepped product | 46.15% | 1.27% | 9.3e-4% | 0 | 0 | 0 | 3 EPS |
| Newton-stepped chain of 5 products | 47.92% | 1.25% | 1.1e-3% | 0 | 0 | 0 | 3 EPS |
| Newton-stepped crate slerp (nearby or random endpoints) | 44.4-47.0% | 1.2% | 7e-4% | 0 | 0 | 0 | 3 EPS |
| Raw product, no renormalization (2.4 two-hop) | 51.21% | 24.98% | 3.94% | 0.357% | 9.6e-3% | 0 | 6.5 EPS |
| Raw chain of 5 products | 69.41% | 52.19% | 25.53% | 11.45% | 3.76% | 6.2e-3% | 12 EPS |
| Raw crate slerp | 36.01% | 14.86% | 1.12% | 0.061% | 9.2e-4% | 0 | 6 EPS |

- **"Always normalize" is not idempotent.** `normalize(normalize(x)) != normalize(x)` for 39.6% of inputs within 1e-6 of unit, and for 22.9% of inputs already within 4e-15. Using division instead of multiplying by the reciprocal does not help (18.9-32%). So §3's "constructors normalize" plus the API comment "stored normalized" breaks bit-exact serde roundtrip for roughly 40-50% of values.
- **The rule that works:** skip normalization when `abs(n2 - 1) <= K * EPS`, otherwise normalize.
  - For the values a draft-style `Rotation` can hold (constructed, Newton-stepped products, Newton-stepped slerp), zero changes were seen at K >= 4 in 1.6e8 samples per population. K = 4 has no margin (constructed values reach exactly 4 EPS, frequency 6e-8). K = 8 has a 2x margin.
  - Idempotence of the banded rule on raw inputs (`R(R(x)) == R(x)`): zero failures at K >= 4 over 3 x 1.6e8 inputs; K = 3 fails 1e-7 to 2e-7 of the time.
  - My first-order rounding bound is 7 EPS for a sqrt-normalized output and 6 EPS for a Newton-stepped one. That is a sketch, not a proof, but it says 8 is the smallest round K with an argument behind it. The review's 8-EPS figure is confirmed.
- If raw (un-stepped) products were values of the type, K = 8 would not be enough (6.2e-3% of raw 5-chains fall outside). The band and the renormalization rule must be chosen together.
- With the band on the constructor and the same band on the Newton step, the fixed point holds by definition: every value either passed the band test or came out of a step that lands within 3 EPS. Only "the step lands inside the band" is empirical.
- Not tested here: the review's remark that `serde_json` with default features does not parse floats exactly (`0.9999999999999999` read as `1.0`). If true, bit-exact JSON roundtrip fails upstream of any rule; pin it with postcard.

## (c) Conjugate inverse

Command: `proto-numerics inverse 1e8` (1e8 chains; f128 comparison on 2e6 chains), plus `target-x/release/proto-numerics inverse 1.6e7` for the last three rows.

The test `registry_chain_roundtrip_composes_to_identity` (`tests/properties.rs:302`) builds 2-5 links with translations up to 1e5 per axis, and asserts each component of `(forward * backward).translation()` within **1e-9 absolute** (`EPSILON`, line 16). At magnitudes of 2.6e5 to 8.7e5 that is 8.6 to 17 ulps.

| Variant (product rule, inverse rule) | Test fails | Mean residual | Max residual | Backward translation vs exact inverse: rms / max, relative |
|---|---|---|---|---|
| 2.4: raw, normalize then conjugate | 2.6e-5% (26 in 1e8) | 8.7e-11 m | 1.28e-9 m | 0.99 / 3.6 EPS |
| raw, conjugate | 0.0469% | 1.41e-10 m | 2.27e-9 m | 2.65 / 12.3 EPS |
| Newton, conjugate (the draft) | 0 in 1e8 | 3.5e-11 m | 4.7e-10 m | 0.74 / 2.9 EPS |
| Newton + band 8, conjugate | 0.0389% | 1.41e-10 m | 1.98e-9 m | 2.64 / 9.6 EPS |
| Newton, normalize then conjugate | 0 in 1e8 | 3.1e-11 m | 4.1e-10 m | 1.13 / 3.9 EPS |
| sqrt products, conjugate | 0 in 1e8 | 4.9e-11 m | 6.4e-10 m | 0.99 / 3.8 EPS |
| Newton + band 8, normalize then conjugate | 6e-6% (1 in 1.6e7) | 8.6e-11 m | 1.05e-9 m | 0.99 / 3.6 EPS |
| raw, conjugate with translation divided by n2 | 6e-6% (1 in 1.6e7) | 7.5e-11 m | 1.05e-9 m | 0.66 / 2.95 EPS |
| Newton + band 8, conjugate with division by n2 | 6e-6% (1 in 1.6e7) | 7.5e-11 m | 1.05e-9 m | 0.66 / 2.95 EPS |

**The review's 0.044% is reproduced (0.047%) and explained.** At proptest's default 256 cases that is an 11% flake rate per run (review: 10.6%).

- The forward rotation is a raw product whose squared norm is `1 + d`, with `d` up to 12 EPS over 5 links.
- `rotate_vector` by a non-unit quaternion scales by `1 + d`. In the test's own `forward * backward`, 2.4 leaves a residual of about `d * |t|`, because the normalizing inverse removes `d` from the backward half only.
- The plain conjugate keeps `d` in the backward half too, so the residual becomes about `2d * |t|`. Measured ratio of means: 1.63.
- `d * |t|` with `d` around 5 EPS and `|t|` around 8e5 is 9e-10. The tolerance sits on the tail of 2.4's own distribution (2.4 fails 1 in 3.8e6; the review saw 1 in 3e6).

**So it is both.** The tolerance is marginal on master already, and the conjugate without renormalization is a real loss: the backward translation is 2.7x worse in rms and 3.4x worse in max against the exact inverse. The review's "same order as the forward error" is true in order of magnitude but hides that factor, and the loss grows with chain depth as the raw drift does.

**Does anything get worse than 2.4 under the draft (Newton plus conjugate)?**

- Inverse accuracy: no. It is better than 2.4 on every measure in the table.
- `t * t.inverse()` for a single constructed transform, translations up to 10 per axis, 1e8 samples: conjugate max 1.6e-14 and mean 2.4e-15, against 1.24e-14 and 2.0e-15 for normalize-then-conjugate. That is 20-30% worse at the last-ulp level. Rotation residual is better (1.6e-16 against 3.8e-16 rad).
- Bit stability against 2.4, same chains: forward translations change in 0% of lookups (they are rotated only by stored hop rotations). Forward rotations change in 84%, backward translations in 77%, backward rotations in 78%.
- The banded Newton step keeps 2.4's forward bits (0.06% change) but then the conjugate is as inaccurate as with raw products. The band and the conjugate do not combine.
- Two ways to have the band and an accurate total inverse: keep the sqrt in `inverse` (it cannot fail under the invariant; 2.4 accuracy, 0.05% of bits change), or divide the inverted translation by n2 (one division, no sqrt; the most accurate row in the table).

## (d) The crate's slerp

Command: `proto-numerics slerp 1e8`, calling `transforms::geometry::Quaternion::slerp`.

| Input | Behaviour |
|---|---|
| Identical inputs | Unit result, but **not bit-identical**: `slerp(a, a, t) != a` in 41.8% of cases. 19.4% of unit `a` have a float self-dot of `1 - EPS` or less and take the trig branch. |
| `q` and `-q` (same rotation) | `other` is negated, result is `q`. At `t = 1` the result is `-other`, not `other`: `slerp(a, b, 1) != b` bitwise in 26.3% of cases, 1.3% being neither `b` nor `-b`. |
| `t = 0` | `slerp(a, b, 0) != a` bitwise in 2.9% of cases; a `-0.0` component comes back as `+0.0`. |
| Rotations 180 degrees apart (dot near 0) | No failure, but discontinuous: dot `+1e-17` gives `(0.707, 0.707, 0, 0)`, dot `-1e-17` gives `(0.707, -0.707, 0, 0)`. Inherent to shortest-path interpolation; silent. |
| Dot above 1 from rounding | Clamped silently by `dot.clamp(-1.0, 1.0)`; happens for 2.2% of random pairs, 24% of self-dots. Harmless: only `1 - EPS/2` and `1` reach the lerp branch, which renormalizes. |
| Unit inputs, any angle from 1e-9 to pi | No NaN in 1e8. Max deviation of the norm 6.4e-16 raw; 2.8e-16 after one Newton step. |
| Inputs at 2.4's tolerance edge (norm 1 ± 1e-6) | Output norm off by up to 1.0e-6. Two identical inputs of norm `1 - 1e-6` produce norm `1 - 5e-7`: the short norm is read as an angle of 2e-3 rad. |
| Zero quaternion | Returned as-is (norm 0) or blended to norm 0.707; no error. |
| Components of 1e200 | Dot overflows to `inf`, clamps to 1, result is all zeros. Silent. |
| `inf` component | Result `[NaN, 0, 0, 0]`. |
| NaN `t` or NaN component | NaN result (documented for `t`). |
| `t` outside [0, 1] | Clamped silently (documented). |

- The trig branch never renormalizes, and the lerp branch returns the blend unnormalized when its norm is below EPS. Neither produces a non-unit result from unit inputs; every non-unit or NaN output above needs a non-unit or non-finite input.
- Under the draft (private slerp, unit operands, ratio in [0, 1], one Newton step) the silent cases are unreachable and the result is unit to 2.8e-16.
- The endpoint and identical-input bit differences matter only if a lookup at a stored instant reaches slerp. In 2.4 it does not (the buffer returns the stored sample when the bracketing range is zero); v3 must keep that short-circuit. Not re-verified beyond reading `src/core/buffer/mod.rs`.

## (e) Translation overflow

Command: `proto-numerics overflow` (2e7 random chains with translations of 1e300 to 1.8e308, mixed identity, axis-aligned and random rotations, some inverted).

**Exit finiteness check at lookup: sound.**

- A non-finite intermediate never returned to an all-finite result: 0 of 1,852,493 chains that overflowed midway.
- Argument: every output component of `q * (0, v) * conj(q)` depends on every component of `v` through a product, and `inf` or NaN times anything is `inf` or NaN; addition keeps it. The rotation never depends on the translation.
- All three components must be checked: 387,788 of those chains ended with some components still finite.
- It is not complete. A chain whose true result is representable can still be refused, and the verdict depends on association order: hops of 1e308, 1e308 and -1.5e308 on x (true 5e307) give `inf` folded from the left and `5e307` folded from the right. The registry's order is fixed, so the answer is deterministic.
- A single `rotate_vector` overflows below `f64::MAX`: `(m, -m, m)` with `m = 1.7e308` rotated 2 rad about (1,2,3) gives `(inf, inf, -inf)`.
- Error surface: `LookupError::NonFinite` stays, on every lookup path.

**Magnitude bound at construction: sound for lookups only, and only with conditions.**

- It needs the bound enforced again at insert (the draft already validates at insert), a bound small enough that depth times bound times the rotation's intermediate growth stays below `f64::MAX`, and a statement of the depth it assumes. With depth limited by memory, a bound near 1e280 would do. Not prototyped.
- Error surface: `LookupError::NonFinite` goes away; `GeometryError` gains an out-of-range case and a magic constant becomes public contract; inputs that 2.4 accepts (anything finite) are refused.
- It does not cover the public `*`. Squaring `t = t * t` from a translation of 1 reaches `inf` after 1024 products. With a total `Mul`, a `Transform` can hold `inf` under either option.

**Consequence for §3.** "Valid by construction is true of every value, not only built ones" holds for `Rotation`. It does not hold for `Transform`, whichever option is picked, unless `Transform * Transform` is fallible or the claim is narrowed to rotations. Insert-time validation remains necessary for that reason, and the draft's own table says insert rejects non-finite. I agree with the draft's lean to the exit check: it is exact where the bound is a guess.

## (f) Newton basin

Command: `proto-numerics basin`. The step maps the norm `n` to `n(3 - n^2)/2`.

| Starting norm | One step | Iterated |
|---|---|---|
| 0 < n < 1 | always closer to 1 | converges (56 steps from 1e-9; 1e-300 underflows to zero) |
| 1 < n < 1.5616 | closer to 1 | converges |
| 1.5616 < n < sqrt(3) | **further from 1** | still converges |
| n = sqrt(3) | collapses to **zero** | stuck at zero |
| sqrt(3) < n < sqrt(5) | sign flips (`-q`, same rotation); norm may be closer or further | converges, slowly near sqrt(5) |
| n >= sqrt(5) = 2.236 | further | **diverges** to non-finite (26 steps from the nearest double to sqrt(5), 6 from 3.0) |

Residual after one step, as a function of the input deviation `e` (2e5 random directions each):

| Input deviation | After one step |
|---|---|
| 1e-6 | 1.5e-12 (theory `1.5 e^2`) |
| 1e-7 | 1.5e-14 |
| 3e-8 | 1.6e-15 |
| 1e-8 | 4.0e-16 |
| 1e-9 and below | 2.8e-16 (rounding floor) |

- One step is a full correction only for inputs within about 1e-8 of unit. It is not a substitute for sqrt normalization at construction, where 2.4-style inputs are up to 1e-6 off; the draft does use sqrt there.
- **Can a value leave the basin through the public API? No, on the draft's surface.** Constructors produce at most 4 EPS; a product of two such values is within about 7 EPS before the step and 3 EPS after; slerp of unit operands is within 6 EPS before the step. Reaching 1.56 would need a deviation 15 orders of magnitude larger. NaN and `inf` cannot be constructed. This depends on the fields staying private and on there being no unchecked public constructor.
- **Can the step make a value worse at the ulp level? Yes, routinely** (1.6e8 products of constructed operands):
  - 45.4% of two-hop products change bits (the review's 45% is confirmed).
  - 5.6% end with an exact squared norm further from 1 than before the step.
  - 26.1% end further from unit than the worse operand.
  - The step is not idempotent: a second step changes 6.8% of results.
  - **`a * identity != a` bitwise in 28.8% of cases.** A registry path that multiplies by `Isometry::identity()` (the fixed-frame lookup does, `src/core/registry/mod.rs:1185`) would return different bits from the plain lookup.
  - With the step skipped inside the 8-EPS band, `a * identity == a` and `a * b` is unchanged in 100% of 1.6e8 cases; the cost is the (c) accuracy.

## What the draft should change

1. State the dead band in the `Rotation` constructor contract: values with `abs(n2 - 1) <= 8 EPS` are stored as given. Without it the serde roundtrip claim fails.
2. Decide the Newton step's band explicitly. The three coherent combinations measured:
   - Unconditional step plus conjugate: best inverse accuracy; identity law fails bitwise in 29% of products; 84% of multi-hop forward rotations differ from 2.4.
   - Banded step plus an inverse that still normalizes or divides by n2: 2.4 accuracy or better, identity law exact, bits nearly unchanged from 2.4, inverse still total.
   - Banded step plus plain conjugate: do not ship; 2.7x worse inverse translations and a flaky property test.
3. Replace "2.2e-16" with "about 3e-16 (1.3 EPS)".
4. Narrow "valid by construction is true of every value" to rotations, or make `Transform * Transform` fallible.
5. Whatever is chosen, re-derive the tolerance of `registry_chain_roundtrip_composes_to_identity` from magnitude (a relative bound); 1e-9 absolute at 1e5-scale translations is marginal on master today.

## Not verified

- No embedded, soft-float or instruction-count measurement; host timings on a throttled, shared laptop.
- No measurement of the step's cost relative to a whole registry lookup.
- Direction drift measured at one N (3e6) and three increments only.
- The 8-EPS closure is empirical (1.6e8 samples per population) plus a first-order bound sketch.
- The overflow soundness argument was checked by 2e7 random chains, not proven for every operation order; interpolation of stored finite samples was reasoned, not sampled.
- The magnitude-bound option was analysed, not prototyped.
- `serde_json` float parsing and the stored-instant short-circuit in the buffer were taken from the review and from reading the source.
