# Migrating from 1.x to 2.0

Every break below was reproduced by compiling the 1.4.1-documented usage
against 2.0. Work through the compile errors first; then read the runtime
changes — code that compiles cleanly can still behave differently.

## Coming from a 2.0.0 pre-release

Five 2.x pre-releases were published — 2.0.0-alpha.1 and beta.1 through
beta.4 — and no release candidate was: on crates.io, 2.0.0 follows beta.4
directly. 2.0.0 deliberately broke the beta-series API freeze to close the
last one-way doors before stable; this section lists what that changed,
from the beta.4 baseline, and the `[2.0.0]` section of
[CHANGELOG.md](CHANGELOG.md) carries the full record. This guide's section
carries the earlier pre-releases too: what changed between them sits
inside API this migration rewrites anyway — the lookup-error variants
beta.3 reworked (break 3), the `Buffer` type the betas extended, private
now (break 5) — except one behavior fix, `get_transform_at`'s
coinciding-frame legs, covered in runtime change 3. The per-pre-release
history stays readable in the [changelog as published with
beta.4](https://github.com/deniz-hofmeister/transforms/blob/v2.0.0-beta.4/CHANGELOG.md)
(one beta.3 entry there mislabels the removed `NotFound` variant
"never-produced"; it was 1.x's primary lookup-miss error). Most of the 1.x
breaks below apply to you unchanged — beta.4 still had 1.x's public
`Transform` fields, public modules and `u128` stamp — so this section only
lists what is beta.4-specific and points at the numbered break where the
fix is written out.

What stops compiling since beta.4:

- `Transform.timestamp` is retyped to `Stamp<T>`, the fields are private, and
  `Transform::new` / `Transform::static_between` return `Result` (the `Stamp`
  section below, and break 7).
- `Transform::identity()` is removed (break 7).
- `Point` gains `Point::new` and `#[non_exhaustive]`, ending its struct
  literal (break 7).
- Deserializing a `Transform` now runs the same validation, so a denormalized
  rotation on the wire is a decode error (break 7).
- `Transform::UNIT_NORM_TOLERANCE` is a module-level const,
  `geometry::UNIT_NORM_TOLERANCE` — no turbofish (break 5's import list names
  it).
- `Quaternion::new(w, x, y, z)` is renamed
  `Quaternion::from_wxyz(w, x, y, z)` — same order and behavior, a name that
  states that order at the call site.
- `Quaternion` keeps only its rotation algebra: `+`, `-`, `/` (with
  `QuaternionError::DivisionByZero`), `scale`, `norm_squared` and the
  `Default` impl are removed (break 8).
- `Vector3` loses `dot`, `cross` and `unit_x`/`unit_y`/`unit_z`
  (break 9).
- `Buffer` and the `core` module are private, and so are the leaf modules
  (`geometry::transform`, `time::timestamp`, ...) that beta.4 still exposed
  (break 5).
- `registry.delete_transforms_before(t)` → `remove_transforms_before(t)`
  (break 6), and `Timestamp::as_seconds_unchecked()` →
  `as_seconds_lossy()` (break 6).
- Every `Registry` call reports `errors::RegistryError<T>`: the lookups
  returned `TransformError` in beta.4 and `add_transform` returned
  `BufferError`, and both of those are gone from the registry's signatures
  (break 3).
- `TimePoint` loses `static_timestamp`, `is_static`, `checked_add` and
  `as_seconds`, requires `as_seconds_lossy`, and gains `Debug` as a
  supertrait (the `Stamp` section below).
- `Timestamp`'s `t` field is private and holds `u64` nanoseconds instead of
  `u128`: `ts.as_nanos()` and `Timestamp::from_nanos(t)` (break 5).
- The wire format changed on both axes: `Stamp` is an explicitly tagged enum
  where beta.4 wrote a bare timestamp with no tag, and `Timestamp` is
  `#[serde(transparent)]` where beta.4 wrote a one-field `{"t": ...}` record.
  beta.4 payloads you persisted must be re-encoded, not reinterpreted — the
  **Serde wire format** notes below give the shapes.

Silent behavior changes since beta.4 — these compile:

- **`Timestamp + Duration` rejects durations beyond the `u64` range.** The
  arithmetic is `u64` nanoseconds now, so a `Duration` longer than
  `u64::MAX` nanoseconds (~584 years) returns `TimeError::DurationOverflow`
  where beta.4's `u128` arithmetic accepted it. `Timestamp - Duration` gained
  the same guard, reporting `TimeError::DurationUnderflow` — for a stamp that
  fits in `u64` at all, that subtraction already underflowed in beta.4, so
  only the addition changes its answer.
- **`Error::source()` on a failed lookup returns `None`.** beta.4's
  `TransformError::NotFoundAt` carried `source: Box<BufferError>`, and the
  covered range lived one link down that chain. `RegistryError::NotFoundAt`
  carries `frame`, `requested` and `covered` as its own fields instead, and
  chains to nothing; `RegistryError::TransformError` is the only variant with
  a source. Code that walked the chain to diagnose a miss now finds an empty
  one — read the payload (break 3).
- **`RegistryError<T>` is generic, so `Send`/`Sync`/`'static` follow `T`.**
  beta.4's `TransformError` and `BufferError` carried no time type and
  satisfied all three unconditionally. `TimePoint` requires only
  `Copy + Ord + Debug`, so a custom clock that is not `Send + Sync + 'static`
  now keeps `RegistryError<YourClock>` out of a
  `Box<dyn Error + Send + Sync + 'static>`. `Timestamp` and `SystemTime` are
  unaffected.
- **A frame drained by the wipe stays registered.** beta.4's
  `delete_transforms_before` dropped a frame left without transforms —
  the next insert under that child frame could re-parent it or change
  its kind, and a dropped leaf vanished from the tree entirely
  (`UnknownFrame`). 2.0's `remove_transforms_before` keeps it, parent
  and static-or-dynamic kind still pinned by its first insert, and
  lookups report `NotFoundAt` with `covered: None` (runtime change 5).

## Compile-time breaks

### 1. Registry construction

```rust
// 1.x
let mut registry = Registry::new(Duration::from_secs(60));

// 2.0
let mut registry = Registry::with_max_age(Duration::from_secs(60));
```

**Do not accept the compiler's suggestion.** rustc's help for this error
says "remove the extra argument", which produces `Registry::new()` — that
compiles, but creates a registry with **no automatic cleanup**: transforms
accumulate until you call `remove_transforms_before`. If you had a
`max_age`, you want `with_max_age`. (`Buffer::new(max_age)` has no
replacement — `Buffer` is private in 2.0; see break 5.) In
`no_std`, both constructors now exist too — automatic cleanup no longer
requires `std`.

### 2. Fallible insertion

```rust
// 1.x
registry.add_transform(transform);

// 2.0
registry.add_transform(transform)?;
```

`add_transform` returns `Result`, with `RegistryError<T>` as the error type
(see break 3). An ignored `Err` means **nothing was stored** — later lookups
will fail mysteriously. In 1.4.1 it returned `()` and refused nothing, so
there is no existing error handling to rename: every rejection below is new
to you, and each one catches something 1.x accepted.

- Values: `NonUnitRotation(norm)` and `NonFiniteValues`. Nothing validated a
  transform in 1.4.1 — a NaN translation and a norm-1.01 rotation both went
  into the registry and came back out of a lookup. The constructors
  reject them earlier now too (break 7), but the insert check is what
  catches a transform you built by composing with `*` or by reading a chain
  back out of a lookup: neither re-validates.
- Topology: `SelfReferentialFrame`, `ReparentingNotSupported` (call
  `remove_frame` first), and `CycleDetected`.
- Kind: `StaticDynamicConflict`, for a static and a dynamic transform under
  the same child frame (runtime change 1).

### Static transforms are a `Stamp` variant, not `t = 0`

`Transform.timestamp` is retyped from a bare timestamp to
`Stamp<T> { Static, At(T) }`: a dynamic sample carries `Stamp::At(t)`, a
static transform carries `Stamp::Static`. **No timestamp value is
reserved** — every real instant, including zero, the first reading of a
boot-relative clock, is ordinary dynamic data.

```rust
// 1.x
let sample = Transform { timestamp: t, /* ... */ };
let mount = Transform { timestamp: Timestamp::zero(), /* ... */ }; // static via sentinel

// 2.0
let sample: Transform = Transform::new(
    "map", "base",
    translation, rotation,
    Stamp::At(t),
)?;
let mount: Transform = Transform::static_between(
    "base", "camera",
    Vector3::new(0.1, 0.0, 0.5),
    Quaternion::identity(),
)?;
```

Every `Transform` literal is a compile error anyway — the fields are private
now (break 7) — so the stamp is one of the arguments you move into the
constructor. A 1.x
static publisher migrated as `Stamp::At(Timestamp::zero())` would store a
**single dynamic sample at the epoch**, so any lookup at a real time
fails loudly with `NotFoundAt` — covering the epoch alone — instead of
serving the mount.
Switch static publishers to `Transform::static_between` (or
`Stamp::Static`).

Two related breaks ride along:

- **`TimePoint` is pure time arithmetic now, in three methods.** Custom
  clock impls delete `static_timestamp()` and `is_static()` — no sentinel
  value to invent — and also `checked_add()` and `as_seconds()`, which
  by 2.0 have no caller left: `Stamp` took over the timestamp
  arithmetic, `as_seconds_lossy` the error formatting. What remains is `duration_since`,
  `checked_sub`, and `as_seconds_lossy`, the last no longer defaulted:
  implement it as a best-effort conversion that yields `f64::NAN` where
  your clock cannot convert, never a plausible-looking number, because it
  is what formats error messages. The trait also gains `Debug` as a
  supertrait — derive it on your clock type if you had not.
- **A child frame is static or dynamic for its lifetime.** The kind is
  fixed by the frame's first insert and survives the frame being drained,
  which also fixes a 1.x bug where an emptied frame silently flipped
  static on the next insert (see runtime change 5).

**Serde wire format:** 1.x had no `serde` feature, so this is the format
you encode *into*, not one you migrate from — but if you hand-rolled
serialization against the 1.x layout, the shapes differ.

`Stamp` is an explicitly tagged enum and `Timestamp` is
`#[serde(transparent)]`, so a JSON transform reads:

```json
{ "translation": { "x": 1.0, "y": 0.0, "z": 0.0 },
  "rotation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 },
  "timestamp": { "At": 1753142400000000000 },
  "parent": "map", "child": "base" }
```

and a static one carries `"timestamp": "Static"`. Staticness is spelled
out, never implied by an absent value: a `timestamp` field that is
*missing* or `null` is a decode error, so a producer that drops or nulls a
stamp cannot mint an eternal static transform. In a non-self-describing
format the stamp is a variant index ahead of the payload (`0` static, `1`
followed by the timestamp), but the width of that index — and of the
timestamp — is the codec's choice, not this crate's: postcard and bincode
2's `config::standard()` write a 1-byte index and a LEB128 varint
timestamp, while bincode 1.x and bincode 2's `config::legacy()` write a
fixed 4-byte little-endian `u32` index and a fixed 8-byte little-endian
timestamp. If you hand-roll an encoder, emit the shape your codec's config
specifies rather than the postcard one.

Cross-version decoding is format-dependent, so version-tag your streams:

- A 1.x-shaped **JSON** payload fails to decode, because the stamp must
  name its variant: the `{ "t": ... }` record 1.x's public field would have
  produced reports `unknown variant 't', expected 'Static' or 'At'`, and a
  bare timestamp is rejected as not being an enum at all. Nothing about the
  1.x `t = 0` static convention survives silently.
- A 1.x-shaped **postcard** stream with a realistic dynamic timestamp
  fails to decode outright (the timestamp varint is not a valid variant
  index). The one exception is a payload stamped exactly `t = 0` — the 1.x
  static convention — whose single `0x00` byte reads as variant 0, keeping
  the stream byte-aligned: it decodes cleanly as `Stamp::Static`. That is
  the right meaning for a 1.x static publisher, but wrong for a genuine
  boot-relative `t = 0` dynamic sample — do not rely on it in place of a
  version tag.

### 3. Error enum overhaul

Every `Registry` call — `add_transform` and all three lookups — now reports
one type, `errors::RegistryError<T>`. `TransformError` stays, but only for
what it describes: geometry and time, i.e. the `Transform` constructors,
`inverse`, `interpolate`, `*`, and `Transformable::transform`.

```rust
// 1.x
match err {
    TransformError::NotFound(from, to) => retry(),
    ...
}

// 2.0
use transforms::errors::RegistryError;

match err {
    RegistryError::UnknownFrame(f) => wait_for_publisher(f),  // typo / not yet published
    RegistryError::Disconnected { target_frame, source_frame } => {
        topology_bug(target_frame, source_frame)
    }
    // `frame` could not answer at `requested`; `covered` says why (see below)
    RegistryError::NotFoundAt { frame, requested, covered: Some((_, end)), .. } => {
        if requested > end { retry() } else { data_is_stale(frame) }
    }
    RegistryError::NotFoundAt { frame, covered: None, .. } => nobody_is_publishing(frame),
    _ => other(),                                             // mandatory: #[non_exhaustive]
}
```

The 1.x catch-all `TransformError::NotFound` is gone, replaced by the three
diagnosed variants above (mirroring tf2's LookupException /
ConnectivityException / ExtrapolationException). `NotFoundAt`'s `covered`
must be inspected before retrying, because it distinguishes two situations:
`Some((start, end))` means `frame` holds data the request falls outside of —
`requested > end` is merely too new (latency: retry), otherwise the data is
stale — while `None` means `frame` holds no data at all, so retrying
achieves nothing until someone inserts into it (see runtime behavior change
5). Both `requested` and `covered` are in the registry's own time type `T`,
not in seconds: compare them against the timestamp you asked with. The
`Display` text still renders seconds. The terminating "latest available"
retry loop — lower the request onto `covered`'s end, never raise it — is
documented on `RegistryError::NotFoundAt` in the crate docs.

`add_transform`'s rejections are variants of the same enum:
`RegistryError::NonUnitRotation(norm)`, `NonFiniteValues`,
`SelfReferentialFrame`, `ReparentingNotSupported { current_parent }`,
`CycleDetected`, `StaticDynamicConflict`. The first two are flat rather than
wrapped, and they are flat on every path that reports them — where a lookup
rejects a chain that overflowed a translation, it reports the same
`NonFiniteValues`, so a condition never has two spellings to match on. Note
what that does *not* say: a lookup never re-validates its result, and only
the half-chain it inverts is checked for finiteness, so a lookup toward an
ancestor (the documented direction, which inverts nothing) returns an
overflowed translation as `Ok`. Call `Transform::validate` yourself on a
result whose inputs can reach those magnitudes. The one wrapping variant,
`RegistryError::TransformError`, carries a geometry or time failure of an
operation on the resolved chain.

`BufferError` is gone from the public API along with `Buffer` itself (break
5); 1.x code that handled it did so around a hand-held `Buffer`, which now
has no replacement type — give the frame pair to a `Registry` and match
`RegistryError`. All error enums are `#[non_exhaustive]`, so every match
needs a `_` arm. Also removed: the `TimestampError` alias (use `TimeError`),
the never-constructed `BufferError::MaxAgeInvalid`, and
`TransformError::TransformTreeEmpty` — which 1.4.1 *did* produce, from a
same-frame lookup on a frame that held data; that lookup now succeeds with
the identity (runtime change 3). Three variants became struct variants, so
their patterns need field names: `TimestampMismatch { lhs, rhs }` (still
two `f64` seconds), `IncompatibleFrames { expected, found }` and
`SameFrameMultiplication { frame }`.

### 4. Exact equality

```rust
// 1.x: tolerant within f64::EPSILON
if tf_a == tf_b { ... }

// 2.0: == is exact IEEE 754; use approx for tolerance
use approx::abs_diff_eq;
if abs_diff_eq!(tf_a, tf_b, epsilon = 1e-9) { ... }
```

The unsound `Eq` on `Transform` and the `PartialOrd` derives on
`Quaternion`, `Vector3`, and `Point` are removed; ordering comparisons on
those types no longer compile. Tolerant comparison lives in the `approx`
traits (`AbsDiffEq`/`RelativeEq`), implemented for all geometry types.

### 5. Private internals

- **Every public type has a canonical import path, and the leaf modules are
  not it.**
  The modules the types live in — `geometry::transform`, `geometry::quaternion`,
  `geometry::vector3`, `geometry::point`, `time::timestamp`, `time::traits`,
  `core::registry`, `core::buffer` — are private in 2.0, so a 1.x deep import
  fails on the `use` line with `error[E0603]: module ... is private`. The rule,
  not an inventory: import a type from the module that re-exports it —
  `transforms::{Registry, Transform, Transformable, Localized}`,
  `transforms::geometry::{Point, Quaternion, Vector3, UNIT_NORM_TOLERANCE}`,
  `transforms::time::{Stamp, TimePoint, Timestamp}`, and `transforms::errors::*`
  for the error types. Those lists overlap in two places, deliberately: the
  crate root re-exports `Transform`, `Transformable` and `Localized`, which
  are also `transforms::geometry::*`, and `TimeError` answers to both
  `time::TimeError` and `errors::TimeError`. Either path compiles; the ones
  listed above are what the docs and examples use. Unlike the suggestion in
  break 1, **rustc's suggestion here is correct; take it**: every one of those
  imports gets a `help: consider importing this struct instead` naming a
  working path (for example `use transforms::time::Timestamp;`). It is marked
  `MaybeIncorrect`, so `cargo fix` will not apply it for you.
- `registry.data` is private. There is no public iteration API — restructure
  around `get_transform`, `remove_frame`, and your own bookkeeping.
- **`Buffer` and the `core` module are gone from the public API.** `Registry`
  is the whole entry point, re-exported at the crate root: `use
  transforms::Registry`, never `transforms::core::Registry`. Code that stored
  transforms in a `Buffer` of its own has no drop-in replacement type — give
  the frame pair to a `Registry` (one buffer per child frame is what it keeps
  internally) and use `add_transform` / `get_transform`. This is the one
  privatized path with no correct suggestion above it: `transforms::core::Buffer`
  gets a bare E0603, because no public re-export exists for rustc to point at.
- `Timestamp`'s inner field is private and holds `u64` nanoseconds instead
  of `u128`: replace `ts.t` with `ts.as_nanos()` and `Timestamp { t }` with
  `Timestamp::from_nanos(t)`, narrowing wider integers at the call site
  (`u64::try_from(nanos)`). u64 nanoseconds cover ~584 years, running out
  in 2554; a clock that must outlive that needs a custom `TimePoint`. In a
  variable-width encoding the narrower width costs nothing — `Timestamp` is
  `#[serde(transparent)]`, a bare JSON integer and a postcard LEB128
  varint, identical bytes for every value that still fits — so there only a
  stamp beyond 2554 stops decoding. A fixed-width codec does change the
  field: bincode 1.x writes eight bytes where a `u128` took sixteen, so
  re-encode such a stream rather than reinterpreting it.

### 6. Small signature changes

- `registry.delete_transforms_before(t)` →
  `registry.remove_transforms_before(t)` (rename only; the crate now spells
  every removal `remove_`, next to `remove_frame`). Its cleanup semantics did
  change — see runtime change 5.
- `Timestamp::as_seconds_unchecked()` → `Timestamp::as_seconds_lossy()`
  (rename only; same behavior).

### 7. Transforms are built, not written

```rust
// 1.x
let mut tf = Transform {
    translation: Vector3::new(1.0, 0.0, 0.0),
    rotation: Quaternion::identity(),
    timestamp: t,
    parent: "map".into(),
    child: "base".into(),
};
tf.translation = Vector3::new(2.0, 0.0, 0.0);
let x = tf.translation.x;

// 2.0
let tf: Transform = Transform::new(
    "map",
    "base",
    Vector3::new(2.0, 0.0, 0.0),
    Quaternion::identity(),
    Stamp::At(t),
)?;
let x = tf.translation().x;
```

`Transform`'s fields are private and the type is `#[non_exhaustive]`:
`Transform::new(parent, child, translation, rotation, stamp)` and
`Transform::static_between(parent, child, translation, rotation)` are the
only ways to build one from components, both return
`Result<_, TransformError>`, and both
reject non-finite components and rotations whose norm deviates from `1.0` by
more than `geometry::UNIT_NORM_TOLERANCE`. Read the components back with
`translation()`, `rotation()`, `timestamp()`, `parent()` and `child()`; to
change one, build a new transform. Deserialization runs the same validation,
so a denormalized rotation on the wire is now a decode error rather than a
transform that answers lookups with plausible nonsense.

`Transform::identity()` is removed: it produced empty frame names, which no
registry would accept and no composition would allow, and its only use was as
a base for the field assignment that no longer compiles.

`Point` keeps its public fields — it is a data record, not an invariant
carrier — but gains `#[non_exhaustive]`, so its literal becomes
`Point::new(position, orientation, timestamp, frame)`. `Vector3` and
`Quaternion` literals are untouched.

### 8. `Quaternion` is a rotation, not a vector

```rust
// 1.x
let blend = (q1.scale(0.75) + q2.scale(0.25)).normalize()?;
let step = (q2 / q1)?;
let n2 = q.norm_squared();
let q0 = Quaternion::default();

// 2.0
let blend = q1.slerp(q2, 0.25); // blending is slerp's job now
let step = q2 * q1.conjugate(); // a unit quaternion's inverse is its conjugate
let n2 = q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z;
let q0 = Quaternion::identity();
```

The public surface is the rotation algebra — `*` composes, `conjugate`
inverts a unit quaternion, `normalize`/`norm` police the unit invariant,
`rotate_vector` applies, `slerp` interpolates — and nothing else. The
vector-space operators `+`, `-` and `/`, `scale`, `norm_squared` and the
`Default` impl leave the public surface: `+`, `-`, `/` and `Default` had
no caller outside their own unit tests, `norm_squared` served only the
removed `/`, and `scale` survives privately inside `normalize` and
`slerp`. Summing rotations and renormalizing is the classic silent wrong
answer, and `/` compounded it: a NaN divisor returned `Ok` with all-NaN
components, and an overflowing one returned `Ok` with an all-zero — and
finite — quaternion, plausible enough to pass an `is_finite` check.
`QuaternionError::DivisionByZero` is gone with it. For a unit divisor,
`q2 / q1` computed `q2 * q1.conjugate()` scaled by `1/norm²` — a factor
that is 1 mathematically but drifts in floating point with the divisor's
norm, by ulps for a freshly normalized rotation and by up to about
`2e-6` for one at the edge of `UNIT_NORM_TOLERANCE` — so the conjugate
spelling is the more accurate of the two, not merely the surviving one.
The components stay public, so anything else is a one-liner.

### 9. `Vector3` sheds `dot`, `cross` and the unit constructors

```rust
// 1.x
let d = a.dot(b);
let c = a.cross(b);
let x = Vector3::unit_x();

// 2.0
let d = a.x * b.x + a.y * b.y + a.z * b.z;
let c = Vector3::new(
    a.y * b.z - a.z * b.y,
    a.z * b.x - a.x * b.z,
    a.x * b.y - a.y * b.x,
);
let x = Vector3::new(1.0, 0.0, 0.0);
```

`dot` and `cross` had no caller outside their own unit tests, the unit
constructors none at all, and the crate already points more general
needs at a linear-algebra library. The operator set stays complete —
`+`, `-`, both scalar multiplications and `/` — and the components stay
public, so the formulas above are the whole migration.

## Runtime behavior changes (compile clean, behave differently)

1. **Two kinds under one child frame are rejected.** A static sample and
   dynamic samples under the same child frame now fail at insert with
   `StaticDynamicConflict`. In 1.x the most recent insert decided how the
   whole buffer was read, measured on 1.4.1: a lookup interpolating between
   two dynamic samples started returning the `t=0` sample at every instant
   once one was inserted, then went back to interpolating after the next
   dynamic insert — one query, three different `Ok` answers. Give static
   mounts their own child frame. Chaining a static transform with dynamic
   ones is unaffected — that is what 1.1.0 enabled, it is what the examples
   demonstrate, and it is a property of the chain, not of one frame's
   buffer. (A 1.x `t=0` sample no longer triggers this either: zero is
   ordinary dynamic data now — see the `Stamp` section above.)
2. **Re-parenting is rejected.** 1.x let a new parent silently win;
   2.0 returns `ReparentingNotSupported`. Escape hatch:
   `registry.remove_frame(child)` then re-add. Removing a mid-tree frame
   strands its descendants — re-add each one.
3. **Same-frame lookup returns the identity.** `get_transform(x, x, t)`
   errored in 1.x; it now returns `Ok(identity)`. The same goes for
   `get_transform_at`'s coinciding-frame legs — `source` equal to the
   fixed frame, or all three frames equal — which failed with
   `SameFrameMultiplication` in 1.x (and still in 2.0.0-alpha.1; resolved
   since beta.1).
4. **Results always carry the requested timestamp**, including over
   all-static chains.
5. **Cleanup preserves static transforms — and frame pins.**
   `remove_transforms_before` (1.x: `delete_transforms_before`) deleted
   static transforms in 1.x; it now spares them. It also never releases a frame: a frame drained of every
   sample keeps the parent and the static-or-dynamic kind pinned by its
   first insert, so routine cleanup cannot quietly re-open it for
   re-parenting or for a change of kind, and lookups on it report
   `NotFoundAt` — with `covered: None`, not a covered range — rather than
   `UnknownFrame`. `remove_frame` is the only
   release — call it when a frame retires, or the frame map grows without
   bound. The wipe also resets each frame's `max_age` expiry reference,
   so a stream restarted at earlier times — a replay, a clock reset —
   stores again; in 1.x the wiped buffer kept its pre-wipe latest
   timestamp and silently evicted the restarted stream's first insert.
6. **No extrapolation anywhere.** An out-of-range lookup fails with
   `RegistryError::NotFoundAt` carrying the frame's covered range, and
   `Transform::interpolate` with `TransformError::TimestampOutOfRange`;
   `Quaternion::slerp` clamps its factor to [0, 1].
7. **Re-publishing at a stored timestamp replaces that sample** (documented
   last-write-wins upsert — unchanged from 1.x mechanics, now a contract).
8. **Interpolated rotations can move by a few ulps — in both feature
   modes.** 1.x had no `libm` dependency at all: `sqrt`, `sin`, and `acos`
   were `f64`'s own — the platform's math library — with and without `std`.
   2.0 uses `libm` in both modes, so a host and the target it replays now
   agree bit for bit, but any 1.x build may differ from 2.0 in the last
   bits of a slerped rotation: up to four ulps per component, measured
   against x86-64 glibc. Only comparisons against recorded 1.x output at
   full precision notice; if you have such fixtures, re-record them —
   whether or not you enabled `std`.

9. **Lookups toward an ancestor return stored data unchanged.** Each half of
   a resolved chain is now composed in its natural direction, so
   `get_transform("map", "lidar", t)` — the documented direction — inverts
   nothing instead of once per hop plus once at the end. A single-hop lookup
   at a stored timestamp returns that stored transform bit for bit; before,
   it came back through two inversions, whose rounding moved the translation
   by up to ten ulps per component (measured on x86-64). Rotations in that
   direction are no longer renormalized on the way out either, another few
   ulps. Only comparisons against recorded output at full precision notice.

## Renamed, but not breaking

`get_transform`'s parameters are now named `target, source` (was
`from, to`) — call sites are positional, so nothing breaks, but note the
direction convention it clarifies: `get_transform("map", "lidar", t)`
returns the transform that expresses **lidar data in the map frame**.
Swapping the arguments silently yields the exact inverse.

## What does not break

Struct literals and public fields of `Vector3` and `Quaternion`, and
`Point`'s public fields (its literal becomes `Point::new` — see break 7);
`Timestamp::zero()`/`now()` and timestamp arithmetic; the call shape of
`get_transform` / `get_transform_for` / `get_transform_at` — same receiver,
same arguments in the same order, timestamps still bare `T`, so call sites
compile untouched even though all three now report `RegistryError<T>`
instead of `TransformError` (break 3); the `Localized` and `Transformable`
traits; the `no_std` `Registry::new()` path.

## After migrating, re-test — don't just re-compile

The silent-wrong-answer failure modes of 1.x are gone, which means data
that previously "worked" by accident (interpolating across a static
sample, re-parented frames, denormalized rotations) now fails loudly at
insert or lookup. That is the point of 2.0.
