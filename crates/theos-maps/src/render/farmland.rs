//! Farmland around settlements, as 0.3.0 drew it (`render_terrain.py`, `_base_rgb`).
//!
//! Every settlement farms a Gaussian of its class's radius: 2 km for a city, 1.5 km for a town,
//! 1 km for a village and 0.55 km for a hamlet. A pixel is farmland by
//! `smoothstep(0.35, 0.6, g)` of the largest Gaussian `g` reaching it; the fields are stripes
//! of two colours, `0.5 + 0.5·sign(sin(x/7 + 0.6·y/9)·sin(y/11))` of pixel column `x` and row
//! `y`. 0.3.0 roughened the edge with spectral noise; the renders have no random stream, so the
//! edge here is round (the slope term of the caller still bends it to the terrain).

use super::palette::{self, Unit};
use crate::features::{FeatureSet, Geometry};
use crate::kernels::num::{max, smoothstep, to_f64};

/// The Gaussian value below which a pixel is no farmland.
const EDGE_LOW: f64 = 0.35;
/// The Gaussian value above which a pixel is all farmland.
const EDGE_HIGH: f64 = 0.6;
/// How far a farm reaches, in radii: beyond it the Gaussian, `exp(−1.125) ≈ 0.32`, is below
/// [`EDGE_LOW`], so cutting there changes nothing.
const REACH: f64 = 1.5;

/// The farmed radius of a settlement of type `ty`, m; `None` for a type that farms nothing.
fn radius_m(ty: &str) -> Option<f64> {
    let km = match ty {
        "city" => 2.0,
        "town" => 1.5,
        "village" => 1.0,
        "hamlet" => 0.55,
        _ => return None,
    };
    Some(km * 1000.0)
}

/// One settlement's farm, in pixels.
#[derive(Debug, Clone, Copy)]
struct Farm {
    /// Centre, pixels east.
    x: f64,
    /// Centre, pixels south.
    y: f64,
    /// Radius, pixels.
    r: f64,
}

/// The farms of every settlement of a render.
#[derive(Debug)]
pub(crate) struct Farmland {
    farms: Vec<Farm>,
}

/// The farms that reach one pixel row.
pub(crate) struct FarmRow<'a> {
    /// The row's centre, pixels south.
    y: f64,
    farms: Vec<&'a Farm>,
}

impl Farmland {
    /// The farms of the settlements of `features`, on pixels of `metres_per_px`.
    pub(crate) fn new(features: &FeatureSet, metres_per_px: f64) -> Self {
        let farms = features
            .of_kind("settlement")
            .filter_map(|(_, f)| {
                let r = radius_m(&f.ty)?;
                let Geometry::Point([x, y]) = f.geometry else {
                    return None;
                };
                Some(Farm {
                    x: x / metres_per_px,
                    y: y / metres_per_px,
                    r: r / metres_per_px,
                })
            })
            .collect();
        Self { farms }
    }

    /// The farms that reach pixel row `y`.
    pub(crate) fn row(&self, y: usize) -> FarmRow<'_> {
        let centre = to_f64(y) + 0.5;
        let farms = self
            .farms
            .iter()
            .filter(|f| (centre - f.y).abs() <= REACH * f.r)
            .collect();
        FarmRow { y: centre, farms }
    }
}

impl FarmRow<'_> {
    /// How much of the pixel in column `x` is farmland, in `[0, 1]`.
    pub(crate) fn share(&self, x: usize) -> f64 {
        let centre = to_f64(x) + 0.5;
        let g = self
            .farms
            .iter()
            .filter(|f| (centre - f.x).abs() <= REACH * f.r)
            .fold(0.0, |g, f| {
                let (dx, dy) = (centre - f.x, self.y - f.y);
                max(g, libm::exp(-(dx * dx + dy * dy) / (2.0 * f.r * f.r)))
            });
        smoothstep(EDGE_LOW, EDGE_HIGH, g)
    }
}

/// The sign of `v`, `0` for zero (numpy's `sign`).
fn sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// The field colour of the pixel in column `x`, row `y`: stripes of the two farm colours.
pub(crate) fn field(x: usize, y: usize) -> Unit {
    let (x, y) = (to_f64(x), to_f64(y));
    let stripes = libm::sin(x / 7.0 + 0.6 * y / 9.0) * libm::sin(y / 11.0);
    let t = 0.5 + 0.5 * sign(stripes);
    palette::mix(
        palette::unit(palette::FARM_A),
        palette::unit(palette::FARM_B),
        t,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::features::Feature;

    fn settlement(ty: &str, geometry: Geometry) -> Feature {
        Feature {
            kind: "settlement",
            ty: ty.to_owned(),
            geometry,
            props: BTreeMap::new(),
        }
    }

    #[test]
    fn farms_reach_their_class_radius() {
        let mut features = FeatureSet::default();
        // 10 m per pixel: a town farms 150 pixels around (500, 500).
        features.add(settlement("town", Geometry::Point([5000.0, 5000.0])));
        features.add(settlement("camp", Geometry::Point([100.0, 100.0])));
        features.add(settlement("city", Geometry::Line(vec![[0.0, 0.0, 0.0]])));
        let land = Farmland::new(&features, 10.0);
        assert_eq!(land.farms.len(), 1);
        let row = land.row(500);
        assert!((row.share(500) - 1.0).abs() < 1e-12);
        assert!(
            (row.share(500 + 140) - 1.0).abs() < 1e-12,
            "within one radius"
        );
        assert!(
            row.share(500 + 190) > 0.0 && row.share(500 + 190) < 1.0,
            "the edge"
        );
        assert_eq!(row.share(500 + 230).to_bits(), 0.0_f64.to_bits());
        assert!(land.row(10).farms.is_empty());
        assert_eq!(land.row(10).share(500).to_bits(), 0.0_f64.to_bits());
        assert_eq!(radius_m("hamlet"), Some(550.0));
        assert_eq!(radius_m("village"), Some(1000.0));
    }

    #[test]
    fn fields_are_stripes_of_the_two_farm_colours() {
        let a = palette::unit(palette::FARM_A);
        let b = palette::unit(palette::FARM_B);
        let mid = palette::mix(a, b, 0.5);
        // Row 0: sin(0) = 0, so the pattern is the even mix.
        assert_eq!(field(3, 0), mid);
        let colours: Vec<Unit> = (0..60).map(|x| field(x, 5)).collect();
        assert!(colours.contains(&a) && colours.contains(&b));
        assert_eq!((sign(2.0), sign(-0.5), sign(0.0)), (1.0, -1.0, 0.0));
    }
}
