# Scope and minimalism review of designer A's v3 draft (section 1)

Reviewed: `claude-position.md` draft 1 against AGENTS.md "Mindset: minimal and
intentional" and "Scope: what belongs in the core", with backwards
compatibility waived. Inputs read: the draft, `map-api.md`,
`map-dependents.md`, `analysis/v3-review/REPORT.md` items 7-9 and
"leaves alone / Not in 3.0", `src/core/registry/mod.rs:1162-1202`,
`src/core/buffer/mod.rs:475-483`. Not read: `map-internals.md`,
`map-prior-art.md`, the dependents' own source (not in the repo).

Rule tags used below:

- **S1** buildable correctly and exactly on the public API -> outside.
- **S2** needs sealed internals -> core or nowhere; never expose primitives.
- **S3a** substitute outside is merely verbose -> wait for demonstrated need.
- **S3b** substitute outside is a wrong-answer generator -> ship.
- **S4** no addition is precedent for its neighbours.
- **M1** no speculative API. **M2** prefer deletion. **M3** single-use
  generality (one caller, one setting).

## 0. What was verified, and how

- A stub crate with my proposed signatures and seven "built outside"
  helpers compiles warning-free:
  `cd <scratch>/critic-scope && CARGO_TARGET_DIR=$PWD/target cargo clippy --quiet`
  -> exit 0 (rustc 1.101.0-nightly b6eeb93c9, this x86_64 Linux machine).
  Bodies are `todo!()`: this proves the types are coherent and the outside
  helpers type-check, **not** that they are numerically identical to an
  implementation that does not exist yet.
- ROS `(i32, u32)` -> `i64` ns cannot overflow:
  `python3 -c "print((2**31-1)*10**9+2**32-1 < 2**63-1, -(2**31)*10**9 > -2**63)"`
  -> `True True`.
- `get_transform_at` is two resolves, one inverse, one product
  (`src/core/registry/mod.rs:1184-1196`), by reading.
- No performance number in this report is mine. No dependent code was read
  or run; usage claims are `map-dependents.md`'s, which itself marks
  downstream line numbers unverified.

## 1. The draft's item count is understated

The draft says "105 named items to roughly 45". Counting the draft's
section 1 the way `map-api.md` section 5 counts 2.4 (types, inherent
methods/consts, enum variants, public fields; no trait impls, no payload
field names):

| Group | Draft | Mine |
|---|---|---|
| Types | 11 | 10 |
| Vector3 fields | 3 | 0 |
| Rotation consts/methods | 8 | 3 |
| Transform consts/methods | 7 | 6 |
| Timestamp consts/methods | 8 | 2 |
| Stamp variants | 2 | 2 |
| Edge fields | 4 | 4 |
| Registry methods | 10 | 7 |
| Geometry/Rotation error variants | 2 | 2 |
| InsertError variants | 9 | 6 |
| BatchError fields | 2 | 2 |
| LookupError variants | 6 | 6 |
| **Total** | **72** | **50** |
| Operator / conversion impls | `Mul` x2, `From` x3, `TryFrom<SystemTime>` | `Mul` x1 |
| Named error payload fields | >= 24 (one `..` unspecified) | 11 |
| Cargo features | `std`, `serde` | none |
| Feature combinations to gate | 4 | 1 |

72 is a 31% cut from 105, not 57%. "~12 type names" is 11, correct.

## 2. Rulings, item by item

### Geometry

| Item | Ruling | Deciding rule |
|---|---|---|
| `Vector3` struct | **Cut -> `[f64; 3]`** | M2/M3. The draft's `Vector3` has no invariant, no operators, no methods; it is a named tuple plus two conversions. It distinguishes nothing (points and free vectors share it). nalgebra and glam both convert from arrays. Cost: outbound code destructures `let [x, y, z] = tf.translation();` instead of `.x`. No swap hazard is added: x/y/z order is not the convention anyone gets wrong. |
| `Rotation` (unit, private) | **Keep** | S2. It is the one type carrying an invariant; it deletes the re-checks. |
| `Rotation::IDENTITY` | **Keep** | The translation-only edge is the commonest construction; without it every such call is `from_wxyz(1.0, 0.0, 0.0, 0.0).unwrap()`. |
| `Rotation::from_wxyz` | **Keep** | The only entry. |
| `w()/x()/y()/z()` | **Merge -> `to_wxyz(self) -> [f64; 4]`** | M2 (4 -> 1), mirrors the constructor's name and order. **Honest counter:** named accessors cannot be mis-ordered on read-out; `let [x, y, z, w] = r.to_wxyz()` is a silent wrong answer, and AGENTS.md ranks that failure first. The same positional hazard is already accepted on `from_wxyz`. This is the one ruling where minimalism trades against swap-safety; if the maintainer weighs safety here, keep the four and the total is 53. |
| `Rotation::inverse`, `Rotation::rotate`, `impl Mul for Rotation` | **Cut (private)** | S1 + M2. Each is reachable exactly through `Transform` (`Transform::new([0.0; 3], r).inverse().rotation()`, `transform_vector`, `*`), which is total in my shape (see `Transform::new`). Three duplicate spellings of operations `Transform` already exports. `Rotation` becomes a validated value, not a second algebra. |
| `Transform` (geometry only, `Copy`) | **Keep** | The answer type. |
| `Transform::IDENTITY` | **Cut** | S1, once `new` is `const` and total: `Transform::new([0.0; 3], Rotation::IDENTITY)`. Keep only if `new` stays fallible. |
| `Transform::new -> Result` | **Change -> total `const fn`** | Exactness. The draft concedes public `*` can yield `inf`, so "a `Transform` is finite" is true of built values only - the same "valid by construction, except derived" split the draft's section 0 criticises in the review. Insert must check finiteness regardless (the draft's own section 3 says so). A check at `new` therefore guards nothing the boundary does not, and costs a `Result` on every construction. A NaN/inf applied outside the registry yields NaN/inf, which is not a plausible-looking answer. Counter: AGENTS.md "validated where they are built". If kept fallible, it returns a two-variant error of which one arm is unreachable. |
| `translation()`, `rotation()` | **Keep** | Read-out. |
| `transform_point` | **Keep** | S2/S3b. The review's "buildable outside" (item 7, option c) held because 2.4 exports `rotate_vector`. With `Rotation`'s algebra sealed it is core or nowhere, and the outside substitute is a hand-written sandwich product (active/passive flip). It is also the executable definition of lookup direction (hazard #8, wrong for 12 releases in one adapter) and gives `tests/golden_vectors.rs` a crate path to pin again. |
| `transform_vector` | **Keep** | S3b. The substitutes users write are `transform_point(v)` (adds the translation: silent, plausible) or `transform_point(v) - t` (cancellation at large `t`). Two adjacent names are the documentation of the distinction. Weakest of the three application items: no studied user. |
| `impl Mul for Transform` | **Keep** | S2 (rotation product is sealed), and it is what lets `lookup_across` leave the core. It is also the only pose-application path now that results carry no labels (`tf * pose`). M1 objection stands: zero studied users of 2.4's `Mul`; the one demonstrated need is indirect (roslibrust's cross-time lookup). |
| `Transform::inverse` | **Keep, low confidence** | S2. For registry data a swapped lookup gives it, so the need is for caller-built transforms (a detection `T_cam_tag` inserted with the tag as parent) and for `lookup_across` outside. Zero studied users; pelorus inverts, but in f32 nalgebra. Under M1 read literally this is "nowhere until asked". I keep it because it is one `pub` on code the lookup needs anyway and because without it my `lookup_across` ruling has no exact outside form. |

### Time

| Item | Ruling | Deciding rule |
|---|---|---|
| `Timestamp(i64 ns)` newtype | **Keep** | The unit lives in `from_nanos`; a bare `i64` parameter accepts seconds. Marginal. |
| `from_nanos`, `as_nanos` | **Keep** | The whole interface. |
| `MIN`, `MAX` | **Cut** | S1 (`from_nanos(i64::MAX)`). `MAX`'s only stated use is the clear recipe, which I replace (below). |
| `from_secs_nanos` | **Cut** | S1. The two adapters that got this wrong did so against `u64` (`sec.max(0) as u64`). With `i64` the naive one-liner `i64::from(sec) * 1_000_000_000 + i64::from(nanosec)` is correct for every ROS stamp and cannot overflow (verified above). Signedness removes the hazard; the constructor is then sugar. The dependents study also ruled "do not" with a reopen condition (a third clamping integration) that has not occurred. |
| `checked_add`, `checked_sub`, `checked_duration_since` | **Cut** | S1. Exact on `as_nanos`/`from_nanos` with `i64::checked_*` (helper compiles in the stub). The registry needs the subtraction privately for expiry; no caller of the public forms is demonstrated. 2.4's `Add<Duration>` had zero internal callers too. |
| `TryFrom<SystemTime>` and the `std` feature | **Cut both** | S1 + M3. dimos, the one `SystemTime` user, already carries a checked signed ns <-> `SystemTime` conversion (`tf.rs:50-81` per the study, unverified by me). A feature whose entire content is one conversion impl doubles the build matrix for one user who has the code. The crate becomes unconditionally `#![no_std]` + `alloc`; errors implement `core::error::Error` (MSRV 1.85 >= 1.81). |
| `Stamp { At, Static }` | **Keep** | Versus two entry points: `insert`/`insert_static`, `insert_all`/`insert_all_static`, (and `reparent` twice) is +2..3 methods to save 3 named items, and `latest_common_time` still needs a non-`Option` way to say "all time". |
| `From<Timestamp> for Stamp` | **Cut** | M2. `stamp: t.into()` hides the static/dynamic decision that `Stamp::At(t)` spells; two ways to write one thing. |

### Registry

| Item | Ruling | Deciding rule |
|---|---|---|
| `Edge<'a>` | **Keep; derive `Copy`** | Positional `(parent, child, ..)` puts two adjacent `&str` on the call whose swap builds a plausible wrong tree; an owned labelled type is two allocations and an insertable/composable value. `Edge` is input-only and is the batch item. `Copy` matters: it is what lets the caller format its own context after a failed insert without cloning (roslibrust clones both names per ingested transform today). |
| `with_max_age` + `unbounded` | **Merge -> `new(max_age: Duration)`** | M3. Zero 2.x users of the unbounded constructor. A `max_age` no stamp difference can reach never expires - 2.4 already treats an unrepresentable threshold as "keep" (`src/core/buffer/mod.rs:475-483`), so `Duration::MAX` is unbounded by ordinary semantics, not by a reserved value. Must be documented. Medium confidence: it reads slightly like a sentinel. |
| `insert` | **Keep** | |
| `insert_all` + `BatchError` | **Keep both; weakest registry keep** | S2: atomicity cannot be built outside (no rollback, no `Clone`). Under S3a the loop is merely different, not wrong (it is what tf2 and both bridges do), so this rests on one user, dimos, on an unmerged PR. If it stays, `index` is mandatory: the batch is the one place the caller cannot know which member failed. A transaction object is S2's "primitives are a larger commitment": a type, a lifetime, commit/drop semantics. Dropping atomic batches is defensible; I do not, because the only non-adapter user is built on it. |
| `reparent` | **Cut, low-medium confidence** | S3a + M1. Zero users since 2.2. Outside form: `remove_frame(child); insert(seed)` under the same `&mut` (compiles in the stub). Difference: on a failed seed (cycle, non-finite) the frame is gone rather than unchanged - loud, with loss of a history `reparent` drops anyway on success; and the kind is not preserved. Cutting it also removes three `InsertError` variants `insert` can never produce (`UnknownFrame`, `NoParent`, `ParentUnchanged`), i.e. the unreachable-arm smell AGENTS.md cites as the reason the buffer errors are split. Against: hazards #9/#14 are real occurrences of re-parenting, and the pre-check for a cycle is not buildable outside, so the validate-before-mutate version is core-or-nowhere. If kept, give it its own error enum rather than widening `InsertError`. |
| `remove_frame -> bool` | **Keep as is** | S2; two users. The `bool` is not a named item and costs nothing; a `Result` would add a type for a condition callers ignore. |
| `remove_samples_through(cutoff)` | **Merge -> `clear_samples()`** | M3. The only demonstrated purpose is clock-reset recovery (ranked need #2), which wants everything gone; the predecessor has zero users. A parameterless clear has no boundary rule - the review's item 12 and the draft's rename exist only because of inclusive/exclusive semantics - and needs no `Timestamp::MAX`. S2/S3b keeps the operation in core: the outside substitute is a `remove_frame` loop that releases pins (whirl_tf does this). This reverses settled D5; D5 was decided with the cutoff method assumed present. Loses: manual trimming of an effectively unbounded registry (no user). |
| `lookup(target, source, at)` | **Keep** | |
| `lookup_across` | **Cut** | S1, high confidence. With unlabelled `Copy` transforms it is `lookup(fixed, target, t_t)?.inverse() * lookup(fixed, source, t_s)?` - the same two resolves, one inverse and one product 2.4 runs internally. One user, who requested it. The failing leg becomes evident to the caller, and the `(&str, Timestamp)` tuple signature disappears. Bit-identity with a v3 internal version is unverifiable (none exists). |
| `latest_common_time` | **Keep** | S2/S3b, per AGENTS.md (the max-of-starts guard). |
| `lookup_latest` (candidate) | **Reject** | S1 + S4: exact on `latest_common_time` + `lookup` under one borrow (compiles in the stub). It cannot replace `latest_common_time` either: choosing one instant for several pairs (multi-sensor fusion) needs the instant without the lookup, and a per-pair "latest" returns different stamps. The adapters' wrong "latest" came from not calling `latest_common_time`, and their private locks are theirs to fix. |
| Lookup argument struct (draft 4.6) | **No change; not decided by the scope rules** | Honest note: `lookup`'s two adjacent `&str` are the swap that returns a valid-looking inverse, and it is the most-called method; `Edge` protects the less-called one. The evidence (hazard #8) traces to 1.x parameter *names* in an adapter, not to position, so I do not add a type on it. |

### Errors

Test applied to each payload field: can the caller act on it, and does the
caller already hold it? A field that echoes the caller's own argument fails
the second half; its only function is a self-contained `Display` after `?`.
I cut echoes. This is the ruling most in tension with the "diagnostics"
goal: an error propagated far from the call site prints without frame
names unless the caller adds them (one `map_err`, possible because `Edge`
is `Copy` and borrowed).

| Item | Ruling | Reason |
|---|---|---|
| Per-operation enums instead of flat `RegistryError` | **Keep** | The draft applies AGENTS.md's own total-conversion argument to the public surface. |
| `GeometryError` | **Rename `RotationError`** | With `Transform::new` total, `from_wxyz` is its only producer. |
| `NonUnitRotation { norm }` | **Keep** | Actionable: the caller can renormalize within its own bound and retry (hazard #10), which is why no normalizing constructor is needed. |
| `InsertError::NonFinite` | **Add - missing in the draft** | Draft section 3 says "insert rejects non-finite" but `InsertError` has no variant for it. |
| `EmptyFrameName` | **Keep**, unit | |
| `SelfParent { frame }` | **Merge into `Cycle`** | A self-edge is a cycle of length one; same caller action. Payload is an echo. |
| `Cycle { child, parent }` | **Keep, unit** | Both fields echo the `Edge`. |
| `ParentMismatch { frame, pinned, offered }` | **Keep `pinned` only** | `pinned` is registry state the caller cannot otherwise obtain; `frame` and `offered` are the `Edge`'s fields. |
| `KindMismatch { frame, pinned_static }` | **Keep, unit** | `pinned_static` is the negation of the offered stamp's kind. |
| `Expired { frame, stamp, newest }` | **Keep `newest` only** | `newest` is what lets the owner detect a clock reset. |
| `UnknownFrame`, `NoParent`, `ParentUnchanged` (reparent only) | **Cut with `reparent`** | |
| `BatchError { index, error }` | **Keep** | Both actionable. |
| `LookupError::UnknownFrame { frame }` | **Keep** | Says which of the two. |
| `Disconnected { target, source }` | **Keep, unit** | Pure echo. |
| `NoData { frame }` | **Keep** | The failing hop. Also serves `latest_common_time`'s "hop holds nothing". |
| `OutOfRange { frame, requested, earliest, latest }` | **Keep; cut `requested`** | `earliest`/`latest` separate transient from permanent; `requested` is the caller's argument. |
| `NoCommonTime { frame, .. }` | **Keep; the draft must specify `..`** | I write `{ frame, earliest, latest }` (the hop's range). An unspecified payload is not a design. |
| `NonFinite` (lookup) | **Keep** if the exit check stays | No payload to act on; it converts a silent `inf` into an error. |
| One `LookupError` for `lookup` and `latest_common_time` | **Accept** | `OutOfRange`/`NonFinite` are unreachable from one, `NoCommonTime` from the other. A third enum costs more than two dead arms; say so in rustdoc. |

### serde

**Cut from 3.0.** M1: zero users. S1: `to_wxyz`/`from_wxyz` and
`as_nanos`/`from_nanos` let a caller derive on its own record type with
`try_from`. The draft never says which types carry serde; with labels gone
the natural 2.4 record (parent, child, stamp, geometry) no longer exists in
this crate, so the crate would be freezing a wire format for bare geometry
that nobody has asked to transmit. Deciding asymmetry: adding serde later
is additive and reversible; a shipped wire format with frozen bytes is
neither. Cost, stated: the `Stamp` decode hazard (`null`/missing ->
`Static`) moves into callers' hand-written records, where an
`Option<i64>` stamp would reintroduce it. That is the reopen condition: the
first user who serializes stamps.

With `std` and `serde` both gone the gate checks one feature combination
instead of four, and `tests/serde.rs` (430 lines), the postcard and
serde_json dev-dependencies and the frozen-bytes obligation go.

## 3. The other direction: what is missing

Things a caller cannot build correctly outside:

1. **`InsertError::NonFinite`.** Omitted in the draft (above).
2. **`NoCommonTime`'s payload.** Unspecified in the draft.
3. **A bound on interpolation gaps.** Core-or-nowhere and the largest
   silent hazard on record (`Ok(4.0)` across an 8 s dropout, all three
   users). Still no defensible shape; not in my API. A later
   `lookup`-variant that reports or bounds the gap is additive, so
   returning a bare `Transform` now forecloses nothing. Unresolved, and it
   should be said plainly in the docs that interpolation is unbounded
   within `max_age`.
4. **Validate-before-mutate re-parenting**, if `reparent` is cut: the
   cycle pre-check has no outside form. I accept the loud, lossy
   substitute; the maintainer may not.
5. **Interpolating two caller-built transforms.** With slerp private this
   is "nowhere". Zero users; correct to leave out, but it is a capability
   2.4 has and v3 would not.

Considered and not missing (S1 or "no raw state"): a normalizing rotation
constructor (normalize outside, `NonUnit { norm }` gives the number),
Euler/axis-angle/matrix conversions, frame enumeration or `parent_of`
(the shadow-log user can track its own inserts; a clear no longer needs
enumeration), `can_transform`, `Registry: Clone`, `is_retryable`.

## 4. The smallest API I would defend

50 public named items, one operator impl, no features, runtime dependency
`libm` only. Compiles as a stub (section 0).

```rust
#![no_std]            // unconditionally; needs alloc
#![forbid(unsafe_code)]

// ---- geometry ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation { /* private unit quaternion */ }
impl Rotation {
    pub const IDENTITY: Self;
    pub fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Result<Self, RotationError>;
    pub const fn to_wxyz(self) -> [f64; 4];
}

/// p_parent = R * p_child + t
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform { /* private */ }
impl Transform {
    pub const fn new(translation: [f64; 3], rotation: Rotation) -> Self;
    pub const fn translation(self) -> [f64; 3];
    pub const fn rotation(self) -> Rotation;
    pub fn inverse(self) -> Self;
    pub fn transform_point(self, p: [f64; 3]) -> [f64; 3];   // R p + t
    pub fn transform_vector(self, v: [f64; 3]) -> [f64; 3];  // R v
}
impl core::ops::Mul for Transform { type Output = Transform; }

// ---- time ----
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(/* i64 ns */);       // + Display, exact
impl Timestamp {
    pub const fn from_nanos(n: i64) -> Self;
    pub const fn as_nanos(self) -> i64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]   // deliberately unordered
pub enum Stamp { At(Timestamp), Static }

// ---- the tree ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge<'a> {
    pub parent: &'a str,
    pub child: &'a str,
    pub transform: Transform,
    pub stamp: Stamp,
}

pub struct Registry { /* private */ }
impl Registry {
    pub fn new(max_age: Duration) -> Self;
    pub fn insert(&mut self, edge: Edge<'_>) -> Result<(), InsertError>;
    pub fn insert_all<'a>(&mut self, edges: impl IntoIterator<Item = Edge<'a>>)
        -> Result<(), BatchError>;
    pub fn remove_frame(&mut self, child: &str) -> bool;
    pub fn clear_samples(&mut self);          // dynamic history only; pins and kinds stay
    pub fn lookup(&self, target: &str, source: &str, at: Timestamp)
        -> Result<Transform, LookupError>;
    pub fn latest_common_time(&self, target: &str, source: &str)
        -> Result<Stamp, LookupError>;
}

// ---- errors: Debug + Clone + PartialEq + Display + core::error::Error ----
#[non_exhaustive]
pub enum RotationError { NonFinite, NonUnit { norm: f64 } }
#[non_exhaustive]
pub enum InsertError {
    EmptyFrameName,
    NonFinite,
    Cycle,                                // includes parent == child
    ParentMismatch { pinned: String },
    KindMismatch,
    Expired { newest: Timestamp },
}
pub struct BatchError { pub index: usize, pub error: InsertError }
#[non_exhaustive]
pub enum LookupError {
    UnknownFrame { frame: String },
    Disconnected,
    NoData { frame: String },
    OutOfRange { frame: String, earliest: Timestamp, latest: Timestamp },
    NoCommonTime { frame: String, earliest: Timestamp, latest: Timestamp },
    NonFinite,
}
```

Items in this list that rest on the "complete core math crate" mandate
rather than on demonstrated use (M1 would cut them): `Transform::inverse`,
`transform_vector`, `insert_all`/`BatchError` (one user). Without them the
count is 44, and `lookup_across` would have no exact outside form.

## 5. Disagreements with the draft

1. `Vector3` -> `[f64; 3]`.
2. Four rotation accessors -> `to_wxyz` (trade-off stated; reversible to 53 items).
3. `Rotation::inverse`, `rotate`, `Mul` private.
4. `Transform::new` total and `const`; `Transform::IDENTITY` cut; finiteness
   is checked at insert and lookup exit only.
5. `GeometryError` -> `RotationError`.
6. `Timestamp::MIN`, `MAX`, `from_secs_nanos`, `checked_add`, `checked_sub`,
   `checked_duration_since` cut.
7. `std` feature and `TryFrom<SystemTime>` cut (draft candidate 5: yes).
8. `From<Timestamp> for Stamp` cut.
9. `with_max_age` + `unbounded` -> `new(max_age)`.
10. `reparent` cut, with its three error variants (low-medium confidence).
11. `remove_samples_through` -> `clear_samples()` (reverses D5).
12. `lookup_across` cut (high confidence).
13. `lookup_latest`: reject (draft candidate 1).
14. Echo payload fields removed from every error; `SelfParent` merged into `Cycle`.
15. `InsertError::NonFinite` added; `NoCommonTime` payload specified - both
    are gaps in the draft, not preferences.
16. serde not in 3.0.
17. The "~45 items" figure: the draft's section 1 is 72 on the inventory's
    own counting method.

Agreements worth recording: unit `Rotation`; label-free `Copy` `Transform`;
concrete signed `Timestamp`; `Edge<'a>`; per-operation error enums;
`BatchError.index`; no `Default` on `Registry`; `latest_common_time` in
core; dropping `thiserror`, `approx`, `hashbrown` (the `BTreeMap` index is
the draft's own "to be measured").

## 6. Limits

- Every usage claim is from a study of three thin adapters and one
  unmerged PR; private users are invisible. Each "zero users" cut is
  reversible additively except items 2, 4, 11 and 14, which change shapes.
- The echo-payload ruling (14) and the `to_wxyz` ruling (2) are where the
  maintainer's goals pull against each other (diagnostics and swap-safety
  versus minimalism); I ruled by the scope text, not by evidence.
- `new(Duration::MAX)` as unbounded, and `clear_samples` semantics on
  static edges (untouched), are stated, not prototyped.
- No implementation exists, so "exact outside" for `lookup_across` means
  "the same operations in the same order as 2.4", not measured bit-identity.
