//! The colours of the renders (sRGB) and the colour arithmetic they share.
//!
//! The ground, water and farmland colours are 0.3.0's (`render_terrain.py`, `C`); the rock
//! colours are the rock table's (`rocks::props`).

use crate::kernels::num::to_u8_unit;
use crate::stages::terrain::rocks::{self, Rock};

/// An sRGB colour, one byte per channel.
pub(crate) type Rgb = [u8; 3];

/// A colour with channels in `[0, 1]`, for mixing.
pub(crate) type Unit = [f64; 3];

/// Bare soil on gentle ground (0.3.0 `soil`).
pub(crate) const SOIL: Rgb = [0xb7, 0xa0, 0x7a];
/// Sparse vegetation (0.3.0 `grass`).
pub(crate) const GRASS: Rgb = [0xa3, 0xb2, 0x6c];
/// Dense low vegetation (0.3.0 `lush`).
pub(crate) const LUSH: Rgb = [0x7e, 0x9c, 0x55];
/// Forest (0.3.0 `forest`).
pub(crate) const FOREST: Rgb = [0x3d, 0x65, 0x37];
/// Snow cover (0.3.0 `snow`).
pub(crate) const SNOW: Rgb = [0xf3, 0xf5, 0xf7];
/// Lake water, flat: a lake cell is exactly this colour in `base` (0.3.0 `lake`).
pub(crate) const LAKE: Rgb = [0x4f, 0x80, 0xad];
/// River water (0.3.0 `river`).
pub(crate) const RIVER: Rgb = [0x3a, 0x6c, 0x9c];
/// The first of the two field colours of farmland (0.3.0 `farm_a`).
pub(crate) const FARM_A: Rgb = [0xc8, 0xc2, 0x7e];
/// The second field colour (0.3.0 `farm_b`).
pub(crate) const FARM_B: Rgb = [0xb4, 0xb4, 0x6a];

/// The map colour of the rock with raster value `code`; soil for a value that is no rock.
pub(crate) fn rock(code: u8) -> Rgb {
    Rock::from_code(code).map_or(SOIL, |r| rocks::props(r).colour)
}

/// `c` with channels in `[0, 1]`.
pub(crate) fn unit(c: Rgb) -> Unit {
    c.map(|v| f64::from(v) / 255.0)
}

/// `c` clamped to `[0, 1]` and rounded to bytes.
pub(crate) fn rgb(c: Unit) -> Rgb {
    c.map(to_u8_unit)
}

/// `a` blended towards `b` by `t` (0: `a`, 1: `b`).
pub(crate) fn mix(a: Unit, b: Unit, t: f64) -> Unit {
    [0, 1, 2].map(|i| a[i] * (1.0 - t) + b[i] * t)
}

/// `c` with every channel multiplied by `k`.
pub(crate) fn scale(c: Unit, k: f64) -> Unit {
    c.map(|v| v * k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_colours_come_from_the_rock_table() {
        assert_eq!(rock(Rock::Basalt.code()), [0x62, 0x5e, 0x60]);
        assert_eq!(
            rock(Rock::Granite.code()),
            rocks::props(Rock::Granite).colour
        );
        assert_eq!(rock(200), SOIL);
    }

    #[test]
    fn colours_round_trip_and_mix() {
        assert_eq!(rgb(unit(LAKE)), LAKE);
        assert_eq!(rgb(mix(unit(SOIL), unit(SNOW), 0.0)), SOIL);
        assert_eq!(rgb(mix(unit(SOIL), unit(SNOW), 1.0)), SNOW);
        assert_eq!(rgb(mix([0.0; 3], [1.0; 3], 0.5)), [128; 3]);
        assert_eq!(rgb(scale([0.5; 3], 4.0)), [255; 3]);
        assert_eq!(rgb([-1.0; 3]), [0; 3]);
    }
}
