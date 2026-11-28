use std::io::Cursor;
use std::path::PathBuf;

use geojson::{Feature, FeatureCollection, GeometryValue, JsonObject};
use theos_map_format::{
    DType, FORMAT, FORMAT_VERSION, Frame, LayerData, LayerEntry, LayerKey, Manifest, Package,
    sha256_hex,
};

use super::{ChildError, ChildFrame, ParentContext, child_frame, feature_id, standalone_frame};
use crate::features::Geometry;
use crate::recipe::{LoadedRecipe, RecipeError, RecipeOrigin, parse};
use crate::stage::MapKind;

const ACCEPTED: [&str; 4] = ["city", "town", "village", "hamlet"];

fn accepted() -> Vec<String> {
    ACCEPTED.iter().map(|s| (*s).to_owned()).collect()
}

fn feature(id: &str, ty: &str, geometry: GeometryValue) -> Feature {
    let mut props = JsonObject::new();
    props.insert("kind".to_owned(), "settlement".into());
    props.insert("type".to_owned(), ty.into());
    props.insert("name".to_owned(), "Somewhere".into());
    Feature {
        bbox: None,
        geometry: Some(geojson::Geometry::new(geometry)),
        id: Some(geojson::feature::Id::String(id.to_owned())),
        properties: Some(props),
        foreign_members: None,
    }
}

fn point(id: &str, ty: &str, at: [f64; 2]) -> Feature {
    feature(id, ty, GeometryValue::new_point(at.to_vec()))
}

/// A synthetic 4,000 m parent with one 4 x 4 layer and the given features.
fn package(features: Vec<Feature>) -> Package {
    let key = LayerKey {
        level: None,
        name: "height".to_owned(),
    };
    let values: Vec<f32> = (0u8..16).map(|i| f32::from(i) * 1.5 + 0.25).collect();
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
            layers: vec![LayerEntry {
                name: "height".to_owned(),
                file: String::new(),
                dtype: DType::F32,
                shape: [4, 4],
                sha256: String::new(),
                description: "synthetic height".to_owned(),
                units: Some("m".to_owned()),
                scale: None,
                offset: None,
                legend: None,
                min: None,
                max: None,
                level: None,
            }],
            features: String::new(),
            results: serde_json::Map::new(),
            summary: serde_json::json!({}),
        },
        features: FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        },
        layers: [(key, LayerData::F32(values))].into(),
    }
}

fn bytes_of(package: &Package) -> Vec<u8> {
    let mut bytes = Vec::new();
    theos_map_format::write(package, Cursor::new(&mut bytes)).unwrap();
    bytes
}

/// Writes the parent into `dir` and returns its path.
fn write_parent(dir: &tempfile::TempDir, features: Vec<Feature>) -> PathBuf {
    let path = dir.path().join("parent.theos-map");
    std::fs::write(&path, bytes_of(&package(features))).unwrap();
    path
}

fn open(path: &std::path::Path, id: &str) -> Result<ParentContext, ChildError> {
    ParentContext::open(path, "feature", id, &accepted())
}

fn recipe(extra: &str) -> LoadedRecipe {
    parse(
        &format!(
            "id = \"town-map\"\ntitle = \"Town\"\nkind = \"settlement\"\n{extra}\n\
             [inputs]\nfeature = {{ type = \"feature\", parent_types = [\"city\", \"town\", \"village\", \"hamlet\"] }}\n"
        ),
        RecipeOrigin::Builtin,
    )
    .unwrap()
}

/// The frame of a feature of type `ty` at `at` in a 4,000 m parent.
fn frame_of(ty: &str, at: [f64; 2], extra: &str) -> Result<ChildFrame, RecipeError> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("x-1", ty, at)]);
    let parent = open(&path, "x-1").unwrap();
    child_frame(MapKind::Settlement, &recipe(extra).recipe, &parent)
}

#[test]
fn a_town_window_is_400_cells_of_4_m_centred_on_the_feature() {
    let frame = frame_of("town", [2000.0, 1500.0], "").unwrap();
    assert_eq!((frame.rows, frame.cols), (400, 400));
    assert_eq!(frame.cell_m, 4.0);
    assert_eq!(frame.origin_m, [1200.0, 700.0]);
}

#[test]
fn a_village_window_has_333_cells_of_3_m() {
    let frame = frame_of("village", [2000.0, 1500.0], "").unwrap();
    assert_eq!((frame.rows, frame.cols), (333, 333));
    assert_eq!(frame.cell_m, 3.0);
}

#[test]
fn a_window_past_the_parent_corner_keeps_its_size_and_a_negative_origin() {
    let frame = frame_of("town", [100.0, 100.0], "").unwrap();
    assert_eq!(frame.origin_m, [-700.0, -700.0]);
    assert_eq!((frame.rows, frame.cols), (400, 400));
}

#[test]
fn the_origin_snaps_to_whole_child_cells() {
    let frame = frame_of("town", [2001.3, 1499.0], "").unwrap();
    assert_eq!(frame.origin_m, [1200.0, 700.0]);
}

#[test]
fn recipe_class_tables_override_the_defaults() {
    let frame = frame_of("town", [2000.0, 1500.0], "[child.class.town]\ncell_m = 5.0").unwrap();
    assert_eq!((frame.rows, frame.cols), (320, 320));
    assert_eq!(frame.cell_m, 5.0);
}

#[test]
fn a_class_table_with_an_unknown_key_is_refused() {
    let err = frame_of("town", [2000.0, 1500.0], "[child.class.town]\ncell = 5.0").unwrap_err();
    assert!(err.to_string().contains("cell"), "{err}");
}

#[test]
fn a_feature_id_the_parent_lacks_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("town-1", "town", [10.0, 10.0])]);
    let err = open(&path, "town-9").unwrap_err();
    assert!(
        matches!(&err, ChildError::UnknownFeature { id, path: p } if id == "town-9" && *p == path)
    );
    assert!(err.to_string().contains("town-9"));
    assert!(err.to_string().contains("parent.theos-map"));
}

#[test]
fn a_ruin_is_refused_by_a_settlement_recipe() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("ruin-1", "ruin", [10.0, 10.0])]);
    let err = open(&path, "ruin-1").unwrap_err();
    match err {
        ChildError::WrongType { id, ty, accepted } => {
            assert_eq!((id.as_str(), ty.as_str()), ("ruin-1", "ruin"));
            assert_eq!(accepted, ACCEPTED);
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
}

#[test]
fn a_polygon_feature_is_centred_on_its_bounding_box() {
    let ring = vec![
        vec![100.0, 100.0],
        vec![300.0, 100.0],
        vec![300.0, 500.0],
        vec![100.0, 500.0],
        vec![100.0, 100.0],
    ];
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(
        &dir,
        vec![feature(
            "town-1",
            "town",
            GeometryValue::new_polygon([ring]),
        )],
    );
    let parent = open(&path, "town-1").unwrap();
    assert!(matches!(parent.feature.geometry, Geometry::Polygon(_)));
    assert_eq!(parent.anchor_m().unwrap(), [200.0, 300.0]);
}

#[test]
fn a_line_or_a_feature_without_geometry_has_no_anchor() {
    let line = feature(
        "town-1",
        "town",
        GeometryValue::new_line_string([vec![0.0, 0.0], vec![5.0, 5.0]]),
    );
    let mut bare = point("town-2", "town", [0.0, 0.0]);
    bare.geometry = None;
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![line, bare]);
    for id in ["town-1", "town-2"] {
        let err = open(&path, id).unwrap().anchor_m().unwrap_err();
        assert!(
            matches!(err, ChildError::NoAnchor { id: ref i } if i == id),
            "{err:?}"
        );
    }
}

#[test]
fn a_parent_with_a_broken_hash_is_refused_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = bytes_of(&package(vec![point("town-1", "town", [1.0, 1.0])]));
    let name = b"layers/height";
    let at = bytes.windows(name.len()).position(|w| w == name).unwrap();
    let flip = at + name.len() + 6;
    bytes[flip] ^= 0xff;
    let path = dir.path().join("broken.theos-map");
    std::fs::write(&path, bytes).unwrap();
    let err = open(&path, "town-1").unwrap_err();
    assert!(matches!(err, ChildError::Read { .. }), "{err:?}");
    assert!(err.to_string().contains("broken.theos-map"), "{err}");
}

#[test]
fn a_missing_parent_file_is_an_io_error_naming_it() {
    let dir = tempfile::tempdir().unwrap();
    let err = open(&dir.path().join("none.theos-map"), "town-1").unwrap_err();
    assert!(matches!(err, ChildError::Io { .. }), "{err:?}");
    assert!(err.to_string().contains("none.theos-map"));
}

#[test]
fn the_package_hash_is_of_the_file_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("town-1", "town", [1.0, 1.0])]);
    let parent = open(&path, "town-1").unwrap();
    assert_eq!(parent.sha256, sha256_hex(&std::fs::read(&path).unwrap()));
    assert_eq!(parent.input, "feature");
    assert_eq!(parent.feature.id, "town-1");
    assert_eq!(
        (parent.feature.kind.as_str(), parent.feature.ty.as_str()),
        ("settlement", "town")
    );
    assert_eq!(parent.feature.properties["name"], "Somewhere");
    assert_eq!(parent.package.manifest.frame.grid, [4, 4]);
}

#[test]
fn a_feature_id_that_is_not_a_string_is_refused() {
    let given = serde_json::json!({"feature": 3});
    let err = feature_id(given.as_object().unwrap(), "feature").unwrap_err();
    assert!(matches!(err, ChildError::NotAFeatureId { ref input } if input == "feature"));
    let given = serde_json::json!({"feature": "town-1"});
    assert_eq!(
        feature_id(given.as_object().unwrap(), "feature").unwrap(),
        "town-1"
    );
}

#[test]
fn an_interior_frame_is_centred_with_the_recipes_size() {
    let loaded = parse(
        "id = \"hut\"\ntitle = \"Hut\"\nkind = \"interior\"\ngrid = [20, 40]\nsize_km = [0.02, 0.01]\n",
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("building-1", "hut", [1000.0, 1000.0])]);
    let parent = ParentContext::open(&path, "feature", "building-1", &["hut".to_owned()]).unwrap();
    let frame = child_frame(MapKind::Interior, &loaded.recipe, &parent).unwrap();
    assert_eq!((frame.rows, frame.cols), (20, 40));
    assert_eq!(frame.cell_m, 0.5);
    assert_eq!(frame.origin_m, [990.0, 995.0]);
}

const TERRAIN: &str =
    "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [4, 8]\nsize_km = [2.0, 1.0]\n";

#[test]
fn a_terrain_map_has_no_child_frame() {
    let loaded = parse(TERRAIN, RecipeOrigin::Builtin).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = write_parent(&dir, vec![point("town-1", "town", [1.0, 1.0])]);
    let parent = open(&path, "town-1").unwrap();
    let err = child_frame(MapKind::Terrain, &loaded.recipe, &parent).unwrap_err();
    assert!(err.to_string().contains("terrain"), "{err}");
}

#[test]
fn a_standalone_terrain_frame_comes_from_grid_and_size() {
    let loaded = parse(TERRAIN, RecipeOrigin::Builtin).unwrap();
    let frame = standalone_frame(MapKind::Terrain, &loaded.recipe).unwrap();
    assert_eq!((frame.rows, frame.cols, frame.cell_m), (4, 8, 250.0));
}

#[test]
fn a_standalone_settlement_frame_comes_from_the_class_of_site_terrain() {
    let loaded = recipe("[[stage]]\nid = \"site_terrain\"\nclass = \"hamlet\"");
    let frame = standalone_frame(MapKind::Settlement, &loaded.recipe).unwrap();
    assert_eq!((frame.rows, frame.cols, frame.cell_m), (300, 300, 2.0));
    let without = recipe("");
    assert!(standalone_frame(MapKind::Settlement, &without.recipe).is_err());
}

#[test]
fn a_window_with_too_few_cells_is_refused() {
    let err = frame_of(
        "town",
        [2000.0, 1500.0],
        "[child.class.town]\nwindow_m = 4.0",
    )
    .unwrap_err();
    assert!(err.to_string().contains("window_m"), "{err}");
}
