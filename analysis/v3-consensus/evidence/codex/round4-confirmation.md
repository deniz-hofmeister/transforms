**SIGN OFF WITH THESE EDITS.** M10 fairly records my preference; the probe confirms the already-known hazard and does not change it.

Remaining corrections in [DESIGN.md](analysis/v3-consensus/DESIGN.md):

1. **Lines 3–5 — prior rulings do exist.** Replace with:

   > Date: 2026-11-17. Baseline: master `adccbdd` (v2.4.0). No integrated v3 implementation exists. This document proposes changes; prior maintainer rulings are identified below. Breaking changes require maintainer sign-off.

2. **Lines 49–51 — the entire surface-count difference is not attributable to labels.** Delete:

   > Claude's first draft counted 72; labelled results and `CrossTimeTransform` cost the difference.

3. **Lines 216–217 — `UnknownFrame` describes current state, not history.** Replace with:

   > `UnknownFrame` there means the child is not currently known to the registry; `NoParent` means it is currently known only as a root.

4. **Lines 239–240 — `MAX` does not universally block insertion.** Replace with:

   > `Timestamp::MAX` exists as a cleanup cutoff. Inserting a dynamic sample at `MAX` anchors bounded retention there: subsequent samples older than the representable `MAX - max_age` cutoff are rejected until that history is removed. Samples within the window remain accepted; `unbounded()` applies no expiry rejection.

5. **Line 588 — cross-time results do describe the supplied times.** Replace the reason with:

   > Both result types describe their endpoint frames; `CrossTimeTransform` also describes both supplied times. Neither detects incorrect caller intent. Swapped times or an incorrectly chosen moving fixed frame can produce `Ok`; these remain caller responsibilities.

6. **M10 — “old JSON still reads correctly” needs qualification.** Replace the Codex cell with:

   > (a) Keep 2.4's field names, order and variant indices (`Static = 0`, `At = 1`) with an `i64` stamp. Least churn. JSON timestamp integers within the new range retain their values, subject to the other new validation rules; out-of-range timestamps are rejected. Old postcard data can decode silently to wrong instants, so external schema identification remains required. The whole-transform probe confirms this known hazard and does not change this preference.

7. **M15 — narrowing the duration to `i64` is not required and can reject valid arithmetic.** Replace its “Grounds” sentence with:

   > Grounds: raw `i64` timestamp subtraction can overflow; duration arithmetic needs wide intermediates and a checked final timestamp range, not an early narrowing of the duration to `i64`.

8. **M16 — the stated alternative is a false dichotomy.** Replace its note with:

   > A precise diagnostic for a failed runtime normalization postcondition. The rounding argument suggests the failure is unreachable but is not a proof. A broader numerical-failure variant is another possible design; mislabelling a finite failure as `NonFinite` is rejected.

9. **Lines 679–683 — the archived stub is not the current API block.** It lacks lookup `NonUnitRotation`, `Stamp`’s `Eq`/`Hash`, and `Registry::Debug`, and declares the opposite `Stamp` order. Replace with:

   > An archived stub of an earlier consensus API, with equivalent adapter sketches, was reported clean under clippy pedantic on 1.85 and stable in both serde modes and built for `thumbv6m-none-eabi`. It predates several current enum and derive changes and does not verify the final API or its runtime contracts.