# Reparenting in ROS and tf2: evidence and implications

Research date: 2026-09-20. This supplements the preserved
[independent assessment](INDEPENDENT.md) and [earlier report](REPORT.md).
The [consolidated assessment](CONSOLIDATED.md) incorporates the findings below.

## Direct answers

**Does tf2 support changing a frame's parent? Yes, deliberately.** Current
ROS 2 tf2 accepts transforms with a different parent for an existing child.
This happens through ordinary transform publication/insertion; it is not a
separate validated, history-discarding operation equivalent to this crate's
`Registry::reparent_frame`. Dynamic and static behavior differ substantially.
[BufferCore implementation](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/buffer_core.cpp)

**Has anybody used it? Yes.** Evidence includes implemented ROS 2 frame-editing
commands, a ROS 2 manipulation service, a firsthand account of static-frame
reparenting, and released support in TF-related tools. These establish actual
implementation and reported use, not prevalence or production reliability.

**Has anybody requested it? Yes.** There is an explicit upstream maintainer
request to enable reparenting, followed by a merged fix and a test, plus later
application requests. A search limited to new-feature issues in
`ros2/geometry2` misses both inherited functionality and requests in tools
that consume TF.

**Does that settle whether this crate should add it? No, but it strengthens
the case materially.** The earlier absence-of-use premise does not survive
this search. The remaining question is whether the crate's deliberately
different temporal contract fits the intended application.

## What current ROS 2 tf2 actually does

Source inspection used `ros2/geometry2` rolling commit
`141cfebad7b31b1163e56c21c8632f4336b2c3a7`. This is a source-and-test review,
not a claim that every released ROS distribution was executed or audited.

### Dynamic transforms

The cache stores a parent identifier per sample. `TimeCache::getData`
interpolates when the bracketing samples have the same parent; when they
differ, it returns the older sample without interpolation. A parent change
does not flush the cache. Ordinary retention and time-range restrictions
still apply. `getParent` also selects according to the requested time.
[TimeCache source](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/cache.cpp)

For example, with an old-parent sample at time 10 and a new-parent sample at
20, a lookup at 15 uses the older sample's parent and pose, assuming the
remaining chain can resolve. At 20 the new sample applies. This avoids
interpolating coordinates expressed in different parent frames; it does not
establish the physically correct trajectory between the samples. The
existing `ReparentingInterpolationProtection` test explicitly verifies the
older-pose behavior between changes.
[Dynamic reparenting test](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/test/cache_unittest.cpp#L351)

### Static transforms

`StaticCache` stores one transform; replacement changes the parent and pose
served for every requested instant. It does not record when a static
reconfiguration occurred.
[StaticCache source](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/static_cache.cpp)

The static broadcaster replaces its stored outgoing entry by child name,
so a new parent for that child supersedes the previous entry.
[Static broadcaster source](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2_ros/src/static_transform_broadcaster.cpp)
The ROS 2 `multiple_parent_test` first connects a child to one tree, republishes
it under another parent, and checks that the old connection disappears.
Despite its name, this tests successive parents, not simultaneous parents.
[Static integration test](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/test_tf2/test/test_static_publisher.cpp#L174)

### Comparison with this branch

| Property | Inspected ROS 2 tf2 | `transforms::Registry::reparent_frame` |
| --- | --- | --- |
| How invoked | Ordinary incoming transform with a different parent | Separate, explicit method; ordinary insertion rejects the change |
| Dynamic history | Parent stored per retained sample | Moved edge replaced by one seed sample |
| Static history | Replacement applies at all times | Same all-times replacement consequence |
| Pose preservation | Publisher supplies new coordinates | Caller supplies new coordinates |
| Kind changes | Insertion can replace static/dynamic cache type | Existing kind preserved; cross-kind seed rejected |
| Cycle protection | Insertion lacks topology-cycle preflight; traversal has a depth guard | Proposed edge checked before committing |
| Atomicity relevant here | Local insertion is mutex-protected | Returned validation errors preserve the old buffer and history |
| Distributed ownership | Applications must coordinate broadcasters | Applications/wrappers must authorize explicit moves |

The tf2 implementation does contain validation and locking; describing it
as having no atomicity would be misleading. It does not provide this crate's
specific validated replacement contract.
[BufferCore insertion and traversal](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2/src/buffer_core.cpp)
The crate side follows its [registry](../../src/core/registry/mod.rs) and
[buffer](../../src/core/buffer/mod.rs).

## Evidence of requests, implementations, and use

The rows distinguish ROS 1, ROS 2, and adjacent tooling. Several artifacts
come from the same project or author; they are not independent user counts.

| Date | Evidence | What it establishes |
| --- | --- | --- |
| 2013-10 | In `ros/geometry2` issue 28, Tully Foote specifies child-only matching so the static broadcaster can support reparenting. PR 29 implements the correction and a test. | Explicit upstream design intention and implementation, originally ROS 1 tf2. [Comment](https://github.com/ros/geometry2/issues/28#issuecomment-26234399), [merged PR](https://github.com/ros/geometry2/pull/29) |
| 2016-11 | The RQT Frame Editor author announces a tool used to arrange frames, including changing parents while preserving global position. | An implemented workflow and author-reported utility, originally ROS 1. [Announcement](https://discourse.openrobotics.org/t/new-rqt-plugin-to-create-manage-and-arrange-tf-frames/880) |
| 2019-03 | `static_transform_mux` changes its cache key from parent-and-child to child alone, specifically allowing restructuring. The change ships in 1.1.0. | Released ROS utility support, beyond an abstract discussion. [Commit](https://github.com/tradr-project/static_transform_mux/commit/cdc7d4d489c9d81443d1b190bbf9e5c6beea0db0), [release history](https://index.ros.org/p/static_transform_mux/) |
| 2021-04 | In a tf2 static-frame deletion discussion, Martin Pecka reports already reparenting deleted transforms under a `nonexistent` frame. | Firsthand application use of ROS 1 tf2 reparenting, as a deletion workaround. [Comment](https://github.com/ros/geometry2/issues/370#issuecomment-825582343) |
| 2021-12 | Foxglove 0.23.1 adds support for incoming `/tf` transforms to reparent an existing connection. | Shipped TF visualization support; not a tf2 core change or user-count measurement. [Release note](https://docs.foxglove.dev/changelog/foxglove/v0.23.1) |
| By 2024-05 | SequencePlanner's ROS 2 Scene Manipulation Service contains an explicit reparent command and a documented handling workflow. | Working-source implementation in a ROS 2 application, with narrower time requirements. [Repository at inspected commit](https://github.com/sequenceplanner/scene_manipulation_service/tree/a315fa67236d905dd42b9285f549b707deb06d06) |
| 2025-05 | RQT Frame Editor announces a ROS 2 version and parent-editing support. | Continuation of the editing workflow in ROS 2, not only historical ROS 1 discussion. [Release announcement](https://discourse.openrobotics.org/t/new-release-rqt-frame-editor-noetic-humble/43905) |
| 2025-06 | A Rerun user explicitly requests changing an object's parent from world to gripper for ROS-based pick-and-place, retaining its children. | Concrete recent demand in ROS visualization tooling. This is a 2025 comment on a broader 2023 issue, not a tf2 feature request. [Request](https://github.com/rerun-io/rerun/issues/1533#issuecomment-2927587600) |

### The two most relevant ROS 2 implementations

**RQT Frame Editor.** At `jazzy-devel` commit
`d9aef75fad759b7d41c03cd093499a52000ecc60`, `Command_SetParent` uses
`tf2_ros` lookup to compute the child's pose relative to the proposed parent,
updates parent and pose, and retains values for undo. Its service interface
exposes that command and its TF interface publishes the edited frames.
This directly demonstrates a deliberate application command layered over
tf2, with pose preservation and undo handled by the application.
[Command](https://github.com/ipa320/rqt_frame_editor_plugin/blob/d9aef75fad759b7d41c03cd093499a52000ecc60/frame_editor_py/commands.py#L331),
[service](https://github.com/ipa320/rqt_frame_editor_plugin/blob/d9aef75fad759b7d41c03cd093499a52000ecc60/frame_editor_py/interface_services.py),
[publication](https://github.com/ipa320/rqt_frame_editor_plugin/blob/d9aef75fad759b7d41c03cd093499a52000ecc60/frame_editor_py/interface_tf.py)

**Scene Manipulation Service.** Its documented item-handling sequence moves
an item between camera, bin, tool, and platform parents. A tool similarly
moves between stand and robot. The application intentionally keeps a latest
state map rather than interpolation history.
[Workflow and buffer design](https://github.com/sequenceplanner/scene_manipulation_service/blob/a315fa67236d905dd42b9285f549b707deb06d06/README.md)
Its Rust `reparent_frame` implementation checks ownership of the broadcast
frame, checks for a cycle, calculates a pose under the new parent, and
preserves frame metadata. It publishes into the ROS 2 TF ecosystem but owns
its own mutation store; it is not evidence of a native tf2 reparent method.
Nor does source availability prove production deployment or correctness.
[Implementation](https://github.com/sequenceplanner/scene_manipulation_service/blob/a315fa67236d905dd42b9285f549b707deb06d06/scene_manipulation_service/src/core/sms.rs#L469)

**Inference for this crate:** these are concrete examples of the application
authority and explicit command anticipated by the proposed API. The second
also shows that historical topology is not universally required. Neither
project establishes demand for this exact Rust crate or automatically
accepts its dynamic coverage collapse to a single seed.

## Broader questions that change the assessment

### Is sequential reparenting the same request as multiple parents?

No. Keeping two simultaneous incoming edges creates ambiguity; changing
which single parent owns an edge is a different operation. The 2020
[multiple-parent issue](https://github.com/ros/geometry2/issues/437) concerns
the former. It is not evidence that tf2 rejects the latter.

Earlier public questions are also mixed. A 2011 user combining gmapping and
AMCL wanted parent-source selection. The answer suggested inversion or a
selective relay and welcomed relay/mux patches. A 2019 camera/SLAM question
involved reversing an edge and creating a loop; the answer favored a stable
tree and correctly composed transforms. These establish requests and
misunderstandings, not two blanket rejections of intentional reparenting.
[2011 question and answer](https://robotics.stackexchange.com/questions/30321/tf-mux-or-multiple-parents),
[2019 question and answer](https://robotics.stackexchange.com/questions/90382/how-to-change-parent-in-tf-tree)

### Does every attachment or map change require a TF parent change?

No. MoveIt represents attachment through planning-scene collision objects;
its example removes an object from the world and attaches it to a robot
link. That does not itself require changing an existing TF edge.
[MoveIt ROS 2 tutorial](https://moveit.picknik.ai/main/doc/examples/planning_scene_ros_api/planning_scene_ros_api_tutorial.html)

REP-105 explicitly assigns odom-frame reparenting during map transitions to
the localization authority. This legitimizes a domain use; it does not mean
all map switches change frame names or topology.
[REP-105](https://raw.githubusercontent.com/ros-infrastructure/rep/master/rep-0105.rst)
In inspected Nav2 AMCL source, `mapReceived`/`handleMapMessage` replace map
data while `sendMapToOdomTransform` uses configured global and odom frame
identifiers. **Inference:** a map reload can retain the same named edge.
[Nav2 AMCL source](https://raw.githubusercontent.com/ros-navigation/navigation2/main/nav2_amcl/src/amcl_node.cpp)

### Can a static transform model a timed attach/detach event safely?

Not if historical queries must distinguish before and after. In the static
deletion discussion, the tf2 maintainer explains static updates as revised
estimates applying to all time and recommends dynamic data for intermittent
attachments. The deletion workaround also leaves supposedly deleted frames
mutually connected beneath their new shared parent. It is evidence of use,
not a general removal recipe for this crate.
[Static semantics discussion](https://github.com/ros/geometry2/issues/370#issuecomment-469491451),
[workaround limitation](https://github.com/ros/geometry2/issues/370#issuecomment-825582964)

For `transforms`, a static reparent is therefore suitable for replacing a
configuration understood to apply at all query times. A timed event with
historical requirements needs a different model. Calling the existing
method cannot make that distinction appear in stored history.

### Why do some ROS diagnostics call reparenting an error?

The ROS 1 `roswtf` TF check flags observed changes of a child's parent as
reparenting contention. This is a useful diagnostic for competing
broadcasters, not proof that the core cannot perform an intentional change.
[Diagnostic implementation](https://raw.githubusercontent.com/ros/geometry/noetic-devel/tf/src/tf/tfwtf.py)

An application-authorized move and two uncontrolled publishers fighting
over a child are materially different. A bridge should not interpret every
parent-conflict error as permission to drop history and switch parents.

### Does one local reparent call coordinate a ROS system?

No. ROS 2 listeners insert received transforms into their own buffers;
the listener processes the transforms individually. Local buffer locking
does not make several subscribers, publishers, or edges change together.
[Transform listener](https://github.com/ros2/geometry2/blob/141cfebad7b31b1163e56c21c8632f4336b2c3a7/tf2_ros/src/transform_listener.cpp)

Likewise, this crate can protect a local registry's invariants and preserve
its old state on a returned error. Choosing the authorized publisher,
quiescing old-parent messages, synchronizing pose calculation, and
coordinating multi-edge changes remain application responsibilities.

### Do callers need automatic world-pose preservation in the core?

The editor and scene service show that it is useful application behavior.
They do not establish that it belongs inside this crate's replacement
operation. A caller can obtain the transform in the intended new parent
frame before replacement, where the old tree connects those frames at an
acceptable time. Cross-tree moves instead need a supplied relationship.
The application also decides the reference time and coordinates any shared
state. None of this requires exporting private buffers.

## Revised value judgment

The evidence supports a **conditional yes** for this crate's narrow explicit
operation. There are concrete users of the underlying operation, including
ROS 2 tools, and explicit commands with application-owned poses are an
established pattern. Preserving the existing registry on rejection is a
useful improvement over destructive remove-then-add, independently of tf2
compatibility. The exact generic guarantee needs sealed buffer state.

The strongest argument against inclusion is now **contract mismatch and
scope**, not absence of real workflows. A tf2-compatible replay buffer would
need temporal parent history that this method deliberately does not offer.
An application owning complete configuration may prefer rebuilding and
swapping a registry. Fixed-topology applications gain little, and every new
public method carries maintenance and misuse costs.

The next useful validation is one representative explicit reconfiguration
workflow that supplies the new pose, needs rejection to preserve old state,
and accepts the time semantics. Retain the narrow design for that purpose;
do not broaden it to historical topology, automatic ingestion changes, or
multi-edge transactions merely because adjacent tools offer more features.
There is no longer a sound basis for waiting for the first evidence that
anyone reparents frames at all. Demand for this exact crate/API and the
frequency of need remain unmeasured.

## Search scope, limits, and verification

Searches covered ROS 2 and ROS 1 geometry repositories, GitHub issue comments
and pull requests, ROS Answers/Robotics Stack Exchange, Open Robotics
Discourse, ROS package histories, frame editors, visualization tools,
manipulation applications, REP-105, MoveIt, and Nav2. Terms included
`reparent`, `reparenting`, `re-parent`, `change parent`, `multiple parents`,
static replacement, attachment, and map switching. Full selected GitHub
comment threads were fetched through the API when web rendering omitted
them. Source inspection followed the strongest results into implementations
and tests instead of treating search snippets as evidence.

The narrowly scoped ROS 2 geometry issue searches produced little direct
new-feature demand. That is unsurprising for behavior already inherited
from tf2 and is not a census of user needs. tf2 cache-kind-switching issue
151, unrelated search hits, and simultaneous-parent requests were not
counted as direct reparent-feature demand. Multiple artifacts from the frame
editor, and the static mux/deletion discussion, were not treated as distinct
independent deployments.

No ROS installation, upstream test execution, field reliability assessment,
or adoption survey was performed. Current implementation claims refer to
the pinned revisions above; historical examples remain labeled as such.
The unchanged crate branch passed `tests/test_all.sh` on 2026-09-20,
ending with `GATE PASSED (all steps completed)`. The original independent
assessment and earlier report are preserved to keep the change in judgment
auditable.
