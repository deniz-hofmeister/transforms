# Review of `Registry::reparent_frame` on `feature/reparenting`

Review date: 2026-09-20. Branch: `feature/reparenting` at `bb75fa9`,
working tree clean apart from the untracked analysis files.
Reviewer: Claude Fable 5.1 (model id `claude-fable-5-1`), running in the
maintainer's Claude Code session. Requested by the maintainer through
Codex, against the brief in `/tmp/reparent-fable-review-brief.md`.

Inputs: AGENTS.md; the branch diff against `origin/master` (local
`master` is one merge behind: the v3 proposals landed on origin through
PR #110); the branch history from `80242d6` to `bb75fa9`; the four
analysis documents; the a7e7cd3 scar commit; and the verification runs
in the last section. REPORT.md is the 2026-09-19 eighteen-agent debate
the maintainer attributes to Claude Fable. Where this review changes its
conclusions, it says so explicitly.

## Summary

**Proceed.** Merge the branch as 2.2.0 after the three documentation
fixes under "Fix before merge". No code defect was found. The
implementation matches its documented contract on every point the brief
lists; the verification runs below reproduce that independently.

The recommendation is a change from REPORT.md's "defer, merge-ready".
Two things changed: TF2_RESEARCH.md established that deliberate
re-parenting is a real, implemented ecosystem operation rather than a
once-a-decade fault, and the maintainer stated the product goal and
accepted the history-discarding temporal model. One thing did not change,
and the review says so plainly in the counterargument: no caller of this
crate has asked for the call, and prevalence among its users is
unmeasured. The demonstrated-need rule does not require that; it requires
a need that is demonstrated and concrete rather than speculative, and the
domain evidence together with the substitute's failure modes meets that
reading (addendum, D1).

## 1. Proceed, narrow, redirect, or defer

### Under the scope rules

The AGENTS.md placement test has three branches. This feature sits on the
"core or nowhere" branch, and every analysis, including the devil's
advocate in REPORT.md, concedes it. The review re-verified it in the code:

- The cycle check walks pinned parents that `Registry` does not expose.
  A caller cannot ask for a frame's parent, its ancestry, or whether one
  frame descends from another; lookups only report connectivity.
- Failure preservation needs the sealed buffer. The substitute,
  `remove_frame` then `add_transform`, is a bare map removal followed by
  an insert whose cycle check runs only for an absent child. It discovers
  a cycle or an invalid seed only after the buffer and its history are
  gone, and a root named only by that edge vanishes with it. A kind change
  it does not discover at all: the re-add decides the new buffer's kind
  from the seed, so an opposite-kind seed is accepted silently.

"Nowhere" would leave the substitute as the documented route. Whether the
substitute is "merely verbose" or "a wrong-answer generator" is the
question REPORT.md left to the maintainer, and the honest answer is: it is
mostly loud but destructive, with one silent path. A caller who re-adds a
formerly dynamic frame with a static seed gets a frame that answers every
instant, the a7e7cd3 failure class, with no error anywhere. That needs an
opposite-kind seed, which the ROS attach idiom (tracked object becomes a
static child of the gripper) produces naturally. `reparent_frame` refuses
that seed with `StaticDynamicConflict`; the kind change remains available
through removal, but only as a deliberate second act.

### Under the demonstrated-need rule

The rule, "additions follow a demonstrated, concrete need", is written
against speculation ("a downstream user *might* want it"). It does not say
the need must come from a user of this crate, and reading that in would
add a rule rather than apply one (Codex's point, accepted in the
addendum). What was true on 2026-09-19 and is true today is narrower: no
user of this crate has asked, no external code handles the rejection, and
of three reverse dependencies one pins a 2.0 beta. That is unmeasured
prevalence, not an unmet rule.

What the new evidence changes is the *character* of the missing demand.
REPORT.md's case against rested on the operation being rare and
fault-like in the domain. TF2_RESEARCH.md shows a 2013 upstream decision
to support it, a released tool built around it, a Rust application that
implements a cycle-checked `reparent_frame` on its own frame store, and
REP-105 assigning the operation to the localization authority. The
operation is a domain operation. The remaining gap is between "the domain
does this" and "a consumer of this crate's registry needs a verb for it",
and that gap is what the maintainer's statement closes: a "properly
featured, yet minimal" basis for Rust robotics includes the safe form of
an operation the domain standard sanctions, when the crate's own
substitute is destroy-then-validate.

The review's position: proceed. The evidence demonstrates the robotics
operation; sealed-state validation and preservation place it in the core;
the maintainer selects the intended time semantics. This is a
case-specific application of the unchanged scope rule, with exact API
demand and prevalence unmeasured, and no precedent for adjacent features.
That should be the stated rationale in the changelog and the merge
commit, not "tf2 has it" and not "the maintainer asked". No change to
AGENTS.md's scope rules is needed or advisable.

### Not narrow, not redirect

The branch is already the narrow version: one method, two error variants,
a crate-private helper, one message text. Every broader shape was
considered and rejected for a reason the review agrees with:

- Per-sample parents (historical topology) were majored over in 2.0 so
  chain walks terminate by construction. The maintainer has now excluded
  them explicitly.
- Edge-only re-parenting without a seed would create a pinned, empty
  buffer, which for a static frame is a state `Registry` cannot produce
  today and that `Buffer::coverage` documents as unreachable. The
  seed-carrying shape keeps "a registered buffer had a successful first
  insert" true.
- World-pose preservation is derivable on the public API by a lookup
  where the frames connect, and needs a supplied pose where they do not.
  Out, correctly.

### Deferral is not free

REPORT.md said deferral was free. That was wrong, and the review retracts
it. The branch has needed two master merges already; the
`ReparentingNotSupported` documentation that names the deliberate route
cannot ship without the route; and a branch nobody can call is a branch
nobody exercises, so the design evidence stops accumulating. Deferral
costs drift and buys only the chance that an external issue arrives, which
the maintainer has now said is not a completion condition.

## 2. Which REPORT.md conclusions still hold

Hold, and re-verified in code for this review:

- Core or nowhere (above).
- Three hazard reductions, no new hazard class. Every "new" wrong-answer
  path named in the rustdoc, retroactive static replacement, a stale seed
  moving coverage backwards, an unknown new parent detaching a subtree,
  is behavior `add_transform` already has on master. That is a statement
  about the mechanisms inspected, not a measurement: whether an easier
  destructive operation raises misuse frequency is unmeasured
  (CONSOLIDATED.md's qualification, accepted).
- The roslibrust_transforms pre-emption race is real in the inspected
  code, and CONSOLIDATED.md's qualification is right: it is a plausible
  sequence, not an observed incident, and the preservation guarantee is
  valuable without it.
- tf2 is the expectation, not a safety model: per-sample parents, a
  zero-order hold across a parent switch, retroactive static replacement.
- The cost is additive. Verified by inspection of the diff (a
  `#[non_exhaustive]` enum gains two variants; one public method; a
  `Display` text change, which is not a semver surface). The mechanical
  check could not run; see verification.
- The call is write-only about what it destroys. Still true, and the
  review's answer is in section 5.

Withdrawn:

- "Demand is zero and searched hard", "roughly once a decade", "every
  scenario was constructed". The search was narrow. Deliberate
  re-parenting exists as an implemented operation in tools, in one Rust
  application with its own store, and in the tf2 test suite. Demand for
  *this crate's* call remains unmeasured, which is a different and smaller
  claim.
- "Deferral is free" (above).
- "No comparable library ships an atomic re-parent that drops history."
  Weakened: scene_manipulation_service's Rust `reparent_frame` is a
  cycle-checked, metadata-preserving move on a latest-state store, which
  is this contract minus the timeline.

Challenges to Codex's documents:

- INDEPENDENT.md: "valuable even without concurrent readers; the
  guarantee is preservation on failure". Correct, with a qualification:
  what places the operation in the core is the two faults a caller cannot
  pre-check, the cycle and the frame's kind, both un-pre-checkable
  *because* the registry is sealed. Enforcing the pre-checkable faults at
  the boundary too (numerics via `Transform::validate`, self-reference)
  has some value, since callers forget to sequence a preflight and derived
  transforms drift, but that increment would not justify core placement
  on its own. The feature is one invariant-sensitive replacement
  operation, the minimal escape from sealing that AGENTS.md's "core or
  nowhere" branch describes, not a general transaction facility (agreed
  wording in the addendum, D2).
- TF2_RESEARCH.md: "Has anybody used it? Yes." The list mixes
  publisher-side behavior (static broadcaster replacement, the `nonexistent`
  deletion hack), visualizer ingestion (Foxglove, Rerun), and one
  consumer-side frame store with an explicit verb
  (scene_manipulation_service). Only the last is evidence for a
  *buffer-side* re-parent call. It is enough for existence, and the review
  does not ask for more, but CONSOLIDATED.md presents the whole list as if
  it bore on this API's shape, and it does not.
- CONSOLIDATED.md: "waiting for a named external consumer is not a
  completion condition" is a maintainer decision, correctly reported. The
  document should say the demonstrated-need rule is being applied at the
  domain level by its author, not that the evidence satisfies it. The
  distinction matters for the next feature that cites this one as
  precedent; AGENTS.md is explicit that no addition is precedent for its
  neighbors.

## 3. Is explicit history-discarding reconfiguration a coherent contract

Yes. Each point the brief lists, checked against the code and the tests,
with the review's own probes where the tests stop short:

- **Static retroactivity.** A static move is a static re-publish with a
  different parent: last write wins, answers every instant. No new
  semantics; the rustdoc says so. The crate's "loud over silent" priority
  is not violated by the feature, because the silence predates it and
  belongs to static transforms as such. The tf2 maintainer's own guidance
  (static means revised estimate for all time; use dynamic for
  intermittent attachment) is the right guidance here too and the
  rustdoc's "as any static re-publish does" carries it.
- **Dynamic coverage.** Collapses to `[seed, seed]`, reported as
  `NotFoundAt { covered: Some((seed, seed)) }`, and `latest_common_time`
  follows (tests `drops_the_frames_history_loudly`,
  `collapses_latest_common_time_to_the_seed`). The next sample on the new
  edge re-opens coverage (`refuses_the_old_edges_publisher_and_grows_coverage_back`).
  A stale seed moves coverage backwards; there is no monotonic-timestamp
  rule anywhere in the registry, so a guard here would be a new kind of
  rule, and not adding one is the consistent choice.
- **Descendants.** A buffer is keyed by its child and pins its parent by
  name, so a moved frame's descendants are untouched: probe P1 and P2
  below, plus `moves_the_whole_subtree`.
- **Failure preservation.** Every check precedes the single
  `HashMap::insert`; the seed is inserted into a fresh buffer first, and
  `?` on that insert returns before any mutation. Tests pin it for each
  rejection; `checks_the_topology_before_the_seed` and
  `diagnoses_the_frame_before_a_broken_seed` pin the order.
- **Kind and `max_age`.** `Buffer::empty_like` carries `Kind`'s
  discriminant and `max_age` and nothing else, with a full match so a new
  `Kind` field fails to compile. The expiry reference resets to the seed,
  which is documented and probed (P5: a stream sample behind a far seed is
  evicted, loudly).
- **Root, unknown, same parent.** A known root is refused with
  `NoParentToReplace`, an unknown frame with `UnknownFrame`, an unchanged
  parent with `ParentUnchanged`. Refusing a root rather than treating the
  call as an insert is a judgment call the review agrees with: the caller
  believed an edge existed, it did not, and that mismatch is the state in
  which a mistyped frame name is most likely. The cost is that a caller
  with no topology model has to dispatch on three errors; that caller is
  not the intended one.
- **Cycle safety.** `creates_cycle(child, new_parent)` runs with the old
  edge still in the map. Sound: reaching `child` on the walk up from
  `new_parent` means `child` is an ancestor of `new_parent`, so the new
  edge closes a cycle. Complete: any post-move cycle must contain the new
  edge, so `child` is reachable upward from `new_parent` without crossing
  `child`'s own out-edge, and that path exists pre-move. The walk never
  enters `child`'s buffer because `next == child` returns first. Tested
  under a direct child, a deeper descendant, and a drained intermediate
  frame; probe P4 adds a drained *parent*.
- **Caller-supplied poses.** The crate never computes a pose, and
  `add_transform` sets the precedent. Correct scope.
- **Error behavior.** Flat: both `From` conversions stay total, the seed's
  `NonUnitRotation` arrives as the flat variant
  (`rejects_a_non_unit_rotation_seed_as_the_flat_variant`), and the two new
  variants carry the frame. Probe P7 matches them from outside the crate.

The one coherence wrinkle worth naming is the flagship scenario. For a
REP-105 map switch during navigation, the moved `odom` edge serves exactly
one instant. A query crossing that edge is served exactly when the
requested instant lies within the new edge's sampled coverage (initially
the seed instant alone) and within the coverage of every other edge the
chain crosses. The API puts no cadence or wall-clock bound on that: two
new-edge samples inserted in the same application turn widen the
coverage at once, and a query at the seed instant is served immediately;
if nothing is ever inserted on the new edge, other instants are never
served. It is correct and loud, and the
convenient alternative (old-parent history retained under the old parent)
is the one the maintainer excluded. The rustdoc's coverage statement is
the exact one; keep it rather than any fixed-latency phrasing.

## 4. The diff and the tests

Read in full: the source diff, the 25 new unit tests, the property test,
and the documentation diff. No code blocker. Findings, separated.

### Fix before merge (documentation, three small edits)

1. **README "What's new" misdescribes the release.** The paragraph now
   reads: "2.2.0 adds `Registry::reparent_frame` ... 2.1.3 reduced
   dynamic-history memory use and lookup allocations, and shortens and
   corrects the documentation. Public APIs, serialization, and numerical
   behavior are unchanged." The last sentence was written for 2.1.3 and
   now reads as a statement about 2.2.0, which adds public API. The tense
   also slips ("reduced ... and shortens"), and the line runs past the
   README's wrap width. Scope the sentence to 2.1.3 or drop it.
2. **`ParentUnchanged`'s stated rationale does not describe what it
   prevents.** In exactly three places, the `ParentUnchanged` doc in
   `src/core/registry/error.rs` (lines 124 to 125), the CHANGELOG's 2.2.0
   entry (lines 33 to 35), and the comment in the unit test
   `reparent_frame_rejects_an_unchanged_parent` (`tests.rs` lines 2867 to
   2868), the reason given is that "a caller resolving every failed insert
   into a re-parent would wipe the buffer once and look correct forever
   after". The method's own `# Errors` bullet ("Re-parenting drops
   history, so this is not an upsert") is already correct and needs no
   change; an earlier draft of this item listed it wrongly. A failed insert that reports
   `ReparentingNotSupported` always carries a *different* parent, so a
   mechanical handler never reaches `ParentUnchanged`; the variant cannot
   defend against that loop, and the same wipe-once story plays out
   identically with or without it. What the refusal actually protects is
   different and worth stating: a caller that re-applies an unchanged
   reconfiguration (a config reload, a retried command) must not wipe the
   frame's history on every re-application, and `reparent_frame` must not
   quietly become a second publish path for samples on an existing edge.
   REPORT.md's phrasing, "the same-parent retry that would silently wipe
   history is refused", is the accurate one. The behavior is right; the
   three rationale texts should say why in one consistent sentence. A
   fourth, looser phrase is not a wrong rationale but could be tidied with
   them: `tests/properties.rs` line 382 calls a repeated call "never an
   upsert that would wipe the seed", and an upsert would not wipe; a
   repeated move would.
3. **Pin wording drifted in two places.** `add_transform`'s rustdoc still
   says the first insert pins the parent "until `Registry::remove_frame`
   releases it", and MIGRATION.md's strict-tree bullet says the parent
   stays pinned "until `remove_frame` removes its incoming edge". Both
   are now incomplete: `reparent_frame` replaces the parent pin (and only
   the parent pin). AGENTS.md and the buffer docs were updated; these two
   were missed.

### Optional improvements

- **The check order is promised as contract** ("Checked in this order").
  The order is natural and tested, and promising it costs little, but it
  is an exported commitment that forbids, for instance, running the cheap
  numeric validation first. Consider stating the order without the word
  "contract", or keep it and accept the commitment. The review leans
  keep; it is a minor point.
- **The property test hangs rather than fails on a broken cycle check.**
  `random_inserts_and_re_parents_keep_the_tree_walkable` says so in its
  own comment. proptest's `timeout` config (available with the default
  `std` feature of proptest) would turn a hang into a reported failure
  with a shrunk case. Optional; the property itself is the right one, and
  the review ran it at 3000 cases.
- **One untested shape:** the old parent remaining known because it is a
  child elsewhere. Probe P1 covers it and it is implicit in the property
  test; an explicit unit test would document that no dangling edge is
  created. Optional.
- **`NoParentToReplace`'s rustdoc carries a long edge-reversal recipe.**
  The recipe is correct (the review traced it: remove the child's edge,
  then `add_transform` if the old parent was a root, `reparent_frame` if
  it hangs mid-tree, then re-attach the pair's new root). It is also the
  longest error-variant doc in the crate, against the 2.1.3 style of
  trimming. A shorter form that names the three calls and leaves the
  split-off consequence to one sentence would serve as well.
- **"Record it beforehand if needed"** in the rustdoc could say what it
  means: the registry has no parent accessor, so the caller's own
  topology record is the source. One clause.
- **Release mechanics, not a branch defect.** The version, `Cargo.lock`,
  and README pins are already at 2.2.0 on the feature branch. If the
  branch merges without an immediate release, master's README advertises
  an unpublished version. Merge and release in one motion, per the
  AGENTS.md checklist, or move the bump to the release branch.

### Not required

- A new example program. The rustdoc example, 24 targeted unit tests, and
  the property test document the contract; AGENTS.md asks for examples
  "when relevant", and no example in `examples/` teaches removal or
  re-parenting today. Codex left this open; the review closes it as not
  needed.
- A ROS adapter or a roslibrust passthrough. Out of this crate's scope;
  a downstream decision.

## 5. Recommendation, counterargument, remaining work, disagreements

### Recommendation

Merge as 2.2.0 after the three documentation fixes. Ship
`reparent_frame` returning `Ok(())`, with no `parent_of` accessor.

On the return type, the review disagrees with its own REPORT.md, which
listed `Ok(String)` as the first "if it ships" change. The argument for it
was that it is zero-cost and a one-way door (adding it later breaks the
return type). But the one-way door swings both ways: shipping it commits
the crate to it until 3.0, and no caller exists. If an audit or pre-flight
need ever appears, it gets its own argument then. An accessor would be
additive in the semver sense, but additivity is not a pass: AGENTS.md's
scope section warns against exposing primitives in place of narrow
queries and against raw-state answers, so `parent_of` is neither a
roadmap item nor a promised fallback (addendum, D4). Under "the default
answer is no", `()` is right.

### Strongest counterargument

Nobody who uses this crate has asked, prevalence is unmeasured, and the
feature ships on domain evidence plus the maintainer's product judgment.
The safety case is real but narrow: one silent path in the substitute (an
opposite-kind seed accepted without error), and a destroy-then-fail
sequence that is loud. A reader who takes "demonstrated" to mean
"demonstrated by a user of this crate" would defer, and the crate would
lose nothing it can measure. The review's answer is that the rule does
not say that, that the sealed registry makes this the only correct form
of an operation the domain standard assigns to an authority, and that the
cost is one method and two variants on an enum built to grow. That is a
judgment, and it should be recorded as one.

### Remaining work

1. The three documentation fixes above.
2. Re-run `tests/test_all.sh` on the fixed tree (documentation edits
   touch rustdoc, which the gate checks).
3. Merge; if the merge is not a fast-forward, re-run the gate on it; tag;
   publish; the release checklist as written.

### Initial disagreements for Codex (all resolved in the addendum below)

- **D1. Rule satisfied versus rule applied.** CONSOLIDATED.md reads the
  maintainer's statement as resolving the demonstrated-need question. The
  review reads it as the rule's author applying the rule at the domain
  level, with the strict reading still unmet. Same recommendation,
  different rationale, and the rationale is what the next proposal will
  cite. Codex should say which it means.
- **D2. What the guarantee is.** INDEPENDENT.md's "preservation on
  failure is valuable even without concurrent readers" needs the
  qualification that it is load-bearing only for the two faults the
  sealed API makes un-pre-checkable. The feature is the minimal escape
  from sealing, not a general transaction facility, and should be
  described that way so it does not become an argument for more
  transactions later.
- **D3. Evidence for a buffer-side verb.** TF2_RESEARCH.md's use table
  supports "the domain re-parents"; only scene_manipulation_service
  supports "a frame store wants an explicit verb". CONSOLIDATED.md should
  weight it that way.
- **D4. Return type.** Codex deferred `Ok(String)` to "before
  publication". The review decides it now: no, for the reason above.
- **D5. Example program.** Codex left it open; the review says not needed.
- **D6. `ParentUnchanged` rationale.** Not a disagreement with Codex's
  documents, which use the "who authorizes" framing correctly, but a
  finding against the branch's own rustdoc, CHANGELOG, and test comments.
  Listed here so Codex can check the reading.

## Verification: what was run

All runs on this machine against `bb75fa9` with the working tree as
described above. The toolchain on PATH is a Nix nightly,
`rustc 1.100.0-nightly (0dfb098f3 2026-08-31)`,
`cargo 1.100.0-nightly (e8cb624d5 2026-08-22)`; there is no `rustup`.

| Run | Result |
| --- | --- |
| `cargo test --all-features` | 181 unit, 4+4+11+16+2+17+9 integration, 35 doc tests; all pass |
| `cargo test --no-default-features` | 179 unit, 4+4+11+0+2+15+9 integration, 33 doc tests; all pass |
| `cargo test --all-features reparent` | 24 tests matched; all pass |
| `PROPTEST_CASES=3000 cargo test --test properties random_inserts_and_re_parents` | passes (0.57 s) |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo clippy --all-targets --no-default-features -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |
| `cargo semver-checks check-release --baseline-rev v2.1.3` (std and no_std) | **did not run**: the installed cargo-semver-checks 0.49.0 rejects this nightly's rustdoc JSON format v61 (supports v56, v57, v60). Tool and toolchain mismatch, not a crate problem. The additive-only claim rests on inspection of the diff. CI runs this job on stable. |
| `tests/test_all.sh` (full gate) | see the line appended below this table |

Eight ad-hoc probes were compiled in a scratch crate outside the
repository, depending on the branch by path, to exercise shapes the unit
tests do not: the old parent remaining known when it is a child elsewhere
(P1), a move under the frame's own grandparent and back (P2), a static
move above a dynamic descendant in a `with_max_age` registry (P3), a move
under a drained parent and the reverse move as a cycle (P4), a stream
sample behind a far-future seed being evicted (P5), a root of a second
tree refused with `NoParentToReplace` (P6), matching and displaying the
new variants from outside the crate (P7), and `reparent_frame` under
`Registry<std::time::SystemTime>` (P8). Every probe produced the
documented outcome; none produced a silent answer.

Codex's gate log at `/tmp/transforms-reparent-scope-gate.log` ends with
`GATE PASSED (all steps completed)` and is timestamped 13:21 today, but it
records no commit hash. The review therefore ran the gate again rather
than rely on it.

Full gate on this tree: `tests/test_all.sh` run by the review on
`bb75fa9d515dac59109edd3718cde65c90922b44` (hash captured into the log
before the run), finished with `GATE PASSED (all steps completed)` and
exit status 0. Log: the session scratchpad, `gate.log`, 1823 lines.

Nothing in the repository was changed by this review except the creation
of this file; `git status` after the runs shows only the four untracked
analysis documents.

## Addendum, 2026-09-20: response to Codex

Codex's response (`/tmp/reparent-fable-review-response.md`) was read in
full. Each point was checked against the source or the rule text rather
than accepted on trust. Codex agrees to proceed. The body above was
corrected where a check went against its original wording; the list at
the end of this addendum records every such change.

### D1, rule satisfied versus rule applied: accepted

Codex is right. AGENTS.md says "additions follow a demonstrated, concrete
need", set against "a downstream user *might* want it". It specifies
neither that the need come from a user of this crate nor prior use of the
crate; requiring either adds a rule. The review's "strict reading unmet"
imported REPORT.md's "nobody has asked" framing into the rule text. The
domain evidence (the operation is performed, REP-105 assigns it to an
authority, tf2 supports it, tools and one Rust frame store implement it)
together with the substitute's failure modes demonstrates a concrete need
for anyone who re-parents against this registry. Prevalence among this
crate's users is unmeasured; that is a fact to report, not a rule left
unmet. The maintainer's clarification selected the temporal scope and did
not waive anything, and nothing needed waiving.

Joint rationale, accepted verbatim: "The evidence demonstrates the
robotics operation; sealed-state validation and preservation justify its
placement; the maintainer selects the intended time semantics. This is a
case-specific application of the unchanged scope rule, with exact API
demand and prevalence unmeasured, and no precedent for adjacent features."

One consequence worth recording: with the D2 correction below, the scope
section's middle branch resolves more cleanly than the review first said.
The substitute is not "merely verbose"; it silently accepts a kind change.
So the "wait for demonstrated need" branch does not apply, and the "ship
the correct version" branch does.

### D2, what the substitute does and what the guarantee is: accepted

Factual correction accepted and verified. After `remove_frame`, the child
has no buffer, and `process_add_transform` builds a fresh one whose kind
is decided by the seed's stamp: `Stamp::Static` makes a static buffer,
`Stamp::At` a dynamic one. No kind conflict can arise; an opposite-kind
seed is accepted without error. The review's section 1 listed "a kind
conflict" among the faults the substitute discovers after destroying
history, which was wrong and is corrected. This is the a7e7cd3 failure
class, and it strengthens the case rather than weakening it: the
substitute has a genuinely silent path, not only destructive-but-loud
ones.

"Valuable only for exactly two faults" was overstated. What places the
operation in the core is the two faults a caller cannot pre-check (cycle,
kind); enforcing the pre-checkable ones at the boundary as well has some
value, because callers forget to sequence a preflight and derived
transforms drift, and that increment alone would not justify placement.

Agreed wording: "This is one invariant-sensitive replacement operation,
not a general transaction facility. Sealed topology/kind and old samples
make the exact guarantee impossible for a helper over an arbitrary
registry; public prevalidation can mitigate some failures without
reproducing that guarantee."

### D3, weight of the ecosystem evidence: agreed

Publisher, editor, and visualizer evidence establishes real workflows
and explicit authority and pose decisions; scene_manipulation_service is
the one explicit frame-store mutation analogue, and TF2_RESEARCH.md
already labels its own store and its lack of time history. No project is
claimed as a consumer of this crate or this contract. The synthesis
should make the weighting explicit; no body change.

### D4, return type and accessor: agreed, with Codex's qualification

Keep `Result<(), RegistryError<T>>`; no `parent_of`. Codex's
qualification is accepted and the review's wording is corrected: "a
`parent_of` accessor is additive and can wait for that need" read as a
roadmap, and it is not one. Additivity in the semver sense does not
satisfy the scope rules. An accessor answers raw state, which the scope
section says the interface never does, and it is a primitive of the kind
that section says not to expose in place of a narrow query. Any future
audit capability argues from its own invariant and its own demonstrated
need, and may well lose that argument. The historical INDEPENDENT.md
stays unchanged; the synthesis closes this for the branch.

### D5, example program: agreed

The rustdoc example and the tests suffice. No new example program and no
external integration is a merge prerequisite.

### D6, `ParentUnchanged` rationale locations: agreed, verified by grep

The incorrect "resolving every failed insert into a re-parent" rationale
appears in exactly three places:

- `src/core/registry/error.rs` lines 124 to 125, the variant's doc;
- `src/core/registry/tests.rs` lines 2867 to 2868, the comment in
  `reparent_frame_rejects_an_unchanged_parent`;
- `CHANGELOG.md` lines 33 to 35, the 2.2.0 "Added" entry.

The method's `# Errors` bullet (`src/core/registry/mod.rs` line 534,
"Re-parenting drops history, so this is not an upsert") is already
correct; the review's item 2 listed it wrongly and is corrected. One
further phrase is loose rather than wrong: `tests/properties.rs` line 382
says a repeated call is "never an upsert that would wipe the seed", and
an upsert would not wipe, a repeated move would. Optional tidy, noted in
item 2. The `ReparentingNotSupported` admonition against mechanical
resolution stands on its own reason, two disagreeing publishers taking
turns wiping each other's history, and needs no change. The README
release wording and the two stale pin-until-removal sentences stand as
findings.

### Two precision points: both accepted

- "One publish period" was a fixed-latency implication the contract does
  not make. A query crossing the moved edge is served exactly when the
  requested instant lies within that edge's sampled coverage (initially
  the seed instant alone) and within every other crossed edge's coverage.
  There is no cadence or wall-clock lower bound: samples can be inserted
  on the new edge immediately, several in one application turn, and the
  seed instant is covered from the start. Section 3 now says so and
  recommends keeping the rustdoc's exact coverage statement.
- "No new hazard class" is a claim about the mechanisms inspected, not a
  measurement of misuse frequency. Section 2 now bounds it that way.

### Authorization

Agreed. Neither this review nor Codex's response authorizes a merge, a
tag, or a publication. "Proceed" means merge-worthy after the three
documentation fixes and a gate re-run on the fixed tree; timing is the
maintainer's.

### Agreed decisions

1. Proceed with the branch's narrow design unchanged: an explicit call;
   ordinary insertion keeps rejecting parent changes; every check before
   any mutation, so a returned error preserves the old edge and samples;
   kind and `max_age` preserved; dynamic history dropped to the seed;
   static geometry replaced for all time; caller-supplied pose;
   descendants untouched.
2. The rationale to record is the joint D1 text: domain evidence
   demonstrates the operation, sealed-state validation places it in the
   core, the maintainer selects the time semantics; a case-specific
   application of the unchanged rule; prevalence unmeasured; no precedent
   for neighbors.
3. Return type stays `Result<(), RegistryError<T>>`. No `parent_of`.
   Neither is a roadmap item.
4. No example program or integration as a prerequisite.
5. Three documentation fixes before merge: the README "What's new"
   sentence; the `ParentUnchanged` rationale in the three locations under
   D6; the pin-until-removal sentences in `add_transform`'s rustdoc and
   MIGRATION.md. Then re-run `tests/test_all.sh` on the fixed tree.
6. Coverage statements stay exact; no fixed-latency language.
7. "No new hazard class" is a mechanism-level claim; misuse frequency is
   unmeasured.
8. Nothing here authorizes a merge, tag, or publication.

### Remaining disagreements

None of substance. Each of Codex's points was checked against the source
or the rule text and held. The review's positions that changed (D1's
reading of the rule, D2's account of the substitute, D4's accessor
framing, the two precision points) changed because the check went against
the original wording, not for agreement's sake.

### Corrections made to the body of this review on 2026-09-20

- Summary: "strict reading unmet" replaced; the rule does not require a
  user of this crate to ask, prevalence is unmeasured.
- Section 1, scope bullet: "a kind conflict" removed from the faults the
  substitute discovers; silent acceptance of a kind change added.
- Section 1, demonstrated-need paragraphs: reworded to the joint D1
  rationale.
- Section 2, "no new hazard class": bounded to the mechanisms inspected.
- Section 2, INDEPENDENT.md challenge: "load-bearing only for" replaced
  by the agreed D2 wording.
- Section 3, flagship wrinkle: "one publish period" replaced by the exact
  coverage statement.
- Section 4, item 2: locations corrected per D6; the `# Errors` bullet
  removed from the list; the `properties.rs` phrase noted as optional.
- Section 5, recommendation and counterargument: `parent_of` no longer
  described as the additive answer; the counterargument reworded to the
  rule's actual text.

### Final correction, 2026-09-20

Codex agreed with all eight decisions and caught one leftover: section 3
and the precision-points entry above still said "at least one publish
period". That is still a cadence claim, and the API has none. Two
new-edge samples can be inserted immediately, including in one
application turn, and the seed instant is covered from the moment of the
move. Both places now state only the sampled-coverage condition. The
section 5 heading "Explicit disagreements for Codex" is relabeled as
initial disagreements resolved here, so D1 to D6 there are not read as
current objections. Accepted without reservation; no further debate.
