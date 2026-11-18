//! A small deterministic generator (splitmix64). No external dependency, so
//! a seed reproduces a run on every machine.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`; `n` must be positive. The modulo bias is
    /// irrelevant at the sizes used here.
    pub fn below(
        &mut self,
        n: u64,
    ) -> u64 {
        self.next() % n
    }

    /// Uniform in `0..n` as an `i64`.
    pub fn below_i(
        &mut self,
        n: i64,
    ) -> i64 {
        i64::try_from(self.below(n.unsigned_abs().max(1))).unwrap_or(0)
    }

    pub fn index(
        &mut self,
        len: usize,
    ) -> usize {
        usize::try_from(self.below(len as u64)).unwrap_or(0)
    }

    /// True with probability `percent / 100`.
    pub fn percent(
        &mut self,
        percent: u64,
    ) -> bool {
        self.below(100) < percent
    }

    /// Uniform in `[0, 1)` with 53 random bits.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(
        &mut self,
        lo: f64,
        hi: f64,
    ) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    pub fn pick<T: Copy>(
        &mut self,
        items: &[T],
    ) -> T {
        items[self.index(items.len())]
    }
}
