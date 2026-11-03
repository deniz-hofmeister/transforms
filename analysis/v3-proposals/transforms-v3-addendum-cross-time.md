# transforms v3 — addendum: the cross-time result type

A companion to `transforms-v3-3.md` §2 and §4, which already place a
cross-time result type in v3. This note holds the detailed design that
[`DESIGN.md`](../v2.4-proposal/DESIGN.md) revision 2 proposed as Part A of
2.4.0, and that revision 3 (2026-11-03) moved here. Nothing here is
implemented. The probes it cites ran at `aa29557`.

## Why v3 and not 2.x

- **Only a type change closes the hazard.** `get_transform_at` returns an
  ordinary `Transform` that carries only the target stamp. Applying it to
  the correct source-time point fails with `TimestampMismatch`; applying it
  to a target-time point returns `Ok` and mislabels the result; inserting it
  serves the source-time relationship as if it held at the target time
  (`cross_time_result_keeps_only_target_stamp_and_remains_insertable`,
  `tests/v2_compatibility.rs`). An additive type beside the method protects
  only callers who migrate.
- **Two cross-time APIs would be a second spelling,** and a 2.x type would
  be fixed before the v3 `Rotation`, public `Isometry`, operator and
  vocabulary decisions it depends on.
- **2.x documents the restriction instead.** The `get_transform_at` rustdoc
  ("Temporal metadata limitation"), the README and MIGRATION say to keep
  both instants and apply the geometry explicitly. The crate docs and the
  `Transform` and `Transformable` rustdoc point to that method's
  restrictions.

## Shape in v3

`get_transform_at` itself returns the new type. There is no deprecation
period and no second method. The compatibility test above is retired with
the behaviour it pins: insertion no longer type-checks.

```rust
// transforms::geometry
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTimeTransform<T = Timestamp>
where
    T: TimePoint,
{
    target_frame: String,
    target_time: T,
    source_frame: String,
    source_time: T,
    isometry: Isometry,
}

impl<T: TimePoint> CrossTimeTransform<T> {
    pub fn target_frame(&self) -> &str;
    pub fn target_time(&self) -> T;
    pub fn source_frame(&self) -> &str;
    pub fn source_time(&self) -> T;
    pub fn translation(&self) -> Vector3;
    pub fn rotation(&self) -> Quaternion;

    /// Maps a source-frame point observed at the source time into the
    /// target frame at the target time.
    pub fn transform_point(&self, point: &mut Point<T>) -> Result<(), TransformError>;
}

impl<T: TimePoint> Registry<T> {
    pub fn get_transform_at(
        &self,
        target_frame: &str,
        target_time: T,
        source_frame: &str,
        source_time: T,
        fixed_frame: &str,
    ) -> Result<CrossTimeTransform<T>, RegistryError<T>>;
}
```

The accessors are a 2.x-era sketch. If v3 makes `Isometry` public or adds
a `Rotation` newtype (v3-3 §1), `translation()`/`rotation()` follow that
decision.

## Semantics

- **Lookup.** Same arguments, order, errors and fixed-frame short-circuits
  as the 2.x method. `process_get_transform_at` returns the `Isometry`.
- **`transform_point` checks before it mutates.**
  - `point.frame` must equal `source_frame`, or it fails with
    `IncompatibleFrames { expected: source_frame, found }`.
  - `point.timestamp` must equal `source_time`, or it fails with
    `TimestampMismatch { lhs: point, rhs: source_time }`.
  - There is no `Stamp::Static` exemption: both instants are explicit.
  - On error, the point is unchanged.
- **On success**, it maps the position as `R·p + t` and the orientation as
  `R·q`, then sets the frame to `target_frame` and the timestamp to
  `target_time`. This is the same arithmetic as `Point`'s `Transformable`
  impl.
- **What the type cannot do.** It has no `Mul`, `inverse`, `interpolate`,
  `validate`, or conversion into `Transform`, and it does not implement
  serde. It cannot be inserted, composed as a single-time transform, or
  serialized with its provenance lost.
- **Numbers.** The result is derived and not re-validated, like every
  lookup.
- **Vocabulary.** `target`/`source`, matching the query arguments (v3-3
  §2). No `parent`/`child`, because no tree edge is implied.
- **Positions.** If 2.x ships `Transform::transform_positions` (DESIGN.md
  Part B), this type gets the matching method, taking `timestamp: &mut T`
  because a cross-time result moves the timestamp rather than preserving
  it.

### Open: the fixed frame

v3-3 §4 lists the fixed frame among the type's fields. Revision 2 of the
2.4 proposal left it out: it affects no check, and the caller passed it in.
Decide when the type is built; add it only if a need appears.

### Example

```rust
let x = registry.get_transform_at("camera", t2, "object", t1, "map")?;
let mut detection = Point::new(p, q, t1, "object");
x.transform_point(&mut detection)?; // now in "camera" at t2
assert!(x.transform_point(&mut Point::new(p, q, t2, "object")).is_err());
```

## Tests

- The happy path matches the manual formula on the `std_advanced`
  scenario, bit for bit, against the 2.x method's geometry.
- `transform_point` rejects a target-time point when the instants differ.
- `transform_point` rejects a wrong source frame.
- On error, the point is unchanged.
- The fixed-frame short-circuits: an endpoint equal to `fixed_frame`, and
  both endpoints equal to it.
- The errors match the 2.x method's for the same arguments: unknown,
  disconnected, not found.
- `tests/properties.rs`: over random trees and instants, a source-time point
  mapped by `transform_point` equals the manual formula, and the result's
  accessors echo the request.
- No compile-fail check is needed: the type has no conversion into
  `Transform`, so insertion does not type-check.

## Migration note to write

Callers that apply the geometry explicitly switch from
`transform.rotation()`/`translation()` on a `Transform` to the same
accessors on the new type, or to `transform_point`. Callers that read
`parent()`/`child()`/`timestamp()` switch to the target/source accessors.
Callers that inserted or composed the result no longer compile, which is
the point.
