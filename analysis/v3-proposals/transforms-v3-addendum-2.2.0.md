# transforms v3 — addendum: what 2.2.0 already did

A companion to `transforms-v3-1.md`, `-2.md` and `-3.md`. It records the
items from those notes that shipped in 2.x, what each one settled, and
what it left for v3. Written after PR #113 (commits `8896b42` and
`6c8bb4e`), which lands in 2.2.0 next to PR #112. Measurements are from
the x86-64 host probe in `analysis/v2-feasibility`; nothing was measured on
hardware.

## Shipped in 2.2.0

### On-chain `NotFoundAt` attribution (v3-3 §3, PR #112)

Recorded in v3-3 already. Listed here for completeness.

### Sample fold (v3-1 "Next 2.x step", v3-2 §4, v3-3 "Order of work")

A lookup now composes bare geometry along the walk. Frame names are
borrowed from the registry, and one `Transform` is built for the answer.
Finding 1 of v3-3 is resolved: no hop clones names any more, and a static
hop no longer clones its stored transform.

| Ancestor-ward lookup | Allocations before → after | Host time before → after |
|---|---|---|
| 1 hop, exact | 3 → 3 | 161 → 114 ns |
| 1 hop, reverse direction | 5 → 3 | — |
| 4 hops, interpolated | 9 → 3 | 739 → 504 ns |
| 64 hops | 133 → 7 | — |

The 278,880-line differential trace is byte-identical before and after, in
both feature modes, and semver-checks reports no change.

**Correction to v3-1.** It expected "2 allocations per lookup regardless of
hops". The real count is 3 up to 4 hops, then it grows with the hop list's
capacity doubling (7 at 64). The walk still samples before it knows the
common ancestor, so it must keep each half's hops to drop the shared suffix.
A constant count needs the common ancestor first, and computing it first is
the topology-first resolver. The allocation argument for that resolver
is now this one extra, logarithmic allocation, not the per-hop cost.

### A crate-private `Isometry` (toward v3-1 #1 and v3-3 §1)

The fold put all transform arithmetic in one crate-private type:
composition, inversion and interpolation. `Transform`'s `*`, `inverse` and
`interpolate` check frames and stamps and delegate to it. The buffer stores
dynamic history as isometries, and lookups compose them. The composition
frame rule is one function (`ensure_composable`), shared by `Transform`'s
`*` and the lookup's hops.

For v3 this is a starting point, not the proposal:

- **Not public.** No raw geometry became reachable from outside the crate,
  so v3-3 §2's concern that splitting geometry from metadata makes
  unchecked geometry easier to reach does not arise yet.
- **No invariant.** It stores whatever a validated transform held. v3-3
  §1's normalized `Rotation` and translation bound `B` would be enforced
  here. It is also the one place where the renormalizing Newton step would
  go.
- **Not total.** `*` and interpolation can't fail, but `inverse` still
  normalizes and rejects a non-finite result. v3-3's correction to v3-1
  still stands: infallible operations need the bound.

Because there is now one implementation, a v3 rewrite of the numerics
changes one type rather than keeping `Transform` and a lookup path in sync.

### Unconnectable lookups are diagnosed before geometry (new, `6c8bb4e`)

This finding is in none of the three notes. When the walk from `target`
stopped short of `source`, the partial chain was composed and inverted
before the lookup checked whether it answered the question. Over hops of
extreme magnitude, that inversion's `NonFiniteValues` masked `UnknownFrame`,
`Disconnected` or `NotFoundAt`, and the variant depended on argument order:

- `get_transform("b", "missing", t)` returned `NonFiniteValues`;
- `get_transform("missing", "b", t)` returned `UnknownFrame`.

The lookup now checks that its two half-walks meet before composing
anything. Nothing had documented the masking, and `get_transform`
documented `UnknownFrame` for an unknown endpoint, so this was a compatible
fix.

- **Trace:** 11 lines changed, each now the diagnosis the reverse argument
  order already gave.
- **Tests:** a regression test and a property test, both failing on the old
  code. The property requires a failed lookup to be diagnosed the same over
  unit and 1e308 translations. Proptest also found a masked `NotFoundAt`
  inside a single connected tree.

What this means for v3:

- **Part of v3-3 §3 already holds for the lookup:** errors are decided on
  the frames first. What remains for v3 is the declared precedence
  (`NotFoundAt` over `Disconnected`, v3-3 finding 6), which is unchanged,
  and making `latest_common_time` agree with it.
- **v3-3 §1's result still describes v3.** It says `NonFiniteValues` can
  then arise only from construction and deserialization. In 2.2.0 it can
  still come from a lookup, but only from a chain that connects the frames.
  An ancestor-ward lookup still returns an infinite translation as `Ok`
  (v3-3 finding 2). That behaviour is documented, so it waits for v3.

## Not changed by 2.2.0

- `tests/v2_compatibility.rs`: all nine tests stay as the v3-3 table lists
  them. The fold and the fix flipped none.
- The v3 breaking items are all still open:
  - `Rotation` and the translation bound;
  - public `Isometry`, resolved and cross-time result types;
  - topology-first precedence;
  - one vocabulary;
  - required `max_age`;
  - `TransformError<T>`.

## Remaining 2.x items

- Compare differential-trace hashes across the x86-64 and ARM64 CI runners
  (v3-1). Not done.
- Hardware measurement with v3-3 §6's protocol. Not done. The lookup cost
  it would measure is now lower: roughly two-thirds of the host time at
  1 and 4 hops.
