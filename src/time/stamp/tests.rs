//! Unit tests.

use super::Stamp;
use crate::Timestamp;
use alloc::format;
use std::collections::HashMap;

#[test]
fn a_stamp_is_matched_to_reach_its_instant() {
    let nanos = |stamp| match stamp {
        Stamp::At(t) => Some(t.as_nanos()),
        Stamp::Static => None,
    };

    assert_eq!(nanos(Stamp::At(Timestamp::from_nanos(5))), Some(5));
    assert_eq!(nanos(Stamp::At(Timestamp::from_nanos(-5))), Some(-5));
    assert_eq!(nanos(Stamp::Static), None);
}

#[test]
fn equality_separates_static_from_every_instant() {
    assert_eq!(Stamp::Static, Stamp::Static);
    assert_eq!(
        Stamp::At(Timestamp::from_nanos(7)),
        Stamp::At(Timestamp::from_nanos(7))
    );
    assert_ne!(
        Stamp::At(Timestamp::from_nanos(7)),
        Stamp::At(Timestamp::from_nanos(8))
    );

    // No instant is reserved for staticness: zero and both ends of the
    // range are ordinary dynamic stamps.
    for nanos in [0, i64::MIN, i64::MAX] {
        assert_ne!(Stamp::At(Timestamp::from_nanos(nanos)), Stamp::Static);
        assert!(matches!(
            Stamp::At(Timestamp::from_nanos(nanos)),
            Stamp::At(t) if t.as_nanos() == nanos
        ));
    }
}

#[test]
fn a_stamp_is_copied_not_moved() {
    let stamp = Stamp::At(Timestamp::from_nanos(3));
    let copy = stamp;

    assert_eq!(stamp, copy);
}

#[test]
fn a_stamp_is_usable_as_a_hash_map_key() {
    let mut counts: HashMap<Stamp, u32> = HashMap::new();
    for stamp in [
        Stamp::Static,
        Stamp::At(Timestamp::from_nanos(0)),
        Stamp::At(Timestamp::from_nanos(1)),
        Stamp::Static,
        Stamp::At(Timestamp::from_nanos(1)),
        Stamp::At(Timestamp::from_nanos(1)),
    ] {
        *counts.entry(stamp).or_insert(0) += 1;
    }

    assert_eq!(counts.len(), 3);
    assert_eq!(counts.get(&Stamp::Static), Some(&2));
    assert_eq!(counts.get(&Stamp::At(Timestamp::from_nanos(0))), Some(&1));
    assert_eq!(counts.get(&Stamp::At(Timestamp::from_nanos(1))), Some(&3));
    assert_eq!(counts.get(&Stamp::At(Timestamp::from_nanos(2))), None);
}

#[test]
fn debug_names_the_variant() {
    assert_eq!(format!("{:?}", Stamp::Static), "Static");
    assert_eq!(
        format!("{:?}", Stamp::At(Timestamp::from_nanos(5))),
        "At(Timestamp(5))"
    );
}
