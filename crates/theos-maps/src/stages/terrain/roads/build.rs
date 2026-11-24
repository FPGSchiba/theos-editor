//! Building the network: the links, then the trails, with progress, cancel and warnings.

use std::collections::BTreeSet;

use super::lay::Laid;
use super::network::{Watch, choose, laying_order};
use super::places::{Place, places, ruins};
use super::{Road, RoadClass, RoadsParams, StepCost, Terrain, end_near, network};
use crate::features::{FeatureHandle, FeatureSet};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::num::to_u32;
use crate::stage::{Finding, StageContext, StageError};
use crate::world::MapFrame;

/// The rasters `roads` reads, copied out of the context so that it stays free for progress
/// events during the searches.
pub(crate) struct Layers {
    /// `height`.
    pub height: Grid<f64>,
    /// `vegetation`.
    pub vegetation: Grid<f64>,
    /// `river`.
    pub river: Grid<bool>,
    /// `big_river`.
    pub big_river: Grid<bool>,
    /// `lake`.
    pub lake: Grid<bool>,
    /// `flow_dir`.
    pub flow_dir: Grid<u8>,
}

impl Layers {
    /// Copies the layers out of `ctx`.
    ///
    /// # Errors
    /// [`StageError`] when a layer is missing or has another type.
    pub(crate) fn read(ctx: &StageContext<'_>) -> Result<Self, StageError> {
        Ok(Self {
            height: ctx.f64("height")?.clone(),
            vegetation: ctx.f64("vegetation")?.clone(),
            river: ctx.mask("river")?.clone(),
            big_river: ctx.mask("big_river")?.clone(),
            lake: ctx.mask("lake")?.clone(),
            flow_dir: ctx.u8("flow_dir")?.clone(),
        })
    }

    /// The layers as the terrain of [`StepCost`].
    pub(crate) fn terrain(&self) -> Terrain<'_> {
        Terrain {
            height: &self.height,
            vegetation: &self.vegetation,
            river: &self.river,
            big_river: &self.big_river,
            lake: &self.lake,
            flow_dir: &self.flow_dir,
        }
    }
}

/// The network built.
pub(super) struct Built {
    /// The roads in the order laid.
    pub roads: Vec<Road>,
    /// The `roads` raster.
    pub raster: Grid<u8>,
    /// One warning per place no road reaches and per ruin no trail reaches.
    pub warnings: Vec<Finding>,
    /// How many places no road reaches.
    pub unreached: usize,
}

/// Counts the routed links into the context's progress events.
struct Counter<'c, 'w> {
    ctx: &'c mut StageContext<'w>,
    done: usize,
    total: usize,
}

impl Watch for Counter<'_, '_> {
    fn cancelled(&self) -> bool {
        self.ctx.is_cancelled()
    }

    fn advance(&mut self, links: usize) {
        self.done += links;
        self.ctx.progress(to_u32(self.done), to_u32(self.total));
    }
}

impl Counter<'_, '_> {
    /// The stage error for `err`.
    fn fail(&self, err: KernelError) -> StageError {
        self.ctx.kernel_failed(err)
    }
}

/// Builds the network of the settlements and gates of `set` over `layers` with `params` (see
/// the [module docs](super)); fewer than two places build nothing.
///
/// # Errors
/// [`StageError::Cancelled`] when the run is cancelled during a search;
/// [`StageError::Failed`] when the places cannot be triangulated.
pub(super) fn build(
    layers: &Layers,
    set: &FeatureSet,
    frame: &MapFrame,
    params: &RoadsParams,
    ctx: &mut StageContext<'_>,
) -> Result<Built, StageError> {
    let cost = StepCost::new(layers.terrain(), params, frame);
    let mut laid = Laid::new(frame.rows, frame.cols);
    let places = places(set, frame);
    if places.len() < 2 {
        return Ok(Built {
            roads: laid.roads,
            raster: laid.raster,
            warnings: Vec::new(),
            unreached: 0,
        });
    }
    let ruins = ruins(set, frame);
    let pairs = network::candidates(&places).map_err(|e| ctx.kernel_failed(e))?;
    let mut watch = Counter {
        ctx,
        done: 0,
        total: 2 * pairs.len() + ruins.len(),
    };
    links(&mut laid, &cost, &places, &pairs, &mut watch)?;
    let unreached: Vec<&Place> = places.iter().filter(|p| !laid.reaches(p.handle)).collect();
    let mut warnings: Vec<Finding> = unreached
        .iter()
        .map(|p| {
            Finding::warning(format!(
                "no road reaches {}: every link to it is blocked by water or too steep",
                p.label
            ))
        })
        .collect();
    warnings.extend(trails(&mut laid, &cost, &places, ruins, &mut watch)?);
    Ok(Built {
        roads: laid.roads,
        raster: laid.raster,
        warnings,
        unreached: unreached.len(),
    })
}

/// Costs the candidate `pairs`, chooses the links and lays them.
fn links(
    laid: &mut Laid,
    cost: &StepCost<'_>,
    places: &[Place],
    pairs: &[(usize, usize)],
    watch: &mut Counter<'_, '_>,
) -> Result<(), StageError> {
    let links =
        network::cost(&laid.router(cost), places, pairs, watch).map_err(|e| watch.fail(e))?;
    let chosen = choose(places, &links, cost.params.loop_detour);
    watch.advance(pairs.len() - chosen.len());
    for link in laying_order(chosen) {
        let (a, b) = (&places[link.a], &places[link.b]);
        let router = laid.router(cost);
        let cancel = || watch.cancelled();
        let path = if link.looped {
            router.route_fresh(&[a.cell], b.cell, link.class, &cancel)
        } else {
            router.route(&[a.cell], b.cell, link.class, &cancel)
        }
        .map_err(|e| watch.fail(e))?;
        // A link is laid with its own cost or less, so its route always exists.
        if let Some(path) = path {
            laid.add(cost, &path, link.class, a.handle, b.handle);
        }
        watch.advance(1);
    }
    Ok(())
}

/// Lays a trail to each of `ruins` from the road cell or place it reaches at the least cost;
/// returns a warning for each ruin no trail reaches. A ruin on a road or place needs none.
fn trails(
    laid: &mut Laid,
    cost: &StepCost<'_>,
    places: &[Place],
    ruins: Vec<(FeatureHandle, usize, String)>,
    watch: &mut Counter<'_, '_>,
) -> Result<Vec<Finding>, StageError> {
    let mut warnings = Vec::new();
    let mut sources: BTreeSet<usize> = laid.cells();
    sources.extend(places.iter().map(|p| p.cell));
    for (ruin, cell, label) in ruins {
        if !sources.contains(&cell) {
            let starts: Vec<usize> = sources.iter().copied().collect();
            let path = laid
                .router(cost)
                .route(&starts, cell, RoadClass::Trail, &|| watch.cancelled())
                .map_err(|e| watch.fail(e))?;
            match path {
                Some(path) => {
                    let from = joined(places, &laid.roads, path[0]).unwrap_or(ruin);
                    laid.add(cost, &path, RoadClass::Trail, from, ruin);
                    sources.extend(path);
                }
                None => warnings.push(Finding::warning(format!(
                    "no trail reaches {label}: it is cut off by water or too steep"
                ))),
            }
        }
        watch.advance(1);
    }
    Ok(warnings)
}

/// The feature a trail starting at `cell` comes from: the place at `cell`, else the end of the
/// road through it ([`end_near`]).
fn joined(places: &[Place], roads: &[Road], cell: usize) -> Option<FeatureHandle> {
    places
        .iter()
        .find(|p| p.cell == cell)
        .map(|p| p.handle)
        .or_else(|| end_near(roads, u32::try_from(cell).ok()?))
}
