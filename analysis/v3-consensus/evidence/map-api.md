# transforms 2.4.0 — public API inventory (as shipped at adccbdd)

Method: read `src/` non-test files (signatures, derives, impls, error enums) plus grep for
callers in `src/` (excluding `tests.rs`), `examples/`, `benches/`, `README.md`.
"Internal callers" = non-test library code. Not done: `cargo public-api`/rustdoc JSON
cross-check, so auto-trait/derive surface is from reading derives, not tool-verified.
Paths are relative to `the repository root`.

## 0. Crate-level

- Features (`Cargo.toml`): `default = ["std"]`, `std = ["serde?/std"]`, `serde = ["dep:serde"]`.
- Feature-gated items: `Timestamp::now` / `try_now` (std, `src/time/timestamp/mod.rs:62-95`),
  `impl TimePoint for SystemTime` (std, `src/time/system_time.rs:13`), serde derives on
  `Vector3`, `Quaternion`, `Point`, `Transform`, `Stamp`, `Timestamp` (serde).
- Runtime deps: thiserror, approx, hashbrown, libm, serde (optional). `approx` traits are part
  of the public API (AbsDiffEq/RelativeEq impls) — an approx major bump is a breaking change.
- Public modules (`src/lib.rs:116-118`): `errors`, `geometry`, `time`. `core` is private.
- Root re-exports (`src/lib.rs:119-120`): `Registry`, `Localized`, `Transform`, `Transformable`.

### Path map (where each item is nameable)

| Item | Paths |
|---|---|
| `Registry` | `transforms::Registry` only |
| `RegistryError` | `transforms::errors::RegistryError` only |
| `Transform`, `Localized`, `Transformable` | root and `geometry::` |
| `Point`, `Quaternion`, `Vector3`, `UNIT_NORM_TOLERANCE` | `geometry::` only |
| `TransformError`, `QuaternionError` | `errors::` only (not re-exported from `geometry`) |
| `TimeError` | `time::TimeError` and `errors::TimeError` |
| `Timestamp`, `Stamp`, `TimePoint` | `time::` only |

## 1. `transforms` root / `core` (private) — Registry

`pub struct Registry<T = Timestamp> where T: TimePoint` (`src/core/registry/mod.rs:94-102`).
Fields private: `data: HashMap<String, Buffer<T>>`, `max_age: Option<Duration>`.
Derives: `Debug` only (no `Clone`). Manual `impl Default` (`:1352`, calls `new`).

| Method | Signature | Purpose / internal callers |
|---|---|---|
| `new` (`:118`) | `fn new() -> Self` `#[must_use]` | No expiry. Called by `Default`. |
| `with_max_age` (`:147`) | `fn with_max_age(max_age: Duration) -> Self` | Global data-driven expiry for dynamic buffers. No internal callers. |
| `add_transform` (`:171`) | `fn add_transform(&mut self, t: Transform<T>) -> Result<(), RegistryError<T>>` | Insert one sample (by value). Delegates to `process_add_transform`. |
| `add_transforms` (`:231`) | `fn add_transforms(&mut self, transforms: impl IntoIterator<Item = Transform<T>>) -> Result<(), RegistryError<T>>` | All-or-nothing batch. No internal callers. |
| `get_transform` (`:334`) | `fn get_transform(&self, target: &str, source: &str, timestamp: T) -> Result<Transform<T>, RegistryError<T>>` | Lookup; result `parent = target`, `child = source`, stamped `At(timestamp)`. Called by `get_transform_for`. |
| `get_transform_for` (`:363`) | `fn get_transform_for<U: Localized<T>>(&self, value: &U, target_frame: &str) -> Result<Transform<T>, RegistryError<T>>` | One-line sugar over `get_transform(target_frame, value.frame(), value.timestamp())`. Sole consumer of `Localized`. |
| `get_transform_at` (`:439`) | `fn get_transform_at(&self, target_frame: &str, target_time: T, source_frame: &str, source_time: T, fixed_frame: &str) -> Result<Transform<T>, RegistryError<T>>` | Cross-time lookup via a fixed frame. |
| `latest_common_time` (`:575`) | `fn latest_common_time(&self, target: &str, source: &str) -> Result<Stamp<T>, RegistryError<T>>` | Newest instant every hop serves; `Stamp::Static` for all-static chain and for `target == source`. |
| `remove_transforms_before` (`:663`) | `fn remove_transforms_before(&mut self, timestamp: T)` | Total; drains dynamic buffers, keeps pins. |
| `remove_frame` (`:720`) | `fn remove_frame(&mut self, child: &str) -> bool` | Drops a child frame's buffer; `bool` = existed. |
| `reparent_frame` (`:827`) | `fn reparent_frame(&mut self, t: Transform<T>) -> Result<(), RegistryError<T>>` | Deliberate re-parent with seed transform; drops history. |

Private helpers (not API): `Hop<'a>` + `Mul`, `process_add_transform`, `empty_buffer_for`,
`creates_cycle`, `ancestry`, `connecting_chain`, `frame_exists`, `diagnose_not_found`, `resolve`,
`process_get_transform_at`, `get_transform_chain`, `truncate_at_common_parent`,
`combine_transforms`, `compose_chain` (`:878-1342`). Crate-private `Buffer<T>`
(`src/core/buffer/mod.rs:50`), `Coverage<T>` (`:95`), `InsertError`, `GetError<T>`.

## 2. `transforms::errors`

Pure re-export module (`src/errors/mod.rs:10`): `RegistryError`, `QuaternionError`,
`TransformError`, `TimeError`. All four: `#[derive(Error, Debug)]`, `#[non_exhaustive]`.
None derive `Clone`, `PartialEq`, or `Eq`.

### `RegistryError<T = Timestamp> where T: TimePoint` (`src/core/registry/error.rs:24`) — 14 variants

| Variant (line) | Payload | Produced by |
|---|---|---|
| `NonUnitRotation` (34) | `f64` norm (tuple) | insert validation, via `From<TransformError>` |
| `NonFiniteValues` (46) | — | insert validation; lookup when an inversion overflows |
| `EmptyFrameName` (53) | — | insert |
| `SelfReferentialFrame` (57) | — | insert |
| `ReparentingNotSupported` (80) | `{ current_parent: String }` | `add_transform` on a pinned child. Its own doc says the name is stale and a rename is deferred to 3.0. |
| `CycleDetected` (87) | — (no frames named) | insert, reparent |
| `StaticDynamicConflict` (106) | — (no frame named) | insert, reparent |
| `NoParentToReplace` (129) | `String` (tuple) | `reparent_frame` on a root |
| `ParentUnchanged` (139) | `String` (tuple) | `reparent_frame` |
| `UnknownFrame` (145) | `String` (tuple) | lookups, `latest_common_time`, `reparent_frame` |
| `Disconnected` (152) | `{ target_frame: String, source_frame: String }` | lookups, `latest_common_time` |
| `NotFoundAt` (177) | `{ target_frame, source_frame, frame: String, requested: T, covered: Option<(T, T)> }` | lookups |
| `NoCommonTime` (204) | `{ target_frame, source_frame, frame: String, covered: Option<(T, T)> }` | `latest_common_time` |
| `TransformError` (223) | `#[source] TransformError` (tuple) | residual geometry/time failures |

Impls: `From<TransformError>` (`:227`, canonicalizes `NonUnitRotation`/`NonFiniteValues` to flat
variants); `From<InsertError>` (`:244`) — public impl on a crate-private type, unusable outside.
Display converts times with `as_seconds_lossy` via private `Coverage` wrapper (`:271`).

Which `TransformError`s can sit inside `RegistryError::TransformError`: from reading the
producers, `TimestampError(TimeError)` (interpolation `duration_since`,
`src/geometry/transform/isometry.rs:82,87`), `QuaternionError` (inverse normalize, `:48`), and
`SameFrameMultiplication`/`IncompatibleFrames` from `Hop::mul`→`ensure_composable`
(`src/core/registry/mod.rs:61-75`; looks defensive/unreachable on a resolved chain — unverified).
`TimestampMismatch`, `StaticInterpolation`, `TimestampOutOfRange` have no registry producer.

### `TransformError` (`src/geometry/transform/error.rs:16`) — 9 variants

| Variant (line) | Payload | Producers (non-test) |
|---|---|---|
| `NonUnitRotation` (18) | `f64` | `Transform::validate` |
| `NonFiniteValues` (22) | — | `validate`, `Isometry::inverse` |
| `TimestampMismatch` (28) | `{ lhs: f64, rhs: f64 }` seconds, lossy | `Mul`, `interpolate` (swapped endpoints), `Point::transform` |
| `StaticInterpolation` (38) | — | `Transform::interpolate` only |
| `TimestampOutOfRange` (45) | `{ requested, start, end: f64 }` seconds, lossy | `Transform::interpolate` only |
| `SameFrameMultiplication` (56) | `{ frame: String }` | `ensure_composable` |
| `IncompatibleFrames` (66) | `{ expected: String, found: String }` | `ensure_composable`, `interpolate` (payload is a formatted `"a -> b"` pair there, `mod.rs:400-403`), `Point::transform` |
| `TimestampError` (75) | `#[from] TimeError` | interpolation arithmetic |
| `QuaternionError` (79) | `#[from] QuaternionError` | `Isometry::inverse` |

### `QuaternionError` (`src/geometry/quaternion/error.rs:6`) — 2 variants
`ZeroLengthNormalization`, `NonFinite`. Only producer: `Quaternion::normalize`.

### `TimeError` (`src/time/error.rs:6`) — 3 variants
`DurationUnderflow`, `DurationOverflow`, `AccuracyLoss`. `AccuracyLoss` is produced only by
`Timestamp::as_seconds`, which has no internal caller.

## 3. `transforms::geometry`

### `const UNIT_NORM_TOLERANCE: f64 = 1e-6` (`src/geometry/transform/mod.rs:30`)
Used once, in `Transform::validate` (`:320`).

### `struct Transform<T = Timestamp> where T: TimePoint` (`:84`)
Private fields `translation: Vector3, rotation: Quaternion, timestamp: Stamp<T>, parent: String,
child: String`. `#[non_exhaustive]`. Derives `Debug, Clone, PartialEq` (exact). Serde:
`Serialize` derived; `Deserialize` via `try_from = "TransformRepr<T>"` (private repr, validates,
`:560-596`).

| Item | Signature | Purpose / internal callers |
|---|---|---|
| `new` (`:145`) | `fn new(parent: &str, child: &str, translation: Vector3, rotation: Quaternion, timestamp: Stamp<T>) -> Result<Self, TransformError>` | Validating constructor; allocates two `String`s. Called by `static_between`. |
| `static_between` (`:194`) | `fn static_between(parent: &str, child: &str, translation: Vector3, rotation: Quaternion) -> Result<Self, TransformError>` | `new(.., Stamp::Static)`. No internal callers. |
| `translation` (`:228`) | `fn translation(&self) -> Vector3` | Getter. |
| `rotation` (`:235`) | `fn rotation(&self) -> Quaternion` | Getter. |
| `timestamp` (`:242`) | `fn timestamp(&self) -> Stamp<T>` | Getter (returns a `Stamp`, not a `T`). |
| `parent` (`:249`) / `child` (`:255`) | `fn (&self) -> &str` | Getters. |
| `validate` (`:304`) | `fn validate(&self) -> Result<(), TransformError>` | Finite + unit-norm check. Called by `new`, deserialization, `Buffer::check` (`src/core/buffer/mod.rs:322`). |
| `interpolate` (`:379`) | `fn interpolate(from: &Transform<T>, to: &Transform<T>, timestamp: T) -> Result<Transform<T>, TransformError>` | Associated fn (no `self`). No internal callers — the buffer interpolates on `Isometry`. |
| `inverse` (`:465`) | `fn inverse(&self) -> Result<Self, TransformError>` | Clones both names. No internal callers (registry inverts `Hop`/`Isometry`). |
| `impl Mul for Transform<T>` (`:507`) | `Output = Result<Transform<T>, TransformError>`; by value only | Composition with frame + stamp checks. No internal callers (registry composes `Hop`). |
| `AbsDiffEq`/`RelativeEq` (`:598`, `:621`) | `Epsilon = f64`, defaults `f64::EPSILON`; stamp and frames compared exactly | Test support. |

Crate-private: `Transform::unvalidated`, `Transform::isometry`, `ensure_composable`, `Isometry`.

### `trait Localized<T = Timestamp> where T: TimePoint` (`src/geometry/transform/traits.rs:34`)
`fn frame(&self) -> &str; fn timestamp(&self) -> T;` — consumed only by
`Registry::get_transform_for`; implemented only by `Point`.

### `trait Transformable<T = Timestamp> where T: TimePoint` (`traits.rs:162`)
`fn transform(&mut self, transform: &Transform<T>) -> Result<(), TransformError>;` — in-place.
No internal consumer; implemented only by `Point`.

### `struct Point<T = Timestamp> where T: TimePoint` (`src/geometry/point/mod.rs:40`)
Public fields `position: Vector3, orientation: Quaternion, timestamp: T, frame: String`;
`#[non_exhaustive]`; derives `Debug, Clone, PartialEq`, serde derives (unvalidated on decode).
- `fn new(position: Vector3, orientation: Quaternion, timestamp: T, frame: &str) -> Self` (`:78`), total, unvalidated.
- `impl Transformable<T>` (`:93`): errors `IncompatibleFrames`, `TimestampMismatch`.
- `impl Localized<T>` (`:140`), `AbsDiffEq`/`RelativeEq` (`:153`, `:177`).
- Documented as the "reference implementation"; no library code uses it.

### `struct Quaternion` (`src/geometry/quaternion/mod.rs:24`)
Public fields `w, x, y, z: f64`. Derives `Debug, Clone, Copy, PartialEq`, serde. No `Default`.

| Item | Signature | Internal callers |
|---|---|---|
| `from_wxyz` (`:50`) | `const fn from_wxyz(w, x, y, z: f64) -> Self` | — (does not normalize) |
| `identity` (`:76`) | `const fn identity() -> Self` | `Isometry::identity` |
| `conjugate` (`:97`) | `fn conjugate(self) -> Quaternion` | `Isometry::inverse`, `rotate_vector` |
| `normalize` (`:149`) | `fn normalize(self) -> Result<Quaternion, QuaternionError>` | `Isometry::inverse` only |
| `norm` (`:172`) | `fn norm(self) -> f64` | `validate`, `normalize`, `slerp` |
| `rotate_vector` (`:237`) | `fn rotate_vector(self, v: Vector3) -> Vector3` | `Isometry`, `Point::transform` |
| `slerp` (`:284`) | `fn slerp(self, other: Quaternion, t: f64) -> Quaternion` | `Isometry::interpolate` only |
| `impl Mul` (`:321`) | `Output = Quaternion` (Hamilton product) | everywhere |
| `AbsDiffEq`/`RelativeEq` (`:338`, `:357`) | component-wise (q and −q differ) | tests |

### `struct Vector3` (`src/geometry/vector3/mod.rs:26`)
Public fields `x, y, z: f64`. Derives `Debug, Clone, Copy, PartialEq`, serde. No `Default`, no `Neg`.
- `const fn new(x, y, z) -> Self` (`:48`); `const fn zero() -> Self` (`:58`, one internal caller).
- Operators: `Add` (`:63`), `Sub` (`:79`), `Mul<f64>` (`:95`), `Mul<Vector3> for f64` (`:111`),
  `Div<f64>` (`:127`, IEEE semantics, no internal caller), `AbsDiffEq`/`RelativeEq` (`:146`, `:164`).

## 4. `transforms::time`

### `trait TimePoint: Copy + Ord + core::fmt::Debug` (`src/time/traits.rs:97`)
- `fn duration_since(self, earlier: Self) -> Result<Duration, TimeError>` — interpolation ratio (`isometry.rs:82,87`), `Buffer::check_interpolation_time` (`buffer/mod.rs:405`).
- `fn checked_sub(self, rhs: Duration) -> Result<Self, TimeError>` — one caller, `max_age` expiry (`buffer/mod.rs:483`).
- `fn as_seconds_lossy(self) -> f64` — error payloads/Display only.
- Impls: `Timestamp` (`src/time/timestamp/mod.rs:293`), `std::time::SystemTime` (std, `system_time.rs:13`).

### `struct Timestamp` (`src/time/timestamp/mod.rs:37`)
Private `t: u64` nanoseconds. Derives `Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord`;
serde `transparent`. No `Hash`, `Default`, or `Display`.

| Item | Signature | Internal callers |
|---|---|---|
| `now` (`:69`, std) | `fn now() -> Self` — panics (documented) | — |
| `try_now` (`:95`, std) | `fn try_now() -> Result<Self, TimeError>` | `now` |
| `zero` (`:121`) | `const fn zero() -> Self` | — |
| `from_nanos` (`:168`) | `const fn from_nanos(nanos: u64) -> Self` | — |
| `as_nanos` (`:183`) | `const fn as_nanos(&self) -> u64` | — |
| `as_seconds` (`:213`) | `fn as_seconds(&self) -> Result<f64, TimeError>` (fails above 2^53 ns) | none |
| `as_seconds_lossy` (`:243`) | `fn as_seconds_lossy(&self) -> f64` | the `TimePoint` impl |
| `Sub<Timestamp>` (`:249`) | `Output = Result<Duration, TimeError>` | `duration_since` |
| `Add<Duration>` (`:263`) | `Output = Result<Timestamp, TimeError>` | none |
| `Sub<Duration>` (`:278`) | `Output = Result<Timestamp, TimeError>` | `checked_sub` |

### `enum Stamp<T = Timestamp> where T: TimePoint` (`src/time/stamp.rs:44`)
Variants `Static`, `At(T)`. Derives `Debug, Clone, Copy, PartialEq, Eq` (deliberately no `Ord`);
serde externally tagged. `const fn is_static(&self) -> bool` (`:60`), `const fn at(self) -> Option<T>`
(`:66`) — neither has a non-test internal caller.

## 5. Count of public named items

| Kind | Count |
|---|---|
| Public modules | 3 |
| Types (6 structs, 5 enums) | 11 |
| Traits | 3 |
| Constants | 1 |
| Inherent methods / associated fns (Registry 11, Transform 10, Quaternion 7, Timestamp 7, Vector3 2, Stamp 2, Point 1) | 40 |
| Trait methods (TimePoint 3, Localized 2, Transformable 1) | 6 |
| Enum variants (RegistryError 14, TransformError 9, TimeError 3, QuaternionError 2, Stamp 2) | 30 |
| Public fields (Quaternion 4, Vector3 3, Point 4) | 11 |
| **Total named items** | **105** |
| Hand-written trait impls (operators 10, approx 8, TimePoint 2, Localized/Transformable 2, Default 1, From 4) | 27 |
| Duplicate paths (root re-exports 3 beyond `Registry`, `errors::TimeError`) | 4 |

## 6. API smells under the v3 goals

### Vocabulary
1. Three frame vocabularies for one relation. `Transform::new(parent, child, ..)`;
   `get_transform(target, source, ..)` returns `parent = target, child = source`;
   `Point`/`Localized` use `frame`; `remove_frame(child)`. Error payloads use
   `target_frame`/`source_frame` (suffix forced by `Error::source`, `error.rs:156-158`) while the
   method parameters are `target`/`source` in two methods and `target_frame`/`source_frame` in two others.
2. `timestamp` means two types: `Transform::timestamp() -> Stamp<T>` vs
   `Localized::timestamp() -> T` vs parameter `timestamp: T`. The field holding a `Stamp` is named `timestamp`.
3. `Point` is a pose (position + orientation), not a point.
4. `ReparentingNotSupported` is false since `reparent_frame` exists (self-admitted, `error.rs:75-79`).
   `RegistryError::TransformError` and `TransformError::QuaternionError`/`TimestampError` are variants
   named after their payload type; `TimestampError` wraps `TimeError` (name mismatch).
5. `add_transform(t)` / `reparent_frame(t)` name the transform `t`, the letter used for time elsewhere (`slerp(.., t)`).
6. `Buffer::static_edge`, docs say "edge", "hop", "frame", "buffer" for the same thing (internal, but leaks into error docs).

### Argument orders that invite swaps
7. `get_transform(target: &str, source: &str, T)` — two adjacent `&str`; a swap yields the inverse
   transform, a valid-looking answer. Same in `latest_common_time` (there symmetric, harmless).
8. `get_transform_at(&str, T, &str, T, &str)` — three `&str` and two `T`, all positional.
9. `get_transform_for(value, target_frame)` puts target last; `get_transform` puts it first.
10. `Transform::new(parent, child, translation, rotation, stamp)` is frames-first; `Point::new(position,
    orientation, timestamp, frame)` is frame-last.
11. `Quaternion::from_wxyz(w, x, y, z)` — four `f64`; the name mitigates the xyzw/wxyz swap but nothing checks.
12. `Transform::interpolate(from, to, timestamp)` is an associated fn, while `inverse` is a method and `slerp(self, other, t)` is a method.

### Duplicate ways to do one thing
13. `Transform::static_between(..)` == `Transform::new(.., Stamp::Static)`.
14. `Registry::new()` / `Default` / `with_max_age` (three constructors; `max_age` is registry-global only).
15. `Timestamp` arithmetic twice: `a - b` and `duration_since`; `a - d` and `checked_sub`.
    Inherent `Timestamp::as_seconds_lossy(&self)` and trait `TimePoint::as_seconds_lossy(self)`.
16. `as_seconds` (fallible) vs `as_seconds_lossy` (total) for one conversion.
17. `get_transform_for` is `get_transform` with arguments read from a trait.
18. `Stamp::is_static()` duplicates `matches!`/`== Stamp::Static`; `Stamp::at()` duplicates a `match`.
19. `TimeError` nameable at `time::` and `errors::`; `Transform`/traits at root and `geometry::`,
    while `Registry` is root-only and `RegistryError` is `errors::`-only — no consistent rule.
20. Two frame-mismatch errors: `SameFrameMultiplication` is a special case of `IncompatibleFrames`.

### Zero internal callers (external use unverified beyond examples/README/benches)
21. `Transform::interpolate`, `Transform::inverse`, `impl Mul for Transform` — the registry never
    uses the public arithmetic; it runs on `Isometry`/`Hop`. Two parallel code paths with separate
    frame/stamp checks. `TransformError::TimestampOutOfRange`, `StaticInterpolation` exist only for `interpolate`.
22. `Transformable` (no consumer anywhere in the library), `Point` (reference impl only),
    `Localized` (one consumer: `get_transform_for`).
23. `Timestamp::as_seconds` + `TimeError::AccuracyLoss`; `Add<Duration> for Timestamp`;
    `Div<f64> for Vector3`; `Stamp::is_static`, `Stamp::at`; `Vector3::zero` (one caller).
24. `Quaternion::normalize`, `slerp`, `conjugate`, `rotate_vector`, `norm` are public only because
    `Quaternion` is a public math type; each has exactly one or two internal callers.

### Fallible things that could be total
25. `Transform * Transform -> Result` and `Timestamp ± x -> Result`: operators returning `Result`
    force `(a * b)?` and cannot chain. Frame mismatch is unavoidable while frames are runtime strings.
26. `Transform::inverse -> Result`: fails only on zero/non-finite rotation or translation overflow —
    states a validated transform cannot be in, except for derived (unvalidated) ones.
27. `Timestamp::as_seconds` fails on precision loss that `as_seconds_lossy` accepts.
28. `TimePoint::duration_since`/`checked_sub` return `Result<_, TimeError>`; the registry maps
    failures into `RegistryError::TransformError(TimestampError(..))`, three levels deep.

### Total things that hide failure
29. `Quaternion::slerp`: silently clamps `t` to [0, 1]; returns an unnormalized blend when its norm
    is below `f64::EPSILON` (`quaternion/mod.rs:303-308`); no unit-input check.
30. `Quaternion::rotate_vector` and `Mul` assume unit input, unchecked; `from_wxyz` accepts anything.
31. `Point::new` and `Point`'s serde decode perform no validation (NaN position, non-unit orientation),
    unlike `Transform`. `Point` fields are public.
32. `Vector3 / 0.0` yields inf/NaN (documented, `vector3/mod.rs:130-132`).
33. `SystemTime::as_seconds_lossy` returns `NaN` before the epoch (`system_time.rs:30-33`).
34. Lookups are not re-validated: an ancestor-ward lookup can return a non-finite translation as `Ok`
    (documented at `error.rs:37-45`).
35. `latest_common_time` returns `Stamp::Static` for an all-static chain; the caller cannot pass a
    `Stamp` to `get_transform` (takes `T`) and must invent an instant.
36. `remove_frame -> bool` and `remove_transforms_before -> ()` report nothing about what was dropped;
    whether `remove_frame` on a frame with children leaves them dangling was not checked (unverified).
37. `Timestamp::now()` panics (documented).

### Stringly typed / allocation
38. Every frame is `&str` in and `String` stored. `Transform` owns two `String`s; `Transform::new`
    allocates twice; each lookup result allocates two `String`s (`Transform::unvalidated(target.into(), ..)`,
    `registry/mod.rs:345` region); `inverse` clones two; `Mul` consumes by value (callers clone to reuse).
39. Errors allocate up to three `String`s (`NotFoundAt`, `NoCommonTime`); `IncompatibleFrames`
    carries either a frame name or a formatted `"a -> b"` pair in the same `String` fields.
40. `CycleDetected` and `StaticDynamicConflict` name no frame; `EmptyFrameName` does not say which side;
    tuple-variant `String` payloads (`UnknownFrame(String)`) mixed with struct variants.
41. `TransformError` time payloads are lossy `f64` seconds; `RegistryError` payloads are exact `T` —
    two conventions, and `TransformError` therefore is not generic over `T`.
42. No error enum derives `Clone`/`PartialEq`; tests and callers need `matches!`.

### Generics and bounds
43. `T: TimePoint` has two instantiations in the crate (`Timestamp`, `SystemTime`); every public type
    carries `<T = Timestamp>`.
44. `where T: TimePoint` is written on the *type definitions* of `Transform`, `Point`, `Stamp`,
    `Registry`, `RegistryError`, `Localized`, `Transformable` — bounds on data declarations propagate
    to every downstream signature; `Stamp<T>` and `Point<T>` need none of `TimePoint`'s methods.
45. `TimePoint: Debug` supertrait exists for derive convenience; `checked_sub` exists for one call site
    (expiry); `as_seconds_lossy` exists for Display and `TransformError` payloads only.
46. `Localized<T>`/`Transformable<T>` are generic traits with a defaulted parameter and one implementor.
47. `impl From<InsertError> for RegistryError` is a public impl over a private type.
48. `Registry` derives only `Debug` (no `Clone`); `Timestamp` lacks `Hash`/`Display`/`Default`;
    `Vector3` lacks `Neg` (the crate writes `-1.0 * v`, `isometry.rs:49`).
49. `approx` impls with `default_epsilon = f64::EPSILON` on composite types compare stamp and frames
    exactly — a public dependency on `approx` 0.5 for what is test support.
