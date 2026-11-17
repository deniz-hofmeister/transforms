//! Throwaway prototype of the draft's section 2 internals.
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V3 { pub x: f64, pub y: f64, pub z: f64 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Q { pub w: f64, pub x: f64, pub y: f64, pub z: f64 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iso { pub t: V3, pub r: Q }

impl Q {
    #[inline] fn mul(self, o: Q) -> Q {
        Q {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }
    #[inline] fn conj(self) -> Q { Q { w: self.w, x: -self.x, y: -self.y, z: -self.z } }
    #[inline] fn scale(self, f: f64) -> Q { Q { w: self.w * f, x: self.x * f, y: self.y * f, z: self.z * f } }
    #[inline] fn norm(self) -> f64 { libm::sqrt(self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z) }
    #[inline] fn wsum(a: Q, wa: f64, b: Q, wb: f64) -> Q {
        Q { w: a.w * wa + b.w * wb, x: a.x * wa + b.x * wb, y: a.y * wa + b.y * wb, z: a.z * wa + b.z * wb }
    }
    #[inline] fn rotate(self, v: V3) -> V3 {
        let r = self.mul(Q { w: 0.0, x: v.x, y: v.y, z: v.z }).mul(self.conj());
        V3 { x: r.x, y: r.y, z: r.z }
    }
    // Copied from transforms 2.4 src/geometry/quaternion/mod.rs.
    #[inline] fn slerp(self, other: Q, t: f64) -> Q {
        let t = t.clamp(0.0, 1.0);
        let mut other = other;
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;
        if dot < 0.0 { other = other.scale(-1.0); dot = -dot; }
        let dot = dot.clamp(-1.0, 1.0);
        if dot > 1.0 - f64::EPSILON {
            let b = Q::wsum(self, 1.0 - t, other, t);
            let n = b.norm();
            return if n < f64::EPSILON { b } else { b.scale(1.0 / n) };
        }
        let theta = libm::acos(dot);
        let st = libm::sin(theta);
        let a = libm::sin((1.0 - t) * theta) / st;
        let b = libm::sin(t * theta) / st;
        Q::wsum(self, a, other, b)
    }
}

impl Iso {
    pub const IDENTITY: Iso = Iso { t: V3 { x: 0.0, y: 0.0, z: 0.0 }, r: Q { w: 1.0, x: 0.0, y: 0.0, z: 0.0 } };
    /// self * rhs, as 2.4's Isometry::mul.
    #[inline] fn compose(self, rhs: Iso) -> Iso {
        let p = self.r.rotate(rhs.t);
        Iso { t: V3 { x: p.x + self.t.x, y: p.y + self.t.y, z: p.z + self.t.z }, r: self.r.mul(rhs.r) }
    }
    /// As 2.4: normalize, conjugate (the draft would use the bare conjugate).
    #[inline] fn inverse(self) -> Iso {
        let n = self.r.norm();
        let r = self.r.scale(1.0 / n).conj();
        let p = r.rotate(self.t);
        Iso { t: V3 { x: -1.0 * p.x, y: -1.0 * p.y, z: -1.0 * p.z }, r }
    }
    #[inline] fn finite(self) -> bool {
        self.t.x.is_finite() && self.t.y.is_finite() && self.t.z.is_finite()
            && self.r.w.is_finite() && self.r.x.is_finite() && self.r.y.is_finite() && self.r.z.is_finite()
    }
    #[inline] fn interpolate(self, to: Iso, t0: i64, t1: i64, at: i64) -> Iso {
        let range = Duration::from_nanos((t1 - t0) as u64);
        let diff = Duration::from_nanos((at - t0) as u64);
        let ratio = diff.as_secs_f64() / range.as_secs_f64();
        let a = 1.0 - ratio;
        Iso {
            t: V3 {
                x: a * self.t.x + ratio * to.t.x,
                y: a * self.t.y + ratio * to.t.y,
                z: a * self.t.z + ratio * to.t.z,
            },
            r: self.r.slerp(to.r, ratio),
        }
    }
}

pub enum Sampled { Ok(Iso), Empty, OutOfRange(i64, i64) }

pub trait History: Default {
    const NAME: &'static str;
    fn insert(&mut self, t: i64, iso: Iso);
    fn evict_before(&mut self, threshold: i64);
    fn newest(&self) -> Option<i64>;
    fn sample(&self, at: i64) -> Sampled;
    fn len(&self) -> usize;
}

#[derive(Default)]
pub struct BTreeHist(BTreeMap<i64, Iso>);
impl History for BTreeHist {
    const NAME: &'static str = "btree";
    #[inline] fn insert(&mut self, t: i64, iso: Iso) { self.0.insert(t, iso); }
    #[inline] fn evict_before(&mut self, threshold: i64) {
        while let Some((&o, _)) = self.0.first_key_value() {
            if o >= threshold { break; }
            self.0.pop_first();
        }
    }
    #[inline] fn newest(&self) -> Option<i64> { self.0.last_key_value().map(|(k, _)| *k) }
    #[inline] fn sample(&self, at: i64) -> Sampled {
        // Same two range probes as 2.4's Buffer::get_nearest.
        let before = self.0.range(..=at).next_back();
        if let Some((&t, iso)) = before {
            if t == at { return Sampled::Ok(*iso); }
        }
        let after = self.0.range(at..).next();
        match (before, after) {
            (Some((&t0, a)), Some((&t1, b))) => Sampled::Ok(a.interpolate(*b, t0, t1, at)),
            _ => match (self.0.first_key_value(), self.0.last_key_value()) {
                (Some((&f, _)), Some((&l, _))) => Sampled::OutOfRange(f, l),
                _ => Sampled::Empty,
            },
        }
    }
    fn len(&self) -> usize { self.0.len() }
}

#[derive(Default)]
pub struct DequeHist(VecDeque<(i64, Iso)>);
impl History for DequeHist {
    const NAME: &'static str = "deque";
    #[inline] fn insert(&mut self, t: i64, iso: Iso) {
        match self.0.back() {
            None => self.0.push_back((t, iso)),
            Some(&(last, _)) if t > last => self.0.push_back((t, iso)),
            _ => match self.0.binary_search_by_key(&t, |e| e.0) {
                Ok(i) => self.0[i].1 = iso,
                Err(i) => self.0.insert(i, (t, iso)),
            },
        }
    }
    #[inline] fn evict_before(&mut self, threshold: i64) {
        while let Some(&(o, _)) = self.0.front() {
            if o >= threshold { break; }
            self.0.pop_front();
        }
    }
    #[inline] fn newest(&self) -> Option<i64> { self.0.back().map(|e| e.0) }
    #[inline] fn sample(&self, at: i64) -> Sampled {
        let i = self.0.partition_point(|e| e.0 <= at);
        if i > 0 {
            let (t0, a) = self.0[i - 1];
            if t0 == at { return Sampled::Ok(a); }
            if let Some(&(t1, b)) = self.0.get(i) {
                return Sampled::Ok(a.interpolate(b, t0, t1, at));
            }
        }
        match (self.0.front(), self.0.back()) {
            (Some(f), Some(l)) => Sampled::OutOfRange(f.0, l.0),
            _ => Sampled::Empty,
        }
    }
    fn len(&self) -> usize { self.0.len() }
}

pub trait NameIndex: Default {
    const NAME: &'static str;
    fn get(&self, name: &str) -> Option<u32>;
    fn put(&mut self, name: Box<str>, id: u32);
}
#[derive(Default)]
pub struct HashIdx(hashbrown::HashMap<Box<str>, u32>);
impl NameIndex for HashIdx {
    const NAME: &'static str = "hash";
    #[inline] fn get(&self, name: &str) -> Option<u32> { self.0.get(name).copied() }
    fn put(&mut self, name: Box<str>, id: u32) { self.0.insert(name, id); }
}
#[derive(Default)]
pub struct BTreeIdx(BTreeMap<Box<str>, u32>);
impl NameIndex for BTreeIdx {
    const NAME: &'static str = "btreeidx";
    #[inline] fn get(&self, name: &str) -> Option<u32> { self.0.get(name).copied() }
    fn put(&mut self, name: Box<str>, id: u32) { self.0.insert(name, id); }
}

const NONE: u32 = u32::MAX;

enum EdgeData<H> { None, Static(Iso), Dynamic(H) }

struct Slot<H> { name: Box<str>, parent: u32, edge: EdgeData<H> }

#[derive(Debug, Clone, PartialEq)]
pub enum InsertError { EmptyFrameName, SelfParent, ParentMismatch, KindMismatch, Cycle, NonFinite, NonUnitRotation }

#[derive(Debug, Clone, PartialEq)]
pub enum LookupError {
    UnknownFrame { frame: String },
    Disconnected { target: String, source: String },
    NoData { frame: String },
    OutOfRange { frame: String, requested: i64, earliest: i64, latest: i64 },
    NonFinite,
}

pub struct Registry<H, I> { slots: Vec<Slot<H>>, index: I, max_age: Option<i64> }

impl<H: History, I: NameIndex> Registry<H, I> {
    pub fn new(max_age: Option<Duration>) -> Self {
        Self { slots: Vec::new(), index: I::default(), max_age: max_age.map(|d| d.as_nanos() as i64) }
    }

    fn intern(&mut self, name: &str) -> (u32, bool) {
        if let Some(id) = self.index.get(name) { return (id, false); }
        let id = self.slots.len() as u32;
        self.slots.push(Slot { name: name.into(), parent: NONE, edge: EdgeData::None });
        self.index.put(name.into(), id);
        (id, true)
    }

    /// `stamp: None` is a static sample.
    pub fn insert(&mut self, parent: &str, child: &str, iso: Iso, stamp: Option<i64>) -> Result<(), InsertError> {
        if !iso.finite() { return Err(InsertError::NonFinite); }
        if (iso.r.norm() - 1.0).abs() > 1e-6 { return Err(InsertError::NonUnitRotation); }
        if parent.is_empty() || child.is_empty() { return Err(InsertError::EmptyFrameName); }
        if parent == child { return Err(InsertError::SelfParent); }

        // Steady state: one index probe, then a string compare against the pinned parent.
        if let Some(c) = self.index.get(child) {
            let pinned = self.slots[c as usize].parent;
            if pinned != NONE {
                if &*self.slots[pinned as usize].name != parent { return Err(InsertError::ParentMismatch); }
                return Self::store(&mut self.slots[c as usize], iso, stamp, self.max_age);
            }
            // Existing root gaining a parent: cycle check before anything is created.
            if let Some(p) = self.index.get(parent) {
                let mut cur = p;
                while cur != NONE {
                    if cur == c { return Err(InsertError::Cycle); }
                    cur = self.slots[cur as usize].parent;
                }
            }
        }
        // New edge. (A freshly created child has no descendants: no cycle possible.)
        let (p, _) = self.intern(parent);
        let (c, _) = self.intern(child);
        let slot = &mut self.slots[c as usize];
        slot.parent = p;
        slot.edge = match stamp { None => EdgeData::Static(iso), Some(_) => EdgeData::Dynamic(H::default()) };
        Self::store(slot, iso, stamp, self.max_age)
    }

    #[inline]
    fn store(slot: &mut Slot<H>, iso: Iso, stamp: Option<i64>, max_age: Option<i64>) -> Result<(), InsertError> {
        match (&mut slot.edge, stamp) {
            (EdgeData::Static(s), None) => { *s = iso; Ok(()) }
            (EdgeData::Dynamic(h), Some(t)) => {
                h.insert(t, iso);
                if let (Some(age), Some(newest)) = (max_age, h.newest()) {
                    if let Some(threshold) = newest.checked_sub(age) { h.evict_before(threshold); }
                }
                Ok(())
            }
            _ => Err(InsertError::KindMismatch),
        }
    }

    #[inline]
    fn sample(&self, id: u32, at: i64) -> Result<Iso, LookupError> {
        let slot = &self.slots[id as usize];
        match &slot.edge {
            EdgeData::Static(iso) => Ok(*iso),
            EdgeData::Dynamic(h) => match h.sample(at) {
                Sampled::Ok(iso) => Ok(iso),
                Sampled::Empty => Err(LookupError::NoData { frame: slot.name.to_string() }),
                Sampled::OutOfRange(earliest, latest) => Err(LookupError::OutOfRange {
                    frame: slot.name.to_string(), requested: at, earliest, latest,
                }),
            },
            EdgeData::None => Err(LookupError::NoData { frame: slot.name.to_string() }),
        }
    }

    #[inline]
    fn depth(&self, mut id: u32) -> (u32, u32) {
        let mut d = 0;
        loop {
            let p = self.slots[id as usize].parent;
            if p == NONE { return (d, id); }
            id = p;
            d += 1;
        }
    }

    /// Expresses `source` coordinates in `target` at `at`.
    pub fn lookup(&self, target: &str, source: &str, at: i64) -> Result<Iso, LookupError> {
        let Some(mut t) = self.index.get(target) else {
            return Err(LookupError::UnknownFrame { frame: target.into() });
        };
        let Some(mut s) = self.index.get(source) else {
            return Err(LookupError::UnknownFrame { frame: source.into() });
        };
        if t == s { return Ok(Iso::IDENTITY); }
        let (t0, s0) = (t, s);
        let (mut dt, rt) = self.depth(t);
        let (mut ds, rs) = self.depth(s);
        if rt != rs {
            return Err(LookupError::Disconnected { target: target.into(), source: source.into() });
        }
        // Every edge sampled below is on the connecting chain: a side is
        // lifted only while deeper than the other, then both step while they differ.
        let mut tacc: Option<Iso> = None;
        let mut sacc: Option<Iso> = None;
        macro_rules! step {
            ($id:ident, $acc:ident) => {{
                let hop = match self.sample($id, at) {
                    Ok(hop) => hop,
                    Err(e) => return Err(self.diagnose(t0, s0, at, e)),
                };
                $acc = Some(match $acc { None => hop, Some(a) => hop.compose(a) });
                $id = self.slots[$id as usize].parent;
            }};
        }
        while dt > ds { step!(t, tacc); dt -= 1; }
        while ds > dt { step!(s, sacc); ds -= 1; }
        while t != s { step!(t, tacc); step!(s, sacc); }
        let out = match (tacc, sacc) {
            (None, None) => return Ok(Iso::IDENTITY),
            (None, Some(s)) => s,
            (Some(t), None) => t.inverse(),
            (Some(t), Some(s)) => t.inverse().compose(s),
        };
        if out.finite() { Ok(out) } else { Err(LookupError::NonFinite) }
    }

    /// Error path only: re-walk the connecting chain so the reported edge is
    /// the lowest failing one on the target side, else on the source side.
    #[cold]
    #[inline(never)]
    fn diagnose(&self, mut t: u32, mut s: u32, at: i64, first: LookupError) -> LookupError {
        let (mut dt, _) = self.depth(t);
        let (mut ds, _) = self.depth(s);
        let (mut terr, mut serr) = (None, None);
        let mut probe = |id: u32, slot: &mut Option<LookupError>| {
            if slot.is_none() { if let Err(e) = self.sample(id, at) { *slot = Some(e); } }
        };
        while dt > ds { probe(t, &mut terr); t = self.slots[t as usize].parent; dt -= 1; }
        while ds > dt { probe(s, &mut serr); s = self.slots[s as usize].parent; ds -= 1; }
        while t != s {
            probe(t, &mut terr); probe(s, &mut serr);
            t = self.slots[t as usize].parent; s = self.slots[s as usize].parent;
        }
        terr.or(serr).unwrap_or(first)
    }

    pub fn samples(&self, child: &str) -> usize {
        match self.index.get(child).map(|c| &self.slots[c as usize].edge) {
            Some(EdgeData::Dynamic(h)) => h.len(),
            Some(EdgeData::Static(_)) => 1,
            _ => 0,
        }
    }
}
