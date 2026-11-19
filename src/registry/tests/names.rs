//! The known-names rule: a name is known exactly while a stored edge names
//! it, as child or as parent. Observed through the public calls only.

use crate::{
    BatchError, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3,
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
    vec::Vec,
};
use core::time::Duration;

/// One second, in nanoseconds.
const SEC: i64 = 1_000_000_000;

fn at(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

fn stamp_of(nanos: Option<i64>) -> Stamp {
    nanos.map_or(Stamp::Static, |nanos| Stamp::At(at(nanos)))
}

/// A transform translated by `x` along the x-axis; static for `None`.
fn edge(
    parent: &str,
    child: &str,
    nanos: Option<i64>,
    x: f64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        stamp_of(nanos),
    )
    .unwrap()
}

fn sample(
    parent: &str,
    child: &str,
    nanos: i64,
    x: f64,
) -> Transform {
    edge(parent, child, Some(nanos), x)
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

/// Whether the registry knows `name`, asserting that every query agrees:
/// a known frame relative to itself is the identity and `Static`, and an
/// unknown one is `UnknownFrame` naming it.
fn is_known(
    registry: &Registry,
    name: &str,
) -> bool {
    let lookup = position(registry, name, name, 0);
    let latest = registry.latest_common_time(name, name);
    let across = registry
        .lookup_across(name, at(0), name, at(SEC), name)
        .map(|transform| transform.translation());
    if lookup == Ok(along_x(0.0)) {
        assert_eq!(latest, Ok(Stamp::Static), "{name}");
        assert_eq!(across, Ok(along_x(0.0)), "{name}");
        true
    } else {
        assert_eq!(lookup, Err(unknown(name)), "{name}");
        assert_eq!(latest, Err(unknown(name)), "{name}");
        assert_eq!(across, Err(unknown(name)), "{name}");
        false
    }
}

#[test]
fn an_empty_registry_knows_no_name() {
    for registry in [
        Registry::unbounded(),
        Registry::with_max_age(Duration::from_secs(1)),
    ] {
        for name in ["map", "a", ""] {
            assert!(!is_known(&registry, name), "{name:?}");
        }
        assert_eq!(position(&registry, "map", "base", 0), Err(unknown("map")));
    }
}

#[test]
fn the_empty_name_is_never_known() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();

    assert!(!is_known(&registry, ""));
    assert_eq!(position(&registry, "", "base", SEC), Err(unknown("")));
    assert_eq!(position(&registry, "map", "", SEC), Err(unknown("")));
    assert!(!registry.remove_frame(""));
}

#[test]
fn an_edge_makes_its_child_and_its_parent_known() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();

    assert!(is_known(&registry, "map"));
    assert!(is_known(&registry, "base"));
    assert!(!is_known(&registry, "Map"));
    assert!(!is_known(&registry, "base "));
}

#[test]
fn repeated_upserts_do_not_count_the_parent_twice() {
    let mut registry = Registry::with_max_age(Duration::from_secs(5));
    for step in 0..500_i32 {
        let nanos = i64::from(step % 50) * SEC / 10 + i64::from(step) * SEC;
        registry
            .insert(&sample("map", "base", nanos, f64::from(step)))
            .unwrap();
        // The same stamp again: an upsert.
        registry
            .insert(&sample("map", "base", nanos, f64::from(step) + 0.5))
            .unwrap();
        registry.insert(&edge("base", "lidar", None, 0.5)).unwrap();
    }

    // One edge named `map`, however many samples went through it.
    assert!(registry.remove_frame("base"));
    assert!(!is_known(&registry, "map"));
    // `base` is still the parent of `lidar`.
    assert!(is_known(&registry, "base"));
    assert!(registry.remove_frame("lidar"));
    assert!(!is_known(&registry, "base"));
    assert!(!is_known(&registry, "lidar"));
    assert_eq!(
        format!("{registry:?}"),
        format!("{:?}", Registry::with_max_age(Duration::from_secs(5)))
    );
}

#[test]
fn a_root_gaining_an_edge_stays_one_known_frame() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();
    registry.insert(&sample("world", "map", SEC, 2.0)).unwrap();

    assert_eq!(position(&registry, "world", "base", SEC), Ok(along_x(3.0)));

    // `map` is named by two edges now. Removing either leaves it known.
    assert!(registry.remove_frame("map"));
    assert!(is_known(&registry, "map"));
    assert!(!is_known(&registry, "world"));
    assert!(registry.remove_frame("base"));
    assert!(!is_known(&registry, "map"));
    assert!(!is_known(&registry, "base"));
}

#[test]
fn an_internal_frame_stays_known_while_a_descendant_names_it() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    registry.insert(&sample("odom", "base", SEC, 2.0)).unwrap();
    registry.insert(&sample("odom", "arm", SEC, 4.0)).unwrap();

    assert!(registry.remove_frame("odom"));
    assert!(is_known(&registry, "odom"));
    assert!(!is_known(&registry, "map"));

    // One of two children gone: still named.
    assert!(registry.remove_frame("base"));
    assert!(is_known(&registry, "odom"));
    assert_eq!(position(&registry, "odom", "arm", SEC), Ok(along_x(4.0)));

    // The last one gone: both endpoints of the last edge are reclaimed.
    assert!(registry.remove_frame("arm"));
    for name in ["map", "odom", "base", "arm"] {
        assert!(!is_known(&registry, name), "{name}");
    }
    assert_eq!(
        format!("{registry:?}"),
        format!("{:?}", Registry::unbounded())
    );
}

#[test]
fn removing_the_last_edge_reclaims_both_endpoints_and_they_can_return() {
    let mut registry = Registry::unbounded();
    for round in 0..50_i32 {
        let x = f64::from(round);
        registry.insert(&sample("map", "base", SEC, x)).unwrap();
        assert_eq!(position(&registry, "map", "base", SEC), Ok(along_x(x)));

        assert!(registry.remove_frame("base"));
        assert!(!is_known(&registry, "map"));
        assert!(!is_known(&registry, "base"));

        // The same two names, the other way round and of the other kind.
        registry.insert(&edge("base", "map", None, x)).unwrap();
        assert_eq!(position(&registry, "base", "map", SEC), Ok(along_x(x)));
        assert!(registry.remove_frame("map"));
        assert!(!is_known(&registry, "map"));
        assert!(!is_known(&registry, "base"));
    }
}

#[test]
fn a_drained_edge_still_makes_its_names_known() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();
    registry.remove_samples_through(Timestamp::MAX);

    assert!(is_known(&registry, "map"));
    assert!(is_known(&registry, "base"));
    assert!(registry.remove_frame("base"));
    assert!(!is_known(&registry, "map"));
    assert!(!is_known(&registry, "base"));
}

#[test]
fn a_failed_insert_batch_or_reparent_makes_no_name_known() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    let before = format!("{registry:?}");

    assert!(
        registry
            .insert(&sample("ghost", "ghost", SEC, 1.0))
            .is_err()
    );
    assert!(
        registry
            .insert(&sample("ghost", "base", 100 * SEC, 1.0))
            .is_err()
    );
    assert!(registry.insert(&sample("base", "map", SEC, 1.0)).is_err());
    assert!(
        registry
            .insert_all(&[
                sample("ghost", "phantom", SEC, 1.0),
                sample("phantom", "spectre", SEC, 1.0),
                sample("map", "base", SEC, 1.0),
            ])
            .is_err()
    );
    assert!(
        registry
            .reparent(&sample("ghost", "phantom", SEC, 1.0))
            .is_err()
    );
    assert!(
        registry
            .reparent(&sample("ghost", "map", SEC, 1.0))
            .is_err()
    );
    assert!(
        registry
            .reparent(&edge("ghost", "base", None, 1.0))
            .is_err()
    );

    for name in ["ghost", "phantom", "spectre"] {
        assert!(!is_known(&registry, name), "{name}");
        assert!(!registry.remove_frame(name));
    }
    assert_eq!(format!("{registry:?}"), before);
}

#[test]
fn a_failed_batch_does_not_corrupt_later_inserts_or_lookups() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();
    registry.insert(&edge("base", "lidar", None, 0.5)).unwrap();
    let reference = registry.lookup("map", "lidar", at(SEC));

    for round in 0..20_i32 {
        // Stages three new frames and a new parent for the root, then
        // fails on the last member.
        let error = registry
            .insert_all(&[
                sample("world", "map", SEC, 1.0),
                sample("lidar", "lens", SEC, 1.0),
                sample("lens", "filter", SEC, 1.0),
                sample("filter", "world", SEC, 1.0),
            ])
            .unwrap_err();
        assert_eq!(
            error,
            BatchError {
                index: 3,
                error: InsertError::Cycle {
                    child: "world".into(),
                    parent: "filter".into(),
                },
            }
        );
        for name in ["world", "lens", "filter"] {
            assert!(!is_known(&registry, name), "round {round}: {name}");
        }
        assert_eq!(registry.lookup("map", "lidar", at(SEC)), reference);
    }

    // The names the failed batches staged are free for any parent and kind.
    registry
        .insert_all(&[
            edge("filter", "world", None, 1.0),
            edge("world", "map", None, 2.0),
            sample("base", "lens", SEC, 4.0),
        ])
        .unwrap();
    assert_eq!(position(&registry, "filter", "lens", SEC), Ok(along_x(8.0)));
    assert_eq!(registry.lookup("map", "lidar", at(SEC)), reference);
}

/// A small tree whose transforms have rotations, so that its lookups
/// exercise more than exact additions.
fn anchored_tree() -> Registry {
    let turn = Rotation::from_wxyz(0.6, 0.0, 0.0, 0.8).unwrap();
    let tilt = Rotation::from_wxyz(0.5, 0.5, -0.5, 0.5).unwrap();
    let mut registry = Registry::unbounded();
    for (parent, child, translation, rotation, nanos) in [
        ("world", "robot", Vector3::new(1.0, 2.0, 3.0), turn, Some(0)),
        (
            "world",
            "robot",
            Vector3::new(1.5, 2.5, 3.5),
            tilt,
            Some(SEC),
        ),
        ("robot", "camera", Vector3::new(0.1, 0.2, 0.3), tilt, None),
        (
            "world",
            "beacon",
            Vector3::new(-4.0, 0.5, 0.0),
            turn,
            Some(0),
        ),
        (
            "world",
            "beacon",
            Vector3::new(-5.0, 0.5, 1.0),
            turn,
            Some(SEC),
        ),
    ] {
        registry
            .insert(&Transform::new(parent, child, translation, rotation, stamp_of(nanos)).unwrap())
            .unwrap();
    }
    registry
}

/// Every lookup among the anchored tree's frames, at a stored instant and
/// at an interpolated one.
fn anchored_answers(registry: &Registry) -> Vec<Result<Transform, LookupError>> {
    let names = ["world", "robot", "camera", "beacon"];
    let mut answers = Vec::new();
    for target in names {
        for source in names {
            for nanos in [0, SEC / 3, SEC] {
                answers.push(registry.lookup(target, source, at(nanos)));
            }
        }
    }
    assert!(answers.iter().all(Result::is_ok));
    answers
}

#[test]
fn reused_slots_leave_lookups_between_unrelated_frames_unchanged() {
    let mut registry = anchored_tree();
    let reference = anchored_answers(&registry);
    let debug = format!("{registry:?}");

    for round in 0..300_i32 {
        let x = f64::from(round);
        let nanos = i64::from(round) * SEC;
        // A second tree grows, is rearranged and is torn down again. Its
        // names take the slots the previous round freed, in an order that
        // changes with the round.
        let names = [
            format!("tmp_{}", round % 7),
            format!("tmp_{}", (round + 1) % 7),
            format!("tmp_{}", (round + 2) % 7),
            format!("tmp_{}", (round + 3) % 7),
        ];
        let [top, upper, lower, leaf] = &names;
        registry.insert(&sample(top, upper, nanos, x)).unwrap();
        registry.insert(&sample(upper, lower, nanos, x)).unwrap();
        registry.insert(&edge(lower, leaf, None, x)).unwrap();
        assert_eq!(reference, anchored_answers(&registry), "round {round}");

        assert_eq!(
            registry.reparent(&sample(leaf, upper, nanos, x)),
            Err(InsertError::Cycle {
                child: upper.clone(),
                parent: leaf.clone(),
            })
        );
        registry
            .reparent(&sample("camera", lower, nanos, x))
            .unwrap();
        registry.reparent(&edge(top, leaf, None, x)).unwrap();
        assert_eq!(reference, anchored_answers(&registry), "round {round}");
        assert_eq!(
            position(&registry, "robot", lower, nanos).map(|v| v.x.is_finite()),
            Ok(true)
        );

        let removal_order = if round % 2 == 0 {
            [upper, lower, leaf]
        } else {
            [leaf, upper, lower]
        };
        for name in removal_order {
            assert!(registry.remove_frame(name), "round {round}: {name}");
        }
        for name in &names {
            assert!(!is_known(&registry, name), "round {round}: {name}");
        }
        assert_eq!(reference, anchored_answers(&registry), "round {round}");
        assert_eq!(format!("{registry:?}"), debug, "round {round}");
    }
}

/// A fixed-seed linear congruential generator.
struct Lcg(u64);

impl Lcg {
    fn draw(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap()
    }

    fn below(
        &mut self,
        bound: usize,
    ) -> usize {
        self.draw() % bound
    }

    fn pick<T: Copy>(
        &mut self,
        items: &[T],
    ) -> T {
        items[self.below(items.len())]
    }
}

/// What the model expects one edge to hold.
#[derive(Clone)]
enum Held {
    Fixed(f64),
    Samples(BTreeMap<i64, f64>),
}

#[derive(Clone)]
struct ModelEdge {
    parent: &'static str,
    held: Held,
}

/// One transform of the stress test: `stamp` is `None` for a static one.
#[derive(Clone, Copy)]
struct Offer {
    parent: &'static str,
    child: &'static str,
    stamp: Option<i64>,
    x: f64,
}

impl Offer {
    fn transform(self) -> Transform {
        edge(self.parent, self.child, self.stamp, self.x)
    }
}

/// An independent statement of the registry's write side: edges by child
/// name, with the documented checks in the documented order.
#[derive(Clone)]
struct Model {
    max_age: Option<i64>,
    edges: BTreeMap<&'static str, ModelEdge>,
}

impl Model {
    fn known(&self) -> BTreeSet<&'static str> {
        self.edges
            .iter()
            .flat_map(|(&child, edge)| [child, edge.parent])
            .collect()
    }

    /// Whether `ancestor` is `from` or lies above it.
    fn is_at_or_above(
        &self,
        ancestor: &str,
        from: &'static str,
    ) -> bool {
        let mut current = from;
        loop {
            if current == ancestor {
                return true;
            }
            match self.edges.get(current) {
                Some(edge) => current = edge.parent,
                None => return false,
            }
        }
    }

    fn insert(
        &mut self,
        offer: Offer,
    ) -> Result<(), InsertError> {
        let Offer {
            parent,
            child,
            stamp,
            x,
        } = offer;
        if parent == child {
            return Err(InsertError::SelfEdge {
                frame: child.into(),
            });
        }
        let max_age = self.max_age;
        let Some(stored) = self.edges.get_mut(child) else {
            if self.is_at_or_above(child, parent) {
                return Err(InsertError::Cycle {
                    child: child.into(),
                    parent: parent.into(),
                });
            }
            self.edges.insert(
                child,
                ModelEdge {
                    parent,
                    held: seeded(stamp, x),
                },
            );
            return Ok(());
        };
        if stored.parent != parent {
            return Err(InsertError::ParentMismatch {
                frame: child.into(),
                pinned: stored.parent.into(),
                offered: parent.into(),
            });
        }
        match (&mut stored.held, stamp) {
            (Held::Fixed(pose), None) => *pose = x,
            (Held::Samples(samples), Some(nanos)) => {
                let newest = samples.keys().next_back().copied();
                if let (Some(max_age), Some(newest)) = (max_age, newest) {
                    if nanos < newest - max_age {
                        return Err(InsertError::Expired {
                            frame: child.into(),
                            stamp: at(nanos),
                            newest: at(newest),
                        });
                    }
                }
                samples.insert(nanos, x);
                if let Some(max_age) = max_age {
                    let cutoff = newest.map_or(nanos, |newest| newest.max(nanos)) - max_age;
                    samples.retain(|&stored, _| stored >= cutoff);
                }
            }
            _ => {
                return Err(InsertError::KindMismatch {
                    frame: child.into(),
                    incoming: stamp_of(stamp),
                });
            }
        }
        Ok(())
    }

    fn insert_all(
        &mut self,
        offers: &[Offer],
    ) -> Result<(), BatchError> {
        let mut staged = self.clone();
        for (index, &offer) in offers.iter().enumerate() {
            staged
                .insert(offer)
                .map_err(|error| BatchError { index, error })?;
        }
        *self = staged;
        Ok(())
    }

    fn reparent(
        &mut self,
        seed: Offer,
    ) -> Result<(), InsertError> {
        let Offer {
            parent,
            child,
            stamp,
            x,
        } = seed;
        let Some(stored) = self.edges.get(child) else {
            return Err(if self.known().contains(child) {
                InsertError::NoParent {
                    frame: child.into(),
                }
            } else {
                InsertError::UnknownFrame {
                    frame: child.into(),
                }
            });
        };
        if stored.parent == parent {
            return Err(InsertError::ParentUnchanged {
                frame: child.into(),
                parent: parent.into(),
            });
        }
        if parent == child {
            return Err(InsertError::SelfEdge {
                frame: child.into(),
            });
        }
        if self.is_at_or_above(child, parent) {
            return Err(InsertError::Cycle {
                child: child.into(),
                parent: parent.into(),
            });
        }
        if matches!(stored.held, Held::Fixed(_)) != stamp.is_none() {
            return Err(InsertError::KindMismatch {
                frame: child.into(),
                incoming: stamp_of(stamp),
            });
        }
        self.edges.insert(
            child,
            ModelEdge {
                parent,
                held: seeded(stamp, x),
            },
        );
        Ok(())
    }

    fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.edges.remove(child).is_some()
    }

    fn remove_samples_through(
        &mut self,
        cutoff: i64,
    ) {
        for edge in self.edges.values_mut() {
            if let Held::Samples(samples) = &mut edge.held {
                samples.retain(|&stored, _| stored > cutoff);
            }
        }
    }

    /// Asserts that the registry knows exactly the names the model's edges
    /// name, and that every model edge resolves to what the model holds.
    fn assert_matches(
        &self,
        registry: &Registry,
        pool: &[&'static str],
        step: u32,
    ) {
        let known = self.known();
        for &name in pool {
            assert_eq!(
                is_known(registry, name),
                known.contains(name),
                "step {step}: {name}"
            );
        }
        for (&child, edge) in &self.edges {
            let parent = edge.parent;
            let latest = registry.latest_common_time(parent, child);
            match &edge.held {
                Held::Fixed(x) => {
                    assert_eq!(latest, Ok(Stamp::Static), "step {step}: {child}");
                    assert_eq!(
                        position(registry, parent, child, 0),
                        Ok(along_x(*x)),
                        "step {step}: {child}"
                    );
                }
                Held::Samples(samples) => {
                    let Some(&newest) = samples.keys().next_back() else {
                        let no_data = LookupError::NoData {
                            frame: child.into(),
                        };
                        assert_eq!(latest, Err(no_data.clone()), "step {step}: {child}");
                        assert_eq!(
                            position(registry, parent, child, 0),
                            Err(no_data),
                            "step {step}: {child}"
                        );
                        continue;
                    };
                    assert_eq!(latest, Ok(Stamp::At(at(newest))), "step {step}: {child}");
                    for (&nanos, &x) in samples {
                        assert_eq!(
                            position(registry, parent, child, nanos),
                            Ok(along_x(x)),
                            "step {step}: {child} at {nanos}"
                        );
                    }
                    // Nothing is stored outside what the model holds.
                    let earliest = samples.keys().next().copied().unwrap_or(newest);
                    for outside in [earliest - 1, newest + 1] {
                        assert_eq!(
                            position(registry, parent, child, outside),
                            Err(LookupError::OutOfRange {
                                frame: child.into(),
                                requested: at(outside),
                                earliest: at(earliest),
                                latest: at(newest),
                            }),
                            "step {step}: {child} at {outside}"
                        );
                    }
                }
            }
        }
    }
}

fn seeded(
    stamp: Option<i64>,
    x: f64,
) -> Held {
    match stamp {
        None => Held::Fixed(x),
        Some(nanos) => Held::Samples(BTreeMap::from([(nanos, x)])),
    }
}

/// How often each kind of operation was accepted, so that a run that
/// rejected everything cannot pass.
#[derive(Default)]
struct Tally {
    inserts: u32,
    rejected_inserts: u32,
    batches: u32,
    rejected_batches: u32,
    reparents: u32,
    rejected_reparents: u32,
    removals: u32,
    cleanups: u32,
}

/// Applies `steps` random operations to a registry and to the model, and
/// compares the two after every one.
fn run_against_model(
    max_age: Option<Duration>,
    seed: u64,
    steps: u32,
) -> Tally {
    const POOL: [&str; 6] = ["a", "b", "c", "d", "e", "f"];
    let mut rng = Lcg(seed);
    let mut registry = match max_age {
        Some(max_age) => Registry::with_max_age(max_age),
        None => Registry::unbounded(),
    };
    let mut model = Model {
        max_age: max_age.map(|max_age| i64::try_from(max_age.as_nanos()).unwrap()),
        edges: BTreeMap::new(),
    };
    let mut tally = Tally::default();

    for step in 0..steps {
        let offer = |rng: &mut Lcg| Offer {
            parent: rng.pick(&POOL),
            child: rng.pick(&POOL),
            stamp: (rng.below(4) != 0).then(|| i64::try_from(rng.below(9)).unwrap() * SEC),
            x: f64::from(step) + 0.25,
        };
        match rng.below(20) {
            0..=8 => {
                let offer = offer(&mut rng);
                let expected = model.insert(offer);
                assert_eq!(registry.insert(&offer.transform()), expected, "step {step}");
                match expected {
                    Ok(()) => tally.inserts += 1,
                    Err(_) => tally.rejected_inserts += 1,
                }
            }
            9..=11 => {
                let offers: Vec<Offer> = (0..=rng.below(4)).map(|_| offer(&mut rng)).collect();
                let transforms: Vec<Transform> =
                    offers.iter().map(|offer| offer.transform()).collect();
                let expected = model.insert_all(&offers);
                assert_eq!(registry.insert_all(&transforms), expected, "step {step}");
                match expected {
                    Ok(()) => tally.batches += 1,
                    Err(_) => tally.rejected_batches += 1,
                }
            }
            12..=15 => {
                let seed = offer(&mut rng);
                let expected = model.reparent(seed);
                assert_eq!(
                    registry.reparent(&seed.transform()),
                    expected,
                    "step {step}"
                );
                match expected {
                    Ok(()) => tally.reparents += 1,
                    Err(_) => tally.rejected_reparents += 1,
                }
            }
            16..=18 => {
                let name = rng.pick(&POOL);
                let expected = model.remove_frame(name);
                assert_eq!(registry.remove_frame(name), expected, "step {step}");
                tally.removals += u32::from(expected);
            }
            _ => {
                let cutoff = i64::try_from(rng.below(9)).unwrap() * SEC;
                model.remove_samples_through(cutoff);
                registry.remove_samples_through(at(cutoff));
                tally.cleanups += 1;
            }
        }
        model.assert_matches(&registry, &POOL, step);
    }
    tally
}

fn assert_every_operation_was_exercised(tally: &Tally) {
    for (count, what) in [
        (tally.inserts, "accepted inserts"),
        (tally.rejected_inserts, "rejected inserts"),
        (tally.batches, "accepted batches"),
        (tally.rejected_batches, "rejected batches"),
        (tally.reparents, "accepted reparents"),
        (tally.rejected_reparents, "rejected reparents"),
        (tally.removals, "removed frames"),
        (tally.cleanups, "cleanups"),
    ] {
        assert!(count >= 20, "only {count} {what}");
    }
}

#[test]
fn random_operations_keep_an_unbounded_registry_equal_to_the_model() {
    let tally = run_against_model(None, 0x5EED_0001, 4_000);
    assert_every_operation_was_exercised(&tally);
}

#[test]
fn random_operations_keep_a_registry_with_max_age_equal_to_the_model() {
    let tally = run_against_model(Some(Duration::from_secs(3)), 0x5EED_0002, 4_000);
    assert_every_operation_was_exercised(&tally);
}

#[test]
fn the_model_run_reaches_expiry_under_max_age() {
    // The windowed run is only worth its name if samples are rejected as
    // expired in it: replay a shorter one and look for the error.
    let mut registry = Registry::with_max_age(Duration::from_secs(3));
    let mut rng = Lcg(0x5EED_0003);
    let mut expired = 0_u32;
    for step in 0..2_000_u32 {
        let nanos = i64::try_from(rng.below(9)).unwrap() * SEC;
        let child = rng.pick(&["a", "b", "c"]);
        let result = registry.insert(&sample("root", child, nanos, f64::from(step)));
        if let Err(error) = result {
            let frame: String = child.into();
            assert!(
                matches!(
                    &error,
                    InsertError::Expired { frame: named, stamp, newest }
                        if *named == frame
                            && *stamp == at(nanos)
                            && newest.as_nanos() - nanos > 3 * SEC
                ),
                "step {step}: {error:?}"
            );
            expired += 1;
        }
    }
    assert!(expired > 100, "only {expired} expired inserts");
}
