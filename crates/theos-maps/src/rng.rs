//! Per-stage random streams and the hashes that seed them.
//!
//! Each stage draws from its own [`StageRng`], seeded from the run seed and the stage key, so a
//! stream never depends on what other stages drew.

use rand_core::{Rng, SeedableRng};
use rand_pcg::Pcg64;

use crate::kernels::num;

/// The 64-bit FNV-1a hash of `bytes`.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The `SplitMix64` generator, used to expand a 64-bit seed.
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator starting at `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64-bit output.
    #[expect(
        clippy::should_implement_trait,
        reason = "an infinite stream is not an Iterator"
    )]
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// A deterministic random stream for one stage (PCG-64).
#[derive(Debug, Clone)]
pub struct StageRng {
    pcg: Pcg64,
}

impl StageRng {
    /// The stream of the stage `key` under `run_seed`.
    #[must_use]
    pub fn for_key(run_seed: u64, key: &str) -> Self {
        let mut mix = SplitMix64::new(run_seed ^ fnv1a64(key.as_bytes()));
        let mut seed = [0_u8; 32];
        for chunk in seed.as_chunks_mut::<8>().0 {
            *chunk = mix.next().to_le_bytes();
        }
        Self {
            pcg: Pcg64::from_seed(seed),
        }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.pcg.next_u64()
    }

    /// A float in `[0, 1)` with 53 random bits.
    pub fn uniform(&mut self) -> f64 {
        num::unit_from_bits(self.next_u64())
    }

    /// An integer in `0..n` without modulo bias (Lemire); `n` must be positive, else `0`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let (mut low, mut high) = num::split_u128(u128::from(self.next_u64()) * u128::from(n));
        if low < n {
            let threshold = n.wrapping_neg() % n;
            while low < threshold {
                (low, high) = num::split_u128(u128::from(self.next_u64()) * u128::from(n));
            }
        }
        high
    }

    /// A float in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.uniform()
    }

    /// A standard normal draw (Box-Muller, one draw per call; the sine partner is discarded).
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform();
        let u2 = self.uniform();
        let radius = (-2.0 * libm::log(u1)).sqrt();
        radius * libm::cos(2.0 * std::f64::consts::PI * u2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_matches_reference_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn splitmix64_matches_reference() {
        assert_eq!(SplitMix64::new(0).next(), 0xe220_a839_7b1d_cdaf);
    }

    fn four(rng: &mut StageRng) -> [u64; 4] {
        [
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
        ]
    }

    #[test]
    fn stage_streams_do_not_depend_on_other_streams() {
        let alone = four(&mut StageRng::for_key(1, "tectonics#0"));
        let mut other = StageRng::for_key(1, "grid#0");
        other.next_u64();
        assert_eq!(alone, four(&mut StageRng::for_key(1, "tectonics#0")));
        assert_ne!(alone, four(&mut StageRng::for_key(1, "tectonics#1")));
        assert_ne!(alone, four(&mut StageRng::for_key(2, "tectonics#0")));
    }

    #[test]
    fn stage_stream_is_pinned() {
        let mut rng = StageRng::for_key(1, "tectonics#0");
        // Checked against an independent Python PCG-64 (XSL-RR, seed bytes = four SplitMix64
        // outputs, little-endian, state then increment).
        assert_eq!(rng.next_u64(), 7_739_573_837_118_461_441);
        assert_eq!(rng.next_u64(), 17_749_986_308_897_808_859);
        assert_eq!(rng.next_u64(), 1_391_999_321_210_941_311);
    }

    #[test]
    fn below_stays_below() {
        let mut rng = StageRng::for_key(3, "below");
        for n in [1_u64, 7, 1 << 40] {
            for _ in 0..1000 {
                assert!(rng.below(n) < n);
            }
        }
    }

    #[test]
    fn uniform_and_range_stay_in_bounds() {
        let mut rng = StageRng::for_key(4, "uniform");
        for _ in 0..1000 {
            let u = rng.uniform();
            assert!((0.0..1.0).contains(&u));
            let r = rng.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&r));
        }
    }

    #[test]
    fn normal_has_unit_spread() {
        let mut rng = StageRng::for_key(5, "normal");
        let draws: Vec<f64> = (0..20_000).map(|_| rng.normal()).collect();
        let count = crate::kernels::num::to_f64(draws.len());
        let mean = draws.iter().sum::<f64>() / count;
        let var = draws.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / count;
        assert!(mean.abs() < 0.03, "mean {mean}");
        assert!((var - 1.0).abs() < 0.05, "variance {var}");
    }
}
