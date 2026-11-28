use super::*;
use crate::package::TERRAIN_LAYERS;
use crate::render::Role;

fn names(kind: MapKind, role: Role) -> Vec<&'static str> {
    table(kind)
        .renders
        .iter()
        .filter(|r| r.role == role)
        .map(|r| r.name)
        .collect()
}

#[test]
fn every_kind_has_exactly_one_table() {
    assert_eq!(KINDS.len(), 3);
    for kind in [MapKind::Terrain, MapKind::Settlement, MapKind::Interior] {
        assert_eq!(table(kind).kind, kind);
        assert_eq!(KINDS.iter().filter(|t| t.kind == kind).count(), 1);
    }
}

#[test]
fn the_terrain_table_is_p08s() {
    let t = table(MapKind::Terrain);
    let layers: Vec<&str> = t.layers.iter().map(|l| l.name).collect();
    let today: Vec<&str> = TERRAIN_LAYERS.iter().map(|l| l.name).collect();
    assert_eq!(layers, today);
    let optional: Vec<&str> = t.optional_layers.iter().map(|l| l.name).collect();
    assert_eq!(optional, ["omega"]);
    assert_eq!(
        names(MapKind::Terrain, Role::Standard),
        ["base", "truth", "geology", "resources", "inworld"]
    );
    assert_eq!(names(MapKind::Terrain, Role::Extra), ["omega"]);
    assert_eq!(t.ids, IdScheme::ByType);
    assert_eq!(t.summary_paths, crate::metrics::PATHS);
    assert_eq!(t.types, crate::features::TYPES);
}

#[test]
fn the_other_tables_are_empty_for_now() {
    for kind in [MapKind::Settlement, MapKind::Interior] {
        let t = table(kind);
        assert!(t.layers.is_empty() && t.optional_layers.is_empty());
        assert!(t.renders.is_empty() && t.types.is_empty() && t.summary_paths.is_empty());
        assert_eq!(
            (t.metrics)(
                &crate::world::World::new(crate::world::MapFrame {
                    rows: 1,
                    cols: 1,
                    cell_m: 1.0
                }),
                &[]
            )
            .unwrap(),
            serde_json::json!({})
        );
        assert_eq!((t.title)("r", 3, &serde_json::json!({})), "r \u{b7} seed 3");
    }
    assert_eq!(table(MapKind::Settlement).ids, IdScheme::ByType);
    assert_eq!(table(MapKind::Interior).ids, IdScheme::ByKind);
}
