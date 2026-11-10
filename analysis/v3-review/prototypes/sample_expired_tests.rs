//! NEW (prototype, draft item A10): an insert the retention window would
//! discard is `RegistryError::SampleExpired`, for `add_transform` and
//! `add_transforms` alike, and a batch equals the insert loop.

use core::time::Duration;
use proptest::prelude::*;
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

const SECOND: u64 = 1_000_000_000;

fn secs(s: u64) -> Timestamp {
    Timestamp::from_nanos(s * SECOND)
}

fn edge(
    parent: &str,
    child: &str,
    at: Timestamp,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(at),
    )
    .unwrap()
}

fn at(s: u64) -> Transform {
    edge("map", "base", secs(s))
}

fn window_10s() -> Registry {
    Registry::with_max_age(Duration::from_secs(10))
}

/// Everything a caller can see of the `map -> base` edge at whole seconds.
fn seen(registry: &Registry) -> Vec<String> {
    let mut seen = vec![format!("{:?}", registry.latest_common_time("map", "base"))];
    for s in 0..=120 {
        seen.push(format!(
            "{:?}",
            registry.get_transform("map", "base", secs(s))
        ));
    }
    seen
}

fn assert_expired(
    result: Result<(), RegistryError>,
    frame: &str,
    stamp: Timestamp,
    newest: Timestamp,
) {
    match result {
        Err(RegistryError::SampleExpired {
            frame: f,
            stamp: s,
            newest: n,
        }) => {
            assert_eq!((f.as_str(), s, n), (frame, stamp, newest));
        }
        other => panic!("expected SampleExpired, got {other:?}"),
    }
}

#[test]
fn add_transform_refuses_a_sample_older_than_the_window_and_stores_nothing() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();
    let before = seen(&registry);

    assert_expired(registry.add_transform(at(50)), "base", secs(50), secs(100));
    assert_eq!(seen(&registry), before);
}

#[test]
fn the_expiry_boundary_is_the_eviction_boundary() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();

    // Exactly max_age old: eviction keeps it, so the insert is accepted.
    registry.add_transform(at(90)).unwrap();
    assert!(registry.get_transform("map", "base", secs(90)).is_ok());

    // One nanosecond older: eviction would drop it.
    let stale = Timestamp::from_nanos(90 * SECOND - 1);
    assert_expired(
        registry.add_transform(edge("map", "base", stale)),
        "base",
        stale,
        secs(100),
    );
}

#[test]
fn a_registry_without_max_age_never_reports_expiry() {
    let mut registry = Registry::new();
    registry.add_transform(at(100)).unwrap();
    registry.add_transform(at(0)).unwrap();
    registry.add_transforms([at(90), at(1)]).unwrap();
    assert!(registry.get_transform("map", "base", secs(0)).is_ok());
}

#[test]
fn a_threshold_that_underflows_the_time_type_refuses_nothing() {
    // newest < max_age: `checked_sub` fails, nothing can be older.
    let mut registry = window_10s();
    registry.add_transform(at(5)).unwrap();
    registry.add_transform(at(0)).unwrap();
    registry.add_transforms([at(9), at(1)]).unwrap();
    assert!(registry.get_transform("map", "base", secs(0)).is_ok());
}

#[test]
fn static_transforms_are_never_expired() {
    let mut registry = window_10s();
    let mount =
        || Transform::static_between("base", "lidar", Vector3::zero(), Quaternion::identity());
    registry.add_transform(at(100)).unwrap();
    registry.add_transform(mount().unwrap()).unwrap();
    registry.add_transform(mount().unwrap()).unwrap();
    registry.add_transforms([mount().unwrap()]).unwrap();
}

// --- A10.3, the four deterministic regression tests ---------------------

#[test]
fn a_batch_on_a_held_frame_fails_whole_when_an_earlier_member_expires_a_later_one() {
    let mut registry = window_10s();
    registry.add_transform(at(45)).unwrap();
    let before = seen(&registry);

    // Against the stored newest (45) both members pass; the loop would
    // store 100 and then refuse 50.
    assert_expired(
        registry.add_transforms([at(100), at(50)]),
        "base",
        secs(50),
        secs(100),
    );
    assert_eq!(
        seen(&registry),
        before,
        "a failed batch must change nothing"
    );

    // The same members in the other order are what the loop accepts.
    registry.add_transforms([at(50), at(100)]).unwrap();
    assert!(registry.get_transform("map", "base", secs(100)).is_ok());
}

#[test]
fn a_poisoned_batch_on_a_fresh_registry_names_the_legit_member_and_stores_nothing() {
    let mut registry = window_10s();
    let (poison, legit) = (secs(1_000_000), secs(50));

    assert_expired(
        registry.add_transforms([edge("map", "base", poison), edge("map", "base", legit)]),
        "base",
        legit,
        poison,
    );
    assert!(matches!(
        registry.get_transform("map", "base", poison),
        Err(RegistryError::UnknownFrame(_))
    ));
}

#[test]
fn an_expired_sample_with_a_second_parent_reports_the_parent_pin() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();

    let stale_elsewhere = || edge("odom", "base", secs(50));
    assert!(matches!(
        registry.add_transform(stale_elsewhere()),
        Err(RegistryError::ReparentingNotSupported { .. })
    ));
    assert!(matches!(
        registry.add_transforms([stale_elsewhere()]),
        Err(RegistryError::ReparentingNotSupported { .. })
    ));
    // Expired only by the batch's own earlier member, on a held frame.
    assert!(matches!(
        registry.add_transforms([at(200), edge("odom", "base", secs(150))]),
        Err(RegistryError::ReparentingNotSupported { .. })
    ));
    // ... and on a frame the batch creates.
    assert!(matches!(
        registry.add_transforms([
            edge("map", "arm", secs(200)),
            edge("odom", "arm", secs(150))
        ]),
        Err(RegistryError::ReparentingNotSupported { .. })
    ));
}

#[test]
fn a_reparent_seed_stamped_ahead_expires_the_live_stream() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();

    // The seed is exempt: its stamp is not measured against the history
    // the move drops, in either direction.
    registry
        .reparent_frame(edge("odom", "base", secs(500)))
        .unwrap();
    assert_expired(
        registry.add_transform(edge("odom", "base", secs(101))),
        "base",
        secs(101),
        secs(500),
    );
    registry
        .reparent_frame(edge("map", "base", secs(3)))
        .unwrap();
    registry.add_transform(at(101)).unwrap();
}

// --- further precedence and atomicity cases -----------------------------

#[test]
fn an_expired_sample_that_is_also_non_finite_reports_the_numbers() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();
    let overflowed = || {
        (edge("map", "via", secs(50)).with_x(f64::MAX)
            * edge("via", "base", secs(50)).with_x(f64::MAX))
        .unwrap()
    };
    assert!(matches!(
        registry.add_transform(overflowed()),
        Err(RegistryError::NonFiniteValues)
    ));
    assert!(matches!(
        registry.add_transforms([at(200), overflowed()]),
        Err(RegistryError::NonFiniteValues)
    ));
}

trait WithX {
    fn with_x(
        self,
        x: f64,
    ) -> Self;
}

impl WithX for Transform {
    fn with_x(
        self,
        x: f64,
    ) -> Self {
        Transform::new(
            self.parent(),
            self.child(),
            Vector3::new(x, 0.0, 0.0),
            self.rotation(),
            self.timestamp(),
        )
        .unwrap()
    }
}

#[test]
fn a_rejected_member_does_not_advance_the_running_newest() {
    // If the refused member's stamp advanced the running newest, the
    // error payload of a later failure would differ from the loop's. A
    // batch stops at its first error, so this is observable only through
    // the first error — pinned here against the loop.
    let mut looped = window_10s();
    let mut batched = window_10s();
    for registry in [&mut looped, &mut batched] {
        registry.add_transform(at(45)).unwrap();
    }
    let batch = || [at(40), at(60), at(49), at(100), at(89)];
    let expected = batch()
        .into_iter()
        .try_for_each(|t| looped.add_transform(t));
    let result = batched.add_transforms(batch());
    assert_eq!(format!("{result:?}"), format!("{expected:?}"));
    assert_expired(result, "base", secs(49), secs(60));
}

#[test]
fn a_zero_window_refuses_every_out_of_order_sample_and_accepts_a_repeat() {
    let mut registry = Registry::with_max_age(Duration::ZERO);
    registry.add_transform(at(2)).unwrap();
    assert_expired(registry.add_transform(at(1)), "base", secs(1), secs(2));
    // The newest instant itself is an upsert, not an expiry.
    registry.add_transform(at(2)).unwrap();
    registry.add_transforms([at(2), at(3), at(3)]).unwrap();
    assert_expired(
        registry.add_transforms([at(4), at(3)]),
        "base",
        secs(3),
        secs(4),
    );
    assert!(registry.get_transform("map", "base", secs(3)).is_ok());
    assert!(registry.get_transform("map", "base", secs(4)).is_err());
}

#[test]
fn a_drained_frame_restarts_at_earlier_stamps_also_inside_a_batch() {
    let mut registry = window_10s();
    registry.add_transform(at(100)).unwrap();
    registry.remove_transforms_before(secs(101));
    // Empty buffer: the running newest starts at nothing.
    assert_expired(
        registry.add_transforms([at(30), at(5)]),
        "base",
        secs(5),
        secs(30),
    );
    registry.add_transforms([at(5), at(10)]).unwrap();
}

// --- batch == loop, biased toward held edges and small windows ----------

/// Three edges the registry already holds, plus a frame the batch creates.
const EDGES: [(&str, &str); 4] = [("r", "a"), ("a", "b"), ("r", "c"), ("c", "d")];
const FRAMES: [&str; 5] = ["r", "a", "b", "c", "d"];
const HORIZON: u64 = 24;

/// (edge, instant, flaw): flaw 0 = wrong parent, 1 = non-finite, 2 =
/// static, anything else = an ordinary dynamic sample.
type Member = (usize, u64, u8, i32);

fn members(len: core::ops::Range<usize>) -> impl Strategy<Value = Vec<Member>> {
    prop::collection::vec(
        (
            // Biased toward the first two (held) edges: repeated members.
            prop_oneof![4 => 0..2_usize, 1 => 2..4_usize],
            0..HORIZON,
            0_u8..40,
            -4_i32..4,
        ),
        len,
    )
}

fn member((which, nanos, flaw, x): Member) -> Transform {
    let (parent, child) = EDGES[which];
    let stamp = Stamp::At(Timestamp::from_nanos(nanos));
    let make = |parent: &str, child: &str, x: f64, stamp| {
        Transform::new(
            parent,
            child,
            Vector3::new(x, 0.0, 0.0),
            Quaternion::identity(),
            stamp,
        )
        .unwrap()
    };
    match flaw {
        0 => make("elsewhere", child, f64::from(x), stamp),
        1 => (make(parent, "via", f64::MAX, stamp) * make("via", child, f64::MAX, stamp)).unwrap(),
        2 => make(parent, child, f64::from(x), Stamp::Static),
        _ => make(parent, child, f64::from(x), stamp),
    }
}

fn observable(registry: &Registry) -> Vec<String> {
    let mut seen = Vec::new();
    for target in FRAMES {
        for source in FRAMES {
            seen.push(format!("{:?}", registry.latest_common_time(target, source)));
            for nanos in 0..=HORIZON {
                seen.push(format!(
                    "{:?}",
                    registry.get_transform(target, source, Timestamp::from_nanos(nanos))
                ));
            }
        }
    }
    seen
}

proptest! {
    #[test]
    fn a_batch_of_repeated_members_under_a_small_window_matches_the_loop_or_changes_nothing(
        max_age in prop_oneof![
            3 => Just(Some(0_u64)),
            6 => (1_u64..6).prop_map(Some),
            2 => (6_u64..30).prop_map(Some),
            1 => Just(None),
        ],
        seed in members(0..6),
        batch in members(1..12),
    ) {
        let fresh = || {
            max_age.map_or_else(Registry::new, |age| {
                Registry::with_max_age(Duration::from_nanos(age))
            })
        };
        let (mut looped, mut batched) = (fresh(), fresh());
        for &m in &seed {
            let _ = looped.add_transform(member(m));
            let _ = batched.add_transform(member(m));
        }
        let before = observable(&batched);

        let expected = batch.iter().try_for_each(|&m| looped.add_transform(member(m)));
        let result = batched.add_transforms(batch.iter().map(|&m| member(m)));

        prop_assert_eq!(format!("{result:?}"), format!("{expected:?}"));
        if result.is_ok() {
            prop_assert_eq!(observable(&batched), observable(&looped));
        } else {
            prop_assert_eq!(observable(&batched), before);
        }
    }

    /// Under SampleExpired an accepted insert is a stored insert: the
    /// sample is served at its own instant right after the call.
    #[test]
    fn an_accepted_dynamic_insert_is_served_at_its_own_instant(
        max_age in prop::option::of(0_u64..8),
        stamps in prop::collection::vec(0_u64..32, 1..40),
    ) {
        let mut registry = max_age.map_or_else(Registry::new, |age| {
            Registry::with_max_age(Duration::from_nanos(age))
        });
        let mut newest: Option<u64> = None;
        for nanos in stamps {
            let t = Timestamp::from_nanos(nanos);
            let result = registry.add_transform(edge("map", "base", t));
            let expired = match (max_age, newest) {
                (Some(age), Some(newest)) => newest.checked_sub(age).is_some_and(|th| nanos < th),
                _ => false,
            };
            prop_assert_eq!(result.is_err(), expired, "{:?}", result);
            if result.is_ok() {
                prop_assert!(registry.get_transform("map", "base", t).is_ok());
                newest = Some(newest.map_or(nanos, |n| n.max(nanos)));
            }
        }
    }
}
