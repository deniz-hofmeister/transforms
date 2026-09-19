# reparent_frame value debate

Analysis date: 2026-09-19. Branch: `feature/reparenting` at `f0cc45b`
(2.2.0 unreleased, master merged in through v2.1.3). This report answers
one question: is the intent of `Registry::reparent_frame` of value to the
community? It does not constitute a release decision.

## Verdict

The intent of `Registry::reparent_frame` is of real value, but that value
is latent today: defer the feature, merge-ready, until someone outside the
crate can call it.

An atomic, kind-preserving, cycle-checked move is the only correct form of
an operation the crate's one-parent model forces onto users. It cannot be
built on the public API, because the cycle check needs sealed internals,
and the `remove_frame` then re-add substitute destroys the frame before it
can discover the cycle. It is strictly safer than that substitute and opens
no new hazard class.

Against that, nobody has asked. The crate's tracker, GitHub code search,
and a decade of ROS questions show zero direct demand. Of the three reverse
dependencies, one pins a 2.0 beta and one exposes no registry mutator, so
on release day no existing code path can reach the call. The crate's own
scope rule says inferred demand waits.

Of eight lenses, four judged it conditionally valuable, three marginal, and
the devil's advocate net negative. After cross-examination, "harmful" did
not survive and "defer" was the position every lens could live with.

## Method

Eighteen agents produced this assessment against the branch above, with
read-only access to the repository, the design record, and the web.

1. Seven lenses each argued independently from a fixed vantage point: a
   ROS2 practitioner, a functional-safety engineer, an embedded no_std
   user, a minimalist maintainer holding the crate's own scope rule, a
   downstream bridge author, a computer-vision user with no ROS, and a
   Rust API designer. An eighth lens, the devil's advocate, was told to
   argue for no or negative value as hard as the code allows.
2. An evidence scout gathered checkable facts in parallel, reading the tf2
   sources, REP-105, the ROS question archive, this crate's tracker, and
   the source of all three reverse dependencies.
3. A cross-examiner attacked each lens's strongest claims on both sides.
   Each claim was marked *holds* only when support was verified,
   *weakened* when it rested on an unverified assumption, and *refuted*
   when contrary evidence was found. The cross-examiner then restated the
   thesis at the strength it earned.
4. A synthesis step kept only claims that survived cross-examination.

Each lens rated the feature on a four-point scale: valuable, conditionally
valuable, marginal, or net negative, with a confidence between 0 and 1.

## Evidence dossier

The debate hinges on a small set of external facts. All of them were
checked against primary sources on 2026-09-19.

| Question | Finding | Source |
| --- | --- | --- |
| Does tf2 let a parent change at runtime? | Yes, silently. The parent is stored per sample. When the two bracketing samples have different parents, the cache returns the older sample uninterpolated with no error. Old-parent history is kept. A static re-publish replaces pose and parent retroactively. | [cache.cpp](https://raw.githubusercontent.com/ros2/geometry2/rolling/tf2/src/cache.cpp), [static_cache.cpp](https://raw.githubusercontent.com/ros2/geometry2/rolling/tf2/src/static_cache.cpp) |
| Does ROS treat re-parenting as normal? | No. The ROS1 diagnostic tool flags any observed parent change as "TF re-parenting contention", a fault symptom. | [tfwtf.py](https://raw.githubusercontent.com/ros/geometry/noetic-devel/tf/src/tf/tfwtf.py) |
| What does REP-105 say? | Each frame has exactly one parent. Yet "it is the responsibility of the localization frame authority to reparent the odom frame appropriately when moving between maps." The operation is sanctioned and assigned to the publisher. | [REP-105](https://raw.githubusercontent.com/ros-infrastructure/rep/master/rep-0105.rst) |
| Do ROS users ask for it? | Rarely as a want, more often as a fault. Deliberate re-parenting was asked about in 2011 and 2019, both answered "not designed for that". Other hits are two-publisher conflicts. A 2020 request for multiple parents in geometry2 was argued down and closed by archival. | [Q30321](https://robotics.stackexchange.com/questions/30321/tf-mux-or-multiple-parents), [Q90382](https://robotics.stackexchange.com/questions/90382/how-to-change-parent-in-tf-tree) |
| How do the reverse dependencies handle the rejection today? | Three dependents exist. transforms_io logs at error level and skips, feeding /tf and /tf_static into one registry, so the static-stale trap is reachable end to end. roslibrust_transforms logs at warn level and skips, and exposes a `remove_frame` wrapper added by this crate's maintainer in PR #338, with separate write locks per call. whirl_tf pins a 2.0 beta and cannot see 2.2.0. None names the rejection variant. | [crates.io reverse deps](https://crates.io/crates/transforms/reverse_dependencies) |
| Has anyone asked this crate? | No. 28 issues and 4 discussions, none about re-parenting. Two external authors ever filed issues, on unrelated topics. | [Issue tracker](https://github.com/deniz-hofmeister/transforms/issues) |
| How large is the audience? | 27,260 all-time downloads and 2,943 in the recent window, against 3 reverse dependencies. | crates.io |
| How do comparable libraries handle it? | Scene graphs (Unity, three.js, Godot, Bevy) offer an explicit reparent with a keep-world-pose or keep-local-pose knob and no history concept. Timed robotics stores either switch silently per sample (tf2, re_tf, tf_rosrust), silently accept multiple parents (cu-transform), or hard-reject with no escape hatch (schiebung). None ships an atomic reparent that drops history. | [Unity](https://docs.unity3d.com/ScriptReference/Transform.SetParent.html), [three.js](https://threejs.org/docs/pages/Object3D.html), [Godot](https://docs.godotengine.org/en/stable/classes/class_node.html), [Bevy](https://docs.rs/bevy/latest/bevy/transform/components/struct.GlobalTransform.html) |

Not verified: the ROS Discourse thread on tf2 outside ROS came through a
summarizing fetch, question bodies from the Stack Exchange API were
truncated, and later comments on the geometry2 multiple-parents issue were
not read.

## The case for

Five arguments survived cross-examination intact.

**It is core or nowhere by the crate's own rule.** `Registry` exposes no
parent accessor and no ancestry query. The lookups report connectivity
through a common ancestor, so a caller cannot distinguish "the new parent
is my own descendant" from "we are connected". The cycle check is private.
The substitute, `remove_frame` then re-add, is a bare map removal followed
by an insert whose cycle check runs only for an absent child. It therefore
discovers the cycle after the buffer is gone, and a root named only by
that edge vanishes. Every lens that examined this point conceded it,
including the devil's advocate.

**It removes three hazards the substitute leaves open and adds no new
class.** The fresh buffer inherits the frame's kind and expiry policy, so
the seed cannot flip a dynamic frame to static, which is the failure class
behind the a7e7cd3 scar. The same-parent retry that would silently wipe
history is refused with `ParentUnchanged`. All checks precede the single
map insert. Three refuters independently checked the caveats the rustdoc
lists, such as retroactive static replacement, stale seeds, and unknown
new parents, and found each is a re-spelling of behavior `add_transform`
already has on master.

**In the one deployment that can reach it, atomicity is correctness, not
convenience.** This was the strongest point of the run, and no lens raised
it in its opening argument. The cross-examiner of the devil's advocate
found it. In roslibrust_transforms the registry sits behind a private
lock, and the remove call, the add call, and the /tf ingest task each take
their own write guard. A deliberate remove-then-add can be pre-empted by
one late old-parent message that re-pins the frame, after which every
new-parent sample is dropped forever. That failure is loud, but a user
cannot fix it from outside. Only a one-call move is atomic there.

**tf2 is the expectation, not a safety model.** When bracketing samples
have different parents, tf2 returns the older sample uninterpolated with
no error. Its diagnostic tool flags every observed re-parent as
contention. Yet REP-105 assigns the localization authority the job of
re-parenting odom between maps. The domain performs this operation and
only ever detects it as a fault. A loud, explicit verb is the safety-minded
answer.

**The cost is bounded.** The change is additive. Both bridges format
errors through `Display` only. The two variants are produced solely inside
the new method, and an uncalled generic method emits no code for a
firmware that never calls it.

## The case against

Five arguments survived on this side too, and the first is the strongest
evidence in the whole assessment.

**Demand is zero, and it was searched hard.** No issue on this crate's
tracker asks for it. A GitHub code search for `ReparentingNotSupported`
hits only this repository. In the ROS question archive, deliberate
re-parenting comes up roughly once a decade, and the accepted answers say
the tree is not designed for it. The design record itself concedes demand
was inferred and struck two of its own three motivations after review.

**The addressable population is one wrapper, and it has not asked.**
transforms_io keeps its registry private with no mutator. whirl_tf pins a
2.0 beta. The static-stale trap the design record cites as reachable can
only be closed by a bridge writing "on `ReparentingNotSupported`, call
`reparent_frame`", which is exactly the mechanical handler the error docs
forbid. The feature's headline reachable benefit and its headline
documented misuse are the same line of code.

**The maintainer, wearing the downstream hat, chose the substitute.**
roslibrust PR #338, authored by this crate's maintainer, added
`remove_frame` as the documented re-parenting route two weeks after
`reparent_frame` was implemented. The only downstream affordance is
self-generated, and it signals the hatch was judged adequate.

**By the scope rule's middle branch, this is "wait".** Every substitute
failure path the refuters traced is loud: `UnknownFrame`, `Disconnected`,
or `ReparentingNotSupported`. None returns wrong geometry. The kind-flip
hazard needs a caller to hand over an opposite-kind seed deliberately. The
safe-use discipline of "act on a decision, not on message arrival" is
prose the crate cannot enforce, against its first priority, and for a
bridge the message is the decision.

**It is write-only about what it destroys and partial about the family.**
The call returns unit and there is no `parent_of`. The rustdoc's "record
the old parent beforehand" has no contractual read path, since `Debug`
output is declared unstable, and it is racy under the one-write-guard rule
the same docs impose. Edge reversal still needs a three-call recipe. The
branch adds two frozen variants, a misnamed `ReparentingNotSupported`
until 3.0, and about 1,400 lines, of which roughly 1,100 are tests.

## Agent stances

No lens rated the feature plainly valuable, and after cross-examination
none held "net negative" either. The table gives each lens's own verdict
and how its claims fared under attack. Confidence is the lens's own, on a
0 to 1 scale.

| Lens | Position | Confidence | One-line verdict | Claims held / weakened / refuted |
| --- | --- | --- | --- | --- |
| ROS2 practitioner | Conditionally valuable | 0.60 | Right verb, wrong audience for now. Correct for application code that owns the switch decision, but the crate's actual users are message-fed bridges that must not call it. | 5 / 6 / 0 |
| Safety engineer | Conditionally valuable | 0.65 | Safer than the substitute and opens nothing new, but the loud guarantee is dynamic-only and safe use rests on a caller discipline the crate cannot enforce. Ship as a deliberate operation, never as error recovery. | 4 / 8 / 1 |
| Embedded user | Marginal | 0.72 | Free to ignore and almost never exercised on a fixed rig. Harmless doc weight rather than value. | 7 / 4 / 0 |
| Minimalist maintainer | Marginal | 0.62 | Core or nowhere by the scope rule, but nowhere fails loudly and nobody has asked. Defer the method and keep the branch warm. | 8 / 6 / 1 |
| Downstream bridge author | Conditionally valuable | 0.60 | A small real gift as an app-facing pass-through, and a one-line-away liability in the message loop. Does nothing for faithful tf2 mirroring, which a one-parent registry cannot do anyway. | 6 / 5 / 2 |
| CV user, no ROS | Marginal | 0.70 | Correctly built, but the workloads that look like re-parenting are root re-expressions this call refuses or per-frame source switching it forbids. The one real fit is a convenience, not a correctness gain. | 7 / 4 / 0 |
| Rust API designer | Conditionally valuable | 0.70 | Well-shaped where it matters, but write-only about what it destroys. Return the replaced parent and the shape stops subtracting from the value. | 6 / 4 / 1 |
| Devil's advocate | Net negative | 0.62 | Defer it. Let the first real issue decide whether the community wants a history-dropping atomic move or a history-keeping one. | 8 / 6 / 2 |

### ROS2 practitioner

- **Strongest for:** runtime re-parenting is a real minority field event,
  such as grasp attach or per-floor maps, and tf2 gives it no verb.
  Publishers just change the frame id and the buffer accepts it silently.
- **Strongest against:** the crate's entire real user base is ROS bridges,
  and by the feature's own contract a bridge must not call it. In the
  dominant tf2 practice the parent name never changes, only the edge's
  content.
- **After cross-examination:** "right verb, wrong audience" survived. The
  charge that bag-analysis teams lose old-parent history did not, because
  that loss belongs to the 2.0 one-parent model, not to this call.
- **Would change its mind:** a dependent adding a re-parent policy hook
  that calls the method, or an issue describing the grasp or elevator case
  in application-held registry code.

### Safety engineer

- **Strongest for:** it closes a silent kind flip and a silent retry wipe,
  and removes the destroyed-then-refused intermediate state from the
  hazard analysis.
- **Strongest against:** the loud design is not loud for static frames,
  and a stale seed corrupts the `latest_common_time` idiom without an
  error.
- **After cross-examination:** "a strict subset of the substitute with
  three hazard reductions and no new class" held. The claim that edge
  reversal is the move that most needs atomicity was refuted, since after
  removal the reversed edge cannot close a cycle.
- **Would change its mind:** a guard rejecting or reporting a seed older
  than the newest dropped sample, or evidence that a downstream wired the
  mechanical handler.

### Embedded user

- **Strongest for:** code size for non-callers is genuinely zero, and the
  new variants are unreachable from any existing call. In the one embedded
  case that re-parents, a config-driven re-mount, validate-first beats
  destroy-then-fail.
- **Strongest against:** the target population carries no runtime frame
  tree at all. PX4 and micro-ROS keep the tree on the host. The doc weight
  lands on the reader least able to use it.
- **After cross-examination:** "harmless, nearly weightless, nearly
  useless from this seat" held. The re-mount case weakened, because a
  firmware owning a const-table tree can pre-check the cycle itself.
- **Would change its mind:** a real no_std downstream calling
  remove-then-add on a config command, or a measured non-zero flash delta
  for a non-caller.

### Minimalist maintainer

- **Strongest for:** the cycle check cannot be reproduced on the public
  API, so this is not merely verbose. It is core or nowhere.
- **Strongest against:** the scope rule's ship branch is reserved for
  wrong-answer generators, and the substitute fails loudly. Demand is
  inferred, and the only consumers are bridges the contract tells not to
  use it.
- **After cross-examination:** the deferral case held. The rider to land
  the subtree-recipe docs fix now was refuted, because 2.1.3 already
  shipped it.
- **Would change its mind:** a reverse dependency opening an issue that
  branches on the rejection, or a reproducible path where the substitute
  returns geometrically wrong `Ok` data.

### Downstream bridge author

- **Strongest for:** it closes a real re-pin race that the `remove_frame`
  route hands to bridge users, and the seed-carrying shape is exactly what
  a bridge has in hand.
- **Strongest against:** the bridge's actual job, mirroring a
  per-sample-parent stream, is unserved either way, and the
  mechanical-resolution trap is one line away from today's code.
- **After cross-examination:** the cross-examiner cut this to "marginal,
  harmless if wired as an explicit app operation". Two claims were
  refuted: that this is the only way to repair a dropped static re-parent,
  and the URDF-reload scenario, since a transient-local re-subscribe also
  fixes it.
- **Would change its mind:** a bridge author exposing the call as an
  app-facing operation, or a downstream commit wiring it into the message
  loop.

### CV user, no ROS

- **Strongest for:** the caller-supplied seed is strictly more general
  than a scene graph's SetParent, and history dropping is correct for
  statically calibrated rig elements.
- **Strongest against:** the dominant AR re-parent is a root
  re-expression, which this call refuses by design. Marker and sensor
  handoff must not be modeled as re-parenting at all.
- **After cross-examination:** "correctly built and correctly bounded,
  which keeps it off the workloads these domains have" held. One ARKit
  premise rested on a bug fixed in iOS 15.2.
- **Would change its mind:** a non-ROS downstream keeping a
  runtime-assembled tree in this crate and re-attaching subtrees on
  tracking events.

### Rust API designer

- **Strongest for:** atomicity is realized in the ownership shape rather
  than promised in prose, and the seed cannot re-decide kind or retention
  because the buffer type carries them.
- **Strongest against:** the API is write-only about state it destroys,
  and the rustdoc instructs callers to record a parent that no accessor
  can read.
- **After cross-examination:** the shape claims held intact. The
  write-only defect narrowed to "hurts an audit tool and is racy in a
  shared registry". The claim that returning dropped samples would be a
  wrong-answer generator was refuted, so returning unit rests on
  minimalism, not safety.
- **Would change its mind:** ship `Ok(String)` with the replaced parent,
  or a `parent_of` accessor alongside.

### Devil's advocate

- **Strongest for (conceded):** atomicity is real and cannot be built
  outside, the dynamic history loss is loud, and kind preservation plus
  `ParentUnchanged` close two genuine traps.
- **Strongest against:** demand is contradicted by the record, the feature
  sits on the wrong side of the wire for its own flagship scenario, and
  history loss solves the wrong problem for map switching.
- **After cross-examination:** "undemanded" held. "Harmful" and "solves
  the wrong problem" did not: every new wrong-answer path in the rustdoc
  re-spells existing `add_transform` behavior, and the history-keeping
  alternative is fiction because no read-back API exists. The
  cross-examiner also found the strongest point for the feature, the lock
  race in roslibrust_transforms.
- **Would change its mind:** a filed issue from a real user whose
  consumer-side registry owns the re-parent decision, or a redesign that
  keeps history under the old parent.

## Where the lenses split

The disagreements reduce to four underlying assumptions. Three of them the
evidence settles or dissolves. One is a policy question only the
maintainer can answer.

**Is old-parent history an asset or a hazard?** The practitioner and the
devil's advocate call it an asset for bag replay and post-incident review.
The safety, API, and CV lenses call it the silent-answer hazard this crate
ranks worst, because tf2's zero-order hold across a parent switch is
exactly that. The refuters showed this split does not bear on the feature:
no read-back API exists, so both routes drop history equally. The loss
belongs to the 2.0 one-parent model.

**Must demand be demonstrated, or may it be inferred from a hazard?** The
minimalist, embedded, and devil's-advocate lenses want it demonstrated,
since the substitute fails loudly. The safety and API lenses say the
kind-flip is a wrong-answer generator, which invokes the scope rule's
"ship the correct version" branch. The real disagreement is whether a
hazard reachable only through a deliberate opposite-kind seed counts as
"the substitute users would build". Only the rule's author can settle
that.

**Who is the caller?** The bridge-facing lenses read the rustdoc as
forbidding bridges. The API and devil's-advocate cross-examiners read it
as forbidding mechanical resolution, not a passthrough to a deciding
application. One rustdoc sentence would settle this.

**Is atomicity correctness or convenience?** The minimalist says
convenience, because the failure is loud and retryable. The
devil's-advocate cross-examiner says correctness, because in the
shared-lock wrapper the retry can be pre-empted forever. The synthesis
sided with correctness, narrowly: it is correctness in exactly one
deployment shape, which happens to be the only reachable one.

## Judgment and recommendations

Defer the feature, merge-ready. The intent is valuable and the
implementation is the version an outside author would get wrong, so
dropping it is the wrong call. But on release day no existing code can
reach it, the one deployment where it is a correctness gain has not asked,
and the crate's own rule says inferred demand waits.

Deferral is free. The error enum is `#[non_exhaustive]`, so the variants
cost nothing more later. The subtree-recipe docs fix already shipped in
2.1.3. The branch is verified, additive, and now merged with master.

**Two questions would settle it:**

1. Does a consumer-side re-parent decision point exist anywhere in
   practice? Every "helps" scenario across seven lenses was constructed.
   Nav2 keeps one map frame, and grasp attachment lives in the planning
   scene, not in tf. An issue or pull request on roslibrust from someone
   other than the maintainer, or a user report of the remove-then-re-pin
   race, would answer this. The cheapest probe is a tracking issue there
   naming the race.
2. Does a hazard reachable only through a deliberate opposite-kind seed
   meet the scope rule's "wrong-answer generator" bar? This is policy, and
   the rule's author decides it. Yes means ship; no means wait.

**If it ships, three changes raise its value most:**

- [ ] Return the replaced parent as `Ok(String)`. The old buffer's pin is
  an owned string the insert already discards, so this is zero-cost. It
  deletes the impossible "record it beforehand" instruction and gives a
  bridge its audit line under one lock.
- [ ] Add a `reparent_frame` passthrough in roslibrust_transforms,
  mirroring the existing `remove_frame` wrapper. This is the change that
  makes the feature reachable at all and closes the lock race.
- [ ] Add one rustdoc sentence stating that a passthrough to a deciding
  application is the intended shape, and that only mechanical resolution
  of the insert error is forbidden.
