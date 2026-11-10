# Open questions, trade-offs and considerations

Date: 2026-11-06. Baseline: master `f9770d2` (released v2.3.0 plus
docs-only commits).

This file collects what the dependent study leaves for the maintainer to
decide. It covers the three dependent reports and their index
([`README.md`](README.md)), revision 4 of the
[2.4 proposal](../v2.4-proposal/DESIGN.md), the
[dependents addendum](../v3-proposals/transforms-v3-addendum-dependents.md)
to the v3 notes, and the edits to the
[v3 warrant report](../v3-warrant/REPORT.md). Nothing here is decided
unless it is marked decided. The evidence lives in the linked sections,
not here; this file only points to it.

## Decisions for the maintainer

Already settled in the 2.4 proposal: D1 and D4 were withdrawn in
revision 3, and D5 (reject empty names in 2.x) was decided on 2026-11-06.
D3 (the name `transform_positions`) matters only if Q4 is Yes.

| ID | Question | Options | Recommendation | Where argued |
|---|---|---|---|---|
| Q1 | 2.4 D6: where is the empty-name check enforced? | (a) Insert boundary: `Buffer::check`, flat `RegistryError::EmptyFrameName`. (b) `Transform::validate`: `TransformError::EmptyFrameName` | (a). The hazard is registry-only, the self-reference rule lives there, decoding is untouched, and the identity clause needs no carve-out | [DESIGN §5a](../v2.4-proposal/DESIGN.md#5a-part-d-reject-empty-frame-names), [§8](../v2.4-proposal/DESIGN.md#8-open-decisions-for-the-maintainer); [T1](#t1-insert-boundary-or-validate-for-empty-names) |
| Q2 | 2.4 D7: payload of `EmptyFrameName` | (a) Unit. (b) Name the empty side | (a), as `SelfReferentialFrame`. Frame fields for all insert errors wait for the v3 row (Q11). If Q11 drops the row, D7 should say it defers to nothing | [DESIGN §8](../v2.4-proposal/DESIGN.md#8-open-decisions-for-the-maintainer) |
| Q3 | 2.4 D8: a property test for the rule | (a) None. (b) Map one name of the `f0..f6` alphabet in `tests/properties.rs` to `""` | (a). The check is a string comparison and §7 covers every insert path | [DESIGN §7](../v2.4-proposal/DESIGN.md#7-tests), [§8](../v2.4-proposal/DESIGN.md#8-open-decisions-for-the-maintainer) |
| Q4 | 2.4 D2: grant Part B (`transform_positions`) a scope exception | Yes / No | No for 2.4. No studied dependent uses `Point`, `Transformable` or `Localized`. Revisit on demonstrated need | [DESIGN §4.1](../v2.4-proposal/DESIGN.md#41-scope-decision-required); [README](README.md#transforms-2x-code) |
| Q5 | If D2 is refused: does Part C item 6 (a `Transformable` recipe) still ship? | (a) Ship the recipe. (b) Drop it too | No recommendation. DESIGN keeps item 6 as the fallback. The revision-4 review notes that the same "strictly necessary" lens may cut it too: no studied dependent uses `Transformable` | [DESIGN §5](../v2.4-proposal/DESIGN.md#5-part-c-documentation-corrections) item 6 |
| Q6 | The `AGENTS.md` edit for Part D | (a) Add "no frame name is empty" beside "a frame cannot be its own parent" (`AGENTS.md:148`), as a tree rule checked at insert. (b) No edit | (a). Under Q1 (a) no other bullet changes. Under Q1 (b) the validation list and the identity clause also change | [DESIGN §9](../v2.4-proposal/DESIGN.md#9-release-obligations) |
| Q7 | Part D's README clause ("Registry insertion also rejects an empty frame name.") | Keep / cut | Keep, as §5a proposes. The revision-4 review asked whether the lens would cut it but did not argue for cutting | [DESIGN §5a](../v2.4-proposal/DESIGN.md#5a-part-d-reject-empty-frame-names) "Docs" |
| Q8 | Is the `try_now` rewording (Part C item 9) enough for `wasm32-unknown-unknown`? | (a) Rewording only; `AGENTS.md` unchanged. (b) Also edit `AGENTS.md:119-122` ("The only documented panic is …"). (c) Remove `now`/`try_now` on that target | (a), per DESIGN revision 4: the trap is std's, and no crate panic is added. The index (rec 9) and the roslibrust report still flag (b). (c) got 0 of 3 votes | [DESIGN §5](../v2.4-proposal/DESIGN.md#5-part-c-documentation-corrections) item 9; [README rec 9](README.md#transforms-docs); [addendum](../v3-proposals/transforms-v3-addendum-dependents.md#considered-and-not-added) |
| Q9 | Kind-switch text (Part C item 8): how wide? | (a) One sentence on the `StaticDynamicConflict` variant. (b) Also on `add_transform`, plus an optional README tf2-table row | (a), as DESIGN item 8. Every document calls the README row optional; `AGENTS.md` says to keep the README brief | [DESIGN §5](../v2.4-proposal/DESIGN.md#5-part-c-documentation-corrections) item 8; [README rec 3](README.md#transforms-docs), [placement table](README.md#placement-disagreements-and-how-they-were-resolved) |
| Q10 | Rec 12 (rotations from looser sources) and rec 16 (negative seconds before the epoch): in or out of 2.4? | In / out | Out. Rec 12 is loud and the bound is already stated; rec 16 affects `Display` only, for one unmerged user. `dimos.md` still lists rec 16 as "Decide" under 2.x code | [DESIGN §6](../v2.4-proposal/DESIGN.md#6-considered-and-not-proposed); [`dimos.md`](dimos.md#decision-negative-seconds-for-pre-epoch-systemtime) |
| Q11 | v3: keep the new row "Insert errors name the rejected transform's frames"? | Keep / drop | Keep. Two of three judges; all four users hit it; D7 defers to it. One judge dissented: loud diagnostics only | [addendum, rows added](../v3-proposals/transforms-v3-addendum-dependents.md#insert-errors-name-the-rejected-transforms-frames); [T3](#t3-ledger-as-record-or-wishlist) |
| Q12 | v3: the row's clause "`ReparentingNotSupported` gains its child" | Keep / drop | Keep, per the addendum: a batch member rejected this way is as ambiguous, and the vocabulary row already breaks this variant. It was added in judging, beyond the study's shape, and the ballots are not in the repository | [addendum, "Added in judging"](../v3-proposals/transforms-v3-addendum-dependents.md#insert-errors-name-the-rejected-transforms-frames) |
| Q13 | Warrant report: qualify the 2.0-migration bullet ("the two active dependents absorbed it within weeks") | (a) Qualify it as the Pros line now is (the maintainer wrote roslibrust_transforms' migration). (b) Remove the qualification from Pros too. (c) Leave as is | No recommendation. No majority flagged the bullet; the Pros line was qualified on its own | [REPORT, what the dependents show](../v3-warrant/REPORT.md#what-the-dependents-show); [addendum corrections](../v3-proposals/transforms-v3-addendum-dependents.md#corrections-to-the-warrant-report) |
| Q14 | Put the re-broadcast evidence (or a pointer) in `transforms-v3-addendum-cross-time.md` | Add the evidence / add a pointer / leave | No recommendation. README rec 6 and the roslibrust report's v3 item 1 both name that file, and it has nothing today. The addendum's review called putting the evidence only in the new addendum and the table cell acceptable, but noted the missing pointer | [README rec 6](README.md#v3-ledger); [`roslibrust_transforms.md`](roslibrust_transforms.md#v3-ledger) |
| Q15 | The warrant report's "deprecated surface accumulates" trigger is unlikely to fire under the 2026-11-03 stance | Reword the trigger / leave it as a note | No recommendation. The addendum records it as a note, not a ledger edit | [addendum, triggers](../v3-proposals/transforms-v3-addendum-dependents.md#does-any-trigger-fire); [REPORT](../v3-warrant/REPORT.md#recommendation) |
| Q16 | Does the maximum interpolation gap stay "design doc first"? | (a) Stay. (b) Write the design doc now. (c) A v3 default bound | (a). Every document keeps it; the only designed fix is an additive per-frame bound, and a default bound has no defensible value. Outreach item dimos-5 is the route to demand evidence | [DESIGN §6](../v2.4-proposal/DESIGN.md#6-considered-and-not-proposed); [`dimos.md`](dimos.md#design-doc-maximum-interpolation-gap); [T8](#t8-the-maximum-interpolation-gap) |
| Q17 | Outreach: who files what, and in what order | See [README, outreach](README.md#outreach) | The README rank gives an order: the adapter halves of ranks 2 and 3 (tio-4, rlt-9 and dimos-2; tio-5 and rlt-8), tio-1 and rlt-3 (rank 4), tio-2 (5), rlt-2 (6), tio-6 and rlt-5 (7), the dimos standalone PR (8), then ranks 11 to 14. Who files: no recommendation. Once Part D ships, tio-3 lapses and rlt-7 shrinks to the leading-`/` question | [README ranked table](README.md#recommendations), [outreach](README.md#outreach); [T5](#t5-outreach-or-core-changes) |
| Q18 | Commit the probe package's `Cargo.lock`? | (a) Leave it untracked, as the README says. (b) Commit it. (c) Ignore it in `.gitignore` | No recommendation. Note: `.gitignore` covers `target/` only, so after a probe run a broad `git add analysis/` picks the lock file up | [README, probes](README.md#probes) |
| Q19 | Refresh superseded text in the reports before this branch merges? | (a) Edit each to match DESIGN revision 4 and the ledger. (b) Add "superseded" notes. (c) Leave; the README keeps its recommendation text "as written" | No recommendation. The list is under [Considerations](#superseded-text-in-the-reports) | — |

## Trade-offs

### T1. Insert boundary or `validate` for empty names

- **Insert boundary (DESIGN's choice).** The join happens only in chain
  resolution, so the check sits where the hazard is. It sits beside the
  self-reference rule. Serde decoding, constructors and the identity
  clause stay as they are. The error stays flat.
- **`validate` (the three reports' shape).** A `Transform` naming `""`
  could not exist at all, which is closer to "invalid states made
  unrepresentable". It fails at construction, nearer the source. The cost:
  stored data with `""` stops decoding, two `AGENTS.md` invariants change,
  and the registry reports a tree rule through the wrapped
  `TransformError` arm.
- **What would tip it.** Evidence that a caller composes `Transform`s by
  hand outside the registry with unset names would favour `validate`:
  `Mul` also matches names. transforms_io and roslibrust_transforms never
  call `Mul`; dimos and other users were not checked for it.
- Argued in [DESIGN §5a](../v2.4-proposal/DESIGN.md#5a-part-d-reject-empty-frame-names)
  ("Not the dependent reports' shape") and
  [`dimos.md`](dimos.md#2x-reject-empty-frame-names).

### T2. A minimal 2.4 or every hazard documented

- **Minimal.** Revision 4 asks of each item "is this strictly necessary
  for 2.4?". §6 leaves out the rotation-tolerance sentence, negative
  seconds and the gap bound. Part C item 8 takes rec 3's sentence on the
  variant without its `add_transform` sentence or README tf2 row.
- **Every hazard.** The study found silent hazards in shipped dependents.
  `AGENTS.md` calls documentation drift a bug. A dropped sentence is cheap
  and the hazard it names is real.
- **What would tip it.** A user report for any dropped item. Apart from
  the gap bound (T8), what is left out is loud, `Display`-only, or a
  second placement of a hazard item 8 already documents; the silent ones
  (Part D and Part C items 2, 5 and 8) are in.
- Argued in [DESIGN §6](../v2.4-proposal/DESIGN.md#6-considered-and-not-proposed)
  and [README, transforms docs](README.md#transforms-docs).

### T3. Ledger as record or wishlist

- **Record.** A row keeps a break-only fix from being lost, and it gives
  2.4 decisions a place to defer to (D7 needs the new row).
- **Wishlist.** Every Low row is carried into a batched v3 and adds to
  the migration bill. The new row buys diagnostics only; one judge voted
  against it on that ground. The `max_age` row gained evidence that it
  closes no studied hazard.
- **What would tip it.** A rule for when a Low row may enter, for example
  "needs a break, evidence from a real user, and a 2.x decision that
  defers to it". The addendum applies the first two.
- Argued in [the addendum](../v3-proposals/transforms-v3-addendum-dependents.md#rows-added)
  and [REPORT, the table](../v3-warrant/REPORT.md#what-genuinely-requires-a-break).

### T4. Reachability or observed occurrence as evidence

- **Reachability.** Probes show silent wrong answers one call away in
  shipped code. The crate's first concern is exactly that failure.
- **Occurrence.** No user has reported any of them. The v3 notes say
  reachability is not demand (`transforms-v3-3.md:229-230`), and the
  ledger keeps the cross-time row at Medium for that reason.
- **The tension in practice.** Part D ships on reachability plus
  recurrence (two exposed, two hand-roll), with no user bitten. The gap
  bound does not ship on reachability in all three studied users. The
  difference the documents give: Part D is a cheap rejection; the gap
  bound needs a design and a `latest_common_time` change.
- **What would tip it.** One observed incident, for example a roslibrust
  caller re-inserting a `get_transform_at` result, which would also fire
  a v3 trigger.
- Argued in [README, do not](README.md#do-not) and
  [the addendum](../v3-proposals/transforms-v3-addendum-dependents.md#does-any-trigger-fire).

### T5. Outreach or core changes

- **Outreach.** The thin adapters own most of their hazards: ROS time
  clamping, `now()` as "latest", backwards parameter names, dropped
  kind-switch inserts. Fixing them in the adapter keeps the core
  minimal, per `AGENTS.md` Scope.
- **Core.** Outreach depends on other maintainers. transforms_io has
  never had an issue or PR on Codeberg. roslibrust_transforms has been
  idle since 2026-09-25, and the transforms maintainer wrote its 2.x
  migration. Nothing has been filed. A hazard left to outreach stays
  live until someone acts.
- **What would tip it.** Outreach that goes unanswered, or a third
  integration that clamps ROS time while negative stamps are shown to
  occur (the README's stated reopening condition for core ROS time
  conversion).
- Argued in [README, outreach](README.md#outreach) and
  [README, do not](README.md#do-not).

### T6. v3 timing or re-export-chain cost

- **Sooner.** "It will never be cheaper": transforms_io re-exports six
  `transforms` types and pelorus_syncio five again, and each new
  downstream lengthens the chain.
- **Later.** No trigger has fired. A third major in about seven months
  costs trust. The study raised v3's cost and made nothing more urgent.
- **What would tip it.** A trigger in
  [REPORT, recommendation](../v3-warrant/REPORT.md#recommendation), or a
  new downstream that re-exports `transforms` types.
- Argued in [REPORT, cons](../v3-warrant/REPORT.md#cons) and
  [README rec 19](README.md#v3-ledger).

### T7. Typed errors or stringification in dependents

- **Typed (outreach tio-7, rlt-1).** Waits could tell a permanent
  failure from a transient one, and callers keep the payloads.
- **Strings (today, three of four).** Their waits are error-blind. But
  error-shape breaks in v3 cost them nothing beyond new `Display` text,
  which the README declares unstable.
- **The tension.** Each dependent that starts matching variants raises
  the cost of every ledger row that reshapes a variant (the new row, the
  vocabulary row, the precedence flip).
- **What would tip it.** How soon v3 is expected. If soon, typed
  matching is worth asking for after v3 lands.
- Argued in [README, hand-rolled](README.md#hand-rolled-around-the-crate)
  and [the addendum](../v3-proposals/transforms-v3-addendum-dependents.md#rows-added)
  ("Cost to the studied users").

### T8. The maximum interpolation gap

- **For acting.** It is the largest silent hazard without a
  recommendation. It is reachable in all three studied users
  (`probes/transforms_io.rs` P9, `probes/roslibrust_transforms.rs` P10,
  dimos Hazard 2). It cannot be built exactly outside: it needs sample
  access the API does not expose. dimos lost its tolerance in migration.
- **Against.** Nobody has asked. The designed fix, a per-frame bound, is
  additive, so it is never a ledger row. A default bound would break and
  has no defensible value. It also changes how `latest_common_time` is
  computed.
- **What would tip it.** dimos's answer to dimos-5 (which per-edge
  tolerances they would set), or a reported incident.
- Argued in [`dimos.md`](dimos.md#design-doc-maximum-interpolation-gap)
  and [DESIGN §6](../v2.4-proposal/DESIGN.md#6-considered-and-not-proposed).

## Considerations

### Evidence that was not verified, and its limits

- No dependent's code was built or run. The probes copy the cited lines
  against this checkout ([README](README.md#probes)).
- The `tests/test_all.sh` gate ran on no prototype. The 2026-10-30
  empty-name prototype was not saved.
- The 2026-11-06 dimos refresh had no evidence-skeptic pass. `dimos.md`
  rests on its 2026-10-30 verification and `probes/dimos.rs`.
- The 2026-10-30 dimos probes O1 to O9 were not saved. O4 (epoch mixing
  on `new()`), O7 (ghost-to-ghost identity) and O9 (the cutoff-recipe
  hole) are reproduced by no saved probe.
- dimos Hazard 5 (lookup re-validation rejecting a valid long chain) is
  reasoned from code, not probed.
- tf2 behaviour was read from `geometry2` source, not run. `tf2_ros`
  `onTimeJump`, `cache.cpp` (`TF_OLD_DATA`) and the static broadcaster
  lines are cited second-hand in places.
- The wasm trap was seen under node 24 only. The cause is std source, so
  other hosts should match; none was run.
- Not checked: ROS 2, zenoh and hiroz `/tf_static` QoS in roslibrust;
  whether `static_transform_publisher` normalizes quaternions; whether
  minot replay rewinds stamps (H7 through minot is plausible only); what
  remote tf2 listeners do with a re-broadcast duplicate stamp; whether
  roslibrust receives its own `/tf` publications.
- How often hand-typed or rounded quaternions occur in real deployments
  is unmeasured (transforms_io H8, roslibrust Z5).
- whirl_tf's failure to build on 2.x is by reading, not by building.
  `transforms_io_hiroz` was not read file by file.
- Not searched or not read: the stelzo/pelorus Codeberg repository (the
  API returned nothing), GitLab, Codeberg beyond transforms_io and minot,
  GitHub's "used by" graph, PyPI dependents, private code. GitHub code
  search indexes default branches only, so users on branches (as dimos
  is) are found only by other routes.
- termviz2 and transforms-py (both 1.x) were not studied.
- "Not examined" cells in the README matrices (dimos on the `/tf` versus
  `/tf_static` and kind-switch rows; whirl_tf on those and the
  broadcaster, normalization and cross-time rows) are gaps in the
  reports, not negative findings.
- Some dimos facts come from earlier documents and were not re-read:
  that dimos is not a crates.io crate (`DISCUSSION.md`) and the fork's
  clone-per-call cost.
- Download counts are a snapshot and drift daily (942 for the 30 days to
  2026-11-06; a later sum gave 944). They cannot be attributed to users.
- The judges' ballots for the addendum are not kept in the repository.
  Vote counts are stated, not checkable.
- The 15 to 25 percent odds for dimos #4722 are an estimate.

### Risks of acting

- **MIGRATION wording for Part D.** The three reports' text says stored
  `""` data "stops decoding". That is true only of the `validate` shape.
  Under the insert-boundary shape, MIGRATION must say decoding still
  accepts `""` and insertion rejects it
  ([DESIGN §9](../v2.4-proposal/DESIGN.md#9-release-obligations)).
- **Serde data still decodes `""`.** A replayed log or snapshot that
  names `""` decodes and then fails at insert. That is loud, but a
  replay that treated every insert as infallible now drops data.
- **semver-checks cannot see `Ok` → `Err`.** It sees one added variant on
  a `#[non_exhaustive]` enum. The behaviour change rests on the D5
  ruling alone. transforms_io (`^2.2`) and roslibrust_transforms
  (`^2.1`) pick it up on their next fresh build without a release of
  their own.
- **Log volume.** transforms_io's r2r, ros2-client and hiroz backends log
  an error per rejected message. A publisher that leaves `frame_id`
  unset turns into one error line per message after 2.4
  ([README corrections](README.md#corrections-to-earlier-analysis),
  `reparent-frame-value/REPORT.md:66` row).
- **Outreach before 2.4 ships** asks adapters to hand-roll an empty-name
  check that 2.4 then makes redundant (tio-3, rlt-7). Waiting instead
  leaves the hazard live until 2.4 is released.

### Superseded text in the reports

These statements predate DESIGN revision 4 or the new ledger row. None
was edited here (Q19).

- `dimos.md:32` and `:373-374`: "2.4 is now Part B plus Part C".
  Revision 4 adds Part D.
- `dimos.md:98-99`: "DESIGN.md mentions it only in passing (§9)". It is
  Part D.
- `dimos.md:102` and `:460`: the frame-names row "is not there yet";
  "the action is to add the row". It was added on 2026-11-06; the same
  file says so at `:32-34`.
- `dimos.md:370-371`: no "maintainer decision". D5 was decided.
- The empty-name "Needs" or "Costs" bullets assume the `validate` shape
  and a possible break: `README.md:263-271`, `dimos.md:394-398`,
  `transforms_io.md:600-607`, `roslibrust_transforms.md:554-560`.
- "Add the row" items: `transforms_io.md:666-669`,
  `roslibrust_transforms.md:617`.
- `README.md:582-583` (placement table): the empty-names fallback is
  moot after D5, and "D2 recommends the exception" is now No.
- `transforms_io.md:67`: pelorus downloads "not fetched". The README has
  16 / 16.

## Already resolved

Flagged as open in earlier runs, closed since. Do not re-open.

- The wasm probe was "scratch only" in `roslibrust_transforms.md`;
  it is [`probes/wasm_try_now.rs`](probes/wasm_try_now.rs), and the
  probes table says so.
- The index's corrections table said the older files were not edited
  and the frame-names item had no ledger row; both now point at the
  [v3 addendum](../v3-proposals/transforms-v3-addendum-dependents.md).
- **Is rejecting `""` a break?** No: the maintainer ruled it
  non-breaking on 2026-11-06
  ([DESIGN D5](../v2.4-proposal/DESIGN.md#8-open-decisions-for-the-maintainer)).
- **Empty names and clock jumps missing from DESIGN.** Added as Part D
  and Part C item 5
  ([DESIGN, revision 4](../v2.4-proposal/DESIGN.md#5a-part-d-reject-empty-frame-names)).
- **The frame-names row was not in the ledger table.** Added
  ([REPORT table](../v3-warrant/REPORT.md#what-genuinely-requires-a-break)).
- **DESIGN D7 said the candidate was "not yet in the ledger table".** It
  now links the row
  ([DESIGN §8](../v2.4-proposal/DESIGN.md#8-open-decisions-for-the-maintainer)).
- **DESIGN D2 recommended the exception; dimos.md deferred.** D2 now
  recommends No and the README says the two agree
  ([README](README.md#transforms-2x-code)).
- **Three stale premises in the warrant report** (dependents match no
  variant; 385 downloads a month; dependents do not use the changed
  parts). Corrected in place with dated notes
  ([addendum](../v3-proposals/transforms-v3-addendum-dependents.md#corrections-to-the-warrant-report)).
- **The required-`max_age` row implied it closed the gap hazard.** The
  cell now says it does not
  ([addendum](../v3-proposals/transforms-v3-addendum-dependents.md#transformerrort-sealed-point-required-max_age)).
- **The transforms_io author's email domain in the Status table.**
  Removed; `transforms_io.md:115` cites only the pelorus homepage.
- **The wasm probe lived only in scratch.** Saved as
  [`probes/wasm_try_now.rs`](probes/wasm_try_now.rs) with build steps
  (one stale table cell remains; see above).
- **roslibrust outreach item 1's fail-fast condition** and **item 6's
  "within tf2's bound"**. Both now match the index
  (`roslibrust_transforms.md:635-640`, `:657`).
- **README-row versus rustdoc for recs 3 and 12.** Resolved to rustdoc,
  README row optional; all reports agree
  ([placement table](README.md#placement-disagreements-and-how-they-were-resolved)).
- **dimos dormancy dates.** `dimos.md` now separates the PR's last
  commit (2026-10-27) from the stack's (2026-10-28).
- **Broken links to `README.md` and sibling reports.** All files exist.
- **pelorus downloads.** Fetched: 16 / 16
  ([README](README.md#who-uses-the-crate)).
