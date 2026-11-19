//! Feed a registry from a message stream, deciding what to do with every
//! transform the registry or the bridge refuses.
//!
//! `TfMessage` stands in for a middleware type such as ROS 2's
//! `TransformStamped`, and a slice of them for one `TFMessage` off the
//! wire. The bridge owns four decisions the registry cannot make for it:
//! converting the stamp, what each error means for the stream, what to do
//! with the rest of a message when one member is refused, and what a
//! sample outside the retention window means for the stored history.

use approx::assert_abs_diff_eq;
use core::time::Duration;
use transforms::{
    BatchError, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform,
    TransformError, Vector3,
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

/// What the bridge did with one transform of a message.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Stored,
    BadStamp,
    EmptyFrame,
    BadNumbers,
    BadTopology,
    ParentConflict,
    KindConflict,
    /// Older than the frame's retention window. The bridge drops it; see
    /// `main` for the case where the window itself is in the wrong place.
    Expired,
    Unexpected,
}

/// What the bridge does with a message after `insert_all` refused it.
#[derive(Clone, Copy, Debug)]
enum Fallback {
    /// Take the member at `BatchError::index` out and submit the rest
    /// again, until the batch is accepted. What is stored goes in
    /// atomically.
    Resubmit,
    /// Insert the members one at a time. A reader between two of those
    /// inserts sees part of the message.
    PerMember,
}

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// `builtin_interfaces/Time { sec: i32, nanosec: u32 }` to a `Timestamp`,
/// as the `Timestamp` documentation gives it. Negative seconds are ordinary
/// instants; a `nanosec` of a second or more is a malformed stamp, and
/// carrying it into the seconds would guess what the publisher meant.
fn from_ros(
    sec: i32,
    nanosec: u32,
) -> Option<Timestamp> {
    let nanosec = i64::from(nanosec);
    if nanosec >= NANOS_PER_SEC {
        return None; // denormalized stamp
    }
    // Cannot overflow: |sec| * 1e9 + nanosec < 2.2e18 < i64::MAX.
    Some(Timestamp::from_nanos(
        i64::from(sec) * NANOS_PER_SEC + nanosec,
    ))
}

fn seconds(sec: i32) -> Timestamp {
    from_ros(sec, 0).unwrap()
}

/// Builds the transform a message describes, or says why it cannot be
/// built. Everything refused here never reaches the registry: a `Transform`
/// that exists has a valid stamp, non-empty names, finite numbers and a
/// unit rotation.
fn convert(msg: &TfMessage) -> Result<Transform, Outcome> {
    let edge = format!("{} -> {}", msg.frame_id, msg.child_frame_id);
    let stamp = if msg.is_static {
        Stamp::Static
    } else if let Some(t) = from_ros(msg.sec, msg.nanosec) {
        Stamp::At(t)
    } else {
        println!(
            "dropped {edge}: nanosec {} is a second or more",
            msg.nanosec
        );
        return Err(Outcome::BadStamp);
    };

    let [x, y, z] = msg.translation;
    let [qx, qy, qz, qw] = msg.rotation;
    Rotation::from_wxyz(qw, qx, qy, qz)
        .and_then(|rotation| {
            Transform::new(
                msg.frame_id,
                msg.child_frame_id,
                Vector3::new(x, y, z),
                rotation,
                stamp,
            )
        })
        .map_err(|error| {
            println!("dropped {edge}: {error}");
            match error {
                // The publisher sent garbage. Do not zero it or rescale it
                // into something storable.
                TransformError::NonFinite | TransformError::NonUnitRotation { .. } => {
                    Outcome::BadNumbers
                }
                // The publisher left a frame name unset (an empty
                // `frame_id`). Do not guess a name.
                TransformError::EmptyFrameName => Outcome::EmptyFrame,
                // `TransformError` is non-exhaustive, and its remaining
                // variants are not reported by a constructor.
                _ => Outcome::Unexpected,
            }
        })
}

/// The decision for each error an insert can report. Every one drops the
/// transform; they differ in who has to act.
fn rejected(
    transform: &Transform,
    error: &InsertError,
) -> Outcome {
    println!(
        "dropped {} -> {}: {error}",
        transform.target(),
        transform.source()
    );
    match error {
        // The publisher's frame layout is wrong; no retry will fix it.
        InsertError::SelfEdge { .. } | InsertError::Cycle { .. } => Outcome::BadTopology,
        // Two publishers disagree on the child's parent. Calling `reparent`
        // here would let them take turns wiping each other's history; that
        // call belongs to whoever decides the layout, not to an error
        // handler.
        InsertError::ParentMismatch { .. } => Outcome::ParentConflict,
        // The frame arrived as static and as dynamic. Removing and
        // re-adding it would make arrival order pick the kind; an operator
        // has to fix the publisher.
        InsertError::KindMismatch { .. } => Outcome::KindConflict,
        // The sample is older than the frame's newest stamp minus the
        // retention window. A late sample is dropped: the registry did not
        // store it and no lookup inside the window needed it.
        InsertError::Expired { .. } => Outcome::Expired,
        // `InsertError` is non-exhaustive, and its remaining variants are
        // reported by `reparent` only.
        _ => Outcome::Unexpected,
    }
}

struct Bridge {
    registry: Registry,
    fallback: Fallback,
}

impl Bridge {
    /// Stores one message and reports an outcome per member, in order.
    fn ingest(
        &mut self,
        message: &[TfMessage],
    ) -> Vec<Outcome> {
        // Members that cannot be built are dropped here and do not count
        // against the rest of the message.
        let mut outcomes = Vec::with_capacity(message.len());
        let mut batch = Vec::with_capacity(message.len());
        let mut members = Vec::with_capacity(message.len());
        for (member, msg) in message.iter().enumerate() {
            match convert(msg) {
                Ok(transform) => {
                    outcomes.push(Outcome::Stored);
                    batch.push(transform);
                    members.push(member);
                }
                Err(outcome) => outcomes.push(outcome),
            }
        }

        // `insert_all` is all or nothing: one refused member refuses the
        // whole message, also the fresh samples of every other frame in
        // it. The error names the member, and the batch is still the
        // bridge's to submit again.
        match self.fallback {
            Fallback::Resubmit => {
                while let Err(BatchError { index, error }) = self.registry.insert_all(&batch) {
                    outcomes[members[index]] = rejected(&batch[index], &error);
                    batch.remove(index);
                    members.remove(index);
                }
            }
            Fallback::PerMember => {
                if self.registry.insert_all(&batch).is_err() {
                    for (transform, member) in batch.iter().zip(members) {
                        if let Err(error) = self.registry.insert(transform) {
                            outcomes[member] = rejected(transform, &error);
                        }
                    }
                }
            }
        }
        outcomes
    }
}

const IDENTITY: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

fn dynamic(
    sec: i32,
    nanosec: u32,
    frame_id: &'static str,
    child_frame_id: &'static str,
    x: f64,
) -> TfMessage {
    TfMessage {
        sec,
        nanosec,
        frame_id,
        child_frame_id,
        translation: [x, 0.0, 0.0],
        rotation: IDENTITY,
        is_static: false,
    }
}

fn fixed(
    frame_id: &'static str,
    child_frame_id: &'static str,
    x: f64,
) -> TfMessage {
    TfMessage {
        is_static: true,
        ..dynamic(0, 0, frame_id, child_frame_id, x)
    }
}

/// Feeds `message` to the bridge and checks the outcome of every member.
fn expect(
    bridge: &mut Bridge,
    message: &[TfMessage],
    expected: &[Outcome],
) {
    assert_eq!(bridge.ingest(message), expected);
}

/// The whole stream, under one fallback. Both fallbacks store the same
/// samples and report the same outcomes.
fn run(fallback: Fallback) {
    use Outcome::{
        BadNumbers, BadStamp, BadTopology, EmptyFrame, Expired, KindConflict, ParentConflict,
        Stored,
    };

    // Every dynamic frame keeps the five seconds behind its newest sample.
    let mut bridge = Bridge {
        registry: Registry::with_max_age(Duration::from_secs(5)),
        fallback,
    };

    // Mounts, then the first samples of two moving frames.
    expect(
        &mut bridge,
        &[fixed("base", "lidar", 0.5), fixed("world", "marker", 1.0)],
        &[Stored, Stored],
    );
    expect(
        &mut bridge,
        &[
            dynamic(10, 0, "odom", "base", 10.0),
            dynamic(10, 500_000_000, "odom", "wheel", 0.0),
        ],
        &[Stored, Stored],
    );

    // Members that cannot become a `Transform`. The last one is fine and is
    // stored: the bridge never offered the others to the registry.
    expect(
        &mut bridge,
        &[
            dynamic(11, 1_000_000_000, "odom", "base", 0.0),
            dynamic(11, 0, "odom", "", 0.0),
            dynamic(11, 0, "odom", "base", f64::NAN),
            TfMessage {
                rotation: [0.0, 0.0, 0.0, 2.0],
                ..dynamic(11, 0, "odom", "base", 0.0)
            },
            dynamic(11, 0, "odom", "base", 11.0),
        ],
        &[BadStamp, EmptyFrame, BadNumbers, BadNumbers, Stored],
    );

    // Valid transforms the tree refuses. Each refusal rejects the message
    // once; the fallback gets the last member stored all the same.
    expect(
        &mut bridge,
        &[
            dynamic(11, 0, "base", "base", 0.0),
            dynamic(11, 0, "lidar", "odom", 0.0),
            dynamic(11, 0, "map", "base", 0.0),
            dynamic(11, 0, "base", "lidar", 0.0),
            dynamic(11, 0, "odom", "wheel", 0.0),
        ],
        &[
            BadTopology,
            BadTopology,
            ParentConflict,
            KindConflict,
            Stored,
        ],
    );

    // A message with one late member: `wheel` at 3 s, while `wheel` already
    // holds 11 s and keeps nothing older than 6 s. First, what the registry
    // itself does with it.
    let late_member = [
        dynamic(12, 0, "odom", "base", 12.0),
        dynamic(3, 0, "odom", "wheel", 0.0),
        dynamic(12, 500_000_000, "odom", "base", 12.5),
    ];
    let batch: Vec<Transform> = late_member
        .iter()
        .map(|msg| convert(msg).unwrap())
        .collect();
    let BatchError { index, error } = bridge.registry.insert_all(&batch).unwrap_err();
    assert_eq!(index, 1);
    match error {
        InsertError::Expired {
            frame,
            stamp,
            newest,
            ..
        } => {
            assert_eq!(frame, "wheel");
            assert_eq!((stamp, newest), (seconds(3), seconds(11)));
        }
        other => panic!("expected Expired, got {other:?}"),
    }
    // The two fresh `base` samples were refused with it: `base` still ends
    // at 11 s.
    assert!(matches!(
        bridge.registry.lookup("odom", "base", seconds(12)),
        Err(LookupError::OutOfRange { latest, .. }) if latest == seconds(11)
    ));
    // The bridge's fallback keeps them and drops the late member alone.
    expect(&mut bridge, &late_member, &[Stored, Expired, Stored]);
    let odom_base = bridge.registry.lookup("odom", "base", seconds(12)).unwrap();
    assert_abs_diff_eq!(odom_base.translation().x, 12.0, epsilon = 1e-12);

    // A sample stamped ahead: a publisher with a wrong clock sends `base`
    // at 1000 s. Nothing distinguishes it from a real sample, so it is
    // stored, and it moves `base`'s window to 995 s..1000 s.
    expect(
        &mut bridge,
        &[dynamic(1000, 0, "odom", "base", 0.0)],
        &[Stored],
    );
    // From here every correct `base` sample reports `Expired`, while
    // `wheel` in the same message is unaffected. The error is loud, and
    // dropping these as late samples would starve the frame for 16 minutes.
    expect(
        &mut bridge,
        &[
            dynamic(13, 0, "odom", "base", 13.0),
            dynamic(13, 0, "odom", "wheel", 0.0),
        ],
        &[Expired, Stored],
    );
    // Whether `Expired` means a late sample or a misplaced window is the
    // operator's call: the registry holds no clock to judge by. Recovery
    // for one frame is `remove_frame`: it drops `base`'s history and
    // releases its parent and kind until the next insert. `lidar` below it
    // keeps its mount.
    assert!(bridge.registry.remove_frame("base"));
    expect(
        &mut bridge,
        &[dynamic(13, 0, "odom", "base", 13.0)],
        &[Stored],
    );

    // The same fault on `wheel`, recovered the other way. The error
    // carries the stamp the window hangs from, and
    // `remove_samples_through` removes it. The cutoff is inclusive and
    // applies to every dynamic frame, so `base` is drained with it. Every
    // frame keeps its parent and kind, and a drained frame has no newest
    // stamp left to expire against. `Timestamp::MAX` as the cutoff clears
    // all dynamic history without knowing the stamp.
    expect(
        &mut bridge,
        &[dynamic(2000, 0, "odom", "wheel", 0.0)],
        &[Stored],
    );
    let wheel = convert(&dynamic(14, 0, "odom", "wheel", 0.0)).unwrap();
    let Err(InsertError::Expired { newest, .. }) = bridge.registry.insert(&wheel) else {
        panic!("expected Expired");
    };
    assert_eq!(newest, seconds(2000));
    bridge.registry.remove_samples_through(newest);
    bridge.registry.insert(&wheel).unwrap();

    // Not a timing problem: `base` is known but the cleanup emptied it, and
    // no instant can be served until its publisher sends again.
    assert!(matches!(
        bridge.registry.lookup("odom", "lidar", seconds(13)),
        Err(LookupError::NoData { frame, .. }) if frame == "base"
    ));
    expect(
        &mut bridge,
        &[
            dynamic(14, 0, "odom", "base", 14.0),
            dynamic(15, 0, "odom", "base", 15.0),
        ],
        &[Stored, Stored],
    );

    // Lookups fail for different reasons, and each asks something different
    // of the caller.
    let registry = &bridge.registry;
    let t = from_ros(14, 500_000_000).unwrap();

    // Served: interpolated odom -> base, static base -> lidar.
    let odom_lidar = registry.lookup("odom", "lidar", t).unwrap();
    assert_abs_diff_eq!(odom_lidar.translation().x, 15.0, epsilon = 1e-12);
    assert_abs_diff_eq!(odom_lidar.translation().y, 0.0, epsilon = 1e-12);
    assert_abs_diff_eq!(odom_lidar.translation().z, 0.0, epsilon = 1e-12);

    // A frame nothing has published: a typo, or a publisher not up yet.
    assert!(matches!(
        registry.lookup("odom", "gripper", t),
        Err(LookupError::UnknownFrame { frame, .. }) if frame == "gripper"
    ));

    // Two trees: the topology has to change. Topology is diagnosed before
    // coverage, so this answer does not depend on the instant asked for.
    assert!(matches!(
        registry.lookup("odom", "marker", seconds(1)),
        Err(LookupError::Disconnected { a, b, .. }) if a == "odom" && b == "marker"
    ));

    // `base` holds data, but not at a stamp from before the cleanup. That
    // is a timing problem: ask inside the range, or ask
    // `latest_common_time`.
    match registry.lookup("odom", "lidar", seconds(11)) {
        Err(LookupError::OutOfRange {
            frame,
            requested,
            earliest,
            latest,
            ..
        }) => {
            assert_eq!(frame, "base");
            assert_eq!(
                (requested, earliest, latest),
                (seconds(11), seconds(14), seconds(15))
            );
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }

    println!(
        "{fallback:?}: odom -> lidar at {t} s is {:?}",
        odom_lidar.translation()
    );
}

fn main() {
    // The stamp conversion: negative seconds are fine, and the nanosecond
    // part counts forward from the floored second.
    assert_eq!(
        from_ros(-1, 500_000_000),
        Some(Timestamp::from_nanos(-500_000_000))
    );
    assert_eq!(from_ros(11, 1_000_000_000), None);

    // An empty frame name is refused where the transform is built, before
    // any registry sees it.
    assert_eq!(
        Transform::new(
            "odom",
            "",
            Vector3::new(0.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Stamp::Static
        ),
        Err(TransformError::EmptyFrameName)
    );

    run(Fallback::Resubmit);
    run(Fallback::PerMember);
}
