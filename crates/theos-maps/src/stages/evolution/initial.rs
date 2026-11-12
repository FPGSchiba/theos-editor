//! The initial surface of `landscape_evolution`: 0.3.0's `z0`, and empty erosion and deposition
//! records.

use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::rng::StageRng;
use crate::stage::{LayerSpec, StageContext, StageError};
use crate::world::Layer;

/// The spectral slope of the initial surface's noise (0.3.0).
const BETA: f64 = 3.0;

/// The layer the initial surface reads.
pub(super) const READ: LayerSpec = LayerSpec::mask("fixed");

/// The layers the initial surface writes.
pub(super) const WRITES: [LayerSpec; 3] = [
    LayerSpec::f64("height"),
    LayerSpec::f64("erosion"),
    LayerSpec::f64("deposition"),
];

/// The initial height (0.3.0): `relief_m · noise + relief_m + jitter` per cell, where `noise` is
/// [`spectral_noise`] with β = 3 and `jitter` is uniform in `[0, 1)` m. Both come from `rng`:
/// first the noise, then one jitter per cell in row-major order, fixed cells included, so the
/// stream does not depend on the edges. Fixed cells are 0, the base level.
///
/// 0.3.0 drew the jitter from the stage's own stream; here both come from the one `z0` stream.
pub(super) fn surface(fixed: &Grid<bool>, relief_m: f64, rng: &mut StageRng) -> Grid<f64> {
    let noise = spectral_noise(fixed.rows(), fixed.cols(), rng, BETA, 0.0, None);
    let mut height = noise.map(|n| relief_m * n + relief_m);
    for (h, &on) in height.as_mut_slice().iter_mut().zip(fixed.as_slice()) {
        let jitter = rng.uniform();
        *h = if on { 0.0 } else { *h + jitter };
    }
    height
}

/// Writes the initial `height` (from the stream `<key>/z0`) and zero `erosion` and
/// `deposition`.
///
/// # Errors
/// [`StageError`] when `fixed` is missing or a write is refused.
pub(super) fn write(ctx: &mut StageContext<'_>, relief_m: f64) -> Result<(), StageError> {
    let height = surface(ctx.mask(READ.name)?, relief_m, &mut ctx.sub_rng("z0"));
    let zeros = Grid::filled(height.rows(), height.cols(), 0.0);
    ctx.write("height", Layer::F64(height))?;
    ctx.write("erosion", Layer::F64(zeros.clone()))?;
    ctx.write("deposition", Layer::F64(zeros))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::surface;
    use crate::grid::Grid;
    use crate::kernels::noise::spectral_noise;
    use crate::rng::StageRng;

    fn edges() -> Grid<bool> {
        let cells = (0..64 * 64).map(|i| i < 64 || i % 64 == 0).collect();
        Grid::from_vec(64, 64, cells).unwrap()
    }

    #[test]
    fn the_initial_surface_is_relief_times_noise_plus_relief_plus_jitter() {
        let fixed = edges();
        let height = surface(&fixed, 40.0, &mut StageRng::for_key(1, "k/z0"));
        let mut rng = StageRng::for_key(1, "k/z0");
        let noise = spectral_noise(64, 64, &mut rng, 3.0, 0.0, None);
        for (i, h) in height.as_slice().iter().enumerate() {
            let jitter = rng.uniform();
            if fixed.as_slice()[i] {
                assert_eq!(h.to_bits(), 0.0_f64.to_bits(), "fixed cell {i}");
            } else {
                let expected = 40.0 * noise.as_slice()[i] + 40.0 + jitter;
                assert_eq!(h.to_bits(), expected.to_bits(), "cell {i}");
            }
        }
    }

    #[test]
    fn without_relief_the_surface_is_the_jitter() {
        let fixed = Grid::filled(64, 64, false);
        let height = surface(&fixed, 0.0, &mut StageRng::for_key(2, "k/z0"));
        assert!(height.as_slice().iter().all(|h| (0.0..1.0).contains(h)));
        let other = surface(&fixed, 0.0, &mut StageRng::for_key(3, "k/z0"));
        assert_ne!(height, other, "another seed, another surface");
    }
}
