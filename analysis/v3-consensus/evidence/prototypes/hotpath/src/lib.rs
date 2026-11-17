pub mod proto;
use proto::{BTreeHist, BTreeIdx, DequeHist, HashIdx, History, Iso, NameIndex, Q, V3};
use std::hint::black_box;
use std::time::Duration;
use transforms::{
    Registry, errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind { Unknown, Disconnected, NoData, OutOfRange, Other }

pub trait Sut: Sized {
    type Sample<'a>;
    fn name() -> String;
    fn new(max_age: Option<Duration>) -> Self;
    fn make<'a>(parent: &'a str, child: &'a str, t: [f64; 3], q: [f64; 4], stamp: Option<u64>) -> Self::Sample<'a>;
    fn insert(&mut self, s: Self::Sample<'_>) -> bool;
    fn get(&self, target: &str, source: &str, at: u64) -> Result<[f64; 7], (Kind, String)>;
    /// Native lookup, result black-boxed; returns whether it succeeded.
    fn lookup(&self, target: &str, source: &str, at: u64) -> bool;
    fn add(&mut self, parent: &str, child: &str, t: [f64; 3], q: [f64; 4], stamp: Option<u64>) {
        assert!(self.insert(Self::make(parent, child, t, q, stamp)));
    }
}

pub struct V24(pub Registry);
impl Sut for V24 {
    type Sample<'a> = Transform;
    fn name() -> String { "v2.4".into() }
    fn new(max_age: Option<Duration>) -> Self {
        V24(match max_age { Some(d) => Registry::with_max_age(d), None => Registry::new() })
    }
    fn make<'a>(parent: &'a str, child: &'a str, t: [f64; 3], q: [f64; 4], stamp: Option<u64>) -> Transform {
        let stamp = match stamp { Some(n) => Stamp::At(Timestamp::from_nanos(n)), None => Stamp::Static };
        Transform::new(parent, child, Vector3::new(t[0], t[1], t[2]), Quaternion::from_wxyz(q[0], q[1], q[2], q[3]), stamp).unwrap()
    }
    #[inline] fn insert(&mut self, s: Transform) -> bool { self.0.add_transform(black_box(s)).is_ok() }
    fn get(&self, target: &str, source: &str, at: u64) -> Result<[f64; 7], (Kind, String)> {
        match self.0.get_transform(target, source, Timestamp::from_nanos(at)) {
            Ok(tf) => {
                let (t, r) = (tf.translation(), tf.rotation());
                Ok([t.x, t.y, t.z, r.w, r.x, r.y, r.z])
            }
            Err(RegistryError::UnknownFrame(f)) => Err((Kind::Unknown, f)),
            Err(RegistryError::Disconnected { .. }) => Err((Kind::Disconnected, String::new())),
            Err(RegistryError::NotFoundAt { frame, covered, .. }) =>
                Err((if covered.is_some() { Kind::OutOfRange } else { Kind::NoData }, frame)),
            Err(e) => Err((Kind::Other, format!("{e:?}"))),
        }
    }
    #[inline] fn lookup(&self, target: &str, source: &str, at: u64) -> bool {
        black_box(self.0.get_transform(black_box(target), black_box(source), Timestamp::from_nanos(black_box(at)))).is_ok()
    }
}

pub struct Proto<H, I>(pub proto::Registry<H, I>);
impl<H: History, I: NameIndex> Sut for Proto<H, I> {
    type Sample<'a> = (&'a str, &'a str, Iso, Option<i64>);
    fn name() -> String { format!("proto-{}-{}", H::NAME, I::NAME) }
    fn new(max_age: Option<Duration>) -> Self { Proto(proto::Registry::new(max_age)) }
    fn make<'a>(parent: &'a str, child: &'a str, t: [f64; 3], q: [f64; 4], stamp: Option<u64>) -> Self::Sample<'a> {
        (parent, child, Iso { t: V3 { x: t[0], y: t[1], z: t[2] }, r: Q { w: q[0], x: q[1], y: q[2], z: q[3] } }, stamp.map(|n| n as i64))
    }
    #[inline] fn insert(&mut self, s: Self::Sample<'_>) -> bool {
        let s = black_box(s);
        self.0.insert(s.0, s.1, s.2, s.3).is_ok()
    }
    fn get(&self, target: &str, source: &str, at: u64) -> Result<[f64; 7], (Kind, String)> {
        use proto::LookupError as E;
        match self.0.lookup(target, source, at as i64) {
            Ok(i) => Ok([i.t.x, i.t.y, i.t.z, i.r.w, i.r.x, i.r.y, i.r.z]),
            Err(E::UnknownFrame { frame }) => Err((Kind::Unknown, frame)),
            Err(E::Disconnected { .. }) => Err((Kind::Disconnected, String::new())),
            Err(E::NoData { frame }) => Err((Kind::NoData, frame)),
            Err(E::OutOfRange { frame, .. }) => Err((Kind::OutOfRange, frame)),
            Err(E::NonFinite) => Err((Kind::Other, "nonfinite".into())),
        }
    }
    #[inline] fn lookup(&self, target: &str, source: &str, at: u64) -> bool {
        black_box(self.0.lookup(black_box(target), black_box(source), black_box(at) as i64)).is_ok()
    }
}

pub type PBH = Proto<BTreeHist, HashIdx>;
pub type PBB = Proto<BTreeHist, BTreeIdx>;
pub type PDH = Proto<DequeHist, HashIdx>;
pub type PDB = Proto<DequeHist, BTreeIdx>;

/// splitmix64
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 { self.next() % n }
    pub fn unit(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 { lo + (hi - lo) * self.unit() }
    pub fn quat(&mut self) -> [f64; 4] {
        loop {
            let q = [self.range(-1.0, 1.0), self.range(-1.0, 1.0), self.range(-1.0, 1.0), self.range(-1.0, 1.0)];
            let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
            if n > 0.1 { return [q[0] / n, q[1] / n, q[2] / n, q[3] / n]; }
        }
    }
}

// ---- workloads, modelled on benches/benchmarks.rs ----
pub const BASE: u64 = 1_000_000_000;
pub const STEP: u64 = 1_000_000;
const AXIS: f64 = 0.577_350_269_189_625_8;
pub const X: [f64; 3] = [1.0, 0.0, 0.0];
pub const ID: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

pub fn rot(nanos: u64) -> [f64; 4] {
    let h = (nanos - BASE) as f64 * 1e-9 / 2.0;
    let (s, c) = (h.sin(), h.cos());
    [c, s * AXIS, s * AXIS, s * AXIS]
}
pub const MID: u64 = BASE + 500 * STEP + STEP / 2;

/// Chain names[0] -> names[1] -> ... each dynamic edge with `samples` samples.
pub fn chain<S: Sut>(names: &[&str], samples: u64) -> S {
    let mut r = S::new(None);
    for i in 0..samples {
        let n = BASE + i * STEP;
        for w in names.windows(2) { r.add(w[0], w[1], X, rot(n), Some(n)); }
    }
    r
}
pub fn robot_tree<S: Sut>(samples: u64) -> S {
    let mut r = S::new(None);
    r.add("base_link", "laser", [0.2, 0.0, 0.1], ID, None);
    r.add("base_link", "imu", [0.0, 0.0, 0.05], ID, None);
    r.add("camera", "camera_optical", [0.0; 3], ID, None);
    for i in 0..samples {
        let n = BASE + i * STEP;
        r.add("map", "odom", X, rot(n), Some(n));
        r.add("odom", "base_link", X, rot(n), Some(n));
        r.add("base_link", "camera", X, rot(n), Some(n));
    }
    r
}
/// r0 -> ... -> r16 dynamic (1k samples each), two static leaves under r16.
pub fn deep_siblings<S: Sut>(depth: usize, samples: u64) -> S {
    let names: Vec<String> = (0..=depth).map(|i| format!("r{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut r: S = chain(&refs, samples);
    r.add(refs[depth], "left", [0.2, 0.0, 0.1], rot(BASE + 300 * STEP), None);
    r.add(refs[depth], "right", [0.0, 0.3, 0.1], rot(BASE + 700 * STEP), None);
    r
}
pub fn deep_static<S: Sut>() -> S {
    let mut r = S::new(None);
    for i in 0..1000 { r.add(&i.to_string(), &(i + 1).to_string(), [0.0; 3], ID, None); }
    r
}
pub fn common_parent_elim<S: Sut>() -> S {
    let mut r = S::new(None);
    r.add("a_999", "b_0", [0.0; 3], ID, None);
    r.add("a_999", "c_0", [0.0; 3], ID, None);
    for i in 0..1000 {
        for p in ["a", "b", "c"] { r.add(&format!("{p}_{i}"), &format!("{p}_{}", i + 1), [0.0; 3], ID, None); }
    }
    r
}
/// The read-side workloads: (label, target, source, at, expect_ok).
pub struct Read<S> { pub label: &'static str, pub reg: S, pub target: &'static str, pub source: &'static str, pub at: u64, pub ok: bool }
pub fn reads<S: Sut>() -> Vec<Read<S>> {
    let last = BASE + 999 * STEP;
    let mk = |label, reg, target, source, at, ok| Read { label, reg, target, source, at, ok };
    vec![
        mk("1hop_exact_1k", chain(&["a", "b"], 1000), "a", "b", last, true),
        mk("1hop_interp_1k", chain(&["a", "b"], 1000), "a", "b", MID, true),
        mk("4hop_dyn_to_ancestor", chain(&["a", "b", "c", "d", "e"], 1000), "a", "e", MID, true),
        mk("4hop_dyn_to_descendant", chain(&["a", "b", "c", "d", "e"], 1000), "e", "a", MID, true),
        mk("robot_map_to_optical", robot_tree(1000), "map", "camera_optical", MID, true),
        mk("robot_leaf_to_leaf", robot_tree(1000), "laser", "camera_optical", MID, true),
        mk("siblings_under_16deep_dyn", deep_siblings(16, 1000), "left", "right", MID, true),
        mk("static_siblings_1k_elim", common_parent_elim(), "b_999", "c_999", 0, true),
        mk("static_climb_1k", deep_static(), "0", "999", 0, true),
        mk("unknown_frame_1k_frames", deep_static(), "1000", "unknown", 0, false),
        mk("past_newest_1k", chain(&["a", "b"], 1000), "a", "b", last + 2 * STEP, false),
    ]
}
