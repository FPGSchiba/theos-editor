//! Spur tracks: the road from the network to a site, laid with the router of `roads`.

use std::collections::BTreeMap;

use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::kernels::KernelError;
use crate::kernels::num::to_u32;
use crate::stage::Finding;
use crate::stages::terrain::roads::{Laid, RoadClass, StepCost, end_near};
use crate::world::MapFrame;

/// The roads laid so far and the cost they are routed with.
pub(super) struct Spurs<'a> {
    cost: StepCost<'a>,
    frame: MapFrame,
    /// The first road through each path cell.
    first_road: BTreeMap<u32, u32>,
    /// The roads of the network, spurs included, and their raster.
    pub(super) laid: Laid,
    /// One warning per site no road could reach.
    pub(super) warnings: Vec<Finding>,
    /// How many spurs were laid.
    pub(super) count: usize,
}

impl<'a> Spurs<'a> {
    /// Spurs over `cost` joining the network `laid`.
    pub(super) fn new(cost: StepCost<'a>, frame: MapFrame, laid: Laid) -> Self {
        let mut first_road = BTreeMap::new();
        for (i, road) in laid.roads.iter().enumerate() {
            let id = to_u32(i);
            for &c in &road.cells {
                first_road.entry(c).or_insert(id);
            }
        }
        Self {
            cost,
            frame,
            first_road,
            laid,
            warnings: Vec::new(),
            count: 0,
        }
    }

    /// Adds `site` to `set` and joins it to the network: its `road` property is the road it
    /// stands on, else the spur of `class` routed to it from the nearest road cell. A map with
    /// no road, or a site no road can reach (a warning), gets no `road`.
    ///
    /// # Errors
    /// [`KernelError::Cancelled`] when `cancel` returns `true` during the search;
    /// [`KernelError::TooMany`] for a grid of more cells than a `u32` numbers.
    pub(super) fn connect(
        &mut self,
        set: &mut FeatureSet,
        mut site: Feature,
        class: RoadClass,
        cancel: &dyn Fn() -> bool,
    ) -> Result<FeatureHandle, KernelError> {
        let handle = FeatureHandle(to_u32(set.len()));
        if let Geometry::Point(at @ [x, y]) = site.geometry {
            let cell = self.frame.index_at_m(at);
            if let Some(road) = self.road_through(cell) {
                site.props.insert("road".to_owned(), Prop::Road(road));
            } else if let Some(road) = self.lay(cell, class, handle, cancel)? {
                site.props.insert("road".to_owned(), Prop::Road(road));
            } else if !self.laid.roads.is_empty() {
                self.warnings.push(Finding::warning(format!(
                    "no spur reaches the {} at ({x:.0} m, {y:.0} m): water or steep ground \
                     cuts it off from every road",
                    site.ty
                )));
            }
        }
        Ok(set.add(site))
    }

    /// The index of the first road whose path passes `cell`.
    fn road_through(&self, cell: usize) -> Option<u32> {
        self.first_road.get(&u32::try_from(cell).ok()?).copied()
    }

    /// Routes and lays the spur to `cell` from the nearest cell of any road; its index, or
    /// `None` when there is no road or no route.
    fn lay(
        &mut self,
        cell: usize,
        class: RoadClass,
        site: FeatureHandle,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Option<u32>, KernelError> {
        let sources: Vec<usize> = self.laid.cells().into_iter().collect();
        if sources.is_empty() {
            return Ok(None);
        }
        let path = self
            .laid
            .router(&self.cost)
            .route(&sources, cell, class, cancel)?;
        let Some(path) = path else { return Ok(None) };
        let joined = u32::try_from(path[0])
            .ok()
            .and_then(|c| end_near(&self.laid.roads, c))
            .unwrap_or(site);
        let index = self.laid.add(&self.cost, &path, class, joined, site);
        self.count += 1;
        let id = to_u32(index);
        for &c in &self.laid.roads[index].cells {
            self.first_road.entry(c).or_insert(id);
        }
        Ok(Some(id))
    }
}
