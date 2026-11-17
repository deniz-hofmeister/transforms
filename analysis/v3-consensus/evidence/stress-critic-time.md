# Stress review: concrete `Timestamp(i64 ns)` vs generic `T: TimePoint`

Scope: one decision of the draft (`claude-position.md` lines 72-85, 141). Repo at
master `adccbdd` (v2.4.0), read-only. Scratch code:
`evidence/critic-time/probe`
(bins `ratio`, `sizes`, `conv`, `mix`).

Machine caveat for every number below: x86_64, i7-12850HX, Linux 7.0,
`rustc 1.101.0-nightly (b6eeb93c9 2026-10-18)`, `cargo build --release --offline`.
Sizes and alignments are x86_64 values. Nothing was run on an ARM target.

Legend: **[V]** verified here by reading code or running a probe; **[D]** derived
by reasoning from verified facts; **[M]** from memory or from the map files, not
re-checked; **[U]** unverified.

## Verdict

Go concrete, `i64` nanoseconds. The generic parameter buys three things
(clocks beyond ±292 years or finer than 1 ns; payloads in the caller's type;
type-level clock separation) and no studied user uses the first or the third,
while the second is what forces `RegistryError<T>` and E0283. No middle design
beats the concrete end. The draft's API needs four corrections (section 6).

Strongest argument against, stated in full in section 7: the decision removes a
capability on the evidence of four thin adapters, and the capability it removes
(exact tick-domain time for non-nanosecond embedded clocks) is the one the
crate's `no_std` positioning advertises.

---

## 1. What disappears

### Code [V] (counts from `wc -l` and `grep` on `adccbdd`)

| Item | Lines | Notes |
|---|---|---|
| `src/time/traits.rs` | 130 | whole file (trait + 60-line adapter doc example) |
| `src/time/error.rs` | 16 | whole file (`TimeError`) |
| `src/time/system_time.rs` | 34 | whole file |
| `src/time/timestamp/mod.rs` | ~182 of 314 | `now`/`try_now` (~60), `as_seconds`/`as_seconds_lossy` (~60), three `Result`-returning operator impls (43), `impl TimePoint` (19). Estimate from line ranges, not a diff |
| `where T: TimePoint` plumbing | 64 | 33 bound lines + 31 `where` lines across 9 non-test files (`grep -rn "T: TimePoint" src`); 96 non-test lines mention `<T>` |
| `Buffer::check_interpolation_time` + call site | ~20 | `buffer/mod.rs:395-410`, `registry/mod.rs:642` |
| `GetError::Interpolation` + its arm | ~5 | `buffer/error.rs:54`, `registry/mod.rs:1050` |
| `Isometry::interpolate` returning `Result` | — | becomes infallible (see 3.2) |

Gross removal is roughly 450 library lines before the replacement `Timestamp`
API (my estimate for that with docs: 120-150 lines). I did not build the
replacement, so the net figure is an estimate.

### Tests [V]

- `tests/time_traits.rs`: 659 lines, 18 `#[test]`. 17 use a custom clock or
  `SystemTime`.
  - 10 exist only because `T` is open and vanish with it: the five
    `latest_common_time_*` span tests (`WideNanos`, `Picos`),
    `a_clock_that_cannot_report_seconds_cannot_mask_the_error`, two `SystemTime`
    tests, two custom-clock static tests.
  - 7 test clock-independent behaviour through `TestTime` (static at extremes,
    kind conflict in both orders, range extremes are ordinary, mixed chain,
    eviction spares static, time travel, connecting-chain diagnosis). **[U]**
    whether each has a `Timestamp` twin in `src/core/registry/tests.rs`; they
    must be ported or confirmed duplicate, not deleted blind.
- `src/time/timestamp/tests.rs`: 130 lines, 12 tests, mostly rewritten.
- 12 further references to `TimeError`/`TimestampError` in `src` and `tests`
  outside those files.

### Public error variants gone

`TimeError::{DurationUnderflow, DurationOverflow, AccuracyLoss}` (the whole
enum) and `TransformError::TimestampError`. With them the nested spelling
`RegistryError::TransformError(TransformError::TimestampError(..))`.

### Failure modes gone

1. A lookup or `latest_common_time` failing on time arithmetic. With the
   default `Timestamp` this is already unreachable **[D]**: for ordered `u64`
   values `checked_sub` succeeds and `Duration::from_nanos(u64)` is total. It is
   reachable only with a custom clock (REPORT.md:296-298: 3,160 of 144,000
   probe queries). So the machinery in the table above serves custom clocks
   exclusively.
2. `Timestamp::now()` panic — the crate's only documented panic — and the
   `wasm32-unknown-unknown` trap (dependents hazard 13 [M]).
3. `as_seconds` returning `Err` for every stamp above 2^53 ns (104 days).
4. NaN in diagnostics for pre-epoch `SystemTime` (hazard 12 [M]).
5. Inexact diagnostics. `as_seconds_lossy` at wall-clock magnitude (1.8e18 ns)
   has 256 ns spacing **[D]**, so a request 1 ns past coverage prints equal to
   the covered end. The `mix` probe shows the format: `at 5 not found (base
   covers [1795000000, 1795000000])` [V].
6. E0283 on `Registry::new()` / `static_between` in all-static code.

### Docs

42 lines in README/MIGRATION/CHANGELOG/AGENTS mention `TimePoint`,
`SystemTime`, `as_seconds`, `TimeError` or `try_now` [V]. AGENTS.md's
architecture line for `time`, the feature-gate list, and the "error formatting
goes through `as_seconds_lossy`" invariant all need rewriting; the last one is
deleted, not reworded.

### User-visible capability lost

1. Clocks outside ±292.277 years of their epoch, or finer than 1 ns
   (the `Picos` and `WideNanos` test clocks are the only instances found).
2. Exact tick-domain time for clocks whose tick is not a whole number of
   nanoseconds (section 3.3, embassy at 32,768 Hz).
3. Payloads (`Stamp`, error fields, `latest_common_time`) in the caller's own
   type; callers convert back.
4. Type-level separation of clocks, for users who newtype each clock (section 4).
5. `Registry<SystemTime>` and `Timestamp::now()` as conveniences.

---

## 2. Who uses a non-default `T`

Source: `map-dependents.md` [M — I did not re-read the dependents' code].

**dimos** — `Registry<SystemTime>`. Its conversions are "signed ROS ns ↔
`SystemTime`, checked, pre-epoch allowed (`tf.rs:50-81`)". So its wire time is
already a signed nanosecond count and `SystemTime` is an intermediate. Under
the draft its boundary is `Timestamp::from_nanos(ns)`: total, no failure case,
and ~30 lines of its checked conversion go away. The draft's stated cost
("dimos converts at the boundary instead") is overstated: dimos converts today
and would convert less. Static results stamped `UNIX_EPOCH` on the wire become
`Timestamp::from_nanos(0)`.
If dimos does hold real `SystemTime` values somewhere [U], the conversion is the
`conv` probe [V]:

| Input | Result |
|---|---|
| `UNIX_EPOCH` | `Some(0)`, round-trips |
| `UNIX_EPOCH − 1 ns` | `Some(-1)`, round-trips |
| epoch + 292 y | `Some(9214629984000000000)`, round-trips |
| epoch ± 293 y, + 584 y | `None` |
| now | round-trips exactly on Linux |

Failure cases: outside 1677-09-21..2262-04-11 only. My own first version of
the pre-epoch branch mishandles exactly `i64::MIN` (negating a magnitude that
does not fit) — mild evidence that the pre-epoch half is worth shipping once
rather than leaving to each user. On Windows `SystemTime` has 100 ns
resolution [M]; `SystemTime → ns` stays exact, `ns → SystemTime` truncates [U].

**roslibrust_transforms** — generic `Registry<T>` with its own `RosTimestamp`
trait implemented for `Timestamp` and `SystemTime`, plus a tests-only
`MockTimestamp: TimePoint` (the only third-party impl found). Under the draft:
its type parameter and trait collapse to one conversion; `MockTimestamp` tests
become `Timestamp::from_nanos`. Its current conversion "clamps to zero, drops
nsec (wrong)". The inferred reason it stringifies errors — `RegistryError<T>`
would force a generic `TransformManagerError<T>` — disappears. If its public
type exposes `T` [U], that is a breaking release for it.

**transforms_io** — `Registry<Timestamp>`, conversion
`sec.max(0) as u64 * 1e9 + nanosec` (wrong: clamps negatives). Uses
`Timestamp::now()` for "latest" (hazard 6).

Boundary conversions and their failure cases under `i64` ns:

| Source | Conversion | Fails when |
|---|---|---|
| ROS 2 `(i32 sec, u32 nanosec)` | `from_secs_nanos(sec.into(), nanosec)` | `nanosec >= 1e9` only; the full i32 range fits [V `conv`] |
| ROS 1 `(u32, u32)` | same | `nsec >= 1e9` only; fits [V] |
| `rcl` time point / tf2 `TimePoint` (int64 ns) [M] | `from_nanos` | never |
| `SystemTime` | `TryFrom` | outside 1677..2262 |
| embassy `Instant` (u64 ticks) | `i64::try_from(i.as_micros()).ok()?.checked_mul(1000)` | > 292 y uptime; inexact if tick is not a whole ns |
| `std::time::Instant` | via an anchor `Instant`, `duration_since(anchor).as_nanos()` | before the anchor; no epoch exists |
| hifitime `Epoch` | ns since a caller-chosen reference in a caller-chosen scale | out of range; method names [U] |

A point the draft does not make: the orphan rule already forces a newtype for
every foreign time type except `SystemTime` (embassy `Instant`, hifitime
`Epoch`, `std::time::Instant`). So today's alternative to a 1-3 line conversion
is a ~25-line `impl TimePoint` on a wrapper (the crate's own doc example,
`traits.rs:36-88`), not zero lines.

---

## 3. Candidate representations

### 3.1 Table

Sizes from the `sizes` probe [V, x86_64]. A stored sample is `(key, 7 × f64)`.

| | Range | Pre-epoch | Sample | `Stamp` | Ordered-pair difference | Ecosystem fit |
|---|---|---|---|---|---|---|
| `u64` ns | 0..584.55 y | no | 64 B | 16 B | never fails | embassy/fugit u64 ticks at 1 ns; ROS only via a fallible, twice-botched cast |
| **`i64` ns** | ±292.28 y | yes | 64 B | 16 B | `abs_diff` → `u64`, never fails [D] | identical to `rcl`/tf2 int64 ns [M]; superset of both ROS wire stamps [V] |
| `i128` ns | unbounded | yes | 80 B (align 16) | 32 B | never fails | none; 128-bit division is a libcall on Cortex-M0+ [M] |
| `(i64 s, u32 ns)` | ±2.9e11 y | yes | 72 B | 24 B | borrow logic | ROS wire shape, `timespec`; needs a normalized-nanos invariant, so construction is fallible and the ratio is not one division |
| `Duration` since epoch | 0..5.8e11 y | no | 72 B | 16 B (niche) | `checked_sub` | std type, but conflates instant with span: `max_age` and a stamp become swappable with no error |

`RegistryError<Timestamp>` is 104 B, `RegistryError<SystemTime>` 120 B;
`Transform<_>` is 120 B for both [V].

`i64` is the only candidate that is 8 bytes, signed, and needs no invariant.
`i128` on 32-bit ARM probably aligns to 8, giving 72 B [U].

### 3.2 Interpolation ratio

Today (`isometry.rs:82-88`) [V]:
`ratio = diff.as_secs_f64() / range.as_secs_f64()` where each `Duration` came
from `duration_since`. `Duration::as_secs_f64` is `secs as f64 + nanos as f64 /
1e9`, so the ratio passes through three roundings (two inexact `/1e9`, one
division).

Probe `ratio`: 2,000,000 xorshift-random `(d, r)` pairs per regime, `0 < d < r`,
compared against a correctly rounded reference (128-bit long division with a
sticky bit, round-half-even; self-checked on 1/2, 1/3, 1/10).

| Span `r` between neighbours | today ≠ exact | today max error | `d as f64 / r as f64` ≠ exact | max error |
|---|---|---|---|---|
| 1 ms..100 ms | 36.38 % | 2 ulp | 0 | 0 |
| 100 ms..10 s | 37.55 % | 2 ulp | 0 | 0 |
| 10 s..1 day | 36.54 % | 2 ulp | 0 | 0 |
| 1 day..2^53 ns | 35.94 % | 2 ulp | 0 | 0 |
| 2^53..2^63 ns | 35.86 % | 2 ulp | 35.45 % | 2 ulp |
| 2^63..2^64 ns | 35.60 % | 2 ulp | 35.20 % | 2 ulp |

Findings:

- The ratio today is not correctly rounded in about 36 % of cases, at every
  realistic span. Error is at most 2 ulp in the sample.
- The integer form `d as f64 / r as f64` on `u64` differences is **exactly
  rounded whenever `r < 2^53` ns (104.25 days between adjacent samples)**:
  both conversions are exact and IEEE division is correctly rounded [D,
  confirmed by 8M samples with zero mismatches].
- Above 2^53 ns it can be made exactly rounded for every span with the
  reference routine (about 20 lines, `u128` division, no `unsafe`, no
  dependency). I would not ship it: it serves two samples more than 104 days
  apart, and costs a 128-bit division on soft-float targets. Document the
  bound instead. This is a judgement, not a measurement.
- Neither form returned a ratio ≥ 1.0 for `d < r` in the random sample; both
  return exactly 1.0 for `d = r − 1` with `r` near 2^63 or 2^64 (adversarial
  rows of the probe). Harmless: the result equals the later sample.
- Ratio is 0 only for `d = 0` [D].
- Consequence the draft does not state: switching forms changes the last bits
  of ~36 % of interpolated lookups. Any test that pins an interpolated lookup
  bit for bit will move [U which tests]; the slerp pin (fixed ratio) will not.
- This improvement does not need the concrete type. A generic design could
  get it with a `ratio`-style trait method. It is an argument for integer
  arithmetic, not by itself for removing `T`.

With `i64`, the span of an ordered pair is `later.abs_diff(earlier)`, a `u64`
that cannot overflow even for `MAX − MIN` [D]. Interpolation becomes total,
which is what deletes `check_interpolation_time` and `GetError::Interpolation`.

### 3.3 Fit and overflow notes

- **ROS**: i64 ns is the internal representation of `rcl` and tf2 [M]. Both
  wire stamps fit entirely [V]. Negative seconds stop being a cast hazard.
- **embassy-time**: `Instant` is u64 ticks since boot [M]. At 1 MHz the
  conversion is exact. At 32,768 Hz (a supported tick rate, common on nRF [M])
  a tick is 30,517.578125 ns, so the conversion rounds by up to 0.5 ns per
  stamp. Order is preserved; interpolation ratios move by at most ~1 ns over
  the sample span. A generic `T` over ticks is exact here. This is the one
  concrete exactness loss; it is 5 nm at 10 m/s.
- **fugit**: same as embassy; its wrapping instants fit neither design.
- **hifitime**: needs a chosen scale and reference either way.
- **`std::time::Instant`**: has no epoch; needs an anchor either way, and a
  newtype under the generic design.
- **Eviction** (`latest − max_age`): with `u64` it underflows routinely for a
  boot-relative clock younger than `max_age` and is skipped by design
  (`buffer/mod.rs:473-475`). With `i64` it underflows only near `MIN`; same
  skip. No behaviour change.
- **`checked_add(Duration)`** can overflow (a `Duration` holds up to 5.8e11
  years); `Option` is right.

---

## 4. The safety argument against: clock separation

Claim under test: a generic `T` keeps sim time and wall time apart; an integer
does not.

What 2.4 enforces [V]:

- Across different `T`: yes. Inserting a `Transform<Timestamp>` into
  `Registry<SystemTime>` and looking it up with a `Timestamp` gives two E0308
  errors (probe built and discarded).
- Within one `T`: nothing. Probe `mix`, `Registry<Timestamp>`:
  - sim-stamped sample (5 s) then wall-stamped sample (1.795e18 ns): both
    `Ok(())`; an unbounded lookup midway returns `Ok(x = 2.0)`, a blend of two
    clocks.
  - with `max_age = 10 s`, wall then sim: the sim insert returns `Ok(())` and
    is evicted on arrival.

What users do [M]:

- Three of four adapters use `Timestamp`; none defines one type per clock.
- dimos's `SystemTime` holds converted ROS stamps, which are simulation time
  whenever the ROS graph runs on sim time [U whether dimos does]. The type
  says "wall clock"; the value need not be.
- The crate itself ships the mixing primitive: `Timestamp::now()` is wall time
  in the type every adapter uses for message stamps. Hazard 6 ("latest
  answered at wall-clock `now()`", silent on post-dated hops) is this exact
  mix, in production in transforms_io for seven releases.

So the protection is real only for a user who writes a newtype per clock, and
no studied user does. The draft removes `now()`, which closes the one mixing
path the crate actively offered; on this axis it is a net gain for existing
users.

Cheap ways to keep it:

1. A phantom-clock wrapper outside the crate: `struct Clocked<C>(Registry,
   PhantomData<C>)` with a `Stamped<C>(Timestamp)`. About 20 lines, exact on
   the public API. Under AGENTS.md's scope rule that places it outside the
   core. I did not write it [U], but nothing in it needs internals.
2. In the core, `Timestamp<C = ()>` with a marker. Rejected: defaulted type
   parameters do not drive inference, so E0283 returns, and every error and
   `Stamp` becomes generic again.

Neither detects what actually went wrong in the field: two *sources* on
different clocks feeding one registry through one adapter type. That is a
data problem (clock reset / jump detection, hazards 2 and 16), not a type
problem.

---

## 5. Middle designs

| Design | What it keeps | What it costs | Verdict |
|---|---|---|---|
| A. Concrete registry + `From`/`TryFrom` impls on `Timestamp` | everything in section 1 | users convert payloads back | this is the draft |
| B. `lookup(.., at: impl TryInto<Timestamp>)` | call-site convenience | a conversion failure needs a lookup-error variant; hides a fallible step inside a query; results still come back as `Timestamp` | worse than A |
| C. Sealed/tiny trait `T: Copy + Ord` ↔ `i64` ns, registry stores `i64`, `PhantomData<T>` at the surface | clock separation, payloads in `T` | `Registry<T>`, `Stamp<T>`, errors generic again; E0283 back; the back-conversion can fail, so errors need a fallible path or keep raw ns anyway | buildable outside as wrapper (4.1), so out by the scope rule |
| D. Keep `TimePoint`, shrink it | open clocks | the core needs order, a ratio, `t − max_age`, and an exact display: still 3-4 methods. `as_seconds_lossy` could become `Display`, `duration_since` could become `ratio`, but nothing can be removed. `check_interpolation_time` and the `TimeError` plumbing stay because a custom method can fail | no smaller than today |
| E. Concrete `i64`, tick-agnostic (unit is "ticks", not ns) | exactness for 32,768 Hz clocks | `max_age: Duration` no longer has a meaning; Display cannot print seconds | breaks more than it fixes |

No middle design beats both ends. C is the only one with a real benefit and it
composes exactly on A's public API.

Reversibility [D]: going from A back to a generic later is `Registry` →
`Registry<T = Timestamp>`. Type positions keep compiling; expression sites
(`Registry::unbounded()`) hit E0283 again. So it is a break, but a narrow one.
A is the cheaper end to be wrong at.

---

## 6. Recommended `Timestamp` API

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
pub struct Timestamp(i64);

impl Timestamp {
    pub const MAX: Self;
    pub const fn from_nanos(nanos: i64) -> Self;
    pub const fn as_nanos(self) -> i64;
    pub const fn from_secs_nanos(secs: i64, subsec_nanos: u32) -> Option<Self>;
    pub const fn as_secs_nanos(self) -> (i64, u32);
    pub const fn checked_add(self, d: Duration) -> Option<Self>;
    pub const fn checked_sub(self, d: Duration) -> Option<Self>;
    pub const fn checked_duration_since(self, earlier: Self) -> Option<Duration>;
}
impl fmt::Display for Timestamp { /* exact */ }
#[cfg(feature = "std")] impl TryFrom<SystemTime> for Timestamp { type Error = TryFromIntError; }
```

| Item | Caller that justifies it |
|---|---|
| `from_nanos` / `as_nanos` | every adapter (transforms_io, roslibrust, dimos's ns wire time); embassy/fugit conversions |
| `from_secs_nanos -> Option` | two of four adapters clamp or alias here; the prior ruling's reopen condition was "a third clamping integration", and the signed type changes the recipe anyway. `None` for `subsec_nanos >= 1e9` or overflow. Negative seconds with a positive nanos field is the ROS convention |
| `as_secs_nanos` | every adapter that publishes back to ROS. **Missing from the draft.** The split must floor (`div_euclid`/`rem_euclid`): −1 ns is `(−1, 999_999_999)`, and truncating division gives `(0, −1)`, which does not fit a `u32` and invites a cast. Introducing signed time without the inverse recreates the hazard on the way out |
| `checked_sub(Duration)` | "lookup 100 ms ago" queries; the crate's own eviction. Name matches `SystemTime::checked_sub` and embassy `Instant::checked_sub` |
| `checked_add(Duration)` | timeouts and staleness bounds in wait loops (all four adapters have one). Weaker than the rest; keep for symmetry with std/embassy only if the maintainer accepts that reason |
| `checked_duration_since` | age of a `latest_common_time` result; raw `a.as_nanos() − b.as_nanos()` can overflow. Name matches `std::time::Instant` and embassy. `None` iff `earlier > self`; it cannot overflow |
| `MAX` | `remove_samples_through(Timestamp::MAX)` (full dynamic clear) |
| `Hash` | caller-side maps keyed by stamp; no concrete user found — weakest derive, zero cost |
| `Display` | error messages. Format `[-]s.nnnnnnnnn`, always nine fractional digits, from integer `unsigned_abs() / 1e9` and `% 1e9`. Exact and reversible; never passes through `f64` |
| `TryFrom<SystemTime>` | the replacement for `Timestamp::now()`; transforms_io. `TryFromIntError` falls out of `i64::try_from(i128)`, so no new error type is needed |
| serde | transparent `i64` |

Corrections to the draft:

1. **Add `as_secs_nanos`** (above).
2. **Drop `MIN`.** No caller. `MAX` has one.
3. **The comment "the ROS/NTP shape" is wrong.** NTP timestamps are u32
   seconds since 1900 plus a 2^-32 s fraction [M], not nanoseconds. It is the
   ROS / POSIX `timespec` shape.
4. **State the wire break.** JSON is unchanged for non-negative stamps. Postcard
   encodes `u64` as a varint and `i64` as a zigzag varint [M], so the frozen
   wire-format bytes change and old postcard data decodes to wrong instants
   rather than failing [D — not run]. That is a silent-wrong-answer path
   across a version boundary and belongs in MIGRATION.

Deliberately absent, each for a reason:

- `now()`: panic, wasm trap, and the hazard 6 clock mix.
- `zero()`: `from_nanos(0)`; a named zero suggests an epoch the crate does not
  define.
- Any `f64` seconds accessor or constructor: lossy above 104 days;
  `as_nanos() as f64 * 1e-9` at the call site makes the loss the caller's
  visible choice.
- `Add`/`Sub` operators: today they return `Result`, which is what the
  `checked_*` names say honestly.
- `TryFrom<Timestamp> for SystemTime`: three lines outside, no demonstrated
  caller once dimos uses ns directly [U].
- A niche (`i64::MIN` reserved so `Stamp` is 8 bytes): contradicts "no
  timestamp value is reserved", and `Stamp` is not stored per sample.

Open point, not resolved here: with time concrete, `TryFrom<SystemTime>` is
the only `std`-gated item. Either it is worth a feature or the draft's
candidate 5 (drop `std`) wins and the six-line conversion becomes a doc
recipe. The `i64::MIN` slip in my own probe argues mildly for keeping the impl.

---

## 7. Strongest argument against the recommendation

The evidence base is four adapters, all ROS-shaped, all on `std`; the
dependents study itself says "unused by three thin adapters and one unmerged
PR is weak evidence about `no_std` … users in private code". The users most
likely to need a non-nanosecond `T` — bare-metal firmware with a 32,768 Hz or
other non-decimal tick — are exactly the ones a crates.io reverse-dependency
search cannot see, and the crate's gate builds four bare-metal targets for
them. For those users the concrete type turns an exact tick-domain registry
into one with a rounding step at every insert and every query, in a crate
whose stated priorities include exactness. And a nanosecond `i64` is a ROS
convention: adopting it as the only time type is the "spiritual mirror of
tf2" priority (ranked last in AGENTS.md) deciding a core type.

Why I still recommend it: the rounding is bounded at 0.5 ns per stamp and is
order-preserving; the same users today must already write a newtype and a
25-line trait impl; the generic's failure path (`TimestampError` inside a
lookup) exists only for them and is itself a diagnostics hazard; and the
decision is narrowly reversible (section 5). The case would flip on one
demonstrated user whose clock cannot be expressed in nanoseconds within
±292 years.

Secondary argument against `i64` specifically (vs `u64`): negative instants
become representable, so an upstream sign bug that today fails at the user's
conversion now inserts successfully. Counter: today that conversion is a
hand-written cast that two of four adapters got wrong silently.

## 8. Not verified

- Dependents' code (all dimos/roslibrust/transforms_io statements are from
  `map-dependents.md`).
- Whether the 7 clock-independent tests in `tests/time_traits.rs` have
  `Timestamp` twins.
- Which existing tests pin interpolated lookups bit for bit.
- embassy tick-rate list, hifitime method names, Windows `SystemTime`
  truncation, postcard zigzag encoding, `rcl`/tf2 int64 representation: memory.
- ARM sizes/alignments; cost of `u128` division on Cortex-M0+.
- The replacement API was not implemented; line counts for it are estimates.
