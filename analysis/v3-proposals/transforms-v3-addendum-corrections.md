# transforms v3 — addendum: corrections to the v3 notes

A companion to `transforms-v3-1.md`, `-3.md`, `transforms-v3-addendum-2.2.0.md`
and [`analysis/v3-warrant/REPORT.md`](../v3-warrant/REPORT.md). Written
2026-11-03 after a consistency check of the v3 documents against each other
and against the code. Each correction below survived three independent
attempts to refute it. Every statement corrected here was already wrong when
it was written. The dated notes stay as written; this note corrects them, as
v3-3's "Corrections to the 2.1.3 notes" does for the 2.1.3 notes.

None of the corrections changes a verdict, a ledger item or a recommendation.

## v3-1

- **Docs cut (line 14).** The note gives "README −845 lines, MIGRATION
  −808". Those are lines changed (insertions plus deletions), not lines
  removed: `git diff --numstat v2.1.2 v2.1.3` gives README +149/−706 and
  MIGRATION +195/−613. The files shrank from 770 to 213 lines (−557) and
  from 645 to 227 lines (−418).
- **Disposition table (line 92).** The note says the feasibility report's
  disposition table "classifies every one of these as breaking". It does
  not, for two of the seven:
  - Item 2's internal-interning option: the table's row "Private frame
    interning and O(1) frame existence" says the assessment "neither
    recommends it nor declares it incompatible with v2"
    (`analysis/v2-feasibility/REPORT.md:103`).
  - Item 5, the topology-first resolver: it is not in the disposition
    table at all. It appears in the report's separate table of
    implementations tried and excluded.
- **Non-goal quotation (line 97).** The note quotes the f32 non-goal as
  "will not be considered". That wording was last present at v2.1.2
  (`src/lib.rs:37`) and was removed in 2.1.3 (`fa6876a`), the version the
  note reviews. Since then, the README and the crate root say f32 is
  "outside this crate's scope". The point the note makes, that the
  non-goal conflicts with full parity on FPv4-SP and FPv5-SP cores, is
  unaffected.

## v3-3

- **Quotation in §3 (line 160).** The note puts "a sampled edge that could
  not serve the requested time" in quotation marks as the wording of the
  rustdoc, MIGRATION and the changelog. None of them used it verbatim at
  `feee3d0`:
  - the rustdoc said "A sampled edge could not serve the requested time.";
  - MIGRATION said "A sampled edge cannot serve the request";
  - the changelog said "a frame that cannot serve the requested time".

  The argument holds as stated: none of the three said which failing edge
  `frame` names.

## Addendum: what 2.2.0 already did

- **"Not total" (line 59).** The note says `*` and interpolation cannot
  fail. Interpolation can: `Isometry::interpolate` returns
  `Result<Self, TimeError>` and fails when a time span cannot be
  represented as a `Duration` (`src/geometry/transform/isometry.rs:72-78`,
  unchanged since v2.2.0). The buffer reports that failure as an
  interpolation error. Only `*` is infallible. Interpolation cannot fail on
  geometry.
- **`latest_common_time` and precedence (line 92).** The note lists
  "making `latest_common_time` agree with" the declared precedence as v3
  work. `latest_common_time` already decides on topology first: when no
  chain connects the frames it returns `UnknownFrame` or `Disconnected`
  before any coverage check (`src/core/registry/mod.rs:511-525`; the same
  at v2.2.0). The v3 change is to the lookup's precedence. Once the lookup
  puts `Disconnected` before `NotFoundAt`, the two agree, as v3-1 (lines
  33-35) and v3-3 §3 say.

## v3 warrant report

- **"None of the v3 notes records this" (line 93).** Half of the
  cross-time inversion was already recorded. v3-3's verified findings
  (row 4) and §4 record that a source-time point is rejected and that
  every correct caller is forced onto raw geometry. That predates the
  report. What no v3 note recorded was the other half: that a target-time
  point is accepted with a wrong answer.
- **"Frame-less errors" (line 141).** The report says `add_transform`
  errors are unit variants whose messages cannot name the frame. Two are
  not unit variants:
  - `NonUnitRotation(f64)`;
  - `ReparentingNotSupported { current_parent }`, whose message names the
    current parent (`src/core/registry/error.rs:71-75`).

  The unit variants are `SelfReferentialFrame`, `CycleDetected`,
  `StaticDynamicConflict` and `NonFiniteValues`. The v3 ledger item the
  dimos report adds names the first three of these, and that item stands.
