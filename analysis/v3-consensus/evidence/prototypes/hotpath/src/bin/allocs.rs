use proto_hotpath::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Duration;

struct Counting;
static N: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { N.fetch_add(1, Relaxed); BYTES.fetch_add(l.size() as u64, Relaxed); unsafe { System.alloc(l) } }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { unsafe { System.dealloc(p, l) } }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { N.fetch_add(1, Relaxed); BYTES.fetch_add(n as u64, Relaxed); unsafe { System.realloc(p, l, n) } }
}
#[global_allocator]
static A: Counting = Counting;

fn run<S: Sut>() {
    const R: u64 = 1000;
    for r in reads::<S>() {
        let (n0, b0) = (N.load(Relaxed), BYTES.load(Relaxed));
        for _ in 0..R { r.reg.lookup(r.target, r.source, r.at); }
        println!("{}\t{}\t{:.3}\t{:.1}", S::name(), r.label, (N.load(Relaxed) - n0) as f64 / R as f64, (BYTES.load(Relaxed) - b0) as f64 / R as f64);
    }
    const DT: u64 = 10_000_000;
    let mut reg = S::new(Some(Duration::from_secs(10)));
    let mut n = BASE;
    for _ in 0..3000 { reg.add("a", "b", X, rot(n), Some(n)); n += DT; }
    // (a) insert only, sample pre-built; (b) build + insert
    let samples: Vec<S::Sample<'static>> = (0..R).map(|i| S::make("a", "b", X, rot(n + i * DT), Some(n + i * DT))).collect();
    let (n0, b0) = (N.load(Relaxed), BYTES.load(Relaxed));
    for s in samples { reg.insert(s); }
    println!("{}\tinsert_steady (sample pre-built)\t{:.3}\t{:.1}", S::name(), (N.load(Relaxed) - n0) as f64 / R as f64, (BYTES.load(Relaxed) - b0) as f64 / R as f64);
    n += R * DT;
    let (n0, b0) = (N.load(Relaxed), BYTES.load(Relaxed));
    for i in 0..R { reg.insert(S::make("a", "b", X, rot(n + i * DT), Some(n + i * DT))); }
    println!("{}\tinsert_steady (build + insert)\t{:.3}\t{:.1}", S::name(), (N.load(Relaxed) - n0) as f64 / R as f64, (BYTES.load(Relaxed) - b0) as f64 / R as f64);
}
fn main() {
    println!("variant\tworkload\tallocs/op\tbytes/op");
    run::<V24>(); run::<PBH>(); run::<PBB>(); run::<PDH>(); run::<PDB>();
}
