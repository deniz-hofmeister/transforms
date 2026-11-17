#![feature(f128)]
#![allow(clippy::all)]
use std::hint::black_box;
use std::time::Instant;
use transforms::{Registry, geometry::{Quaternion, Transform, Vector3}, time::{Stamp, Timestamp}};

type Q = [f64; 4];
const EPS: f64 = f64::EPSILON;

struct Rng([u64; 4]);
impl Rng {
    fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x1234567);
        let mut s = [0u64; 4];
        for v in &mut s {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            *v = x ^ (x >> 31);
        }
        Rng(s)
    }
    #[inline] fn next(&mut self) -> u64 {
        let s = &mut self.0;
        let r = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0]; s[3] ^= s[1]; s[1] ^= s[2]; s[0] ^= s[3]; s[2] ^= t; s[3] = s[3].rotate_left(45);
        r
    }
    #[inline] fn u(&mut self) -> f64 { (self.next() >> 11) as f64 * (1.0 / 9007199254740992.0) }
    #[inline] fn range(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * self.u() }
    #[inline] fn gauss2(&mut self) -> (f64, f64) {
        let u1 = 1.0 - self.u(); let u2 = self.u();
        let r = (-2.0 * u1.ln()).sqrt(); let (s, c) = (std::f64::consts::TAU * u2).sin_cos();
        (r * c, r * s)
    }
    #[inline] fn raw4(&mut self) -> Q { let (a, b) = self.gauss2(); let (c, d) = self.gauss2(); [a, b, c, d] }
    /// constructed unit quaternion: crate's sqrt normalization of a random 4-vector
    #[inline] fn unit(&mut self) -> Q { normalize(self.raw4()) }
}

// ---- replicas of the crate's arithmetic (expression order copied from src/geometry/quaternion/mod.rs)
#[inline(always)] fn qmul(a: Q, b: Q) -> Q {
    [a[0]*b[0] - a[1]*b[1] - a[2]*b[2] - a[3]*b[3],
     a[0]*b[1] + a[1]*b[0] + a[2]*b[3] - a[3]*b[2],
     a[0]*b[2] - a[1]*b[3] + a[2]*b[0] + a[3]*b[1],
     a[0]*b[3] + a[1]*b[2] - a[2]*b[1] + a[3]*b[0]]
}
#[inline(always)] fn conj(q: Q) -> Q { [q[0], -q[1], -q[2], -q[3]] }
#[inline(always)] fn n2f(q: Q) -> f64 { q[0]*q[0] + q[1]*q[1] + q[2]*q[2] + q[3]*q[3] }
#[inline(always)] fn scale(q: Q, f: f64) -> Q { [q[0]*f, q[1]*f, q[2]*f, q[3]*f] }
#[inline(always)] fn normalize(q: Q) -> Q { scale(q, 1.0 / libm::sqrt(n2f(q))) }
#[inline(always)] fn normalize_div(q: Q) -> Q { let n = libm::sqrt(n2f(q)); [q[0]/n, q[1]/n, q[2]/n, q[3]/n] }
#[inline(always)] fn newton(q: Q) -> Q { scale(q, 0.5 * (3.0 - n2f(q))) }
#[inline(always)] fn newton_band(q: Q, k: f64) -> Q { let n2 = n2f(q); if (n2 - 1.0).abs() <= k * EPS { q } else { scale(q, 0.5 * (3.0 - n2)) } }
#[inline(always)] fn sqrt_band(q: Q, k: f64) -> Q { let n2 = n2f(q); if (n2 - 1.0).abs() <= k * EPS { q } else { scale(q, 1.0 / libm::sqrt(n2)) } }
#[inline(always)] fn rot(q: Q, v: [f64; 3]) -> [f64; 3] { let r = qmul(qmul(q, [0.0, v[0], v[1], v[2]]), conj(q)); [r[1], r[2], r[3]] }
fn bits(q: Q) -> [u64; 4] { [q[0].to_bits(), q[1].to_bits(), q[2].to_bits(), q[3].to_bits()] }

/// |q|^2 - 1 evaluated exactly (error-free products and sums; needs hardware FMA to be fast)
#[inline] fn exact_dev(q: Q) -> f64 {
    let mut s = -1.0f64; let mut c = 0.0f64;
    for x in q {
        let p = x * x; let e = x.mul_add(x, -p);
        let t = s + p; let bb = t - s; let err = (s - (t - bb)) + (p - bb);
        s = t; c += err + e;
    }
    s + c
}
/// |q| - 1 from the exact squared deviation
fn norm_dev(d: f64) -> f64 { d / ((1.0 + d).sqrt() + 1.0) }

fn par<T: Send>(threads: u64, f: impl Fn(u64) -> T + Sync) -> Vec<T> {
    std::thread::scope(|s| { let hs: Vec<_> = (0..threads).map(|i| { let f = &f; s.spawn(move || f(i)) }).collect(); hs.into_iter().map(|h| h.join().unwrap()).collect() })
}

#[derive(Clone, Copy)] enum Fix { None, Newton, NewtonBand8, Sqrt, SqrtBand8 }
const FIXES: [(Fix, &str); 5] = [(Fix::None, "none"), (Fix::Newton, "newton"), (Fix::NewtonBand8, "newton+band8"), (Fix::Sqrt, "sqrt"), (Fix::SqrtBand8, "sqrt+band8")];
#[inline(always)] fn fix(f: Fix, q: Q) -> Q { match f { Fix::None => q, Fix::Newton => newton(q), Fix::NewtonBand8 => newton_band(q, 8.0), Fix::Sqrt => normalize(q), Fix::SqrtBand8 => sqrt_band(q, 8.0) } }

fn axis_angle(ax: [f64; 3], ang: f64) -> Q {
    let n = (ax[0]*ax[0] + ax[1]*ax[1] + ax[2]*ax[2]).sqrt(); let (s, c) = (ang / 2.0).sin_cos();
    normalize([c, s*ax[0]/n, s*ax[1]/n, s*ax[2]/n])
}

// ================= (a) drift
fn drift(n: u64) {
    println!("## (a) drift: max over the run of | |q|-1 | (exact |q|^2 via error-free transforms), N={n}");
    let incs: [(&str, Q); 4] = [
        ("1e-3 rad about z", axis_angle([0.0, 0.0, 1.0], 1e-3)),
        ("1 rad about (1,2,3)", axis_angle([1.0, 2.0, 3.0], 1.0)),
        ("0.1 rad about (-3,1,2)", axis_angle([-3.0, 1.0, 2.0], 0.1)),
        ("random unit (seed 7)", Rng::new(7).unit()),
    ];
    let jobs: Vec<(usize, usize)> = (0..incs.len()).flat_map(|i| (0..FIXES.len()).map(move |j| (i, j))).collect();
    let res = par(jobs.len() as u64, |t| {
        let (i, j) = jobs[t as usize]; let d = incs[i].1; let f = FIXES[j].0;
        let mut q: Q = [1.0, 0.0, 0.0, 0.0]; let mut mx = 0.0f64;
        for _ in 0..n { q = fix(f, qmul(q, d)); let e = exact_dev(q).abs(); if e > mx { mx = e; } }
        (mx, exact_dev(q))
    });
    for (t, (mx, last)) in res.iter().enumerate() {
        let (i, j) = jobs[t];
        println!("const-inc [{}] {:<13} max||q|-1| = {:.3e}  final |q|-1 = {:+.3e}", incs[i].0, FIXES[j].1, norm_dev(*mx), norm_dev(*last));
    }
    // random walk: q <- fix(q * r_k), r_k fresh constructed
    let res = par(FIXES.len() as u64 * 4, |t| {
        let f = FIXES[(t % 5) as usize].0; let mut rng = Rng::new(1000 + t / 5);
        let mut q: Q = [1.0, 0.0, 0.0, 0.0]; let mut mx = 0.0f64;
        for _ in 0..n { q = fix(f, qmul(q, rng.unit())); let e = exact_dev(q).abs(); if e > mx { mx = e; } }
        mx
    });
    for j in 0..5 { let m = (0..4).map(|s| res[s * 5 + j]).fold(0.0, f64::max); println!("random-walk (4 seeds x N) {:<13} max||q|-1| = {:.3e}", FIXES[j].1, norm_dev(m)); }
    // independent single products of constructed operands
    let res = par(16, |t| {
        let mut rng = Rng::new(2000 + t); let mut mx = [0.0f64; 5];
        for _ in 0..n / 16 { let p = qmul(rng.unit(), rng.unit()); for j in 0..5 { let e = exact_dev(fix(FIXES[j].0, p)).abs(); if e > mx[j] { mx[j] = e; } } }
        mx
    });
    for j in 0..5 { let m = res.iter().map(|r| r[j]).fold(0.0, f64::max); println!("single product of constructed ({} samples) {:<13} max||q|-1| = {:.3e}", n / 16 * 16, FIXES[j].1, norm_dev(m)); }
}

// ================= cost
fn bench() {
    println!("## cost per composition: variants interleaved, 40 rounds, min per variant");
    let d = black_box(axis_angle([1.0, 2.0, 3.0], 1e-3));
    let mut rng = Rng::new(5); let m = 4096usize;
    let a: Vec<Q> = (0..m).map(|_| rng.unit()).collect(); let b: Vec<Q> = (0..m).map(|_| rng.unit()).collect(); let mut o = vec![[0.0; 4]; m];
    let ta: Vec<[f64; 3]> = (0..m).map(|_| [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)]).collect();
    let mut ot = vec![[0.0; 3]; m];
    let mut best = [[f64::MAX; 5]; 4]; let mut chk = [0u64; 5];
    for _ in 0..40 {
        for (j, (f, _)) in FIXES.iter().enumerate() { let f = *f;
            let n = 2_000_000u64; let t = Instant::now(); let mut q: Q = [1.0, 0.0, 0.0, 0.0]; for _ in 0..n { q = fix(f, qmul(q, black_box(d))); } let e = t.elapsed().as_secs_f64() / n as f64; best[0][j] = best[0][j].min(e); chk[j] = bits(black_box(q))[0];
            let reps = 500; let t = Instant::now(); for _ in 0..reps { for i in 0..m { o[i] = fix(f, qmul(a[i], b[i])); } black_box(&mut o); black_box(&a); } best[1][j] = best[1][j].min(t.elapsed().as_secs_f64() / (reps * m) as f64);
            let reps = 250; let t = Instant::now(); for _ in 0..reps { for i in 0..m { let r = rot(a[i], ta[i]); ot[i] = [r[0] + ta[i][0], r[1] + ta[i][1], r[2] + ta[i][2]]; o[i] = fix(f, qmul(a[i], b[i])); } black_box(&mut o); black_box(&mut ot); black_box(&a); } best[2][j] = best[2][j].min(t.elapsed().as_secs_f64() / (reps * m) as f64);
            let n = 1_000_000u64; let dt = black_box([0.1, 0.2, 0.3]); let t = Instant::now(); let mut q: Q = [1.0, 0.0, 0.0, 0.0]; let mut tr = [0.0f64; 3];
            for _ in 0..n { let r = rot(black_box(d), tr); tr = [r[0] + dt[0], r[1] + dt[1], r[2] + dt[2]]; q = fix(f, qmul(black_box(d), q)); } black_box((q, tr)); best[3][j] = best[3][j].min(t.elapsed().as_secs_f64() / n as f64);
        }
    }
    for (k, name) in ["rotation chain q=fix(q*d) (latency-bound)", "rotation, independent pairs (throughput)", "isometry product, independent pairs (throughput)", "isometry chain, registry order hop*composed (latency-bound)"].iter().enumerate() {
        for j in 0..5 { println!("{name:<60} {:<13} {:6.2} ns/op  (+{:5.2} ns vs none, x{:.2})", FIXES[j].1, best[k][j] * 1e9, (best[k][j] - best[k][0]) * 1e9, best[k][j] / best[k][0]); } }
    println!("checksums {:x?}", chk);
}

// ================= (b) fixed point
const BANDS: [f64; 8] = [0.5, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 16.0];
const NR: usize = 2 + BANDS.len();
#[inline(always)] fn rule(j: usize, q: Q) -> Q { match j { 0 => normalize(q), 1 => normalize_div(q), _ => sqrt_band(q, BANDS[j - 2]) } }
fn rule_name(j: usize) -> String { match j { 0 => "always sqrt, x*(1/n)".into(), 1 => "always sqrt, x/n".into(), _ => format!("band |n2-1|<={}EPS", BANDS[j - 2]) } }
#[derive(Clone)] struct St { n: u64, changed: [u64; NR], hist: [u64; 42], maxex: f64 }
impl St { fn new() -> Self { St { n: 0, changed: [0; NR], hist: [0; 42], maxex: 0.0 } }
    #[inline] fn feed(&mut self, q: Q) { self.n += 1; let b = bits(q); for j in 0..NR { if bits(rule(j, q)) != b { self.changed[j] += 1; } }
        let h = ((n2f(q) - 1.0).abs() / (EPS / 2.0)).round(); self.hist[if h > 41.0 { 41 } else { h as usize }] += 1; let e = exact_dev(q).abs(); if e > self.maxex { self.maxex = e; } }
    fn merge(v: Vec<St>) -> St { let mut s = St::new(); for x in v { s.n += x.n; for j in 0..NR { s.changed[j] += x.changed[j]; } for j in 0..42 { s.hist[j] += x.hist[j]; } s.maxex = s.maxex.max(x.maxex); } s }
    fn print(&self, name: &str) {
        println!("### population: {name}  (n={})", self.n);
        let top = (0..42).rev().find(|&i| self.hist[i] > 0).unwrap_or(0);
        println!("  max float-computed |n2-1| = {} EPS ; max exact ||q|^2-1| = {:.3} EPS", top as f64 / 2.0, self.maxex / EPS);
        print!("  hist of float |n2-1| in EPS/2 units:"); for i in 0..=top { print!(" {}:{:.2e}", i, self.hist[i] as f64 / self.n as f64); } println!();
        for j in 0..NR { println!("  rule [{:<22}] changes {:>11} = {:.6}%", rule_name(j), self.changed[j], 100.0 * self.changed[j] as f64 / self.n as f64); }
    }
}
fn cq(q: Q) -> Quaternion { Quaternion::from_wxyz(q[0], q[1], q[2], q[3]) }
fn qc(q: Quaternion) -> Q { [q.w, q.x, q.y, q.z] }
fn near(rng: &mut Rng, a: Q) -> Q { // a times a rotation of log-uniform angle 1e-9..pi about a random axis; constructed (normalized)
    let ang = 10f64.powf(rng.range(-9.0, 0.497)); let (x, y) = rng.gauss2(); let (z, _) = rng.gauss2();
    normalize(qmul(a, axis_angle([x, y, z], ang)))
}
fn fixedpoint(n: u64) {
    println!("## (b) fixed point of reconstruction, n={n} per population, 16 threads");
    let th = 16u64; let per = n / th;
    let pops: Vec<(&str, Box<dyn Fn(&mut Rng) -> Q + Sync>)> = vec![
        ("constructed: normalize(random 4-vector, any scale)", Box::new(|r| r.unit())),
        ("constructed: normalize(unit*(1+u*1e-6)) [2.4-acceptable inputs]", Box::new(|r| { let u = r.unit(); let s = 1.0 + r.range(-1e-6, 1e-6); normalize(scale(u, s)) })),
        ("constructed with x/n division", Box::new(|r| normalize_div(r.raw4()))),
        ("constructed via band8 rule from unit*(1+u*1e-6)", Box::new(|r| { let u = r.unit(); let s = 1.0 + r.range(-1e-6, 1e-6); sqrt_band(scale(u, s), 8.0) })),
        ("raw product a*b (no renorm) [2.4 two-hop]", Box::new(|r| qmul(r.unit(), r.unit()))),
        ("newton(a*b)", Box::new(|r| newton(qmul(r.unit(), r.unit())))),
        ("newton_band8(a*b)", Box::new(|r| newton_band(qmul(r.unit(), r.unit()), 8.0))),
        ("newton chain of 5 products", Box::new(|r| { let mut q = r.unit(); for _ in 0..5 { q = newton(qmul(q, r.unit())); } q })),
        ("raw chain of 5 products", Box::new(|r| { let mut q = r.unit(); for _ in 0..5 { q = qmul(q, r.unit()); } q })),
        ("crate slerp(a, near(a), t) raw", Box::new(|r| { let a = r.unit(); let b = near(r, a); qc(cq(a).slerp(cq(b), r.u())) })),
        ("newton(crate slerp(a, near(a), t))", Box::new(|r| { let a = r.unit(); let b = near(r, a); newton(qc(cq(a).slerp(cq(b), r.u()))) })),
        ("newton(crate slerp(a, random b, t))", Box::new(|r| { let a = r.unit(); let b = r.unit(); newton(qc(cq(a).slerp(cq(b), r.u()))) })),
        ("newton(newton(a*b)) i.e. second step", Box::new(|r| newton(newton(qmul(r.unit(), r.unit()))))),
    ];
    for (pi, (name, g)) in pops.iter().enumerate() {
        let s = St::merge(par(th, |t| { let mut rng = Rng::new(31 * pi as u64 + 100 * t + 1); let mut s = St::new(); for _ in 0..per { s.feed(g(&mut rng)); } s }));
        s.print(name);
    }
    // idempotence of each rule on raw (unnormalized) inputs: R(R(x)) == R(x)?
    for (name, lo, hi) in [("raw unit*(1+u*1e-6)", -1e-6, 1e-6), ("raw unit*(1+u*1e-12)", -1e-12, 1e-12), ("raw unit*(1+u*4e-15)", -4e-15, 4e-15)] {
        let r = par(th, |t| { let mut rng = Rng::new(777 + t); let mut c = [0u64; NR];
            for _ in 0..per { let u = rng.unit(); let x = scale(u, 1.0 + rng.range(lo, hi)); for j in 0..NR { let y = rule(j, x); if bits(rule(j, y)) != bits(y) { c[j] += 1; } } } c });
        println!("### idempotence R(R(x)) != R(x) on {name} (n={})", per * th);
        for j in 0..NR { let c: u64 = r.iter().map(|x| x[j]).sum(); println!("  rule [{:<22}] not idempotent {:>11} = {:.6}%", rule_name(j), c, 100.0 * c as f64 / (per * th) as f64); }
    }
    // Newton specifics
    let r = par(th, |t| { let mut rng = Rng::new(4242 + t); let mut c = [0u64; 8];
        for _ in 0..per { let a = rng.unit(); let b = rng.unit(); let p = qmul(a, b); let pn = newton(p);
            if bits(pn) != bits(p) { c[0] += 1; }                              // newton changes a two-hop product
            if exact_dev(pn).abs() > exact_dev(p).abs() { c[1] += 1; }          // newton made it worse
            if bits(newton(pn)) != bits(pn) { c[2] += 1; }                      // newton not idempotent
            if bits(newton(qmul(a, [1.0, 0.0, 0.0, 0.0]))) != bits(a) { c[3] += 1; } // a*identity != a under newton
            if bits(newton_band(qmul(a, [1.0, 0.0, 0.0, 0.0]), 8.0)) != bits(a) { c[4] += 1; }
            if bits(newton_band(p, 8.0)) != bits(p) { c[5] += 1; }
            if exact_dev(pn).abs() > exact_dev(a).abs().max(exact_dev(b).abs()) { c[6] += 1; }
            if bits(qmul(a, conj(a))) != bits([1.0, 0.0, 0.0, 0.0]) && bits(newton(qmul(a, conj(a)))) != bits([1.0, 0.0, 0.0, 0.0]) { c[7] += 1; }
        } c });
    let tot = (per * th) as f64; let s = |i: usize| -> f64 { 100.0 * r.iter().map(|x| x[i]).sum::<u64>() as f64 / tot };
    println!("### newton specifics over constructed a,b (n={})", per * th);
    println!("  newton(a*b) != a*b bitwise: {:.4}%", s(0));
    println!("  exact ||q|^2-1| larger after the step than before: {:.4}%", s(1));
    println!("  newton(newton(a*b)) != newton(a*b): {:.4}%", s(2));
    println!("  newton(a*identity) != a: {:.4}%", s(3));
    println!("  newton_band8(a*identity) != a: {:.4}%", s(4));
    println!("  newton_band8(a*b) != a*b: {:.4}%", s(5));
    println!("  newton(a*b) further from unit than the worse operand: {:.4}%", s(6));
    println!("  newton(a*conj(a)) != identity bitwise (and raw also not): {:.4}%", s(7));
}

// ================= (c) inverse
#[derive(Clone, Copy)] struct Iso { t: [f64; 3], q: Q }
#[inline(always)] fn imul(a: Iso, b: Iso, f: Fix) -> Iso { let r = rot(a.q, b.t); Iso { t: [r[0] + a.t[0], r[1] + a.t[1], r[2] + a.t[2]], q: fix(f, qmul(a.q, b.q)) } }
#[inline(always)] fn iinv(a: Iso, norm: bool) -> Iso { iinv3(a, norm as u8) }
#[inline(always)] fn iinv3(a: Iso, mode: u8) -> Iso { let q = if mode == 1 { conj(normalize(a.q)) } else { conj(a.q) }; let r = rot(q, a.t); let s = if mode == 2 { -1.0 / n2f(a.q) } else { -1.0 }; Iso { t: [s * r[0], s * r[1], s * r[2]], q } }
fn prop_quat(rng: &mut Rng) -> Q { loop { let (x, y, z) = (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)); let ang = rng.range(0.0, std::f64::consts::TAU);
    let n = (x*x + y*y + z*z).sqrt(); if n < 1e-3 { continue; } let h = ang / 2.0; let s = h.sin() / n; return normalize([h.cos(), s*x, s*y, s*z]); } }
fn prop_links(rng: &mut Rng) -> Vec<Iso> { let k = 2 + (rng.next() % 4) as usize; (0..k).map(|_| { let t = [rng.range(-1e5, 1e5), rng.range(-1e5, 1e5), rng.range(-1e5, 1e5)]; Iso { t, q: prop_quat(rng) } }).collect() }
/// registry order (compose_chain): walk upward from the leaf, `hop * composed`
fn fold(l: &[Iso], f: Fix) -> Iso { let mut a = l[l.len() - 1]; for x in l[..l.len() - 1].iter().rev() { a = imul(*x, a, f); } a }
// f128 reference
type F = f128;
fn sqrt128(x: F) -> F { let mut s = (x as f64).sqrt() as F; for _ in 0..3 { s = (s + x / s) / 2.0; } s }
fn qmul128(a: [F; 4], b: [F; 4]) -> [F; 4] { [a[0]*b[0] - a[1]*b[1] - a[2]*b[2] - a[3]*b[3], a[0]*b[1] + a[1]*b[0] + a[2]*b[3] - a[3]*b[2], a[0]*b[2] - a[1]*b[3] + a[2]*b[0] + a[3]*b[1], a[0]*b[3] + a[1]*b[2] - a[2]*b[1] + a[3]*b[0]] }
/// exact (to f128) inverse translation of the rigid transform the f64 value `a` denotes (rotation = q/|q|)
fn inv_t128(a: Iso) -> [F; 3] { let q = a.q.map(|x| x as F); let n = sqrt128(q[0]*q[0] + q[1]*q[1] + q[2]*q[2] + q[3]*q[3]); let qc = [q[0]/n, -q[1]/n, -q[2]/n, -q[3]/n];
    let r = qmul128(qmul128(qc, [0.0, a.t[0] as F, a.t[1] as F, a.t[2] as F]), [qc[0], -qc[1], -qc[2], -qc[3]]); [-r[1], -r[2], -r[3]] }
const NV: usize = 9;
const VARS: [(&str, Fix, u8); NV] = [("2.4: raw products, normalize+conj", Fix::None, 1), ("raw products, conjugate", Fix::None, 0), ("newton products, conjugate", Fix::Newton, 0), ("newton+band8 products, conjugate", Fix::NewtonBand8, 0), ("newton products, normalize+conj", Fix::Newton, 1), ("sqrt products, conjugate", Fix::Sqrt, 0), ("newton+band8 products, normalize+conj", Fix::NewtonBand8, 1), ("raw products, conjugate with t/n2", Fix::None, 2), ("newton+band8 products, conjugate with t/n2", Fix::NewtonBand8, 2)];
const THR: [f64; 5] = [2.5e-10, 5e-10, 1e-9, 2e-9, 4e-9];
fn inverse(n: u64) {
    println!("## (c) inverse");
    // 1. validate the replica against the crate's registry, bit for bit
    { let mut rng = Rng::new(99); let mut ok_l = 0; let mut ok_b = 0; let m = 20000;
      for _ in 0..m { let links = prop_links(&mut rng); let ts = Timestamp::from_nanos(5); let mut reg = Registry::new();
        for (i, l) in links.iter().enumerate() { reg.add_transform(Transform::new(&format!("f{i}"), &format!("f{}", i + 1), Vector3::new(l.t[0], l.t[1], l.t[2]), cq(l.q), Stamp::At(ts)).unwrap()).unwrap(); }
        let leaf = format!("f{}", links.len()); let fw = reg.get_transform("f0", &leaf, ts).unwrap(); let bw = reg.get_transform(&leaf, "f0", ts).unwrap();
        let f = fold(&links, Fix::None); let b = iinv(f, true);
        let same = |i: Iso, t: &Transform<Timestamp>| bits(i.q) == bits(qc(t.rotation())) && i.t.map(f64::to_bits) == [t.translation().x.to_bits(), t.translation().y.to_bits(), t.translation().z.to_bits()];
        if same(f, &fw) { ok_l += 1; } if same(b, &bw) { ok_b += 1; } }
      println!("replica check vs crate Registry over {m} chains: forward bit-equal {ok_l}, backward bit-equal {ok_b}"); }
    // 2. property replica: forward * backward
    let th = 16u64; let per = n / th;
    let res = par(th, |t| { let mut rng = Rng::new(31337 + t); let mut st = [[0f64; 10]; NV]; // fails, max, sum, thr0..4, rotfails, max by d|t|
        for _ in 0..per { let links = prop_links(&mut rng);
            for (v, (_, f, norm)) in VARS.iter().enumerate() { let fw = fold(&links, *f); let bw = iinv3(fw, *norm); let c = imul(fw, bw, *f);
                let e = c.t[0].abs().max(c.t[1].abs()).max(c.t[2].abs()); let s = &mut st[v];
                if e > 1e-9 { s[0] += 1.0; } if e > s[1] { s[1] = e; } s[2] += e; for k in 0..5 { if e > THR[k] { s[3 + k] += 1.0; } }
                if !((c.q[0].abs() - 1.0).abs() < 1e-9) { s[8] += 1.0; }
                let rel = e / (fw.t[0].abs().max(fw.t[1].abs()).max(fw.t[2].abs())); if rel > s[9] { s[9] = rel; } } }
        st });
    let tot = (per * th) as f64;
    println!("property replica (links 2..=5 uniform, |t_i|<=1e5/axis, tolerance 1e-9 absolute), cases = {}", per * th);
    for (v, (name, _, _)) in VARS.iter().enumerate() { let g = |i: usize| res.iter().map(|r| r[v][i]).sum::<f64>(); let mx = |i: usize| res.iter().map(|r| r[v][i]).fold(0.0, f64::max);
        println!("  [{name}] fail(>1e-9) {:.0} = {:.5}% ; rot fails {:.0}; max err {:.3e} m; mean err {:.3e} m; max err/|t|max {:.2} EPS", g(0), 100.0 * g(0) / tot, g(8), mx(1), g(2) / tot, mx(9) / EPS);
        print!("     P(err > thr):"); for k in 0..5 { print!("  {:.1e}: {:.5}%", THR[k], 100.0 * g(3 + k) / tot); } println!(); }
    // 3. true error against f128 exact inverse
    let m = 2_000_000u64 / th;
    let res = par(th, |t| { let mut rng = Rng::new(555 + t); let mut st = [[0f64; 4]; NV]; // max abs, sum sq abs, max rel(|t|), sumsq rel
        for _ in 0..m { let links = prop_links(&mut rng);
            for (v, (_, f, norm)) in VARS.iter().enumerate() { let fw = fold(&links, *f); let bw = iinv3(fw, *norm); let ex = inv_t128(fw);
                let mut e2: F = 0.0; let mut n2: F = 0.0; for i in 0..3 { let d = bw.t[i] as F - ex[i]; e2 += d * d; n2 += ex[i] * ex[i]; }
                let e = (e2 as f64).sqrt(); let r = e / (n2 as f64).sqrt(); let s = &mut st[v]; if e > s[0] { s[0] = e; } s[1] += e * e; if r > s[2] { s[2] = r; } s[3] += r * r; } }
        st });
    println!("true error of the backward translation vs exact (f128) inverse of the f64 forward value, {} chains", m * th);
    for (v, (name, _, _)) in VARS.iter().enumerate() { let g = |i: usize| res.iter().map(|r| r[v][i]).sum::<f64>(); let mx = |i: usize| res.iter().map(|r| r[v][i]).fold(0.0, f64::max);
        println!("  [{name}] abs: max {:.3e} m rms {:.3e} m ; relative to |t|: max {:.2} EPS rms {:.2} EPS", mx(0), (g(1) / (m * th) as f64).sqrt(), mx(2) / EPS, (g(3) / (m * th) as f64).sqrt() / EPS); }
    // 4. single transform t * t.inverse(), unit-scale translations
    let res = par(th, |t| { let mut rng = Rng::new(888 + t); let mut st = [[0f64; 3]; 2];
        for _ in 0..per { let a = Iso { t: [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)], q: rng.unit() };
            for (v, norm) in [true, false].iter().enumerate() { let c = imul(a, iinv(a, *norm), Fix::None); let e = c.t[0].abs().max(c.t[1].abs()).max(c.t[2].abs()); let s = &mut st[v]; if e > s[0] { s[0] = e; } s[1] += e;
                let re = (c.q[1]*c.q[1] + c.q[2]*c.q[2] + c.q[3]*c.q[3]).sqrt() * 2.0; if re > s[2] { s[2] = re; } } }
        st });
    for (v, name) in ["normalize+conj", "conjugate"].iter().enumerate() { println!("  single constructed t*t.inverse(), |t_i|<=10, n={}: [{name}] max |transl| {:.3e}, mean {:.3e}, max rot angle {:.3e} rad", per * th, res.iter().map(|r| r[v][0]).fold(0.0, f64::max), res.iter().map(|r| r[v][1]).sum::<f64>() / tot, res.iter().map(|r| r[v][2]).fold(0.0, f64::max)); }
    // 5. bits changed in lookups by each variant relative to 2.4
    let res = par(th, |t| { let mut rng = Rng::new(4711 + t); let mut c = [[0u64; 4]; NV];
        for _ in 0..per { let links = prop_links(&mut rng); let f0 = fold(&links, Fix::None); let b0 = iinv(f0, true);
            for (v, (_, f, norm)) in VARS.iter().enumerate() { let fw = fold(&links, *f); let bw = iinv3(fw, *norm);
                if fw.t.map(f64::to_bits) != f0.t.map(f64::to_bits) { c[v][0] += 1; } if bits(fw.q) != bits(f0.q) { c[v][1] += 1; }
                if bw.t.map(f64::to_bits) != b0.t.map(f64::to_bits) { c[v][2] += 1; } if bits(bw.q) != bits(b0.q) { c[v][3] += 1; } } }
        c });
    println!("lookup bits changed relative to 2.4 (same chains as the property), cases = {}", per * th);
    for (v, (name, _, _)) in VARS.iter().enumerate() { let g = |i: usize| 100.0 * res.iter().map(|r| r[v][i]).sum::<u64>() as f64 / tot;
        println!("  [{name}] forward: translation {:.3}% rotation {:.3}% ; backward: translation {:.3}% rotation {:.3}%", g(0), g(1), g(2), g(3)); }
    // 5b. how often does normalize change a constructed / product quaternion (i.e. bits of inverse differ)
}

// ================= (d) slerp
fn slerp(n: u64) {
    println!("## (d) crate slerp (transforms::geometry::Quaternion::slerp at the path dependency)");
    let p = |name: &str, a: Quaternion, b: Quaternion, t: f64| { let r = a.slerp(b, t); println!("  {name}: slerp({:?},{:?},{t}) = [{:e},{:e},{:e},{:e}] norm-1 = {:.3e}", qc(a), qc(b), r.w, r.x, r.y, r.z, norm_dev(exact_dev(qc(r)))); };
    let id = Quaternion::identity(); let h = std::f64::consts::FRAC_1_SQRT_2;
    p("identical", id, id, 0.3);
    p("q vs -q (same rotation)", id, Quaternion::from_wxyz(-1.0, 0.0, 0.0, 0.0), 0.3);
    p("q vs -q, t=1 (returns self-sign, not other)", Quaternion::from_wxyz(h, h, 0.0, 0.0), Quaternion::from_wxyz(-h, -h, 0.0, 0.0), 1.0);
    p("180 deg apart, dot=+0", id, Quaternion::from_wxyz(0.0, 1.0, 0.0, 0.0), 0.5);
    p("180 deg apart, dot=+1e-17", id, Quaternion::from_wxyz(1e-17, 1.0, 0.0, 0.0), 0.5);
    p("180 deg apart, dot=-1e-17", id, Quaternion::from_wxyz(-1e-17, 1.0, 0.0, 0.0), 0.5);
    p("dot=1+2e-6 (both norm 1+1e-6, identical)", Quaternion::from_wxyz(1.0 + 1e-6, 0.0, 0.0, 0.0), Quaternion::from_wxyz(1.0 + 1e-6, 0.0, 0.0, 0.0), 0.5);
    p("dot=1-2e-6 (both norm 1-1e-6, identical)", Quaternion::from_wxyz(1.0 - 1e-6, 0.0, 0.0, 0.0), Quaternion::from_wxyz(1.0 - 1e-6, 0.0, 0.0, 0.0), 0.5);
    p("dot=1-EPS exactly (trig branch, theta=2.1e-8)", id, Quaternion::from_wxyz(1.0 - EPS, 0.0, 0.0, 0.0), 0.5);
    p("dot=1-EPS/2 (lerp branch)", id, Quaternion::from_wxyz(1.0 - EPS / 2.0, 0.0, 0.0, 0.0), 0.5);
    p("zero quaternions", Quaternion::from_wxyz(0.0, 0.0, 0.0, 0.0), Quaternion::from_wxyz(0.0, 0.0, 0.0, 0.0), 0.5);
    p("zero self, unit other", Quaternion::from_wxyz(0.0, 0.0, 0.0, 0.0), id, 0.5);
    p("tiny self, huge other, t=0 (lerp branch norm<EPS)", Quaternion::from_wxyz(1e-17, 0.0, 0.0, 0.0), Quaternion::from_wxyz(1e17, 0.0, 0.0, 0.0), 0.0);
    p("t = NaN", id, Quaternion::from_wxyz(h, h, 0.0, 0.0), f64::NAN);
    p("t = 2 (clamped)", id, Quaternion::from_wxyz(h, h, 0.0, 0.0), 2.0);
    p("NaN component", id, Quaternion::from_wxyz(f64::NAN, 0.0, 0.0, 0.0), 0.5);
    p("inf component", id, Quaternion::from_wxyz(f64::INFINITY, 0.0, 0.0, 0.0), 0.5);
    p("1e200 components (dot overflows)", Quaternion::from_wxyz(1e200, 0.0, 0.0, 0.0), Quaternion::from_wxyz(1e200, 0.0, 0.0, 0.0), 0.5);
    p("-0.0 component at t=0", Quaternion::from_wxyz(h, -0.0, h, 0.0), Quaternion::from_wxyz(0.0, 1.0, 0.0, 0.0), 0.0);
    let th = 16u64; let per = n / th;
    let res = par(th, |t| { let mut rng = Rng::new(60 + t); let mut c = [0f64; 12];
        for _ in 0..per { let a = rng.unit(); let b = if rng.next() & 1 == 0 { near(&mut rng, a) } else { rng.unit() }; let tt = rng.u();
            let r = qc(cq(a).slerp(cq(b), tt)); if r.iter().any(|x| !x.is_finite()) { c[0] += 1.0; }
            let e = exact_dev(r).abs(); if e > c[1] { c[1] = e; }
            let dot = a[0]*b[0] + a[1]*b[1] + a[2]*b[2] + a[3]*b[3];
            if dot > 1.0 { c[2] += 1.0; } if dot.abs() > 1.0 - EPS { c[3] += 1.0; }
            let s = qc(cq(a).slerp(cq(a), tt)); if bits(s) != bits(a) { c[4] += 1.0; } let es = exact_dev(s).abs(); if es > c[5] { c[5] = es; }
            if n2f(a) > 1.0 { c[6] += 1.0; } if n2f(a) < 1.0 - EPS / 2.0 { c[11] += 1.0; }
            let r0 = qc(cq(a).slerp(cq(b), 0.0)); if bits(r0) != bits(a) { c[7] += 1.0; }
            let r1 = qc(cq(a).slerp(cq(b), 1.0)); if bits(r1) != bits(b) { c[8] += 1.0; if bits(r1) != bits(scale(b, -1.0)) { c[9] += 1.0; } }
            let en = exact_dev(newton(r)).abs(); if en > c[10] { c[10] = en; }
        } c });
    let tot = (per * th) as f64; let g = |i: usize| res.iter().map(|r| r[i]).sum::<f64>(); let mx = |i: usize| res.iter().map(|r| r[i]).fold(0.0, f64::max);
    println!("  random scan n={} (constructed unit a; b = half near(a) log-uniform angle 1e-9..pi, half random; t uniform [0,1))", per * th);
    println!("  non-finite results: {}", g(0));
    println!("  max | |slerp|-1 |: {:.3e}   after one newton step: {:.3e}", norm_dev(mx(1)), norm_dev(mx(10)));
    println!("  computed dot > 1 (silently clamped): {:.4}% ; |dot| > 1-EPS (lerp branch): {:.4}%", 100.0 * g(2) / tot, 100.0 * g(3) / tot);
    println!("  slerp(a,a,t) != a bitwise: {:.4}% ; max | |slerp(a,a,t)|-1 | {:.3e} ; self-dot>1: {:.3}% ; self-dot<=1-EPS (trig branch on identical inputs): {:.3}%", 100.0 * g(4) / tot, norm_dev(mx(5)), 100.0 * g(6) / tot, 100.0 * g(11) / tot);
    println!("  slerp(a,b,0) != a bitwise: {:.4}% ; slerp(a,b,1) != b bitwise: {:.4}% (of which neither b nor -b: {:.4}%)", 100.0 * g(7) / tot, 100.0 * g(8) / tot, 100.0 * g(9) / tot);
    // 2.4-acceptable non-unit inputs (norm within 1e-6): how far can the output norm go
    let res = par(th, |t| { let mut rng = Rng::new(160 + t); let mut c = [0f64; 2];
        for _ in 0..per / 10 { let a0 = rng.unit(); let b0 = if rng.next() & 1 == 0 { near(&mut rng, a0) } else { rng.unit() }; let a = scale(a0, 1.0 + rng.range(-1e-6, 1e-6)); let b = scale(b0, 1.0 + rng.range(-1e-6, 1e-6));
            let r = qc(cq(a).slerp(cq(b), rng.u())); let e = norm_dev(exact_dev(r)).abs(); if e > c[0] { c[0] = e; } if r.iter().any(|x| !x.is_finite()) { c[1] += 1.0; } } c });
    println!("  inputs with norms in 1 +- 1e-6 (2.4 tolerance): max | |slerp|-1 | = {:.3e}, non-finite {}", res.iter().map(|r| r[0]).fold(0.0, f64::max), res.iter().map(|r| r[1]).sum::<f64>());
}

// ================= (e) overflow
fn overflow() {
    println!("## (e) translation overflow");
    let th = 16u64; let per = 20_000_000u64 / th;
    let res = par(th, |t| { let mut rng = Rng::new(9000 + t); let mut c = [0u64; 4];
        for _ in 0..per { let k = 2 + (rng.next() % 5) as usize; let mut acc: Option<Iso> = None; let mut seen = false;
            for _ in 0..k { let mag = 10f64.powf(rng.range(300.0, 308.25)); let mut tr = [0.0; 3]; for x in &mut tr { *x = match rng.next() % 4 { 0 => 0.0, 1 => mag, 2 => -mag, _ => rng.range(-mag, mag) }; }
                let q = match rng.next() % 3 { 0 => [1.0, 0.0, 0.0, 0.0], 1 => axis_angle([0.0, 0.0, 1.0], std::f64::consts::PI * (rng.next() % 4) as f64 / 2.0), _ => rng.unit() };
                let l = Iso { t: tr, q }; let l = if rng.next() % 4 == 0 { iinv(l, false) } else { l };
                acc = Some(match acc { None => l, Some(a) => imul(a, l, Fix::Newton) }); let a = acc.unwrap(); if a.t.iter().any(|x| !x.is_finite()) { seen = true; } }
            let a = acc.unwrap(); let fin = a.t.iter().all(|x| x.is_finite()); let anyfin = a.t.iter().any(|x| x.is_finite());
            if seen { c[0] += 1; if fin { c[1] += 1; } if anyfin { c[2] += 1; } } if a.q.iter().any(|x| !x.is_finite()) { c[3] += 1; } } c });
    let g = |i: usize| res.iter().map(|r| r[i]).sum::<u64>();
    println!("  {} random chains near f64::MAX: {} had a non-finite intermediate translation; of those, final all-finite: {}, final with any finite component: {}; non-finite rotation: {}", per * th, g(0), g(1), g(2), g(3));
    // false alarm: true result representable but intermediate overflows
    let a = Iso { t: [1.0e308, 0.0, 0.0], q: [1.0, 0.0, 0.0, 0.0] }; let b = Iso { t: [1.0e308, 0.0, 0.0], q: [1.0, 0.0, 0.0, 0.0] }; let c = Iso { t: [-1.5e308, 0.0, 0.0], q: [1.0, 0.0, 0.0, 0.0] };
    println!("  (a*b)*c with x = 1.5e308, 1.5e308, -1.5e308 (true 5e307): left fold {:?} ; right fold {:?}", imul(imul(a, b, Fix::None), c, Fix::None).t, imul(a, imul(b, c, Fix::None), Fix::None).t);
    // rotate_vector intermediate overflow below f64::MAX: single hop
    let q = axis_angle([1.0, 1.0, 1.0], 1.0); for m in [1.0e308, 6e307, 3e307, 1e307] { let v = [m, m, m]; let r = rot(q, v); println!("  rotate (m,m,m), m={m:e} by 1 rad about (1,1,1): {:?}", r); }
    let q = axis_angle([1.0, 2.0, 3.0], 2.0); for m in [1.7e308, 1e308, 6e307, 4.4e307] { let v = [m, -m, m]; let r = rot(q, v); println!("  rotate (m,-m,m), m={m:e} by 2 rad about (1,2,3): {:?}", r); }
    // squaring growth outside the registry
    let mut tq = Iso { t: [1.0, 0.0, 0.0], q: [1.0, 0.0, 0.0, 0.0] }; let mut k = 0; while tq.t[0].is_finite() { tq = imul(tq, tq, Fix::Newton); k += 1; } println!("  public total `*`: t = t*t from |t|=1 reaches inf after {k} products");
}

// ================= (f) basin
fn basin() {
    println!("## (f) newton basin: norm map g(n) = n(3-n^2)/2 applied to q = (n,0,0,0)*random unit");
    let u = Rng::new(3).unit();
    for n0 in [1e-300, 1e-9, 0.1, 0.5, 0.9, 0.999999, 1.000001, 1.1, 1.3, 1.5, 1.56, 1.57, 1.7, 1.7320508075688772, 1.8, 2.0, 2.2, 2.236, 2.2360679774997896, 2.2361, 2.3, 3.0] {
        let mut q = scale(u, n0); let one = norm_dev(exact_dev(newton(q))).abs(); let mut steps = 0; let mut state = "no convergence in 2000";
        for i in 1..=2000 { q = newton(q); let n = n2f(q); if !n.is_finite() { state = "DIVERGED (non-finite)"; steps = i; break; } if n == 0.0 { state = "COLLAPSED to zero"; steps = i; break; } if (n - 1.0).abs() <= 4.0 * EPS { state = "converged"; steps = i; break; } }
        println!("  |q0| = {n0:<22e} | |q0|-1 | = {:.3e}; after 1 step {:.3e} ({}) ; {state} after {steps} steps", (n0 - 1.0f64).abs(), one, if one < (n0 - 1.0f64).abs() { "better" } else { "WORSE" });
    }
    // residual after one step as function of input deviation
    for e in [1e-6, 1e-7, 3e-8, 1.5e-8, 1e-8, 1e-9, 1e-12, 1e-15] { let mut mx = 0.0f64; let mut rng = Rng::new(11); for _ in 0..200000 { let q = scale(rng.unit(), 1.0 + e * if rng.next() & 1 == 0 { 1.0 } else { -1.0 }); mx = mx.max(norm_dev(exact_dev(newton(q))).abs()); } println!("  input | |q|-1 | = {e:e}: max after one step = {:.3e} (theory 1.5 e^2 = {:.3e})", mx, 1.5 * e * e); }
}

fn drift9(n: u64) {
    println!("## (a) const-increment drift, N={n}, 3 increments x (newton, newton+band8, sqrt, none)");
    let incs: [(&str, Q); 3] = [("1e-3 rad about z", axis_angle([0.0, 0.0, 1.0], 1e-3)), ("1 rad about (1,2,3)", axis_angle([1.0, 2.0, 3.0], 1.0)), ("random unit (seed 7)", Rng::new(7).unit())];
    let fx = [1usize, 2, 3, 0];
    let res = par(12, |t| { let d = incs[(t / 4) as usize].1; let f = FIXES[fx[(t % 4) as usize]].0; let mut q: Q = [1.0, 0.0, 0.0, 0.0]; let mut mx = 0.0f64;
        for _ in 0..n { q = fix(f, qmul(q, d)); let e = exact_dev(q).abs(); if e > mx { mx = e; } } (mx, exact_dev(q)) });
    for (t, (mx, last)) in res.iter().enumerate() { println!("const-inc [{}] {:<13} max||q|-1| = {:.3e}  final |q|-1 = {:+.3e}", incs[t / 4].0, FIXES[fx[t % 4]].1, norm_dev(*mx), norm_dev(*last)); }
}
/// direction error of long constant-increment chains against an f128 reference chain
fn dir(n: u64) {
    println!("## rotation-direction error after N={n} constant-increment products vs f128 reference (angle between rotations, rad)");
    let incs: [(&str, Q); 3] = [("1e-3 rad about z", axis_angle([0.0, 0.0, 1.0], 1e-3)), ("1 rad about (1,2,3)", axis_angle([1.0, 2.0, 3.0], 1.0)), ("random unit (seed 7)", Rng::new(7).unit())];
    let res = par(3, |t| { let d = incs[t as usize].1; let d128 = d.map(|x| x as F); let mut r: [F; 4] = [1.0, 0.0, 0.0, 0.0]; let mut q = [[1.0, 0.0, 0.0, 0.0]; 5];
        for i in 0..n { r = qmul128(r, d128); if i % 1024 == 0 { let n2 = r[0]*r[0] + r[1]*r[1] + r[2]*r[2] + r[3]*r[3]; let k = 1.0 / sqrt128(n2); r = r.map(|x| x * k); } for j in 0..5 { q[j] = fix(FIXES[j].0, qmul(q[j], d)); } }
        let rn = sqrt128(r[0]*r[0] + r[1]*r[1] + r[2]*r[2] + r[3]*r[3]); let mut out = [0f64; 5];
        for j in 0..5 { let qq = q[j].map(|x| x as F); let qn = sqrt128(qq[0]*qq[0] + qq[1]*qq[1] + qq[2]*qq[2] + qq[3]*qq[3]); let e = qmul128([r[0], -r[1], -r[2], -r[3]], qq); let v = sqrt128(e[1]*e[1] + e[2]*e[2] + e[3]*e[3] + 1e-70) / (rn * qn); out[j] = 2.0 * (v as f64); } out });
    for (t, o) in res.iter().enumerate() { print!("  [{}]", incs[t].0); for j in 0..5 { print!("  {}: {:.3e}", FIXES[j].1, o[j]); } println!(); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect(); let n: u64 = a.get(2).map(|s| s.parse::<f64>().unwrap() as u64).unwrap_or(100_000_000);
    match a[1].as_str() { "drift" => drift(n), "dir" => dir(n), "drift9" => drift9(n), "bench" => bench(), "fixed" => fixedpoint(n), "inverse" => inverse(n), "slerp" => slerp(n), "overflow" => overflow(), "basin" => basin(), _ => panic!() }
}
