use proto_hotpath::*;

struct Stats { cats: std::collections::BTreeMap<String,u64>, ok: u64, err_same: u64, kind_mismatch: u64, frame_mismatch: u64, ok_err_mismatch: u64, max_t: f64, max_r: f64,
    single: u64, single_bit: u64, same_unknown: u64 }

fn run<S: Sut>(seed: u64, trees: u64, queries: u64) -> Stats {
    let mut st = Stats { cats: Default::default(), ok: 0, err_same: 0, kind_mismatch: 0, frame_mismatch: 0, ok_err_mismatch: 0, max_t: 0.0, max_r: 0.0, single: 0, single_bit: 0, same_unknown: 0 };
    let mut rng = Rng(seed);
    let mut shown = 0;
    for _ in 0..trees {
        let n = 2 + rng.below(40) as usize;
        let names: Vec<String> = (0..n).map(|i| format!("f{i}")).collect();
        let mut a = V24::new(None);
        let mut b = S::new(None);
        let mut parent = vec![usize::MAX; n];
        let mut stamps: Vec<Vec<u64>> = vec![vec![]; n];
        for i in 1..n {
            if rng.below(10) == 0 { continue; } // a new root: disconnected forests
            let p = rng.below(i as u64) as usize;
            parent[i] = p;
            let stat = rng.below(10) < 3;
            let k = if stat { 1 } else { 1 + rng.below(20) };
            for _ in 0..k {
                let t = [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)];
                let q = rng.quat();
                let stamp = if stat { None } else { let s = rng.below(1_000_000_000); stamps[i].push(s); Some(s) };
                a.add(&names[p], &names[i], t, q, stamp);
                b.add(&names[p], &names[i], t, q, stamp);
            }
        }
        for _ in 0..queries {
            let ti = rng.below(n as u64 + 1) as usize;
            let si = rng.below(n as u64 + 1) as usize;
            let tn = if ti == n { "nope" } else { &names[ti] };
            let sn = if si == n { "nada" } else { &names[si] };
            // time: stored stamp of the source edge, a uniform instant, or far outside
            let at = match rng.below(10) {
                0..=3 if si < n && !stamps[si].is_empty() => stamps[si][rng.below(stamps[si].len() as u64) as usize],
                9 => 2_000_000_000,
                _ => rng.below(1_000_000_000),
            };
            let (ra, rb) = (a.get(tn, sn, at), b.get(tn, sn, at));
            let cat = |r: &Result<[f64;7],(Kind,String)>| match r { Ok(_) => "Ok".to_string(), Err((k,_)) => format!("{k:?}") };
            if cat(&ra) != cat(&rb) { *st.cats.entry(format!("2.4={} proto={}", cat(&ra), cat(&rb))).or_default() += 1; }
            match (&ra, &rb) {
                (Ok(x), Ok(y)) => {
                    st.ok += 1;
                    for i in 0..3 { st.max_t = st.max_t.max((x[i] - y[i]).abs()); }
                    for i in 3..7 { st.max_r = st.max_r.max((x[i] - y[i]).abs()); }
                    // single hop toward the parent at a stored stamp (or a static edge)
                    if si < n && parent[si] == ti && (stamps[si].is_empty() || stamps[si].contains(&at)) {
                        st.single += 1;
                        if x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()) { st.single_bit += 1; }
                    }
                }
                (Err((ka, fa)), Err((kb, fb))) => {
                    if ka != kb { st.kind_mismatch += 1; if shown < 5 { shown += 1; eprintln!("kind: {tn}<-{sn}@{at}: 2.4 {ra:?} proto {rb:?}"); } }
                    else if fa != fb { st.frame_mismatch += 1; if shown < 5 { shown += 1; eprintln!("frame: {tn}<-{sn}@{at}: 2.4 {ra:?} proto {rb:?}"); } }
                    else { st.err_same += 1; }
                }
                _ => {
                    if tn == sn { st.same_unknown += 1; }
                    else { st.ok_err_mismatch += 1; if shown < 5 { shown += 1; eprintln!("ok/err: {tn}<-{sn}@{at}: 2.4 {ra:?} proto {rb:?}"); } }
                }
            }
        }
    }
    st
}

fn report<S: Sut>() {
    let s = run::<S>(0xC0FFEE, 2000, 500);
    println!("{}: ok={} max|dt|={:e} max|dq|={:e} single_hop_exact={} bit_identical={} err_same={} kind_mismatch={} frame_mismatch={} ok_vs_err={} same_unknown_name(2.4 identity, proto UnknownFrame)={}",
        S::name(), s.ok, s.max_t, s.max_r, s.single, s.single_bit, s.err_same, s.kind_mismatch, s.frame_mismatch, s.ok_err_mismatch, s.same_unknown);
}
fn main() { let s = run::<PDB>(0xC0FFEE, 2000, 500); println!("category disagreements: {:?}", s.cats); report::<PBH>(); report::<PBB>(); report::<PDH>(); report::<PDB>(); }
