# Design: Transformable v2 for `Registry::transform_into`

- Status: Proposed for v2.0.0
- Scope: Issue #76 only
- Decision: Break `Transformable` to expose frame and timestamp; add one registry helper

## Problem

- Current usage is manual and error-prone: lookup, possible inverse, then apply.
- `Registry` cannot provide a safe one-call API because `Transformable` does not expose frame/timestamp.
- This creates avoidable mistakes in critical runtime paths.

## Goals

- Provide one correct call: `registry.transform_into(&mut value, target_frame)`.
- Keep API minimal and deterministic.
- Keep runtime overhead negligible.

## Non-Goals

- No changes to transform math, interpolation, chaining, or buffering.
- No additional abstraction layers beyond what is required.
- No backward compatibility guarantees.

## Breaking API Change

```rust
pub trait Transformable<T = Timestamp>
where
    T: TimePoint,
{
    fn frame(&self) -> &str;
    fn timestamp(&self) -> T;
    fn transform(&mut self, transform: &Transform<T>) -> Result<(), TransformError>;
}
```

## New Registry API

```rust
impl<T> Registry<T>
where
    T: TimePoint,
{
    pub fn transform_into<U>(
        &mut self,
        value: &mut U,
        target_frame: &str,
    ) -> Result<(), TransformError>
    where
        U: Transformable<T>,
    {
        if value.frame() == target_frame {
            return Ok(());
        }

        let tf = self.get_transform(target_frame, value.frame(), value.timestamp())?;
        value.transform(&tf)
    }
}
```

## Normative Contract

- `frame()` MUST return the object's current frame.
- `timestamp()` MUST return the object's exact timestamp used for lookup/apply consistency.
- `transform()` MUST apply in place and preserve object invariants.
- After successful `transform()`, object frame MUST be the transform `parent`.
- `timestamp()` MUST remain unchanged in this API path.

## Error Semantics

- Lookup failures propagate from `get_transform` as `TransformError`.
- Apply failures propagate from `Transformable::transform` as `TransformError`.
- Same-frame requests return `Ok(())` without lookup.

## Performance Requirements

- Exactly one lookup and one in-place apply on success.
- No transform inversion in the helper path.
- Early same-frame return to avoid unnecessary traversal.
- Accessors are zero-allocation (`&str`, `Copy` timestamp).

## Implementation Scope

- `src/geometry/transform/traits.rs`: add `frame()` and `timestamp()` requirements.
- `src/geometry/point/mod.rs`: implement new trait methods.
- `src/core/registry/mod.rs`: add `transform_into`.
- Update docs/examples that currently perform manual lookup/inverse/apply.

## Validation

- Test success path with `Point`.
- Test same-frame short-circuit on empty registry.
- Test lookup-not-found propagation.
- Test transform-apply error propagation.
- Run full `std` and `no_std` test matrix.
