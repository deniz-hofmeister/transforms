//! `Registry::insert_all`: equal to the insert loop, and all or nothing.

use crate::{
    BatchError, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3,
};
use alloc::{format, string::String, vec, vec::Vec};
use core::{error::Error, time::Duration};

/// One second, in nanoseconds.
const SEC: i64 = 1_000_000_000;

fn at(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

/// A dynamic sample translated by `x` along the x-axis.
fn sample(
    parent: &str,
    child: &str,
    nanos: i64,
    x: f64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(at(nanos)),
    )
    .unwrap()
}

/// A static transform translated by `x` along the x-axis.
fn fixed(
    parent: &str,
    child: &str,
    x: f64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap()
}

/// The translation of a lookup, or its error.
fn position(
    registry: &Registry,
    target: &str,
    source: &str,
    nanos: i64,
) -> Result<Vector3, LookupError> {
    registry
        .lookup(target, source, at(nanos))
        .map(|transform| transform.translation())
}

fn along_x(x: f64) -> Vector3 {
    Vector3::new(x, 0.0, 0.0)
}

fn unknown(frame: &str) -> LookupError {
    LookupError::UnknownFrame {
        frame: frame.into(),
    }
}

fn out_of_range(
    frame: &str,
    requested: i64,
    earliest: i64,
    latest: i64,
) -> LookupError {
    LookupError::OutOfRange {
        frame: frame.into(),
        requested: at(requested),
        earliest: at(earliest),
        latest: at(latest),
    }
}

fn rejected(
    index: usize,
    error: InsertError,
) -> Result<(), BatchError> {
    Err(BatchError { index, error })
}

fn expired(
    frame: &str,
    stamp: i64,
    newest: i64,
) -> InsertError {
    InsertError::Expired {
        frame: frame.into(),
        stamp: at(stamp),
        newest: at(newest),
    }
}

fn parent_mismatch(
    frame: &str,
    pinned: &str,
    offered: &str,
) -> InsertError {
    InsertError::ParentMismatch {
        frame: frame.into(),
        pinned: pinned.into(),
        offered: offered.into(),
    }
}

fn kind_mismatch(
    frame: &str,
    incoming: Stamp,
) -> InsertError {
    InsertError::KindMismatch {
        frame: frame.into(),
        incoming,
    }
}

fn cycle(
    child: &str,
    parent: &str,
) -> InsertError {
    InsertError::Cycle {
        child: child.into(),
        parent: parent.into(),
    }
}

/// A registry state and a batch to apply to it.
struct Scenario {
    name: &'static str,
    max_age: Option<Duration>,
    /// Inserted one at a time before the batch.
    stored: Vec<Transform>,
    /// Applied with `remove_samples_through` after `stored`.
    drained_through: Option<Timestamp>,
    batch: Vec<Transform>,
}

impl Scenario {
    fn new(
        name: &'static str,
        max_age: Option<Duration>,
        stored: Vec<Transform>,
        batch: Vec<Transform>,
    ) -> Self {
        Self {
            name,
            max_age,
            stored,
            drained_through: None,
            batch,
        }
    }

    /// The registry before the batch.
    fn registry(&self) -> Registry {
        let mut registry = match self.max_age {
            Some(max_age) => Registry::with_max_age(max_age),
            None => Registry::unbounded(),
        };
        for transform in &self.stored {
            registry.insert(transform).unwrap();
        }
        if let Some(cutoff) = self.drained_through {
            registry.remove_samples_through(cutoff);
        }
        registry
    }

    /// Every name the scenario mentions, and one it does not.
    fn names(&self) -> Vec<String> {
        let mut names = vec![String::from("nowhere")];
        for transform in self.stored.iter().chain(&self.batch) {
            for name in [transform.target(), transform.source()] {
                if !names.iter().any(|known| known == name) {
                    names.push(name.into());
                }
            }
        }
        names
    }

    /// Every instant the scenario mentions, each with its two neighbours.
    fn instants(&self) -> Vec<Timestamp> {
        let mut instants = vec![at(0)];
        for transform in self.stored.iter().chain(&self.batch) {
            if let Stamp::At(stamp) = transform.stamp() {
                let nanos = stamp.as_nanos();
                instants.extend([nanos.saturating_sub(1), nanos, nanos.saturating_add(1)].map(at));
            }
        }
        instants
    }

    /// Asserts that two registries of this scenario answer alike: the same
    /// result for every lookup and every `latest_common_time` over the
    /// scenario's names and instants. Their `Debug` output is not compared:
    /// it lists frames in slot order, which no caller can observe.
    fn assert_same(
        &self,
        left: &Registry,
        right: &Registry,
    ) {
        let name = self.name;
        let names = self.names();
        let instants = self.instants();
        for target in &names {
            for source in &names {
                assert_eq!(
                    left.latest_common_time(target, source),
                    right.latest_common_time(target, source),
                    "{name}: latest_common_time({target}, {source})"
                );
                for &instant in &instants {
                    assert_eq!(
                        left.lookup(target, source, instant),
                        right.lookup(target, source, instant),
                        "{name}: lookup({target}, {source}, {instant})"
                    );
                }
            }
        }
    }
}

/// The insert loop `insert_all` is specified against: it stops at the first
/// rejected member and reports its index.
fn insert_each(
    registry: &mut Registry,
    batch: &[Transform],
) -> Result<(), BatchError> {
    for (index, transform) in batch.iter().enumerate() {
        registry
            .insert(transform)
            .map_err(|error| BatchError { index, error })?;
    }
    Ok(())
}

/// The retention window of the scenarios that have one.
const WINDOW: Option<Duration> = Some(Duration::from_secs(10));

/// Batches that build a tree, in either order, or repeat members.
fn ordering_scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("the empty batch on an empty registry", None, vec![], vec![]),
        Scenario::new(
            "the empty batch on a filled registry",
            WINDOW,
            vec![sample("map", "base", SEC, 1.0), fixed("base", "lidar", 0.5)],
            vec![],
        ),
        Scenario::new(
            "a chain created parent first",
            None,
            vec![],
            vec![
                sample("map", "base", SEC, 1.0),
                fixed("base", "lidar", 0.5),
                sample("lidar", "lens", SEC, 0.25),
            ],
        ),
        Scenario::new(
            "a chain created child first",
            None,
            vec![],
            vec![
                sample("lidar", "lens", SEC, 0.25),
                fixed("base", "lidar", 0.5),
                sample("map", "base", SEC, 1.0),
            ],
        ),
        Scenario::new(
            "repeated members, the last one wins",
            None,
            vec![sample("map", "base", SEC, 1.0)],
            vec![
                sample("map", "base", SEC, 2.0),
                sample("map", "base", SEC, 3.0),
                fixed("base", "lidar", 0.5),
                fixed("base", "lidar", 0.75),
                sample("map", "arm", 2 * SEC, 4.0),
                sample("map", "arm", 2 * SEC, 5.0),
            ],
        ),
    ]
}

/// Batches that meet stored samples, a window or a drained frame.
fn history_scenarios() -> Vec<Scenario> {
    let mut drained = Scenario::new(
        "a drained frame restarts behind its old newest stamp",
        WINDOW,
        vec![
            sample("map", "base", 100 * SEC, 1.0),
            sample("map", "arm", 200 * SEC, 1.0),
        ],
        vec![
            sample("map", "base", 5 * SEC, 2.0),
            sample("map", "base", 3 * SEC, 3.0),
            sample("map", "arm", 195 * SEC, 4.0),
        ],
    );
    drained.drained_through = Some(at(100 * SEC));

    vec![
        Scenario::new(
            "a member evicts an earlier member",
            WINDOW,
            vec![],
            vec![
                sample("map", "base", 50 * SEC, 1.0),
                sample("map", "base", 100 * SEC, 2.0),
            ],
        ),
        Scenario::new(
            "members evict stored samples and each other",
            WINDOW,
            vec![
                sample("map", "base", 80 * SEC, 1.0),
                sample("map", "base", 95 * SEC, 2.0),
            ],
            vec![
                sample("map", "base", 100 * SEC, 3.0),
                sample("map", "base", 91 * SEC, 4.0),
                sample("map", "base", 105 * SEC, 5.0),
                sample("map", "base", 95 * SEC, 6.0),
            ],
        ),
        Scenario::new(
            "a known root gains a parent and its subtree new samples",
            None,
            vec![sample("a", "b", SEC, 1.0)],
            vec![
                sample("w", "a", SEC, 2.0),
                fixed("w", "c", 3.0),
                sample("a", "b", 2 * SEC, 4.0),
                sample("b", "d", 2 * SEC, 5.0),
            ],
        ),
        Scenario::new(
            "out of order samples on an unbounded registry",
            None,
            vec![sample("map", "base", 50 * SEC, 1.0)],
            vec![
                sample("map", "base", 100 * SEC, 2.0),
                sample("map", "base", SEC, 3.0),
                sample("map", "base", 75 * SEC, 4.0),
            ],
        ),
        Scenario::new(
            "two trees in one batch",
            WINDOW,
            vec![],
            vec![
                sample("map", "base", SEC, 1.0),
                sample("world", "robot", 500 * SEC, 2.0),
                sample("map", "base", 2 * SEC, 3.0),
            ],
        ),
        drained,
    ]
}

fn accepted_scenarios() -> Vec<Scenario> {
    let mut scenarios = ordering_scenarios();
    scenarios.extend(history_scenarios());
    scenarios
}

fn rejection(
    index: usize,
    error: InsertError,
) -> BatchError {
    BatchError { index, error }
}

/// Batches rejected for an edge, a parent or a kind.
fn pin_rejections() -> Vec<(Scenario, BatchError)> {
    vec![
        (
            Scenario::new(
                "a self edge",
                None,
                vec![],
                vec![
                    sample("map", "base", SEC, 1.0),
                    sample("base", "base", SEC, 1.0),
                ],
            ),
            rejection(
                1,
                InsertError::SelfEdge {
                    frame: "base".into(),
                },
            ),
        ),
        (
            Scenario::new(
                "a second parent for a stored frame",
                None,
                vec![sample("map", "base", SEC, 1.0)],
                vec![
                    sample("base", "lidar", SEC, 1.0),
                    sample("world", "base", SEC, 1.0),
                ],
            ),
            rejection(1, parent_mismatch("base", "map", "world")),
        ),
        (
            Scenario::new(
                "a second parent for a frame the batch creates",
                None,
                vec![sample("map", "base", SEC, 1.0)],
                vec![
                    sample("map", "arm", SEC, 1.0),
                    sample("map", "base", 2 * SEC, 1.0),
                    sample("world", "arm", SEC, 1.0),
                ],
            ),
            rejection(2, parent_mismatch("arm", "map", "world")),
        ),
        (
            Scenario::new(
                "the other kind for a stored frame",
                None,
                vec![fixed("base", "lidar", 0.5)],
                vec![
                    sample("map", "base", SEC, 1.0),
                    sample("base", "lidar", SEC, 1.0),
                ],
            ),
            rejection(1, kind_mismatch("lidar", Stamp::At(at(SEC)))),
        ),
        (
            Scenario::new(
                "a static member for a frame the batch creates dynamic",
                None,
                vec![],
                vec![sample("map", "arm", SEC, 1.0), fixed("map", "arm", 1.0)],
            ),
            rejection(1, kind_mismatch("arm", Stamp::Static)),
        ),
        (
            Scenario::new(
                "a dynamic member for a frame the batch creates static",
                None,
                vec![],
                vec![fixed("map", "arm", 1.0), sample("map", "arm", SEC, 1.0)],
            ),
            rejection(1, kind_mismatch("arm", Stamp::At(at(SEC)))),
        ),
        (
            Scenario::new(
                "a cycle through the batch's own edges",
                None,
                vec![],
                vec![
                    sample("a", "b", SEC, 1.0),
                    sample("b", "c", SEC, 1.0),
                    sample("c", "a", SEC, 1.0),
                ],
            ),
            rejection(2, cycle("a", "c")),
        ),
        (
            Scenario::new(
                "a cycle through a stored and a staged edge",
                None,
                vec![sample("a", "b", SEC, 1.0)],
                vec![sample("w", "a", SEC, 1.0), sample("b", "w", SEC, 1.0)],
            ),
            rejection(1, cycle("w", "b")),
        ),
    ]
}

/// Batches rejected for a stamp, and batches with several rejected members.
fn expiry_and_order_rejections() -> Vec<(Scenario, BatchError)> {
    vec![
        (
            Scenario::new(
                "a sample behind the stored window",
                WINDOW,
                vec![
                    sample("map", "base", 95 * SEC, 1.0),
                    sample("map", "base", 100 * SEC, 1.0),
                ],
                vec![
                    sample("map", "arm", SEC, 1.0),
                    sample("map", "base", 50 * SEC, 1.0),
                ],
            ),
            rejection(1, expired("base", 50 * SEC, 100 * SEC)),
        ),
        (
            Scenario::new(
                "a sample behind an earlier member",
                WINDOW,
                vec![sample("map", "base", 60 * SEC, 1.0)],
                vec![
                    sample("map", "base", 100 * SEC, 1.0),
                    sample("map", "base", 50 * SEC, 1.0),
                ],
            ),
            rejection(1, expired("base", 50 * SEC, 100 * SEC)),
        ),
        (
            Scenario::new(
                "two rejected members, the first is reported",
                None,
                vec![fixed("base", "lidar", 0.5)],
                vec![
                    sample("map", "base", SEC, 1.0),
                    sample("mast", "lidar", SEC, 1.0),
                    sample("base", "map", SEC, 1.0),
                    sample("x", "x", SEC, 1.0),
                ],
            ),
            rejection(1, parent_mismatch("lidar", "base", "mast")),
        ),
        (
            Scenario::new(
                "the first member is rejected",
                None,
                vec![sample("map", "base", SEC, 1.0)],
                vec![fixed("map", "base", 1.0), sample("map", "arm", SEC, 1.0)],
            ),
            rejection(0, kind_mismatch("base", Stamp::Static)),
        ),
    ]
}

fn rejected_scenarios() -> Vec<(Scenario, BatchError)> {
    let mut scenarios = pin_rejections();
    scenarios.extend(expiry_and_order_rejections());
    scenarios
}

#[test]
fn an_accepted_batch_leaves_what_the_insert_loop_leaves() {
    for scenario in accepted_scenarios() {
        let mut batched = scenario.registry();
        let mut looped = scenario.registry();

        assert_eq!(
            batched.insert_all(&scenario.batch),
            Ok(()),
            "{}",
            scenario.name
        );
        assert_eq!(
            insert_each(&mut looped, &scenario.batch),
            Ok(()),
            "{}",
            scenario.name
        );
        scenario.assert_same(&batched, &looped);
    }
}

#[test]
fn a_rejected_batch_reports_the_loops_first_error_and_changes_nothing() {
    for (scenario, expected) in rejected_scenarios() {
        let untouched = scenario.registry();
        let mut batched = scenario.registry();
        let mut looped = scenario.registry();

        assert_eq!(
            batched.insert_all(&scenario.batch),
            Err(expected.clone()),
            "{}",
            scenario.name
        );
        assert_eq!(
            insert_each(&mut looped, &scenario.batch),
            Err(expected),
            "{}",
            scenario.name
        );
        scenario.assert_same(&batched, &untouched);
    }
}

#[test]
fn a_batch_can_hang_frames_from_a_frame_it_creates() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", SEC, 1.0),
            sample("base", "arm", SEC, 2.0),
            fixed("arm", "tool", 4.0),
        ]),
        Ok(())
    );
    assert_eq!(position(&registry, "map", "tool", SEC), Ok(along_x(7.0)));
}

#[test]
fn a_failed_batch_leaves_no_edge_or_eviction_behind() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&sample("a", "b", 2 * SEC, 1.0)).unwrap();
    let before = format!("{registry:?}");

    // The first member alone would evict the sample at 1 s. The third
    // closes a cycle through the frame the second creates.
    assert_eq!(
        registry.insert_all(&[
            sample("a", "b", 10 * SEC, 2.0),
            sample("b", "c", 10 * SEC, 2.0),
            sample("c", "a", 10 * SEC, 2.0),
        ]),
        rejected(2, cycle("a", "c"))
    );

    assert_eq!(format!("{registry:?}"), before);
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(1.0)));
    assert_eq!(
        position(&registry, "a", "b", 10 * SEC),
        Err(out_of_range("b", 10 * SEC, SEC, 2 * SEC))
    );
    assert_eq!(position(&registry, "c", "c", SEC), Err(unknown("c")));
}

#[test]
fn a_failed_batch_on_an_empty_registry_makes_no_name_known() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", SEC, 1.0),
            fixed("base", "lidar", 0.5),
            sample("lidar", "lidar", SEC, 1.0),
        ]),
        rejected(
            2,
            InsertError::SelfEdge {
                frame: "lidar".into()
            }
        )
    );

    for name in ["map", "base", "lidar"] {
        assert_eq!(position(&registry, name, name, SEC), Err(unknown(name)));
        assert!(!registry.remove_frame(name));
    }
    assert_eq!(
        format!("{registry:?}"),
        format!("{:?}", Registry::unbounded())
    );
}

#[test]
fn a_batch_rejects_a_second_parent_for_a_frame_it_creates() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[
            sample("map", "arm", SEC, 1.0),
            sample("world", "arm", 2 * SEC, 1.0),
        ]),
        rejected(1, parent_mismatch("arm", "map", "world"))
    );
    assert_eq!(position(&registry, "map", "arm", SEC), Err(unknown("map")));
}

#[test]
fn a_batch_rejects_mixing_kinds_for_a_frame_it_creates() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[sample("map", "arm", SEC, 1.0), fixed("map", "arm", 1.0)]),
        rejected(1, kind_mismatch("arm", Stamp::Static))
    );
    assert_eq!(position(&registry, "map", "arm", SEC), Err(unknown("map")));
}

#[test]
fn a_batch_rejects_a_kind_conflict_with_a_stored_frame() {
    let mut registry = Registry::unbounded();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();

    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", SEC, 1.0),
            sample("base", "lidar", SEC, 9.0),
        ]),
        rejected(1, kind_mismatch("lidar", Stamp::At(at(SEC))))
    );
    assert_eq!(position(&registry, "base", "lidar", SEC), Ok(along_x(0.5)));
    assert_eq!(position(&registry, "map", "base", SEC), Err(unknown("map")));
}

#[test]
fn a_batch_detects_a_cycle_closed_across_its_members() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[
            sample("a", "b", SEC, 1.0),
            sample("b", "c", SEC, 1.0),
            sample("c", "d", SEC, 1.0),
            sample("d", "a", SEC, 1.0),
        ]),
        rejected(3, cycle("a", "d"))
    );

    // Two members that name each other as parent.
    assert_eq!(
        registry.insert_all(&[fixed("a", "b", 1.0), fixed("b", "a", 1.0)]),
        rejected(1, cycle("a", "b"))
    );

    // A stored root that the batch first hangs below its own descendant's
    // new child.
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    assert_eq!(
        registry.insert_all(&[sample("b", "c", SEC, 1.0), sample("c", "a", SEC, 1.0)]),
        rejected(1, cycle("a", "c"))
    );
    assert_eq!(position(&registry, "c", "c", SEC), Err(unknown("c")));
}

#[test]
fn the_order_of_a_batch_decides_expiry_against_the_running_newest_stamp() {
    let max_age = Duration::from_secs(10);

    // Oldest first: both are accepted at their turn, and the second evicts
    // the first before the call returns.
    let mut registry = Registry::with_max_age(max_age);
    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", 50 * SEC, 1.0),
            sample("map", "base", 100 * SEC, 2.0),
        ]),
        Ok(())
    );
    assert_eq!(
        position(&registry, "map", "base", 100 * SEC),
        Ok(along_x(2.0))
    );
    assert_eq!(
        position(&registry, "map", "base", 50 * SEC),
        Err(out_of_range("base", 50 * SEC, 100 * SEC, 100 * SEC))
    );

    // Newest first: the second is behind the first one's window. The
    // reported newest stamp is the earlier member's, which is not stored.
    let mut registry = Registry::with_max_age(max_age);
    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", 100 * SEC, 2.0),
            sample("map", "base", 50 * SEC, 1.0),
        ]),
        rejected(1, expired("base", 50 * SEC, 100 * SEC))
    );
    assert_eq!(
        position(&registry, "map", "base", 100 * SEC),
        Err(unknown("map"))
    );
    assert_eq!(
        format!("{registry:?}"),
        format!("{:?}", Registry::with_max_age(max_age))
    );
}

#[test]
fn the_running_newest_stamp_starts_from_the_stored_one() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    // An earlier member behind the stored newest stamp does not lower it.
    assert_eq!(
        registry.insert_all(&[
            sample("map", "base", 95 * SEC, 2.0),
            sample("map", "base", 89 * SEC, 3.0),
        ]),
        rejected(1, expired("base", 89 * SEC, 100 * SEC))
    );
    assert_eq!(
        position(&registry, "map", "base", 95 * SEC),
        Err(out_of_range("base", 95 * SEC, 100 * SEC, 100 * SEC))
    );
}

#[test]
fn one_expired_member_rejects_fresh_samples_of_other_frames() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    registry
        .insert(&sample("map", "arm", 100 * SEC, 1.0))
        .unwrap();

    let batch = [
        sample("map", "arm", 101 * SEC, 2.0),
        sample("map", "base", 50 * SEC, 2.0),
        sample("map", "wheel", 101 * SEC, 2.0),
    ];
    let error = registry.insert_all(&batch).unwrap_err();
    assert_eq!(
        error,
        BatchError {
            index: 1,
            error: expired("base", 50 * SEC, 100 * SEC),
        }
    );
    assert_eq!(
        position(&registry, "map", "arm", 101 * SEC),
        Err(out_of_range("arm", 101 * SEC, 100 * SEC, 100 * SEC))
    );
    assert_eq!(
        position(&registry, "wheel", "wheel", SEC),
        Err(unknown("wheel"))
    );

    // The documented fallback: resubmit without the member at `index`.
    let mut rest = batch.to_vec();
    rest.remove(error.index);
    assert_eq!(registry.insert_all(&rest), Ok(()));
    assert_eq!(
        position(&registry, "map", "arm", 101 * SEC),
        Ok(along_x(2.0))
    );
    assert_eq!(
        position(&registry, "map", "wheel", 101 * SEC),
        Ok(along_x(2.0))
    );
}

#[test]
fn a_batch_keeps_the_last_of_repeated_members() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert_all(&[
            sample("a", "b", SEC, 1.0),
            sample("a", "b", SEC, 2.0),
            sample("a", "b", SEC, 3.0),
            fixed("b", "c", 4.0),
            fixed("b", "c", 5.0),
        ]),
        Ok(())
    );
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(3.0)));
    assert_eq!(position(&registry, "b", "c", SEC), Ok(along_x(5.0)));
    assert_eq!(
        position(&registry, "a", "b", SEC + 1),
        Err(out_of_range("b", SEC + 1, SEC, SEC))
    );
}

#[test]
fn an_empty_batch_is_accepted_and_changes_nothing() {
    let mut registry = Registry::unbounded();
    assert_eq!(registry.insert_all(&[]), Ok(()));
    assert_eq!(position(&registry, "a", "a", SEC), Err(unknown("a")));

    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    let before = format!("{registry:?}");
    assert_eq!(registry.insert_all(&[]), Ok(()));
    assert_eq!(format!("{registry:?}"), before);
}

#[test]
fn a_failed_batch_does_not_disturb_the_next_one() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    let batch = [
        sample("map", "base", 100 * SEC, 1.0),
        sample("base", "arm", 100 * SEC, 2.0),
        sample("arm", "map", 100 * SEC, 3.0),
    ];
    assert_eq!(
        registry.insert_all(&batch),
        rejected(2, cycle("map", "arm"))
    );

    // Nothing of the failed batch is staged any more: the same frames take
    // other parents, another kind and an older stamp.
    assert_eq!(
        registry.insert_all(&[
            sample("arm", "map", SEC, 3.0),
            fixed("world", "arm", 2.0),
            sample("map", "base", SEC, 1.0),
        ]),
        Ok(())
    );
    assert_eq!(position(&registry, "world", "base", SEC), Ok(along_x(6.0)));
}

#[test]
fn a_batch_error_displays_its_index_and_sources_the_insert_error() {
    let mut registry = Registry::unbounded();
    let error = registry
        .insert_all(&[sample("map", "base", SEC, 1.0), sample("x", "x", SEC, 1.0)])
        .unwrap_err();

    assert_eq!(format!("{error}"), "batch member 1 was rejected");
    let source = error.source().unwrap();
    assert_eq!(
        source.downcast_ref::<InsertError>(),
        Some(&InsertError::SelfEdge { frame: "x".into() })
    );
    assert_eq!(format!("{source}"), "frame x cannot be its own parent");
    assert!(source.source().is_none());
}
