use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use proto_hotpath::*;
use std::cell::Cell;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn reads_bench<S: Sut>(c: &mut Criterion) {
    let mut g = c.benchmark_group(S::name());
    g.sample_size(100).warm_up_time(Duration::from_millis(300)).measurement_time(Duration::from_millis(1500));
    for r in reads::<S>() {
        assert_eq!(r.reg.lookup(r.target, r.source, r.at), r.ok, "{}", r.label);
        g.bench_function(r.label, |b| b.iter(|| r.reg.lookup(r.target, r.source, r.at)));
    }
    // steady-state insert: 10 s max_age, 10 ms spacing => ~1k resident samples
    g.bench_function("insert_steady_maxage10s_1k", |b| {
        const DT: u64 = 10_000_000;
        let mut reg = S::new(Some(Duration::from_secs(10)));
        let mut n = BASE;
        for _ in 0..1000 { reg.add("a", "b", X, rot(n), Some(n)); n += DT; }
        let next = Cell::new(n);
        b.iter_batched(
            || { let n = next.get(); next.set(n + DT); S::make("a", "b", X, rot(n), Some(n)) },
            |s| reg.insert(s),
            BatchSize::SmallInput,
        );
    });
    g.finish();
}

/// Out-of-order inserts of fresh keys into an unbounded history of `n`
/// samples. The history is rebuilt (untimed) every n/10 inserts so it grows
/// by at most 10%; only the inserts are timed.
fn ooo_bench<S: Sut>(c: &mut Criterion) {
    let mut g = c.benchmark_group(S::name());
    g.sample_size(20).warm_up_time(Duration::from_millis(200)).measurement_time(Duration::from_millis(600));
    for n in [1_000u64, 10_000, 60_000] {
        for (pat, newest) in [("uniform", false), ("newest1pct", true)] {
            g.bench_function(format!("ooo_insert_{}k_{pat}", n / 1000), |b| {
                let mut rng = Rng(0xBEEF ^ n);
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    let mut left = iters;
                    while left > 0 {
                        let k = left.min(n / 10);
                        let mut reg: S = chain(&["a", "b"], n);
                        let last = BASE + (n - 1) * STEP;
                        let span = (n - 1) * STEP;
                        let samples: Vec<S::Sample<'static>> = (0..k).map(|_| {
                            // odd nanos: never on the even 1 ms grid => always a fresh key (modulo repeats)
                            let off = if newest { span - span / 100 + rng.below(span / 100) } else { rng.below(span) };
                            let t = (BASE + off) | 1;
                            debug_assert!(t < last);
                            let _ = last;
                            S::make("a", "b", X, rot(t), Some(t))
                        }).collect();
                        let start = Instant::now();
                        for s in samples { black_box(reg.insert(s)); }
                        total += start.elapsed();
                        black_box(&reg);
                        left -= k;
                    }
                    total
                });
            });
        }
    }
    g.finish();
}

fn all(c: &mut Criterion) {
    reads_bench::<V24>(c); reads_bench::<PBH>(c); reads_bench::<PBB>(c); reads_bench::<PDH>(c); reads_bench::<PDB>(c);
    ooo_bench::<V24>(c); ooo_bench::<PBH>(c); ooo_bench::<PDH>(c);
}
criterion_group!(benches, all);
criterion_main!(benches);
