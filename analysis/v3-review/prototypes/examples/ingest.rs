//! The ingest side of a bridge: wire messages in, a decision per failure.
//!
//! Plain structs stand in for ROS `tf2_msgs/TFMessage`. The loop converts
//! time with a check, inserts one element at a time, matches every
//! `RegistryError` variant an insert can report, and recovers from a bad
//! newest timestamp by its cause. The read side matches lookup failures the
//! same way, so a caller learns whether to retry, to wait, or to stop.
//!
//! Runs unchanged with and without the crate's `std` feature.

use core::time::Duration;
use transforms::{
    Registry, Transform,
    errors::RegistryError,
    geometry::{Quaternion, Vector3},
    time::{Stamp, Timestamp},
};

/// `builtin_interfaces/Time`: signed seconds, so a stamp can be pre-epoch.
#[derive(Clone, Copy)]
struct WireTime {
    sec: i32,
    nanosec: u32,
}

/// `geometry_msgs/TransformStamped`, plus the topic it arrived on.
struct WireTransform {
    stamp: WireTime,
    parent: &'static str,
    child: &'static str,
    translation: [f64; 3],
    /// ROS order: x, y, z, w.
    rotation: [f64; 4],
    /// Arrived on `/tf_static`.
    is_static: bool,
}

/// One message, with the bridge's clock reading when it arrived.
struct WireMessage {
    clock: WireTime,
    transforms: Vec<WireTransform>,
}

/// A wire stamp as a `Timestamp`, or `None` where the crate's unsigned
/// nanoseconds cannot hold it. Clamping a pre-epoch stamp to zero would
/// alias it onto a real instant.
fn timestamp(time: WireTime) -> Option<Timestamp> {
    let sec = u64::try_from(time.sec).ok()?;
    if time.nanosec >= 1_000_000_000 {
        return None;
    }
    // Cannot overflow: i32::MAX seconds is far below u64::MAX nanoseconds.
    Some(Timestamp::from_nanos(
        sec * 1_000_000_000 + u64::from(time.nanosec),
    ))
}

/// Builds the transform, or `None` for an element that cannot be one.
/// Kept as a function because recovery builds the sample a second time:
/// `add_transform` consumes it and the error does not hand it back.
fn build(wire: &WireTransform) -> Option<Transform> {
    let [x, y, z] = wire.translation;
    let [qx, qy, qz, qw] = wire.rotation;
    let stamp = if wire.is_static {
        Stamp::Static
    } else {
        Stamp::At(timestamp(wire.stamp)?)
    };
    // Rejects non-finite components and a rotation outside the unit-norm
    // tolerance; the publisher is wrong, and nothing here can repair it.
    Transform::new(
        wire.parent,
        wire.child,
        Vector3::new(x, y, z),
        Quaternion::from_wxyz(qw, qx, qy, qz),
        stamp,
    )
    .ok()
}

#[derive(Debug, Default, PartialEq)]
struct Counters {
    stored: u32,
    /// Bad stamp, non-finite value or non-unit rotation.
    malformed: u32,
    /// Self edge, cycle, another parent, or the other kind.
    conflicting: u32,
    /// Arrived after its frame's window had moved past it.
    late: u32,
    /// One frame's newest timestamp was ahead of the clock.
    frames_reset: u32,
    /// The clock stepped back and every dynamic sample was drained.
    clock_resets: u32,
    /// An insert error this bridge has no decision for.
    unclassified: u32,
}

struct Bridge {
    registry: Registry,
    /// The clock reading of the previous message.
    clock: Option<Timestamp>,
    /// The newest dynamic stamp ever stored, over all frames.
    newest_stored: Option<Timestamp>,
    counters: Counters,
}

impl Bridge {
    const WINDOW: Duration = Duration::from_secs(10);
    /// How far ahead of the bridge's clock a stamp may honestly be.
    const SKEW: Duration = Duration::from_millis(500);

    fn new() -> Self {
        Self {
            registry: Registry::with_max_age(Self::WINDOW),
            clock: None,
            newest_stored: None,
            counters: Counters::default(),
        }
    }

    fn ingest(
        &mut self,
        message: &WireMessage,
    ) {
        let Some(now) = timestamp(message.clock) else {
            self.counters.malformed += 1;
            return;
        };

        // A clock that stepped back (a bag looping, a simulator restarting)
        // is detected on the clock, not on `SampleExpired`: a step smaller
        // than the window stores the new run beside the old one without any
        // error, and lookups then interpolate across the two.
        if self.clock.is_some_and(|previous| now < previous) {
            self.drain_dynamic_frames();
        }
        self.clock = Some(now);

        // One element at a time. A batch (`add_transforms`) is all or
        // nothing: one late sample would reject every other member.
        for wire in &message.transforms {
            self.insert(wire, now);
        }
    }

    /// Drops every dynamic sample; statics, parents and kinds stay.
    fn drain_dynamic_frames(&mut self) {
        let Some(newest) = self.newest_stored.take() else {
            return;
        };
        // The cutoff removes the sample stamped exactly at it, so the
        // newest stored stamp drains every dynamic frame.
        self.registry.remove_transforms_before(newest);
        self.counters.clock_resets += 1;
    }

    fn insert(
        &mut self,
        wire: &WireTransform,
        now: Timestamp,
    ) {
        let Some(transform) = build(wire) else {
            self.counters.malformed += 1;
            return;
        };
        let stamp = transform.timestamp().at();

        match self.registry.add_transform(transform) {
            Ok(()) => self.stored(stamp),

            // The frame's window discards the sample. The error cannot say
            // which of the two stamps is wrong, so the bridge's clock does.
            Err(RegistryError::SampleExpired { frame, newest, .. }) => {
                let newest_is_plausible = (now + Self::SKEW).is_ok_and(|limit| newest <= limit);
                if newest_is_plausible {
                    // The sample is late. Dropping it is the whole answer;
                    // resetting the frame here would let one straggler wipe
                    // a healthy history.
                    self.counters.late += 1;
                    return;
                }
                // The frame's newest timestamp is ahead of the clock, so it
                // is the bad one, and waiting it out would take until the
                // stream catches up. Discard it with the frame and insert
                // the sample again: expiry is the last insert check, so the
                // sample is known to have the frame's parent and kind, and
                // a first insert cannot expire.
                self.registry.remove_frame(&frame);
                self.counters.frames_reset += 1;
                match build(wire).map(|sample| self.registry.add_transform(sample)) {
                    Some(Ok(())) => self.stored(stamp),
                    // Unreachable for a sample that just passed every other
                    // check; counted, never unwrapped.
                    _ => self.counters.unclassified += 1,
                }
            }

            // Two publishers disagree about the tree, or one publishes a
            // frame on both topics. Neither is resolved here: re-parenting
            // or re-adding the frame on every failure would let the two take
            // turns wiping each other's history. Report it and drop the
            // element. These variants do not name the frame; the element in
            // hand does.
            Err(
                error @ (RegistryError::ReparentingNotSupported { .. }
                | RegistryError::StaticDynamicConflict
                | RegistryError::CycleDetected
                | RegistryError::SelfReferentialFrame),
            ) => {
                println!("ingest: dropped {} -> {}: {error}", wire.parent, wire.child);
                self.counters.conflicting += 1;
            }

            // Not reachable for a transform straight from `Transform::new`,
            // which validated it. A bridge that inserts lookup results (a
            // flattened chain) reaches it.
            Err(RegistryError::NonUnitRotation(_) | RegistryError::NonFiniteValues) => {
                self.counters.malformed += 1;
            }

            // The enum is non-exhaustive: a variant a later release adds
            // must be counted as a drop, never pass as stored.
            Err(error) => {
                println!("ingest: dropped {} -> {}: {error}", wire.parent, wire.child);
                self.counters.unclassified += 1;
            }
        }
    }

    fn stored(
        &mut self,
        stamp: Option<Timestamp>,
    ) {
        self.counters.stored += 1;
        // `Stamp` has no order on purpose; compare the instants.
        self.newest_stored = self.newest_stored.max(stamp);
    }

    /// The newest transform the chain can serve, or what to do without it.
    fn latest(
        &self,
        target: &str,
        source: &str,
    ) -> Result<Transform, Unavailable> {
        let instant = match self.registry.latest_common_time(target, source) {
            Ok(Stamp::At(instant)) => instant,
            // An all-static chain serves every instant; any one will do.
            Ok(Stamp::Static) => Timestamp::zero(),
            Err(error) => return Err(Unavailable::from(error)),
        };
        self.registry
            .get_transform(target, source, instant)
            .map_err(Unavailable::from)
    }
}

/// Why a lookup has no answer, by what the caller can do about it.
#[derive(Debug, PartialEq)]
enum Unavailable {
    /// A frame nobody has published: a typo, or a publisher not up yet.
    UnknownFrame(String),
    /// Both frames exist in different trees. No other instant helps; only
    /// an insert that joins the trees does.
    Disconnected,
    /// `frame` holds samples, none of which serve the request. Ask again
    /// inside `covered`, or once newer data has arrived.
    OutsideCoverage {
        frame: String,
        covered: (Timestamp, Timestamp),
    },
    /// `frame` holds nothing at all. No instant helps until its publisher
    /// sends again.
    NoData { frame: String },
    /// A numeric or time failure on the chain itself.
    Failed,
}

impl From<RegistryError> for Unavailable {
    fn from(error: RegistryError) -> Self {
        match error {
            RegistryError::UnknownFrame(frame) => Self::UnknownFrame(frame),
            // Decided before any sampling, so the answer for a disconnected
            // pair does not change with the instant asked for.
            RegistryError::Disconnected { .. } => Self::Disconnected,
            RegistryError::NotFoundAt {
                frame,
                covered: Some(covered),
                ..
            }
            | RegistryError::NoCommonTime {
                frame,
                covered: Some(covered),
                ..
            } => Self::OutsideCoverage { frame, covered },
            RegistryError::NotFoundAt {
                frame,
                covered: None,
                ..
            }
            | RegistryError::NoCommonTime {
                frame,
                covered: None,
                ..
            } => Self::NoData { frame },
            _ => Self::Failed,
        }
    }
}

const fn at(sec: i32) -> WireTime {
    WireTime { sec, nanosec: 0 }
}

fn dynamic(
    parent: &'static str,
    child: &'static str,
    sec: i32,
    x: f64,
) -> WireTransform {
    WireTransform {
        stamp: at(sec),
        parent,
        child,
        translation: [x, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        is_static: false,
    }
}

fn seconds(sec: u64) -> Timestamp {
    Timestamp::from_nanos(sec * 1_000_000_000)
}

fn main() {
    let mut bridge = Bridge::new();

    // 1. A healthy start, with everything a publisher can get wrong.
    bridge.ingest(&WireMessage {
        clock: at(100),
        transforms: vec![
            WireTransform {
                is_static: true,
                translation: [0.2, 0.0, 0.1],
                ..dynamic("base", "lidar", 0, 0.0)
            },
            dynamic("map", "base", 100, 1.0),
            // Malformed: a pre-epoch stamp, a zero quaternion, a NaN.
            dynamic("map", "base", -5, 0.0),
            WireTransform {
                rotation: [0.0; 4],
                ..dynamic("map", "base", 100, 0.0)
            },
            dynamic("map", "base", 100, f64::NAN),
            // Conflicting: another parent, the other kind, a self edge, a
            // cycle.
            dynamic("odom", "base", 100, 0.0),
            WireTransform {
                is_static: true,
                ..dynamic("map", "base", 0, 0.0)
            },
            dynamic("base", "base", 100, 0.0),
            dynamic("lidar", "map", 100, 0.0),
        ],
    });
    assert_eq!(
        bridge.counters,
        Counters {
            stored: 2,
            malformed: 3,
            conflicting: 4,
            ..Counters::default()
        }
    );

    // 2. The stream advances; "latest" follows it.
    bridge.ingest(&WireMessage {
        clock: at(101),
        transforms: vec![dynamic("map", "base", 101, 2.0)],
    });
    let latest = bridge.latest("map", "lidar").unwrap();
    assert_eq!(latest.timestamp(), Stamp::At(seconds(101)));
    assert_eq!(latest.translation(), Vector3::new(2.2, 0.0, 0.1));

    // 3. A straggler from before the window: refused, counted, and the
    //    frame keeps its history.
    bridge.ingest(&WireMessage {
        clock: at(101),
        transforms: vec![dynamic("map", "base", 80, 9.0)],
    });
    assert_eq!(bridge.counters.late, 1);
    assert_eq!(bridge.counters.frames_reset, 0);
    assert_eq!(bridge.latest("map", "lidar").unwrap(), latest);

    // 4. One publisher stamps a sample 400 s ahead. It is accepted: the
    //    sample that moves a window forward always is. The next honest
    //    sample is the one refused, and the clock says `newest` is at fault.
    //    (A bridge that owns a clock can also refuse the stamp from the
    //    future before inserting it. That is the better guard where it
    //    applies; this path is for the sample that got in.)
    bridge.ingest(&WireMessage {
        clock: at(102),
        transforms: vec![dynamic("map", "base", 500, 7.0)],
    });
    assert_eq!(bridge.counters.stored, 4);
    bridge.ingest(&WireMessage {
        clock: at(103),
        transforms: vec![dynamic("map", "base", 103, 3.0)],
    });
    assert_eq!(bridge.counters.frames_reset, 1);
    assert_eq!(bridge.counters.late, 1);
    let latest = bridge.latest("map", "lidar").unwrap();
    assert_eq!(latest.timestamp(), Stamp::At(seconds(103)));
    assert_eq!(latest.translation(), Vector3::new(3.2, 0.0, 0.1));

    // 5. The clock steps back by more than the window (a bag looping).
    //    Every dynamic sample goes; the static mount and the tree stay.
    bridge.ingest(&WireMessage {
        clock: at(20),
        transforms: vec![
            dynamic("map", "base", 20, 5.0),
            dynamic("odom", "base", 20, 0.0),
        ],
    });
    assert_eq!(bridge.counters.clock_resets, 1);
    assert_eq!(bridge.counters.late, 1);
    assert_eq!(bridge.counters.conflicting, 5);
    let latest = bridge.latest("map", "lidar").unwrap();
    assert_eq!(latest.timestamp(), Stamp::At(seconds(20)));
    assert_eq!(latest.translation(), Vector3::new(5.2, 0.0, 0.1));

    // 6. The read side, by what the caller can do.
    bridge.ingest(&WireMessage {
        clock: at(21),
        transforms: vec![dynamic("world", "marker", 21, 0.0)],
    });
    assert_eq!(
        bridge.latest("map", "gripper"),
        Err(Unavailable::UnknownFrame("gripper".into()))
    );
    assert_eq!(
        bridge.latest("map", "marker"),
        Err(Unavailable::Disconnected)
    );
    // Also at an instant neither tree can serve: topology is reported
    // before sampling. 2.x answered this one `NotFoundAt`, which sends the
    // caller off to wait for data that would not have helped.
    assert_eq!(
        bridge
            .registry
            .get_transform("map", "marker", seconds(5))
            .map_err(Unavailable::from),
        Err(Unavailable::Disconnected)
    );
    assert_eq!(
        bridge
            .registry
            .get_transform("map", "lidar", seconds(5))
            .map_err(Unavailable::from),
        Err(Unavailable::OutsideCoverage {
            frame: "base".into(),
            covered: (seconds(20), seconds(20)),
        })
    );
    // A frame drained by a second step back holds nothing until its
    // publisher sends again.
    bridge.ingest(&WireMessage {
        clock: at(1),
        transforms: Vec::new(),
    });
    assert_eq!(bridge.counters.clock_resets, 2);
    assert_eq!(
        bridge.latest("map", "lidar"),
        Err(Unavailable::NoData {
            frame: "base".into()
        })
    );

    println!("ingest: ok ({:?})", bridge.counters);
}
