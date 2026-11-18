
# AGENT 1
SUMMARY: Most measurements in DESIGN.md trace correctly to evidence/, but the headline surface estimate is wrong: the document's own API block counts to 78 named items, not "roughly 60". Three other statements are wrong (the 15-variant count, the six generic types, "the only differences" in the differential run), three are unsupported, and three overstate their source. I ran no prototypes or probes; every check is a read of evidence/ and src/ plus a scripted count of the API block.

## [wrong] **Public surface:** from 105 named items to roughly 60 (counted the way [`evidence/map-api.md`](evidence/map-api.md) reaches 105; the exact figure depends on M1 to M3).
PROBLEM: Counting the document's API block by the map-api.md section 5 method gives 78, not about 60. That is a 26% cut from 105, and it is larger than the 72 the scope critique counted for Claude's first draft. The M1 to M3 spread is 76 to 80.
EVIDENCE: Scripted count of the rust block in DESIGN.md:53-182: 11 types; 39 `pub fn` plus 2 associated consts = 41 (Vector3 1, Rotation 7, Transform 8, CrossTimeTransform 6, Timestamp 8, Stamp 1, Registry 10); 21 enum variants (TransformError 5, InsertError 8, LookupError 6, Stamp 2); 5 public fields (Vector3 3, BatchError 2). 11+41+21+5 = 78. Method: evidence/map-api.md:209-221 (modules, types, traits, constants, methods, trait methods, variants, fields). evidence/stress-critic-scope.md:41-66 counts ass
REPLACEMENT: **Public surface:** from 105 named items to 78 (counted the way [`evidence/map-api.md`](evidence/map-api.md) reaches 105: 11 types, 41 methods and associated constants, 21 variants, 5 public fields; 76 without M3, up to 80 with M1 and M2). That is a 26% cut, and more items than the 72 counted for Claude's first draft, because labelled results and `CrossTimeTransform` came back.

## [wrong] - The surface count of "roughly 60" is an estimate; the scope critique
  counted Claude's first draft at 72 and its own minimum at 50.
PROBLEM: Same error as the Verdict line. The 72 and 50 are quoted correctly, but the block counts to 78, so calling 60 an estimate understates by 18 items.
EVIDENCE: Same count as above; evidence/stress-critic-scope.md:60 (72 / 50).
REPLACEMENT: - The surface count is 78 by the map-api.md method (76 to 80 depending on M1 to M3); the scope critique counted Claude's first draft at 72 and its own minimum at 50.

## [wrong] Flat `RegistryError<T>` (15 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError`
PROBLEM: `RegistryError` has 14 variants in 2.4, not 15. The 9 for `TransformError` is correct.
EVIDENCE: src/core/registry/error.rs:24 onward lists 14 variants (NonUnitRotation, NonFiniteValues, EmptyFrameName, SelfReferentialFrame, ReparentingNotSupported, CycleDetected, StaticDynamicConflict, NoParentToReplace, ParentUnchanged, UnknownFrame, Disconnected, NotFoundAt, NoCommonTime, TransformError). evidence/map-api.md:64 says "14 variants"; evidence/claude-position.md:142 says 14.
REPLACEMENT: Flat `RegistryError<T>` (14 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError`

## [wrong] `TimePoint`, `TimeError`, `impl TimePoint for SystemTime`, `<T>` on six public types, the `Result`-returning operators, `as_seconds`, `as_seconds_lossy`, `now`, `try_now`, `zero`
PROBLEM: 2.4 has `<T>` on five public types and two public traits, seven items in all. The document's own Time section lists those seven ("`Registry`, `Transform`, `Stamp`, `Point`, `RegistryError` and both traits"), so "six" also contradicts it.
EVIDENCE: `grep -rnE 'pub (struct|enum|trait) \w+<T' src` returns RegistryError (core/registry/error.rs:24), Registry (core/registry/mod.rs:95), Transform (geometry/transform/mod.rs:84), Stamp (time/stamp.rs:44), Point (geometry/point/mod.rs:40), and the traits Localized and Transformable (geometry/transform/traits.rs:34, 162). DESIGN.md:270-271.
REPLACEMENT: `TimePoint`, `TimeError`, `impl TimePoint for SystemTime`, `<T>` on five public types and two traits, the `Result`-returning operators, `as_seconds`, `as_seconds_lossy`, `now`, `try_now`, `zero`

## [wrong] zero named-frame disagreements;
the only differences are the intended `Disconnected` precedence.
PROBLEM: The differential run has a second difference class: 3,344 queries where 2.4 returns `Ok(identity)` and the prototype returns `UnknownFrame` (target == source, name stored nowhere). Codex's rerun reports both classes. The prototype also contradicts the design's own rule here ("A non-empty frame relative to itself is the identity, registered or not"), so on this case it does not implement the design.
EVIDENCE: evidence/stress-proto-hotpath.md:39-40 (73,889 precedence differences; 3,344 identity vs `UnknownFrame`) and :46. evidence/codex/round2-response.md:98 ("the documented disconnected-precedence and unknown-self differences"). 2.4 behaviour: src/core/registry/mod.rs:1089-1091. DESIGN.md:318-319.
REPLACEMENT: zero named-frame disagreements;
two difference classes: the intended `Disconnected` precedence (73,889 queries) and 3,344 same-name queries on an unregistered frame, where 2.4 returns the identity and the prototype returns `UnknownFrame`. The design keeps 2.4's identity there, so the prototype differs from the design on that case.

## [unsupported] Codex signed off on this text in a third
  ([`evidence/codex/`](evidence/codex/)).
PROBLEM: The cited directory holds no third round. Round 2 is a response to Claude's positions, written before this text existed, with one HOLD (B5) and several ACCEPT WITH CHANGE; it is not a sign-off on DESIGN.md.
EVIDENCE: `ls evidence/codex`: round1-position.md, round1-prompt.md, round2-prompt.md, round2-response.md, probes/. No round3 file. round2-response.md:110 "B5 — HOLD".
REPLACEMENT: Codex's round-2 response ([`evidence/codex/round2-response.md`](evidence/codex/round2-response.md)) is the last recorded exchange; its sign-off on this text is not in the evidence directory. (Or add the round-3 prompt and response to `evidence/codex/` and keep the sentence.)

## [unsupported] It leaves measured hot-path waste alone.** 2.4 samples edges above the
  common ancestor and discards them, hashes once per hop, and scans every
  buffer to decide that a frame is unknown (66 µs at 1k frames).
PROBLEM: The three behaviours are true of 2.4, but the 66 µs is not the cost of the scan. That workload queries from the leaf of a 1k-deep static chain, so 2.4 first walks about 1,000 hops into a `Vec` and only then scans. The evidence attributes the gain to interning in general and never isolates the scan. 66 µs is also run 1 only; the three runs give 49.6, 66.1 and 94.7 µs.
EVIDENCE: Behaviours: src/core/registry/mod.rs:1002 (`data.values().any(..)`), :1230-1231 (`data.get(current_frame)` per hop), :1251-1272 (walk to root, then truncate). Workload: evidence/prototypes/hotpath/src/lib.rs:187 (`deep_static(), "1000", "unknown"`). evidence/stress-proto-hotpath.md:60 (2.4: 10 allocations, 180 kB on that workload, i.e. a 1k-hop Vec), :84 (66062/94676/49618 ns), :105.
REPLACEMENT: It leaves measured hot-path waste alone.** 2.4 samples edges above the
  common ancestor and discards them, hashes once per hop, and scans every
  buffer to decide that a frame is unknown (an unknown-frame query against a 1k-deep chain costs 50-95 µs, which includes the data-first walk of the known side as well as the scan; the scan was not measured alone).

## [unsupported] Same number of types, five fewer public items.
PROBLEM: No evidence file contains this count and neither obvious reading gives five. Against `Quaternion` alone the difference is four. Including `QuaternionError`, which the same table row lists as replaced, it is seven fewer items and one fewer type, so "same number of types" also fails on that reading.
EVIDENCE: 2.4 per evidence/map-api.md:158-171 and :108: `Quaternion` = 1 type + 4 fields + 7 methods = 12; `QuaternionError` = 1 type + 2 variants = 3. v3 `Rotation` (DESIGN.md:62-71) = 1 type + IDENTITY + 6 methods = 8. `grep -rn 'five fewer\|fewer public' evidence` returns nothing.
REPLACEMENT: One type replaces two (`Quaternion`, `QuaternionError`): 8 named items against 15.

## [overstated] | One hop, exact stamp, 1k samples | 1.2-2.9x |
| One hop, interpolated | 1.2-2.0x |
| Four dynamic hops, interpolated | 0.9-1.2x (slerp dominates) |
| Robot tree, leaf to leaf | 2.7-4.3x |
| Siblings under a 16-deep dynamic chain | 20-33x |
PROBLEM: These ranges span all four prototype variants, including the `VecDeque` history and the `BTreeMap` name index that the design rejects. For the adopted combination (BTreeMap history, hashbrown index) the ranges are narrower; the top of the one-hop and leaf-to-leaf ranges comes from rejected variants. Static climb (2.3-3.3x) and the unknown-frame row do match the adopted variant.
EVIDENCE: evidence/stress-proto-hotpath.md:75-81, column proto-btree-hash: 1hop_exact 2.19/1.64/1.51; 1hop_interp 1.51/1.43/1.25; 4hop 0.90/1.06/1.12 and 0.97/1.09/1.13; leaf_to_leaf 3.71/3.71/3.31; siblings 24.10/28.46/27.67. The 2.82-2.85x and 4.29x maxima are the btreeidx and deque columns. The all-variant ranges are at :104-106.
REPLACEMENT: | One hop, exact stamp, 1k samples | 1.5-2.2x |
| One hop, interpolated | 1.25-1.5x |
| Four dynamic hops, interpolated | 0.9-1.1x (slerp dominates) |
| Robot tree, leaf to leaf | 3.3-3.7x |
| Siblings under a 16-deep dynamic chain | 24-28x |

(Add under the table: "Ranges are the three runs of the adopted variant, BTreeMap history with a hashbrown index.")

## [overstated] at 32,768 Hz a stamp
  rounds by up to 0.5 ns, and interpolation ratios shift in the eighth
  digit.
PROBLEM: The 0.5 ns is sourced; "the eighth digit" is not. The time critique bounds the shift at about 1 ns over the sample span, so the digit depends on spacing: 1e-8 only at about 100 ms spacing. Codex's own 32,768 Hz example shifts the ratio by 8.2e-6, the sixth decimal place. The sentence understates the cost by up to three orders of magnitude for adjacent-tick samples.
EVIDENCE: evidence/stress-critic-time.md:248-251 ("rounds by up to 0.5 ns per stamp ... ratios move by at most ~1 ns over the sample span"). evidence/codex/round2-response.md:53 (ratios 0.49999180797902842 and 0.50000819202097158).
REPLACEMENT: at 32,768 Hz a stamp
  rounds by up to 0.5 ns, and interpolation ratios shift by up to about 1 ns divided by the sample span (1e-8 at 100 ms spacing; 8e-6 in Codex's adjacent-tick example).

## [overstated] | Unknown frame, 1k frames | over 1000x (66 µs to 44-92 ns) |
PROBLEM: The left side is one run and the right side is the range over three runs. The 2.4 figure across the same runs is 49.6-94.7 µs. The ratio (1027-1270x within run) is correct.
EVIDENCE: evidence/stress-proto-hotpath.md:84 (v2.4 66062/94676/49618 ns; proto-btree-hash 52.0/92.2/44.3 ns) and :105 ("66-95 us -> 44-92 ns").
REPLACEMENT: | Unknown frame, 1k frames | over 1000x (50-95 µs to 44-92 ns) |

## [nit] `QuaternionError`, the tolerance check in three places
PROBLEM: The check itself exists once in 2.4, in `Transform::validate`. It is invoked from three call sites. As written it reads as three copies of the comparison.
EVIDENCE: src/geometry/transform/mod.rs:320 is the only comparison against `UNIT_NORM_TOLERANCE`; `validate()` is called at mod.rs:159 (`new`), mod.rs:593 (deserialize) and src/core/buffer/mod.rs:322 (`Buffer::check`). evidence/map-api.md:117-118, 134.
REPLACEMENT: `QuaternionError`, the tolerance check invoked from three call sites (`new`, decoding, insert)

## [nit] A sorted `VecDeque`
  makes steady inserts 3.3-4.8x faster and out-of-order inserts far
  slower (newest 1% of a 60k window: 9-13x; uniform: 115-230x)
PROBLEM: The 115x lower bound compares across runs (70 µs from run 3 against 2.4's ~0.6 µs from run 1). Within-run ratios are 129x, 230x and 147x, and the source itself says only within-run ratios are usable. The 3.3-4.8x and 9-13x figures are correct.
EVIDENCE: evidence/stress-proto-hotpath.md:91 (2.4 604.1/584.8/475.9 ns; deque 77666/134550/69921 ns), :71, :113.
REPLACEMENT: A sorted `VecDeque`
  makes steady inserts 3.3-4.8x faster and out-of-order inserts far
  slower (newest 1% of a 60k window: 9-13x; uniform: 130-230x)

## [incomplete] The review's open decisions, as resolved here: (1) `inverse` — the
translation-corrected conjugate;
PROBLEM: The review's decision table has 16 rows. The paragraph covers 1-11 and 15, and M4 covers 16, but 14 (what the 3.0 `ingest` example shows) is still open in the review and is not mentioned. Rows 12 and 13 are marked decided in the review.
EVIDENCE: analysis/v3-review/REPORT.md:940-956 (rows 1-16; row 14 "No firm recommendation").
REPLACEMENT: Append to the paragraph: "Decision 14 (the 3.0 `ingest` example) is not addressed here; 12 and 13 were already decided, and 16 is reopened as M4."

NOT CHECKED: No prototype, benchmark or probe was rerun; all measurements were checked only against the recorded evidence files and raw outputs; Codex's own probe figures (17,426/528,047, the i64::MIN case, the postcard decode, the 32,768 Hz ratios) cite a scratch path outside the repository; I did not confirm that evidence/codex/probes/*.rs reproduces them; The Codex model id `gpt-6-astra` and "high effort" appear nowhere in evidence/; no source found; "Sub-agents ran `claude-fable-5-2` at low effort" has no source in evidence/; Downstream facts (dimos, transforms_io, roslibrust, the unmerged PR) were checked only against map-dependents.md, which itself marks downstream line numbers unverified; E0283 on all-static setups was not compiled; taken from stress-critic-time.md:89 and map-dependents.md:267; "With the default `Timestamp` a time-arithmetic failure inside a lookup is already unreachable" was checked only against stress-critic-time.md:76, not re-derived from src/; Non-numeric design claims (semantics, AGENTS.md consequences, the Not-in-3.0 reasons) were out of scope for this check; Whether a thresholded sqrt-normalized product stays inside the 8-EPS band: the measured populations are constructed values and Newton-stepped products (stress-proto-numerics.md:89), not exactly the maintenance rule the design specifies

# AGENT 2
SUMMARY: DESIGN.md violates no AGENTS.md Non-negotiable or invariant outright, but its "AGENTS.md under this design" section under-declares what changes, two prior maintainer rulings are changed without an M-row, and the scope rule is applied unevenly. Nothing was built or run; this is a read-through against AGENTS.md, evidence/map-api.md, the 2.4 proposal section 8 and v3-review/REPORT.md.

**AGENTS.md, bullet by bullet**

Non-negotiables:
- `forbid(unsafe_code)`: preserved by silence; the design never mentions it.
- Dependency list: changed and declared (M7).
- `no_std` parity and additive features: changed and declared (M6). The "slerp is pinned bit for bit" clause is changed but not declared.
- Non-Goals: preserved.
- No panics, checked time arithmetic: changed and declared (the `now` panic clause goes); the checked-arithmetic rule is preserved.

Correctness invariants:
- Transform direction: preserved in substance; the parent/child wording changes and is listed as "stands".
- Composition: changed and declared.
- Static xor dynamic, unordered `Stamp`, tagged serde: preserved; the error name changes and is not declared.
- Strict tree: the rules are preserved, but "EmptyFrameName, checked at insert" moves to the constructor and is not declared; the error names change too.
- Batch all-or-nothing: preserved; the "`Buffer::check` must never read the stored samples" clause needs the expiry exception and that is not declared.
- Lookup diagnosis: changed and declared, except the `NoCommonTime` payload change and the empty-name self-lookup.
- Flat `RegistryError`: changed and declared (M9).
- No extrapolation, `TimestampOutOfRange`: changed and declared.
- Error formatting through `as_seconds_lossy`: changed and declared.
- Expiry and cleanup: cleanup is changed and declared (M4); the new `Expired` error on insert is not declared here, and expiry is listed as "stands".
- Validated where built: changed and declared.
- The two "derived values are not re-validated" bullets and insert-time validation: changed and declared.
- "`from_wxyz` does not normalize": declared; the clause "anything that inverts must normalize first" is replaced by the conjugate inverse and is not declared in this section.
- One inversion, bit-exact single hop: preserved.
- Exact `==`: changed and declared.
- `#[non_exhaustive]` errors: preserved.

Scope and Mindset:
- Raw state is not exposed: preserved.
- "Buildable outside belongs outside": applied to M1, M2 and M3 only. `checked_add`, `checked_sub` and `checked_duration_since` break it undeclared; `Rotation::rotate`, `compose`, `inverse`, `Timestamp::MAX` and `Stamp::at` are kept with no scope argument.
- "No speculative API": `checked_add`, `Hash` and `Display` on `Timestamp` have no demonstrated caller.
- "No addition is precedent for its neighbors": M3's note is in tension with it.

Style, the verification gate and the Releasing checklist also change (constructor list, std-mode matrix, four semver combinations) and the section does not mention them.

**Internal consistency**
- Two counts are wrong against the document's own source: the surface is about 78 items by the map-api.md method, not "roughly 60", and `RegistryError` has 14 variants, not 15.
- Several API-block items appear in no table row or prose, and several 2.4 items have no stated fate (listed in the findings).
- The "Review of the review" verdicts agree with the design sections, apart from the "adopt as settled" wording on item 10 and an incomplete "reopened" list.
- The "signed off in a third round" claim has no record in evidence/codex/.

**Rulings changed without an M-row**
- 2.4 D6: the enforcement site for empty names, and the ruled lookup rule that `""` against itself is the identity.
- 2.4 D2: M1 reopens checked application without citing the ruling.
- Review item 10 D1: `RegistryError::SampleExpired` becomes `InsertError::Expired`.
- Review decisions 13 and 14 (examples) are not addressed at all.

## [wrong] **Public surface:** from 105 named items to roughly 60 (counted the way
  [`evidence/map-api.md`](evidence/map-api.md) reaches 105; the exact
  figure depends on M1 to M3).
PROBLEM: Counting the API block the way map-api.md section 5 does (types, constants, inherent methods, enum variants, public fields; no trait impls, no payload field names) gives about 78, not 60. M1 to M3 move it by at most +2 or -2.
EVIDENCE: Hand count of DESIGN.md:53-182: 11 types + 2 constants (`IDENTITY`, `MAX`) + 39 methods (Vector3 1, Rotation 6, Transform 8, CrossTimeTransform 6, Timestamp 7, Stamp 1, Registry 10) + 21 variants (TransformError 5, InsertError 8, LookupError 6, Stamp 2) + 5 public fields (Vector3 3, BatchError 2) = 78. Method is map-api.md:209-221. evidence/stress-critic-scope.md:60 counted the smaller first draft at 72 by the same method.
REPLACEMENT: **Public surface:** from 105 named items to 78 (counted the way [`evidence/map-api.md`](evidence/map-api.md) reaches 105: 11 types, 2 constants, 39 methods, 21 variants, 5 public fields; 76 without M3, 80 with M1 and M2). Also correct the Limits bullet: "The surface count is 78 by hand count of the API block; it was not tool-verified."

## [inconsistent] pub fn checked_add(self, d: Duration) -> Option<Self>;
    pub fn checked_sub(self, d: Duration) -> Option<Self>;
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration>;
PROBLEM: The scope rule is applied to M1, M2 and M3 but not to these three. All are exact on `as_nanos`/`from_nanos`, none has a demonstrated public caller, and `checked_add` has no internal caller either. They appear in no table row, no prose and no M-row. The design's own scope critique cut them.
EVIDENCE: evidence/stress-critic-scope.md:97: "Cut | S1. Exact on `as_nanos`/`from_nanos` with `i64::checked_*` ... no caller of the public forms is demonstrated. 2.4's `Add<Duration>` had zero internal callers too." evidence/stress-critic-time.md:363 calls `checked_add` "Weaker than the rest; keep for symmetry ... only if the maintainer accepts that reason". map-api.md:201 (`Add<Duration>`: internal callers none). AGENTS.md Scope first bullet and Mindset "No speculative API".
REPLACEMENT: Either delete the three signatures from the API block (expiry keeps a private subtraction), or add a row to the agreed table: "| M13 | `Timestamp::checked_add`, `checked_sub`, `checked_duration_since` in the core | All three are exact outside on `as_nanos`/`from_nanos`, so this is a scope exception like M3. Grounds: raw `i64` subtraction of two stamps can overflow and `Duration` to `i64` needs a checked narrowing. `checked_add` has no demonstrated caller and is the weakest; cut it first. |"

## [inconsistent] - **Names.** Empty names are rejected by `Transform::new` and by decoding.
PROBLEM: This changes maintainer ruling D6 of the 2.4 proposal (2026-11-12: enforced at the insert boundary; constructors, `Deserialize` and `validate` unchanged). The same ruling's lookup rule, that `""` against itself is the identity, is changed by "An empty query name is `UnknownFrame`" (DESIGN.md:318). Neither appears in M1 to M12. The AGENTS section lists "strict tree" as standing although that bullet says "`EmptyFrameName`, checked at insert".
EVIDENCE: analysis/v2.4-proposal/DESIGN.md:905 (D6 ruling "2026-11-12: the insert boundary"); :463-465 ("Constructors, `Deserialize` and `Transform::validate` are unchanged"); :467-470 ("`""` against itself is the identity, as documented for every unregistered" name). AGENTS.md strict-tree bullet. DESIGN.md:318-319, :323.
REPLACEMENT: Add to the agreed table: "| M13 | Empty frame names are rejected by `Transform::new` and by decoding, not at insert; `lookup("", "", t)` is `UnknownFrame`, not the identity | Changes the 2.4 ruling D6 of 2026-11-12 (insert boundary, decoding untouched, `""` against itself is the identity). Grounds: the invariant is the type's, so insert has nothing left to check; 3.0 decoders reject payloads with an empty name that 2.4 accepted. |" In the AGENTS section, move "strict tree" to "Invariants that change text" with "(empty names rejected at construction; error names)".

## [incomplete] - **Invariants that stand:** direction of a transform; static xor
  dynamic; strict tree and `reparent`; batch atomicity; exact frames or
  an error decided before geometry; no extrapolation; data-driven expiry;
  one inversion and a bit-exact single hop; `#[non_exhaustive]` errors.
PROBLEM: Several of these bullets change text under the design:
- Direction: `(parent, child)` becomes target/source.
- Static xor dynamic: `StaticDynamicConflict` becomes `KindMismatch`.
- Strict tree: `ReparentingNotSupported`, `CycleDetected`, `reparent_frame` and the empty-name site all change.
- Batch atomicity: `add_transforms` is renamed, and "`Buffer::check` must never read the stored samples" needs the expiry exception with its running newest stamp.
- Expiry: "removed on insert" gains the `Expired` rejection.

The section also omits these changes:
- The "anything that inverts must normalize first" clause.
- The slerp bit-pin in Non-negotiables.
- The Style constructor list.
- The gate's std-mode matrix.
- The four semver combinations in Releasing.
EVIDENCE: AGENTS.md invariants 1, 3, 4, 5, 10 and 14; Style "Construction goes through constructors"; "Definition of done" ("both std modes"); Releasing ("four feature combinations"). REPORT.md:908-910 already noted the Style constructor list and counted ten changing bullets under a smaller design. REPORT.md:423-424 says an AGENTS.md bullet for `SampleExpired` is drafted in sample-expired-contract.md.
REPLACEMENT: - **Invariants that stand in substance but change names:** direction of a transform (target/source); static xor dynamic (`KindMismatch`); strict tree and `reparent` (`ParentMismatch`, `Cycle`, `SelfEdge`; empty names rejected at construction); batch atomicity (`insert_all`, `BatchError`; the expiry check reads a per-frame running newest stamp, the other checks still read no stored sample); data-driven expiry (an insert older than the window is `Expired`, not dropped).
- **Invariants that stand unchanged:** exact frames or an error decided before geometry; no extrapolation; one inversion and a bit-exact single hop; `#[non_exhaustive]` errors.
- **Also changes:** "anything that inverts must normalize first" (conjugate with corrected translation); the slerp bit-pin (kept as a regression pin, no longer a std-parity pin); the Style constructor list; the gate and CI feature matrix (two modes, 

## [inconsistent] | M1 | Add `Transform::transform_point(&self, p: Vector3) -> Vector3` (`R p + t`)? |
PROBLEM: M1 reopens ruling D2 of the 2.4 proposal (2026-11-12: no scope exception for checked application to positions; "Revisit on a demonstrated need") and does not say so. The proposed method is also weaker than the rejected one, because it checks no frame or stamp. Codex's round-2 response cites the earlier discussion; DESIGN.md dropped the reference.
EVIDENCE: analysis/v2.4-proposal/DESIGN.md:901 (D2 ruling) and :216-258. evidence/codex/round2-response.md:122 links v2.4-proposal/DESIGN.md:224.
REPLACEMENT: Append to the M1 question cell: "Reopens the 2.4 ruling D2 of 2026-11-12 (no scope exception for checked application; revisit on demonstrated need). Unlike the rejected `transform_positions`, this method checks no frame and no stamp. The new fact since D2 is that `Point` is deleted, which removes the crate's only application path."

## [wrong] Flat `RegistryError<T>` (15 variants for every call)
PROBLEM: 2.4 `RegistryError` has 14 variants.
EVIDENCE: src/core/registry/error.rs has 14 variants (lines 35, 47, 54, 58, 81, 88, 107, 130, 140, 146, 153, 182, 208, 224). evidence/map-api.md:64 says "14 variants".
REPLACEMENT: Flat `RegistryError<T>` (14 variants for every call)

## [unsupported] Codex signed off on this text in a third
  ([`evidence/codex/`](evidence/codex/)).
PROBLEM: The linked directory holds no third-round record.
EVIDENCE: `ls analysis/v3-consensus/evidence/codex/` shows round1-position.md, round1-prompt.md, round2-prompt.md, round2-response.md and probes/. There is no round-3 file.
REPLACEMENT: Either add the round-3 prompt and response to evidence/codex/, or write: "Each wrote a position without seeing the other's, then exchanged them over two rounds ([`evidence/codex/`](evidence/codex/)). Codex's sign-off on this text is not recorded in the evidence."

## [incomplete] Deleted without replacement: `Point`, `Transformable`, `Localized`,
`get_transform_for`, `Transform::interpolate`, `Transform::validate`,
`UNIT_NORM_TOLERANCE` as a public constant unless the constructor doc
needs to name it, the `errors`, `geometry` and `time` modules (the crate
root is flat).
PROBLEM: These 2.4 public items have no stated fate anywhere in DESIGN.md:
- `Vector3::zero`.
- `Stamp::is_static`.
- `impl Mul for Quaternion`: v3 has no public rotation product, which is a capability removal.
- `Quaternion::identity` (to `IDENTITY`) and `Quaternion::rotate_vector` (to `rotate`, named only inside a Reason cell).
- `Debug` on `Registry` and `Eq` on `Stamp`.
- `TransformError::StaticInterpolation` and `TimestampError`.
- Variant renames that are never mapped: `IncompatibleFrames` to `FrameMismatch`, `TimestampMismatch` to `StampMismatch`, `NonFiniteValues` to `NonFinite`, `SelfReferentialFrame` to `SelfEdge`, `CycleDetected` to `Cycle`, `StaticDynamicConflict` to `KindMismatch`, `NoParentToReplace` to `NoParent`.
- The `target_frame`/`source_frame` fields dropped from the lookup errors.
- The serialized field names of `Transform` (parent/child/timestamp, or target/source/stamp).

Two further defects: `UNIT_NORM_TOLERANCE` is left undecided, and `Transform::validate` is listed both here and as "replaced" in the table.
EVIDENCE: evidence/map-api.md:175 (`zero`), :206 (`is_static`), :170 (`Mul`), :164, :168, :36 (`Debug`), :205 (`Eq`), :101, :105, :94-106, :66-81. DESIGN.md contains no occurrence of `is_static`, `StaticInterpolation`, `IncompatibleFrames`, `CycleDetected` or `StaticDynamicConflict`. stress-critic-dependents.md:24 already flagged the `Stamp` methods and serde as unstated.
REPLACEMENT: Deleted without replacement: `Point`, `Transformable`, `Localized`, `get_transform_for`, `Transform::interpolate` (with `StaticInterpolation` and `TimestampOutOfRange`), `Vector3::zero`, `Stamp::is_static`, the public Hamilton product (`impl Mul for Quaternion`; rotations compose only through `Transform::compose`), `UNIT_NORM_TOLERANCE` (the constructor doc states 1e-6 in prose), the `errors`, `geometry` and `time` modules (the crate root is flat).

Renamed: `Quaternion::identity` to `Rotation::IDENTITY`; `rotate_vector` to `rotate`; `IncompatibleFrames` to `FrameMismatch`; `TimestampMismatch` to `StampMismatch`; `NonFiniteValues` to `NonFinite`; `SelfReferentialFrame` to `SelfEdge`; `ReparentingNotSupported` to `ParentMismatch`; `CycleDetected` to `Cycle`; `StaticDynamicConflict` to `KindMismatch`; `NoParentToReplace` to `NoParent`. Lookup errors drop `target_frame`/`source_frame`. `Reg

## [incomplete] NoCommonTime { frame: String, earliest: Timestamp, latest: Timestamp, latest_possible: Timestamp },
PROBLEM: 2.4 `NoCommonTime` carries `covered: Option<(T, T)>`, where `None` means a hop holding nothing, and AGENTS.md pins that meaning. The v3 variant has no `None` case, and DESIGN.md never says what `latest_common_time` reports for an empty hop. `NoData` is presumably meant. The AGENTS section names only `NoData`/`OutOfRange` for lookups.
EVIDENCE: AGENTS.md lookup invariant: "`NoCommonTime` ... `Some` names a hop whose range starts past the newest instant ..., `None` a hop holding nothing." evidence/map-api.md:80. DESIGN.md:179 and :314-319 do not mention `latest_common_time` for this case.
REPLACEMENT: Add to Registry semantics: "- **`latest_common_time`** reports a hop holding nothing as `NoData { frame }` and disjoint coverage as `NoCommonTime`; `latest_possible` is the newest instant the rest of the chain serves. The 2.4 `covered: None` case is the `NoData` arm." In the AGENTS section, extend the lookup item to "(precedence, `NoData`/`OutOfRange`, `NoCommonTime` payload)".

## [overstated] | 10 | `SampleExpired` | Adopt as settled. |
PROBLEM: D1 settled the spelling `RegistryError::SampleExpired { frame, stamp, newest }`. The design ships `InsertError::Expired`. M9 covers the enum split but not the variant rename. The design also never restates D3 (never reported without `max_age`; accepted where the subtraction is not representable; `reparent` seed exempt), and `unbounded()` plus a signed type change where that boundary sits.
EVIDENCE: analysis/v3-review/REPORT.md:430 (D1), :432 (D3). DESIGN.md:164 (`Expired`), :306-310.
REPLACEMENT: | 10 | `SampleExpired` | Adopt as settled (D1 to D6, D3's exemptions included), spelled `InsertError::Expired` with the same `frame`, `stamp`, `newest` payload; the rename rides on M9. |

## [incomplete] Decisions 4 (2.x notice) and 11 (minor-release
policy) are not design questions and stay with the maintainer.
PROBLEM: The paragraph accounts for review decisions 1-3, 5-11 and 15. It skips 12 and 16 (handled elsewhere) and 13 and 14, which no part of DESIGN.md addresses. Decision 13 is to port the 2.4 examples to 3.0. Decision 14 is whether `ingest` shows the `remove_frame` recovery or a guard. The design has no examples section, although `Timestamp::zero`, `static_between` and the operators the examples use are deleted.
EVIDENCE: analysis/v3-review/REPORT.md:953-954 (decisions 13, 14) and :721 ("Examples in 3.0"). DESIGN.md mentions examples only at :309.
REPLACEMENT: Decisions 4 (2.x notice) and 11 (minor-release policy) are not design questions and stay with the maintainer. Decisions 12 and 16 are item 10 and M4. Decisions 13 and 14 (porting the examples; what `ingest` shows on `Expired`) are not addressed by this design and remain open.

## [overstated] Reopened and adopted:
  a `Rotation` type, named methods for operators, typed time payloads,
  explicit retention constructors, internal interning, the
  `ReparentingNotSupported` rename, a flat crate root.
PROBLEM: The list omits three other rows of the review's "Not in 3.0" table that the design reverses: signed `Timestamp` (deferred there), approx ("Optional approx" rejected there; the design removes it outright), and the conjugate `inverse`. "It never questions the time generic" is also slightly too strong, since the review considered and deferred a signed `Timestamp`, `TransformError<T>` and deleting `TimePoint::checked_sub`.
EVIDENCE: analysis/v3-review/REPORT.md:662 (conjugate), :675 (`checked_sub`), :676 (optional approx), :677 (signed `Timestamp` deferred), :673 (`TransformError<T>`).
REPLACEMENT: Reopened and adopted: a `Rotation` type, named methods for operators, typed time payloads, a signed `Timestamp`, a conjugate-form `inverse`, `approx` out of the public API, explicit retention constructors, internal interning, the `ReparentingNotSupported` rename, a flat crate root.

And change the later bullet to: "**It defers a signed `Timestamp` and never questions the time generic itself**, the mixed error enum, or the `std` feature."

## [inconsistent] If the scope rule is applied strictly here, apply it to M1 and M2 the same way.
PROBLEM: AGENTS.md Scope says no addition is precedent for its neighbours and each is argued from its own invariant and demonstrated need; this sentence couples three decisions. Separately, the inbound ground for M3 is weakened by the design's own signed type: the scope critique found the hand-rolled errors were made against `u64` and that the naive `i64` one-liner is correct for every ROS stamp. Only the outbound `/ %` ground survives. The design keeps `Rotation::rotate`, `compose`, `inverse`, `Timestamp::MAX` and `Stamp::at` with no scope argument, although the critique cut `rotate` and `MAX` and map-api.md shows no internal caller for 2.4's `Mul`, `inverse` or `Stamp::at`.
EVIDENCE: AGENTS.md Scope, last bullet. evidence/stress-critic-scope.md:96 (`from_secs_nanos`: "With `i64` the naive one-liner ... is correct for every ROS stamp ... Signedness removes the hazard"), :79 (`rotate` cut), :95 (`MAX` cut), :87 (`inverse` "Keep, low confidence ... Zero studied users"). map-api.md:136-137, :206-207.
REPLACEMENT: | M3 | `Timestamp::from_secs_nanos` and `as_secs_nanos` in the core | Both are exact outside, so this is a scope exception, argued on its own. Outbound: a signed type makes the naive `/ %` wrong (-0.5 s gives `(0, 3794967296)`). Inbound: weaker; the two wrong hand-rolled conversions were written against `u64`, and with `i64` the naive `secs * 1e9 + nanos` is correct for every ROS `(i32, u32)` stamp. |

And add one sentence under the API block: "Kept without a scope exception because the substitute is a wrong-answer generator or the algebra is sealed: `Rotation::rotate` (hand-written sandwich product), `compose` and `inverse` (the rotation product is private). Kept as convenience and cuttable under the strict rule: `Timestamp::MAX`, `Stamp::at`, `Rotation::IDENTITY`."

## [nit] Same number of types, five fewer public items.
PROBLEM: The row itself lists `QuaternionError` as replaced, so the types go from two to one. By map-api.md's count, `Quaternion` (1 type, 4 fields, 7 methods) plus `QuaternionError` (1 type, 2 variants) is 15 named items, and `Rotation` is 8 (type, `IDENTITY`, `from_wxyz`, four accessors, `rotate`). That is seven fewer.
EVIDENCE: evidence/map-api.md:108-109, :158-171, :217-220. DESIGN.md:62-71.
REPLACEMENT: One type fewer (`QuaternionError` goes) and seven fewer public items.

## [nit] `<T>` on six public types
PROBLEM: 2.4 has `<T>` on five types (`Registry`, `Transform`, `Stamp`, `Point`, `RegistryError`) and two traits, seven declarations in all. The Time section's own list (DESIGN.md:270-271) names those seven.
EVIDENCE: evidence/map-api.md:315-316. DESIGN.md:270-271.
REPLACEMENT: `<T>` on five public types and two traits

## [nit] pub fn from_wxyz(w: f64, x: f64, y: f64, z: f64) -> Result<Self, TransformError>;
PROBLEM: The design's stated principle is "one error enum per operation family", and it criticises the review for not questioning "the mixed error enum". Here a rotation constructor returns an enum that also carries `EmptyFrameName`, `FrameMismatch` and `StampMismatch`. Either choice is defensible; the document should say which it chose. It is also unstated which error a failed normalization postcondition (Numerics, "reports failure") produces from `from_wxyz`, `compose` and a lookup; `LookupError` has only `NonFinite`.
EVIDENCE: DESIGN.md:39-41, :65, :150-156, :232-233, :414, :174-181.
REPLACEMENT: Add to Numerics, Construction: "`from_wxyz` reports `TransformError::NonFinite` or `NonUnitRotation`; a failed normalization postcondition is `NonUnitRotation` from `from_wxyz` and `compose`, and `LookupError::NonFinite` is renamed `Numerical` (or gains a sibling) to carry it from a lookup. A separate two-variant `RotationError` was considered and not taken: one more type for two variants `TransformError` already has."

## [nit] Two items remain split
  and are the maintainer's ([Decisions](#decisions-for-the-maintainer), M1
  and M2).
PROBLEM: M2 is not a split: Claude's cell reads "Maintainer's call; leaning no" and Codex's reads "No".
EVIDENCE: DESIGN.md:469.
REPLACEMENT: One item remains split (M1); on M2 both lean no and leave it to the maintainer.

NOT CHECKED: Whether the "new mandate" (full refactors allowed, no backwards compatibility) was given by the maintainer; no source for it is in the evidence I read.; Numerical and timing claims (38-40%, 0.66 EPS, 17,426 of 528,047, the speed-up table, 372,996 lookups); no prototype was run.; Dependents claims (dimos carrying signed nanoseconds, "every studied user sets a window", "three of the seven slips"); map-dependents.md and stress-critic-dependents.md were only grepped.; Whether Codex's round-2 response agrees with each M-row cell; only grepped lines of codex/round2-response.md were read.; sample-expired-contract.md, the drafted AGENTS.md bullet for expiry, was not opened.; Whether 2.4 code returns the identity for `get_transform("", "", t)`; taken from the 2.4 proposal text (v2.4-proposal/DESIGN.md:467-470), not from running the crate.; Derive and auto-trait surface of 2.4; map-api.md itself says it was not tool-verified.; The AGENTS.md Style section beyond the constructor list, and the README Non-Goals text against the crate root.

# AGENT 3
SUMMARY: Safety review of the FINAL analysis/v3-consensus/DESIGN.md. Probe (2.4.0 as path dependency + postcard 1.1.3): evidence/verify-hazards/probe/src/main.rs. Nothing in the repository was edited. All v3 behaviour is a reading of signatures and prose; no v3 code exists.

(1) DISPOSITION OF KNOWN HAZARDS (map-dependents.md section 4, silent rows; row numbers are that table's)
- #1 empty frame id joins trees: CLOSED. Transform::new and decoding reject empty names, so hand composition can no longer join on "" either; empty query name is UnknownFrame.
- #2 clock reset / two epochs interleaved: NARROWED only. Expired makes a backwards jump larger than max_age loud. Left open: a jump smaller than max_age, and every Registry::unbounded() registry (Expired can never fire). Not recorded as a residual (finding 6).
- #3 kind switch static->dynamic freezes: NARROWED (diagnostic only). KindMismatch { frame, incoming } names the frame; an adapter that drops errors still freezes.
- #4 interpolation across a dropout: LEFT, and recorded honestly in "Not in 3.0" and M12.
- #5 negative ROS stamps alias: CLOSED inbound by the signed type itself (naive secs*1e9+nanos is now correct); outbound depends on M3 (naive / and % is wrong for negatives, as the document states).
- #6 "latest" answered at wall-clock now: NARROWED (now/try_now removed); the adapter's own clock remains; M2 open. New small idiom risk: latest_common_time(..)?.at().unwrap_or(x).
- #7 cross-time result re-inserted: CLOSED for the zero-line accident (CrossTimeTransform is not insertable, not serde); deliberate rebuild through six accessors remains, same as the review.
- #8 swapped lookup arguments: LEFT at 2.4 level (labels restored, result describes itself). Not closed.
- #9 static re-published under new parent: NARROWED (ParentMismatch names frame, pinned, offered).
- #11 lookup Ok(inf): CLOSED for lookup, compose, inverse via NonFinite.
- #16 Ok for data evicted on arrival: CLOSED by Expired, but that introduces a new batch hazard (finding 2).
- #18 two implementations disagree: outside the crate, untouched.
stress-critic-safety.md amendments against the final text: 1 (named lookup args) REJECTED but mooted for lookup by labels, not for lookup_across (finding 5); 2 (labels) ADOPTED, relabel-on-reinsert closed; 3 (Mul) ADOPTED as checked fallible compose; 4 (transform_vector) gone; 5 (named-field quaternion input) REJECTED and not recorded in "Not in 3.0" (finding 7); 6 mostly ADOPTED (decode via constructors, abs_diff, floor semantics, integer Display) with SystemTime mooted; 7 ADOPTED; 8 ADOPTED; 9 ADOPTED ("A failed insert or batch interns no frame").

(2) NEW HAZARDS RELATIVE TO 2.4, by the prompts given
- serde decode of old data: REAL, highest (finding 1).
- failed batches: REAL (finding 2).
- compose stamp rules with statics: UNSPECIFIED in the document (finding 4).
- lookup_across five positional args: same signature as 2.4 (src/core/registry/mod.rs:439-445), so not new, but the document's stated reason for refusing an argument struct does not cover it (finding 5).
- Rotation::from_wxyz positional: REAL regression, acknowledged in M8, closure not recorded (finding 7).
- Timestamp::MAX exported: new poison-stamp idiom (finding 8).
- Timestamp epoch mixing / negatives: mostly loud. Negative instants that 2.4 could not represent (checked_sub returned an error) are now ordinary values; aftermath is OutOfRange/Expired, loud. Silent residue is hazard #2 above.
- Transform::new(target, source) vs (parent, child): positions are identical to 2.4, so no swap is introduced; both args were same-typed (&str, &str) in 2.4 too (src/geometry/transform/mod.rs:145-151). But the document never states that insert reads target as parent and source as child (finding 9).
- thresholded normalization changing stored values: benign; a rotation within 1e-6 but outside 8 EPS is stored normalized, differing from the wire by <= ~1e-6. Declared in Acceptance. No wrong-answer path found.
- remove_samples_through(Timestamp::MAX): no silent wrong answer found; aftermath is NoData. It clears every dynamic frame, not one.
- identity for an unregistered frame relative to itself: NOT new (2.4 probe: get_transform("typo","typo",t) = Ok identity; latest_common_time = Ok(Static)); existing silent path, unrecorded (finding 10).
- inverse of a static transform: no hazard found; stays Static, labels swap, re-insert the wrong way round is caught by ParentMismatch/Cycle where the tree exists.
- interned-slot reclamation and name reuse: no public hazard (ids are private, reclamation requires no edge and no child, drained frames keep their edge). Internal correctness is unverified: the prototype omits it (finding 11).

(3) CLOSURE CLAIMS: "valid always" has a specification gap for lookups (finding 3) and admits geometry-inconsistent self-named values (finding 10). "Residual ... disappears" (review item 2) holds as specified. "Old Timestamp bytes are not readable" is false as worded (finding 1).

(4) "Not in 3.0" omissions: named-field rotation input; sub-max_age and unbounded clock reset; wire-format guard against 2.x bytes; an argument struct for lookup_across specifically; a fixed() accessor. Details in findings.

## [wrong] A frozen serde schema for 3.0. Old `Timestamp` bytes are **not** readable: postcard decodes an old `u64` 1 as `i64` −1. Persisted data must be migrated with the old decoder; no dual decoding.
PROBLEM: The sentence contradicts itself and understates the hazard. Old bytes ARE readable: they decode with Ok and a wrong instant. That is exactly the AGENTS.md worst failure, across a version boundary, and nothing in the design makes it loud. Whether a whole 2.x `Transform` decodes silently depends on two things the document leaves unspecified: field order and `Stamp` variant order. The document's own signature writes `pub enum Stamp { At(Timestamp), Static }`, the reverse of 2.4 (`Static`, then `At`), so the frozen schema could go either way by accident. M10 repeats the wording ("cannot read 2.x `Timestamp` bytes").
EVIDENCE: Probe output (scratch probe, postcard 1.1.3): `u64 1 -> i64 Ok(-1)`, `u64 2 -> i64 Ok(1)`, `u64 1700000000000000000 -> i64 Ok(850000000000000000)`. A real 2.4.0 `Transform` (map<-base, At(1.7e18 ns)) encoded by 2.4 and decoded into a v3-shaped struct with 2.4's field order (src/geometry/transform/mod.rs:565-573) and 2.4's variant order (src/time/stamp.rs:44-52): `Ok(NewT { translation: (1,2,3), rotation: identity, stamp: At(850000000000000000), target: "map", source: "base" })` - a valid transfo
REPLACEMENT: A frozen serde schema for 3.0, fixed down to field order and `Stamp` variant order. 2.x postcard data is **not safe to read**: an old `u64` stamp decodes without error as a wrong `i64` (1 becomes −1, 1.7e18 becomes 8.5e17), and with 2.4's field and variant order a whole 2.x `Transform` decodes `Ok` with a wrong instant (probed). The 3.0 layout must therefore be chosen so that the frozen 2.4 byte fixtures in `tests/serde.rs` fail to decode, and a test pins that failure; if no such layout is found, this is a recorded silent hazard in MIGRATION and in "Not in 3.0". JSON is unaffected for stamps below 2^63. Persisted data is migrated with the 2.x decoder; no dual decoding.  [and in M10: "A new serde schema; 2.x postcard bytes decode to wrong instants unless the layout is made to reject them"]

## [incomplete] an insert older than the frame's newest stamp minus `max_age` returns `InsertError::Expired` and stores
  nothing. The batch check carries a per-frame running newest stamp.
PROBLEM: New hazard from combining `Expired` with all-or-nothing batches. One stale member for frame X now rejects fresh samples for every other frame in the same batch. 2.4 stores them. A ROS `/tf` message is a batch of unrelated frames, and the dependents study records that adapters drop loud insert errors (map-dependents hazards #3, #9). An adapter that logs and drops `BatchError` now silently loses fresh data for healthy frames; lookups then interpolate or serve stale coverage for them (hazard #4 path). The document mentions only "Recovery examples handle a failed re-insert" and does not state this consequence or the fallback.
EVIDENCE: Probe on 2.4.0: registry `with_max_age(1 s)`, x newest at 10 s; `add_transforms([x@1s, y@10s])` returns `Ok(())` and `get_transform("odom","y",10s)` is `Ok`. Under DESIGN.md lines 304-309 the same call is `Err(BatchError { index: 0, error: Expired })` and y is not stored. Scenario: `if let Err(e) = reg.insert_all(&msg_transforms) { log::warn!("{e}"); }`. No evidence file discusses this interaction (grep for per-member/whole-batch fallback in analysis/v3-review/REPORT.md and evidence/*.md found n
REPLACEMENT: an insert older than the frame's newest stamp minus `max_age` returns `InsertError::Expired` and stores nothing. The batch check carries a per-frame running newest stamp. Consequence, declared: one expired member fails the whole batch, so a late sample for one frame rejects fresh samples for every other frame in it (2.4 stored them). The `insert_all` rustdoc and the recovery example show the fallback - on `BatchError` whose error is `Expired`, re-submit the members other than `index` (or insert per member) - and the differential harness lists this as a declared difference.

## [inconsistent] The
  normalization checks its postcondition and reports failure; it does not
  assume the rounding argument.
PROBLEM: Counterexample to "valid always" from the stated API. Every crate-produced rotation (product, slerp) goes through this normalization, including inside `lookup` and `lookup_across`. `LookupError` has six variants and none can report a failed normalization postcondition; `compose` could use `TransformError::NonUnitRotation`, and `inverse` does not need it. So in a lookup the failure must be (a) returned as an out-of-band `Rotation`, breaking the type's stated invariant, (b) misreported as `NonFinite`, or (c) a panic, breaking the no-panic non-negotiable. The document picks none. The failure is empirically unreachable (0 in 1.6e8) but the document itself says the bound is "empirical ... plus a first-order bound sketch" (Limits).
EVIDENCE: DESIGN.md lines 174-181 (`LookupError`: UnknownFrame, Disconnected, NoData, OutOfRange, NoCommonTime, NonFinite); lines 234-235 ("Every rotation the crate produces (product, slerp) passes through the same thresholded normalization"); lines 517-519. Source of the requirement: evidence/codex/round1-position.md:123 ("If normalization cannot establish the band, it returns an error"), which also names no lookup variant.
REPLACEMENT: The normalization checks its postcondition and reports failure; it does not assume the rounding argument. The failure has one spelling per operation family: `TransformError::NonUnitRotation { norm }` from `from_wxyz`, `compose` and decoding, and a `LookupError` variant from lookups (either a new `NonUnitRotation { norm }` or `NonFinite` renamed to a `Numerical` variant covering both; decide before the enum is frozen). It never panics and never returns an out-of-band `Rotation`.

## [incomplete] pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError>;
PROBLEM: The stamp rule for `compose` is nowhere stated. The frame rule is implied by `FrameMismatch`, but `StampMismatch { lhs: Timestamp, rhs: Timestamp }` cannot even express a static operand, so the design implicitly keeps 2.4's rule (a static operand adopts the other's stamp; static with static is static) without saying so. This matters for safety: lookup results carry `At(requested)` even over all-static chains, a hand-built `Stamp::Static` transform composes with a sample from any instant, and with `SameFrameMultiplication` deleted nothing else limits it. AGENTS.md lists the rule as a correctness invariant; the design's AGENTS section says only "composition and its errors" change text.
EVIDENCE: 2.4 rule at src/geometry/transform/mod.rs:529-540. DESIGN.md has no sentence on compose stamps (searched: lines 90, 155, 206, 495). `inverse`'s stamp (unchanged, static stays static) is also unstated.
REPLACEMENT: pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError>;   // requires self.source() == rhs.target(), else FrameMismatch; stamps must be equal unless one operand is Static, else StampMismatch; the result carries the non-static operand's stamp, and Static only when both are Static. `inverse` keeps the stamp.  [plus one line in Registry semantics: "Lookup results carry `Stamp::At(at)`, also over all-static chains, as in 2.4."]

## [overstated] | Lookup argument struct | With labels restored a swapped lookup describes itself; no invariant justifies the type. |
PROBLEM: True for `lookup`. Not true for `lookup_across`, which keeps 2.4's five positional arguments (`&str, Timestamp, &str, Timestamp, &str`). (i) `fixed` is not carried by `CrossTimeTransform` (six accessors, none for the fixed frame), so a wrong fixed frame - including one of the two moving endpoints - returns `Ok` and the result does not describe it. (ii) Swapping `target_time` and `source_time` compiles and the result echoes the swapped query; a consumer that stamps its outgoing message from its own variables sees nothing. The safety critique's closure for this (pairing each time with its frame, stress-critic-safety.md section 5: "which fixes the 2.4 five-positional signature") was dropped without a recorded reason.
EVIDENCE: DESIGN.md lines 138-143 and 96-103; 2.4 signature src/core/registry/mod.rs:439-445; evidence/stress-critic-safety.md lines 167-171. Scenario: `reg.lookup_across("odom", t_now, "base", t_scan, "base")` (fixed = moving frame) and `reg.lookup_across("odom", t_scan, "base", t_now, "map")` (times swapped) both return a finite, plausible `Ok`.
REPLACEMENT: | Lookup argument struct | For `lookup`, labels restored make a swapped call describe itself. For `lookup_across` this does not hold: the result does not carry `fixed`, and swapped times or a moving frame passed as `fixed` return `Ok`. Open in 2.4 as well; recorded as a residual. Cheapest narrowing if wanted: a seventh accessor `CrossTimeTransform::fixed()`, or pairing `(frame, time)` per endpoint. |

## [incomplete] | `clear_history`, automatic reset detection, capacity limits | D5 stands; `Timestamp::MAX` covers the full clear. |
PROBLEM: The row records the rejected features but not the silent hazard they leave. Dependents hazard #2 ("3000 inserts Ok ... later Ok(7.55) blend of 25.1 and −10") is only narrowed by `Expired`: a backwards clock jump smaller than `max_age` still interleaves two epochs and interpolates between them, and a `Registry::unbounded()` registry never returns `Expired` at all, so there the hazard is fully open. The gap-limit row is recorded as "the largest silent hazard left"; this one, with the same downstream evidence (dimos H1, tio H7, rlt Z3), is not recorded as a hazard anywhere in the final text. Also `remove_samples_through(Timestamp::MAX)` clears every dynamic frame, so recovering one edge costs all history; worth one clause.
EVIDENCE: evidence/map-dependents.md table row 2 (line 156) and row 16; evidence/stress-critic-safety.md section 6 ("The silent case is the existing one, hazard #2 ... Neither the draft nor the review closes it") and its last line listing "sub-`max_age` clock jumps" as an open residual. DESIGN.md lines 306-313 state the mechanism but no residual.
REPLACEMENT: | `clear_history`, automatic reset detection, capacity limits | D5 stands; `Timestamp::MAX` covers the full clear (of every dynamic frame at once). Residual, silent: a backwards clock jump smaller than `max_age`, and any jump on an `unbounded()` registry, is accepted and two epochs interpolate into one answer. `Expired` narrows this; it does not close it. |

## [incomplete] | M8 | `Rotation` replaces `Quaternion`; `Vector3` loses its operators | Four positional `f64` replace the `Quaternion { w, x, y, z }` literal both ROS adapters use; any permutation of a unit quaternion passes validation, as it does today. |
PROBLEM: "As it does today" is true of validation but hides that this is a regression in reachability: today both ROS adapters write a named-field literal, which cannot be permuted; v3 moves every user to four positional `f64` in an order (w first) opposite to ROS, SciPy and glam (x, y, z, w). The safety critique rated this Medium-High and gave a closure (a named-field input); the final design rejects it silently - it appears neither here nor in "Not in 3.0". Scenario: `Rotation::from_wxyz(q.x, q.y, q.z, q.w)` with the ROS identity (0,0,0,1) is accepted and is a 180 degree rotation about z.
EVIDENCE: evidence/stress-critic-safety.md section 8, first bullet, and probe E there (`E accepted = true`); ranked amendment 5. evidence/map-dependents.md section 2 as cited there for the struct-literal usage (not re-verified by me).
REPLACEMENT: | M8 | `Rotation` replaces `Quaternion`; `Vector3` loses its operators | A regression in one respect: four positional `f64`, scalar first, replace the `Quaternion { w, x, y, z }` literal both ROS adapters use, and ROS, SciPy and glam order the scalar last. `from_wxyz(q.x, q.y, q.z, q.w)` is accepted for every unit quaternion (the xyzw identity becomes a half turn). The critique's closure, a named-field input, was not adopted; decide here whether to accept the residual or add it. |   [and a "Not in 3.0" row: "Named-field rotation input | Would close positional permutation at `from_wxyz`; costs one type. Residual recorded under M8."]

## [incomplete] `Timestamp::MAX` clears all dynamic
  history, so no `clear_history` is needed (D5 stands).
PROBLEM: 2.4 exports no maximum instant (grep of src/time/timestamp/mod.rs finds no `MAX` constant). Exporting `Timestamp::MAX` creates two idioms one identifier away from a silent outcome: (i) `remove_samples_through(cutoff.checked_add(d).unwrap_or(Timestamp::MAX))` wipes all dynamic history with a `()` return; (ii) `Stamp::At(t.checked_add(latency).unwrap_or(Timestamp::MAX))` inserted under `with_max_age` evicts the frame's whole history and makes every later honest insert `Expired`, and `latest_common_time` then returns `MAX`. (ii) is loud afterwards; (i) is not. The safety critique flagged (i) as needing a doc sentence; the final text has none.
EVIDENCE: evidence/stress-critic-safety.md section 7 ("Residual, Low ... a doc sentence suffices"). DESIGN.md line 109, 311-313.
REPLACEMENT: `Timestamp::MAX` clears all dynamic history, so no `clear_history` is needed (D5 stands). `MAX` exists for this call only; its rustdoc says that it is a cutoff, not a stamp, and that a sample inserted at `MAX` poisons its frame until removed.

## [incomplete] | `target()` / `source()` / `stamp()` | `parent()` / `child()` / `timestamp()` | A sibling lookup has no parent-child relation. `parent`/`child` remain for topology calls and their errors. |
PROBLEM: The document never states which label `insert`, `insert_all` and `reparent` read as the parent. A reader must infer target = parent and source = child from the unchanged argument position. With two vocabularies in one API (`InsertError::ParentMismatch { pinned, offered }`, `Cycle { child, parent }`, `remove_frame(child)` against `Transform::new(target, source, ..)`) this is the one mapping that must be written down; getting it backwards inserts the inverse relationship, which no check can detect (noted as unchanged from 2.4 in the critique). Positions are unchanged from 2.4, so there is no new swap.
EVIDENCE: DESIGN.md lines 73-84, 130-133, 159-168; no sentence maps target/source to parent/child. 2.4: src/geometry/transform/mod.rs:145-151 (`parent: &str, child: &str` in the same positions).
REPLACEMENT: | `target()` / `source()` / `stamp()` | `parent()` / `child()` / `timestamp()` | A sibling lookup has no parent-child relation. `parent`/`child` remain for topology calls and their errors: `insert`, `insert_all` and `reparent` read a transform's `target` as the parent and its `source` as the child, the same argument positions as 2.4's `(parent, child)`. |

## [incomplete] A non-empty
  frame relative to itself is the identity, registered or not.
PROBLEM: Existing 2.4 behaviour, kept, but it is a silent path the document neither weighs nor records: a misspelled or never-published frame queried against itself returns `Ok(identity)` at any instant, with no coverage check, and `latest_common_time` returns `Ok(Static)`. Generic code where target and source come from configuration (`lookup(cfg.base, cfg.sensor, t)` with both set to the same wrong name) or an existence probe by self-lookup gets a confident answer. Related: the document says "A self-named transform is constructible (identity, round trips)" but `Transform::new("a", "a", (9,0,0), ..)` is also constructible, and with `SameFrameMultiplication` deleted it composes on either side of any transform touching `a`, shifting the result while every label still checks. So "every value ... valid" covers numbers, not label/geometry consistency.
EVIDENCE: Probe on 2.4.0: `get_transform("typo","typo",5ns)` on an empty registry = `Ok(("typo", 0.0))`; `latest_common_time("typo","typo")` = `Ok(Static)`; `Transform::new("a","a",(9,0,0),identity,Static)` is `Ok`. In 2.4 `selfnamed * a<-b` is `Ok(10.0)` too, so the compose path is not new; only the right-hand case is newly allowed by deleting `SameFrameMultiplication`. Decision record: evidence/codex/round2-response.md:12 (A6 ACCEPT), no safety argument given.
REPLACEMENT: A non-empty frame relative to itself is the identity, registered or not (2.4 behaviour, kept: a self-lookup proves nothing about a frame's existence, and the rustdoc says so). `Transform::new` with equal names requires nothing of the geometry; a non-identity self-named transform is a caller error no check catches. [Either add both as residuals to "Not in 3.0", or make `Transform::new` reject equal names with non-identity geometry.]

## [unsupported] A slot is reclaimed when it has no edge and no child.
PROBLEM: The lifecycle rule is stated as design but nothing exercised it: the prototype has no removal, no re-parenting, no batch and no reclamation, so interactions (a reclaimed slot's index still held as a child's parent index if reclamation renumbers; name index entry left behind; a slot re-created under the same name after `remove_frame` with a different kind; batch preflight interning nothing on failure) are unverified. I found no public-API wrong answer from the rule as written - ids are private, a slot with a child is retained, a drained frame keeps its edge - but the claim "frame existence is O(1)" plus correct `UnknownFrame`/`Disconnected` diagnosis depends on it. The document's Internals caveat covers timings only ("These support the architecture, not a finished-v3 figure").
EVIDENCE: evidence/stress-proto-hotpath.md:20 ("Batch insert and its atomicity; `reparent`; `remove_frame`; `remove_samples_through`; slot reclamation and the `children` count ..." listed as omitted); evidence/codex/round2-response.md:156 ("Slot lifecycle, transactional interning and the omitted mutation operations need verification before implementation acceptance").
REPLACEMENT: A slot is reclaimed when it has no edge and no child; reclamation never renumbers a live slot. Unverified: the prototype has no removal, re-parenting, batch or reclamation path, so the slot lifecycle and transactional interning need their own property tests (remove/re-add under a new parent and kind; a failed batch leaves `UnknownFrame` answers unchanged) before acceptance.  [and add the same sentence to Limits]

## [nit] `Timestamp` is also `Ord`, `Hash` and `Display`
  (`[-]s.nnnnnnnnn`, computed in integers).
PROBLEM: Two edge cases a naive implementation gets wrong and the "Details that must be specified" list omits: `i64::MIN` (negating overflows; needs `unsigned_abs`) and values in (−1 s, 0) where the whole-seconds part is 0 and loses its sign. A wrong sign in a diagnostic is the misleading-diagnostic class the design removes. Also unspecified: `with_max_age(d)` where `newest − d` is below `i64::MIN` (must mean "nothing expires", not an error or a wrap).
EVIDENCE: evidence/stress-critic-safety.md section 6 ("`Display` of -0.5 s must print `-0.500000000`"); DESIGN.md lines 285-289 list abs_diff, from_secs_nanos and as_secs_nanos only.
REPLACEMENT: `Timestamp` is also `Ord`, `Hash` and `Display` (`[-]s.nnnnnnnnn`, computed in integers on `unsigned_abs`: −0.5 s prints `-0.500000000`, `i64::MIN` prints without overflow). An expiry threshold below `i64::MIN` expires nothing.

NOT CHECKED: Any v3 behaviour: no v3 implementation exists; every v3 statement is a reading of DESIGN.md signatures and prose.; The numerics figures (38-40%, 0 in 1.6e8, 0.0062%, 0.66/0.99/2.65 EPS, 17,426 of 528,047) and hot-path figures: not re-run; outside this check's scope.; Whether thresholded normalization's postcondition can actually fail; only that the design has no reporting path for it in lookups.; Downstream adapter facts (struct-literal quaternion usage, adapters dropping errors, hazard probe values) are taken from evidence/map-dependents.md and stress-critic-safety.md; no dependent source was read.; JSON decode of 2.x data into v3 (field names parent/child vs target/source, Timestamp JSON shape): the design does not specify the v3 schema, so not testable.; The task refers to 9 items under 'Silent wrong answers downstream'; the file's section 4 table has 18 rows under the heading 'Bugs / wrong-answer generators found downstream'. I dispositioned the rows marked silent (1-9, 11, 16, 18) and did not locate a separate 9-item list.; analysis/v3-review/REPORT.md was only grepped (for batch/Expired fallback), not read in full; D1-D6 of review item 10 were not checked against the batch finding.; Embedded/soft-float behaviour and the gate: nothing run.

# AGENT 4
SUMMARY: The API block compiles as written, and every adapter usage in the task could be written against it. A literal stub (no_std + alloc, stated derives, thiserror 2 / hashbrown / libm / optional serde) is clean under clippy pedantic on stable, 1.85 and nightly, in both serde modes, and builds for thumbv6m-none-eabi. The only allows needed were stub artefacts (unused variables, must_use, `# Errors` docs).

No signature is infeasible. The problems are in the prose around the block and in what it leaves unspecified:

- **Wrong counts.** The surface is 78 named items by the map-api method, not "roughly 60". `RegistryError` has 14 variants, not 15. The time generic sits on five types and two traits, not six types.
- **Overstated claims.** The stub cited in Limits was built from Claude's first draft, not this API, and is not in the repository. "Each call's error type lists its own failures" is false for three of the four error types. ROS stamps do not convert "totally" through an `Option`-returning constructor.
- **Missing derives.** `Stamp` loses `Eq` as the derive list is written, and `Timestamp`'s `Ord` needs `Eq` and `PartialOrd` that the list does not name.
- **Unspecified semantics an implementer must guess.** These are `compose`/`inverse` stamp rules, which label is the child, `latest_common_time`'s error set and `Static` handling, signed-time and `Duration` edges in expiry, the serde field names, and `remove_frame` on a root.

Stub and usage file: evidence/verify-api/stub (src/lib.rs, tests/adapter.rs). Nothing in the repository was edited.

## [wrong] **Public surface:** from 105 named items to roughly 60 (counted the way
  [`evidence/map-api.md`](evidence/map-api.md) reaches 105; the exact
  figure depends on M1 to M3).
PROBLEM: Counting the API block the way map-api.md section 5 counts (types, constants, inherent methods, enum variants, public fields) gives 78, not about 60. M1 to M3 move it only between 76 and 80. The Limits bullet repeats the figure.
EVIDENCE: evidence/map-api.md:211-221 gives the method. DESIGN.md:54-181 by that method: 11 types (7 structs, 4 enums); 2 constants (IDENTITY, MAX); 39 methods (Vector3 1, Rotation 6, Transform 8, CrossTimeTransform 6, Timestamp 7, Stamp 1, Registry 10); 21 variants (Stamp 2, TransformError 5, InsertError 8, LookupError 6); 5 public fields (Vector3 3, BatchError 2). Total 78; 76 without M3; 80 with M1 and M2.
REPLACEMENT: **Public surface:** from 105 named items to 78 (counted the way [`evidence/map-api.md`](evidence/map-api.md) reaches 105: 11 types, 2 constants, 39 methods, 21 variants, 5 public fields; 76 without M3, up to 80 with M1 and M2).

And in Limits: "The surface count of 78 is taken from the API block above; the scope critique counted Claude's first draft at 72 and its own minimum at 50."

## [overstated] No
  dependent was built against these signatures; sketches of their adapter
  code compile against a stub
  ([dependents critique](evidence/stress-critic-dependents.md)).
PROBLEM: The cited stub was written from Claude's first draft, which had a different API (`Edge<'a>: Copy`, total `inverse`/`Mul`, unlabelled results). It is not the API in this document, and the stub and sketch files it names are not in the repository.
EVIDENCE: evidence/stress-critic-dependents.md:10-20 ("signature-only stub of the draft API", assumes `Edge<'a>` derives Copy) and :100 ("total `inverse`/`Mul`", "`Edge: Copy`"). `find analysis/v3-consensus -path '*critic-dependents*'` returns only the .md file. The final API does compile under equivalent adapter sketches in my stub (tests/adapter.rs; 1.85, stable and nightly; both serde modes).
REPLACEMENT: No dependent was built against these signatures. Sketches of adapter code compiled against a stub of Claude's first draft ([dependents critique](evidence/stress-critic-dependents.md)); that stub is not this API and was not kept. A stub of the API block above, with equivalent sketches, compiles warning-free under clippy pedantic on 1.85 and stable in both serde modes; nothing in it computes.

## [overstated] Each call's error type lists its own failures.
PROBLEM: Three of the four error types carry variants their callers cannot produce. `InsertError` has three `reparent`-only variants, so `insert` and `BatchError.error` carry three dead arms, and `reparent` can never return `Expired`. `LookupError` is shared: `OutOfRange` and `NonFinite` are unreachable from `latest_common_time`, and `NoCommonTime` from both lookups. `TransformError` is returned by `Rotation::from_wxyz` (two reachable variants of five) and `Transform::new` (two of five). AGENTS.md names this unreachable-arm smell as the reason the buffer errors are split.
EVIDENCE: DESIGN.md:165-168 ("`reparent` only:"), :136-145, :65, :84. evidence/stress-critic-scope.md:110 ("three `InsertError` variants `insert` can never produce ... If kept, give it its own error enum") and :149 ("`OutOfRange`/`NonFinite` are unreachable from one, `NoCommonTime` from the other ... say so in rustdoc").
REPLACEMENT: Each operation family has its own error type. They are not minimal per call: `InsertError` carries three `reparent`-only variants, and `LookupError` serves both lookups and `latest_common_time` (`OutOfRange` and `NonFinite` never come from the latter, `NoCommonTime` never from the former). Rustdoc on each method lists the variants it can return.

## [incomplete] pub fn compose(&self, rhs: &Self) -> Result<Self, TransformError>;
    pub fn inverse(&self) -> Result<Self, TransformError>;
PROBLEM: The document never states several things an implementer needs. (a) The frame precondition and the result labels. (b) The stamp rule and the result stamp, including static with static and static with dynamic. (c) Which names fill `FrameMismatch { expected, found }`. (d) What `inverse` does to labels and stamp, and when it fails; the Numerics section says only that it is total for the rotation. (e) Whether composing a self-named transform with itself is legal; the Numerics example `t.compose(&t)` relies on it.
EVIDENCE: DESIGN.md:90-91, :154-155, :241-252. The only source is AGENTS.md ("requires `lhs.child == rhs.parent`. Timestamps must be equal unless one operand is static"), which DESIGN.md:495 says changes text. `StampMismatch` carries two `Timestamp`s, which implies a static operand never mismatches, but that is nowhere said.
REPLACEMENT: Add under the API block:

"`a.compose(&b)` requires `a.source() == b.target()`, else `FrameMismatch { expected: a.source, found: b.target }`. The result is labelled `(a.target, b.source)`. Stamps: `At(t)` with `At(t)` gives `At(t)`; `At(t)` with `Static` gives `At(t)`; `Static` with `Static` gives `Static`; two different instants give `StampMismatch`. A self-named operand is legal. `inverse` swaps the labels and keeps the stamp; its only failure is `NonFinite` (an inverted translation component overflowing). `Transform::new` returns only `NonFinite` or `EmptyFrameName`; `Rotation::from_wxyz` only `NonFinite` or `NonUnitRotation`."

## [incomplete] pub fn latest_common_time(&self, target: &str, source: &str)
        -> Result<Stamp, LookupError>;
PROBLEM: Four things are unspecified. (a) It can return `Stamp::Static` while `lookup` takes a bare `Timestamp`, so every caller needs an arbitrary-instant arm; the adapter sketch had to invent `Timestamp::from_nanos(0)`. (b) Which variant reports a hop holding nothing; `NoData` is stated only in a critique, and 2.4 used `NoCommonTime { covered: None }`. (c) The meaning of `NoCommonTime`'s four fields. (d) The result for a frame relative to itself, for an unregistered name, and whether the empty-name rule applies.
EVIDENCE: DESIGN.md:144-145, :179; Registry semantics :314-319 covers lookups only. evidence/stress-critic-scope.md:145 ("`NoData` ... Also serves `latest_common_time`'s 'hop holds nothing'"). evidence/stress-critic-dependents.md:52 and :94 ("`Stamp::Static => <instant>` arm at every latest call site survives"). src/core/registry/mod.rs:542-543 shows the same arm in the 2.4 doc example.
REPLACEMENT: Add to Registry semantics:

"**`latest_common_time`** diagnoses `UnknownFrame` and `Disconnected` by the lookup rule. A connecting chain with no dynamic hop, including a frame relative to itself, returns `Stamp::Static`: any instant is served, and the caller picks one. A dynamic hop holding nothing is `NoData { frame }`. `NoCommonTime { frame, earliest, latest, latest_possible }` names a hop whose coverage `earliest..=latest` starts after `latest_possible`, the newest instant the rest of the chain serves. It never returns `OutOfRange` or `NonFinite`."

## [incomplete] `Vector3`, `Rotation`, `Timestamp`, `Stamp`: `Copy`, exact `PartialEq`,
  `Debug`. `Timestamp` is also `Ord`, `Hash` and `Display`
PROBLEM: The derive list is not buildable as stated and silently drops capabilities. `Ord` requires `Eq` and `PartialOrd`, neither listed. `Clone` is implied by `Copy` but not listed. `Stamp` has `Eq` in 2.4 and loses it and `Hash` here with no reason given, although all its fields are `Eq`. No type is `Default`, and `Vector3::zero`, `Stamp::is_static` and `Quaternion::identity()` disappear without appearing in the replaces table.
EVIDENCE: Compile probes against the literal derives: `error[E0277]: the trait bound transforms::Stamp: Eq is not satisfied`; same for `Stamp: Hash` and `Timestamp: Default`. evidence/map-api.md (Stamp: "Derives `Debug, Clone, Copy, PartialEq, Eq`"). src/geometry/vector3/mod.rs:58 (`zero`), src/time/stamp.rs:60 (`is_static`).
REPLACEMENT: - `Vector3`, `Rotation`: `Clone`, `Copy`, exact `PartialEq`, `Debug` (no `Eq` or `Hash`: `f64`).
- `Timestamp`: `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Debug`, `Display` (`[-]s.nnnnnnnnn`, computed in integers).
- `Stamp`: `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Debug`; deliberately no `PartialOrd`/`Ord`.
- No `Default` on any type.

And add to "Deleted without replacement": `Vector3::zero`, `Stamp::is_static`, `Quaternion::identity()` (now `Rotation::IDENTITY`).

## [incomplete] pub fn insert(&mut self, transform: &Transform) -> Result<(), InsertError>;
PROBLEM: Nothing in the document says which label is the child for `insert`, `insert_all` and `reparent`. The implementer must infer target = parent and source = child from the `Transform` doc comment. The replaces-table row says `parent`/`child` "remain for topology calls and their errors" without giving the mapping. For `reparent`, which name of the seed is moved is unstated, as is which frame `NoParent`, `UnknownFrame` and `Expired.frame` name.
EVIDENCE: DESIGN.md:73, :130-133, :159-168, :205. The 2.4 doc states it explicitly (src/core/registry/mod.rs:729: "`t.child()` is the frame to move and `t.parent()` its new parent"); v3 has no equivalent sentence.
REPLACEMENT: Add to Registry semantics:

"**Edge direction.** An inserted transform is the edge whose child is `source()` and whose parent is `target()`. `reparent(seed)` moves `seed.source()` under `seed.target()`. Every `frame` field in `InsertError` is the child (`source()`). `UnknownFrame` means the child has never been mentioned; `NoParent` means it is known only as a root."

## [overstated] Clamped or aliased negative
  ROS seconds: every ROS `(i32, u32)` stamp converts totally.
PROBLEM: The only inbound constructor returns `Option` and rejects `nanos >= 1_000_000_000`, which a ROS `u32` nanosec field can hold. Inbound conversion is therefore fallible at the type level, and the adapter needs an error path. Outbound needs `i32::try_from` on the seconds. "Totally" holds only for normalized stamps.
EVIDENCE: DESIGN.md:112 (`-> Option<Self>`) and :286 ("rejects `nanos >= 1_000_000_000`"). tests/adapter.rs `ros_in` returns `Option<Timestamp>`; `ros_out` needs `i32::try_from(s).ok()?`. `i32 * 1e9 + u32` fits `i64` for all inputs, so the rejection is a choice, not a range limit.
REPLACEMENT: Clamped or aliased negative ROS seconds: every ROS `(i32, u32)` stamp with `nanosec < 1e9` converts exactly, negative seconds included. A denormalized `nanosec` is `None`, and the adapter must handle it. Outbound, `as_secs_nanos` returns `i64` seconds; narrowing to `i32` is the adapter's checked step.

## [incomplete] pub fn with_max_age(max_age: Duration) -> Self;
PROBLEM: `Duration` is used four times and never qualified or imported; presumably `core::time::Duration`. Signed time adds edges the document does not specify. (a) `max_age` above `i64::MAX` ns (`Duration` holds `u64` seconds). (b) `newest - max_age` below `i64::MIN`, reachable now that negative stamps exist, which decides whether `Expired` fires. (c) `Duration::ZERO`, which 2.4 documents. (d) `checked_add`/`checked_sub` with a `Duration` beyond the i64 nanosecond range. (e) `checked_duration_since` when `earlier > self`. The Time section specifies `from_secs_nanos` and `as_secs_nanos` edges only.
EVIDENCE: DESIGN.md:114-116, :127, :285-289, :306-308. src/core/registry/mod.rs:128 (2.4 documents `Duration::ZERO`). evidence/stress-critic-time.md:364 ("`None` iff `earlier > self`; it cannot overflow") did not reach the design text.
REPLACEMENT: Add to Time, "Details that must be specified":

"`Duration` is `core::time::Duration`. `checked_add` and `checked_sub` return `None` when the duration exceeds `i64::MAX` ns or the result leaves the range. `checked_duration_since` returns `None` iff `earlier > self` (computed with `abs_diff`; it cannot overflow). The expiry cutoff is `newest - max_age`, saturating at the minimum timestamp: a `max_age` larger than the representable span expires nothing. `Duration::ZERO` keeps only the newest stamp, as in 2.4."

## [incomplete] `serde` (default-off) on `Vector3`, `Rotation`, `Timestamp`, `Stamp` and
  `Transform`; decoding runs the constructors.
PROBLEM: The wire shape is unspecified where v3 renames or hides fields. `Transform`'s field names are not given (2.x wrote parent/child/timestamp; v3 accessors are target/source/stamp). `Rotation` has private fields, so its shape (struct `w,x,y,z` or a sequence) is open. `Timestamp` is only implied to be a bare i64 by the postcard remark. Acceptance asks for a "frozen serde schema" without stating it. A plain derive on `Rotation` or `Transform` would bypass the constructors, so hand-written or `try_from` decoding is required and should be stated. A non-finite `Vector3` decoded on its own is not rejected (public fields, no constructor check); only `Transform` decoding catches it.
EVIDENCE: DESIGN.md:193-196, :380-382. The stub compiles with a plain derive on all five types, so nothing in the block forces constructor-running decode.
REPLACEMENT: `serde` (default-off) on `Vector3`, `Rotation`, `Timestamp`, `Stamp` and `Transform`. Schema: `Vector3 { x, y, z }`; `Rotation { w, x, y, z }`; `Timestamp` as a bare `i64`; `Stamp` as `{"At": t}` / `"Static"`; `Transform { target, source, translation, rotation, stamp }`. `Rotation` and `Transform` decode through `from_wxyz` and `Transform::new` (`#[serde(try_from = ...)]` or hand-written), never a field-wise derive. A standalone `Vector3` decodes unchecked; finiteness is enforced when it enters a `Transform`.

(The field names are my proposal; the document must pick them.)

## [wrong] Flat `RegistryError<T>` (15 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError`
PROBLEM: `RegistryError` has 14 variants in 2.4, not 15.
EVIDENCE: src/core/registry/error.rs: 14 variants between `pub enum RegistryError` and its closing brace (awk/grep count). evidence/map-api.md:219 ("RegistryError 14, TransformError 9").
REPLACEMENT: Flat `RegistryError<T>` (14 variants for every call), `TransformError` (9), `TimeError`, `QuaternionError`

## [wrong] `<T>` on six public types
PROBLEM: The time parameter is on five public types and two public traits, seven items in all. The document's own Time section lists five types plus "both traits".
EVIDENCE: `grep -rnE '^pub (struct|enum|trait) \w+<' src` outside the buffer: Registry, RegistryError, Stamp, Transform, Point (types); Localized, Transformable (traits). DESIGN.md:270-271.
REPLACEMENT: `<T>` on five public types and two traits

## [unsupported] Same number of types, five fewer public items.
PROBLEM: The figure does not reproduce under either reading. `Quaternion` is 1 type, 4 fields and 7 methods (12 items); `Rotation` is 1 type, 1 constant and 6 methods (8 items): four fewer. If the row's `QuaternionError` (1 type, 2 variants) is included, it is seven fewer items and one fewer type, which contradicts "same number of types".
EVIDENCE: evidence/map-api.md:215-218 (Quaternion 7 methods, 4 fields; QuaternionError 2 variants). DESIGN.md:62-71.
REPLACEMENT: One type fewer (`QuaternionError` goes) and seven fewer public items (15 to 8); four fewer counting `Quaternion` against `Rotation` alone.

## [incomplete] pub fn remove_frame(&mut self, child: &str) -> bool;
PROBLEM: With interned slots for every mentioned name, roots included, the return value for a root or for a frame known only as a parent is ambiguous: a slot exists, an edge does not. 2.4 returns false there because there is no buffer; "Unchanged from 2.4 unless listed" covers it only by implication. Also unstated: that removing a frame with children leaves it as a root slot, and whether `remove_samples_through(Timestamp::MAX)` resets each frame's expiry anchor (the running newest). The `Expired` recovery depends on that.
EVIDENCE: src/core/registry/mod.rs:720-725 (`self.data.remove(child).is_some()`). DESIGN.md:332-335. evidence/stress-critic-dependents.md:107 ("Unstated in the draft: that the clear also resets each frame's expiry anchor").
REPLACEMENT: Add to Registry semantics:

"`remove_frame(child)` returns `true` iff `child` had an edge to a parent, which it removes with its history. A root, a name known only as a parent, an unknown name and the empty name return `false` and change nothing. Children of a removed frame keep their pins, so the name stays as a root. `remove_samples_through` also resets the expiry anchor of every frame it empties: after `Timestamp::MAX`, any stamp is accepted again."

## [incomplete] A non-empty
  frame relative to itself is the identity, registered or not.
PROBLEM: Three gaps. (a) The result's stamp is unstated: `Stamp::At(at)`, as for every lookup including all-static chains, is inherited from 2.4 only by the "unchanged" clause, and a v3 implementer with a bare static isometry could plausibly return `Static`. (b) For `lookup_across`, it is unstated whether `target == source` at two different times still requires `fixed` to be known and connected, or whether the "registered or not" identity applies. (c) `CrossTimeTransform` has no `fixed` accessor, which is fine but should be said to be deliberate.
EVIDENCE: DESIGN.md:314-319, :94-103. AGENTS.md: "Results always carry the requested timestamp (also over static chains)".
REPLACEMENT: A non-empty frame relative to itself at one instant is the identity, registered or not. Every `lookup` result carries `Stamp::At(at)`, also over an all-static chain. In `lookup_across`, the shortcut applies only when `target == source` and `target_time == source_time`; otherwise both legs to `fixed` are resolved and `fixed` must be known.

## [nit] pub fn from_secs_nanos(secs: i64, nanos: u32) -> Option<Self>;   // M3
    pub fn as_secs_nanos(self) -> (i64, u32);                        // M3
PROBLEM: `from_nanos`, `as_nanos` and `Stamp::at` are `const fn`; these two and the three `checked_*` methods are not, although all are const-feasible on 1.85 with integer arithmetic. Either the omission is deliberate, and should be said, or it is an inconsistency. `Rotation`'s accessors are likewise const-feasible; `from_wxyz` is not (`libm::sqrt`).
EVIDENCE: The stub compiles `const` items using `Vector3::new`, `Timestamp::from_nanos`, `as_nanos`, `Stamp::at` and `Rotation::IDENTITY` on 1.85 (tests/adapter.rs).
REPLACEMENT: pub const fn from_secs_nanos(secs: i64, nanos: u32) -> Option<Self>;   // M3
    pub const fn as_secs_nanos(self) -> (i64, u32);                        // M3

(and `const` on `checked_add`, `checked_sub`, `checked_duration_since`, `Rotation::{w,x,y,z}`), or add the sentence: "Only the functions marked `const` are promised const."

## [nit] Struct variants are
  `#[non_exhaustive]`.
PROBLEM: The consequences are unstated and the rule is uneven. Downstream code cannot construct any struct variant, so an adapter cannot fabricate a `LookupError::OutOfRange` to unit-test its own wait classifier, and every pattern needs `..`. `BatchError`, a public-field struct, is not `#[non_exhaustive]`, so it is constructible and exhaustively destructurable: adding a field later is a major break. The unit variants (`NonFinite`, `EmptyFrameName`) remain constructible.
EVIDENCE: Compile probes: `error[E0639]: cannot create non-exhaustive variant using struct expression`; `error[E0638]: .. required with variant marked as non-exhaustive`. `BatchError { index: 0, error }` compiles downstream.
REPLACEMENT: Struct variants are `#[non_exhaustive]`: downstream patterns need `..` and downstream code cannot construct them (tests of an adapter's error handling must provoke the error from a `Registry`). `BatchError` is deliberately a plain two-field struct, constructible and exhaustively matchable; adding a field is a breaking change.

## [nit] target: impl Into<String>,
        source: impl Into<String>,
PROBLEM: This compiles and accepts `&str`, `String`, `&String`, `Cow<str>` and `Box<str>`. It also accepts `char` (`'c'` becomes the frame "c") and rejects `&&str`, which is what iterating `&[&str]` yields. A borrowed name allocates on every construction, including for a transform that `insert(&Transform)` then only reads. That is the stated design, but the claim elsewhere of "no allocation" applies to the resolver only.
EVIDENCE: tests/adapter.rs `build` (compiles with all five kinds and with `'c'`). Probe: `error[E0277]: the trait bound String: From<&&str> is not satisfied`.
REPLACEMENT: No signature change needed. Add to the prose: "`Transform::new` allocates both names when given borrowed strings; an adapter converting messages pays two allocations per transform before `insert`. `impl Into<String>` also admits `char`."

NOT CHECKED: Any behaviour: every stub body is todo!(). Numerics claims (8-EPS band, inverse accuracy, the 0.0062% and 38-40% figures), timings and the differential-run counts were not re-run; they are outside this check.; Whether constructor-running serde decode round-trips bit-exactly (the fixed-point claim for `Rotation`). The stub's serde is a plain derive; the roundtrip function is compiled, not executed.; riscv32imc, thumbv7em and thumbv8m builds of the stub (only thumbv6m was built), and the repository gate (tests/test_all.sh), which does not apply to a stub.; Clippy with `missing_docs` on, and the lints the stub suppresses (`must_use_candidate`, `missing_errors_doc`), as the task directed.; The dependents' real adapter code: the sketches are mine, modelled on the task list, not on dimos, roslibrust or transforms_io sources.; Downstream claims in the replaces table ("Every studied user sets a window; none uses `new`", "two of four hand-rolled inbound conversions are wrong") were not traced to evidence/map-dependents.md.; Whether `t = t.compose(&t)` reaches infinity in exactly 1024 products; it depends on the starting translation and on the unspecified legality of self-composition.; Whether `evidence/codex/` records the claimed third-round sign-off on this exact API text.
