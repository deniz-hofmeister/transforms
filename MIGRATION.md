# Migrating from 1.x to 2.0

Every break below was reproduced by compiling the 1.4.1-documented usage
against 2.0. Work through the compile errors first; then read the runtime
changes — code that compiles cleanly can still behave differently.

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

`add_transform` returns `Result`. An ignored `Err` means **nothing was
stored** — later lookups will fail mysteriously. New rejections your 1.x
data may already trigger: self-referential frames, re-parenting
(`ReparentingNotSupported` — call `remove_frame` first), cycles
(`CycleDetected`), and mixing static with dynamic transforms in one child
frame (`StaticDynamicConflict`). Non-finite values and non-unit rotations
are rejected earlier, by the constructor — see break 7.

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
fails loudly with `TimestampOutOfRange` instead of serving the mount.
Switch static publishers to `Transform::static_between` (or
`Stamp::Static`).

Two related breaks ride along:

- **`TimePoint` is pure time arithmetic now, in three methods.** Custom
  clock impls delete `static_timestamp()` and `is_static()` — no sentinel
  value to invent — and also `checked_add()` and `as_seconds()`, neither
  of which the crate ever called. What remains is `duration_since`,
  `checked_sub`, and `as_seconds_lossy`, the last no longer defaulted:
  implement it as a best-effort conversion that yields `f64::NAN` where
  your clock cannot convert, never a plausible-looking number, because it
  is what formats error messages. The trait also gains `Debug` as a
  supertrait — derive it on your clock type if you had not.
- **A child frame is static or dynamic for its lifetime.** The kind is
  fixed by the frame's first insert and survives the frame being drained,
  which also fixes a 1.x bug where an emptied frame silently flipped
  static on the next insert (see runtime change 5).

**Serde wire format:** `Stamp` serializes as an optional timestamp. In
JSON a dynamic transform's shape is unchanged
(`"timestamp": { "t": ... }`), a static one is `"timestamp": null`, and a
*missing* `timestamp` field is a hard error — it never silently becomes
static. In postcard/bincode the stamp gains a 1-byte `Option` tag.

Cross-version decoding is format-dependent, so version-tag your streams:

- An old **JSON** payload that encoded staticness as `t = 0` decodes as a
  dynamic sample at the epoch — real-time lookups then fail loudly with
  `TimestampOutOfRange` rather than silently serving stale calibration.
- An old **postcard** stream with a realistic dynamic timestamp fails to
  decode outright (the timestamp varint is not a valid `Option` tag). The
  one exception is a payload stamped exactly `t = 0` — the 1.x static
  convention — whose single `0x00` byte reads as the `None` tag, keeping
  the stream byte-aligned: it decodes cleanly as `Stamp::Static`. That is
  the right meaning for a 1.x static publisher, but wrong for a genuine
  boot-relative `t = 0` dynamic sample — do not rely on it in place of a
  version tag.

### 3. Error enum overhaul

```rust
// 1.x
match err {
    TransformError::NotFound(from, to) => retry(),
    ...
}

// 2.0
match err {
    TransformError::UnknownFrame(f) => wait_for_publisher(),   // typo / not yet published
    TransformError::Disconnected { target_frame, source_frame } => topology_bug(),
    TransformError::NotFoundAt { frame, source, .. } => inspect(source), // see below: `frame` cannot answer
    _ => other(),                                              // mandatory: #[non_exhaustive]
}
```

The 1.x catch-all `TransformError::NotFound` is gone, replaced by the three
diagnosed variants above (mirroring tf2's LookupException /
ConnectivityException / ExtrapolationException). `NotFoundAt`'s `source`
must be inspected before retrying, because it distinguishes two situations:
`TimestampOutOfRange { requested, start, end }` means `frame` holds data the
request falls outside of — `requested > end` is merely too new (latency:
retry), otherwise the data is stale — while `NoTransformAvailable` means
`frame` holds no data at all and carries no range, so retrying achieves
nothing until someone inserts into it (see runtime behavior change 5).
All error enums are `#[non_exhaustive]`, so every match
needs a `_` arm. Also removed: the `TimestampError` alias (use
`TimeError`), `BufferError::MaxAgeInvalid`, and
`TransformError::TransformTreeEmpty` (never produced).
`IncompatibleFrames` and `SameFrameMultiplication` are now struct variants
carrying the offending frame names.

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

- `registry.data` is private. There is no public iteration API — restructure
  around `get_transform`, `remove_frame`, and your own bookkeeping.
- **`Buffer` and the `core` module are gone from the public API.** `Registry`
  is the whole entry point, re-exported at the crate root: `use
  transforms::Registry`, never `transforms::core::Registry`. Code that stored
  transforms in a `Buffer` of its own has no drop-in replacement type — give
  the frame pair to a `Registry` (one buffer per child frame is what it keeps
  internally) and use `add_transform` / `get_transform`. Both imports fail with
  E0603 (`module core is private`) on the `use` line, but rustc helps unevenly:
  for `transforms::core::Registry` it adds `help: consider importing this
  struct instead` with the replacement `use transforms::Registry;` — unlike the
  suggestion in break 1, **that one is correct; take it**. For
  `transforms::core::Buffer` there is no suggestion, because no public
  re-export exists for rustc to point at.
- `Timestamp`'s inner field is private and holds `u64` nanoseconds instead
  of `u128`: replace `ts.t` with `ts.as_nanos()` and `Timestamp { t }` with
  `Timestamp::from_nanos(t)`, narrowing wider integers at the call site
  (`u64::try_from(nanos)`). u64 nanoseconds cover ~584 years, running out
  in 2554; a clock that must outlive that needs a custom `TimePoint`. The
  serde wire shape is unchanged for every value that still fits — a JSON
  integer, a postcard LEB128 varint — so only a stamp beyond 2554 stops
  decoding.

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
only ways to build one, both return `Result<_, TransformError>`, and both
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

## Runtime behavior changes (compile clean, behave differently)

1. **Static + dynamic mixing is rejected.** A static sample and dynamic
   samples in the same child frame — the pattern 1.1.0 explicitly
   enabled — now fails at insert with `StaticDynamicConflict`. Give
   static mounts their own child frames. (A 1.x `t=0` sample no longer
   triggers this: zero is ordinary dynamic data now — see the `Stamp`
   section above.)
2. **Re-parenting is rejected.** 1.x let a new parent silently win;
   2.0 returns `ReparentingNotSupported`. Escape hatch:
   `registry.remove_frame(child)` then re-add. Removing a mid-tree frame
   strands its descendants — re-add each one.
3. **Same-frame lookup returns the identity.** `get_transform(x, x, t)`
   errored in 1.x; it now returns `Ok(identity)`.
4. **Results always carry the requested timestamp**, including over
   all-static chains.
5. **Cleanup preserves static transforms — and frame pins.**
   `remove_transforms_before` (1.x: `delete_transforms_before`) deleted
   static transforms in 1.x; it now spares them. It also never releases a frame: a frame drained of every
   sample keeps the parent and the static-or-dynamic kind pinned by its
   first insert, so routine cleanup cannot quietly re-open it for
   re-parenting or for a change of kind, and lookups on it report
   `NotFoundAt` — with `NoTransformAvailable` as the cause, not a covered
   range — rather than `UnknownFrame`. `remove_frame` is the only
   release — call it when a frame retires, or the frame map grows without
   bound.
6. **No extrapolation anywhere.** Out-of-range queries fail with
   `TimestampOutOfRange`; `Quaternion::slerp` clamps its factor to [0, 1].
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
`Timestamp::zero()`/`now()` and timestamp arithmetic;
`get_transform` / `get_transform_for` / `get_transform_at` signatures (all
`&self`, all taking bare timestamps); the `Localized` and `Transformable`
traits; the `no_std` `Registry::new()` path.

## After migrating, re-test — don't just re-compile

The silent-wrong-answer failure modes of 1.x are gone, which means data
that previously "worked" by accident (interpolating across a static
sample, re-parented frames, denormalized rotations) now fails loudly at
insert or lookup. That is the point of 2.0.
