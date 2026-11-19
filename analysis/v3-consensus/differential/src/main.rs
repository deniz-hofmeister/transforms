//! Differential harness: the same operation stream applied to a
//! `transforms` 2.4.0 registry and to the v3 registry of this working tree,
//! compared outcome by outcome. See README.md for the difference classes.

mod generate;
mod model;
mod rng;

use core::time::Duration;
use generate::{Generator, Op, Spec};
use model::{ChainInfo, Hist, MAG_SCALE, Model, Sample, scaled_mag};
use std::{collections::BTreeSet, process::ExitCode};
use transforms::{
    BatchError, CrossTimeTransform, InsertError, LookupError, Registry, Rotation, Stamp, Timestamp,
    Transform, TransformError, Vector3,
};
use transforms_v2::{
    Registry as Registry2,
    errors::{RegistryError as Error2, TransformError as TransformError2},
    geometry::{Quaternion as Quaternion2, Transform as Transform2, Vector3 as Vector32},
    time::{Stamp as Stamp2, Timestamp as Timestamp2},
};

const DEFAULT_SEED: u64 = 0x7472_616e_7366_6f72;
const DEFAULT_OPS: u64 = 300_000;

/// Class 5 bound, per component, for a chain of `hops` edges whose read
/// translations sum to `mag`: `K5 * EPSILON * (hops + 1) * (mag + 1)` on
/// the translation and `K5 * EPSILON * (hops + 1)` on the rotation. The
/// translation side is evaluated in scaled coordinates, see `compare`.
const K5: f64 = 8.0;
/// Class 4 adds, per edge that read a normalized rotation,
/// `K4 * 1e-6 * (mag + 1)` on the translation and `K4 * 1e-6` on the
/// rotation. 1e-6 is the norm tolerance both versions accept.
const K4: f64 = 4.0;
const NORM_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Equal,
    Disconnected,
    Expired,
    Cutoff,
    Normalized,
    LastBits,
    NonFinite,
    EqualUnknown,
    UnknownOrder,
    EmptyName,
    CrossTimeType,
    LegOrder,
    /// Declared by the maintainer on 2026-11-18; not in the Acceptance list
    /// as first written.
    OverflowEdge,
}

const CLASSES: [(Class, &str); 13] = [
    (Class::Equal, "EQUAL"),
    (Class::Disconnected, " 1 Disconnected precedence"),
    (Class::Expired, " 2 Expired (insert and batch)"),
    (Class::Cutoff, " 3 inclusive cleanup cutoff"),
    (Class::Normalized, " 4 off-band rotation normalized"),
    (Class::LastBits, " 5 last-bit numeric change"),
    (Class::NonFinite, " 6 Ok(non-finite) to Err(NonFinite)"),
    (Class::EqualUnknown, " 7 equal unregistered names"),
    (Class::UnknownOrder, " 8 unknown-name report order"),
    (Class::EmptyName, " 9 empty name"),
    (Class::CrossTimeType, "10 cross-time result type"),
    (Class::LegOrder, "11 cross-time leg order"),
    (Class::OverflowEdge, "12 finite/NonFinite at overflow edge"),
];
/// The leading entries of `CLASSES` that are equal or declared.
const DECLARED: usize = 13;

#[derive(Default)]
struct Stats {
    counts: [u64; 13],
    unclassified: u64,
    ops: u64,
    scenarios: u64,
    /// Lookups that had to be bit-identical, and were.
    exact_lookups: u64,
    /// Largest observed class 5 difference in units of its bound.
    worst_last_bits: f64,
    /// Largest observed class 4 difference in units of its bound.
    worst_normalized: f64,
    first_overflow_edge: Option<String>,
}

impl Stats {
    fn count(
        &mut self,
        class: Class,
    ) {
        let slot = CLASSES.iter().position(|(c, _)| *c == class).unwrap_or(0);
        self.counts[slot] += 1;
    }
}

/// v3's final answer to a batch, the members left in it, and how many
/// expired members were dropped on the way.
type BatchOutcome = (Result<(), BatchError>, Vec<Built>, u64);

/// How the two geometries of a successful pair relate.
enum Geometry {
    Identical,
    LastBits,
    Normalized,
    /// 2.4 returned a non-finite value where v3 returned a finite one, on a
    /// chain at the overflow edge.
    OverflowEdge,
}

/// The query a lookup outcome belongs to, kept so that it can be replayed.
#[derive(Clone, Copy, Debug)]
enum Query<'a> {
    Lookup {
        target: &'a str,
        source: &'a str,
        at: i64,
    },
    Across {
        target: &'a str,
        target_time: i64,
        source: &'a str,
        source_time: i64,
        fixed: &'a str,
    },
}

/// A chain longer than this is refused by `compare`: the scaled magnitude
/// sum is proven finite only below it.
const MAX_HOPS: usize = 1000;

/// Translations of the scaled replay are multiplied by this power of two.
/// It leaves a factor of 2^64 between any intermediate of the chain and
/// `f64::MAX`, and it is exact for every component of magnitude at least
/// 2^-958.
const REPLAY_SCALE: f64 = 1.0 / 18_446_744_073_709_551_616.0;

/// A difference between two finite geometries that is within its bound.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Within {
    /// Class 5, with the difference in units of the bound.
    LastBits(f64),
    /// Class 4, with the difference in units of the bound.
    Normalized(f64),
}

/// The class 4 / class 5 predicate on two finite geometries that are not
/// bit-identical.
///
/// `mag` is the chain's magnitude sum times `unit`, and `unit` is the
/// factor by which the comparison is scaled relative to the coordinates of
/// `old` and `new` times `MAG_SCALE`: `MAG_SCALE` for a result in the
/// scenario's own coordinates. Every translation component is multiplied
/// by `MAG_SCALE` before it is differenced, so neither the difference nor
/// the bound `K * (mag + unit)` can overflow, and for inputs away from the
/// overflow and underflow edges the ratio is the unscaled one exactly.
/// Anything that is not a ratio at most 1, a NaN included, is an error.
fn compare(
    old: [f64; 7],
    new: [f64; 7],
    hops: usize,
    off: usize,
    mag: f64,
    unit: f64,
) -> Result<Within, String> {
    if !finite(old) || !finite(new) || hops > MAX_HOPS {
        return Err(format!(
            "not comparable over {hops} hops: 2.4 {old:?}, v3 {new:?}"
        ));
    }
    let translation = (0..3)
        .map(|i| (old[i] * MAG_SCALE - new[i] * MAG_SCALE).abs())
        .fold(0.0, f64::max);
    let rotation = (3..7).map(|i| (old[i] - new[i]).abs()).fold(0.0, f64::max);
    let rounding = K5 * f64::EPSILON * (hops + 1) as f64;
    let bound = rounding + K4 * NORM_TOLERANCE * off as f64;
    let scale = mag + unit;
    let (on_translation, on_rotation) = (translation / (bound * scale), rotation / bound);
    let worst = on_translation.max(on_rotation);
    // `f64::max` discards a NaN operand, so each ratio is tested itself.
    let within = scale.is_finite() && scale > 0.0 && on_translation <= 1.0 && on_rotation <= 1.0;
    let class = if off == 0 { 5 } else { 4 };
    if !within {
        return Err(format!(
            "geometry differs by {worst:.3} times the class {class} bound over {hops} hops, \
             {off} normalized, scaled magnitude {mag:e}: 2.4 {old:?}, v3 {new:?}"
        ));
    }
    Ok(if off == 0 {
        Within::LastBits(worst)
    } else {
        Within::Normalized(worst)
    })
}

struct Scenario<'a> {
    v2: Registry2,
    v3: Registry,
    model: Model,
    stats: &'a mut Stats,
}

/// A transform built on both sides, with what the model stores for it.
struct Built {
    spec: Spec,
    v2: Transform2,
    v3: Transform,
    sample: Sample,
}

enum Construction {
    Both(Box<Built>),
    /// Both constructors refused it for corresponding reasons.
    Neither,
    /// v3 refused an empty name; 2.4 built the transform.
    EmptyName(Transform2),
    /// v3 refused an empty name and 2.4 refused the translation.
    EmptyNameNeither,
}

fn ts2(nanos: i64) -> Timestamp2 {
    Timestamp2::from_nanos(u64::try_from(nanos).expect("stamps are non-negative"))
}

fn ts3(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

fn same_instant(
    old: Timestamp2,
    new: Timestamp,
) -> bool {
    u64::try_from(new.as_nanos()).is_ok_and(|nanos| nanos == old.as_nanos())
}

fn same_stamp(
    old: Stamp2,
    new: Stamp,
) -> bool {
    match (old, new) {
        (Stamp2::Static, Stamp::Static) => true,
        (Stamp2::At(old), Stamp::At(new)) => same_instant(old, new),
        _ => false,
    }
}

fn stamp3(stamp: Option<i64>) -> Stamp {
    stamp.map_or(Stamp::Static, |nanos| Stamp::At(ts3(nanos)))
}

fn geo2(t: &Transform2) -> [f64; 7] {
    let (p, q) = (t.translation(), t.rotation());
    [p.x, p.y, p.z, q.w, q.x, q.y, q.z]
}

fn geo3(
    p: Vector3,
    q: Rotation,
) -> [f64; 7] {
    [p.x, p.y, p.z, q.w(), q.x(), q.y(), q.z()]
}

fn bits(geo: [f64; 7]) -> [u64; 7] {
    geo.map(f64::to_bits)
}

fn finite(geo: [f64; 7]) -> bool {
    geo.iter().all(|c| c.is_finite())
}

const IDENTITY: [f64; 7] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

/// Builds the transform with both constructors and compares the outcomes.
///
/// v3 builds the rotation first and the transform second, so the two
/// stages are compared separately: the rotation against a 2.4 transform
/// with a zero translation, then the whole transform.
fn construct(
    spec: &Spec,
    stats: &mut Stats,
) -> Result<Construction, String> {
    let [w, x, y, z] = spec.rotation;
    let [tx, ty, tz] = spec.translation;
    let stamp2 = spec
        .stamp
        .map_or(Stamp2::Static, |nanos| Stamp2::At(ts2(nanos)));
    let quaternion = Quaternion2::from_wxyz(w, x, y, z);

    let probe = Transform2::new("p", "c", Vector32::new(0.0, 0.0, 0.0), quaternion, stamp2);
    let rotation = match (probe, Rotation::from_wxyz(w, x, y, z)) {
        (Ok(_), Ok(rotation)) => rotation,
        (Err(TransformError2::NonFiniteValues), Err(TransformError::NonFinite)) => {
            stats.count(Class::Equal);
            return Ok(Construction::Neither);
        }
        (
            Err(TransformError2::NonUnitRotation(old)),
            Err(TransformError::NonUnitRotation { norm, .. }),
        ) if old == norm || (old - norm).abs() <= 4.0 * f64::EPSILON * norm.abs() => {
            stats.count(Class::Equal);
            return Ok(Construction::Neither);
        }
        (old, new) => {
            return Err(format!("rotation construction: 2.4 {old:?}, v3 {new:?}"));
        }
    };

    let old = Transform2::new(
        &spec.target,
        &spec.source,
        Vector32::new(tx, ty, tz),
        quaternion,
        stamp2,
    );
    let new = Transform::new(
        spec.target.as_str(),
        spec.source.as_str(),
        Vector3::new(tx, ty, tz),
        rotation,
        stamp3(spec.stamp),
    );
    let empty = spec.target.is_empty() || spec.source.is_empty();
    match (old, new) {
        (Ok(v2), Ok(v3)) => {
            let geo = geo3(v3.translation(), v3.rotation());
            let sample = Sample {
                geo,
                off: [geo[3], geo[4], geo[5], geo[6]].map(f64::to_bits)
                    != [w, x, y, z].map(f64::to_bits),
                raw: spec.rotation,
                mag: scaled_mag(spec.translation),
            };
            if [geo[0], geo[1], geo[2]].map(f64::to_bits) != [tx, ty, tz].map(f64::to_bits) {
                return Err(format!("v3 changed a translation at construction: {v3:?}"));
            }
            stats.count(Class::Equal);
            Ok(Construction::Both(Box::new(Built {
                spec: spec.clone(),
                v2,
                v3,
                sample,
            })))
        }
        (Err(TransformError2::NonFiniteValues), Err(TransformError::NonFinite)) => {
            stats.count(Class::Equal);
            Ok(Construction::Neither)
        }
        (Ok(v2), Err(TransformError::EmptyFrameName)) if empty => Ok(Construction::EmptyName(v2)),
        (Err(TransformError2::NonFiniteValues), Err(TransformError::EmptyFrameName)) if empty => {
            stats.count(Class::EmptyName);
            Ok(Construction::EmptyNameNeither)
        }
        (old, new) => Err(format!("construction: 2.4 {old:?}, v3 {new:?}")),
    }
}

/// Whether a 2.4 insert error and a v3 insert error are the same finding
/// about the transform `spec`.
fn insert_errors_match(
    old: &Error2,
    new: &InsertError,
    spec: &Spec,
) -> bool {
    match (old, new) {
        (Error2::SelfReferentialFrame, InsertError::SelfEdge { frame, .. }) => {
            *frame == spec.source
        }
        (
            Error2::ReparentingNotSupported { current_parent },
            InsertError::ParentMismatch {
                frame,
                pinned,
                offered,
                ..
            },
        ) => *frame == spec.source && pinned == current_parent && *offered == spec.target,
        (
            Error2::StaticDynamicConflict,
            InsertError::KindMismatch {
                frame, incoming, ..
            },
        ) => *frame == spec.source && *incoming == stamp3(spec.stamp),
        (Error2::CycleDetected, InsertError::Cycle { child, parent, .. }) => {
            *child == spec.source && *parent == spec.target
        }
        (Error2::UnknownFrame(old), InsertError::UnknownFrame { frame, .. })
        | (Error2::NoParentToReplace(old), InsertError::NoParent { frame, .. }) => {
            old == frame && *frame == spec.source
        }
        (Error2::ParentUnchanged(old), InsertError::ParentUnchanged { frame, parent, .. }) => {
            old == frame && *frame == spec.source && *parent == spec.target
        }
        _ => false,
    }
}

/// Whether a 2.4 lookup error and a v3 lookup error are the same finding.
/// `leg` is set for one leg of a cross-time lookup, where 2.4 names a
/// disconnected pair as (fixed, endpoint) and v3 as (endpoint, fixed).
fn lookup_errors_match(
    old: &Error2,
    new: &LookupError,
    leg: bool,
) -> bool {
    match (old, new) {
        (Error2::UnknownFrame(old), LookupError::UnknownFrame { frame, .. }) => old == frame,
        (
            Error2::Disconnected {
                target_frame,
                source_frame,
            },
            LookupError::Disconnected { a, b, .. },
        ) => {
            if leg {
                a == source_frame && b == target_frame
            } else {
                a == target_frame && b == source_frame
            }
        }
        (
            Error2::NotFoundAt {
                frame: old,
                covered: None,
                ..
            },
            LookupError::NoData { frame, .. },
        ) => old == frame,
        (
            Error2::NotFoundAt {
                frame: old,
                requested: old_requested,
                covered: Some((start, end)),
                ..
            },
            LookupError::OutOfRange {
                frame,
                requested,
                earliest,
                latest,
                ..
            },
        ) => {
            old == frame
                && same_instant(*old_requested, *requested)
                && same_instant(*start, *earliest)
                && same_instant(*end, *latest)
        }
        (Error2::NonFiniteValues, LookupError::NonFinite)
        | (Error2::NonUnitRotation(_), LookupError::NonUnitRotation { .. }) => true,
        _ => false,
    }
}

fn is_coverage(error: &LookupError) -> bool {
    matches!(
        error,
        LookupError::NoData { .. } | LookupError::OutOfRange { .. }
    )
}

impl Scenario<'_> {
    /// Classifies two successful geometries.
    fn geometry(
        &mut self,
        old: [f64; 7],
        new: [f64; 7],
        info: &ChainInfo,
        query: Query,
    ) -> Result<Geometry, String> {
        if !finite(new) {
            return Err(format!(
                "v3 returned Ok with a non-finite component: {new:?}"
            ));
        }
        if info.stored.is_some_and(|stored| bits(stored) != bits(new)) {
            return Err(format!(
                "v3 direct lookup at a stored instant is not the stored sample: stored {:?}, \
                 returned {new:?}",
                info.stored
            ));
        }
        if bits(old) == bits(new) {
            if info.exact {
                self.stats.exact_lookups += 1;
            }
            return Ok(Geometry::Identical);
        }
        if !finite(old) {
            let text = format!("{query:?}: 2.4 Ok(non-finite) {old:?}, v3 Ok(finite) {new:?}");
            return self
                .overflow_edge(query, Some(info), &text)
                .map(|_| Geometry::OverflowEdge);
        }
        if info.exact && info.off == 0 {
            return Err(format!(
                "direct stored-direction lookup at a stored instant is not bit-identical: 2.4 \
                 {old:?}, v3 {new:?}"
            ));
        }
        match compare(old, new, info.hops, info.off, info.mag, MAG_SCALE)? {
            Within::LastBits(worst) => {
                self.stats.worst_last_bits = self.stats.worst_last_bits.max(worst);
                Ok(Geometry::LastBits)
            }
            Within::Normalized(worst) => {
                self.stats.worst_normalized = self.stats.worst_normalized.max(worst);
                Ok(Geometry::Normalized)
            }
        }
    }

    /// Not declared: one version returns a finite result and the other
    /// reports or returns a non-finite one. The explanation is that both
    /// use the same sandwich product, whose intermediates overflow before
    /// the result does, and that last-bit differences decide which side of
    /// the overflow a computation lands on. It is accepted only where the
    /// chain's read translations sum to at least a quarter of `f64::MAX`
    /// and the scaled replay confirms it.
    fn overflow_edge(
        &mut self,
        query: Query,
        info: Option<&ChainInfo>,
        text: &str,
    ) -> Result<Class, String> {
        let Some(info) = info else {
            return Err(format!("{text}; the model has no chain"));
        };
        if info.mag < f64::MAX / 4.0 * MAG_SCALE {
            return Err(format!("{text}; the chain is not at the overflow edge"));
        }
        self.replay_scaled(query, info)
            .map_err(|why| format!("{text}; scaled replay: {why}"))?;
        self.stats
            .first_overflow_edge
            .get_or_insert_with(|| text.to_owned());
        Ok(Class::OverflowEdge)
    }

    /// Replays one query where overflow cannot happen. The model's stored
    /// samples of every edge the query's chain crosses (both legs of a
    /// cross-time lookup) go into a fresh 2.4 registry and a fresh v3
    /// registry, with the rotation inputs the scenario gave and every
    /// translation multiplied by `REPLAY_SCALE`. Both must answer the query
    /// with `Ok`, and the two answers must be bit-identical or within the
    /// class 4 / class 5 bound taken in the scaled coordinates.
    fn replay_scaled(
        &self,
        query: Query,
        info: &ChainInfo,
    ) -> Result<(), String> {
        let legs = match query {
            Query::Lookup { target, source, .. } => vec![(target, source)],
            Query::Across {
                target,
                source,
                fixed,
                ..
            } => vec![(fixed, target), (fixed, source)],
        };
        let mut children = BTreeSet::new();
        for (a, b) in legs {
            let (up, down) = self.model.chain(a, b).ok_or("the model has no chain")?;
            children.extend(up);
            children.extend(down);
        }
        let (mut v2, mut v3) = (Registry2::new(), Registry::unbounded());
        for child in &children {
            let edge = self.model.edges.get(child).ok_or("the model has no edge")?;
            let samples: Vec<(Option<i64>, &Sample)> = match &edge.hist {
                Hist::Static(sample) => vec![(None, sample)],
                Hist::Dynamic(samples) => samples.iter().map(|(at, s)| (Some(*at), s)).collect(),
            };
            for (stamp, sample) in samples {
                let [x, y, z] = [0, 1, 2].map(|i| sample.geo[i] * REPLAY_SCALE);
                let [w, i, j, k] = sample.raw;
                let stamp2 = stamp.map_or(Stamp2::Static, |nanos| Stamp2::At(ts2(nanos)));
                let old = Transform2::new(
                    &edge.parent,
                    child,
                    Vector32::new(x, y, z),
                    Quaternion2::from_wxyz(w, i, j, k),
                    stamp2,
                )
                .map_err(|error| format!("2.4 construction: {error:?}"))?;
                let rotation = Rotation::from_wxyz(w, i, j, k)
                    .map_err(|error| format!("v3 rotation: {error:?}"))?;
                let new = Transform::new(
                    edge.parent.as_str(),
                    child.as_str(),
                    Vector3::new(x, y, z),
                    rotation,
                    stamp3(stamp),
                )
                .map_err(|error| format!("v3 construction: {error:?}"))?;
                v2.add_transform(old)
                    .map_err(|error| format!("2.4 insert: {error:?}"))?;
                v3.insert(&new)
                    .map_err(|error| format!("v3 insert: {error:?}"))?;
            }
        }
        let (old, new) = match query {
            Query::Lookup { target, source, at } => (
                v2.get_transform(target, source, ts2(at)),
                v3.lookup(target, source, ts3(at))
                    .map(|t| geo3(t.translation(), t.rotation())),
            ),
            Query::Across {
                target,
                target_time,
                source,
                source_time,
                fixed,
            } => (
                v2.get_transform_at(target, ts2(target_time), source, ts2(source_time), fixed),
                v3.lookup_across(target, ts3(target_time), source, ts3(source_time), fixed)
                    .map(|t| cross_geo(&t)),
            ),
        };
        let (old, new) = match (old, new) {
            (Ok(old), Ok(new)) => (geo2(&old), new),
            (old, new) => return Err(format!("2.4 {old:?}, v3 {new:?}")),
        };
        if bits(old) == bits(new) && finite(new) {
            return Ok(());
        }
        compare(
            old,
            new,
            info.hops,
            info.off,
            info.mag * REPLAY_SCALE,
            MAG_SCALE * REPLAY_SCALE,
        )
        .map(|_| ())
    }

    /// Class 7 or 9: 2.4 answered a query on one unregistered name used
    /// for every argument; v3 reports that name as unknown.
    fn equal_unknown(
        &self,
        name: &str,
        error: &LookupError,
    ) -> Option<Class> {
        let named = matches!(error, LookupError::UnknownFrame { frame, .. } if frame == name);
        if !named || self.model.known(name) {
            return None;
        }
        Some(if name.is_empty() {
            Class::EmptyName
        } else {
            Class::EqualUnknown
        })
    }

    /// Class 2 predicate: the model's frame holds a newest stamp (or an
    /// earlier batch member supplied one) further than `max_age` ahead of
    /// the rejected stamp, and the payload says exactly that.
    fn expired_is_justified(
        &self,
        error: &InsertError,
        spec: &Spec,
        earlier: &[Spec],
    ) -> bool {
        let InsertError::Expired {
            frame,
            stamp,
            newest,
            ..
        } = error
        else {
            return false;
        };
        let (Some(max_age), Some(at)) = (self.model.max_age, spec.stamp) else {
            return false;
        };
        let running = earlier
            .iter()
            .filter(|member| member.source == spec.source)
            .filter_map(|member| member.stamp)
            .chain(self.model.newest(&spec.source))
            .max();
        *frame == spec.source
            && stamp.as_nanos() == at
            && running == Some(newest.as_nanos())
            && i128::from(at) < i128::from(newest.as_nanos()) - max_age
    }

    fn insert(
        &mut self,
        spec: &Spec,
        reparent: bool,
    ) -> Result<Class, String> {
        let built = match construct(spec, self.stats)? {
            Construction::Both(built) => built,
            Construction::Neither | Construction::EmptyNameNeither => return Ok(Class::Equal),
            Construction::EmptyName(v2) => {
                // Class 9: the transform reaches only the 2.4 registry,
                // which must refuse it.
                let old = if reparent {
                    self.v2.reparent_frame(v2)
                } else {
                    self.v2.add_transform(v2)
                };
                return match old {
                    Err(_) => Ok(Class::EmptyName),
                    Ok(()) => Err("2.4 stored a transform with an empty frame name".into()),
                };
            }
        };
        let (old, new) = if reparent {
            (
                self.v2.reparent_frame(built.v2),
                self.v3.reparent(&built.v3),
            )
        } else {
            (self.v2.add_transform(built.v2), self.v3.insert(&built.v3))
        };
        match (old, new) {
            (Ok(()), Ok(())) => {
                let (parent, child) = (spec.target.as_str(), spec.source.as_str());
                if reparent {
                    self.model.reparent(parent, child, spec.stamp, built.sample);
                } else {
                    self.model.insert(parent, child, spec.stamp, built.sample);
                }
                Ok(Class::Equal)
            }
            (Ok(()), Err(new)) if !reparent && self.expired_is_justified(&new, spec, &[]) => {
                Ok(Class::Expired)
            }
            (Err(old), Err(new)) if insert_errors_match(&old, &new, spec) => Ok(Class::Equal),
            (old, new) => Err(format!("2.4 {old:?}, v3 {new:?}")),
        }
    }

    /// Submits a batch to v3 the way the `insert_all` rustdoc recovers
    /// from `Expired`: resubmit without the rejected member. Returns the
    /// final result, the members left in the batch, and how many expired.
    fn v3_batch(
        &mut self,
        mut members: Vec<Built>,
        sentinel: Option<&Transform>,
    ) -> Result<BatchOutcome, String> {
        let mut expired = 0;
        loop {
            let transforms: Vec<Transform> = members
                .iter()
                .map(|member| member.v3.clone())
                .chain(sentinel.cloned())
                .collect();
            let result = self.v3.insert_all(&transforms);
            let Err(error) = &result else {
                return Ok((result, members, expired));
            };
            if !matches!(error.error, InsertError::Expired { .. }) || error.index >= members.len() {
                return Ok((result, members, expired));
            }
            let earlier: Vec<Spec> = members[..error.index]
                .iter()
                .map(|member| member.spec.clone())
                .collect();
            let spec = &members[error.index].spec;
            if !self.expired_is_justified(&error.error, spec, &earlier) {
                return Err(format!("v3 batch reported an unjustified {error:?}"));
            }
            members.remove(error.index);
            expired += 1;
        }
    }

    fn batch(
        &mut self,
        specs: &[Spec],
    ) -> Result<Class, String> {
        // Members neither version can construct never exist for a caller.
        let mut old_batch = Vec::new();
        let mut members = Vec::new();
        let mut first_empty = None;
        for spec in specs {
            match construct(spec, self.stats)? {
                Construction::Both(built) => {
                    old_batch.push(built.v2.clone());
                    if first_empty.is_none() {
                        members.push(*built);
                    }
                }
                Construction::EmptyName(v2) => {
                    first_empty.get_or_insert(old_batch.len());
                    old_batch.push(v2);
                }
                Construction::Neither | Construction::EmptyNameNeither => {}
            }
        }
        let old = self.v2.add_transforms(old_batch);

        if first_empty.is_some() {
            // v3 cannot hold the batch. 2.4 must refuse it: with
            // `EmptyFrameName` (class 9), or with an earlier member's error,
            // which v3 must report for the members before the empty name.
            // The sentinel is a self-edge, refused after every real member,
            // so this v3 call stores nothing.
            let sentinel = Transform::new(
                "sentinel",
                "sentinel",
                Vector3::new(0.0, 0.0, 0.0),
                Rotation::IDENTITY,
                Stamp::Static,
            )
            .map_err(|error| format!("sentinel: {error:?}"))?;
            return match old {
                Ok(()) => Err("2.4 stored a batch holding an empty frame name".into()),
                Err(Error2::EmptyFrameName) => Ok(Class::EmptyName),
                Err(old) => {
                    let (new, members, _) = self.v3_batch(members, Some(&sentinel))?;
                    match new {
                        Err(new)
                            if new.index < members.len()
                                && insert_errors_match(
                                    &old,
                                    &new.error,
                                    &members[new.index].spec,
                                ) =>
                        {
                            Ok(Class::Equal)
                        }
                        new => Err(format!("2.4 {old:?}, v3 prefix {new:?}")),
                    }
                }
            };
        }

        let (new, members, expired) = self.v3_batch(members, None)?;
        let class = if expired > 0 {
            Class::Expired
        } else {
            Class::Equal
        };
        match (old, new) {
            (Ok(()), Ok(())) => {
                for member in members {
                    let spec = &member.spec;
                    self.model
                        .insert(&spec.target, &spec.source, spec.stamp, member.sample);
                }
                Ok(class)
            }
            (Err(old), Err(new))
                if new.index < members.len()
                    && insert_errors_match(&old, &new.error, &members[new.index].spec) =>
            {
                Ok(class)
            }
            (old, new) => Err(format!(
                "2.4 {old:?}, v3 {new:?} after dropping {expired} expired members"
            )),
        }
    }

    fn remove_frame(
        &mut self,
        name: &str,
    ) -> Result<Class, String> {
        let (old, new) = (self.v2.remove_frame(name), self.v3.remove_frame(name));
        let modelled = self.model.remove_frame(name);
        if old == new && new == modelled {
            Ok(Class::Equal)
        } else {
            Err(format!(
                "remove_frame: 2.4 {old}, v3 {new}, model {modelled}"
            ))
        }
    }

    /// The same cutoff value on both sides. v3 also removes the samples at
    /// exactly `cutoff`; each such sample is probed (class 3) and then
    /// removed from 2.4 as well, so the registries agree again.
    fn cleanup_same_cutoff(
        &mut self,
        cutoff: i64,
    ) -> Result<(), String> {
        self.v2.remove_transforms_before(ts2(cutoff));
        self.v3.remove_samples_through(ts3(cutoff));
        let affected = self.model.edges_with_sample_at(cutoff);
        self.model.remove_through(cutoff);
        for (parent, child) in affected {
            let old = self.v2.get_transform(&parent, &child, ts2(cutoff));
            let new = self.v3.lookup(&parent, &child, ts3(cutoff));
            let removed = match &new {
                Err(LookupError::NoData { frame, .. }) => {
                    *frame == child && self.model.newest(&child).is_none()
                }
                Err(LookupError::OutOfRange {
                    frame,
                    requested,
                    earliest,
                    ..
                }) => {
                    *frame == child
                        && requested.as_nanos() == cutoff
                        && earliest.as_nanos() > cutoff
                }
                _ => false,
            };
            if old.is_ok() && removed {
                self.stats.count(Class::Cutoff);
            } else {
                return Err(format!(
                    "sample of {child} at the cutoff {cutoff}: 2.4 {old:?}, v3 {new:?}"
                ));
            }
        }
        match cutoff.checked_add(1) {
            Some(next) => self.v2.remove_transforms_before(ts2(next)),
            None => self
                .v2
                .remove_transforms_before(Timestamp2::from_nanos(u64::MAX)),
        }
        Ok(())
    }

    fn lookup(
        &mut self,
        target: &str,
        source: &str,
        at: i64,
    ) -> Result<Class, String> {
        let query = Query::Lookup { target, source, at };
        let old = self.v2.get_transform(target, source, ts2(at));
        let new = self.v3.lookup(target, source, ts3(at));
        match (old, new) {
            (Ok(old), Ok(new)) => {
                if old.parent() != new.target()
                    || old.child() != new.source()
                    || new.target() != target
                    || new.source() != source
                    || !same_stamp(old.timestamp(), new.stamp())
                    || new.stamp() != Stamp::At(ts3(at))
                {
                    return Err(format!("labels differ: 2.4 {old:?}, v3 {new:?}"));
                }
                let info = self
                    .model
                    .chain_info(target, source, at)
                    .ok_or_else(|| format!("both Ok, the model has no chain: v3 {new:?}"))?;
                let new = geo3(new.translation(), new.rotation());
                Ok(match self.geometry(geo2(&old), new, &info, query)? {
                    Geometry::Identical => Class::Equal,
                    Geometry::LastBits => Class::LastBits,
                    Geometry::Normalized => Class::Normalized,
                    Geometry::OverflowEdge => Class::OverflowEdge,
                })
            }
            (Ok(old), Err(LookupError::NonFinite)) if !finite(geo2(&old)) => Ok(Class::NonFinite),
            (Ok(old), Err(new)) => match self.equal_unknown(target, &new) {
                Some(class) if target == source && bits(geo2(&old)) == bits(IDENTITY) => Ok(class),
                _ => {
                    let text = format!(
                        "lookup({target:?}, {source:?}, {at}): 2.4 Ok({old:?}), v3 {new:?}"
                    );
                    if new == LookupError::NonFinite {
                        let info = self.model.chain_info(target, source, at);
                        self.overflow_edge(query, info.as_ref(), &text)
                    } else {
                        Err(text)
                    }
                }
            },
            (Err(Error2::NonFiniteValues), Ok(new)) => {
                let text = format!(
                    "lookup({target:?}, {source:?}, {at}): 2.4 NonFiniteValues, v3 Ok({new:?})"
                );
                let info = self.model.chain_info(target, source, at);
                self.overflow_edge(query, info.as_ref(), &text)
            }
            (Err(old), Err(new)) if lookup_errors_match(&old, &new, false) => Ok(Class::Equal),
            (Err(Error2::NotFoundAt { .. }), Err(LookupError::Disconnected { a, b, .. }))
                if a == target && b == source && self.model.disconnected(target, source) =>
            {
                Ok(Class::Disconnected)
            }
            (old, new) => Err(format!("2.4 {old:?}, v3 {new:?}")),
        }
    }

    fn latest(
        &mut self,
        target: &str,
        source: &str,
    ) -> Result<Class, String> {
        let old = self.v2.latest_common_time(target, source);
        let new = self.v3.latest_common_time(target, source);
        let same = match (&old, &new) {
            (Ok(old), Ok(new)) => same_stamp(*old, *new),
            (Ok(Stamp2::Static), Err(new)) if target == source => {
                return self
                    .equal_unknown(target, new)
                    .ok_or_else(|| format!("2.4 Ok(Static), v3 {new:?}"));
            }
            (
                Err(Error2::NoCommonTime {
                    frame: old,
                    covered: None,
                    ..
                }),
                Err(LookupError::NoData { frame, .. }),
            ) => old == frame,
            (
                Err(Error2::NoCommonTime {
                    frame: old,
                    covered: Some((start, end)),
                    ..
                }),
                Err(LookupError::NoCommonTime {
                    frame,
                    earliest,
                    latest,
                    latest_possible,
                    ..
                }),
            ) => {
                old == frame
                    && same_instant(*start, *earliest)
                    && same_instant(*end, *latest)
                    && latest_possible < earliest
            }
            (
                Err(old @ (Error2::UnknownFrame(_) | Error2::Disconnected { .. })),
                Err(new @ (LookupError::UnknownFrame { .. } | LookupError::Disconnected { .. })),
            ) => lookup_errors_match(old, new, false),
            _ => false,
        };
        if same {
            Ok(Class::Equal)
        } else {
            Err(format!("2.4 {old:?}, v3 {new:?}"))
        }
    }

    /// The error DESIGN.md's diagnosis rule prescribes for a failed
    /// cross-time lookup, derived from v3's own answers for the two legs:
    /// unknown names (target, source, fixed), then a disconnected leg, then
    /// coverage, then numerics, the target's leg first within each step.
    /// `None` when both legs resolve, where only combining them can fail.
    fn prescribed_across_error(
        &self,
        target: &str,
        target_time: i64,
        source: &str,
        source_time: i64,
        fixed: &str,
    ) -> Option<LookupError> {
        let legs = [
            (
                target,
                self.v3.lookup(fixed, target, ts3(target_time)).err(),
            ),
            (
                source,
                self.v3.lookup(fixed, source, ts3(source_time)).err(),
            ),
        ];
        for name in [target, source, fixed] {
            if !self.model.known(name) {
                return self.v3.lookup(name, name, ts3(0)).err();
            }
        }
        for (endpoint, error) in &legs {
            if matches!(error, Some(LookupError::Disconnected { .. })) {
                // The pair as the cross-time lookup names it.
                return self.v3.latest_common_time(endpoint, fixed).err();
            }
        }
        let coverage = legs
            .iter()
            .find_map(|(_, error)| error.clone().filter(is_coverage));
        coverage.or_else(|| legs.into_iter().find_map(|(_, error)| error))
    }

    /// What both legs of a cross-time lookup read, per the model.
    fn across_info(
        &self,
        target: &str,
        target_time: i64,
        source: &str,
        source_time: i64,
        fixed: &str,
    ) -> Option<ChainInfo> {
        let target_leg = self.model.chain_info(fixed, target, target_time)?;
        let source_leg = self.model.chain_info(fixed, source, source_time)?;
        Some(target_leg.joined(&source_leg))
    }

    fn across(
        &mut self,
        target: &str,
        target_time: i64,
        source: &str,
        source_time: i64,
        fixed: &str,
    ) -> Result<Class, String> {
        let query = Query::Across {
            target,
            target_time,
            source,
            source_time,
            fixed,
        };
        let old =
            self.v2
                .get_transform_at(target, ts2(target_time), source, ts2(source_time), fixed);
        let new = self
            .v3
            .lookup_across(target, ts3(target_time), source, ts3(source_time), fixed);

        // v3 against its own rule, whatever 2.4 says.
        if let Err(new) = &new {
            let prescribed =
                self.prescribed_across_error(target, target_time, source, source_time, fixed);
            let numeric = !is_coverage(new)
                && matches!(
                    new,
                    LookupError::NonFinite | LookupError::NonUnitRotation { .. }
                );
            if !(prescribed.as_ref() == Some(new) || prescribed.is_none() && numeric) {
                return Err(format!(
                    "v3 reported {new:?}; its legs and the diagnosis order prescribe \
                     {prescribed:?}"
                ));
            }
        }

        match (old, new) {
            (Ok(old), Ok(new)) => {
                if old.parent() != new.target()
                    || old.child() != new.source()
                    || new.target() != target
                    || new.source() != source
                    || !same_stamp(old.timestamp(), Stamp::At(new.target_time()))
                    || new.target_time() != ts3(target_time)
                    || new.source_time() != ts3(source_time)
                {
                    return Err(format!("labels differ: 2.4 {old:?}, v3 {new:?}"));
                }
                let info = self
                    .across_info(target, target_time, source, source_time, fixed)
                    .ok_or_else(|| format!("both Ok, the model has no chain: v3 {new:?}"))?;
                let new = cross_geo(&new);
                Ok(match self.geometry(geo2(&old), new, &info, query)? {
                    Geometry::Identical => Class::CrossTimeType,
                    Geometry::LastBits => Class::LastBits,
                    Geometry::Normalized => Class::Normalized,
                    Geometry::OverflowEdge => Class::OverflowEdge,
                })
            }
            (Ok(old), Err(LookupError::NonFinite)) if !finite(geo2(&old)) => Ok(Class::NonFinite),
            (Ok(old), Err(new)) => match self.equal_unknown(target, &new) {
                Some(class)
                    if target == source
                        && source == fixed
                        && bits(geo2(&old)) == bits(IDENTITY) =>
                {
                    Ok(class)
                }
                _ => {
                    let text = format!(
                        "lookup_across({target:?}, {target_time}, {source:?}, {source_time}, \
                         {fixed:?}): 2.4 Ok({old:?}), v3 {new:?}"
                    );
                    if new == LookupError::NonFinite {
                        let info =
                            self.across_info(target, target_time, source, source_time, fixed);
                        self.overflow_edge(query, info.as_ref(), &text)
                    } else {
                        Err(text)
                    }
                }
            },
            (Err(Error2::NonFiniteValues), Ok(new)) => {
                let text = format!(
                    "lookup_across({target:?}, {target_time}, {source:?}, {source_time}, \
                     {fixed:?}): 2.4 NonFiniteValues, v3 Ok({new:?})"
                );
                let info = self.across_info(target, target_time, source, source_time, fixed);
                self.overflow_edge(query, info.as_ref(), &text)
            }
            (Err(old), Err(new)) if lookup_errors_match(&old, &new, true) => Ok(Class::Equal),
            (Err(Error2::NotFoundAt { .. }), Err(LookupError::Disconnected { a, b, .. }))
                if b == fixed
                    && (a == target || a == source)
                    && self.model.disconnected(&a, fixed) =>
            {
                Ok(Class::Disconnected)
            }
            (Err(Error2::UnknownFrame(old)), Err(LookupError::UnknownFrame { .. }))
                if !self.model.known(&old) =>
            {
                // v3's name is the prescribed one (checked above); 2.4
                // named another unknown name of the same query.
                Ok(Class::UnknownOrder)
            }
            (Err(old), Err(new)) => {
                // Class 11: both legs fail, 2.4 reports the failure of
                // the leg it resolves first (the source's), v3 the one its
                // diagnosis order prescribes (checked above).
                let old_text = format!("{old:?}");
                let old_is_a_leg_failure = [(source, source_time), (target, target_time)]
                    .into_iter()
                    .filter_map(|(endpoint, at)| {
                        self.v2.get_transform(fixed, endpoint, ts2(at)).err()
                    })
                    .any(|leg| format!("{leg:?}") == old_text);
                if old_is_a_leg_failure {
                    Ok(Class::LegOrder)
                } else {
                    Err(format!("2.4 {old_text}, v3 {new:?}"))
                }
            }
            (old, new) => Err(format!("2.4 {old:?}, v3 {new:?}")),
        }
    }

    fn apply(
        &mut self,
        op: &Op,
    ) -> Result<(), String> {
        let class = match op {
            Op::Insert(spec) => self.insert(spec, false)?,
            Op::Reparent(spec) => self.insert(spec, true)?,
            Op::Batch(specs) => self.batch(specs)?,
            Op::RemoveFrame(name) => self.remove_frame(name)?,
            Op::Cleanup(cutoff) => {
                self.v2.remove_transforms_before(ts2(*cutoff));
                self.v3.remove_samples_through(ts3(*cutoff - 1));
                self.model.remove_through(*cutoff - 1);
                Class::Equal
            }
            Op::CleanupSameCutoff(cutoff) => {
                // Counted per probed sample inside.
                return self.cleanup_same_cutoff(*cutoff);
            }
            Op::CleanupAll => {
                self.v2
                    .remove_transforms_before(Timestamp2::from_nanos(u64::MAX));
                self.v3.remove_samples_through(Timestamp::MAX);
                self.model.remove_through(i64::MAX);
                Class::Equal
            }
            Op::Lookup { target, source, at } => self.lookup(target, source, *at)?,
            Op::Across {
                target,
                target_time,
                source,
                source_time,
                fixed,
            } => self.across(target, *target_time, source, *source_time, fixed)?,
            Op::Latest { target, source } => self.latest(target, source)?,
        };
        self.stats.count(class);
        Ok(())
    }
}

fn cross_geo(t: &CrossTimeTransform) -> [f64; 7] {
    geo3(t.translation(), t.rotation())
}

/// The seed of one scenario, so a scenario replays without its
/// predecessors.
fn scenario_seed(
    seed: u64,
    index: u64,
) -> u64 {
    seed ^ index.wrapping_mul(0xD6E8_FEB8_6659_FD93)
}

/// Runs one scenario. Returns the number of operations applied and, for an
/// unclassified difference, the report to print.
fn run_scenario(
    seed: u64,
    index: u64,
    stats: &mut Stats,
) -> (u64, Option<String>) {
    let mut generator = Generator::new(scenario_seed(seed, index));
    let max_age = generator.max_age();
    let (v2, v3) = match max_age {
        None => (Registry2::new(), Registry::unbounded()),
        Some(nanos) => {
            let window = Duration::from_nanos(nanos);
            (
                Registry2::with_max_age(window),
                Registry::with_max_age(window),
            )
        }
    };
    let mut scenario = Scenario {
        v2,
        v3,
        model: Model::new(max_age.map(i128::from)),
        stats,
    };
    let mut applied = Vec::new();
    for _ in 0..generator.scenario_len() {
        let op = generator.op(&scenario.model);
        let result = scenario.apply(&op);
        applied.push(op);
        if let Err(difference) = result {
            let mut report = format!(
                "UNCLASSIFIED DIFFERENCE in scenario {index} (seed {seed}, max_age \
                 {max_age:?} ns), operation {}:\n  {difference}\noperations:\n",
                applied.len() - 1
            );
            for (i, op) in applied.iter().enumerate() {
                report.push_str(&format!("  {i:4}: {op:?}\n"));
            }
            report.push_str("tree held when it failed (parent <- child: v3 samples):\n");
            report.push_str(&scenario.model.describe());
            return (applied.len() as u64, Some(report));
        }
    }
    (applied.len() as u64, None)
}

fn parse_args() -> Result<(u64, u64, Option<u64>), String> {
    let (mut seed, mut ops, mut scenario) = (DEFAULT_SEED, DEFAULT_OPS, None);
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| format!("{flag} needs an unsigned integer"))
        };
        match flag.as_str() {
            "--seed" => seed = value()?,
            "--ops" => ops = value()?,
            "--scenario" => scenario = Some(value()?),
            _ => {
                return Err(format!(
                    "unknown argument {flag}\nusage: transforms-differential [--seed N] [--ops \
                     N] [--scenario INDEX]"
                ));
            }
        }
    }
    Ok((seed, ops, scenario))
}

fn main() -> ExitCode {
    let (seed, ops, only) = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let mut stats = Stats::default();
    let mut printed = 0;
    let mut index = only.unwrap_or(0);
    while stats.ops < ops {
        let (applied, report) = run_scenario(seed, index, &mut stats);
        stats.ops += applied;
        stats.scenarios += 1;
        if let Some(report) = report {
            stats.unclassified += 1;
            if printed < 3 {
                println!("{report}");
                printed += 1;
            }
        }
        if only.is_some() {
            break;
        }
        index += 1;
    }

    println!(
        "seed {seed}, {} scenarios, {} operations",
        stats.scenarios, stats.ops
    );
    println!("{:<40} {:>10}", "class", "outcomes");
    for ((_, label), count) in CLASSES.iter().zip(stats.counts) {
        println!("{label:<40} {count:>10}");
    }
    println!(
        "{:<40} {:>10}",
        "UNCLASSIFIED (scenario abandoned)", stats.unclassified
    );
    println!(
        "bit-identical where required (direct lookups at stored instants): {}",
        stats.exact_lookups
    );
    println!(
        "largest class 5 difference: {:.4} of its bound; largest class 4 difference: {:.4} of \
         its bound",
        stats.worst_last_bits, stats.worst_normalized
    );
    let undeclared: u64 = stats.counts[DECLARED..].iter().sum();
    if let Some(example) = &stats.first_overflow_edge {
        println!("first finite/NonFinite difference at the overflow edge:\n  {example}");
    }
    if stats.unclassified == 0 && undeclared == 0 {
        println!("RESULT: no undeclared difference");
        ExitCode::SUCCESS
    } else {
        println!(
            "RESULT: FAILED, {} unclassified and {undeclared} undeclared differences",
            stats.unclassified
        );
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY_ROTATION: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

    fn geo(translation: [f64; 3]) -> [f64; 7] {
        let [x, y, z] = translation;
        let [w, i, j, k] = IDENTITY_ROTATION;
        [x, y, z, w, i, j, k]
    }

    /// The comparator on two translations over a chain of `hops` edges
    /// whose read samples are `read`, in the scenario's coordinates.
    fn verdict(
        old: [f64; 3],
        new: [f64; 3],
        hops: usize,
        off: usize,
        read: &[[f64; 3]],
    ) -> Result<Within, String> {
        let mag = read.iter().map(|t| scaled_mag(*t)).sum();
        compare(geo(old), geo(new), hops, off, mag, MAG_SCALE)
    }

    fn ulps_above(
        x: f64,
        n: u64,
    ) -> f64 {
        f64::from_bits(x.to_bits() + n)
    }

    #[test]
    fn opposite_extremes_over_an_infinite_magnitude_sum_are_rejected() {
        let (old, new) = ([f64::MAX, 0.0, 0.0], [-f64::MAX, 0.0, 0.0]);
        // The unscaled sum is infinite; the scaled one is not.
        assert_eq!(f64::MAX + f64::MAX, f64::INFINITY);
        assert!(verdict(old, new, 2, 0, &[old, old]).is_err());
        assert!(verdict(old, new, 2, 1, &[old, old]).is_err());
    }

    #[test]
    fn a_finite_difference_over_an_infinite_magnitude_sum_is_measured() {
        // The unscaled sum is infinite, and every finite difference used to
        // pass. The scaled bound is about 6 * f64::MAX * 24 * EPSILON.
        let hop = [f64::MAX, f64::MAX, f64::MAX];
        let old = [1e300, 0.0, 0.0];
        assert!(verdict(old, [1e300 + 1e297, 0.0, 0.0], 2, 0, &[hop, hop]).is_err());
        assert!(verdict(old, [1e300 + 1e290, 0.0, 0.0], 2, 0, &[hop, hop]).is_ok());
    }

    #[test]
    fn a_few_ulp_near_an_eighth_of_the_maximum_are_accepted() {
        let x = f64::MAX / 8.0;
        let (old, new) = ([x, 0.0, 0.0], [ulps_above(x, 3), 0.0, 0.0]);
        let verdict = verdict(old, new, 2, 0, &[old, old]);
        assert!(
            matches!(verdict, Ok(Within::LastBits(worst)) if worst > 0.0 && worst < 0.1),
            "{verdict:?}"
        );
    }

    #[test]
    fn a_relative_difference_of_one_in_a_thousand_at_huge_magnitude_is_rejected() {
        let x = f64::MAX / 8.0;
        let (old, new) = ([x, 0.0, 0.0], [x * (1.0 + 1e-3), 0.0, 0.0]);
        assert!(verdict(old, new, 2, 0, &[old, old]).is_err());
        // Also beyond what normalized edges allow.
        assert!(verdict(old, new, 2, 2, &[old, old]).is_err());
    }

    #[test]
    fn small_magnitudes_keep_the_unscaled_ratio() {
        let old = [1.5, -2.0, 0.25];
        let new = [ulps_above(1.5, 2), -2.0, 0.25];
        let read = [[1.0, 2.0, 0.5], [0.5, 0.0, 0.25]];
        // (hops + 1) = 3, M + 1 = 5.25, difference = 2 ulp of 1.5 = 2 * EPSILON.
        let expected = 2.0 * f64::EPSILON / (K5 * f64::EPSILON * 3.0 * 5.25);
        assert_eq!(
            verdict(old, new, 2, 0, &read),
            Ok(Within::LastBits(expected))
        );
    }

    #[test]
    fn small_magnitudes_reject_what_they_rejected_before() {
        let read = [[1.0, 2.0, 0.5], [0.5, 0.0, 0.25]];
        let old = [1.5, -2.0, 0.25];
        assert!(verdict(old, [1.5 + 1e-9, -2.0, 0.25], 2, 0, &read).is_err());
        // One normalized edge allows 4e-6 * (M + 1), not more.
        assert!(matches!(
            verdict(old, [1.5 + 1e-5, -2.0, 0.25], 2, 1, &read),
            Ok(Within::Normalized(_))
        ));
        assert!(verdict(old, [1.5 + 1e-4, -2.0, 0.25], 2, 1, &read).is_err());
    }

    #[test]
    fn rotation_differences_are_bounded_without_the_magnitude() {
        let huge = [[f64::MAX, f64::MAX, f64::MAX]; 2];
        let old = geo([0.0, 0.0, 0.0]);
        let mut new = old;
        new[3] = 1.0 - 1e-9;
        let mag = huge.iter().map(|t| scaled_mag(*t)).sum();
        assert!(compare(old, new, 2, 0, mag, MAG_SCALE).is_err());
        new[3] = ulps_above(1.0, 1);
        assert!(matches!(
            compare(old, new, 2, 0, mag, MAG_SCALE),
            Ok(Within::LastBits(_))
        ));
    }

    #[test]
    fn non_finite_operands_are_rejected() {
        let old = geo([1.0, 0.0, 0.0]);
        for bad in [f64::NAN, f64::INFINITY] {
            let new = geo([bad, 0.0, 0.0]);
            assert!(compare(old, new, 1, 0, MAG_SCALE, MAG_SCALE).is_err());
            assert!(compare(new, old, 1, 0, MAG_SCALE, MAG_SCALE).is_err());
            assert!(compare(old, old, 1, 0, bad, MAG_SCALE).is_err());
        }
    }
}
