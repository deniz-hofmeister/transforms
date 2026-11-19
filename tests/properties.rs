//! Property-based tests for the geometric, time and registry invariants.
//!
//! Nothing here depends on a feature: the file runs unchanged with and
//! without `serde`.

use core::time::Duration;
use proptest::{
    prelude::*,
    test_runner::{RngSeed, TestCaseError},
};
use std::collections::BTreeMap;
use transforms::{
    BatchError, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform,
    TransformError, Vector3,
};

/// The seed every property runs on. It is pinned so that a run exercises
/// the same cases each time and a failure reproduces anywhere; the value
/// itself means nothing.
const SEED: u64 = 0x7472_616E_7366_6F72;

/// The configuration of one `proptest!` block: `cases` cases on [`SEED`].
fn config(cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases,
        rng_seed: RngSeed::Fixed(SEED),
        ..ProptestConfig::default()
    }
}

/// How far the squared norm of any observable [`Rotation`] may sit from 1.
const BAND: f64 = 8.0 * f64::EPSILON;

/// The rounding budget of one geometric operation (a composition, an
/// inversion, an interpolation, applying a transform to a point), relative
/// to the largest length the operation handles.
///
/// Derivation, to first order in the unit roundoff `u = EPSILON / 2`. One
/// operation rotates a vector by the sandwich product: two quaternion
/// products whose components are sums of at most four products of factors
/// bounded by the vector's length `M`, so each product adds at most about
/// `4u * 2M` per component, and the second carries the error of the first
/// through an isometry. That is below `16u * M`. The rotation in use is
/// itself off by its own history: its squared norm sits within `8 *
/// EPSILON` of 1 (a scale error of `8u * M` on the rotated vector, which
/// `inverse` divides out and composition does not), and a quaternion
/// product adds at most about `8u` to each of its components, which moves
/// a vector of length `M` by at most about `16u * M`. The closing addition
/// or scaling adds `u * M`. The sum is about `41u * M`, below `21 *
/// EPSILON * M`; the constant leaves a factor of three for the
/// second-order terms and for the bounds above being per component. A
/// result of `k` operations is held to `k * ROUNDING * M`.
const ROUNDING: f64 = 64.0 * f64::EPSILON;

/// How far a component of an interpolated rotation may sit from the
/// in-test reference slerp.
///
/// Both sides compute the same dot product bit for bit and differ in the
/// transcendental functions (`libm` in the crate, `std` here; each within
/// one ulp) and in the last normalization. The arc `theta = acos(dot)`
/// loses accuracy as `dot` nears 1, by about `EPSILON / theta^2` relative,
/// and the weights `sin(r * theta) / sin(theta)` are insensitive to it by
/// the matching factor `theta^2 * (1 - r^2) / 3`, so the weights agree to
/// a few `EPSILON` at every arc length. Below `theta = 1e-7` the reference
/// blends linearly, which differs from the slerp weights by at most
/// `theta^2 / 15 < 1e-15`. The crate's renormalization moves a component
/// by at most `4 * EPSILON`. The sum is below `16 * EPSILON`; the constant
/// leaves a factor of four.
const SLERP_TOLERANCE: f64 = 64.0 * f64::EPSILON;

/// 2^53 nanoseconds: beyond it `f64` no longer represents every
/// nanosecond count.
const ACCURACY_CLIFF_NANOS: i64 = 1 << 53;

const NANOS_PER_SEC: i64 = 1_000_000_000;

// ---- small helpers ----

fn at(nanos: i64) -> Stamp {
    Stamp::At(Timestamp::from_nanos(nanos))
}

fn frame(index: usize) -> String {
    format!("f{index}")
}

fn edge(
    parent: &str,
    child: &str,
    translation: Vector3,
    rotation: Rotation,
    stamp: Stamp,
) -> Transform {
    Transform::new(parent, child, translation, rotation, stamp).unwrap()
}

/// A count as `f64`, for scaling a bound by a number of operations.
fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap())
}

/// An unsigned nanosecond count as the nearest `f64`, through the decimal
/// parser, which rounds correctly: independent of the crate's conversion.
fn nearest_f64(nanos: u64) -> f64 {
    nanos.to_string().parse().unwrap()
}

fn length(v: Vector3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

fn vector_bits(v: Vector3) -> [u64; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

fn components(r: Rotation) -> [f64; 4] {
    [r.w(), r.x(), r.y(), r.z()]
}

fn rotation_bits(r: Rotation) -> [u64; 4] {
    components(r).map(f64::to_bits)
}

/// The documented recipe for applying a transform to a point.
fn apply(
    transform: &Transform,
    p: Vector3,
) -> Vector3 {
    let r = transform.rotation().rotate(p);
    let t = transform.translation();
    Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z)
}

/// The invariant of every observable rotation: finite, inside the band,
/// and a fixed point of its own constructor.
fn check_rotation(rotation: Rotation) -> Result<(), TestCaseError> {
    let [w, x, y, z] = components(rotation);
    prop_assert!(
        w.is_finite() && x.is_finite() && y.is_finite() && z.is_finite(),
        "non-finite rotation {rotation:?}"
    );
    let norm_squared = w * w + x * x + y * y + z * z;
    prop_assert!(
        (norm_squared - 1.0).abs() <= BAND,
        "squared norm {norm_squared} of {rotation:?} is outside the band"
    );
    match Rotation::from_wxyz(w, x, y, z) {
        Ok(rebuilt) => prop_assert_eq!(rotation_bits(rebuilt), rotation_bits(rotation)),
        Err(error) => prop_assert!(false, "{rotation:?} does not rebuild: {error:?}"),
    }
    Ok(())
}

fn check_geometry(
    translation: Vector3,
    rotation: Rotation,
) -> Result<(), TestCaseError> {
    prop_assert!(
        translation.x.is_finite() && translation.y.is_finite() && translation.z.is_finite(),
        "non-finite translation {translation:?} in a successful result"
    );
    check_rotation(rotation)
}

fn check_transform(transform: &Transform) -> Result<(), TestCaseError> {
    check_geometry(transform.translation(), transform.rotation())
}

/// Two geometries agree within `budget` relative to `magnitude` in
/// translation and within `budget` in rotation. `q` and `-q` are the same
/// rotation, so the rotation is compared up to that sign.
fn check_close(
    actual: (Vector3, Rotation),
    expected: (Vector3, Rotation),
    budget: f64,
    magnitude: f64,
) -> Result<(), TestCaseError> {
    let (a, b) = (actual.0, expected.0);
    let bound = budget * magnitude;
    prop_assert!(
        (a.x - b.x).abs() <= bound && (a.y - b.y).abs() <= bound && (a.z - b.z).abs() <= bound,
        "translation {a:?} is further than {bound:e} from {b:?}"
    );
    let (q, p) = (components(actual.1), components(expected.1));
    let apart = |sign: f64| {
        q.iter()
            .zip(p)
            .map(|(q, p)| (q - sign * p).abs())
            .fold(0.0, f64::max)
    };
    let distance = apart(1.0).min(apart(-1.0));
    prop_assert!(
        distance <= budget,
        "rotation {q:?} is {distance:e} from {p:?}, budget {budget:e}"
    );
    Ok(())
}

// ---- the in-test interpolation reference ----

/// The interpolation ratio from integer nanosecond differences.
fn reference_ratio(
    from: i64,
    at: i64,
    to: i64,
) -> f64 {
    nearest_f64(at.abs_diff(from)) / nearest_f64(to.abs_diff(from))
}

/// Linear interpolation, written as an offset from the start so that it
/// is a different expression from the crate's weighted sum.
fn reference_lerp(
    a: Vector3,
    b: Vector3,
    ratio: f64,
) -> Vector3 {
    Vector3::new(
        a.x + ratio * (b.x - a.x),
        a.y + ratio * (b.y - a.y),
        a.z + ratio * (b.z - a.z),
    )
}

/// Shortest-arc spherical interpolation with `std` trigonometry.
fn reference_slerp(
    a: Rotation,
    b: Rotation,
    ratio: f64,
) -> [f64; 4] {
    let from = components(a);
    let mut to = components(b);
    let mut dot = from[0] * to[0] + from[1] * to[1] + from[2] * to[2] + from[3] * to[3];
    if dot < 0.0 {
        to = to.map(|c| -c);
        dot = -dot;
    }
    let theta = dot.min(1.0).acos();
    let (keep, take) = if theta < 1e-7 {
        (1.0 - ratio, ratio)
    } else {
        (
            ((1.0 - ratio) * theta).sin() / theta.sin(),
            (ratio * theta).sin() / theta.sin(),
        )
    };
    let blended = [0, 1, 2, 3].map(|i| keep * from[i] + take * to[i]);
    let norm = blended.iter().map(|c| c * c).sum::<f64>().sqrt();
    blended.map(|c| c / norm)
}

/// The geometry a lookup must return between two stored samples, within
/// the stated tolerances.
fn check_interpolated(
    actual: &Transform,
    from: (i64, &Transform),
    to: (i64, &Transform),
    requested: i64,
) -> Result<(), TestCaseError> {
    let ratio = reference_ratio(from.0, requested, to.0);
    let (a, b) = (from.1.translation(), to.1.translation());
    let expected = reference_lerp(a, b, ratio);
    let got = actual.translation();
    // The crate's `(1 - r) * a + r * b` and the reference's
    // `a + r * (b - a)` each make three rounded operations on values no
    // larger than `|a| + |b|`: at most `1.5 * EPSILON * (|a| + |b|)`
    // each, `3 * EPSILON * (|a| + |b|)` apart. The smallest normal number
    // covers an underflowing product.
    for (got, expected, a, b) in [
        (got.x, expected.x, a.x, b.x),
        (got.y, expected.y, a.y, b.y),
        (got.z, expected.z, a.z, b.z),
    ] {
        let bound = 4.0 * f64::EPSILON * (a.abs() + b.abs()) + f64::MIN_POSITIVE;
        prop_assert!(
            (got - expected).abs() <= bound,
            "interpolated coordinate {got} is further than {bound:e} from {expected}"
        );
    }
    let expected = reference_slerp(from.1.rotation(), to.1.rotation(), ratio);
    for (got, expected) in components(actual.rotation()).into_iter().zip(expected) {
        prop_assert!(
            (got - expected).abs() <= SLERP_TOLERANCE,
            "interpolated rotation {:?} is not the reference {expected:?}",
            actual.rotation()
        );
    }
    check_transform(actual)
}

// ---- strategies ----

/// Translations of ordinary size.
fn translations() -> impl Strategy<Value = Vector3> {
    let axis = -100_000.0..100_000.0_f64;
    (axis.clone(), axis.clone(), axis).prop_map(|(x, y, z)| Vector3::new(x, y, z))
}

/// Translations whose length spans fifteen decades, so that a bound which
/// is right only at one scale fails.
fn wide_translations() -> impl Strategy<Value = Vector3> {
    let axis = -1.0..1.0_f64;
    (axis.clone(), axis.clone(), axis, -6_i32..=9).prop_map(|(x, y, z, exponent)| {
        let scale = 10.0_f64.powi(exponent);
        Vector3::new(x * scale, y * scale, z * scale)
    })
}

/// Coordinates at and near the limits of `f64`, mixed with ordinary ones.
fn extreme_coordinates() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(0.0),
        Just(f64::MAX),
        Just(-f64::MAX),
        Just(1.0e308),
        Just(-1.0e308),
        Just(f64::MAX / 2.0),
        Just(f64::MIN_POSITIVE),
        Just(5.0e-324),
        1.0e150..1.0e160_f64,
        -1.0e300..1.0e300_f64,
        -10.0..10.0_f64,
    ]
}

fn extreme_translations() -> impl Strategy<Value = Vector3> {
    (
        extreme_coordinates(),
        extreme_coordinates(),
        extreme_coordinates(),
    )
        .prop_map(|(x, y, z)| Vector3::new(x, y, z))
}

/// The components of a unit quaternion from a random axis and angle, as
/// accurate as `f64` arithmetic makes them.
fn unit_components() -> impl Strategy<Value = [f64; 4]> {
    let axis = -1.0..1.0_f64;
    (
        axis.clone(),
        axis.clone(),
        axis,
        0.0..core::f64::consts::TAU,
    )
        .prop_filter_map("axis too short to normalize", |(x, y, z, angle)| {
            let norm = (x * x + y * y + z * z).sqrt();
            if norm < 1e-3 {
                return None;
            }
            let half = angle / 2.0;
            let s = half.sin() / norm;
            Some([half.cos(), s * x, s * y, s * z])
        })
}

/// Rotations: mostly random, some built from components up to 9e-7 off
/// unit length (which the constructor rescales), plus the identity and
/// the half turns, where a component is exactly zero.
fn rotations() -> impl Strategy<Value = Rotation> {
    let built = |[w, x, y, z]: [f64; 4], factor: f64| {
        Rotation::from_wxyz(w * factor, x * factor, y * factor, z * factor).unwrap()
    };
    prop_oneof![
        6 => unit_components().prop_map(move |c| built(c, 1.0)),
        2 => (unit_components(), -9e-7..9e-7_f64).prop_map(move |(c, off)| built(c, 1.0 + off)),
        1 => Just(Rotation::IDENTITY),
        1 => prop_oneof![
            Just([0.0, 1.0, 0.0, 0.0]),
            Just([0.0, 0.0, 1.0, 0.0]),
            Just([0.0, 0.0, 0.0, 1.0]),
            Just([-1.0, 0.0, 0.0, 0.0]),
        ]
        .prop_map(move |c| built(c, 1.0)),
    ]
}

/// Nanosecond counts across the regimes that behave differently: around
/// zero on both sides, both sides of the 2^53 `f64` accuracy cliff, 2020s
/// wall-clock nanoseconds, and both ends of the `i64` range. `headroom`
/// keeps the end bands that far inside the range, for callers that move
/// the drawn value.
fn timestamp_nanos(headroom: i64) -> impl Strategy<Value = i64> {
    let top = i64::MAX - headroom;
    let bottom = i64::MIN + headroom;
    prop_oneof![
        -1_000_000_000_000_i64..1_000_000_000_000,
        ACCURACY_CLIFF_NANOS - NANOS_PER_SEC..ACCURACY_CLIFF_NANOS + NANOS_PER_SEC,
        -ACCURACY_CLIFF_NANOS - NANOS_PER_SEC..-ACCURACY_CLIFF_NANOS + NANOS_PER_SEC,
        1_700_000_000_000_000_000_i64..1_800_000_000_000_000_000,
        top - NANOS_PER_SEC..=top,
        bottom..=bottom + NANOS_PER_SEC,
    ]
}

/// Durations across the full range of the type, weighted toward the
/// sizes at which a sum with an `i64` instant changes from representable
/// to not: 2^63 and 2^64 nanoseconds.
fn durations() -> impl Strategy<Value = Duration> {
    let subsec = 0_u32..1_000_000_000;
    prop_oneof![
        (0_u64..1 << 40).prop_map(Duration::from_nanos),
        any::<u64>().prop_map(Duration::from_nanos),
        (9_223_372_030_u64..9_223_372_040, subsec.clone()).prop_map(|(s, n)| Duration::new(s, n)),
        (18_446_744_070_u64..18_446_744_080, subsec.clone()).prop_map(|(s, n)| Duration::new(s, n)),
        (any::<u64>(), subsec).prop_map(|(s, n)| Duration::new(s, n)),
        Just(Duration::ZERO),
        Just(Duration::MAX),
    ]
}

/// Instants for the arithmetic properties: the whole range, with the
/// neighbourhoods of zero and of both ends drawn often.
fn instants() -> impl Strategy<Value = i64> {
    prop_oneof![
        any::<i64>(),
        -1_000_i64..1_000,
        i64::MAX - 1_000..=i64::MAX,
        i64::MIN..=i64::MIN + 1_000,
        timestamp_nanos(0),
    ]
}

// ---- a random tree with known coverage ----

/// One edge of the random tree: frame `i + 1` hangs under frame
/// `i - pick % (i + 1)`, so a pick of zero continues a chain. `None` is a
/// static edge; `Some((start, span))` is a
/// dynamic one holding samples at `start` and `start + span`.
type TreeEdge = (usize, Option<(i64, i64)>);

fn tree_edges() -> impl Strategy<Value = Vec<TreeEdge>> {
    let edges = |start: i64, span: i64| {
        let coverage = prop::option::weighted(0.8, (0..start, 0..span));
        // Half the frames hang under the previous one: chains several
        // edges long are what the coverage rules are about.
        let pick = prop_oneof![Just(0_usize), 0_usize..100];
        prop::collection::vec((pick, coverage), 1..8)
    };
    // Mostly wide ranges, where edges overlap or miss each other by
    // chance; sometimes a whole tree over a handful of instants, where
    // several edges start or end together and the order among equals
    // decides which one an error names.
    prop_oneof![
        2 => edges(100, 100),
        1 => edges(3, 2),
    ]
}

/// What an edge of the tree can serve, as the oracle knows it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Coverage {
    Static,
    Empty,
    Range(i64, i64),
}

impl Coverage {
    fn serves(
        self,
        at: i64,
    ) -> bool {
        match self {
            Self::Static => true,
            Self::Empty => false,
            Self::Range(earliest, latest) => (earliest..=latest).contains(&at),
        }
    }
}

/// A registry over frames `f0..`, rooted at `f0`, with the oracle's view
/// of its topology and coverage.
struct Tree {
    registry: Registry,
    /// `parents[i]` is the parent of frame `i + 1`.
    parents: Vec<usize>,
    /// `coverage[i]` is the coverage of the edge above frame `i + 1`.
    coverage: Vec<Coverage>,
}

impl Tree {
    /// Builds the tree with every edge translating by `x`, then removes
    /// the samples at or before `cutoff`.
    fn build(
        edges: &[TreeEdge],
        cutoff: Option<i64>,
        x: f64,
    ) -> Self {
        let mut registry = Registry::unbounded();
        let mut parents = Vec::new();
        let mut coverage = Vec::new();
        for (i, (pick, range)) in edges.iter().enumerate() {
            let parent = i - pick % (i + 1);
            parents.push(parent);
            let stamps = match range {
                None => vec![Stamp::Static],
                Some((start, span)) => vec![at(*start), at(start + span)],
            };
            for stamp in stamps {
                let transform = edge(
                    &frame(parent),
                    &frame(i + 1),
                    Vector3::new(x, 0.0, 0.0),
                    Rotation::IDENTITY,
                    stamp,
                );
                registry.insert(&transform).unwrap();
            }
            coverage.push(match range {
                None => Coverage::Static,
                Some((start, span)) => {
                    let kept = |stamp: &i64| cutoff.is_none_or(|cutoff| *stamp > cutoff);
                    match (kept(start), kept(&(start + span))) {
                        (true, _) => Coverage::Range(*start, start + span),
                        (false, true) => Coverage::Range(start + span, start + span),
                        (false, false) => Coverage::Empty,
                    }
                }
            });
        }
        if let Some(cutoff) = cutoff {
            registry.remove_samples_through(Timestamp::from_nanos(cutoff));
        }
        Self {
            registry,
            parents,
            coverage,
        }
    }

    fn frames(&self) -> usize {
        self.parents.len() + 1
    }

    /// The frames from `from` up to, and excluding, the root. Each one
    /// stands for the edge above it.
    fn path(
        &self,
        mut from: usize,
    ) -> Vec<usize> {
        let mut path = Vec::new();
        while from != 0 {
            path.push(from);
            from = self.parents[from - 1];
        }
        path
    }

    /// The edges the chain between two frames crosses, in diagnosis
    /// order: the target's side from the bottom up, then the source's.
    /// The edges above the meeting point are on both paths and on neither
    /// side of the chain.
    fn chain(
        &self,
        target: usize,
        source: usize,
    ) -> Vec<(usize, Coverage)> {
        let (target_path, source_path) = (self.path(target), self.path(source));
        let target_side = target_path.iter().filter(|f| !source_path.contains(f));
        let source_side = source_path.iter().filter(|f| !target_path.contains(f));
        target_side
            .chain(source_side)
            .map(|&f| (f, self.coverage[f - 1]))
            .collect()
    }
}

/// Whether `error` is the coverage error for `frame` with that coverage.
fn is_coverage_error(
    error: &LookupError,
    frame: &str,
    coverage: Coverage,
    requested: i64,
) -> bool {
    match (error, coverage) {
        (LookupError::NoData { frame: named, .. }, Coverage::Empty) => named == frame,
        (
            LookupError::OutOfRange {
                frame: named,
                requested: echoed,
                earliest,
                latest,
                ..
            },
            Coverage::Range(start, end),
        ) => {
            named == frame
                && echoed.as_nanos() == requested
                && earliest.as_nanos() == start
                && latest.as_nanos() == end
        }
        _ => false,
    }
}

/// What `latest_common_time` must answer over a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Latest {
    Static,
    At(i64),
    NoData(usize),
    NoCommonTime {
        frame: usize,
        earliest: i64,
        latest: i64,
        latest_possible: i64,
    },
}

/// The oracle for `latest_common_time`: the first empty dynamic edge in
/// chain order refuses; otherwise the answer is the oldest of the newest
/// samples, provided the latest-starting range (the first such in chain
/// order) reaches back to it.
fn expected_latest(chain: &[(usize, Coverage)]) -> Latest {
    let mut latest_possible: Option<i64> = None;
    let mut latest_start: Option<(usize, i64, i64)> = None;
    for &(frame, coverage) in chain {
        match coverage {
            Coverage::Static => {}
            Coverage::Empty => return Latest::NoData(frame),
            Coverage::Range(earliest, latest) => {
                latest_possible = Some(latest_possible.map_or(latest, |old| old.min(latest)));
                if latest_start.is_none_or(|(_, start, _)| earliest > start) {
                    latest_start = Some((frame, earliest, latest));
                }
            }
        }
    }
    match (latest_possible, latest_start) {
        (Some(latest_possible), Some((frame, earliest, latest))) if earliest > latest_possible => {
            Latest::NoCommonTime {
                frame,
                earliest,
                latest,
                latest_possible,
            }
        }
        (Some(latest_possible), _) => Latest::At(latest_possible),
        (None, _) => Latest::Static,
    }
}

/// Reads a `latest_common_time` result in the oracle's terms. Frames are
/// `f<index>`; any other answer does not match an expectation.
fn observed_latest(result: &Result<Stamp, LookupError>) -> Option<Latest> {
    let index = |name: &str| name.strip_prefix('f')?.parse::<usize>().ok();
    match result {
        Ok(Stamp::Static) => Some(Latest::Static),
        Ok(Stamp::At(instant)) => Some(Latest::At(instant.as_nanos())),
        Err(LookupError::NoData { frame, .. }) => Some(Latest::NoData(index(frame)?)),
        Err(LookupError::NoCommonTime {
            frame,
            earliest,
            latest,
            latest_possible,
            ..
        }) => Some(Latest::NoCommonTime {
            frame: index(frame)?,
            earliest: earliest.as_nanos(),
            latest: latest.as_nanos(),
            latest_possible: latest_possible.as_nanos(),
        }),
        _ => None,
    }
}

// ---- what a caller can observe of a registry ----

type Seen = (
    Vec<Result<Stamp, LookupError>>,
    Vec<Result<Transform, LookupError>>,
);

/// Every ordered pair of `names` (each name with itself included, which
/// is the known-name probe): its newest common instant and its lookups at
/// `instants`.
fn observable(
    registry: &Registry,
    names: &[String],
    instants: &[i64],
) -> Seen {
    let mut latest = Vec::new();
    let mut lookups = Vec::new();
    for target in names {
        for source in names {
            latest.push(registry.latest_common_time(target, source));
            for &nanos in instants {
                lookups.push(registry.lookup(target, source, Timestamp::from_nanos(nanos)));
            }
        }
    }
    (latest, lookups)
}

fn is_unknown(
    registry: &Registry,
    name: &str,
) -> bool {
    matches!(
        registry.lookup(name, name, Timestamp::from_nanos(0)),
        Err(LookupError::UnknownFrame { frame, .. }) if frame == name
    )
}

// ---- the batch property's inserts ----

/// Frames `f0..f3` for the batch property: few enough that most members
/// of a batch land on a frame an earlier member touched.
const FEW: usize = 4;

/// One insert: (parent, child, static?, instant, x translation).
type Insert = (usize, usize, bool, i64, i32);

/// Nearly all samples for the fixed tree `f0 <- f1 <- f2 <- f3` (the last edge
/// static), so that structural rejections do not end most batches before
/// two samples for one frame meet; the rest are arbitrary edges, which
/// supply self-edges, second parents, kind conflicts and cycles.
fn inserts() -> impl Strategy<Value = Insert> {
    let on_tree = (1..FEW, 0_i64..8, -8_i32..8)
        .prop_map(|(child, nanos, x)| (child - 1, child, child == FEW - 1, nanos, x));
    let arbitrary = (
        0..FEW,
        0..FEW,
        prop::bool::weighted(0.25),
        0_i64..8,
        -8_i32..8,
    );
    prop_oneof![11 => on_tree, 1 => arbitrary]
}

fn build((parent, child, fixed, nanos, x): Insert) -> Transform {
    let stamp = if fixed { Stamp::Static } else { at(nanos) };
    edge(
        &frame(parent),
        &frame(child),
        Vector3::new(f64::from(x), 0.0, 0.0),
        Rotation::IDENTITY,
        stamp,
    )
}

// ---- the known-names model ----

/// The names the model-based property draws from: `f0..f5`.
const POOL: usize = 6;

/// One edge for the model-based property: (parent, child, static?,
/// instant) over the pool.
type Link = (usize, usize, bool, i64);

#[derive(Debug, Clone)]
enum Op {
    Insert(Link),
    InsertAll(Vec<Link>),
    Reparent(Link),
    /// An index into the probe names, so that the unknown and the empty
    /// name are removed too.
    Remove(usize),
    Drain(i64),
}

fn links() -> impl Strategy<Value = Link> {
    (0..POOL, 0..POOL, prop::bool::weighted(0.3), 0_i64..8)
}

fn ops() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => links().prop_map(Op::Insert),
        2 => prop::collection::vec(links(), 0..5).prop_map(Op::InsertAll),
        3 => links().prop_map(Op::Reparent),
        3 => (0..POOL + 2).prop_map(Op::Remove),
        1 => (0_i64..8).prop_map(Op::Drain),
    ]
}

/// The pool, a name no operation ever uses, and the empty name.
fn probe_names() -> Vec<String> {
    let mut names: Vec<String> = (0..POOL).map(frame).collect();
    names.extend(["ghost".into(), String::new()]);
    names
}

fn link_transform((parent, child, fixed, nanos): Link) -> Transform {
    let stamp = if fixed { Stamp::Static } else { at(nanos) };
    edge(
        &frame(parent),
        &frame(child),
        Vector3::new(1.0, 0.0, 0.0),
        Rotation::IDENTITY,
        stamp,
    )
}

/// The model: each child's pinned parent and whether its edge is static.
/// Everything the known-names rule and the structural insert checks
/// depend on, and nothing else.
type Model = BTreeMap<usize, (usize, bool)>;

/// Why the model refuses an edge.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Refusal {
    SelfEdge,
    ParentMismatch { pinned: usize },
    KindMismatch,
    Cycle,
    UnknownFrame,
    NoParent,
    ParentUnchanged,
}

fn model_knows(
    model: &Model,
    name: usize,
) -> bool {
    model.contains_key(&name) || model.values().any(|&(parent, _)| parent == name)
}

fn model_root(
    model: &Model,
    mut name: usize,
) -> usize {
    while let Some(&(parent, _)) = model.get(&name) {
        name = parent;
    }
    name
}

/// Whether `ancestor` is `from` or above it.
fn model_reaches(
    model: &Model,
    mut from: usize,
    ancestor: usize,
) -> bool {
    loop {
        if from == ancestor {
            return true;
        }
        match model.get(&from) {
            Some(&(parent, _)) => from = parent,
            None => return false,
        }
    }
}

fn model_insert(
    model: &mut Model,
    (parent, child, fixed, _): Link,
) -> Result<(), Refusal> {
    if parent == child {
        return Err(Refusal::SelfEdge);
    }
    match model.get(&child) {
        Some(&(pinned, _)) if pinned != parent => Err(Refusal::ParentMismatch { pinned }),
        Some(&(_, is_static)) if is_static != fixed => Err(Refusal::KindMismatch),
        Some(_) => Ok(()),
        None if model_reaches(model, parent, child) => Err(Refusal::Cycle),
        None => {
            model.insert(child, (parent, fixed));
            Ok(())
        }
    }
}

fn model_reparent(
    model: &mut Model,
    (parent, child, fixed, _): Link,
) -> Result<(), Refusal> {
    if !model_knows(model, child) {
        return Err(Refusal::UnknownFrame);
    }
    let Some(&(pinned, is_static)) = model.get(&child) else {
        return Err(Refusal::NoParent);
    };
    if pinned == parent {
        return Err(Refusal::ParentUnchanged);
    }
    if parent == child {
        return Err(Refusal::SelfEdge);
    }
    if model_reaches(model, parent, child) {
        return Err(Refusal::Cycle);
    }
    if is_static != fixed {
        return Err(Refusal::KindMismatch);
    }
    model.insert(child, (parent, fixed));
    Ok(())
}

/// Whether `error` is the model's `refusal` of `link`, payload included.
fn is_refusal(
    error: &InsertError,
    refusal: Refusal,
    link: Link,
) -> bool {
    let (parent, child) = (frame(link.0), frame(link.1));
    let stamp = link_transform(link).stamp();
    match (error, refusal) {
        (InsertError::SelfEdge { frame, .. }, Refusal::SelfEdge)
        | (InsertError::UnknownFrame { frame, .. }, Refusal::UnknownFrame)
        | (InsertError::NoParent { frame, .. }, Refusal::NoParent) => *frame == child,
        (
            InsertError::ParentMismatch {
                frame: named,
                pinned,
                offered,
                ..
            },
            Refusal::ParentMismatch { pinned: expected },
        ) => *named == child && *pinned == frame(expected) && *offered == parent,
        (
            InsertError::KindMismatch {
                frame, incoming, ..
            },
            Refusal::KindMismatch,
        ) => *frame == child && *incoming == stamp,
        (
            InsertError::Cycle {
                child: named_child,
                parent: named_parent,
                ..
            },
            Refusal::Cycle,
        ) => *named_child == child && *named_parent == parent,
        (
            InsertError::ParentUnchanged {
                frame,
                parent: named_parent,
                ..
            },
            Refusal::ParentUnchanged,
        ) => *frame == child && *named_parent == parent,
        _ => false,
    }
}

/// What the known-names rule and the topology decide about a query over
/// `names[..]` indices, before any coverage is considered.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Unknown(usize),
    Disconnected(usize, usize),
    /// Names known and connected: the answer is a value or a coverage
    /// error, which the model does not predict.
    Resolved,
}

fn observed_verdict<V>(
    result: &Result<V, LookupError>,
    names: &[String],
) -> Option<Verdict> {
    let index = |name: &String| names.iter().position(|n| n == name);
    match result {
        Err(LookupError::UnknownFrame { frame, .. }) => Some(Verdict::Unknown(index(frame)?)),
        Err(LookupError::Disconnected { a, b, .. }) => {
            Some(Verdict::Disconnected(index(a)?, index(b)?))
        }
        _ => Some(Verdict::Resolved),
    }
}

/// Checks every query over the probe names against the model: which
/// names are known, in which order unknown ones are reported, and which
/// pairs are connected.
fn check_against_model(
    registry: &Registry,
    model: &Model,
) -> Result<(), TestCaseError> {
    let names = probe_names();
    let known = |i: usize| i < POOL && model_knows(model, i);
    let connected = |a: usize, b: usize| model_root(model, a) == model_root(model, b);
    let instant = Timestamp::from_nanos(4);
    for target in 0..names.len() {
        prop_assert_eq!(is_unknown(registry, &names[target]), !known(target));
        for source in 0..names.len() {
            let expected = if !known(target) {
                Verdict::Unknown(target)
            } else if !known(source) {
                Verdict::Unknown(source)
            } else if !connected(target, source) {
                Verdict::Disconnected(target, source)
            } else {
                Verdict::Resolved
            };
            let lookup = registry.lookup(&names[target], &names[source], instant);
            prop_assert_eq!(
                observed_verdict(&lookup, &names),
                Some(expected),
                "lookup({}, {}) answered {:?}",
                target,
                source,
                lookup
            );
            let latest = registry.latest_common_time(&names[target], &names[source]);
            prop_assert_eq!(
                observed_verdict(&latest, &names),
                Some(expected),
                "latest_common_time({}, {}) answered {:?}",
                target,
                source,
                latest
            );

            // Names are reported target, source, fixed; then the target
            // leg's connection to the fixed frame, then the source leg's.
            let fixed = (target + 2 * source + 1) % names.len();
            let expected = if !known(target) {
                Verdict::Unknown(target)
            } else if !known(source) {
                Verdict::Unknown(source)
            } else if !known(fixed) {
                Verdict::Unknown(fixed)
            } else if !connected(target, fixed) {
                Verdict::Disconnected(target, fixed)
            } else if !connected(source, fixed) {
                Verdict::Disconnected(source, fixed)
            } else {
                Verdict::Resolved
            };
            let across = registry.lookup_across(
                &names[target],
                instant,
                &names[source],
                instant,
                &names[fixed],
            );
            prop_assert_eq!(
                observed_verdict(&across, &names),
                Some(expected),
                "lookup_across({}, {}, fixed {}) answered {:?}",
                target,
                source,
                fixed,
                across
            );
        }
    }
    Ok(())
}

// ---- geometry ----

proptest! {
    #![proptest_config(config(1024))]

    #[test]
    fn from_wxyz_keeps_in_band_components_and_rescales_the_rest(
        unit in unit_components(),
        off in -9e-7..9e-7_f64,
        exact in any::<bool>(),
    ) {
        let factor = if exact { 1.0 } else { 1.0 + off };
        let [w, x, y, z] = unit.map(|c| c * factor);
        let rotation = Rotation::from_wxyz(w, x, y, z).unwrap();
        check_rotation(rotation)?;

        let norm_squared = w * w + x * x + y * y + z * z;
        if (norm_squared - 1.0).abs() <= BAND {
            // Inside the band the given components are the stored ones.
            prop_assert_eq!(rotation_bits(rotation), [w, x, y, z].map(f64::to_bits));
        } else {
            // Outside it they are divided by their norm: one division and
            // one product per component, a few ulps from the quotient.
            let norm = norm_squared.sqrt();
            for (stored, given) in components(rotation).into_iter().zip([w, x, y, z]) {
                prop_assert!(
                    (stored - given / norm).abs() <= 4.0 * f64::EPSILON,
                    "{stored} is not {given} over {norm}"
                );
            }
        }
    }

    #[test]
    fn from_wxyz_rejects_norms_beyond_tolerance(
        unit in unit_components(),
        deviation in 2e-6..1e-3_f64,
        above in any::<bool>(),
    ) {
        let factor = if above { 1.0 + deviation } else { 1.0 - deviation };
        let [w, x, y, z] = unit.map(|c| c * factor);
        let result = Rotation::from_wxyz(w, x, y, z);

        prop_assert!(
            matches!(
                result,
                Err(TransformError::NonUnitRotation { norm, .. }) if (norm - factor).abs() < 1e-12
            ),
            "norm deviation {deviation} must be rejected with the norm, got {result:?}",
        );
    }

    #[test]
    fn from_wxyz_accepts_norms_within_tolerance(
        unit in unit_components(),
        deviation in -5e-7..5e-7_f64,
    ) {
        let [w, x, y, z] = unit.map(|c| c * (1.0 + deviation));
        let result = Rotation::from_wxyz(w, x, y, z);

        prop_assert!(
            result.is_ok(),
            "norm deviation {deviation} must be accepted, got {result:?}",
        );
    }

    #[test]
    fn from_wxyz_never_accepts_non_finite_or_oversized_components(
        raw in prop::array::uniform4(prop_oneof![
            Just(f64::NAN),
            Just(f64::INFINITY),
            Just(f64::NEG_INFINITY),
            Just(f64::MAX),
            Just(1.0e200),
            Just(0.0),
            -2.0..2.0_f64,
        ]),
    ) {
        let [w, x, y, z] = raw;
        let result = Rotation::from_wxyz(w, x, y, z);
        if raw.iter().any(|c| !c.is_finite()) {
            prop_assert_eq!(result, Err(TransformError::NonFinite));
        } else {
            match result {
                Ok(rotation) => {
                    check_rotation(rotation)?;
                    let norm = (w * w + x * x + y * y + z * z).sqrt();
                    prop_assert!((norm - 1.0).abs() <= 1.0e-6 + f64::EPSILON);
                }
                // A squared norm that overflows reports an infinite norm:
                // finite components are never `NonFinite`.
                Err(error) => prop_assert!(
                    matches!(error, TransformError::NonUnitRotation { .. }),
                    "finite components were refused as {error:?}"
                ),
            }
        }
    }

    #[test]
    fn inverse_twice_returns_the_original(
        translation in wide_translations(),
        rotation in rotations(),
        nanos in timestamp_nanos(0),
    ) {
        let original = edge("a", "b", translation, rotation, at(nanos));

        let inverse = original.inverse().unwrap();
        prop_assert_eq!(inverse.target(), "b");
        prop_assert_eq!(inverse.source(), "a");
        prop_assert_eq!(inverse.stamp(), at(nanos));
        check_transform(&inverse)?;

        let roundtrip = inverse.inverse().unwrap();
        prop_assert_eq!(roundtrip.target(), "a");
        prop_assert_eq!(roundtrip.source(), "b");
        prop_assert_eq!(roundtrip.stamp(), at(nanos));
        check_transform(&roundtrip)?;
        // Conjugation only flips signs, so the rotation returns bit for bit.
        prop_assert_eq!(rotation_bits(roundtrip.rotation()), rotation_bits(rotation));
        // Two operations on a vector of the translation's length.
        check_close(
            (roundtrip.translation(), roundtrip.rotation()),
            (translation, rotation),
            2.0 * ROUNDING,
            length(translation),
        )?;
    }

    #[test]
    fn a_transform_and_its_inverse_return_a_point_to_its_origin(
        translation in wide_translations(),
        rotation in rotations(),
        position in wide_translations(),
    ) {
        let transform = edge("a", "b", translation, rotation, Stamp::Static);

        let moved = apply(&transform, position);
        let back = apply(&transform.inverse().unwrap(), moved);

        // Three operations (apply, invert, apply) on vectors no longer
        // than the point's distance plus the translation's length.
        let bound = 3.0 * ROUNDING * (length(position) + length(translation));
        prop_assert!(
            (back.x - position.x).abs() <= bound
                && (back.y - position.y).abs() <= bound
                && (back.z - position.z).abs() <= bound,
            "position drifted further than {bound:e}: {back:?} vs {position:?}",
        );
    }

    #[test]
    fn composing_with_the_inverse_gives_the_identity(
        translation in wide_translations(),
        rotation in rotations(),
        static_side in prop::option::of(any::<bool>()),
        nanos in timestamp_nanos(0),
    ) {
        // One operand static, or neither: a static operand adopts the
        // other's stamp.
        let (lhs_stamp, rhs_stamp) = match static_side {
            Some(true) => (Stamp::Static, at(nanos)),
            Some(false) => (at(nanos), Stamp::Static),
            None => (at(nanos), at(nanos)),
        };
        let forward = edge("a", "b", translation, rotation, lhs_stamp);
        let backward = edge("b", "a", translation, rotation, rhs_stamp).inverse().unwrap();
        prop_assert_eq!(backward.target(), "a");

        // `a <- b` composed with its own inverse relabelled `b <- a`.
        let backward = edge("b", "a", backward.translation(), backward.rotation(), rhs_stamp);
        let composed = forward.compose(&backward).unwrap();

        prop_assert_eq!(composed.target(), "a");
        prop_assert_eq!(composed.source(), "a");
        prop_assert_eq!(composed.stamp(), at(nanos));
        check_transform(&composed)?;
        // Two operations: the inversion and the composition.
        check_close(
            (composed.translation(), composed.rotation()),
            (Vector3::new(0.0, 0.0, 0.0), Rotation::IDENTITY),
            2.0 * ROUNDING,
            length(translation),
        )?;
    }

    #[test]
    fn composition_keeps_the_rotation_invariant_over_long_products(
        links in prop::collection::vec((translations(), rotations()), 2..40),
    ) {
        let mut composed = edge("f0", "f1", links[0].0, links[0].1, Stamp::Static);
        for (i, (translation, rotation)) in links.iter().enumerate().skip(1) {
            let next = edge(&frame(i), &frame(i + 1), *translation, *rotation, Stamp::Static);
            composed = composed.compose(&next).unwrap();
            prop_assert_eq!(composed.target(), "f0");
            prop_assert_eq!(composed.source(), frame(i + 1));
            check_transform(&composed)?;
            check_transform(&composed.inverse().unwrap())?;
        }
    }

    #[test]
    fn no_operation_returns_a_non_finite_translation(
        first in extreme_translations(),
        second in extreme_translations(),
        third in extreme_translations(),
        rotation_ab in rotations(),
        rotation_bc in rotations(),
    ) {
        // Finite components always build, whatever their size.
        let a_b = edge("a", "b", first, rotation_ab, at(0));
        let a_b_later = edge("a", "b", second, rotation_bc, at(10));
        let b_c = edge("b", "c", third, rotation_bc, Stamp::Static);

        let mut derived = Vec::new();
        for result in [
            a_b.inverse(),
            b_c.inverse(),
            a_b.compose(&b_c),
            a_b.compose(&b_c).and_then(|a_c| a_c.inverse()),
            a_b.inverse().and_then(|b_a| b_a.compose(&a_b)),
            b_c.inverse().and_then(|c_b| a_b.inverse().and_then(|b_a| c_b.compose(&b_a))),
        ] {
            match result {
                Ok(transform) => {
                    check_transform(&transform)?;
                    derived.push(transform);
                }
                Err(error) => prop_assert_eq!(error, TransformError::NonFinite),
            }
        }
        // Derived values are ordinary operands.
        for transform in &derived {
            match transform.inverse() {
                Ok(inverse) => check_transform(&inverse)?,
                Err(error) => prop_assert_eq!(error, TransformError::NonFinite),
            }
        }

        let mut registry = Registry::unbounded();
        registry.insert_all(&[a_b, a_b_later, b_c]).unwrap();
        for (target, source) in [("a", "b"), ("b", "a"), ("a", "c"), ("c", "a"), ("c", "b")] {
            // Stored instants and interpolated ones.
            for nanos in [0, 3, 5, 10] {
                match registry.lookup(target, source, Timestamp::from_nanos(nanos)) {
                    Ok(transform) => check_transform(&transform)?,
                    Err(error) => prop_assert_eq!(error, LookupError::NonFinite),
                }
            }
            for fixed in ["a", "b", "c"] {
                let across = registry.lookup_across(
                    target,
                    Timestamp::from_nanos(2),
                    source,
                    Timestamp::from_nanos(9),
                    fixed,
                );
                match across {
                    Ok(across) => check_geometry(across.translation(), across.rotation())?,
                    Err(error) => prop_assert_eq!(error, LookupError::NonFinite),
                }
            }
        }
    }

    #[test]
    fn a_non_finite_translation_never_builds(
        coordinates in prop::array::uniform3(prop_oneof![
            Just(f64::NAN),
            Just(f64::INFINITY),
            Just(f64::NEG_INFINITY),
            extreme_coordinates(),
        ]),
        rotation in rotations(),
    ) {
        let [x, y, z] = coordinates;
        let result = Transform::new("a", "b", Vector3::new(x, y, z), rotation, Stamp::Static);
        if coordinates.iter().all(|c| c.is_finite()) {
            let transform = result.unwrap();
            prop_assert_eq!(vector_bits(transform.translation()), coordinates.map(f64::to_bits));
            prop_assert_eq!(rotation_bits(transform.rotation()), rotation_bits(rotation));
        } else {
            prop_assert_eq!(result, Err(TransformError::NonFinite));
        }
    }
}

// ---- time ----

proptest! {
    #![proptest_config(config(4096))]

    #[test]
    fn checked_add_and_sub_match_wide_integer_arithmetic(
        nanos in instants(),
        duration in durations(),
    ) {
        let instant = Timestamp::from_nanos(nanos);
        // A `Duration` holds fewer than 2^95 nanoseconds.
        let span = i128::try_from(duration.as_nanos()).unwrap();

        let sum = i64::try_from(i128::from(nanos) + span).ok();
        prop_assert_eq!(instant.checked_add(duration).map(Timestamp::as_nanos), sum);
        let difference = i64::try_from(i128::from(nanos) - span).ok();
        prop_assert_eq!(instant.checked_sub(duration).map(Timestamp::as_nanos), difference);

        if let Some(later) = instant.checked_add(duration) {
            prop_assert_eq!(later.checked_sub(duration), Some(instant));
            prop_assert_eq!(later.checked_duration_since(instant), Some(duration));
        }
        if let Some(earlier) = instant.checked_sub(duration) {
            prop_assert_eq!(earlier.checked_add(duration), Some(instant));
            prop_assert_eq!(instant.checked_duration_since(earlier), Some(duration));
        }
    }

    #[test]
    fn checked_duration_since_matches_wide_integer_arithmetic(
        later in instants(),
        earlier in instants(),
    ) {
        let difference = i128::from(later) - i128::from(earlier);
        let expected = (difference >= 0).then(|| {
            let per_sec = i128::from(NANOS_PER_SEC);
            Duration::new(
                u64::try_from(difference / per_sec).unwrap(),
                u32::try_from(difference % per_sec).unwrap(),
            )
        });
        let (later, earlier) = (Timestamp::from_nanos(later), Timestamp::from_nanos(earlier));

        prop_assert_eq!(later.checked_duration_since(earlier), expected);
        // `None` exactly when the "earlier" instant is the later one.
        prop_assert_eq!(later.checked_duration_since(earlier).is_none(), earlier > later);
        prop_assert_eq!(later.cmp(&earlier), later.as_nanos().cmp(&earlier.as_nanos()));
    }
}

// ---- single edges: storage, interpolation, retention ----

proptest! {
    #![proptest_config(config(512))]

    #[test]
    fn an_edge_is_exact_at_its_samples_interpolates_between_and_refuses_outside(
        translation_from in translations(),
        translation_to in translations(),
        rotation_from in rotations(),
        // Sometimes the same rotation, or its negation: the nearly
        // parallel branch and the shortest-arc flip.
        rotation_to in prop_oneof![
            4 => rotations().prop_map(Some),
            1 => Just(None),
        ],
        negate in prop::bool::weighted(0.2),
        // `span` and `outside` are each at most about a second, so two
        // seconds of headroom keep every probe inside the range.
        start in timestamp_nanos(2 * NANOS_PER_SEC),
        span in prop_oneof![1_i64..1_000, 1_i64..NANOS_PER_SEC],
        inside in 0.0..1.0_f64,
        outside in 1_i64..NANOS_PER_SEC,
    ) {
        let rotation_to = rotation_to.unwrap_or(rotation_from);
        let rotation_to = if negate {
            let [w, x, y, z] = components(rotation_to);
            Rotation::from_wxyz(-w, -x, -y, -z).unwrap()
        } else {
            rotation_to
        };
        let end = start + span;
        let from = edge("a", "b", translation_from, rotation_from, at(start));
        let to = edge("a", "b", translation_to, rotation_to, at(end));
        let mut registry = Registry::unbounded();
        registry.insert(&to).unwrap();
        registry.insert(&from).unwrap();

        // Toward the parent at a stored instant: the stored transform,
        // bit for bit, labels and stamp included.
        for stored in [&from, &to] {
            let Stamp::At(instant) = stored.stamp() else {
                unreachable!("both samples are dynamic");
            };
            let found = registry.lookup("a", "b", instant).unwrap();
            prop_assert_eq!(&found, stored);
            prop_assert_eq!(vector_bits(found.translation()), vector_bits(stored.translation()));
            prop_assert_eq!(rotation_bits(found.rotation()), rotation_bits(stored.rotation()));
        }

        // Strictly between the samples. `span * inside` is exact below
        // 2^53 and floors to an offset in `0..span`.
        let offset = (nearest_f64(span.unsigned_abs()) * inside).floor().to_string();
        let offset: i64 = offset.parse().unwrap();
        let requested = start + offset.clamp(1, (span - 1).max(1));
        if requested < end {
            let found = registry.lookup("a", "b", Timestamp::from_nanos(requested)).unwrap();
            prop_assert_eq!(found.target(), "a");
            prop_assert_eq!(found.source(), "b");
            prop_assert_eq!(found.stamp(), at(requested));
            check_interpolated(&found, (start, &from), (end, &to), requested)?;
        }

        // Strictly outside the covered range, on both sides: no
        // extrapolation, and the error carries the range.
        for requested in [start - outside, end + outside] {
            let result = registry.lookup("a", "b", Timestamp::from_nanos(requested));
            prop_assert!(
                matches!(
                    &result,
                    Err(error) if is_coverage_error(error, "b", Coverage::Range(start, end), requested)
                ),
                "expected OutOfRange at {requested}, got {result:?}",
            );
        }
    }

    #[test]
    fn a_static_edge_answers_every_instant_with_its_stored_bits(
        translation in translations(),
        rotation in rotations(),
        replacement in translations(),
        nanos in timestamp_nanos(0),
    ) {
        let mut registry = Registry::unbounded();
        let first = edge("a", "b", translation, rotation, Stamp::Static);
        registry.insert(&first).unwrap();
        let instant = Timestamp::from_nanos(nanos);

        let found = registry.lookup("a", "b", instant).unwrap();
        prop_assert_eq!(found.stamp(), Stamp::At(instant));
        prop_assert_eq!(vector_bits(found.translation()), vector_bits(translation));
        prop_assert_eq!(rotation_bits(found.rotation()), rotation_bits(rotation));
        prop_assert_eq!(registry.latest_common_time("a", "b"), Ok(Stamp::Static));

        // A static update replaces the pose at every instant.
        registry.insert(&edge("a", "b", replacement, rotation, Stamp::Static)).unwrap();
        let found = registry.lookup("a", "b", instant).unwrap();
        prop_assert_eq!(vector_bits(found.translation()), vector_bits(replacement));
    }

    #[test]
    fn dynamic_history_matches_a_reference_map_after_upserts_expiry_and_cleanup(
        max_age in prop::option::of(0_i64..64),
        operations in prop::collection::vec((0_u8..5, 0_i64..128, -100_i32..100), 1..100),
    ) {
        let mut registry = max_age.map_or_else(Registry::unbounded, |age| {
            Registry::with_max_age(Duration::from_nanos(age.unsigned_abs()))
        });
        let quarter_turn = Rotation::from_wxyz(
            core::f64::consts::FRAC_1_SQRT_2,
            0.0,
            0.0,
            core::f64::consts::FRAC_1_SQRT_2,
        )
        .unwrap();
        let mut reference: BTreeMap<i64, Transform> = BTreeMap::new();
        let mut exists = false;
        for (operation, nanos, coordinate) in operations {
            if operation == 0 {
                // The cutoff is inclusive.
                registry.remove_samples_through(Timestamp::from_nanos(nanos));
                reference.retain(|stamp, _| *stamp > nanos);
            } else {
                let rotation = if coordinate % 2 == 0 { Rotation::IDENTITY } else { quarter_turn };
                let transform = edge(
                    "parent",
                    "child",
                    Vector3::new(f64::from(coordinate), -0.0, 0.25),
                    rotation,
                    at(nanos),
                );
                // The newest stored sample decides; a drained history has
                // none, and the boundary sample is accepted.
                let newest = reference.last_key_value().map(|(&stamp, _)| stamp);
                let expired = newest
                    .zip(max_age)
                    .is_some_and(|(newest, age)| nanos < newest - age);
                let result = registry.insert(&transform);
                if expired {
                    prop_assert!(
                        matches!(
                            &result,
                            Err(InsertError::Expired { frame, stamp, newest: reported, .. })
                                if frame == "child"
                                    && stamp.as_nanos() == nanos
                                    && Some(reported.as_nanos()) == newest
                        ),
                        "expected Expired against {newest:?}, got {result:?}",
                    );
                } else {
                    prop_assert_eq!(result, Ok(()));
                    exists = true;
                    reference.insert(nanos, transform);
                    let newest = newest.map_or(nanos, |old| old.max(nanos));
                    if let Some(age) = max_age {
                        reference.retain(|stamp, _| *stamp >= newest - age);
                    }
                }
            }

            let covered = reference
                .first_key_value()
                .zip(reference.last_key_value())
                .map(|((&earliest, _), (&latest, _))| (earliest, latest));
            let latest = registry.latest_common_time("parent", "child");
            match covered {
                Some((_, newest)) => prop_assert_eq!(latest, Ok(at(newest))),
                None if exists => prop_assert!(
                    matches!(&latest, Err(LookupError::NoData { frame, .. }) if frame == "child"),
                    "expected NoData, got {latest:?}",
                ),
                None => prop_assert!(
                    matches!(&latest, Err(LookupError::UnknownFrame { frame, .. }) if frame == "parent"),
                    "expected UnknownFrame, got {latest:?}",
                ),
            }

            for requested in [0, nanos, 32, 64, 96, 127] {
                let actual = registry.lookup("parent", "child", Timestamp::from_nanos(requested));
                let before = reference.range(..=requested).next_back();
                let after = reference.range(requested..).next();
                match (before, after) {
                    (Some((&stored, transform)), _) | (_, Some((&stored, transform)))
                        if stored == requested =>
                    {
                        let found = actual.unwrap();
                        prop_assert_eq!(&found, transform);
                        prop_assert_eq!(
                            vector_bits(found.translation()),
                            vector_bits(transform.translation())
                        );
                    }
                    (Some((&from, start)), Some((&to, end))) => {
                        let found = actual.unwrap();
                        prop_assert_eq!(found.stamp(), at(requested));
                        check_interpolated(&found, (from, start), (to, end), requested)?;
                    }
                    _ if exists => {
                        let coverage = covered.map_or(Coverage::Empty, |(earliest, latest)| {
                            Coverage::Range(earliest, latest)
                        });
                        prop_assert!(
                            matches!(
                                &actual,
                                Err(error) if is_coverage_error(error, "child", coverage, requested)
                            ),
                            "expected the coverage error for {coverage:?}, got {actual:?}",
                        );
                    }
                    _ => prop_assert!(
                        matches!(
                            &actual,
                            Err(LookupError::UnknownFrame { frame, .. }) if frame == "parent"
                        ),
                        "expected UnknownFrame, got {actual:?}",
                    ),
                }
            }
        }
    }
}

// ---- lookups over trees ----

proptest! {
    #![proptest_config(config(768))]

    #[test]
    fn a_lookup_there_and_back_composes_to_the_identity(
        // Frame i + 1 hangs under frame pick % (i + 1); some edges are
        // static, the rest hold one sample at the shared instant.
        edges in prop::collection::vec(
            (0_usize..100, wide_translations(), rotations(), prop::bool::weighted(0.25)),
            1..7,
        ),
        picks in (0_usize..100, 0_usize..100),
        nanos in timestamp_nanos(0),
    ) {
        let mut registry = Registry::unbounded();
        let mut magnitude = 0.0;
        for (i, (pick, translation, rotation, fixed)) in edges.iter().enumerate() {
            let stamp = if *fixed { Stamp::Static } else { at(nanos) };
            let stored = edge(&frame(pick % (i + 1)), &frame(i + 1), *translation, *rotation, stamp);
            registry.insert(&stored).unwrap();
            magnitude += length(*translation);

            // A single hop toward the parent returns what was inserted.
            let found = registry
                .lookup(stored.target(), stored.source(), Timestamp::from_nanos(nanos))
                .unwrap();
            prop_assert_eq!(found.stamp(), at(nanos));
            prop_assert_eq!(vector_bits(found.translation()), vector_bits(*translation));
            prop_assert_eq!(rotation_bits(found.rotation()), rotation_bits(*rotation));
        }
        let frames = edges.len() + 1;
        let (target, source) = (frame(picks.0 % frames), frame(picks.1 % frames));
        let instant = Timestamp::from_nanos(nanos);

        let forward = registry.lookup(&target, &source, instant).unwrap();
        prop_assert_eq!(forward.target(), target.as_str());
        prop_assert_eq!(forward.source(), source.as_str());
        prop_assert_eq!(forward.stamp(), Stamp::At(instant));
        check_transform(&forward)?;

        let backward = registry.lookup(&source, &target, instant).unwrap();
        prop_assert_eq!(backward.target(), source.as_str());
        prop_assert_eq!(backward.source(), target.as_str());
        prop_assert_eq!(backward.stamp(), Stamp::At(instant));
        check_transform(&backward)?;

        // The reverse lookup is the inverse. Each lookup composes at most
        // one hop per edge and inverts once, and the closing composition
        // is one more: at most `2 * edges + 3` operations. No
        // intermediate is longer than the sum of the edge lengths, which
        // is the magnitude the bound scales with. A tree of zero-length
        // edges has a zero bound, and its answer is exactly zero.
        let budget = count(2 * edges.len() + 3) * ROUNDING;
        let composed = forward.compose(&backward).unwrap();
        check_close(
            (composed.translation(), composed.rotation()),
            (Vector3::new(0.0, 0.0, 0.0), Rotation::IDENTITY),
            budget,
            magnitude,
        )?;
        let inverse = forward.inverse().unwrap();
        check_close(
            (backward.translation(), backward.rotation()),
            (inverse.translation(), inverse.rotation()),
            budget,
            magnitude,
        )?;

        if target == source {
            prop_assert_eq!(vector_bits(forward.translation()), [0.0_f64.to_bits(); 3]);
            prop_assert_eq!(forward.rotation(), Rotation::IDENTITY);
        }
    }

    #[test]
    fn a_cross_time_lookup_agrees_with_the_same_time_lookups_it_reduces_to(
        // Each dynamic edge holds two samples, at `start` and at
        // `start + span`, so every instant in between interpolates.
        edges in prop::collection::vec(
            (
                0_usize..100,
                (translations(), rotations()),
                (translations(), rotations()),
                prop::bool::weighted(0.25),
            ),
            1..6,
        ),
        picks in (0_usize..100, 0_usize..100, 0_usize..100),
        start in timestamp_nanos(2 * NANOS_PER_SEC),
        span in 1_i64..NANOS_PER_SEC,
        offsets in (0.0..=1.0_f64, 0.0..=1.0_f64),
    ) {
        let mut registry = Registry::unbounded();
        let mut magnitude = 0.0;
        for (i, (pick, first, second, fixed)) in edges.iter().enumerate() {
            let (parent, child) = (frame(pick % (i + 1)), frame(i + 1));
            if *fixed {
                registry.insert(&edge(&parent, &child, first.0, first.1, Stamp::Static)).unwrap();
            } else {
                registry.insert(&edge(&parent, &child, first.0, first.1, at(start))).unwrap();
                registry.insert(&edge(&parent, &child, second.0, second.1, at(start + span))).unwrap();
            }
            magnitude += length(first.0).max(length(second.0));
        }
        let frames = edges.len() + 1;
        let (target, source, fixed) = (
            frame(picks.0 % frames),
            frame(picks.1 % frames),
            frame(picks.2 % frames),
        );
        let instant = |offset: f64| {
            let offset = (nearest_f64(span.unsigned_abs()) * offset).floor().to_string();
            Timestamp::from_nanos(start + offset.parse::<i64>().unwrap())
        };
        let (target_time, source_time) = (instant(offsets.0), instant(offsets.1));

        let across = registry
            .lookup_across(&target, target_time, &source, source_time, &fixed)
            .unwrap();
        prop_assert_eq!(across.target(), target.as_str());
        prop_assert_eq!(across.target_time(), target_time);
        prop_assert_eq!(across.source(), source.as_str());
        prop_assert_eq!(across.source_time(), source_time);
        check_geometry(across.translation(), across.rotation())?;

        // Each of the two legs, and each lookup compared against, samples
        // every edge at most once (an interpolation), composes one hop
        // per edge and inverts once; one composition joins the legs.
        let budget = count(8 * edges.len() + 8) * ROUNDING;
        let geometry = (across.translation(), across.rotation());

        // Equal times: the fixed frame drops out.
        let same = registry
            .lookup_across(&target, target_time, &source, target_time, &fixed)
            .unwrap();
        let direct = registry.lookup(&target, &source, target_time).unwrap();
        check_close(
            (same.translation(), same.rotation()),
            (direct.translation(), direct.rotation()),
            budget,
            magnitude,
        )?;

        // An endpoint that is the fixed frame does not move, so its time
        // drops out.
        if fixed == target {
            let direct = registry.lookup(&target, &source, source_time).unwrap();
            check_close(geometry, (direct.translation(), direct.rotation()), budget, magnitude)?;
        }
        if fixed == source {
            let direct = registry.lookup(&target, &source, target_time).unwrap();
            check_close(geometry, (direct.translation(), direct.rotation()), budget, magnitude)?;
        }

        // Swapping the endpoints with their times gives the inverse.
        let swapped = registry
            .lookup_across(&source, source_time, &target, target_time, &fixed)
            .unwrap();
        let there = edge("s", "t", across.translation(), across.rotation(), Stamp::Static);
        let back = edge("t", "s", swapped.translation(), swapped.rotation(), Stamp::Static);
        let composed = there.compose(&back).unwrap();
        check_close(
            (composed.translation(), composed.rotation()),
            (Vector3::new(0.0, 0.0, 0.0), Rotation::IDENTITY),
            2.0 * budget,
            magnitude,
        )?;
    }

    #[test]
    fn a_lookup_fails_exactly_when_an_edge_of_the_chain_does_not_cover_the_instant(
        edges in tree_edges(),
        cutoff in prop::option::weighted(0.3, 0_i64..150),
        picks in (0_usize..100, 0_usize..100),
        requested in 0_i64..200,
    ) {
        let tree = Tree::build(&edges, cutoff, 1.0);
        let (a, b) = (picks.0 % tree.frames(), picks.1 % tree.frames());
        let (target, source) = (frame(a), frame(b));

        // The oracle: the first edge in chain order that cannot serve the
        // instant. Edges above the meeting point may fail too, and must
        // not fail the lookup.
        let gap = tree
            .chain(a, b)
            .into_iter()
            .find(|(_, coverage)| !coverage.serves(requested));

        let result = tree.registry.lookup(&target, &source, Timestamp::from_nanos(requested));
        match gap {
            None => {
                let found = result.unwrap();
                prop_assert_eq!(found.target(), target.as_str());
                prop_assert_eq!(found.source(), source.as_str());
                prop_assert_eq!(found.stamp(), at(requested));
                check_transform(&found)?;
            }
            Some((failing, coverage)) => prop_assert!(
                matches!(
                    &result,
                    Err(error) if is_coverage_error(error, &frame(failing), coverage, requested)
                ),
                "expected the coverage error of f{failing} ({coverage:?}), got {result:?}",
            ),
        }
    }

    #[test]
    fn a_failed_lookup_is_diagnosed_the_same_whatever_the_geometry(
        // The random tree, plus a disconnected island and a frame that
        // exists nowhere, built twice: with unit translations, where no
        // chain can overflow and every error is a diagnosis, and with
        // translations whose sum over two hops overflows. Geometry must
        // not change a diagnosis.
        edges in tree_edges(),
        cutoff in prop::option::weighted(0.3, 0_i64..150),
        picks in (0_usize..100, 0_usize..100, 0_usize..100),
        requested in (0_i64..200, 0_i64..200),
    ) {
        let build = |x: f64| {
            let mut registry = Tree::build(&edges, cutoff, x).registry;
            let island = edge("island", "g", Vector3::new(x, 0.0, 0.0), Rotation::IDENTITY, Stamp::Static);
            registry.insert(&island).unwrap();
            registry
        };
        let (unit, extreme) = (build(1.0), build(1.0e308));

        let mut frames: Vec<String> = (0..=edges.len()).map(frame).collect();
        frames.extend(["island".into(), "g".into(), "missing".into()]);
        let pick = |index: usize| frames[index % frames.len()].as_str();
        let (a, b, fixed) = (pick(picks.0), pick(picks.1), pick(picks.2));
        let (first, second) = (
            Timestamp::from_nanos(requested.0),
            Timestamp::from_nanos(requested.1),
        );

        if let Err(diagnosis) = unit.lookup(a, b, first) {
            prop_assert_eq!(extreme.lookup(a, b, first), Err(diagnosis));
        }
        if let Err(diagnosis) = unit.lookup_across(a, first, b, second, fixed) {
            prop_assert_eq!(extreme.lookup_across(a, first, b, second, fixed), Err(diagnosis));
        }
        prop_assert_eq!(extreme.latest_common_time(a, b), unit.latest_common_time(a, b));
    }

    #[test]
    fn latest_common_time_is_the_newest_instant_every_edge_of_the_chain_covers(
        edges in tree_edges(),
        cutoff in prop::option::weighted(0.3, 0_i64..150),
        picks in (0_usize..100, 0_usize..100),
        later in 1_i64..1_000,
        anywhere in timestamp_nanos(0),
    ) {
        let tree = Tree::build(&edges, cutoff, 1.0);
        let (a, b) = (picks.0 % tree.frames(), picks.1 % tree.frames());
        let (target, source) = (frame(a), frame(b));
        let chain = tree.chain(a, b);

        let outcome = tree.registry.latest_common_time(&target, &source);
        let mirrored = tree.registry.latest_common_time(&source, &target);
        let expected = expected_latest(&chain);
        prop_assert_eq!(observed_latest(&outcome),
            Some(expected), "got {:?}", outcome);
        prop_assert_eq!(
            observed_latest(&mirrored),
            Some(expected_latest(&tree.chain(b, a))),
            "mirrored: got {:?}",
            mirrored
        );
        // Both directions serve the same instants. A refusal may name a
        // different edge of the same chain, since the chain is read from
        // the other end.
        prop_assert_eq!(outcome.is_ok(), mirrored.is_ok());
        if outcome.is_ok() {
            prop_assert_eq!(&outcome, &mirrored);
        }

        match expected {
            Latest::At(newest) => {
                // The instant is covered by every edge of the chain, and
                // it is the newest such instant: the edge whose samples
                // end there covers nothing later.
                let served = tree.registry.lookup(&target, &source, Timestamp::from_nanos(newest));
                prop_assert!(served.is_ok(), "the promised instant fails: {served:?}");
                for beyond in [newest + 1, newest + later] {
                    let gap = chain.iter().find(|(_, coverage)| !coverage.serves(beyond));
                    let refused = tree.registry.lookup(&target, &source, Timestamp::from_nanos(beyond));
                    prop_assert!(
                        matches!(
                            (&refused, gap),
                            (Err(error), Some((failing, coverage)))
                                if is_coverage_error(error, &frame(*failing), *coverage, beyond)
                        ),
                        "expected OutOfRange past the promised instant, got {refused:?}",
                    );
                }
            }
            Latest::Static => {
                // No dynamic edge on the chain: every instant is served.
                let served = tree.registry.lookup(&target, &source, Timestamp::from_nanos(anywhere));
                prop_assert!(served.is_ok(), "a static chain must serve any instant: {served:?}");
            }
            Latest::NoData(_) | Latest::NoCommonTime { .. } => {
                // No instant is covered by every edge.
                for nanos in 0..200 {
                    prop_assert!(chain.iter().any(|(_, coverage)| !coverage.serves(nanos)));
                }
            }
        }
    }
}

// ---- mutation: trees, batches, known names ----

proptest! {
    #![proptest_config(config(512))]

    #[test]
    fn random_inserts_and_reparents_keep_the_tree_walkable(
        // Each op is (reparent?, parent frame, child frame, instant). Most
        // of the reparents are rejected (an unknown frame, a root, an
        // unchanged parent, a cycle) and every rejection is a valid
        // outcome: what is under test is that no accepted sequence can
        // leave a tree a walk gets lost in.
        ops in prop::collection::vec((any::<bool>(), 0..POOL, 0..POOL, 1_i64..8), 1..24),
    ) {
        let mut registry = Registry::unbounded();
        for (reparent, parent, child, nanos) in ops {
            let transform = link_transform((parent, child, false, nanos));

            if reparent {
                if registry.reparent(&transform).is_ok() {
                    // The move happened, so repeating it is no move at
                    // all, and must not wipe the seed the first call
                    // stored.
                    let repeated = registry.reparent(&transform);
                    prop_assert!(
                        is_refusal_of_repeat(&repeated, parent, child),
                        "expected ParentUnchanged for f{child}, got {repeated:?}",
                    );
                    let seed = registry.lookup(&frame(parent), &frame(child), Timestamp::from_nanos(nanos));
                    prop_assert_eq!(seed.as_ref(), Ok(&transform));
                }
            } else {
                // Cycles, second parents and self-edges are all expected
                // here; only their acceptance would be a bug.
                let _ = registry.insert(&transform);
            }
        }

        // Every ordered pair of frames, at instants inside and outside the
        // published range. A cycle would make the chain walk run forever,
        // so returning at all is the property; an answer that does come
        // back must carry exactly the frames and instant asked for.
        for target in (0..POOL).map(frame) {
            for source in (0..POOL).map(frame) {
                for nanos in [0_i64, 4, 9] {
                    if let Ok(transform) = registry.lookup(&target, &source, Timestamp::from_nanos(nanos)) {
                        prop_assert_eq!(transform.target(), target.as_str());
                        prop_assert_eq!(transform.source(), source.as_str());
                        prop_assert_eq!(transform.stamp(), at(nanos));
                        check_transform(&transform)?;
                    }
                }
                // The coverage query walks the same tree, and an instant
                // it promises must be servable: a reparent's collapsed
                // history is the hop best placed to break that.
                match registry.latest_common_time(&target, &source) {
                    Ok(Stamp::At(promised)) => {
                        let served = registry.lookup(&target, &source, promised);
                        prop_assert!(
                            served.is_ok(),
                            "latest_common_time promised {promised:?}, lookup failed: {served:?}",
                        );
                    }
                    Ok(Stamp::Static) => {
                        let served = registry.lookup(&target, &source, Timestamp::from_nanos(4));
                        prop_assert!(
                            served.is_ok(),
                            "a static chain must serve any instant, got {served:?}",
                        );
                    }
                    Err(_) => {}
                }
            }
        }
    }

    #[test]
    fn insert_all_matches_the_insert_loop_or_changes_nothing(
        // A small window over a narrow range of stamps, on few frames:
        // two samples for one frame often sit further apart than the
        // window, in either order, so a member is often expired by an
        // earlier member of the same batch and not by anything stored
        // (about one run in five with these parameters).
        max_age in prop::option::weighted(0.85, 0_u64..4),
        seed in prop::collection::vec(inserts(), 0..3),
        batch in prop::collection::vec(inserts(), 0..10),
    ) {
        let fresh = || {
            max_age.map_or_else(Registry::unbounded, |age| {
                Registry::with_max_age(Duration::from_nanos(age))
            })
        };
        let mut names: Vec<String> = (0..FEW).map(frame).collect();
        names.push("nobody".into());
        let instants: Vec<i64> = (0..9).collect();

        // Both orders: ascending stamps evict, descending stamps expire.
        let reversed: Vec<Insert> = batch.iter().rev().copied().collect();
        for batch in [batch, reversed] {
            let (mut looped, mut batched) = (fresh(), fresh());
            for &insert in &seed {
                // Rejections are part of the setup, not under test.
                let _ = looped.insert(&build(insert));
                let _ = batched.insert(&build(insert));
            }
            let before = observable(&batched, &names, &instants);
            let transforms: Vec<Transform> = batch.iter().map(|&insert| build(insert)).collect();

            let expected = transforms.iter().enumerate().try_for_each(|(index, transform)| {
                looped.insert(transform).map_err(|error| BatchError { index, error })
            });
            let result = batched.insert_all(&transforms);

            // The same verdict, and on failure the same first error at the
            // same index.
            prop_assert_eq!(&result, &expected);
            if result.is_ok() {
                prop_assert_eq!(observable(&batched, &names, &instants), observable(&looped, &names, &instants));
            } else {
                prop_assert_eq!(observable(&batched, &names, &instants), before);
            }
        }
    }

    #[test]
    fn a_registry_matches_the_known_names_model_after_every_step(
        ops in prop::collection::vec(ops(), 1..24),
    ) {
        let names = probe_names();
        let instants = [0_i64, 4, 9];
        let mut registry = Registry::unbounded();
        let mut model = Model::new();
        check_against_model(&registry, &model)?;

        for op in ops {
            let before = observable(&registry, &names, &instants);
            let changed = match &op {
                Op::Insert(link) => {
                    let result = registry.insert(&link_transform(*link));
                    match model_insert(&mut model, *link) {
                        Ok(()) => prop_assert_eq!(&result, &Ok(()), "{:?}", op),
                        Err(refusal) => prop_assert!(
                            matches!(&result, Err(error) if is_refusal(error, refusal, *link)),
                            "{op:?}: expected {refusal:?}, got {result:?}",
                        ),
                    }
                    result.is_ok()
                }
                Op::InsertAll(batch) => {
                    let transforms: Vec<Transform> = batch.iter().map(|&link| link_transform(link)).collect();
                    let result = registry.insert_all(&transforms);
                    // The model applies the members in order to a copy and
                    // keeps it only if every one is accepted.
                    let mut staged = model.clone();
                    let refused = batch
                        .iter()
                        .enumerate()
                        .find_map(|(index, &link)| model_insert(&mut staged, link).err().map(|refusal| (index, refusal)));
                    match refused {
                        None => {
                            prop_assert_eq!(&result, &Ok(()), "{:?}", op);
                            model = staged;
                        }
                        Some((index, refusal)) => prop_assert!(
                            matches!(
                                &result,
                                Err(BatchError { index: reported, error })
                                    if *reported == index && is_refusal(error, refusal, batch[index])
                            ),
                            "{op:?}: expected {refusal:?} at {index}, got {result:?}",
                        ),
                    }
                    result.is_ok()
                }
                Op::Reparent(link) => {
                    let result = registry.reparent(&link_transform(*link));
                    match model_reparent(&mut model, *link) {
                        Ok(()) => prop_assert_eq!(&result, &Ok(()), "{:?}", op),
                        Err(refusal) => prop_assert!(
                            matches!(&result, Err(error) if is_refusal(error, refusal, *link)),
                            "{op:?}: expected {refusal:?}, got {result:?}",
                        ),
                    }
                    result.is_ok()
                }
                Op::Remove(index) => {
                    let removed = registry.remove_frame(&names[*index]);
                    prop_assert_eq!(removed, model.remove(index).is_some(), "{:?}", op);
                    removed
                }
                Op::Drain(cutoff) => {
                    // Samples go; topology, kinds and names stay.
                    registry.remove_samples_through(Timestamp::from_nanos(*cutoff));
                    true
                }
            };
            // A refused operation changes no answer, the known names
            // among them.
            if !changed {
                prop_assert_eq!(observable(&registry, &names, &instants), before, "{:?}", op);
            }
            check_against_model(&registry, &model)?;
        }
    }

    #[test]
    fn a_removed_frame_can_return_under_a_new_parent_and_kind(
        was_static in any::<bool>(),
        has_child in any::<bool>(),
        nanos in timestamp_nanos(0),
        translation in translations(),
        rotation in rotations(),
    ) {
        let stamp = |fixed: bool| if fixed { Stamp::Static } else { at(nanos) };
        let instant = Timestamp::from_nanos(nanos);
        let mut registry = Registry::unbounded();
        registry.insert(&edge("old", "frame", translation, rotation, stamp(was_static))).unwrap();
        if has_child {
            registry.insert(&edge("frame", "leaf", translation, rotation, at(nanos))).unwrap();
        }

        prop_assert!(registry.remove_frame("frame"));
        prop_assert!(!registry.remove_frame("frame"));
        // The parent lost its last edge. The frame itself is known only
        // while its child still names it.
        prop_assert!(is_unknown(&registry, "old"));
        prop_assert_eq!(is_unknown(&registry, "frame"), !has_child);
        prop_assert_eq!(is_unknown(&registry, "leaf"), !has_child);
        if has_child {
            prop_assert!(registry.lookup("frame", "leaf", instant).is_ok());
        }

        // Neither the old parent nor the old kind is remembered.
        let returned = edge("new", "frame", translation, rotation, stamp(!was_static));
        prop_assert_eq!(registry.insert(&returned), Ok(()));
        let found = registry.lookup("new", "frame", instant).unwrap();
        prop_assert_eq!(vector_bits(found.translation()), vector_bits(translation));
        prop_assert_eq!(rotation_bits(found.rotation()), rotation_bits(rotation));
        prop_assert!(is_unknown(&registry, "old"));
        let stale = registry.lookup("old", "frame", instant);
        prop_assert!(
            matches!(&stale, Err(LookupError::UnknownFrame { frame, .. }) if frame == "old"),
            "the old parent still answers: {stale:?}",
        );
        if has_child {
            prop_assert!(registry.lookup("new", "leaf", instant).is_ok());
        }

        // The new pins hold like any others.
        let old_parent = registry.insert(&edge("old", "frame", translation, rotation, stamp(!was_static)));
        prop_assert!(
            matches!(
                &old_parent,
                Err(InsertError::ParentMismatch { frame, pinned, offered, .. })
                    if frame == "frame" && pinned == "new" && offered == "old"
            ),
            "expected ParentMismatch, got {old_parent:?}",
        );
        let old_kind = registry.insert(&edge("new", "frame", translation, rotation, stamp(was_static)));
        prop_assert!(
            matches!(
                &old_kind,
                Err(InsertError::KindMismatch { frame, incoming, .. })
                    if frame == "frame" && *incoming == stamp(was_static)
            ),
            "expected KindMismatch, got {old_kind:?}",
        );
        prop_assert!(is_unknown(&registry, "old"));
    }

    #[test]
    fn a_parent_is_known_until_it_loses_its_last_edge(
        order in Just((0..5_usize).collect::<Vec<_>>()).prop_shuffle(),
        children in 1_usize..=5,
        has_grandparent in any::<bool>(),
        last_leaves_by_reparent in any::<bool>(),
        fixed in any::<bool>(),
    ) {
        let stamp = if fixed { Stamp::Static } else { at(7) };
        let hop = |parent: &str, child: &str| {
            edge(parent, child, Vector3::new(1.0, 0.0, 0.0), Rotation::IDENTITY, stamp)
        };
        let mut registry = Registry::unbounded();
        if has_grandparent {
            registry.insert(&hop("grandparent", "parent")).unwrap();
        }
        let order: Vec<usize> = order.into_iter().filter(|child| *child < children).collect();
        for &child in &order {
            registry.insert(&hop("parent", &frame(child))).unwrap();
        }

        for (removed, &child) in order.iter().enumerate() {
            prop_assert!(!is_unknown(&registry, "parent"));
            let last = removed + 1 == order.len();
            if last && last_leaves_by_reparent {
                prop_assert_eq!(registry.reparent(&hop("elsewhere", &frame(child))), Ok(()));
                prop_assert!(!is_unknown(&registry, &frame(child)));
                prop_assert!(!is_unknown(&registry, "elsewhere"));
            } else {
                prop_assert!(registry.remove_frame(&frame(child)));
                prop_assert!(is_unknown(&registry, &frame(child)));
            }
            // Known while any edge names it: a remaining child, or its
            // own edge to the grandparent.
            prop_assert_eq!(is_unknown(&registry, "parent"), last && !has_grandparent);
        }

        if !has_grandparent {
            let lost = registry.latest_common_time("parent", "parent");
            prop_assert!(
                matches!(&lost, Err(LookupError::UnknownFrame { frame, .. }) if frame == "parent"),
                "expected UnknownFrame, got {lost:?}",
            );
            prop_assert!(!registry.remove_frame("parent"));
        }
    }

    #[test]
    fn a_failed_insert_batch_or_reparent_leaves_every_answer_unchanged(
        edges in tree_edges(),
        cutoff in prop::option::weighted(0.3, 0_i64..150),
        nanos in 0_i64..100,
    ) {
        let tree = Tree::build(&edges, cutoff, 1.0);
        let mut registry = tree.registry;
        // `f1` always exists, directly under the root `f0`.
        let f1_static = edges[0].1.is_none();
        let kind = |fixed: bool| if fixed { Stamp::Static } else { at(nanos) };
        let hop = |parent: &str, child: &str, stamp: Stamp| {
            edge(parent, child, Vector3::new(1.0, 0.0, 0.0), Rotation::IDENTITY, stamp)
        };

        let fresh = ["fresh_a", "fresh_b", "fresh_c"];
        let mut names: Vec<String> = (0..=edges.len()).map(frame).collect();
        names.extend(fresh.map(String::from));
        let instants = [0_i64, 50, 99, 150];
        let before = observable(&registry, &names, &instants);

        // Every refusal below offers at least one name the registry does
        // not know. None of them may become known, and no other answer
        // may move.
        let refused_inserts = [
            hop("fresh_a", "fresh_a", kind(false)),
            hop("fresh_a", "f1", kind(f1_static)),
            hop("f0", "f1", kind(!f1_static)),
            hop("f1", "f0", kind(false)),
        ];
        for transform in &refused_inserts {
            let result = registry.insert(transform);
            prop_assert!(result.is_err(), "insert of {transform:?} was accepted");
            prop_assert_eq!(&observable(&registry, &names, &instants), &before, "{:?}", result);
        }

        // Batches whose leading members are acceptable and introduce
        // names, and whose last member is refused.
        let refused_batches = [
            vec![
                hop("fresh_a", "fresh_b", kind(false)),
                hop("fresh_b", "fresh_c", kind(true)),
                hop("fresh_c", "fresh_a", kind(false)),
            ],
            vec![hop("fresh_a", "fresh_b", kind(true)), hop("fresh_c", "fresh_c", kind(true))],
            vec![hop("f1", "fresh_a", kind(false)), hop("fresh_b", "fresh_a", kind(false))],
            vec![hop("f1", "fresh_a", kind(false)), hop("f1", "fresh_a", kind(true))],
        ];
        for batch in &refused_batches {
            let result = registry.insert_all(batch);
            prop_assert!(
                matches!(&result, Err(BatchError { index, .. }) if *index == batch.len() - 1),
                "expected the last member to be refused, got {result:?}",
            );
            prop_assert_eq!(&observable(&registry, &names, &instants), &before, "{:?}", result);
        }

        // An unknown child, a root, an unchanged parent, the frame itself,
        // the other kind, and, where `f1` has a child, a cycle through it.
        let mut refused_reparents = vec![
            hop("fresh_a", "fresh_b", kind(false)),
            hop("fresh_a", "f0", kind(false)),
            hop("f0", "f1", kind(f1_static)),
            hop("f1", "f1", kind(f1_static)),
            hop("fresh_a", "f1", kind(!f1_static)),
        ];
        if let Some(below) = tree.parents.iter().position(|&parent| parent == 1) {
            refused_reparents.push(hop(&frame(below + 1), "f1", kind(f1_static)));
        }
        for seed in &refused_reparents {
            let result = registry.reparent(seed);
            prop_assert!(result.is_err(), "reparent to {seed:?} was accepted");
            prop_assert_eq!(&observable(&registry, &names, &instants), &before, "{:?}", result);
        }
        for name in fresh {
            prop_assert!(is_unknown(&registry, name));
            prop_assert!(!registry.remove_frame(name));
        }

        // A member expired by an earlier member of its own batch: the
        // frames that batch introduced stay unknown.
        let mut bounded = Registry::with_max_age(Duration::from_nanos(1));
        bounded.insert(&hop("f0", "f1", at(nanos))).unwrap();
        let before = observable(&bounded, &names, &instants);
        let result = bounded.insert_all(&[
            hop("fresh_a", "fresh_b", at(nanos + 2)),
            hop("f1", "fresh_c", at(nanos)),
            hop("fresh_a", "fresh_b", at(nanos)),
        ]);
        prop_assert!(
            matches!(
                &result,
                Err(BatchError { index: 2, error: InsertError::Expired { frame, stamp, newest, .. } })
                    if frame == "fresh_b"
                        && stamp.as_nanos() == nanos
                        && newest.as_nanos() == nanos + 2
            ),
            "expected Expired at index 2, got {result:?}",
        );
        prop_assert_eq!(observable(&bounded, &names, &instants), before);
    }
}

fn is_refusal_of_repeat(
    repeated: &Result<(), InsertError>,
    parent: usize,
    child: usize,
) -> bool {
    matches!(
        repeated,
        Err(InsertError::ParentUnchanged { frame: named, parent: pinned, .. })
            if *named == frame(child) && *pinned == frame(parent)
    )
}
