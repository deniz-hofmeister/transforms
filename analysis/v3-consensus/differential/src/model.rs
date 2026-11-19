//! A reference model of the frame tree both registries are expected to
//! hold. It never decides whether an operation succeeds: it is updated only
//! with what v3 accepted. The comparison uses it to evaluate the predicates
//! of the declared difference classes (which edges a chain crosses, which
//! samples a lookup reads, whether two frames are connected) and the
//! generator uses it to aim operations at existing frames.

use std::collections::BTreeMap;

/// Magnitudes are accumulated in units of `1 / MAG_SCALE`, so that their
/// sums stay finite for every finite input: one sample contributes at most
/// `3 * f64::MAX * MAG_SCALE`, and a chain would need more than 21,000
/// edges to reach `f64::MAX`. Scaling by a power of two is exact (down to
/// the subnormal range, where the lost bits are far below the bound's
/// unit term).
pub const MAG_SCALE: f64 = 1.0 / 65536.0;

/// `(|x| + |y| + |z|) * MAG_SCALE`, computed so that it cannot overflow.
pub fn scaled_mag(translation: [f64; 3]) -> f64 {
    translation.iter().map(|c| c.abs() * MAG_SCALE).sum()
}

/// One stored pose as v3 holds it.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Translation x, y, z then rotation w, x, y, z, read back from the
    /// constructed v3 transform.
    pub geo: [f64; 7],
    /// The rotation was outside v3's unit band and v3 normalized it, so
    /// 2.4 holds different rotation bits for this sample.
    pub off: bool,
    /// The rotation w, x, y, z as the scenario gave it to both versions,
    /// before any normalization. 2.4 stores these.
    pub raw: [f64; 4],
    /// Sum of the absolute translation components, times `MAG_SCALE`.
    pub mag: f64,
}

#[derive(Debug)]
pub enum Hist {
    Static(Sample),
    Dynamic(BTreeMap<i64, Sample>),
}

#[derive(Debug)]
pub struct Edge {
    pub parent: String,
    pub hist: Hist,
}

/// What a chain between two frames crosses at the instants a query reads.
#[derive(Clone, Debug, Default)]
pub struct ChainInfo {
    /// Edges on the chain.
    pub hops: usize,
    /// Edges whose samples read by the query include a normalized one.
    pub off: usize,
    /// Sum over the edges of the largest translation magnitude read, times
    /// `MAG_SCALE`. Finite for every chain of finite samples.
    pub mag: f64,
    /// Zero hops, or one hop in the stored direction answered by a stored
    /// sample: the result must be bit-identical.
    pub exact: bool,
    /// For one stored-direction hop at a stored instant, the stored v3
    /// geometry.
    pub stored: Option<[f64; 7]>,
}

impl ChainInfo {
    pub fn joined(
        &self,
        other: &Self,
    ) -> Self {
        Self {
            hops: self.hops + other.hops,
            off: self.off + other.off,
            mag: self.mag + other.mag,
            exact: false,
            stored: None,
        }
    }
}

#[derive(Debug)]
pub struct Model {
    pub edges: BTreeMap<String, Edge>,
    /// The retention window in nanoseconds, if the registries have one.
    pub max_age: Option<i128>,
}

impl Model {
    pub fn new(max_age: Option<i128>) -> Self {
        Self {
            edges: BTreeMap::new(),
            max_age,
        }
    }

    /// A name is known iff a stored edge names it as child or parent.
    pub fn known(
        &self,
        name: &str,
    ) -> bool {
        self.edges.contains_key(name) || self.edges.values().any(|edge| edge.parent == name)
    }

    /// The frames from `name` up to its root, `name` first.
    fn path<'a>(
        &'a self,
        name: &'a str,
    ) -> Vec<&'a str> {
        let mut path = vec![name];
        let mut current = name;
        while let Some(edge) = self.edges.get(current) {
            current = &edge.parent;
            path.push(current);
        }
        path
    }

    /// The child frames of the edges between `target`, `source` and their
    /// meeting point: the target's side, then the source's side, each from
    /// the endpoint upward. `None` if a name is unknown or no chain exists.
    pub fn chain(
        &self,
        target: &str,
        source: &str,
    ) -> Option<(Vec<String>, Vec<String>)> {
        if !self.known(target) || !self.known(source) {
            return None;
        }
        let target_path = self.path(target);
        let source_path = self.path(source);
        if target_path.last() != source_path.last() {
            return None;
        }
        let (mut i, mut j) = (target_path.len(), source_path.len());
        while i > 0 && j > 0 && target_path[i - 1] == source_path[j - 1] {
            i -= 1;
            j -= 1;
        }
        let names = |path: &[&str]| path.iter().map(|&name| name.to_owned()).collect();
        Some((names(&target_path[..i]), names(&source_path[..j])))
    }

    pub fn disconnected(
        &self,
        a: &str,
        b: &str,
    ) -> bool {
        self.known(a) && self.known(b) && self.chain(a, b).is_none()
    }

    /// The newest stored instant of a dynamic frame that holds samples.
    pub fn newest(
        &self,
        child: &str,
    ) -> Option<i64> {
        match &self.edges.get(child)?.hist {
            Hist::Static(_) => None,
            Hist::Dynamic(samples) => samples.keys().next_back().copied(),
        }
    }

    /// The sample stored for exactly `at`, or the static pose.
    pub fn stored_at(
        &self,
        child: &str,
        at: i64,
    ) -> Option<&Sample> {
        match &self.edges.get(child)?.hist {
            Hist::Static(sample) => Some(sample),
            Hist::Dynamic(samples) => samples.get(&at),
        }
    }

    /// The samples a lookup at `at` reads on this edge: the stored one, or
    /// the two that bracket `at`. Empty where the edge does not cover `at`.
    fn read_at(
        &self,
        child: &str,
        at: i64,
    ) -> Vec<&Sample> {
        match self.edges.get(child).map(|edge| &edge.hist) {
            None => Vec::new(),
            Some(Hist::Static(sample)) => vec![sample],
            Some(Hist::Dynamic(samples)) => {
                if let Some(sample) = samples.get(&at) {
                    return vec![sample];
                }
                let before = samples.range(..at).next_back();
                let after = samples.range(at..).next();
                match (before, after) {
                    (Some((_, a)), Some((_, b))) => vec![a, b],
                    _ => Vec::new(),
                }
            }
        }
    }

    pub fn chain_info(
        &self,
        target: &str,
        source: &str,
        at: i64,
    ) -> Option<ChainInfo> {
        let (target_side, source_side) = self.chain(target, source)?;
        let mut info = ChainInfo::default();
        for child in target_side.iter().chain(&source_side) {
            let read = self.read_at(child, at);
            info.hops += 1;
            info.off += usize::from(read.iter().any(|sample| sample.off));
            info.mag += read.iter().map(|sample| sample.mag).fold(0.0, f64::max);
        }
        if target_side.is_empty() {
            match source_side.as_slice() {
                [] => info.exact = true,
                [child] => {
                    info.stored = self.stored_at(child, at).map(|sample| sample.geo);
                    info.exact = info.stored.is_some();
                }
                _ => {}
            }
        }
        Some(info)
    }

    /// Applies an insert v3 accepted.
    pub fn insert(
        &mut self,
        parent: &str,
        child: &str,
        stamp: Option<i64>,
        sample: Sample,
    ) {
        let max_age = self.max_age;
        match self.edges.get_mut(child) {
            None => {
                self.edges.insert(
                    child.to_owned(),
                    Edge {
                        parent: parent.to_owned(),
                        hist: seeded(stamp, sample),
                    },
                );
            }
            Some(edge) => match (&mut edge.hist, stamp) {
                (Hist::Static(pose), None) => *pose = sample,
                (Hist::Dynamic(samples), Some(at)) => {
                    samples.insert(at, sample);
                    let newest = samples.keys().next_back().copied().unwrap_or(at);
                    if let Some(max_age) = max_age {
                        let cutoff = i128::from(newest) - max_age;
                        samples.retain(|&stamp, _| i128::from(stamp) >= cutoff);
                    }
                }
                // v3 accepted a kind the model's edge does not have; the
                // caller's next comparison exposes the disagreement.
                _ => {}
            },
        }
    }

    /// Applies a `reparent` v3 accepted: the edge holds only the seed.
    pub fn reparent(
        &mut self,
        parent: &str,
        child: &str,
        stamp: Option<i64>,
        sample: Sample,
    ) {
        self.edges.insert(
            child.to_owned(),
            Edge {
                parent: parent.to_owned(),
                hist: seeded(stamp, sample),
            },
        );
    }

    pub fn remove_frame(
        &mut self,
        child: &str,
    ) -> bool {
        self.edges.remove(child).is_some()
    }

    /// Removes every dynamic sample at or before `cutoff`.
    pub fn remove_through(
        &mut self,
        cutoff: i64,
    ) {
        for edge in self.edges.values_mut() {
            if let Hist::Dynamic(samples) = &mut edge.hist {
                samples.retain(|&stamp, _| stamp > cutoff);
            }
        }
    }

    /// The stored tree, one edge per line, for a failure report.
    pub fn describe(&self) -> String {
        let mut text = String::new();
        for (child, edge) in &self.edges {
            let held = match &edge.hist {
                Hist::Static(sample) => format!("static {:?}", sample.geo),
                Hist::Dynamic(samples) => samples
                    .iter()
                    .map(|(stamp, sample)| format!("{stamp}: {:?}", sample.geo))
                    .collect::<Vec<_>>()
                    .join("; "),
            };
            text.push_str(&format!("  {} <- {child}: {held}\n", edge.parent));
        }
        text
    }

    /// The dynamic edges holding a sample at exactly `at`, as
    /// `(parent, child)`.
    pub fn edges_with_sample_at(
        &self,
        at: i64,
    ) -> Vec<(String, String)> {
        self.edges
            .iter()
            .filter(|(_, edge)| matches!(&edge.hist, Hist::Dynamic(s) if s.contains_key(&at)))
            .map(|(child, edge)| (edge.parent.clone(), child.clone()))
            .collect()
    }
}

fn seeded(
    stamp: Option<i64>,
    sample: Sample,
) -> Hist {
    match stamp {
        None => Hist::Static(sample),
        Some(at) => Hist::Dynamic(BTreeMap::from([(at, sample)])),
    }
}
