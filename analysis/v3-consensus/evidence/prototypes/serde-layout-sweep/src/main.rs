use serde::Deserialize;
use std::collections::BTreeMap;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
enum StampSA { Static, At(i64) }
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
enum StampAS { At(i64), Static }

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum F { Tg, Sr, Tr, Ro, St }
fn fname(f: F) -> &'static str {
    match f { F::Tg => "target", F::Sr => "source", F::Tr => "translation", F::Ro => "rotation", F::St => "stamp" }
}

#[derive(Debug, Default)]
struct V3 { target: String, source: String, t: [f64; 3], r: [f64; 4], stamp: Option<Option<i64>> }

// Decode sequentially with postcard (a postcard struct is the concatenation of its fields).
fn decode(order: &[F; 5], at_first: bool, mut b: &[u8], strict: bool) -> Result<V3, String> {
    let mut v = V3::default();
    for f in order {
        match f {
            F::Tg | F::Sr => {
                let (s, rest) = postcard::take_from_bytes::<String>(b).map_err(|e| format!("{}: postcard {e}", fname(*f)))?;
                b = rest;
                if *f == F::Tg { v.target = s } else { v.source = s }
            }
            F::Tr => { let (t, rest) = postcard::take_from_bytes::<[f64; 3]>(b).map_err(|e| format!("translation: postcard {e}"))?; b = rest; v.t = t; }
            F::Ro => { let (r, rest) = postcard::take_from_bytes::<[f64; 4]>(b).map_err(|e| format!("rotation: postcard {e}"))?; b = rest; v.r = r; }
            F::St => {
                if at_first {
                    let (s, rest) = postcard::take_from_bytes::<StampAS>(b).map_err(|e| format!("stamp: postcard {e}"))?; b = rest;
                    v.stamp = Some(match s { StampAS::Static => None, StampAS::At(n) => Some(n) });
                } else {
                    let (s, rest) = postcard::take_from_bytes::<StampSA>(b).map_err(|e| format!("stamp: postcard {e}"))?; b = rest;
                    v.stamp = Some(match s { StampSA::Static => None, StampSA::At(n) => Some(n) });
                }
            }
        }
    }
    if strict && !b.is_empty() { return Err("trailing bytes (strict only)".into()); }
    // v3 validation
    if !v.r.iter().all(|c| c.is_finite()) { return Err("validate: non-finite rotation".into()); }
    let n = (v.r.iter().map(|c| c * c).sum::<f64>()).sqrt();
    if !((n - 1.0).abs() <= 1e-6) { return Err("validate: non-unit rotation".into()); }
    if !v.t.iter().all(|c| c.is_finite()) { return Err("validate: non-finite translation".into()); }
    if v.target.is_empty() || v.source.is_empty() { return Err("validate: empty frame name".into()); }
    if v.target == v.source && !(v.t == [0.0; 3] && v.r == [1.0, 0.0, 0.0, 0.0]) { return Err("validate: self transform not identity".into()); }
    Ok(v)
}

fn perms() -> Vec<[F; 5]> {
    let items = [F::Tg, F::Sr, F::Tr, F::Ro, F::St];
    let mut out = vec![];
    fn rec(cur: &mut Vec<F>, items: &[F; 5], out: &mut Vec<[F; 5]>) {
        if cur.len() == 5 { out.push([cur[0], cur[1], cur[2], cur[3], cur[4]]); return; }
        for i in items { if !cur.contains(i) { cur.push(*i); rec(cur, items, out); cur.pop(); } }
    }
    rec(&mut vec![], &items, &mut out);
    out
}
fn oname(o: &[F; 5]) -> String { o.iter().map(|f| fname(*f)).collect::<Vec<_>>().join(",") }

// real derive cross-checks
#[derive(Deserialize, Debug)] struct Vec3 { x: f64, y: f64, z: f64 }
#[derive(Deserialize, Debug)] struct Rot { w: f64, x: f64, y: f64, z: f64 }
#[derive(Deserialize, Debug)] #[allow(dead_code)]
struct D1 { translation: Vec3, rotation: Rot, stamp: StampSA, target: String, source: String }
#[derive(Deserialize, Debug)] #[allow(dead_code)]
struct D2 { target: String, source: String, translation: Vec3, rotation: Rot, stamp: StampSA }
#[derive(Deserialize, Debug)] #[allow(dead_code)]
struct D3 { stamp: StampAS, translation: Vec3, target: String, rotation: Rot, source: String }

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15); let mut z = self.0; z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9); z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB); z ^ (z >> 31) }
    fn unit(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    fn range(&mut self, n: u64) -> u64 { self.next() % n }
    fn name(&mut self) -> String {
        const C: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_/";
        let l = 1 + self.range(12);
        (0..l).map(|_| C[self.range(C.len() as u64) as usize] as char).collect()
    }
}

fn gen_a(r: &mut Rng) -> Transform {
    loop {
        let (p, c) = (r.name(), r.name());
        let mag = 10f64.powf(r.unit() * 8.0 - 3.0);
        let t = Vector3::new((r.unit() * 2.0 - 1.0) * mag, (r.unit() * 2.0 - 1.0) * mag, (r.unit() * 2.0 - 1.0) * mag);
        let q: Vec<f64> = (0..4).map(|_| r.unit() * 2.0 - 1.0).collect();
        let n = q.iter().map(|c| c * c).sum::<f64>().sqrt();
        if n < 0.1 || n > 1.0 { continue; }
        let q = Quaternion::from_wxyz(q[0] / n, q[1] / n, q[2] / n, q[3] / n);
        let stamp = if r.range(2) == 0 { Stamp::Static } else { let bits = r.range(64); Stamp::At(Timestamp::from_nanos(r.next() >> bits)) };
        if let Ok(tf) = Transform::new(&p, &c, t, q, stamp) { return tf; }
    }
}
// "ordinary fixture-like" data: round numbers, identity / axis rotations, small or second-aligned stamps
fn gen_b(r: &mut Rng) -> Transform {
    const V: &[f64] = &[0.0, 0.0, 0.0, 1.0, -1.0, 0.5, 2.0, 0.1, 0.25, 3.0];
    const N: &[&str] = &["map", "odom", "base_link", "base", "lidar", "camera", "a", "b", "world", "imu", "tool0", "laser"];
    let h = core::f64::consts::FRAC_1_SQRT_2;
    let qs = [(1.0, 0.0, 0.0, 0.0), (h, 0.0, 0.0, h), (0.0, 1.0, 0.0, 0.0), (0.5, 0.5, 0.5, 0.5), (h, h, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0)];
    loop {
        let mut pick = |r: &mut Rng| V[r.range(V.len() as u64) as usize];
        let t = Vector3::new(pick(r), pick(r), pick(r));
        let q = qs[r.range(qs.len() as u64) as usize];
        let stamp = match r.range(4) { 0 => Stamp::Static, 1 => Stamp::At(Timestamp::from_nanos(r.range(200))), 2 => Stamp::At(Timestamp::from_nanos(r.range(100) * 1_000_000_000)), _ => Stamp::At(Timestamp::from_nanos(1_753_142_400_000_000_000 + r.range(1_000_000_000_000))) };
        let (p, c) = (N[r.range(N.len() as u64) as usize], N[r.range(N.len() as u64) as usize]);
        if let Ok(tf) = Transform::new(p, c, t, Quaternion::from_wxyz(q.0, q.1, q.2, q.3), stamp) { return tf; }
    }
}

fn main() {
    let f_dyn: &[u8] = &[0,0,0,0,0,0,248,63, 0,0,0,0,0,0,2,192, 0,0,0,0,0,0,9,64, 0,0,0,0,0,0,240,63, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 1, 128,128,180,197,150,183,154,170,24, 3,109,97,112, 9,98,97,115,101,95,108,105,110,107];
    let f_static: &[u8] = &[0,0,0,0,0,0,248,63, 0,0,0,0,0,0,2,192, 0,0,0,0,0,0,9,64, 0,0,0,0,0,0,240,63, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0, 3,109,97,112, 9,98,97,115,101,95,108,105,110,107];
    let f_gen: &[u8] = &[0,0,0,0,0,0,248,63, 0,0,0,0,0,0,2,192, 0,0,0,0,0,0,9,64, 23,93,116,209,69,23,237,63, 70,23,93,116,209,69,183,63, 70,23,93,116,209,69,199,63, 70,23,93,116,209,69,215,63, 1, 128,128,180,197,150,183,154,170,24, 3,109,97,112, 9,98,97,115,101,95,108,105,110,107];
    let f_point: &[u8] = &[0,0,0,0,0,0,248,63, 0,0,0,0,0,0,2,192, 0,0,0,0,0,0,9,64, 0,0,0,0,0,0,240,63, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 128,128,180,197,150,183,154,170,24, 6,99,97,109,101,114,97];
    // fixtures are really what 2.4 emits
    let t24: Transform = postcard::from_bytes(f_dyn).unwrap();
    assert_eq!(postcard::to_allocvec(&t24).unwrap(), f_dyn);
    let fixtures = [("dyn", f_dyn), ("static", f_static), ("general", f_gen)];

    // postcard::from_bytes ignores trailing bytes?
    let mut tb = f_dyn.to_vec(); tb.extend_from_slice(&[1, 2, 3]);
    println!("postcard::from_bytes with 3 trailing bytes (2.4 type): ok={}", postcard::from_bytes::<Transform>(&tb).is_ok());

    let ps = perms();
    let mut rejecting = vec![];
    let mut reasons: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for at_first in [false, true] {
        for o in &ps {
            let res: Vec<_> = fixtures.iter().map(|(_, b)| decode(o, at_first, b, false)).collect();
            let key = format!("{} | {}", if at_first { "At,Static" } else { "Static,At" }, oname(o));
            if res.iter().all(|r| r.is_err()) {
                rejecting.push((at_first, *o));
                let sig = res.iter().zip(fixtures.iter()).map(|(r, (n, _))| format!("{n}: {}", r.as_ref().unwrap_err())).collect::<Vec<_>>().join(" ; ");
                reasons.entry(sig).or_default().push(key.clone());
            } else {
                let sig = res.iter().zip(fixtures.iter()).map(|(r, (n, _))| match r { Ok(v) => format!("{n}: OK {v:?}"), Err(e) => format!("{n}: {e}") }).collect::<Vec<_>>().join(" ; ");
                println!("NOT-REJECTING {key}\n    {sig}");
            }
            // point fixture
            if let Ok(v) = decode(o, at_first, f_point, false) { println!("POINT FIXTURE DECODES under {key}: {v:?}"); }
        }
    }
    println!("\nlayouts rejecting all 3 Transform fixtures (lenient trailing): {} of 240", rejecting.len());
    for (sig, keys) in &reasons {
        println!("\nREASON [{}] ({} layouts)", sig, keys.len());
        for k in keys { println!("    {k}"); }
    }

    // cross-check sequential decode against real derives
    let mut rng = Rng(42);
    let n = 100_000;
    let pops: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("A random", (0..n).map(|_| postcard::to_allocvec(&gen_a(&mut rng)).unwrap()).collect()),
        ("B fixture-like", (0..n).map(|_| postcard::to_allocvec(&gen_b(&mut rng)).unwrap()).collect()),
    ];
    for (_, pop) in &pops {
        for b in pop.iter() {
            let seq = |o: [F; 5], af| { let mut bb: &[u8] = b; let mut ok = true; for f in o { let r = match f {
                F::Tg | F::Sr => postcard::take_from_bytes::<String>(bb).map(|x| x.1).ok(),
                F::Tr => postcard::take_from_bytes::<[f64; 3]>(bb).map(|x| x.1).ok(),
                F::Ro => postcard::take_from_bytes::<[f64; 4]>(bb).map(|x| x.1).ok(),
                F::St => if af { postcard::take_from_bytes::<StampAS>(bb).map(|x| x.1).ok() } else { postcard::take_from_bytes::<StampSA>(bb).map(|x| x.1).ok() } };
                match r { Some(rest) => bb = rest, None => { ok = false; break; } } } ok };
            assert_eq!(postcard::from_bytes::<D1>(b).is_ok(), seq([F::Tr, F::Ro, F::St, F::Tg, F::Sr], false));
            assert_eq!(postcard::from_bytes::<D2>(b).is_ok(), seq([F::Tg, F::Sr, F::Tr, F::Ro, F::St], false));
            assert_eq!(postcard::from_bytes::<D3>(b).is_ok(), seq([F::St, F::Tr, F::Tg, F::Ro, F::Sr], true));
        }
    }
    println!("\ncross-check of sequential decode vs 3 derive structs: passed");

    for (pname, pop) in &pops {
        println!("\n=== population {pname}: {n} valid 2.4 transforms, Ok counts per layout (lenient / strict) ===");
        let mut rows = vec![];
        for at_first in [false, true] {
            for o in &ps {
                let mut ok = 0u32; let mut oks = 0u32; let mut ex = None;
                for b in pop {
                    if let Ok(v) = decode(o, at_first, b, false) { ok += 1; if ex.is_none() { ex = Some((b.clone(), v)); } }
                    if decode(o, at_first, b, true).is_ok() { oks += 1; }
                }
                rows.push((ok, oks, at_first, *o, rejecting.contains(&(at_first, *o)), ex));
            }
        }
        rows.sort_by_key(|r| (r.0, r.1));
        let zero = rows.iter().filter(|r| r.0 == 0).count();
        let zero_rej = rows.iter().filter(|r| r.0 == 0 && r.4).count();
        println!("layouts with 0 Ok (lenient): {zero}; of those also rejecting all fixtures: {zero_rej}");
        let mut hist: BTreeMap<u32, u32> = BTreeMap::new();
        for r in &rows { *hist.entry(r.0).or_default() += 1; }
        println!("histogram lenient Ok-count -> #layouts: {hist:?}");
        for r in rows.iter().filter(|r| r.0 > 0) {
            println!("  ok={:6} strict_ok={:6} rejects_fixtures={} {} | {}", r.0, r.1, r.4, if r.2 { "At,Static" } else { "Static,At" }, oname(&r.3));
            if let Some((b, v)) = &r.5 { let orig: Transform = postcard::from_bytes(b).unwrap(); println!("      e.g. 2.4 {orig:?}\n        -> v3 {v:?}"); }
        }
        for r in rows.iter().filter(|r| r.0 == 0) {
            println!("  ZERO {} | {}  (rejects_fixtures={})", if r.2 { "At,Static" } else { "Static,At" }, oname(&r.3), r.4);
        }
    }


    // ---- stream context: record followed by 5 more records (Vec<Transform>, log file, struct with trailing fields) ----
    for (pname, pop) in &pops {
        let mut nonzero = 0; let mut total_ok = 0u64; let mut worst = (0u32, String::new());
        let mut eof_dep = 0;
        for at_first in [false, true] { for o in &ps {
            if !rejecting.contains(&(at_first, *o)) { continue; }
            let mut ok = 0u32;
            for i in 0..pop.len() - 5 { let mut b = pop[i].clone(); for k in 1..=5 { b.extend_from_slice(&pop[i + k]); }
                if decode(o, at_first, &b, false).is_ok() { ok += 1; } }
            if ok > 0 { nonzero += 1; }
            total_ok += ok as u64;
            if ok > worst.0 { worst = (ok, format!("{} | {}", if at_first { "At,Static" } else { "Static,At" }, oname(o))); }
            if o == &[F::Tg, F::Sr, F::Tr, F::Ro, F::St] && !at_first { println!("stream, natural order target,source,translation,rotation,stamp: ok={ok}"); }
            let _ = &mut eof_dep;
        } }
        println!("stream context, population {pname}: fixture-rejecting layouts with >0 Ok: {nonzero} of 238; total Ok {total_ok}; worst {worst:?}");
    }
    // fixture reasons in stream context for natural order, and how many layouts' fixture rejections are all EOF
    let nat = [F::Tg, F::Sr, F::Tr, F::Ro, F::St];
    for (n, b) in fixtures { println!("natural order, fixture {n}: {:?}", decode(&nat, false, b, false).map(|_| ()).unwrap_err()); }
    let mut eof_only = 0; let mut eof_any = 0; let mut stream_fix_ok = vec![];
    for (af, o) in &rejecting {
        let rs: Vec<String> = fixtures.iter().map(|(_, b)| decode(o, *af, b, false).unwrap_err()).collect();
        if rs.iter().all(|r| r.contains("end of buffer")) { eof_only += 1; }
        if rs.iter().any(|r| r.contains("end of buffer")) { eof_any += 1; }
        // fixtures concatenated: dyn+static+general+dyn+static+general
        for s in 0..3 { let mut b = vec![]; for k in 0..6 { b.extend_from_slice(fixtures[(s + k) % 3].1); }
            if let Ok(v) = decode(o, *af, &b, false) { stream_fix_ok.push(format!("{} | {} start {} -> {v:?}", if *af { "At,Static" } else { "Static,At" }, oname(o), fixtures[s].0)); } }
    }
    println!("layouts whose rejection of all 3 fixtures is only end-of-buffer: {eof_only}; of at least one fixture: {eof_any}");
    println!("fixture-rejecting layouts that decode a fixture Ok when fixtures are concatenated: {}", stream_fix_ok.len());
    for l in stream_fix_ok.iter().take(6) { println!("   {l}"); }
    // standalone Stamp / Timestamp
    let s24 = postcard::to_allocvec(&Stamp::At(Timestamp::from_nanos(1_753_142_400_000_000_000))).unwrap();
    println!("\nstandalone 2.4 Stamp::At(1753142400e9) under v3 Stamp{{Static,At(i64)}}: {:?}", postcard::from_bytes::<StampSA>(&s24));
    println!("standalone under reversed variants: {:?}", postcard::from_bytes::<StampAS>(&s24));
    let s24s = postcard::to_allocvec(&Stamp::<Timestamp>::Static).unwrap();
    println!("standalone 2.4 Stamp::Static under reversed variants: {:?}", postcard::from_bytes::<StampAS>(&s24s));
    let v: Vec<Transform> = vec![t24.clone(), t24.clone()];
    let vb = postcard::to_allocvec(&v).unwrap();
    println!("Vec<2.4 Transform> len {} bytes (probe of containers left to per-layout test)", vb.len());

    // JSON
    let j = serde_json::to_string(&t24).unwrap();
    println!("\n2.4 JSON: {j}");
    println!("v3 derive (default serde): {:?}", serde_json::from_str::<D2>(&j).map_err(|e| e.to_string()));
    #[derive(Deserialize, Debug)] #[serde(deny_unknown_fields)] #[allow(dead_code)]
    struct D2s { target: String, source: String, translation: Vec3, rotation: Rot, stamp: StampSA }
    println!("v3 derive (deny_unknown_fields): {:?}", serde_json::from_str::<D2s>(&j).map_err(|e| e.to_string()));
    // v3 JSON with a stray 2.4 key: ignored silently by default
    let mixed = r#"{"target":"map","source":"base","translation":{"x":1.0,"y":0.0,"z":0.0},"rotation":{"w":1.0,"x":0.0,"y":0.0,"z":0.0},"stamp":"Static","timestamp":{"At":5},"parent":"odom"}"#;
    println!("v3 JSON carrying stray 2.4 keys (default): {:?}", serde_json::from_str::<D2>(mixed).map_err(|e| e.to_string()));
    println!("same, deny_unknown_fields: {:?}", serde_json::from_str::<D2s>(mixed).map_err(|e| e.to_string()));
    // 2.4 Timestamp JSON > i64::MAX
    println!("2.4 JSON stamp u64::MAX under i64: {:?}", serde_json::from_str::<StampSA>(r#"{"At":18446744073709551615}"#).map_err(|e| e.to_string()));
    // v3 JSON under 2.4
    let v3j = r#"{"target":"map","source":"base","translation":{"x":1.0,"y":0.0,"z":0.0},"rotation":{"w":1.0,"x":0.0,"y":0.0,"z":0.0},"stamp":{"At":-5}}"#;
    println!("v3 JSON under 2.4: {:?}", serde_json::from_str::<Transform>(v3j).map_err(|e| e.to_string()));

    // ---- M1 ----
    let q = Quaternion::from_wxyz(0.723_317_411_364_711_8, 0.391_903_837_329_119_9, 0.200_562_121_146_575_12, 0.531_975_695_182_166_8);
    let t = Vector3::new(-1.0, 0.5, 0.25);
    let p = Vector3::new(0.3, -0.7, 1.1);
    let rot = |q: Quaternion, v: Vector3| q.rotate_vector(v);
    let add = |a: Vector3, b: Vector3| Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z);
    let sub = |a: Vector3, b: Vector3| Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z);
    let qc = q.conjugate();
    let exp = Vector3::new(0.312_544_181_470_241, 0.300_345_740_105_364_36, 0.415_426_564_355_733_33);
    let rp = rot(q, p);
    let xyzw_as_wxyz = Quaternion::from_wxyz(q.x, q.y, q.z, q.w);
    let cases: Vec<(&str, Vector3)> = vec![
        ("correct  R p + t", add(rp, t)),
        ("V1 R (p + t)  [add before rotate]", rot(q, add(p, t))),
        ("V2 R^-1 p + t [inverse rotation, original translation]", add(rot(qc, p), t)),
        ("V3 R p        [translation forgotten]", rp),
        ("V4 R^-1 (p - t) [whole inverse applied: wrong direction]", rot(qc, sub(p, t))),
        ("V5 R p - t    [sign]", sub(rp, t)),
        ("V6 p + R t    [rotated the translation]", add(p, rot(q, t))),
        ("V7 p + t      [rotation forgotten]", add(p, t)),
        ("V8 add typo (x+x, y+y, z+y)", Vector3::new(rp.x + t.x, rp.y + t.y, rp.z + t.y)),
        ("V9 xyzw read as wxyz, then R p + t", add(rot(xyzw_as_wxyz, p), t)),
    ];
    println!("\nM1 golden vector 4, expected {exp:?}");
    for (n, v) in cases {
        let d = ((v.x - exp.x).powi(2) + (v.y - exp.y).powi(2) + (v.z - exp.z).powi(2)).sqrt();
        println!("  {n:58} -> ({:+.6}, {:+.6}, {:+.6})  |err|={d:.4} finite={}", v.x, v.y, v.z, v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
    }
}
