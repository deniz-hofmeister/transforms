//! `Debug` output of a `Registry`: a summary, never a dump of samples.

use super::lookup::{mount, posed, sample, yaw};
use crate::{Registry, Stamp, Timestamp, Vector3};
use alloc::format;
use core::time::Duration;

/// `map <- base` with one sample at each of `stamps`, and the static
/// `base <- lidar`, in a registry with a ten second window.
fn robot(stamps: impl IntoIterator<Item = i64>) -> Registry {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    for nanos in stamps {
        registry.insert(&sample("map", "base", 1.0, nanos)).unwrap();
    }
    registry.insert(&mount("base", "lidar", 0.5)).unwrap();
    registry
}

#[test]
fn empty_registry_prints_no_frames() {
    assert_eq!(
        format!("{:?}", Registry::unbounded()),
        "Registry { frames: {}, max_age: None }"
    );
}

#[test]
fn output_shows_the_retention_window() {
    assert!(format!("{:?}", Registry::unbounded()).contains("max_age: None"));
    assert!(
        format!("{:?}", Registry::with_max_age(Duration::from_secs(10)))
            .contains("max_age: Some(10s)")
    );
    assert!(format!("{:?}", Registry::with_max_age(Duration::ZERO)).contains("max_age: Some(0ns)"));
}

#[test]
fn output_shows_each_known_frame_with_its_parent() {
    let output = format!("{:?}", robot([100, 200]));

    assert!(output.contains(r#""map": "root""#), "{output}");
    assert!(output.contains(r#""base": ("map", "#), "{output}");
    assert!(output.contains(r#""lidar": ("base", "#), "{output}");
}

#[test]
fn output_summarizes_a_dynamic_history_as_its_count_and_covered_range() {
    let output = format!("{:?}", robot([100, 150, 200]));

    assert!(
        output.contains(
            r#""base": ("map", Dynamic { samples: 3, covered: Some((Timestamp(100), Timestamp(200))) })"#
        ),
        "{output}"
    );
    // The sample between the ends of the range is not printed.
    assert!(!output.contains("150"), "{output}");
}

#[test]
fn output_shows_a_drained_history_as_empty() {
    let mut registry = robot([100, 200]);
    registry.remove_samples_through(Timestamp::MAX);

    let output = format!("{registry:?}");

    assert!(
        output.contains(r#""base": ("map", Dynamic { samples: 0, covered: None })"#),
        "{output}"
    );
    assert!(output.contains(r#""lidar": ("base", Static("#), "{output}");
}

#[test]
fn output_shows_a_static_pose_in_full() {
    let mut registry = Registry::unbounded();
    let rotation = yaw(1.0);
    registry
        .insert(&posed(
            "base",
            "camera",
            Vector3::new(0.25, -1.5, 3.0),
            rotation,
            Stamp::Static,
        ))
        .unwrap();

    let output = format!("{registry:?}");

    assert!(output.contains(r#""camera": ("base", Static("#), "{output}");
    assert!(
        output.contains("translation: Vector3 { x: 0.25, y: -1.5, z: 3.0 }"),
        "{output}"
    );
    assert!(output.contains(&format!("{rotation:?}")), "{output}");
}

#[test]
fn output_length_does_not_grow_with_the_number_of_samples() {
    let few = format!("{:?}", robot([0, 1_999]));
    let many = format!("{:?}", robot(0..2_000));

    // Only the count differs: no sample is printed.
    assert_eq!(many, few.replace("samples: 2,", "samples: 2000,"));
    assert_eq!(many.len(), few.len() + 3);
}

#[test]
fn pretty_output_length_does_not_grow_with_the_number_of_samples() {
    let few = format!("{:#?}", robot([0, 1_999]));
    let many = format!("{:#?}", robot(0..2_000));

    assert_eq!(many, few.replace("samples: 2,", "samples: 2000,"));
}

#[test]
fn output_drops_a_frame_that_is_no_longer_known() {
    let mut registry = robot([100, 200]);
    assert!(registry.remove_frame("lidar"));
    assert!(registry.remove_frame("base"));

    assert_eq!(
        format!("{registry:?}"),
        "Registry { frames: {}, max_age: Some(10s) }"
    );
}
