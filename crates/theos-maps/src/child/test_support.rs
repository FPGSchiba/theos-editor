//! A synthetic parent package, test stages that read parent values and recipes that use them,
//! for the tests of planning and running a child map. Everything here is synthetic.

use std::fmt::Write as _;
use std::io::Cursor;
use std::path::PathBuf;

use geojson::{Feature, FeatureCollection, GeometryValue, JsonObject};
use serde::{Deserialize, Serialize};
use theos_map_format::{
    DType, FORMAT, FORMAT_VERSION, Frame, LayerData, LayerEntry, LayerKey, Manifest, Package,
};

use super::ParentContext;
use crate::recipe::{LoadedRecipe, RecipeOrigin, parse};
use crate::stage::{
    LayerSpec, MapKind, Registry, Runs, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::Layer;

/// Parameters of a stage that takes none.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Knob {}

const KINDS: &[MapKind] = &[MapKind::Settlement, MapKind::Interior];

/// A stage that reads `$reads`, writes `$writes` (copying the first read into it when both are
/// rasters) and runs for `$runs`.
macro_rules! stage {
    ($name:ident, $id:literal, $runs:expr, reads: [$($reads:expr),*], writes: [$($writes:expr),*]) => {
        pub(crate) struct $name;
        impl TypedStage for $name {
            type Params = Knob;
            const ID: &'static str = $id;
            const SUMMARY: &'static str = "A synthetic test stage.";
            const KINDS: &'static [MapKind] = KINDS;
            const RUNS: Runs = $runs;
            fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
                vec![$($reads),*]
            }
            fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
                vec![$($writes),*]
            }
            fn run(&self, ctx: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
                run_copy(ctx, &[$($reads),*], &[$($writes),*])
            }
        }
    };
}

/// Copies the first float read into the first float write; reports the first draw of the
/// stage's own random stream as `draw`.
fn run_copy(
    ctx: &mut StageContext<'_>,
    reads: &[LayerSpec],
    writes: &[LayerSpec],
) -> Result<StageReport, StageError> {
    if let (Some(read), Some(write)) = (reads.first(), writes.first())
        && read.kind == crate::world::LayerKind::F64
    {
        let grid = ctx.f64(read.name)?.clone();
        ctx.write(write.name, Layer::F64(grid))?;
    } else if let Some(write) = writes.first() {
        let (rows, cols) = (ctx.frame().rows, ctx.frame().cols);
        ctx.write(
            write.name,
            Layer::F64(crate::grid::Grid::filled(rows, cols, 1.0)),
        )?;
    }
    let mut report = StageReport::default();
    report.put("draw", ctx.rng().uniform());
    Ok(report)
}

stage!(Height, "ph", Runs::Child, reads: [LayerSpec::f64("parent.height")], writes: [LayerSpec::f64("h")]);
stage!(Discharge, "pd", Runs::Child, reads: [LayerSpec::f64("parent.discharge")], writes: []);
stage!(Water, "pw", Runs::Child, reads: [LayerSpec::f64("parent.water")], writes: [LayerSpec::f64("w")]);
stage!(Alone, "sa", Runs::Standalone, reads: [], writes: [LayerSpec::f64("s")]);
stage!(Plain, "b", Runs::Always, reads: [], writes: [LayerSpec::f64("y")]);
stage!(OnlyChild, "a", Runs::Child, reads: [], writes: [LayerSpec::f64("x")]);
stage!(
    Feats,
    "pf",
    Runs::Child,
    reads: [LayerSpec::value("parent.features", "parent_features")],
    writes: []
);

stage!(
    Class,
    "cf",
    Runs::Always,
    reads: [LayerSpec::value("class_frame", "class_frame")],
    writes: []
);

/// A registry of the test stages.
pub(crate) fn registry() -> Registry {
    add_test_stages(Registry::empty())
}

/// The built-in stages and the test stages, for a parent run and a test child in one registry.
pub(crate) fn registry_with_builtin() -> Registry {
    add_test_stages(Registry::builtin())
}

fn add_test_stages(mut registry: Registry) -> Registry {
    registry.add(Box::new(Height));
    registry.add(Box::new(Discharge));
    registry.add(Box::new(Water));
    registry.add(Box::new(Alone));
    registry.add(Box::new(Plain));
    registry.add(Box::new(OnlyChild));
    registry.add(Box::new(Feats));
    registry.add(Box::new(Class));
    registry
}

/// An interior-kind child recipe (8 x 8 cells of 100 m) with the given stage ids, plus a
/// `site_terrain`-less settlement variant through [`settlement_recipe`].
pub(crate) fn recipe(stages: &[&str]) -> LoadedRecipe {
    parse(
        &text("interior", "grid = [8, 8]\nsize_km = [0.8, 0.8]\n", stages),
        RecipeOrigin::Builtin,
    )
    .unwrap()
}

/// A settlement-kind child recipe with the given stage ids.
pub(crate) fn settlement_recipe(stages: &[&str]) -> LoadedRecipe {
    parse(&text("settlement", "", stages), RecipeOrigin::Builtin).unwrap()
}

fn text(kind: &str, frame: &str, stages: &[&str]) -> String {
    let mut text = format!(
        "id = \"child\"\ntitle = \"Child\"\nkind = \"{kind}\"\n{frame}\n\
         [inputs.feature]\ntype = \"feature\"\nparent_types = [\"town\", \"village\"]\n"
    );
    for stage in stages {
        let _ = write!(text, "[[stage]]\nid = \"{stage}\"\n");
    }
    text
}

fn feature(id: &str, ty: &str, at: [f64; 2]) -> Feature {
    let mut props = JsonObject::new();
    props.insert("kind".to_owned(), "settlement".into());
    props.insert("type".to_owned(), ty.into());
    Feature {
        bbox: None,
        geometry: Some(geojson::Geometry::new(GeometryValue::new_point(
            at.to_vec(),
        ))),
        id: Some(geojson::feature::Id::String(id.to_owned())),
        properties: Some(props),
        foreign_members: None,
    }
}

fn entry(name: &str, dtype: DType, legend: Option<&[(&str, &str)]>) -> LayerEntry {
    LayerEntry {
        name: name.to_owned(),
        file: String::new(),
        dtype,
        shape: [4, 4],
        sha256: String::new(),
        description: "synthetic".to_owned(),
        units: None,
        scale: None,
        offset: None,
        legend: legend.map(|l| {
            l.iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect()
        }),
        min: None,
        max: None,
        level: None,
    }
}

/// A quantised `u8` layer: `offset + raw * scale`, with a legend when it is a class layer.
fn scaled(
    name: &str,
    scale: f64,
    offset: Option<f64>,
    legend: Option<&[(&str, &str)]>,
) -> LayerEntry {
    LayerEntry {
        scale: Some(scale),
        offset,
        ..entry(name, DType::U8, legend)
    }
}

/// A 4,000 m parent of 4 x 4 cells of 1,000 m: a `height` (`f32`), a class layer `water` with a
/// legend, a town at (2,000, 2,000), a village inside its window and a hamlet outside it.
pub(crate) fn package() -> Package {
    let height: Vec<f32> = (0u8..16).map(|i| f32::from(i) * 1.5 + 0.25).collect();
    let water: Vec<u8> = (0u8..16).map(|i| if i % 5 == 0 { 2 } else { 0 }).collect();
    let codes: Vec<u8> = (0u8..16).map(|i| i * 16).collect();
    let layer = |name: &str| LayerKey {
        level: None,
        name: name.to_owned(),
    };
    Package {
        manifest: Manifest {
            format: FORMAT.to_owned(),
            format_version: FORMAT_VERSION.to_owned(),
            generator: "test@0".to_owned(),
            requires: Vec::new(),
            kind: "terrain".to_owned(),
            recipe_id: "synthetic-parent".to_owned(),
            recipe: serde_json::json!({}),
            seed: 1,
            inputs: serde_json::Map::new(),
            frame: Frame {
                size_m: [4000.0, 4000.0],
                cell_m: 1000.0,
                grid: [4, 4],
                origin_m: None,
                parent: None,
                levels: None,
            },
            layers: vec![
                entry("height", DType::F32, None),
                entry("water", DType::U8, Some(&[("0", "dry"), ("2", "river")])),
                scaled("vegetation", 1.0 / 255.0, None, None),
                scaled("moisture", 0.5, Some(10.0), None),
                scaled(
                    "soil",
                    2.0,
                    Some(1.0),
                    Some(&[("1", "clay"), ("3", "sand")]),
                ),
            ],
            features: String::new(),
            results: serde_json::Map::new(),
            summary: serde_json::json!({}),
        },
        features: FeatureCollection {
            bbox: None,
            features: vec![
                feature("town-1", "town", [2000.0, 2000.0]),
                feature("village-1", "village", [2300.0, 2200.0]),
                feature("hamlet-1", "hamlet", [3900.0, 3900.0]),
            ],
            foreign_members: None,
        },
        layers: [
            (layer("height"), LayerData::F32(height)),
            (layer("water"), LayerData::U8(water)),
            (layer("vegetation"), LayerData::U8(codes.clone())),
            (layer("moisture"), LayerData::U8(codes.clone())),
            (layer("soil"), LayerData::U8(codes)),
        ]
        .into(),
    }
}

/// The parent, written to `dir` and opened on the feature `id`.
pub(crate) fn open(dir: &tempfile::TempDir, id: &str) -> ParentContext {
    let mut bytes = Vec::new();
    theos_map_format::write(&package(), Cursor::new(&mut bytes)).unwrap();
    let path: PathBuf = dir.path().join("parent.theos-map");
    std::fs::write(&path, bytes).unwrap();
    let types = ["town", "village"].map(str::to_owned);
    ParentContext::open(&path, "feature", id, &types).unwrap()
}
