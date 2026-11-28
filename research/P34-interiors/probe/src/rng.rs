//! Seeded PCG stream with small integer helpers.
use rand_core::{RngCore, SeedableRng};
use rand_pcg::Pcg64;

#[derive(Clone)]
pub struct Rng(Pcg64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(Pcg64::seed_from_u64(seed))
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.0.next_u64() % n as u64) as usize }
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1).max(1) as usize) as i32
    }
    pub fn pct(&mut self, p: u32) -> bool {
        (self.below(100) as u32) < p
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> Option<&'a T> {
        if v.is_empty() { None } else { Some(&v[self.below(v.len())]) }
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}
