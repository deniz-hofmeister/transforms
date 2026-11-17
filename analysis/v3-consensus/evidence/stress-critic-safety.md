# Safety critique of designer A's v3 draft

Subject: `claude-position.md` (draft 1). Baselines: v2.4.0 (master `adccbdd`)
and the round-2 review (`analysis/v3-review/REPORT.md`). Yardstick: AGENTS.md
priority 2, "a plausible-looking wrong answer returned silently".

## Evidence status

- **Run:** one probe against 2.4.0 as a path dependency,
  `evidence/critic-safety/probe/src/main.rs`,
  with `CARGO_TARGET_DIR=<scratch>/critic-safety/target cargo run --offline -q`
  (debug build, this Linux host; behaviours only, no timings). Output:

  ```
  A a*b = Ok(("map", "base", 3.0))
  A b*a = Err(IncompatibleFrames { expected: "base", found: "map" })
  B cross = 6 odom<-base stamp At(Timestamp { t: 2 })
  B before = 1
  B reinsert = Ok(())
  B after = 6
  C swapped = -6 base<-map
  F reinsert map<-base = Err(ReparentingNotSupported { current_parent: "odom" })
  D built = true
  D inverse = Err(NonFiniteValues)
  E accepted = true
  G 0.7071 = Err(NonUnitRotation(0.9999904099540154))
  ```

- **Not run:** everything about the draft. It is a sketch with no code, so
  every statement about v3 behaviour is a reading of the signatures in its §1
  and the prose in §2–3. Where the sketch is silent I say so.
- **Not re-verified:** dependent behaviour is cited from `map-dependents.md`,
  whose downstream line numbers are themselves unverified. No dependent source
  was read.

## Findings per examined point

Severity scale: **High** = silent wrong geometry reachable by an ordinary
call pattern with downstream evidence; **Medium** = silent, reachable, no
observed user; **Low** = loud, or needs extreme inputs.

### 1. Unlabelled `Copy` results versus labelled `Transform` / `CrossTimeTransform`

**The roslibrust re-insert (review item 1, dependents hazard #7).** Probe B
reproduces it on 2.4: the cross-time result re-inserts with `Ok` and the
honest `odom <- base @ t2` sample goes from 1 to 6.

| | Accidental pass-through | Deliberate rebuild | Distinguishable after it leaves the call site |
|---|---|---|---|
| 2.4 | zero lines, `Ok` | n/a | no |
| Review | does not compile | one `Transform::new` from six accessors on a type named `CrossTimeTransform` | yes, by type |
| Draft | does not compile (no `Edge`) | one `Edge { .. }` literal, identical in shape to every honest insert | **no** |

Honest verdict: the draft **closes the zero-line accident** (equal to the
review), **leaves the capability** (equal), and is **weaker than the review
downstream**. `lookup` and `lookup_across` return the same type, so an
adapter helper `fn to_msg(parent, child, stamp, Transform)` accepts both. The
review notes roslibrust's `FromTransform` "cannot convert the result to a ROS
message"; under the draft it can, and the adapter must invent the stamp. That
is the broadcast half of hazard #7 with the type barrier removed. The draft's
claim "there is no insertable labelled value to mis-insert, so the special
type is unnecessary" is true only inside this crate.

**New path the draft opens: relabelling on re-insert.** Probe F: on 2.4 a
flattened `map <- base` result offered to a registry where `base` is pinned to
`odom` fails with `ReparentingNotSupported`, because the result carries the
frames it was computed for. Under the draft:

```rust
let t = reg.lookup("map", "base", at)?;            // map <- base
reg.insert(Edge { parent: "odom", child: "base", transform: t, stamp: at.into() })?; // Ok
```

All checks pass: the parent matches the pin and the geometry is valid. AGENTS.md
calls "flatten a chain, re-publish it" the ordinary pattern. The same applies
to the stamp: a result looked up at `t1` can be inserted `At(t2)` or `Static`.

**Swapped lookups stop being self-describing.** Probe C: 2.4 answers a swapped
call with `-6 base<-map`. roslibrust builds its outgoing messages from "accessors
on the way out" (`map-dependents.md` §2), so a swapped call there yields a
message that is at least internally consistent. Under the draft the adapter
labels the result from its own variables; transforms_io had those named
backwards in 3 of 4 backends for 12 releases (hazard #8). The geometry and the
`frame_id` then disagree on the wire.

- Severity: **High** (relabel and swap), Medium (cross-time).
- Who: roslibrust (exposes cross-time, inserts and publishes); transforms_io
  and pelorus (re-export `Transform` in public signatures).
- Smallest closure: see point 5 (named lookup arguments) for the swap. For
  cross-time, either keep a distinct result type for `lookup_across`
  (the review's item, about 6 accessors) or accept the residual and say in
  the rustdoc that the result is not a sample of any edge. The relabel path
  has no cheap closure short of labelled results; it is the price of the
  design and must be declared as a cost, which the draft does not do.

### 2. `Edge<'a>` with public fields and no constructor

Mostly sound, and I could not make it worse than 2.4's positional
`Transform::new(parent, child, ..)`:

- `transform` is a private-field value, names are checked at insert, and
  `stamp` has no default. `Edge` really has no invariant of its own.
- Named fields remove the positional parent/child swap. No dependent evidence
  exists for that swap at insert; the benefit is real but unobserved.

Residual, all Low:

- A reused `let mut e = Edge { .. }` in a loop can update `transform` and keep
  a stale `stamp`. 2.4 forces a fresh fallible constructor per sample.
- The struct is not `#[non_exhaustive]` (it cannot be, with literal
  construction). Any later field is a break; not a safety matter.
- Unchanged from 2.4: inserting the inverse pose (parent expressed in child)
  is undetectable.

No amendment required.

### 3 and 9. Public total `Mul`/`inverse` and the removed frame check

Probe A: 2.4 rejects `b * a` with `IncompatibleFrames`. Under the draft both
orders type-check and return a value. A concrete slip:

```rust
let map_odom = reg.lookup("map", "odom", t)?;
let odom_base = reg.lookup("odom", "base", t)?;
let wrong = odom_base * map_odom;   // compiles, finite, plausible
```

With pure translations the two orders agree, so a test fixture without
rotation passes. The stamp check goes too: two results at different instants
compose.

Weighing it honestly:

- No studied dependent calls `Mul`, `inverse` or `interpolate`
  (`map-dependents.md` §5, §8 item 3; "dimos and other users were not checked"
  for `Mul`). pelorus inverts in nalgebra, unlabelled already. So the removed
  check protects nobody observed.
- The review keeps this as its last checked path ("for a pose written as a
  `Transform`, `*` still checks frames and stamps"), and issue #43 is the one
  public case of a label check catching a wrong-direction application. The
  draft removes that path and at the same time makes the operator cheap and
  `Copy`, which invites the use nobody has today.
- The labelled `Mul` has its own hole the draft does remove: it joins on equal
  names, including `""` (trade-off T1).

`inverse` being total is a small regression. Probe D: a valid transform with
translation `(f64::MAX, f64::MAX, 0)` and a 45 degree rotation returns
`Err(NonFiniteValues)` from 2.4's `inverse`; a total conjugate inverse returns
`inf`. The draft's §3 sentence "valid by construction is true of every value"
is therefore **false for translations**: public `*` and `inverse` can both
produce a non-finite `Transform`, and `transform_point` then returns NaN.

- Severity: **Medium** (direction), Low (overflow).
- Smallest closure: do not ship `impl Mul for Transform`. Zero users, and
  AGENTS.md says no speculative API; the registry composes internally. Keep
  `inverse` (its misuse yields the other direction's exact answer). If `Mul`
  stays, the draft must state the lost check as a cost and correct the §3
  claim to "rotations are unit by construction; translations are finite only
  where built, inserted or returned by a lookup".

### 4. `transform_point` versus `transform_vector`

Both take and return `Vector3`. A velocity or surface normal through
`transform_point` gains the translation; a lidar return through
`transform_vector` loses a lever arm of a few decimetres. Both are finite and
plausible. nalgebra, where pelorus and whirl_tf apply transforms today, keeps
`Point3` and `Vector3` apart, so the draft's application path is less safe
than the one users already have.

The review deletes application entirely, so this is a path the draft adds.

- Severity: **Medium**. Who: any caller adopting the new method; none today.
- Smallest closure, a deletion: drop `Transform::transform_vector`. It is
  `tf.rotation().rotate(v)`, which the draft already exports and which cannot
  add a translation by accident. One application method on `Transform`, and
  the golden vectors still pin `R p + t`.

### 5. Positional `lookup(target, source, at)` and `lookup_across`

The order equals 2.4 and tf2, so the swap itself is not new. What is new is
the consequence (point 1): the result no longer says which direction was
computed. Hazard #8 is the only direction bug with downstream evidence, and
the draft spends its named-argument device on `Edge`, where there is none.

`lookup_across((&str, Timestamp), (&str, Timestamp), &str)` binds each time to
its frame, which fixes the 2.4 five-positional signature. Swapping the two
pairs still yields the exact inverse silently, and a moving frame passed as
`fixed` is silent in every design.

- Severity: **High** in combination with point 1.
- Smallest closure: the draft's own candidate 6. Take the query as a
  struct literal, `Lookup { target, source, at }`, and the cross-time one as
  `{ target, target_at, source, source_at, fixed }`. No allocation, and
  consistent with the `Edge` argument.

### 6. Concrete `i64` `Timestamp`

**Clock mixing.** The generic `T` separated clocks only for users who chose
two types. That is dimos (`SystemTime`) and roslibrust's test-only
`MockTimestamp`; transforms_io and roslibrust's default already put every
clock in `Timestamp`. Mixed epochs in one registry are mostly loud under the
draft: same edge gives `InsertError::Expired` (better than 2.4's `Ok` then
evicted), different edges give `OutOfRange` or `NoCommonTime`. The silent case
is the existing one, hazard #2: a backwards jump smaller than `max_age`
interleaves two epochs and interpolates between them. Neither the draft nor
the review closes it. Severity of the type loss itself: **Low**.

**Signed arithmetic, unspecified in the sketch, each a required definition:**

- Interpolation ratio and expiry subtract stamps. With samples near `MIN` and
  `MAX`, `t1 - t0` overflows `i64`: a panic in debug, a wrapped (wrong) ratio
  in release. Use `abs_diff` or `i128`, and pin it with a test at both
  extremes.
- `from_secs_nanos(-1, 500_000_000)` must mean -0.5 s (ROS floor convention)
  and `nanos >= 1_000_000_000` must be `None`, not carried. The draft's claim
  that every ROS stamp "converts losslessly" depends on this.
- `Display` of -0.5 s must print `-0.500000000`. Naive `/` and `%` on negative
  nanoseconds prints a different instant, which would reintroduce the
  misleading-diagnostic class the draft removes.
- `TryFrom<SystemTime>` must map pre-epoch to negative, not to `Err` or zero.
- Migrating adapters will write `from_nanos(x as i64)` from 2.4's `u64`;
  wrapping needs a stamp past 2262. Low.

### 7. `remove_samples_through(Timestamp::MAX)`

I found no silent wrong answer. Removing a prefix cannot open an interior gap,
drained frames keep pin and kind, and lookups then fail with `NoData`. The
rename makes the review's "silent change" (its item 12) loud, which is an
improvement.

Residual, Low: the public `MAX` constant makes
`cutoff.checked_add(d).unwrap_or(Timestamp::MAX)` a natural idiom that wipes
all dynamic history, and the `()` return gives no sign. Acceptable; a doc
sentence suffices.

### 8. `Rotation` normalizing at construction

- **Argument order: Medium-High, a regression against 2.4.** Probe E: the
  ROS/SciPy/glam-ordered identity `(0, 0, 0, 1)` passed to `from_wxyz` is
  accepted, and it is a 180 degree yaw. Any permutation of a unit quaternion
  is unit, so no validation can catch it. On 2.4 both ROS adapters avoid this
  by writing the struct literal `Quaternion { w, x, y, z }`
  (`map-dependents.md` §2, transforms_io and roslibrust). The draft makes the
  fields private, so every user is moved onto four positional `f64`. Smallest
  closure: keep a plain named-field input, e.g.
  `Rotation::try_from(Quaternion { w, x, y, z })` with `Quaternion` as
  field-only data like `Vector3`. Cost: one type the draft wanted gone.
- **Tolerance.** Normalizing inside the 1e-6 band is safe and removes the
  scale-by-squared-norm hazard. The sketch must say whether the band is on the
  norm or the squared norm (2.4: norm; probe G rejects `0.7071` at norm
  0.9999904).
- **Candidate 3, an unbounded normalizing constructor: reject.** It turns
  `(0.1, 0, 0, 1e-4)` into a confident rotation. If one is ever added it needs
  an explicit caller-stated bound.
- **Sign.** `q` and `-q` stay distinct under exact `==`. Same as 2.4.
- **Deserialization, unspecified.** The conjugate inverse and the Newton step
  are correct only near unit norm (the step diverges for squared norm above
  3). If `Rotation` or `Transform` derive `Deserialize` over private fields,
  the invariant is bypassed and every later result is silently wrong. The
  draft must state that decoding goes through `from_wxyz`.

## What the draft genuinely removes

Paths open in 2.4 and left open (or only narrowed) by the review:

1. **Off-unit rotations anywhere.** The review normalizes at three
   constructors and lists as residual that insert validates without
   normalizing and derived values drift. A unit type with renormalizing
   composition removes the class, provided the deserialization point above
   holds. Unverified: the 2.2e-16 bound over 1e9 compositions is the draft's
   citation, not re-run here.
2. **Error decided above the common ancestor.** Topology-first resolution
   makes the AGENTS.md precedence rule structural instead of a second
   diagnosis pass.
3. **`covered: Option` misread.** `NoData` and `OutOfRange` are separate
   variants.
4. **Misleading diagnostics.** Exact timestamp `Display` replaces lossy `f64`
   seconds, where a 1 ns miss prints as inside the range.
5. **Clamped ROS stamps** (hazard #5, two of four adapters), if
   `from_secs_nanos` is specified as above. The review leaves this outside.
6. **Evicted-on-arrival `Ok`** via `Expired`. Shared with the review.
7. **Silently unbounded registry.** No `new`, no `Default`.
8. **`""`-joined hand composition** (T1) and the zero-line cross-time
   re-insert. The second is shared with the review.
9. **Wall-clock "latest".** No `Timestamp::now()` to reach for (hazard #6);
   also removes the only documented panic.
10. **Boundary change made loud** by renaming the cleanup call.

Frame names in insert errors and `BatchError::index` are diagnostics, not
wrong-answer closures, but they address why adapters drop loud errors
(hazards #3, #9).

## Other gaps found while reading

- `LookupError::NonFinite` is marked "only if the exit check stays". It must
  stay, or hazard #11 (`Ok(inf)`) reopens.
- Interning: a failed `insert` or `insert_all` must leave no slot behind.
  Otherwise a name from a rejected batch turns `UnknownFrame` into
  `Disconnected`, a wrong diagnosis. Needs a test; the sketch does not say.
- Hazard #4 (interpolation across a dropout, `Ok(4.0)`) stays open in every
  design. The draft lists it as undecided; it remains the largest silent
  hazard.

## Ranked amendments

Required before the draft can claim parity with the review on safety:

1. Named-argument lookups (`Lookup { target, source, at }` and the cross-time
   equivalent). Closes the swap that unlabelled results make invisible.
2. Declare the cost of unlabelled results: relabel-on-reinsert (probe F is
   rejected today, silent under the draft) and the weaker cross-time barrier.
   Decide `lookup_across`: distinct result type, or documented residual.
3. Remove `impl Mul for Transform` from the public API, or keep it with the
   lost frame and stamp checks stated. Correct the §3 "every value is valid"
   claim.
4. Delete `transform_vector`; free vectors go through `Rotation::rotate`.
5. A named-field quaternion input alongside or instead of positional
   `from_wxyz`.
6. Specify: decoding goes through the validating constructor; overflow-free
   stamp differences; `from_secs_nanos` floor semantics; negative `Display`;
   pre-epoch `SystemTime`.
7. Keep the lookup exit finiteness check.
8. No unbounded normalizing rotation constructor.
9. Failed inserts intern nothing.

Acceptable residual risks:

- Loss of type-level clock separation (two users, mostly loud).
- `remove_samples_through(Timestamp::MAX)` as a full clear (loud aftermath).
- `Edge` public fields and stale-stamp reuse.
- `inverse` total, with `inf` at extreme magnitudes, if amendment 3's wording
  is fixed.
- `q` versus `-q` under exact equality.
- Wrong `fixed` frame in cross-time lookups, sub-`max_age` clock jumps, and
  the interpolation gap: open in 2.4 and the review as well.
