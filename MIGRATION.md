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
accumulate until you call `delete_transforms_before`. If you had a
`max_age`, you want `with_max_age`. (`Buffer::new(max_age)` splits the same
way.) In `no_std`, both constructors now exist too — automatic cleanup no
longer requires `std`.

### 2. Fallible insertion

```rust
// 1.x
registry.add_transform(transform);

// 2.0
registry.add_transform(transform)?;
```

`add_transform` and `Buffer::insert` return `Result` and validate on
insertion. An ignored `Err` means **nothing was stored** — later lookups
will fail mysteriously. New rejections your 1.x data may already trigger:
non-finite values, non-unit rotations (beyond `UNIT_NORM_TOLERANCE`),
self-referential frames, re-parenting (`ReparentingNotSupported` — call
`remove_frame` first), cycles (`CycleDetected`), and mixing static with
dynamic transforms in one child frame (`StaticDynamicConflict`).

### Static transforms are no longer `t = 0`

The static sentinel moved from `t = 0` to `Timestamp::STATIC`
(`u128::MAX` nanoseconds), so every real instant — including zero, the
first reading of a boot-relative clock — is ordinary dynamic data.

```rust
// 1.x: static mount published at t = 0
let mount = Transform { timestamp: Timestamp::zero(), /* ... */ };

// 2.0
let mount: Transform = Transform::static_between(
    "base", "camera",
    Vector3::new(0.1, 0.0, 0.5),
    Quaternion::identity(),
);
```

A 1.x static transform inserted with `t = 0` still compiles — but it is
now a **single dynamic sample at the epoch**, so any lookup at a real
time fails loudly with `TimestampOutOfRange` instead of serving the
mount. Switch static publishers to `Transform::static_between` (or
`Timestamp::STATIC`).

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
    TransformError::NotFoundAt { frame, source, .. } => retry_later(), // data gap at `frame`
    _ => other(),                                              // mandatory: #[non_exhaustive]
}
```

The 1.x catch-all `TransformError::NotFound` is gone, replaced by the three
diagnosed variants above (mirroring tf2's LookupException /
ConnectivityException / ExtrapolationException). `NotFoundAt`'s `source`
carries `TimestampOutOfRange(requested, start, end)` — `requested > end`
means the lookup is merely too new (latency: retry), otherwise the data is
stale or missing. All error enums are `#[non_exhaustive]`, so every match
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
- `Timestamp`'s inner field is private: replace `ts.t` with `ts.as_nanos()`
  and `Timestamp { t }` with `Timestamp::from_nanos(t)`. The serde wire
  format is unchanged.

### 6. Small signature changes

- `Buffer::get(&ts)` → `Buffer::get(ts)` (timestamp by value).
- `Timestamp::as_seconds_unchecked()` → `Timestamp::as_seconds_lossy()`
  (rename only; same behavior).

## Runtime behavior changes (compile clean, behave differently)

1. **Static + dynamic mixing is rejected.** A static sample and dynamic
   samples in the same child frame — the pattern 1.1.0 explicitly
   enabled — now fails at insert with `StaticDynamicConflict`. Give
   static mounts their own child frames. (A 1.x `t=0` sample no longer
   triggers this: zero is ordinary dynamic data now — see the sentinel
   section above.)
2. **Re-parenting is rejected.** 1.x let a new parent silently win;
   2.0 returns `ReparentingNotSupported`. Escape hatch:
   `registry.remove_frame(child)` then re-add. Removing a mid-tree frame
   strands its descendants — re-add each one.
3. **Same-frame lookup returns the identity.** `get_transform(x, x, t)`
   errored in 1.x; it now returns `Ok(identity)`.
4. **Results always carry the requested timestamp**, including over
   all-static chains.
5. **Cleanup preserves static transforms.** `delete_transforms_before`
   deleted them in 1.x; it now spares them and prunes frames left empty.
6. **No extrapolation anywhere.** Out-of-range queries fail with
   `TimestampOutOfRange`; `Quaternion::slerp` clamps its factor to [0, 1].
7. **Re-publishing at a stored timestamp replaces that sample** (documented
   last-write-wins upsert — unchanged from 1.x mechanics, now a contract).

## Renamed, but not breaking

`get_transform`'s parameters are now named `target, source` (was
`from, to`) — call sites are positional, so nothing breaks, but note the
direction convention it clarifies: `get_transform("map", "lidar", t)`
returns the transform that expresses **lidar data in the map frame**.
Swapping the arguments silently yields the exact inverse.

## What does not break

Struct literals and public fields of `Transform`, `Point`, `Vector3`, and
`Quaternion`; `Timestamp::zero()`/`now()` and timestamp arithmetic;
`get_transform` / `get_transform_for` / `get_transform_at` signatures (all
`&self`); the `Localized` and `Transformable` traits; the `no_std`
`Registry::new()` path.

## After migrating, re-test — don't just re-compile

The silent-wrong-answer failure modes of 1.x are gone, which means data
that previously "worked" by accident (interpolating across a static
sample, re-parented frames, denormalized rotations) now fails loudly at
insert or lookup. That is the point of 2.0.
