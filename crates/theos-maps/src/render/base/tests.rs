use theos_map_format::sha256_hex;
use tiny_skia::Pixmap;

use super::{Ground, Layers, draw_rivers, fill_lakes, paint_ground};
use crate::grid::Grid;
use crate::kernels::num::{to_f64, to_index};
use crate::progress::CancelToken;
use crate::render::test_support::{TestResult, TestWorld, decode, pixel};
use crate::render::{RenderError, canvas, palette};
use crate::stages::terrain::rocks::Rock;
use crate::world::Layer;

/// The `base` PNG of a 64² flat world of 62.5 m cells with a lake of 5 x 5 cells centred on
/// `centre` (metres), and a wide river that runs into the lake's centre.
fn render_base_of_test_world_with_lake(centre: [f64; 2]) -> Result<Vec<u8>, RenderError> {
    let mut w = TestWorld::flat(64, 64, 62.5);
    let (r, c) = w.world.frame().cell_at_m(centre[0], centre[1]);
    w.lake(r - 2..r + 3, c - 2..c + 3);
    w.river(&[[centre[0] - 900.0, centre[1]], centre], 200.0);
    w.png("base")
}

// AC: CE-28.6
#[test]
fn a_lake_at_a_known_position_lands_on_its_pixel() -> TestResult {
    // 64² flat world, cell 62.5 m; a lake of 5×5 cells centred on (1000 m, 1500 m)
    let png = render_base_of_test_world_with_lake([1000.0, 1500.0])?;
    let (px, py) = (1000.0 / 62.5 * 2.0, 1500.0 / 62.5 * 2.0); // 32, 48
    assert_eq!(
        pixel(&png, to_index(px, 128), to_index(py, 128)),
        palette::LAKE
    );
    // The whole lake is lake colour, though the river's stroke ends in its centre; the river
    // west of it is river colour, and the ground beyond both is neither.
    let image = decode(&png);
    // Two pixels per cell, exactly the frame (final review round 4, A2).
    assert_eq!((image.width, image.height), (128, 128));
    for y in 44..54 {
        for x in 28..38 {
            assert_eq!(image.rgb(x, y), palette::LAKE, "({x}, {y})");
        }
    }
    // The pixels just beyond the lake's cells, north, south and east, are no lake.
    for (x, y) in [(32, 43), (32, 54), (38, 48)] {
        assert_ne!(image.rgb(x, y), palette::LAKE, "({x}, {y})");
    }
    assert_eq!(image.rgb(20, 48), palette::RIVER);
    let ground = image.rgb(100, 100);
    assert!(ground != palette::LAKE && ground != palette::RIVER);
    Ok(())
}

#[test]
fn base_is_frame_exact() -> TestResult {
    let w = TestWorld::flat(24, 40, 50.0);
    let pixmap = w.draw("base", &CancelToken::new())?;
    assert_eq!((pixmap.width(), pixmap.height()), (80, 48));
    let image = decode(&w.png("base")?);
    assert_eq!((image.width, image.height), (80, 48));
    assert!((0..48).all(|y| (0..80).all(|x| image.rgba(x, y)[3] == u8::MAX)));
    Ok(())
}

#[test]
fn a_render_stops_on_cancel() {
    let w = TestWorld::flat(16, 16, 50.0);
    let cancel = CancelToken::new();
    cancel.cancel();
    assert!(matches!(
        w.draw("base", &cancel),
        Err(RenderError::Cancelled)
    ));
}

/// A cancel check that lets `n` rows (or lines) through and then reports a cancel, as the
/// run's token would when it is set while a pass runs.
fn cancel_after(n: usize) -> impl FnMut() -> Result<(), RenderError> {
    let mut passed = 0;
    move || {
        if passed < n {
            passed += 1;
            Ok(())
        } else {
            Err(RenderError::Cancelled)
        }
    }
}

/// Whether each pixel row of `pixmap` has been drawn at column `x`.
fn drawn_rows(pixmap: &Pixmap, x: u32) -> Vec<bool> {
    (0..pixmap.height())
        .map(|y| pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 0))
        .collect()
}

#[test]
fn every_row_and_line_checks_for_cancel() -> TestResult {
    // 4 x 4 cells of 50 m, 8 x 8 pixels; all lake; rivers along pixel rows 0-1 and 6-7.
    let mut w = TestWorld::flat(4, 4, 50.0);
    w.lake(0..4, 0..4);
    w.river(&[[0.0, 25.0], [200.0, 25.0]], 50.0);
    w.river(&[[0.0, 175.0], [200.0, 175.0]], 50.0);
    let cancel = CancelToken::new();
    let input = w.input(&cancel);
    let layers = Layers::read(&input)?;
    let frame = *w.world.frame();
    let ground = Ground::new(&layers, &frame, &w.features);
    let cancelled = |r: Result<(), RenderError>| matches!(r, Err(RenderError::Cancelled));

    // The ground pass stops after two rows: they are painted, the rest is still empty.
    let mut p = canvas::pixmap(&frame)?;
    assert!(cancelled(paint_ground(&mut p, &ground, cancel_after(2))));
    let first_two = [true, true, false, false, false, false, false, false];
    assert_eq!(drawn_rows(&p, 3), first_two);

    // The lake pass stops after three rows.
    let mut p = canvas::pixmap(&frame)?;
    assert!(cancelled(fill_lakes(&mut p, layers.lake, cancel_after(3))));
    let first_three = [true, true, true, false, false, false, false, false];
    assert_eq!(drawn_rows(&p, 3), first_three);

    // The river pass stops after the first line: the second is not drawn.
    let mut p = canvas::pixmap(&frame)?;
    let rivers = &layers.water.rivers;
    assert!(cancelled(draw_rivers(
        &mut p,
        rivers,
        25.0,
        cancel_after(1)
    )));
    let first_line = [true, true, false, false, false, false, false, false];
    assert_eq!(drawn_rows(&p, 3), first_line);

    // Without a cancel every pass runs to the end.
    let mut p = canvas::pixmap(&frame)?;
    draw_rivers(&mut p, rivers, 25.0, cancel_after(usize::MAX))?;
    let both_lines = [true, true, false, false, false, false, true, true];
    assert_eq!(drawn_rows(&p, 3), both_lines);
    paint_ground(&mut p, &ground, cancel_after(usize::MAX))?;
    fill_lakes(&mut p, layers.lake, cancel_after(usize::MAX))?;
    assert_eq!(drawn_rows(&p, 3), [true; 8]);
    Ok(())
}

// Final review round 4, class I: a pixel of a farmed row sums over the farms in reach, whose
// number follows the settlement counts, so the ground pass checks before every such pixel.
#[test]
fn a_farmed_row_checks_for_cancel_before_every_pixel() -> TestResult {
    let mut w = TestWorld::flat(4, 4, 50.0);
    w.settlement("town", [100.0, 100.0]);
    let cancel = CancelToken::new();
    let input = w.input(&cancel);
    let layers = Layers::read(&input)?;
    let frame = *w.world.frame();
    let ground = Ground::new(&layers, &frame, &w.features);
    let mut p = canvas::pixmap(&frame)?;
    // The row check and three pixel checks pass: three pixels of row 0 are painted.
    let result = paint_ground(&mut p, &ground, cancel_after(4));
    assert!(matches!(result, Err(RenderError::Cancelled)));
    let painted: Vec<bool> = (0..8)
        .map(|x| p.pixel(x, 0).is_some_and(|px| px.alpha() > 0))
        .collect();
    assert_eq!(
        painted,
        [true, true, true, false, false, false, false, false]
    );
    Ok(())
}

#[test]
fn a_missing_layer_is_named() {
    for layer in [
        "height",
        "rock",
        "vegetation",
        "snow",
        "lake",
        "water_features",
    ] {
        let mut w = TestWorld::flat(8, 8, 50.0);
        w.world.remove(layer);
        let err = w.draw("base", &CancelToken::new()).unwrap_err();
        assert!(
            matches!(err, RenderError::MissingLayer { layer: l } if l == layer),
            "{layer}: {err}"
        );
    }
}

#[test]
fn rivers_are_drawn_by_width_with_a_minimum() -> TestResult {
    // 25 m per pixel. A 150 m river along pixel row 16, a 1 m river along pixel row 48.
    let mut w = TestWorld::flat(32, 32, 50.0);
    w.river(&[[100.0, 412.5], [700.0, 412.5]], 150.0);
    w.river(&[[100.0, 1212.5], [700.0, 1212.5]], 1.0);
    let image = decode(&w.png("base")?);
    let ground = image.rgb(14, 30);
    assert_eq!(image.rgb(14, 16), palette::RIVER);
    assert_eq!(image.rgb(14, 18), palette::RIVER, "6 px wide");
    assert_eq!(image.rgb(14, 20), ground);
    // The thin river is drawn at the minimum width: its row is tinted, two rows off is ground.
    let thin = image.rgb(14, 48);
    assert!(thin != ground && thin[2] > thin[0], "{thin:?}");
    assert_eq!(image.rgb(14, 50), ground);
    Ok(())
}

#[test]
fn farmland_lies_around_settlements() -> TestResult {
    // 4 km square; a town farms 1.5 km around it.
    let mut w = TestWorld::flat(64, 64, 62.5);
    let ground = decode(&w.png("base")?).rgb(64, 64);
    w.settlement("town", [2000.0, 2000.0]);
    let image = decode(&w.png("base")?);
    assert_ne!(image.rgb(64, 64), ground);
    assert_ne!(image.rgb(64 + 30, 64), ground);
    assert_eq!(image.rgb(2, 2), ground);
    // A settlement type without farmland adds none.
    let mut odd = TestWorld::flat(64, 64, 62.5);
    odd.settlement("camp", [2000.0, 2000.0]);
    assert_eq!(decode(&odd.png("base")?).rgb(64, 64), ground);
    Ok(())
}

#[test]
fn vegetation_snow_and_rock_colour_the_ground() -> TestResult {
    let mut w = TestWorld::flat(32, 32, 50.0);
    w.set("vegetation", |_, c| if c < 16 { 1.0 } else { 0.0 });
    w.set("snow", |r, _| if r < 8 { 1.0 } else { 0.0 });
    let image = decode(&w.png("base")?);
    let bare = image.rgb(50, 50);
    let forest = image.rgb(10, 50);
    let snow = image.rgb(50, 4);
    assert!(
        forest[1] > forest[0] && forest[1] < bare[1],
        "{forest:?} {bare:?}"
    );
    assert!((0..3).all(|i| snow[i] > bare[i]), "{snow:?} {bare:?}");

    // On steep ground the rock shows: basalt is darker than sandstone.
    let mut steep = TestWorld::flat(32, 32, 50.0);
    steep.set("height", |_, c| 2000.0 - 60.0 * to_f64(c));
    let sandstone = decode(&steep.png("base")?).rgb(32, 32);
    let rock = Grid::filled(32, 32, Rock::Basalt.code());
    steep.world.insert("rock", Layer::U8(rock))?;
    let basalt = decode(&steep.png("base")?).rgb(32, 32);
    assert!(
        (0..3).all(|i| basalt[i] < sandstone[i]),
        "{basalt:?} {sandstone:?}"
    );
    Ok(())
}

/// A small world with every element of `base`: a hill with snow, rocks by column, vegetation
/// by row, a lake, a river with a bend and a village.
fn every_element() -> TestWorld {
    let mut w = TestWorld::flat(20, 28, 100.0);
    w.set("height", |r, c| {
        let (dr, dc) = (to_f64(r) - 6.0, to_f64(c) - 8.0);
        400.0 + 900.0 * libm::exp(-(dr * dr + dc * dc) / 18.0)
    });
    w.set("snow", |r, c| {
        if r < 8 && (5..12).contains(&c) {
            0.8
        } else {
            0.0
        }
    });
    w.set("vegetation", |r, _| to_f64(r) / 19.0);
    let rocks = (0..20 * 28)
        .map(|i| u8::try_from(i % 28 / 4).unwrap())
        .collect();
    let rocks = Grid::from_vec(20, 28, rocks).unwrap();
    w.world.insert("rock", Layer::U8(rocks)).unwrap();
    w.lake(14..17, 20..24);
    w.river(&[[1250.0, 300.0], [1700.0, 900.0], [2050.0, 1450.0]], 30.0);
    w.settlement("village", [900.0, 1500.0]);
    w
}

#[test]
fn base_gives_the_same_bytes_twice_and_matches_the_recorded_hash() -> TestResult {
    let w = every_element();
    let first = w.png("base")?;
    assert_eq!(w.png("base")?, first);
    // Recorded on Windows x64 only (Task 39, rustc 1.99.1, dev and release profiles). Linux CI
    // on every PR and the weekly `platforms` workflow (Windows, macOS) confirm it elsewhere.
    assert_eq!(
        sha256_hex(&first),
        "39d87125346c4e4fc2bea721a7efe0c4426c852fbe805cddcf4a26e91422772b"
    );
    Ok(())
}
