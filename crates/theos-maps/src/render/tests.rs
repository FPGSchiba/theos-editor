//! Tests of the renders for people: `truth`, `geology`, `resources`, `inworld` and `omega`.

use theos_map_format::sha256_hex;

use super::test_support::{TestResult, TestWorld, decode};
use super::truth::{TRUTH_BOTTOM_PX, TRUTH_TOP_PX};
use super::{RENDERS, RenderError, canvas, render, styles};
use crate::features::{FeatureSet, Prop};
use crate::kernels::num::to_f64;
use crate::progress::CancelToken;
use crate::stages::terrain::roads::RoadClass;

const ROWS: usize = 40;
const COLS: usize = 48;
const CELL_M: f64 = 50.0;
const PEOPLE: [&str; 5] = ["truth", "geology", "resources", "inworld", "omega"];

/// A world (synthetic, not canon) with one of everything the renders draw. The settlement names
/// are stand-ins for generated names.
fn world() -> TestWorld {
    world_with_names(true)
}

/// [`world`], with or without the names of its settlements.
fn world_with_names(named: bool) -> TestWorld {
    let mut w = TestWorld::flat(ROWS, COLS, CELL_M);
    w.set("height", |r, c| 500.0 + 8.0 * to_f64(r + 2 * c));
    w.set("vegetation", |r, _| if r > 20 { 0.9 } else { 0.1 });
    w.omega(|_, c| to_f64(c) / 48.0);
    w.lake(30..34, 36..42);
    w.river(&[[0.0, 1800.0], [2400.0, 1800.0]], 30.0);
    w.big_river(36..37, 0..COLS);
    let name = |n: &str| {
        named
            .then(|| ("name", Prop::Text(n.to_owned())))
            .into_iter()
            .collect::<Vec<_>>()
    };
    w.feature("settlement", "village", [600.0, 500.0], &name("Testburg"));
    w.feature("settlement", "hamlet", [1500.0, 700.0], &name("Smallplace"));
    w.feature("poi", "ruin", [1500.0, 1200.0], &[]);
    w.feature("poi", "chime", [900.0, 1400.0], &[]);
    w.feature("site", "mine", [1200.0, 400.0], &[]);
    let deposit = [
        ("radius_m", Prop::Int(300)),
        ("tier", Prop::Text("exceptional".to_owned())),
    ];
    w.feature("resource", "iron-ore", [1800.0, 1500.0], &deposit);
    let row = |r: u32, from: u32, to: u32| (from..to).map(|c| r * 48 + c).collect::<Vec<u32>>();
    w.roads(&[
        (RoadClass::Main, row(10, 5, 30)),
        (RoadClass::Trail, row(20, 2, 20)),
    ]);
    w
}

/// The size of the frame in pixels.
const FRAME_PX: (u32, u32) = (2 * 48, 2 * 40);

// Ledger ruling P3: each render's `needs` lists exactly the layers it reads, with their kind: it
// draws from a world that holds only those (features aside; roads are optional), and fails
// naming the layer without any one of them.
#[test]
fn each_render_needs_exactly_the_layers_it_reads() -> TestResult {
    for r in RENDERS {
        let mut w = world();
        let names: Vec<String> = w.world.names().map(str::to_owned).collect();
        for name in names {
            if r.needs.iter().all(|spec| spec.name != name) {
                w.world.remove(&name);
            }
        }
        let cancel = CancelToken::new();
        (r.draw)(&w.input(&cancel)).map_err(|e| format!("{}: {e}", r.name))?;
        for spec in r.needs {
            let layer = w.world.remove(spec.name).ok_or(spec.name)?;
            assert_eq!(layer.kind(), spec.kind, "{}: {}", r.name, spec.name);
            let missing = (r.draw)(&w.input(&cancel)).err();
            assert!(
                matches!(missing, Some(RenderError::MissingLayer { layer }) if layer == spec.name),
                "{} without `{}`: {missing:?}",
                r.name,
                spec.name
            );
            w.world.insert(spec.name, layer)?;
        }
    }
    Ok(())
}

fn distinct_colours(png: &[u8]) -> usize {
    let image = decode(png);
    let mut seen = std::collections::BTreeSet::new();
    for y in 0..usize::try_from(image.height).unwrap_or(0) {
        for x in 0..usize::try_from(image.width).unwrap_or(0) {
            seen.insert(image.rgb(x, y));
        }
    }
    seen.len()
}

#[test]
fn each_render_has_its_size_and_is_not_blank() -> TestResult {
    let w = world();
    let (width, height) = FRAME_PX;
    for name in PEOPLE {
        let pixmap = w.draw_with(name, None, "A title")?;
        let expected = if name == "truth" {
            (width, height + TRUTH_TOP_PX + TRUTH_BOTTOM_PX)
        } else {
            (width, height)
        };
        assert_eq!((pixmap.width(), pixmap.height()), expected, "{name}");
        let png = canvas::encode_png(&pixmap)?;
        assert!(distinct_colours(&png) > 12, "{name} looks blank");
    }
    Ok(())
}

/// The PNG of `inworld` of `w` with `options`.
fn inworld(w: &TestWorld, options: Option<&toml::Table>) -> Result<Vec<u8>, RenderError> {
    canvas::encode_png(&w.draw_with("inworld", options, "")?)
}

fn table(text: &str) -> toml::Table {
    text.parse().unwrap()
}

#[test]
fn inworld_draws_no_text_without_options() -> TestResult {
    let w = world();
    let empty = table("title = \"\"\nlabels = []");
    assert_eq!(inworld(&w, None)?, inworld(&w, Some(&empty))?);
    assert_eq!(inworld(&w, None)?, inworld(&w, Some(&table("")))?);
    Ok(())
}

#[test]
fn inworld_draws_the_recipe_labels() -> TestResult {
    let w = world();
    let plain = decode(&inworld(&w, None)?);
    let with = |text: &str| inworld(&w, Some(&table(text)));
    let label = decode(&with(
        "labels = [{ text = \"Here\", at = [1200.0, 1000.0] }]",
    )?);
    let (cx, cy) = (1200 / 25, 1000 / 25);
    let changed = (cy - 20..cy + 5)
        .flat_map(|y| (cx - 5..cx + 60).map(move |x| (x, y)))
        .any(|(x, y)| label.rgb(x, y) != plain.rgb(x, y));
    assert!(
        changed,
        "the label must change the pixels near its position"
    );
    let title = decode(&with("title = \"Heading\"")?);
    assert!((0..96).any(|x| (0..40).any(|y| title.rgb(x, y) != plain.rgb(x, y))));
    // A label whose position lies far outside the frame is drawn inside it, whole.
    let far = decode(&with("labels = [{ text = \"Far\", at = [1e9, -1e9] }]")?);
    assert_eq!((far.width, far.height), (plain.width, plain.height));
    let w_px = usize::try_from(far.width)?;
    let h_px = usize::try_from(far.height)?;
    let changed: Vec<(usize, usize)> = (0..h_px)
        .flat_map(|y| (0..w_px).map(move |x| (x, y)))
        .filter(|&(x, y)| far.rgb(x, y) != plain.rgb(x, y))
        .collect();
    assert!(!changed.is_empty(), "a far label is clamped into the frame");
    assert!(
        changed.iter().all(|&(x, y)| x >= w_px - 60 && y < 30),
        "{changed:?}"
    );
    Ok(())
}

#[test]
fn a_malformed_inworld_option_is_an_error() {
    let w = world();
    let bad = [
        "title = 3",
        "labels = 3",
        "labels = [{ text = \"x\" }]",
        "labels = [{ text = \"x\", at = [1] }]",
    ];
    for text in bad {
        let options = table(text);
        let err = w.draw_with("inworld", Some(&options), "").unwrap_err();
        assert!(matches!(err, RenderError::Option { .. }), "{text}: {err}");
    }
}

#[test]
fn omega_marks_ruins() -> TestResult {
    let w = world();
    let mut bare = world();
    bare.features = FeatureSet::default();
    let with = decode(&canvas::encode_png(&w.draw_with("omega", None, "")?)?);
    let without = decode(&canvas::encode_png(&bare.draw_with("omega", None, "")?)?);
    let (x, y) = (1500 / 25, 1200 / 25);
    let ruin = styles::poi_mark("ruin").face;
    assert_eq!(with.rgb(x, y), ruin);
    assert_ne!(without.rgb(x, y), ruin);
    // Omega colours the ground: the western and eastern ends differ.
    assert_ne!(without.rgb(2, 40), without.rgb(90, 40));
    Ok(())
}

#[test]
fn every_render_gives_the_same_bytes_twice() -> TestResult {
    let w = world();
    for name in PEOPLE {
        let first = canvas::encode_png(&w.draw_with(name, None, "A title")?)?;
        let second = canvas::encode_png(&w.draw_with(name, None, "A title")?)?;
        assert_eq!(first, second, "{name}");
    }
    Ok(())
}

#[test]
fn truth_matches_its_recorded_hash() -> TestResult {
    // Recorded on Windows x64 (Task 40, rustc 1.99.1, dev profile); Linux CI on every PR and
    // the weekly `platforms` workflow (Windows, macOS) confirm it elsewhere.
    let png = canvas::encode_png(&world().draw_with("truth", None, "Recipe · seed 7")?)?;
    assert_eq!(
        sha256_hex(&png),
        "a67c11e785c7f2842798e21cc9e3058ee044c2a3b8fc397b7a34847f422646bc"
    );
    Ok(())
}

#[test]
fn every_render_stops_on_cancel() {
    let w = world();
    let cancel = CancelToken::new();
    cancel.cancel();
    for name in PEOPLE {
        let draw = render(name).unwrap();
        let result = draw(&w.input(&cancel));
        assert!(matches!(result, Err(RenderError::Cancelled)), "{name}");
    }
}
