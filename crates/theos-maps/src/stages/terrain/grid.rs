//! The `grid` stage: which edges of the map drain.

use std::any::Any;

use serde::Deserialize;

use crate::grid::Grid;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::{Layer, ValueType, WorldValue};

/// The sides of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    /// Row 0.
    North,
    /// The last column.
    East,
    /// The last row.
    South,
    /// Column 0.
    West,
}

/// Which edges drain; the others are closed walls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four sides of a map, as the `frame` value defines them"
)]
pub struct Edges {
    /// Row 0 drains.
    pub north: bool,
    /// The last column drains.
    pub east: bool,
    /// The last row drains.
    pub south: bool,
    /// Column 0 drains.
    pub west: bool,
}

impl Edges {
    /// The edges named in `list`.
    #[must_use]
    pub fn from_list(list: &[Edge]) -> Self {
        let has = |edge| list.contains(&edge);
        Self {
            north: has(Edge::North),
            east: has(Edge::East),
            south: has(Edge::South),
            west: has(Edge::West),
        }
    }
}

/// The world value `frame`: the drained edges of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameValue {
    /// The drained edges.
    pub drained: Edges,
}

impl WorldValue for FrameValue {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        // Booleans only: nothing to check.
        let d = self.drained;
        Ok(serde_json::json!({"drained": {
            "north": d.north, "east": d.east, "south": d.south, "west": d.west,
        }}))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for FrameValue {
    const NAME: &'static str = "frame";
}

/// The parameters of `grid`.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GridParams {
    /// The drained edges; the rest are closed walls.
    pub edges: Vec<Edge>,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            edges: vec![Edge::South],
        }
    }
}

/// The stage `grid`: raster domain and base level.
pub struct GridStage;

impl TypedStage for GridStage {
    type Params = GridParams;
    const ID: &'static str = "grid";
    const SUMMARY: &'static str =
        "Raster domain and base level: which edges of the map drain (the rest are closed walls).";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &GridParams) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn writes(&self, _: &GridParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("frame", FrameValue::NAME),
            LayerSpec::mask("fixed"),
        ]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &GridParams,
    ) -> Result<StageReport, StageError> {
        let (rows, cols) = (ctx.frame().rows, ctx.frame().cols);
        let drained = Edges::from_list(&params.edges);
        let mut fixed = Grid::filled(rows, cols, false);
        for r in 0..rows {
            for c in 0..cols {
                let on = (drained.north && r == 0)
                    || (drained.south && r + 1 == rows)
                    || (drained.west && c == 0)
                    || (drained.east && c + 1 == cols);
                fixed.set(r, c, on);
            }
        }
        ctx.write("frame", Layer::Value(Box::new(FrameValue { drained })))?;
        ctx.write("fixed", Layer::Mask(fixed))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;
    use crate::stages::terrain::support::{run_stage, world};

    fn mask_of(rows: usize, cols: usize, toml: &str) -> Vec<(usize, usize)> {
        let mut w = world(rows, cols);
        run_stage(&mut w, &GridStage, toml).unwrap();
        let mask = w.mask("fixed").unwrap();
        (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .filter(|&(r, c)| mask.get(r, c))
            .collect()
    }

    #[test]
    fn drained_edges_are_marked_on_their_sides() {
        let marked = mask_of(6, 8, "edges = [\"north\"]");
        assert_eq!(marked, (0..8).map(|c| (0, c)).collect::<Vec<_>>());
        let default = mask_of(6, 8, "");
        assert_eq!(default, (0..8).map(|c| (5, c)).collect::<Vec<_>>());
        let west_east = mask_of(3, 4, "edges = [\"west\", \"east\"]");
        assert_eq!(west_east.len(), 6);
        assert!(west_east.iter().all(|&(_, c)| c == 0 || c == 3));
    }

    #[test]
    fn closed_and_open_grids_are_valid() {
        assert_eq!(mask_of(5, 5, "edges = []").len(), 0);
        let rim = mask_of(5, 6, "edges = [\"north\", \"east\", \"south\", \"west\"]");
        assert_eq!(rim.len(), 5 * 6 - 3 * 4);
        assert!(
            rim.iter()
                .all(|&(r, c)| r == 0 || r == 4 || c == 0 || c == 5)
        );
    }

    #[test]
    fn the_frame_value_records_the_drained_edges() {
        let mut w = world(4, 4);
        run_stage(&mut w, &GridStage, "edges = [\"east\", \"south\"]").unwrap();
        let frame = w.value::<FrameValue>("frame").unwrap();
        assert_eq!(
            frame.drained,
            Edges {
                north: false,
                east: true,
                south: true,
                west: false
            }
        );
        assert_eq!(
            frame.canonical_json().unwrap(),
            serde_json::json!({"drained": {"north": false, "east": true, "south": true, "west": false}})
        );
        assert_eq!(frame.type_name(), "frame");
    }

    #[test]
    fn unknown_edge_names_are_parameter_errors() {
        let err = GridStage
            .params(&crate::stages::terrain::support::table("edges = [\"up\"]"))
            .unwrap_err();
        assert!(err.to_string().contains("up"), "{err}");
        let err = GridStage
            .params(&crate::stages::terrain::support::table("bogus = 1"))
            .unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("bogus"));
    }
}
