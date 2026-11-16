//! Feed a registry from a message stream, deciding what to do with every
//! message the registry or the bridge refuses.
//!
//! `TfMessage` stands in for a middleware type such as ROS 2's
//! `TransformStamped`. The bridge owns three decisions the registry cannot
//! make for it: converting the stamp, what each insert error means for the
//! stream, and what a backward clock jump means for the stored history.

use approx::assert_abs_diff_eq;
use core::time::Duration;
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

/// One transform as it arrives off the wire. `rotation` is `[x, y, z, w]`.
struct TfMessage {
    sec: i32,
    nanosec: u32,
    frame_id: &'static str,
    child_frame_id: &'static str,
    translation: [f64; 3],
    rotation: [f64; 4],
    is_static: bool,
}

/// What the bridge did with a message.
#[derive(Debug, PartialEq)]
enum Outcome {
    Stored,
    /// Stored after a backward clock jump wiped the dynamic history.
    StoredAfterReset,
    BadStamp,
    EmptyFrame,
    BadNumbers,
    BadTopology,
    ParentConflict,
    KindConflict,
    Unexpected,
}

/// This bridge's assumption, not a rule of the crate: a stamp older than the
/// newest one by more than this is a clock reset (a restarted simulation, a
/// looped recording), not a late sample. The registry cannot tell the two
/// apart, and neither can a bridge without knowing its publishers.
const CLOCK_JUMP: Duration = Duration::from_secs(5);

/// Converts a wire stamp, or refuses it. Clamping a negative `sec` to zero
/// would alias every such message onto one instant, where each overwrites
/// the last.
fn checked_stamp(
    sec: i32,
    nanosec: u32,
) -> Option<Timestamp> {
    let sec = u64::try_from(sec).ok()?;
    (nanosec < 1_000_000_000)
        .then(|| Timestamp::from_nanos(sec * 1_000_000_000 + u64::from(nanosec)))
}

struct Bridge {
    registry: Registry,
    /// The newest dynamic stamp stored: the clock-jump reference. It is one
    /// value for the whole stream, so one publisher with a stale clock looks
    /// like a reset of all of them.
    newest: Option<Timestamp>,
}

impl Bridge {
    fn ingest(
        &mut self,
        msg: &TfMessage,
    ) -> Outcome {
        let stamp = if msg.is_static {
            Stamp::Static
        } else {
            match checked_stamp(msg.sec, msg.nanosec) {
                Some(t) => Stamp::At(t),
                None => return Outcome::BadStamp,
            }
        };

        let [x, y, z] = msg.translation;
        let [qx, qy, qz, qw] = msg.rotation;
        // Construction validates the numbers; converting its error lets one
        // `match` below reach every cause.
        let transform = match Transform::new(
            msg.frame_id,
            msg.child_frame_id,
            Vector3::new(x, y, z),
            Quaternion::from_wxyz(qw, qx, qy, qz),
            stamp,
        ) {
            Ok(transform) => transform,
            Err(error) => return Self::rejected(msg, &RegistryError::from(error)),
        };

        // The registry does not detect a clock that steps backward; whoever
        // owns the clock must, and here that is the bridge. Left alone, the
        // restarted stream would sit below the old history: a lookup
        // between the two interpolates across the jump, and a registry with
        // a `max_age` accepts a sample older than its window and evicts it
        // at once.
        //
        // `remove_transforms_before` discards the samples and the newest
        // timestamp of every dynamic frame at once and keeps each frame's
        // parent and kind. Its cutoff is strict, hence one nanosecond past
        // the newest stored stamp (an `i32` of seconds cannot reach the
        // `Timestamp` maximum, where no such cutoff exists).
        // `Registry::remove_frame` does it for one frame, and also releases
        // that frame's parent and kind until its next insert.
        //
        // The cost of this policy: one message with an old stamp empties
        // every dynamic frame. The clear also runs before the insert below,
        // so a message the registry then refuses, such as one with an empty
        // frame name, has still cleared the history.
        let mut reset = false;
        if let (Stamp::At(t), Some(newest)) = (stamp, self.newest) {
            if (newest - t).is_ok_and(|gap| gap > CLOCK_JUMP) {
                let cutoff = (newest + Duration::from_nanos(1)).unwrap();
                self.registry.remove_transforms_before(cutoff);
                self.newest = None;
                reset = true;
            }
        }

        // One transform per call: a refused message costs only itself.
        match self.registry.add_transform(transform) {
            Ok(()) => {
                if let Stamp::At(t) = stamp {
                    self.newest = self.newest.max(Some(t));
                }
                if reset {
                    Outcome::StoredAfterReset
                } else {
                    Outcome::Stored
                }
            }
            Err(error) => Self::rejected(msg, &error),
        }
    }

    /// The decision for each error an insert can report. Every one drops
    /// the message; they differ in who has to act.
    fn rejected(
        msg: &TfMessage,
        error: &RegistryError,
    ) -> Outcome {
        let edge = format!("{} -> {}", msg.frame_id, msg.child_frame_id);
        match error {
            // The publisher sent garbage. Do not normalize or zero it into
            // something storable.
            RegistryError::NonFiniteValues | RegistryError::NonUnitRotation(_) => {
                println!("dropped {edge}: {error}");
                Outcome::BadNumbers
            }
            // The publisher left a frame name unset (an empty `frame_id`).
            // The registry refuses it at the insert, so the bridge needs no
            // check of its own. Do not guess a name.
            RegistryError::EmptyFrameName => {
                println!("dropped {edge}: {error}");
                Outcome::EmptyFrame
            }
            // The publisher's frame layout is wrong; no retry will fix it.
            RegistryError::SelfReferentialFrame | RegistryError::CycleDetected => {
                println!("dropped {edge}: {error}");
                Outcome::BadTopology
            }
            // Two publishers disagree on the child's parent. Calling
            // `reparent_frame` here would let them take turns wiping each
            // other's history; that call belongs to whoever decides the
            // layout, not to an error handler.
            RegistryError::ReparentingNotSupported { current_parent } => {
                println!("dropped {edge}: the child already hangs under {current_parent}");
                Outcome::ParentConflict
            }
            // The frame arrived as static and as dynamic. Removing and
            // re-adding it would make arrival order pick the kind; an
            // operator has to fix the publisher.
            RegistryError::StaticDynamicConflict => {
                println!("dropped {edge}: {error}");
                Outcome::KindConflict
            }
            // `RegistryError` is non-exhaustive, and its remaining variants
            // are not reported by an insert.
            _ => {
                println!("dropped {edge}: unexpected {error}");
                Outcome::Unexpected
            }
        }
    }
}

fn main() {
    const IDENTITY: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    let dynamic = |sec, nanosec, frame_id, child_frame_id, x| TfMessage {
        sec,
        nanosec,
        frame_id,
        child_frame_id,
        translation: [x, 0.0, 0.0],
        rotation: IDENTITY,
        is_static: false,
    };
    let fixed = |frame_id, child_frame_id, x| TfMessage {
        is_static: true,
        ..dynamic(0, 0, frame_id, child_frame_id, x)
    };

    let stream = [
        (fixed("base", "lidar", 0.5), Outcome::Stored),
        (fixed("world", "marker", 1.0), Outcome::Stored),
        (dynamic(10, 0, "odom", "base", 10.0), Outcome::Stored),
        (
            dynamic(10, 500_000_000, "odom", "wheel", 0.0),
            Outcome::Stored,
        ),
        (dynamic(11, 0, "odom", "base", 11.0), Outcome::Stored),
        (dynamic(-1, 0, "odom", "base", 0.0), Outcome::BadStamp),
        (
            dynamic(11, 1_000_000_000, "odom", "base", 0.0),
            Outcome::BadStamp,
        ),
        (dynamic(11, 0, "odom", "", 0.0), Outcome::EmptyFrame),
        (
            dynamic(11, 0, "odom", "base", f64::NAN),
            Outcome::BadNumbers,
        ),
        (
            TfMessage {
                rotation: [0.0, 0.0, 0.0, 2.0],
                ..dynamic(11, 0, "odom", "base", 0.0)
            },
            Outcome::BadNumbers,
        ),
        (dynamic(11, 0, "base", "base", 0.0), Outcome::BadTopology),
        (dynamic(11, 0, "lidar", "odom", 0.0), Outcome::BadTopology),
        (dynamic(11, 0, "map", "base", 0.0), Outcome::ParentConflict),
        (dynamic(11, 0, "base", "lidar", 0.0), Outcome::KindConflict),
        // 9 s behind the newest stamp, past `CLOCK_JUMP`: the clock went back.
        (
            dynamic(2, 0, "odom", "base", 2.0),
            Outcome::StoredAfterReset,
        ),
        (dynamic(3, 0, "odom", "base", 3.0), Outcome::Stored),
    ];

    let mut bridge = Bridge {
        registry: Registry::new(),
        newest: None,
    };
    for (i, (msg, expected)) in stream.iter().enumerate() {
        assert_eq!(bridge.ingest(msg), *expected, "message {i}");
    }

    // Lookups fail for different reasons, and each asks something different
    // of the caller.
    let registry = &bridge.registry;
    let t = Timestamp::from_nanos(2_500_000_000);

    // Served: interpolated odom -> base, static base -> lidar.
    let odom_lidar = registry.get_transform("odom", "lidar", t).unwrap();
    assert_abs_diff_eq!(
        odom_lidar.translation(),
        Vector3::new(3.0, 0.0, 0.0),
        epsilon = 1e-12
    );

    // A frame nothing has published: a typo, or a publisher not up yet.
    assert!(matches!(
        registry.get_transform("odom", "gripper", t),
        Err(RegistryError::UnknownFrame(frame)) if frame == "gripper"
    ));

    // Two trees: the topology has to change. This reports `Disconnected`
    // only because `marker` is static. A sampling failure is reported
    // first, so a disconnected pair whose frames cannot serve the instant
    // reports `NotFoundAt` instead.
    assert!(matches!(
        registry.get_transform("odom", "marker", t),
        Err(RegistryError::Disconnected { .. })
    ));

    // `base` holds data, but not at the pre-reset stamp. Here that is a
    // timing problem: ask inside `covered`, or ask `latest_common_time`.
    // `NotFoundAt` does not prove the frames are connected (see above).
    let stale = Timestamp::from_nanos(10_500_000_000);
    match registry.get_transform("odom", "lidar", stale) {
        Err(RegistryError::NotFoundAt { frame, covered, .. }) => {
            assert_eq!(frame, "base");
            let range = (
                Timestamp::from_nanos(2_000_000_000),
                Timestamp::from_nanos(3_000_000_000),
            );
            assert_eq!(covered, Some(range));
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }

    // Not a timing problem: `wheel` is known but the reset emptied it, and
    // no time can be served until its publisher sends again.
    match registry.get_transform("odom", "wheel", t) {
        Err(RegistryError::NotFoundAt { frame, covered, .. }) => {
            assert_eq!(frame, "wheel");
            assert_eq!(covered, None);
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }
}
