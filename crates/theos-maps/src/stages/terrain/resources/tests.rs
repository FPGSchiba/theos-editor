//! Tests of the `resources` stage.

use super::{ResourceFields, Resources, ResourcesParams};
use crate::features::{Feature, FeatureSet, Geometry, Prop};
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stage::Stage;
use crate::stages::terrain::climate::{self, Climate, ClimateFields, ClimateParams};
use crate::stages::terrain::hydrology::{LakeShape, WaterFeatures};
use crate::stages::terrain::support::{run_cancelled, run_stage, table, world};
use crate::world::{Layer, World};

const N: usize = 64;
const ELEMENTS: [&str; 5] = [
    "element_water",
    "element_wood",
    "element_earth",
    "element_metal",
    "element_fire",
];

fn field(f: impl Fn(usize, usize) -> f64) -> Grid<f64> {
    Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap()
}

fn mask(f: impl Fn(usize, usize) -> bool) -> Layer {
    Layer::Mask(Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap())
}

/// The rock of column `c`: basalt, quartzite, granite, limestone, shale.
fn rock_at(c: usize) -> u8 {
    match c {
        0..8 => 2,
        8..24 => 1,
        24..28 => 0,
        28..52 => 3,
        _ => 5,
    }
}

fn lake(id: u32, terminal: bool) -> LakeShape {
    LakeShape {
        id,
        rings: Vec::new(),
        surface_m: 0.0,
        terminal,
        outlet: None,
        depth_max_m: 0.0,
    }
}

/// A world of rock bands, a quartzite hill at (20, 14), a river along column 60, a terminal lake
/// at rows 40..44, columns 40..44 and a lake that drains at rows 50..53, columns 10..13.
///
/// Slope 0 above row 8 and 0.2 below; precipitation 300 in the first four columns and 1000
/// elsewhere; vegetation 0.5 left of column 32 and 0.8 right of it; 3 degrees, 8 from row 56.
fn setup(elements: [f64; 5]) -> World {
    let mut w = world(N, N);
    let hill = |r: usize, c: usize| {
        let (dr, dc) = (to_f64(r) - 20.0, to_f64(c) - 14.0);
        400.0 * libm::exp(-(dr * dr + dc * dc) / 18.0)
    };
    w.insert("height", Layer::F64(field(|r, c| 100.0 + hill(r, c))))
        .unwrap();
    w.insert(
        "slope",
        Layer::F64(field(|r, _| if r < 8 { 0.0 } else { 0.2 })),
    )
    .unwrap();
    let rock = (0..N * N).map(|i| rock_at(i % N)).collect();
    w.insert("rock", Layer::U8(Grid::from_vec(N, N, rock).unwrap()))
        .unwrap();
    w.insert("river", mask(|_, c| c == 60)).unwrap();
    let terminal = |r: usize, c: usize| (40..44).contains(&r) && (40..44).contains(&c);
    let draining = |r: usize, c: usize| (50..53).contains(&r) && (10..13).contains(&c);
    w.insert("lake", mask(|r, c| terminal(r, c) || draining(r, c)))
        .unwrap();
    let ids = (0..N * N)
        .map(|i| {
            let (r, c) = (i / N, i % N);
            if terminal(r, c) {
                1
            } else if draining(r, c) {
                2
            } else {
                0
            }
        })
        .collect();
    w.insert("lake_id", Layer::U32(Grid::from_vec(N, N, ids).unwrap()))
        .unwrap();
    let features = WaterFeatures {
        rivers: Vec::new(),
        lakes: vec![lake(1, true), lake(2, false)],
    };
    w.insert("water_features", Layer::Value(Box::new(features)))
        .unwrap();
    w.insert(
        "vegetation",
        Layer::F64(field(|_, c| if c < 32 { 0.5 } else { 0.8 })),
    )
    .unwrap();
    w.insert(
        "temperature",
        Layer::F64(field(|r, _| if r >= 56 { 8.0 } else { 3.0 })),
    )
    .unwrap();
    let climate = Climate {
        params: ClimateParams::default(),
        fields: Some(ClimateFields {
            precipitation: field(|_, c| if c < 4 { 300.0 } else { 1000.0 }),
            pet: field(|_, _| 0.0),
            aet: field(|_, _| 0.0),
            runoff: field(|_, _| 0.0),
        }),
    };
    w.insert("climate", Layer::Value(Box::new(climate)))
        .unwrap();
    for (name, v) in ELEMENTS.into_iter().zip(elements) {
        w.insert(name, Layer::F64(field(|_, _| v))).unwrap();
    }
    w
}

fn plain() -> World {
    setup([0.1, 0.2, 0.3, 0.4, 0.5])
}

fn fields_of(w: &World) -> &std::collections::BTreeMap<String, Grid<f64>> {
    &w.value::<ResourceFields>("resource_fields").unwrap().fields
}

fn deposits(w: &World) -> Vec<Feature> {
    w.value::<FeatureSet>("features")
        .unwrap()
        .of_kind("resource")
        .map(|(_, f)| f.clone())
        .collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn cell_of(w: &World, f: &Feature) -> (usize, usize) {
    let Geometry::Point([x, y]) = f.geometry else {
        panic!("a deposit is a point")
    };
    w.frame().cell_at_m(x, y)
}

#[test]
fn deposits_carry_the_format_properties() {
    let mut w = plain();
    run_stage(
        &mut w,
        &Resources,
        "counts = { timber = 3, stone = 2, iron-ore = 1 }\nspacing_km = 0.5",
    )
    .unwrap();
    let found = deposits(&w);
    assert_eq!(found.len(), 6);
    let harvest = [
        ("timber", "fell"),
        ("stone", "quarry"),
        ("iron-ore", "mine"),
    ];
    for f in &found {
        assert_eq!(f.kind, "resource");
        let keys: Vec<&str> = f.props.keys().map(String::as_str).collect();
        assert_eq!(keys, ["element", "harvest", "radius_m", "richness", "tier"]);
        let Some(Prop::Num(richness)) = f.props.get("richness") else {
            panic!("richness")
        };
        assert!((0.0..=1.0).contains(richness), "{richness}");
        assert!(
            near((richness * 100.0).round(), richness * 100.0),
            "{richness}"
        );
        let Some(Prop::Text(tier)) = f.props.get("tier") else {
            panic!("tier")
        };
        let want = if *richness > 0.81 {
            Some("exceptional")
        } else if *richness < 0.49 {
            Some("common")
        } else if (0.51..=0.79).contains(richness) {
            Some("rich")
        } else {
            None
        };
        assert!(want.is_none_or(|t| t == tier), "{richness} {tier}");
        let Some(Prop::Int(radius)) = f.props.get("radius_m") else {
            panic!("radius_m")
        };
        let rich_radius = 100.0 + 300.0 * richness;
        assert!((to_f64(usize::try_from(*radius).unwrap()) - rich_radius).abs() <= 2.5);
        assert_eq!(f.props.get("element"), Some(&Prop::Text("fire".into())));
        let want_harvest = harvest.iter().find(|(t, _)| *t == f.ty).unwrap().1;
        assert_eq!(
            f.props.get("harvest"),
            Some(&Prop::Text(want_harvest.into()))
        );
    }
    let count = |ty: &str| found.iter().filter(|f| f.ty == ty).count();
    assert_eq!(
        (count("timber"), count("stone"), count("iron-ore")),
        (3, 2, 1)
    );
}

#[test]
fn the_element_is_the_strongest_and_ties_go_to_the_first() {
    let element = |mix: [f64; 5]| {
        let mut w = setup(mix);
        run_stage(&mut w, &Resources, "counts = { timber = 1 }").unwrap();
        deposits(&w)[0].props.get("element").cloned()
    };
    let text = |s: &str| Some(Prop::Text(s.into()));
    assert_eq!(element([0.1, 0.2, 0.3, 0.9, 0.5]), text("metal"));
    assert_eq!(element([0.5, 0.5, 0.5, 0.5, 0.5]), text("water"));
    assert_eq!(element([0.1, 0.5, 0.5, 0.4, 0.2]), text("wood"));
    assert_eq!(element([0.0, 0.0, 0.0, 0.0, 0.0]), text("water"));
}

#[test]
fn places_stay_off_the_margin() {
    let mut w = plain();
    run_stage(
        &mut w,
        &Resources,
        "counts = { timber = 6 }\nspacing_km = 0.3\nedge_margin = 0.1",
    )
    .unwrap();
    let found = deposits(&w);
    assert_eq!(found.len(), 6);
    for f in &found {
        let (r, c) = cell_of(&w, f);
        // 0.1 x 64 = 6.4, rounded to 6 cells.
        assert!(
            (6..N - 6).contains(&r) && (6..N - 6).contains(&c),
            "{r},{c}"
        );
    }
}

#[test]
fn deposits_keep_the_spacing() {
    let mut w = plain();
    run_stage(
        &mut w,
        &Resources,
        "counts = { timber = 5 }\nspacing_km = 1.0",
    )
    .unwrap();
    let found = deposits(&w);
    assert_eq!(found.len(), 5);
    let at = |f: &Feature| match f.geometry {
        Geometry::Point(p) => p,
        _ => panic!("a point"),
    };
    for (i, a) in found.iter().enumerate() {
        for b in &found[i + 1..] {
            let (p, q) = (at(a), at(b));
            let d = libm::sqrt((p[0] - q[0]) * (p[0] - q[0]) + (p[1] - q[1]) * (p[1] - q[1]));
            assert!(d >= 1000.0, "{d}");
        }
    }
}

#[test]
fn the_fields_follow_the_formulas() {
    let mut w = plain();
    run_stage(&mut w, &Resources, "").unwrap();
    let f = fields_of(&w);
    let keys: Vec<&str> = f.keys().map(String::as_str).collect();
    let mut want = [
        "iron-ore",
        "copper-ore",
        "silver-ore",
        "stone",
        "clay",
        "timber",
        "peat-reeds",
        "salt",
        "resonance-crystal",
        "herbs",
    ];
    want.sort_unstable();
    assert_eq!(keys, want);
    let at = |name: &str, r: usize, c: usize| f[name].get(r, c);
    let flat = libm::exp(-0.2 / 0.04);
    // Ores: quartzite, basalt on a slope above 0.1, and granite contact in limestone; the contact
    // reaches 4 cells (350 m on cells of 100 m is 3.5, rounded to even).
    assert!(near(at("iron-ore", 40, 12), 0.8) && near(at("iron-ore", 40, 3), 0.35));
    assert!(near(at("copper-ore", 40, 3), 0.3) && near(at("copper-ore", 40, 12), 0.0));
    for c in [28, 29, 31] {
        assert!(near(at("iron-ore", 30, c), 0.7) && near(at("copper-ore", 30, c), 1.0));
    }
    assert!(near(at("iron-ore", 30, 32), 0.0) && near(at("copper-ore", 30, 32), 0.0));
    // Stone: limestone or granite on a slope between 0.08 and 0.5.
    assert!(near(at("stone", 30, 40), 1.0) && near(at("stone", 30, 25), 1.0));
    assert!(near(at("stone", 4, 40), 0.0) && near(at("stone", 30, 12), 0.0));
    // Clay and peat: wet flats within 0.6 km of the river (600 m is not within).
    assert!(near(at("clay", 30, 60), flat) && near(at("clay", 30, 56), flat));
    assert!(near(at("clay", 30, 54), 0.0));
    assert!(near(at("peat-reeds", 30, 60), flat) && near(at("peat-reeds", 30, 54), 0.0));
    // Timber: dense vegetation; herbs: 0.25 to 0.75 and between -1 and 7 degrees.
    assert!(near(at("timber", 30, 50), 1.0) && near(at("timber", 30, 5), 0.0));
    assert!(near(at("herbs", 30, 5), 1.0) && near(at("herbs", 30, 50), 0.0));
    assert!(near(at("herbs", 60, 5), 0.0));
    // Salt: dry flats, and the shore of terminal lakes only.
    assert!(near(at("salt", 2, 2), 1.0) && near(at("salt", 20, 2), 0.0));
    assert!(near(at("salt", 39, 41), 1.0) && near(at("salt", 44, 41), 1.0));
    assert!(near(at("salt", 41, 41), 0.0) && near(at("salt", 49, 11), 0.0));
    // The shore is one cell wide (125 m on cells of 100 m).
    assert!(near(at("salt", 38, 41), 0.0) && near(at("salt", 45, 41), 0.0));
    // Silver: hard rock on a ridge above the 70th height percentile.
    assert!(at("silver-ore", 20, 14) > 0.99);
    assert!(near(at("silver-ore", 30, 40), 0.0) && near(at("silver-ore", 40, 12), 0.0));
}

#[test]
fn resonance_crystal_follows_ruins_and_omega() {
    let summit = [1450.0, 2050.0];
    let run = |ruin: bool, omega: Option<f64>| {
        let mut w = plain();
        if ruin {
            let mut set = FeatureSet::default();
            set.add(Feature {
                kind: "poi",
                ty: "ruin".into(),
                geometry: Geometry::Point(summit),
                props: std::collections::BTreeMap::new(),
            });
            w.insert("features", Layer::Value(Box::new(set))).unwrap();
        }
        if let Some(v) = omega {
            w.insert("omega", Layer::F64(field(|_, _| v))).unwrap();
        }
        run_stage(&mut w, &Resources, "").unwrap();
        fields_of(&w)["resonance-crystal"].clone()
    };
    let bare = run(false, None);
    assert!(near(bare.get(20, 14), 0.3), "{}", bare.get(20, 14));
    // A zero omega is no omega.
    assert_eq!(run(false, Some(0.0)), bare);
    let with_omega = run(false, Some(0.25));
    assert!(near(with_omega.get(20, 14), 0.55));
    // The ruin adds exp(-d^2 / (2 * 2500^2)); three cells from the summit is 300 m.
    let ruined = run(true, None);
    assert!(near(ruined.get(20, 14), 1.3));
    let gauss = libm::exp(-300.0 * 300.0 / (2.0 * 2500.0 * 2500.0));
    let ratio = ruined.get(20, 17) / bare.get(20, 17);
    assert!(bare.get(20, 17) > 0.0);
    assert!(near(ratio, (0.3 + gauss) / 0.3), "{ratio}");
    // Soft rock holds none.
    assert!(near(ruined.get(30, 40), 0.0));
}

#[test]
fn without_water_the_land_is_dry() {
    let mut w = plain();
    w.insert("river", mask(|_, _| false)).unwrap();
    w.insert("lake", mask(|_, _| false)).unwrap();
    w.insert("lake_id", Layer::U32(Grid::filled(N, N, 0)))
        .unwrap();
    run_stage(&mut w, &Resources, "counts = { clay = 2, salt = 1 }").unwrap();
    let f = fields_of(&w);
    assert!(f["clay"].as_slice().iter().all(|v| *v == 0.0));
    assert!(f["peat-reeds"].as_slice().iter().all(|v| *v == 0.0));
    // Dry flats still hold salt, the shore of a terminal lake is gone.
    assert!(near(f["salt"].get(2, 2), 1.0) && near(f["salt"].get(39, 41), 0.0));
    let found = deposits(&w);
    assert!(
        found.iter().all(|d| d.ty == "salt"),
        "no clay without water"
    );
}

/// [`plain`] with the climate's `fields` replaced, after `resources` ran on it.
fn run_with_fields(fields: Option<ClimateFields>) -> World {
    let mut w = plain();
    let mut climate = w.value::<Climate>("climate").unwrap().clone();
    climate.fields = fields;
    w.remove("climate");
    w.insert("climate", Layer::Value(Box::new(climate)))
        .unwrap();
    run_stage(&mut w, &Resources, "counts = { timber = 1 }").unwrap();
    w
}

// Ruling P2: without climate fields the stage computes the precipitation from the height with
// the climate model. The fields that depend on it (peat-reeds needs > 900 mm/yr, salt < 400)
// equal those of a run given exactly the model's fields, and differ from those of the setup's
// own precipitation, so another fallback would not pass.
#[test]
fn the_precipitation_falls_back_to_the_climate_model() {
    let base = plain();
    let model = climate::fields(
        base.f64("height").unwrap(),
        base.frame(),
        &ClimateParams::default(),
    );
    let fallback = run_with_fields(None);
    let given = run_with_fields(Some(model));
    let own = run_with_fields(base.value::<Climate>("climate").unwrap().fields.clone());
    let wet = ["peat-reeds", "salt"];
    for name in wet {
        assert_eq!(
            fields_of(&fallback)[name],
            fields_of(&given)[name],
            "{name}"
        );
    }
    assert!(
        wet.iter()
            .any(|name| fields_of(&own)[*name] != fields_of(&given)[*name]),
        "the precipitation changes no field of this world"
    );
    assert_eq!(deposits(&fallback).len(), 1);
}

#[test]
fn existing_features_stay_and_deposits_follow() {
    let mut w = plain();
    let mut set = FeatureSet::default();
    set.add(Feature {
        kind: "poi",
        ty: "ruin".into(),
        geometry: Geometry::Point([100.0, 100.0]),
        props: std::collections::BTreeMap::new(),
    });
    w.insert("features", Layer::Value(Box::new(set))).unwrap();
    run_stage(&mut w, &Resources, "counts = { timber = 2 }").unwrap();
    let all = w.value::<FeatureSet>("features").unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all.iter().next().map(|(_, f)| f.ty.as_str()), Some("ruin"));
}

#[test]
fn the_same_inputs_give_the_same_deposits() {
    let (mut a, mut b) = (plain(), plain());
    let params = "counts = { timber = 3, iron-ore = 2, herbs = 2 }";
    run_stage(&mut a, &Resources, params).unwrap();
    run_stage(&mut b, &Resources, params).unwrap();
    assert_eq!(
        a.value::<FeatureSet>("features"),
        b.value::<FeatureSet>("features")
    );
    assert_eq!(fields_of(&a), fields_of(&b));
}

#[test]
fn parameters_have_defaults_and_are_checked() {
    let d = ResourcesParams::default();
    assert!(d.counts.is_empty());
    assert!((d.spacing_km - 2.0).abs() < 1e-12);
    assert!((d.edge_margin - 0.0125).abs() < 1e-12);
    assert!(Resources.params(&table("")).is_ok());
    assert!(
        Resources
            .params(&table("counts = { peat-reeds = 1, resonance-crystal = 2 }"))
            .is_ok()
    );
    for bad in [
        "spacing_km = 0.0",
        "spacing_km = nan",
        "edge_margin = 0.5",
        "edge_margin = -0.1",
        "counts = { iron_ore = 1 }",
        "counts = { gold = 1 }",
    ] {
        assert!(Resources.params(&table(bad)).is_err(), "{bad}");
    }
    let err = Resources
        .params(&table("counts = { gold = 1 }"))
        .unwrap_err();
    assert_eq!(err.parameter.as_deref(), Some("counts.gold"));
    // Final review round 5, finding 1: an unknown type reads as an unknown key.
    assert_eq!(err.message, "unknown parameter");
    let err = Resources.params(&table("spacing = 1.0")).unwrap_err();
    assert!(err.to_string().contains("spacing"), "{err}");
}

#[test]
fn the_stage_reports_its_counts_under_declared_names() {
    let mut w = plain();
    let report = run_stage(&mut w, &Resources, "counts = { timber = 2, herbs = 1 }").unwrap();
    let defaults = Resources.params(&table("")).unwrap();
    let metrics = Stage::metrics(&Resources, &w, defaults.as_ref(), &report);
    let mut names = Stage::metric_names(&Resources, defaults.as_ref());
    names.sort_unstable();
    let keys: Vec<&str> = metrics.0.keys().map(String::as_str).collect();
    assert_eq!(names, keys);
    assert_eq!(metrics.0.get("deposits"), Some(&3.0));
}

// Final review round 4, class I: work a recipe count sizes reaches a cancel point (CE-28.4).
#[test]
fn a_cancel_stops_the_picking() {
    let mut w = setup([0.2; 5]);
    let result = run_cancelled(&mut w, &Resources, "");
    assert!(
        matches!(result, Err(crate::stage::StageError::Cancelled)),
        "{result:?}"
    );
}

// Final review round 4, class K: the types a stage spells are the table's types of its kind,
// in both directions.
#[test]
fn the_resource_types_are_the_tables() {
    let table: Vec<&str> = crate::features::types_of("resource").collect();
    assert_eq!(super::fields::NAMES.to_vec(), table);
}
