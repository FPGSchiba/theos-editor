//! The `sites` stage: the secondary places of a map, ported from 0.3.0's `features.py::sites`,
//! with spur tracks routed by the `roads` router.
//!
//! In 0.3.0's order, so that later sites see the spurs of earlier ones:
//! 1. **Mills** (`industry::mills`): for each town, village and city, the river cell with at
//!    least `mill_q_factor` × `river_q` of discharge nearest to it within 1.5 km (spec ruling 45).
//! 2. **Deposit sites** (`industry::deposit_sites`): iron, copper and silver ore give mines,
//!    stone a quarry, timber a lumber camp and resonance crystal a shrine, for deposits of at
//!    least `min_richness`. Each is linked to its deposit by `deposit` and joined to the network
//!    by a spur, a track (a trail to a shrine), from the nearest road cell by the least cost
//!    (`spurs`). Its `road` is the spur, or the road it stands on.
//! 3. **Watchtowers**, **inns** and **caves** (`scatter`, `inns`): on prominent ground
//!    150 to 900 m from a main road or road; at the midpoint of main roads and roads longer than
//!    6 km, 2 km from every settlement; in steep limestone.
//! 4. **Camps** (`scatter::camps`): remote, covered, gentle ground.
//!
//! Every site keeps `edge_margin` of the shorter side from the edge (spec "Margins"). 0.3.0's
//! lengths are metres here (cells of 62.5 m). The spurs are added to the world value
//! `road_network` and to the raster `roads`, where the higher class wins. Lore properties of
//! 0.3.0 (a shrine's faith, a cave's detail, a camp's faction and threat) are dropped.
//!
//! Draws from the stream `siting`: one noise value per cell in index order for the camps.
//!
//! The metrics are the number of each type of site and `spurs`, the spurs laid.

mod industry;
mod inns;
mod params;
mod scatter;
mod spurs;
#[cfg(test)]
mod tests;

use self::scatter::Ground;
use self::spurs::Spurs;
use crate::features::{Feature, FeatureSet, interior};
use crate::kernels::num::{to_f64, to_u32};
use crate::stage::{
    LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::hydrology::WaterThresholds;
use crate::stages::terrain::roads::{Laid, Layers, RoadNetwork, StepCost};
use crate::world::{Layer, ValueType, World};

pub use self::params::{Counts, SitesParams};

/// The site types the stage adds, with their metric names.
const TYPES: [(&str, &str); 9] = [
    ("mill", "mill"),
    ("mine", "mine"),
    ("quarry", "quarry"),
    ("lumber-camp", "lumber_camp"),
    ("shrine", "shrine"),
    ("watchtower", "watchtower"),
    ("inn", "inn"),
    ("cave", "cave"),
    ("camp", "camp"),
];

/// The stage `sites`.
pub struct Sites;

impl TypedStage for Sites {
    type Params = SitesParams;
    const ID: &'static str = "sites";
    const SUMMARY: &'static str = "Mills, mines, quarries, lumber camps, shrines, watchtowers, inns, caves and camps, with spur tracks.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &SitesParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("slope"),
            LayerSpec::u8("rock"),
            LayerSpec::f64("vegetation"),
            LayerSpec::mask("river"),
            LayerSpec::mask("big_river"),
            LayerSpec::mask("lake"),
            LayerSpec::u8("flow_dir"),
            LayerSpec::f64("discharge"),
            LayerSpec::value("water_thresholds", WaterThresholds::NAME),
            LayerSpec::value("road_network", RoadNetwork::NAME),
            LayerSpec::u8("roads"),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &SitesParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("features", FeatureSet::NAME),
            LayerSpec::value("road_network", RoadNetwork::NAME),
            LayerSpec::u8("roads"),
        ]
    }

    fn metric_names(&self, _: &SitesParams) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = TYPES.iter().map(|&(_, metric)| metric).collect();
        names.push("spurs");
        names
    }

    fn validate(&self, params: &mut SitesParams) -> Result<(), ParamError> {
        params.check(Self::ID)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &SitesParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let layers = Layers::read(ctx)?;
        let slope = ctx.f64("slope")?.clone();
        let rock = ctx.u8("rock")?.clone();
        let discharge = ctx.f64("discharge")?.clone();
        let river_q = ctx.value::<WaterThresholds>("water_thresholds")?.river_q;
        let raster = ctx.u8("roads")?.clone();
        let network = *ctx.take_value::<RoadNetwork>("road_network")?;
        let mut set = *ctx.take_value_or_default::<FeatureSet>("features")?;
        let mut rng = ctx.sub_rng("siting");
        let inside = interior(&frame, params.edge_margin);
        let cost = StepCost::new(layers.terrain(), &network.params, &frame);
        let mut spurs = Spurs::new(cost, frame, Laid::resume(network.roads.clone(), raster));
        let mut found: Vec<Feature> = Vec::new();

        let water = industry::River {
            cells: &layers.river,
            discharge: &discharge,
            min_discharge: params.mill_q_factor * river_q,
            inside: &inside,
        };
        for mill in industry::mills(&set, &water, &frame) {
            found.push(mill.clone());
            set.add(mill);
        }
        let wanted = industry::deposit_sites(&set, params.min_richness, &inside, &frame);
        let total = to_u32(wanted.len());
        for (done, (site, class)) in wanted.into_iter().enumerate() {
            found.push(site.clone());
            let cancel = || ctx.is_cancelled();
            spurs
                .connect(&mut set, site, class, &cancel)
                .map_err(|e| ctx.kernel_failed(e))?;
            ctx.progress(to_u32(done + 1), total);
        }

        let settlements = set.points("settlement", None);
        let ground = Ground {
            height: &layers.height,
            slope: &slope,
            rock: &rock,
            vegetation: &layers.vegetation,
            roads: &spurs.laid.raster,
            inside: &inside,
            frame,
        };
        let mut scattered = scatter::watchtowers(&ground, params.counts.watchtower);
        scattered.extend(inns::inns(&spurs.laid.roads, &set, &inside, &frame));
        scattered.extend(scatter::caves(&ground, params.counts.cave));
        scattered.extend(scatter::camps(
            &ground,
            params.counts.camp,
            &settlements,
            &mut rng,
        ));
        for site in scattered {
            found.push(site.clone());
            set.add(site);
        }

        let mut report = StageReport::default();
        for (ty, metric) in TYPES {
            let n = found.iter().filter(|f| f.ty == ty).count();
            report.values.insert(metric.to_owned(), to_f64(n));
        }
        report
            .values
            .insert("spurs".to_owned(), to_f64(spurs.count));
        report.findings = std::mem::take(&mut spurs.warnings);
        let laid = spurs.laid;
        ctx.write("features", Layer::Value(Box::new(set)))?;
        ctx.write(
            "road_network",
            Layer::Value(Box::new(RoadNetwork {
                roads: laid.roads,
                params: network.params.clone(),
            })),
        )?;
        ctx.write("roads", Layer::U8(laid.raster))?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &SitesParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}
