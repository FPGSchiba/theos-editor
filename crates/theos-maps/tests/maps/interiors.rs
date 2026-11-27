//! Interior maps through the public API (P34): the fixture runs through every interior stage,
//! and the recipe loader refuses what the stages cannot build before any stage runs.

use std::path::PathBuf;

use theos_maps::recipe::RecipeError;
use theos_maps::{CancelToken, Progress, RunError, run};

use crate::support::{TestResult, recipes, request, run_package};

/// The constraint result `key` of a run.
fn result(manifest: &theos_maps::RunManifest, key: &str) -> Option<f64> {
    manifest
        .results
        .get(key)
        .and_then(serde_json::Value::as_f64)
}

// AC: CE-80.2
#[test]
fn fixture_interior_runs_and_passes() -> TestResult {
    for seed in 1..=3 {
        let (package, manifest) =
            run_package("fixture-interior", seed, &recipes()).map_err(|e| e.to_string())?;
        assert_eq!(
            manifest.results.get("pass"),
            Some(&serde_json::Value::Bool(true)),
            "seed {seed}: {:?}",
            manifest.results
        );
        let rooms =
            result(&manifest, "stages_playability_rooms").ok_or("the room count is a result")?;
        assert!((8.0..=13.0).contains(&rooms), "seed {seed}: {rooms} rooms");
        // The exit distances exist only when the graph has an exit room the party reaches
        // (ruling R29: `leaf` with `role = "exit"` places the program's exit).
        let playability = &package.manifest.summary["stages"]["playability"];
        for metric in ["start_to_exit_min_m", "goal_to_exit_min_m"] {
            let m = playability[metric]
                .as_f64()
                .ok_or(format!("seed {seed}: no {metric}"))?;
            assert!(m > 0.0, "seed {seed}: {metric} = {m}");
        }
        assert_eq!(manifest.frame.size_m, [40.0, 40.0]);
    }
    Ok(())
}

/// The fixture with `edit` applied to its text, as a recipe named `broken` in a new folder.
fn broken(edit: impl Fn(String) -> String) -> Result<tempfile::TempDir, std::io::Error> {
    let text = std::fs::read_to_string(recipes()[0].join("fixture-interior.toml"))?;
    let text = edit(text).replace("id      = \"fixture-interior\"", "id      = \"broken\"");
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("broken.toml"), text)?;
    Ok(dir)
}

/// A broken recipe: what is wrong, the edit that breaks the fixture, and the stage and the
/// parameter the refusal names.
type Refusal = (
    &'static str,
    fn(String) -> String,
    Option<&'static str>,
    &'static str,
);

// Spec §5.16: the recipe loader refuses what the stages cannot build.
#[test]
fn loader_refusals() -> TestResult {
    let cases: [Refusal; 6] = [
        (
            "an underscore in a room type",
            |t| t.replace("type = \"store\"", "type = \"store_room\""),
            Some("mission"),
            "room[3].type",
        ),
        (
            "17 keys",
            |t| {
                t.replacen(
                    "  [[stage.rule]]\n  rule = \"leaf\"",
                    "  [[stage.rule]]\n  rule = \"locked_leaf\"\n  count = 17",
                    1,
                )
            },
            Some("mission"),
            "rule",
        ),
        (
            "a pitch of 15.5 m at 1 m",
            |t| t.replace("pitch_m  = 8.0", "pitch_m  = 15.5"),
            Some("envelope"),
            "pitch_m",
        ),
        (
            "a cell size of 0.1 m",
            |t| t.replace("grid    = [40, 40]", "grid    = [400, 400]"),
            None,
            "size_km",
        ),
        (
            "an unknown rule",
            |t| t.replacen("rule = \"leaf\"", "rule = \"attic\"", 1),
            Some("mission"),
            "rule[1].rule",
        ),
        (
            "walls of 3 m in a block of 8 m with a jitter of 2 m",
            |t| {
                t.replace("pitch_m  = 8.0", "pitch_m  = 8.0\njitter_m = 2.0")
                    .replace("id = \"space\"", "id = \"space\"\nwall_m = 3.0")
            },
            Some("space"),
            "wall_m",
        ),
    ];
    for (what, edit, stage, parameter) in cases {
        let dir = broken(edit)?;
        let out = tempfile::tempdir()?;
        let mut stages_run = 0_u32;
        let result = run(
            &request("broken", 1),
            &[PathBuf::from(dir.path())],
            out.path(),
            &mut |p: &Progress| {
                if p.done == 0 && p.stage != "constraints" {
                    stages_run += 1;
                }
            },
            &CancelToken::new(),
        );
        match result {
            Err(RunError::Recipe(RecipeError::Invalid {
                stage: s,
                parameter: p,
                message,
                ..
            })) => {
                assert_eq!(s.as_deref(), stage, "{what}: {message}");
                assert_eq!(p.as_deref(), Some(parameter), "{what}: {message}");
            }
            other => {
                return Err(
                    format!("{what}: expected a refusal, got {:?}", other.map(|_| ())).into(),
                );
            }
        }
        assert_eq!(stages_run, 0, "{what}: no stage ran");
    }
    Ok(())
}

/// The package and manifest of `fixture-interior` with seed 1.
fn fixture_run()
-> Result<(theos_map_format::Package, theos_maps::RunManifest), Box<dyn std::error::Error>> {
    Ok(run_package("fixture-interior", 1, &recipes()).map_err(|e| e.to_string())?)
}

/// The id of a feature.
fn id_of(f: &geojson::Feature) -> String {
    match &f.id {
        Some(geojson::feature::Id::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// A property of a feature.
fn prop<'a>(f: &'a geojson::Feature, key: &str) -> Option<&'a serde_json::Value> {
    f.properties.as_ref()?.get(key)
}

/// The features of `kind`.
fn of_kind<'a>(package: &'a theos_map_format::Package, kind: &str) -> Vec<&'a geojson::Feature> {
    package
        .features
        .features
        .iter()
        .filter(|f| prop(f, "kind").and_then(serde_json::Value::as_str) == Some(kind))
        .collect()
}

/// Half the shoelace sum of a closed ring.
fn shoelace(ring: &[geojson::Position]) -> f64 {
    ring.windows(2)
        .map(|w| {
            let (a, b) = (w[0].as_slice(), w[1].as_slice());
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

// AC: CE-80.1
#[test]
fn room_polygons_enclose_their_cells() -> TestResult {
    use geojson::GeometryValue;
    use theos_map_format::{LayerData, LayerKey};
    let (package, _) = fixture_run()?;
    let cell_m = package.manifest.frame.cell_m;
    let key = LayerKey {
        level: None,
        name: "rooms".to_owned(),
    };
    let Some(LayerData::U16(numbers)) = package.layers.get(&key) else {
        return Err("the package has no u16 `rooms` layer".into());
    };
    let rooms = of_kind(&package, "room");
    assert!(rooms.len() >= 8, "{} rooms", rooms.len());
    for (k, room) in rooms.iter().enumerate() {
        let id = id_of(room);
        assert_eq!(id, format!("room-{}", k + 1));
        let number = prop(room, "number")
            .and_then(serde_json::Value::as_u64)
            .ok_or(format!("{id}: no number"))?;
        assert_eq!(number, u64::try_from(k)?, "{id}");
        let cells = numbers
            .iter()
            .filter(|&&v| u64::from(v) == number + 1)
            .count();
        assert!(cells > 0, "{id} has no cells");
        let Some(GeometryValue::Polygon { coordinates }) = room.geometry.as_ref().map(|g| &g.value)
        else {
            return Err(format!("{id} is not a Polygon").into());
        };
        let (outline, holes) = coordinates.split_first().ok_or("no outline")?;
        let area = shoelace(outline).abs() - holes.iter().map(|h| shoelace(h).abs()).sum::<f64>();
        let expected = f64::from(u32::try_from(cells)?) * cell_m * cell_m;
        assert!(
            (area - expected).abs() < 1e-9,
            "{id}: {area} m2, {cells} cells"
        );
        let area_m2 = prop(room, "area_m2")
            .and_then(serde_json::Value::as_f64)
            .ok_or(format!("{id}: no area_m2"))?;
        assert!((area_m2 - expected).abs() < 1e-9, "{id}: area_m2 {area_m2}");
        for key in ["role", "shape", "keys", "locked"] {
            assert!(prop(room, key).is_some(), "{id}: no {key}");
        }
    }
    Ok(())
}

// AC: CE-80.1
#[test]
fn doors_and_connections_follow_the_format() -> TestResult {
    let (package, _) = fixture_run()?;
    let room_ids: Vec<String> = of_kind(&package, "room").into_iter().map(id_of).collect();
    let doors = of_kind(&package, "door");
    let door_ids: Vec<String> = doors.iter().map(|f| id_of(f)).collect();
    assert_ne!(doors.len(), 0, "no doors");
    for door in &doors {
        let id = id_of(door);
        let rooms = prop(door, "rooms")
            .and_then(serde_json::Value::as_array)
            .ok_or(format!("{id}: no rooms"))?;
        assert_eq!(rooms.len(), 2, "{id}");
        for room in rooms {
            let room = room.as_str().ok_or("a room id is text")?;
            assert!(room_ids.iter().any(|r| r == room), "{id}: {room}");
        }
        assert!(
            matches!(
                door.geometry.as_ref().map(|g| &g.value),
                Some(geojson::GeometryValue::Point { .. })
            ),
            "{id} is not a Point"
        );
    }
    let bytes = theos_map_format::features_bytes(&package.features)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)?;
    let connections: Vec<&serde_json::Value> = json["features"]
        .as_array()
        .ok_or("no features")?
        .iter()
        .filter(|f| f["properties"]["kind"] == "connection")
        .collect();
    assert_eq!(connections.len(), doors.len(), "one door per connection");
    for (i, c) in connections.iter().enumerate() {
        let id = &c["id"];
        assert_eq!(c.get("geometry"), Some(&serde_json::Value::Null), "{id}");
        for key in ["from", "to"] {
            let room = c["properties"][key]
                .as_str()
                .ok_or(format!("{id}: no {key}"))?;
            assert!(room_ids.iter().any(|r| r == room), "{id}: {key} {room}");
        }
        assert_eq!(c["properties"]["door"], door_ids[i].as_str(), "{id}");
    }
    Ok(())
}

// AC: CE-80.1
#[test]
fn one_critical_route_from_start_to_goal() -> TestResult {
    let (package, _) = fixture_run()?;
    let routes = of_kind(&package, "route");
    assert_eq!(routes.len(), 1);
    let route = routes[0];
    assert_eq!(id_of(route), "route-1");
    assert_eq!(
        prop(route, "type"),
        Some(&serde_json::Value::from("critical"))
    );
    let role_of = |id: &str| {
        of_kind(&package, "room")
            .into_iter()
            .find(|r| id_of(r) == id)
            .and_then(|r| prop(r, "role").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
    };
    let rooms = prop(route, "rooms")
        .and_then(serde_json::Value::as_array)
        .ok_or("no rooms")?;
    let first = rooms
        .first()
        .and_then(serde_json::Value::as_str)
        .ok_or("empty")?;
    let last = rooms
        .last()
        .and_then(serde_json::Value::as_str)
        .ok_or("empty")?;
    assert_eq!(role_of(first).as_deref(), Some("start"));
    assert_eq!(role_of(last).as_deref(), Some("goal"));
    let Some(geojson::GeometryValue::LineString { coordinates }) =
        route.geometry.as_ref().map(|g| &g.value)
    else {
        return Err("the route is not a LineString".into());
    };
    assert!(coordinates.len() >= 2);
    assert!(
        coordinates
            .iter()
            .all(|p| p.as_slice().len() == 3 && p.as_slice()[2] == 0.0)
    );
    // Ruling R26: `length_m` is the analysis's critical path, not the line's own length.
    let length = prop(route, "length_m").and_then(serde_json::Value::as_f64);
    let critical = package.manifest.summary["stages"]["playability"]["critical_path_m"].as_f64();
    assert!(
        length.is_some() && length == critical,
        "{length:?} vs {critical:?}"
    );
    Ok(())
}

// AC: CE-80.1
#[test]
fn ids_are_unique_per_kind() -> TestResult {
    let (package, _) = fixture_run()?;
    let mut seen = std::collections::BTreeSet::new();
    let mut next: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for f in &package.features.features {
        let id = id_of(f);
        assert!(seen.insert(id.clone()), "{id} twice");
        let kind = prop(f, "kind")
            .and_then(serde_json::Value::as_str)
            .ok_or(format!("{id}: no kind"))?;
        let n = next.entry(kind.to_owned()).or_insert(0);
        *n += 1;
        assert_eq!(id, format!("{kind}-{n}"));
    }
    for kind in ["room", "door", "connection", "route"] {
        assert!(next.contains_key(kind), "no {kind}");
    }
    Ok(())
}

// AC: CE-80.1
#[test]
fn package_layers_and_dtypes() -> TestResult {
    use theos_map_format::DType;
    let (package, _) = fixture_run()?;
    let layers: Vec<(&str, DType)> = package
        .manifest
        .layers
        .iter()
        .map(|l| (l.name.as_str(), l.dtype))
        .collect();
    assert_eq!(
        layers,
        [
            ("walkable", DType::U8),
            ("rooms", DType::U16),
            ("corridors", DType::U8),
            ("cover", DType::U8),
            ("walls", DType::U8),
        ]
    );
    Ok(())
}

/// The fixture with one more constraint line.
fn constrained(text: &str, line: &str) -> String {
    text.replace("[constraints]\n", &format!("[constraints]\n{line}\n"))
}

// AC: CE-80.1
#[test]
fn room_type_constraint_counts() -> TestResult {
    let dir = broken(|t| constrained(&t, "\"features.entrance\" = [1, 1]"))?;
    let out = tempfile::tempdir()?;
    let manifest = run(
        &request("broken", 1),
        &[PathBuf::from(dir.path())],
        out.path(),
        &mut |_: &Progress| {},
        &CancelToken::new(),
    )?;
    assert_eq!(result(&manifest, "features_entrance"), Some(1.0));
    assert_eq!(
        manifest.results.get("pass"),
        Some(&serde_json::Value::Bool(true)),
        "{:?}",
        manifest.results
    );
    let dir = broken(|t| constrained(&t, "\"features.no-such\" = [0, 0]"))?;
    let out = tempfile::tempdir()?;
    let mut stages_run = 0_u32;
    let refused = run(
        &request("broken", 1),
        &[PathBuf::from(dir.path())],
        out.path(),
        &mut |p: &Progress| {
            if p.done == 0 && p.stage != "constraints" {
                stages_run += 1;
            }
        },
        &CancelToken::new(),
    );
    assert!(
        matches!(refused, Err(RunError::Recipe(RecipeError::Invalid { .. }))),
        "{:?}",
        refused.map(|_| ())
    );
    assert_eq!(stages_run, 0);
    Ok(())
}

// AC: CE-80.1
#[test]
fn the_summary_holds_the_mission_entries() -> TestResult {
    let (package, _) = fixture_run()?;
    let entries = package.manifest.summary["mission"]["entries"]
        .as_array()
        .ok_or("no summary.mission.entries")?;
    let rules: Vec<&str> = entries.iter().filter_map(|e| e["rule"].as_str()).collect();
    assert_eq!(rules, ["main_cycle", "leaf", "leaf"]);
    for e in entries {
        for key in ["asked", "applied", "undone"] {
            assert!(e[key].is_u64(), "{e}: {key}");
        }
    }
    Ok(())
}

/// The built-in interior recipes that run on their own (the child `ruin-undercroft` needs a
/// parent package; ruling R2 keeps it out of these tests).
const BUILTIN_INTERIORS: [&str; 2] = ["choir-hall-ruin", "works-halls"];

// AC: CE-80.1
#[test]
fn builtins_produce_all_outputs() -> TestResult {
    for recipe in BUILTIN_INTERIORS {
        let (package, manifest) = run_package(recipe, 1, &[]).map_err(|e| e.to_string())?;
        for output in ["base", "blueprint", "graph", "features", "package"] {
            assert!(
                manifest.outputs.contains_key(output),
                "{recipe}: no {output} in {:?}",
                manifest.outputs.keys()
            );
        }
        assert_eq!(package.manifest.kind, "interior", "{recipe}");
    }
    Ok(())
}

/// The text of a feature property.
fn text<'a>(f: &'a geojson::Feature, key: &str) -> Option<&'a str> {
    prop(f, key).and_then(serde_json::Value::as_str)
}

/// The rooms the key fixed point of §5.9 reaches in `package`, read from its features alone:
/// start with no keys at the start room, walk open connections, locked ones whose key is held
/// and one-way ones in their direction, pick up every key of the rooms reached, repeat.
fn fixed_point(package: &theos_map_format::Package) -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    let rooms = of_kind(package, "room");
    let connections = of_kind(package, "connection");
    let mut reached: BTreeSet<String> = rooms
        .iter()
        .filter(|r| text(r, "role") == Some("start"))
        .map(|r| id_of(r))
        .collect();
    let mut keys: BTreeSet<String> = BTreeSet::new();
    loop {
        let before = (reached.len(), keys.len());
        for r in &rooms {
            if reached.contains(&id_of(r)) {
                let held = prop(r, "keys").and_then(serde_json::Value::as_array);
                keys.extend(
                    held.into_iter()
                        .flatten()
                        .filter_map(|k| k.as_str().map(str::to_owned)),
                );
            }
        }
        for c in &connections {
            let (Some(from), Some(to)) = (text(c, "from"), text(c, "to")) else {
                continue;
            };
            let ty = text(c, "type");
            if ty == Some("locked") && !text(c, "key").is_some_and(|k| keys.contains(k)) {
                continue;
            }
            if reached.contains(from) {
                reached.insert(to.to_owned());
            }
            if ty != Some("oneway") && reached.contains(to) {
                reached.insert(from.to_owned());
            }
        }
        if (reached.len(), keys.len()) == before {
            return reached;
        }
    }
}

// AC: CE-80.2
// About 5 s in a release build, so not a `slow_` test (`docs/gate.md`).
#[test]
fn builtins_pass_playability_on_seeds_1_to_8() -> TestResult {
    use theos_map_format::{LayerData, LayerKey};
    for recipe in BUILTIN_INTERIORS {
        for seed in 1..=8 {
            let at = format!("{recipe} seed {seed}");
            // A run whose analysis reports a finding fails with an error (spec §5.9).
            let (package, manifest) =
                run_package(recipe, seed, &[]).map_err(|e| format!("{at}: {e}"))?;
            assert_eq!(
                manifest.results.get("pass"),
                Some(&serde_json::Value::Bool(true)),
                "{at}: {:?}",
                manifest.results
            );
            let key = LayerKey {
                level: None,
                name: "rooms".to_owned(),
            };
            let Some(LayerData::U16(numbers)) = package.layers.get(&key) else {
                return Err(format!("{at}: no u16 rooms layer").into());
            };
            let carved: std::collections::BTreeSet<u16> = numbers.iter().copied().collect();
            let rooms = of_kind(&package, "room");
            let reached = fixed_point(&package);
            for room in &rooms {
                let id = id_of(room);
                let number = prop(room, "number")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or(format!("{at}: {id} has no number"))?;
                assert!(
                    carved.contains(&u16::try_from(number + 1)?),
                    "{at}: {id} is not carved"
                );
                assert!(reached.contains(&id), "{at}: {id} is not reached");
            }
            let goal = rooms
                .iter()
                .find(|r| text(r, "role") == Some("goal"))
                .ok_or(format!("{at}: no goal"))?;
            assert!(
                reached.contains(&id_of(goal)),
                "{at}: the goal is not reached"
            );
        }
    }
    Ok(())
}
