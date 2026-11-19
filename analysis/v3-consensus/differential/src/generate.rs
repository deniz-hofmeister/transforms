//! The scenario generator: one deterministic stream of operations per
//! scenario, aimed at the model's current tree often enough that most
//! inserts land, and off it often enough that every rejection is reached.

use crate::{
    model::{Hist, Model},
    rng::Rng,
};
use std::f64::consts::FRAC_1_SQRT_2;

/// Names inserts draw from.
const FRAMES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];
/// Names no insert ever uses, so they stay unregistered.
const UNREGISTERED: [&str; 2] = ["yy", "zz"];

/// What the caller hands to both constructors. `stamp: None` is static.
#[derive(Clone, Debug)]
pub struct Spec {
    pub target: String,
    pub source: String,
    pub translation: [f64; 3],
    /// w, x, y, z as given, before any normalization.
    pub rotation: [f64; 4],
    pub stamp: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum Op {
    Insert(Spec),
    Batch(Vec<Spec>),
    Reparent(Spec),
    RemoveFrame(String),
    /// 2.4 `remove_transforms_before(t)`, v3 `remove_samples_through(t - 1)`.
    Cleanup(i64),
    /// The same cutoff value `t` given to both calls.
    CleanupSameCutoff(i64),
    /// 2.4 `remove_transforms_before(u64::MAX)`, v3
    /// `remove_samples_through(Timestamp::MAX)`.
    CleanupAll,
    Lookup {
        target: String,
        source: String,
        at: i64,
    },
    Across {
        target: String,
        target_time: i64,
        source: String,
        source_time: i64,
        fixed: String,
    },
    Latest {
        target: String,
        source: String,
    },
}

pub struct Generator {
    rng: Rng,
    /// Nanoseconds per clock tick in this scenario.
    unit: i64,
    clock: i64,
    /// Every stamp generated so far, for aiming queries.
    stamps: Vec<i64>,
}

impl Generator {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let unit = rng.pick(&[1, 1, 1_000_003, 1_000_000_000]);
        Self {
            rng,
            unit,
            clock: 0,
            stamps: Vec::new(),
        }
    }

    /// The retention window of this scenario's registries, in nanoseconds.
    pub fn max_age(&mut self) -> Option<u64> {
        if self.rng.percent(35) {
            return None;
        }
        let ticks = self.rng.pick(&[0, 50, 200, 5_000]);
        Some(ticks * self.unit.unsigned_abs())
    }

    pub fn scenario_len(&mut self) -> usize {
        60 + self.rng.index(140)
    }

    pub fn op(
        &mut self,
        model: &Model,
    ) -> Op {
        match self.rng.below(1000) {
            0..300 => Op::Insert(self.spec(model)),
            300..380 => Op::Batch(self.batch(model)),
            380..420 => Op::Reparent(self.reparent(model)),
            420..440 => Op::RemoveFrame(self.insert_name()),
            440..460 => Op::Cleanup(self.query_time().max(1)),
            460..477 => Op::CleanupSameCutoff(self.query_time()),
            477..480 => Op::CleanupAll,
            480..780 => Op::Lookup {
                target: self.query_name(),
                source: self.query_name(),
                at: self.query_time(),
            },
            780..900 => Op::Across {
                target: self.query_name(),
                target_time: self.query_time(),
                source: self.query_name(),
                source_time: self.query_time(),
                fixed: self.query_name(),
            },
            _ => Op::Latest {
                target: self.query_name(),
                source: self.query_name(),
            },
        }
    }

    fn insert_name(&mut self) -> String {
        if self.rng.percent(3) {
            String::new()
        } else {
            self.rng.pick(&FRAMES).to_owned()
        }
    }

    fn query_name(&mut self) -> String {
        match self.rng.below(100) {
            0..4 => String::new(),
            4..12 => self.rng.pick(&UNREGISTERED).to_owned(),
            _ => self.rng.pick(&FRAMES).to_owned(),
        }
    }

    /// An existing edge as `(parent, child, is_static)`.
    fn existing_edge(
        &mut self,
        model: &Model,
    ) -> Option<(String, String, bool)> {
        if model.edges.is_empty() {
            return None;
        }
        let (child, edge) = model.edges.iter().nth(self.rng.index(model.edges.len()))?;
        Some((
            edge.parent.clone(),
            child.clone(),
            matches!(edge.hist, Hist::Static(_)),
        ))
    }

    fn spec(
        &mut self,
        model: &Model,
    ) -> Spec {
        let existing = if self.rng.percent(55) {
            self.existing_edge(model)
        } else {
            None
        };
        let (target, source, is_static) = match existing {
            // Mostly another sample of the edge's own kind.
            Some((parent, child, is_static)) => (parent, child, is_static ^ self.rng.percent(8)),
            None => (self.insert_name(), self.insert_name(), self.rng.percent(30)),
        };
        Spec {
            target,
            source,
            translation: self.translation(),
            rotation: self.rotation(),
            stamp: (!is_static).then(|| self.insert_time()),
        }
    }

    fn batch(
        &mut self,
        model: &Model,
    ) -> Vec<Spec> {
        let len = 2 + self.rng.index(5);
        if self.rng.percent(40) {
            // One frame's samples in ascending, descending or mixed order:
            // under a retention window the order decides whether a member
            // is older than the running newest stamp.
            let mut first = self.spec(model);
            let base = *first.stamp.get_or_insert(self.clock * self.unit);
            let order = self.rng.below(3);
            return (0..len)
                .map(|i| {
                    let step = i64::try_from(i).unwrap_or(0) * self.rng.below_i(400) * self.unit;
                    let stamp = match order {
                        0 => base.saturating_add(step),
                        1 => base.saturating_sub(step).max(0),
                        _ => base
                            .saturating_add(self.rng.below_i(800) * self.unit)
                            .saturating_sub(400 * self.unit)
                            .max(0),
                    };
                    self.stamps.push(stamp);
                    Spec {
                        translation: self.translation(),
                        rotation: self.rotation(),
                        stamp: Some(stamp),
                        ..first.clone()
                    }
                })
                .collect();
        }
        (0..len).map(|_| self.spec(model)).collect()
    }

    fn reparent(
        &mut self,
        model: &Model,
    ) -> Spec {
        let existing = if self.rng.percent(85) {
            self.existing_edge(model)
        } else {
            None
        };
        let (source, is_static) = match existing {
            Some((_, child, is_static)) => (child, is_static ^ self.rng.percent(10)),
            None => (self.insert_name(), self.rng.percent(50)),
        };
        Spec {
            target: self.insert_name(),
            source,
            translation: self.translation(),
            rotation: self.rotation(),
            stamp: (!is_static).then(|| self.insert_time()),
        }
    }

    fn insert_time(&mut self) -> i64 {
        let stamp = match self.rng.below(1000) {
            0..600 => {
                self.clock += self.rng.below_i(20);
                self.clock * self.unit
            }
            600..700 if !self.stamps.is_empty() => self.stamps[self.rng.index(self.stamps.len())],
            // Behind the clock: inside or outside a retention window.
            600..920 => (self.clock - self.rng.below_i(400)).max(0) * self.unit,
            920..995 => self.rng.below_i(self.clock + 50) * self.unit,
            // Near the top of the range both timestamp types represent.
            _ => self
                .rng
                .pick(&[i64::MAX, i64::MAX - 1, i64::MAX - 7, 1 << 62]),
        };
        if self.stamps.len() < 4096 {
            self.stamps.push(stamp);
        }
        stamp
    }

    /// An instant for a query or a cutoff: a stored instant, next to one,
    /// between two, or anywhere. Never negative: 2.4 cannot represent it.
    fn query_time(&mut self) -> i64 {
        let stored = |this: &mut Self| {
            if this.stamps.is_empty() {
                0
            } else {
                this.stamps[this.rng.index(this.stamps.len())]
            }
        };
        match self.rng.below(100) {
            0..40 => stored(self),
            40..55 => {
                let offset = self.rng.below_i(7) - 3;
                stored(self).saturating_add(offset).max(0)
            }
            55..80 => {
                let (a, b) = (stored(self), stored(self));
                let (lo, hi) = (a.min(b), a.max(b));
                lo + self.rng.below_i((hi - lo).saturating_add(1))
            }
            80..95 => self.rng.below_i(self.clock + 100) * self.unit,
            _ => self.rng.pick(&[0, 1, i64::MAX, i64::MAX - 1]),
        }
    }

    fn translation(&mut self) -> [f64; 3] {
        let rng = &mut self.rng;
        match rng.below(100) {
            0..66 => [
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
            ],
            66..76 => [
                rng.pick(&[-2.0, -1.0, 0.0, 1.0, 3.0]),
                rng.pick(&[-2.0, -1.0, 0.0, 1.0, 3.0]),
                rng.pick(&[-2.0, -1.0, 0.0, 1.0, 3.0]),
            ],
            76..84 => [0.0, 0.0, 0.0],
            84..91 => [
                rng.range(-1e6, 1e6),
                rng.range(-1e6, 1e6),
                rng.range(-1e-6, 1e-6),
            ],
            // Finite values whose sums, differences or rotations overflow.
            91..99 => {
                const EXTREME: [f64; 8] =
                    [1e308, -1e308, 1.7e308, -1.7e308, f64::MAX, 1e200, 0.0, 1.0];
                [rng.pick(&EXTREME), rng.pick(&EXTREME), rng.pick(&EXTREME)]
            }
            _ => [
                rng.pick(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]),
                0.0,
                1.0,
            ],
        }
    }

    fn unit_rotation(&mut self) -> [f64; 4] {
        loop {
            let q: [f64; 4] = core::array::from_fn(|_| self.rng.range(-1.0, 1.0));
            let n2: f64 = q.iter().map(|c| c * c).sum();
            if n2 > 1e-3 {
                let n = n2.sqrt();
                return q.map(|c| c / n);
            }
        }
    }

    fn rotation(&mut self) -> [f64; 4] {
        let scaled = |q: [f64; 4], s: f64| q.map(|c| c * s);
        match self.rng.below(100) {
            0..50 => self.unit_rotation(),
            50..62 => [1.0, 0.0, 0.0, 0.0],
            62..70 => self.rng.pick(&[
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0],
                [FRAC_1_SQRT_2, 0.0, 0.0, -FRAC_1_SQRT_2],
                [0.5, 0.5, 0.5, 0.5],
                [-1.0, 0.0, 0.0, 0.0],
            ]),
            // A unit quaternion that went through f32: off by about 1e-8.
            70..77 => self.unit_rotation().map(|c| f64::from(c as f32)),
            // Off unit, inside the 1e-6 tolerance both versions accept.
            77..91 => {
                let s = self.rng.pick(&[
                    1.0 + 1e-12,
                    1.0 - 1e-12,
                    1.0 + 1e-9,
                    1.0 - 1e-9,
                    1.0 + 3e-7,
                    1.0 - 3e-7,
                    1.0 + 9.9e-7,
                    1.0 - 9.9e-7,
                ]);
                scaled(self.unit_rotation(), s)
            }
            // Outside the tolerance.
            91..97 => {
                let s =
                    self.rng
                        .pick(&[1.0 + 1.01e-6, 1.0 - 1.01e-6, 1.001, 0.999, 2.0, 0.0, 1e200]);
                scaled(self.unit_rotation(), s)
            }
            _ => {
                let mut q = self.unit_rotation();
                q[self.rng.index(4)] = self.rng.pick(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
                q
            }
        }
    }
}
