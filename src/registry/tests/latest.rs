//! `Registry::latest_common_time`: the newest instant a chain serves.

use super::lookup::{at, disconnected, mount, no_data, registry_of, sample, unknown};
use crate::{LookupError, Registry, Stamp, Timestamp};

fn no_common_time(
    frame: &str,
    earliest: i64,
    latest: i64,
    latest_possible: i64,
) -> LookupError {
    LookupError::NoCommonTime {
        frame: frame.into(),
        earliest: at(earliest),
        latest: at(latest),
        latest_possible: at(latest_possible),
    }
}

/// Dynamic edges, each `(parent, child, [stamps])`, in an unbounded
/// registry.
fn ranges(edges: &[(&str, &str, &[i64])]) -> Registry {
    let mut registry = Registry::unbounded();
    for (parent, child, stamps) in edges {
        for nanos in *stamps {
            registry
                .insert(&sample(parent, child, 1.0, *nanos))
                .unwrap();
        }
    }
    registry
}

/// Asserts that the pair has a newest common instant, the same in both
/// argument orders, that a lookup there succeeds in both directions, and
/// that a lookup one nanosecond later does not. Returns the instant.
fn served_instant(
    registry: &Registry,
    target: &str,
    source: &str,
) -> Timestamp {
    let answer = registry.latest_common_time(target, source);
    let Ok(Stamp::At(newest)) = answer else {
        panic!("expected an instant for {target} <- {source}, found {answer:?}");
    };
    assert_eq!(
        registry.latest_common_time(source, target),
        Ok(Stamp::At(newest))
    );
    assert!(registry.lookup(target, source, newest).is_ok());
    assert!(registry.lookup(source, target, newest).is_ok());
    let later = at(newest.as_nanos() + 1);
    assert!(matches!(
        registry.lookup(target, source, later),
        Err(LookupError::OutOfRange { .. })
    ));
    newest
}

#[test]
fn all_static_chain_is_static() {
    let registry = registry_of(&[
        mount("a", "b", 1.0),
        mount("b", "c", 1.0),
        mount("a", "d", 1.0),
    ]);

    for (target, source) in [("a", "c"), ("c", "a"), ("a", "b"), ("c", "d"), ("d", "c")] {
        assert_eq!(
            registry.latest_common_time(target, source),
            Ok(Stamp::Static)
        );
        // Any instant is served then.
        assert!(registry.lookup(target, source, at(0)).is_ok());
        assert!(registry.lookup(target, source, Timestamp::MAX).is_ok());
    }
}

#[test]
fn known_frame_relative_to_itself_is_static() {
    let mut registry = registry_of(&[sample("a", "b", 1.0, 100), mount("b", "c", 1.0)]);

    for frame in ["a", "b", "c"] {
        assert_eq!(registry.latest_common_time(frame, frame), Ok(Stamp::Static));
    }

    // A drained frame's own edge is not on the chain to itself.
    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(registry.latest_common_time("b", "b"), Ok(Stamp::Static));
}

#[test]
fn static_siblings_under_a_dynamic_edge_are_static() {
    let registry = registry_of(&[
        sample("map", "base", 1.0, 100),
        mount("base", "left", 1.0),
        mount("base", "right", 1.0),
    ]);

    assert_eq!(
        registry.latest_common_time("left", "right"),
        Ok(Stamp::Static)
    );
    assert_eq!(
        registry.latest_common_time("base", "left"),
        Ok(Stamp::Static)
    );
}

#[test]
fn unknown_frame_names_the_target_before_the_source() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100)]);

    assert_eq!(
        registry.latest_common_time("a", "does_not_exist"),
        Err(unknown("does_not_exist"))
    );
    assert_eq!(
        registry.latest_common_time("does_not_exist", "a"),
        Err(unknown("does_not_exist"))
    );
    assert_eq!(
        registry.latest_common_time("first", "second"),
        Err(unknown("first"))
    );
    assert_eq!(registry.latest_common_time("", "a"), Err(unknown("")));
    assert_eq!(registry.latest_common_time("a", ""), Err(unknown("")));
}

#[test]
fn equal_unknown_names_are_an_unknown_frame_and_not_static() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100)]);

    assert_eq!(
        registry.latest_common_time("nowhere", "nowhere"),
        Err(unknown("nowhere"))
    );
    assert_eq!(
        Registry::unbounded().latest_common_time("a", "a"),
        Err(unknown("a"))
    );
}

#[test]
fn frames_in_separate_trees_are_disconnected() {
    let registry = registry_of(&[sample("a", "b", 1.0, 100), sample("x", "y", 1.0, 100)]);

    assert_eq!(
        registry.latest_common_time("b", "y"),
        Err(disconnected("b", "y"))
    );
    assert_eq!(
        registry.latest_common_time("y", "b"),
        Err(disconnected("y", "b"))
    );
    assert_eq!(
        registry.latest_common_time("a", "x"),
        Err(disconnected("a", "x"))
    );
}

#[test]
fn disconnected_is_reported_before_an_empty_edge() {
    let mut registry = registry_of(&[sample("a", "b", 1.0, 100), sample("x", "y", 1.0, 100)]);
    registry.remove_samples_through(Timestamp::MAX);

    assert_eq!(
        registry.latest_common_time("b", "y"),
        Err(disconnected("b", "y"))
    );
    assert_eq!(
        registry.latest_common_time("b", "nowhere"),
        Err(unknown("nowhere"))
    );
}

#[test]
fn single_dynamic_edge_answers_with_its_newest_sample() {
    let registry = ranges(&[("a", "b", &[-50, 10, 40])]);

    assert_eq!(served_instant(&registry, "a", "b"), at(40));
}

#[test]
fn answer_is_bounded_by_the_edge_that_lags() {
    // map <- odom covers [4us, 9us], odom <- base covers [3us, 7us].
    let registry = ranges(&[
        ("map", "odom", &[4_000, 9_000]),
        ("odom", "base", &[3_000, 7_000]),
    ]);

    assert_eq!(served_instant(&registry, "map", "base"), at(7_000));
    assert_eq!(served_instant(&registry, "map", "odom"), at(9_000));
    assert_eq!(served_instant(&registry, "odom", "base"), at(7_000));
}

#[test]
fn answer_is_the_minimum_of_the_newest_stamps_guarded_by_the_maximum_of_the_starts() {
    // Ranges [1, 10], [5, 8] and [7, 20]: the oldest newest stamp is 8,
    // and the latest start, 7, still reaches back to it.
    let registry = ranges(&[
        ("a", "b", &[1, 10]),
        ("b", "c", &[5, 8]),
        ("c", "d", &[7, 20]),
    ]);
    assert_eq!(served_instant(&registry, "a", "d"), at(8));

    // With the last range starting at 9, nothing reaches back to 8.
    let registry = ranges(&[
        ("a", "b", &[1, 10]),
        ("b", "c", &[5, 8]),
        ("c", "d", &[9, 20]),
    ]);
    assert_eq!(
        registry.latest_common_time("a", "d"),
        Err(no_common_time("d", 9, 20, 8))
    );
    assert_eq!(
        registry.latest_common_time("d", "a"),
        Err(no_common_time("d", 9, 20, 8))
    );
}

#[test]
fn siblings_are_bounded_by_both_sides_of_the_chain() {
    let registry = ranges(&[
        ("top", "p", &[10, 60]),
        ("p", "q", &[20, 50]),
        ("top", "u", &[30, 40]),
        ("u", "v", &[5, 70]),
    ]);

    assert_eq!(served_instant(&registry, "q", "v"), at(40));
    assert_eq!(served_instant(&registry, "q", "top"), at(50));
    assert_eq!(served_instant(&registry, "v", "top"), at(40));
}

#[test]
fn static_edges_do_not_bound_a_mixed_chain() {
    let registry = registry_of(&[
        mount("a", "b", 1.0),
        sample("b", "c", 1.0, 3_000),
        sample("b", "c", 1.0, 8_000),
        mount("c", "d", 1.0),
    ]);

    assert_eq!(served_instant(&registry, "a", "d"), at(8_000));
    assert_eq!(served_instant(&registry, "a", "c"), at(8_000));
    assert_eq!(registry.latest_common_time("c", "d"), Ok(Stamp::Static));
}

#[test]
fn ranges_that_touch_in_one_instant_answer_with_that_instant() {
    let registry = ranges(&[("a", "b", &[0, 5_000]), ("b", "c", &[5_000, 20_000])]);

    assert_eq!(served_instant(&registry, "a", "c"), at(5_000));
}

#[test]
fn edges_above_the_meeting_point_do_not_constrain_the_answer() {
    // world <- map went stale long ago; map <- a and map <- b are fresh.
    let registry = ranges(&[
        ("world", "map", &[1_000, 2_000]),
        ("map", "a", &[10_000, 20_000]),
        ("map", "b", &[15_000, 25_000]),
    ]);

    assert_eq!(served_instant(&registry, "a", "b"), at(20_000));
    assert_eq!(served_instant(&registry, "map", "b"), at(25_000));

    // A chain that does cross the stale edge has no common instant.
    assert_eq!(
        registry.latest_common_time("world", "b"),
        Err(no_common_time("b", 15_000, 25_000, 2_000))
    );
}

#[test]
fn drained_edge_above_the_meeting_point_does_not_constrain_the_answer() {
    let mut registry = ranges(&[
        ("world", "map", &[1_000]),
        ("map", "a", &[10_000, 20_000]),
        ("map", "b", &[15_000, 25_000]),
    ]);
    registry.remove_samples_through(at(5_000));

    assert_eq!(served_instant(&registry, "a", "b"), at(20_000));
    assert_eq!(
        registry.latest_common_time("world", "b"),
        Err(no_data("map"))
    );
}

#[test]
fn disjoint_ranges_have_no_common_time() {
    // a <- b covers [0, 5us], b <- c covers [10us, 20us]. The minimum of
    // the newest stamps alone would be 5us, which `c` cannot serve.
    let registry = ranges(&[("a", "b", &[0, 5_000]), ("b", "c", &[10_000, 20_000])]);
    let expected = no_common_time("c", 10_000, 20_000, 5_000);

    assert_eq!(registry.latest_common_time("a", "c"), Err(expected.clone()));
    assert_eq!(registry.latest_common_time("c", "a"), Err(expected));
    assert!(registry.lookup("a", "c", at(5_000)).is_err());
    assert!(registry.lookup("a", "c", at(10_000)).is_err());
}

#[test]
fn no_common_time_names_the_edge_with_the_latest_start() {
    // The upper edge starts last here, and the lower edge ends first.
    let registry = ranges(&[
        ("a", "b", &[30, 40]),
        ("b", "c", &[0, 10]),
        ("c", "d", &[5, 50]),
    ]);

    assert_eq!(
        registry.latest_common_time("a", "d"),
        Err(no_common_time("b", 30, 40, 10))
    );
    assert_eq!(
        registry.latest_common_time("d", "a"),
        Err(no_common_time("b", 30, 40, 10))
    );
}

#[test]
fn no_common_time_on_equal_starts_names_the_first_edge_in_chain_order() {
    // `q` and `v` both start at 30, after `p` ends at 10. Chain order is
    // the target's side from the bottom, then the source's side.
    let registry = ranges(&[
        ("top", "p", &[0, 10]),
        ("p", "q", &[30, 40]),
        ("top", "v", &[30, 60]),
    ]);

    assert_eq!(
        registry.latest_common_time("q", "v"),
        Err(no_common_time("q", 30, 40, 10))
    );
    assert_eq!(
        registry.latest_common_time("v", "q"),
        Err(no_common_time("v", 30, 60, 10))
    );
}

#[test]
fn no_common_time_on_equal_starts_within_one_side_names_the_lower_edge() {
    let registry = ranges(&[
        ("a", "b", &[0, 10]),
        ("b", "c", &[30, 40]),
        ("c", "d", &[30, 60]),
    ]);

    assert_eq!(
        registry.latest_common_time("a", "d"),
        Err(no_common_time("d", 30, 60, 10))
    );
    assert_eq!(
        registry.latest_common_time("d", "a"),
        Err(no_common_time("d", 30, 60, 10))
    );
}

#[test]
fn drained_edge_reports_no_data() {
    let mut registry = registry_of(&[sample("world", "object", 1.0, 1_000_000_000)]);
    registry.remove_samples_through(at(3_000_000_000));

    assert_eq!(
        registry.latest_common_time("world", "object"),
        Err(no_data("object"))
    );
    assert_eq!(
        registry.latest_common_time("object", "world"),
        Err(no_data("object"))
    );
}

#[test]
fn no_data_names_the_first_empty_edge_in_chain_order() {
    // `p` and `v` hold only old samples, which the cutoff removes.
    let mut registry = ranges(&[
        ("top", "p", &[1]),
        ("p", "q", &[100, 200]),
        ("top", "u", &[100, 200]),
        ("u", "v", &[2]),
    ]);
    registry.remove_samples_through(at(50));

    // Target's side from the bottom (q, p), then the source's (v, u).
    assert_eq!(registry.latest_common_time("q", "v"), Err(no_data("p")));
    assert_eq!(registry.latest_common_time("v", "q"), Err(no_data("v")));
    assert_eq!(registry.latest_common_time("top", "v"), Err(no_data("v")));
    assert_eq!(served_instant(&registry, "top", "u"), at(200));
}

#[test]
fn no_data_names_the_lowest_empty_edge_of_a_side() {
    let mut registry = ranges(&[("a", "b", &[1]), ("b", "c", &[2]), ("c", "d", &[100, 200])]);
    registry.remove_samples_through(at(50));

    assert_eq!(registry.latest_common_time("a", "d"), Err(no_data("c")));
    assert_eq!(registry.latest_common_time("d", "a"), Err(no_data("c")));
    assert_eq!(registry.latest_common_time("a", "b"), Err(no_data("b")));
}

#[test]
fn no_data_is_reported_although_other_edges_have_no_common_time() {
    // The two lower edges are disjoint, and the top edge is drained.
    let mut registry = ranges(&[
        ("a", "b", &[1]),
        ("b", "c", &[100, 200]),
        ("c", "d", &[300, 400]),
    ]);
    registry.remove_samples_through(at(50));

    assert_eq!(registry.latest_common_time("a", "d"), Err(no_data("b")));
    assert_eq!(registry.latest_common_time("d", "a"), Err(no_data("b")));
    assert_eq!(
        registry.latest_common_time("b", "d"),
        Err(no_common_time("d", 300, 400, 200))
    );
}

#[test]
fn answer_does_not_evaluate_geometry() {
    // The chain overflows when composed. Its coverage is still answered,
    // and the lookup at that instant reports the overflow.
    let registry = registry_of(&[
        sample("root", "a", 1.0e308, 100),
        sample("a", "b", 1.0e308, 100),
        sample("a", "b", 1.0e308, 200),
    ]);

    assert_eq!(
        registry.latest_common_time("root", "b"),
        Ok(Stamp::At(at(100)))
    );
    assert_eq!(
        registry.lookup("root", "b", at(100)),
        Err(LookupError::NonFinite)
    );
}

#[test]
fn errors_are_never_those_of_a_lookup_at_an_instant() {
    let mut registry = ranges(&[
        ("root", "a", &[1]),
        ("a", "b", &[100, 200]),
        ("b", "c", &[300, 400]),
        ("a", "d", &[150, 250]),
        ("island", "x", &[100]),
    ]);
    registry.insert(&mount("d", "e", 1.0e308)).unwrap();
    registry.insert(&mount("e", "f", 1.0e308)).unwrap();
    registry.remove_samples_through(at(50));

    let frames = [
        "root", "a", "b", "c", "d", "e", "f", "island", "x", "nowhere", "",
    ];
    for target in frames {
        for source in frames {
            let answer = registry.latest_common_time(target, source);
            assert!(
                matches!(
                    answer,
                    Ok(_)
                        | Err(LookupError::UnknownFrame { .. }
                            | LookupError::Disconnected { .. }
                            | LookupError::NoData { .. }
                            | LookupError::NoCommonTime { .. })
                ),
                "{target} <- {source}: {answer:?}"
            );
            // Covered wherever an instant is returned.
            if let Ok(Stamp::At(newest)) = answer {
                assert!(!matches!(
                    registry.lookup(target, source, newest),
                    Err(LookupError::OutOfRange { .. } | LookupError::NoData { .. })
                ));
            }
        }
    }
}
