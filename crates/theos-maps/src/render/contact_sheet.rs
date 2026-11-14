//! The contact sheet (spec §5.11): PNG renders of several runs tiled in rows, each under its
//! label in Inter. The developer example and `theos gen run` use it to compare seeds.
//!
//! Tiles are read with `png` and shrunk by a whole factor (box average, exact integer
//! arithmetic) until each fits `TILE_MAX_PX`, so a sheet of large renders stays a size one can
//! open; all tiles share the factor, so their scale is the same.

use std::fs::File;
use std::path::{Path, PathBuf};

use tiny_skia::{Color, Pixmap};

use super::fonts::fonts;
use super::palette::Rgb;
use super::styles::INK;
use super::{RenderError, canvas, text};
use crate::kernels::num::{max, to_f32, to_f64};

/// The widest or tallest a tile gets on the sheet, pixels.
const TILE_MAX_PX: usize = 512;
/// The space around and between tiles, pixels.
const PAD_PX: usize = 10;
/// The height of the label above a tile, pixels.
const LABEL_BAND_PX: usize = 26;
/// The label's size, pixels.
const LABEL_PX: f32 = 16.0;
/// The sheet's background.
const BACKGROUND: Rgb = [0xf4, 0xea, 0xd2];

/// A decoded tile: opaque RGB.
struct Tile {
    width: usize,
    height: usize,
    rgb: Vec<Rgb>,
}

/// The error for the tile at `path`.
fn tile_error(path: &Path, reason: &impl ToString) -> RenderError {
    RenderError::Tile {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

/// Reads the 8-bit PNG at `path`.
fn read_tile(path: &Path) -> Result<Tile, RenderError> {
    let file = File::open(path).map_err(|e| tile_error(path, &e))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| tile_error(path, &e))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| tile_error(path, &"the image is too large"))?;
    let mut data = vec![0; size];
    let info = reader
        .next_frame(&mut data)
        .map_err(|e| tile_error(path, &e))?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(tile_error(path, &"only 8-bit PNGs are read"));
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(tile_error(path, &"the palette was not expanded")),
    };
    let rgb = data[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|px| {
            if channels < 3 {
                [px[0]; 3]
            } else {
                [px[0], px[1], px[2]]
            }
        })
        .collect();
    let (width, height) = (
        usize::try_from(info.width).unwrap_or(0),
        usize::try_from(info.height).unwrap_or(0),
    );
    Ok(Tile { width, height, rgb })
}

/// The mean colour of the `factor` x `factor` block of `tile` at `(x, y)`.
fn block_mean(tile: &Tile, x: usize, y: usize, factor: usize) -> Rgb {
    let mut sum = [0_u32; 3];
    for dy in 0..factor {
        for dx in 0..factor {
            let px = tile.rgb[(y * factor + dy) * tile.width + x * factor + dx];
            for (s, v) in sum.iter_mut().zip(px) {
                *s += u32::from(v);
            }
        }
    }
    let n = u32::try_from(factor * factor).unwrap_or(1);
    sum.map(|s| u8::try_from(s / n).unwrap_or(u8::MAX))
}

/// Tiles the PNGs of `tiles` (label and path) in rows of `columns` under their labels, and
/// writes the sheet as one PNG to `out`. Five tiles in two columns give three rows.
///
/// # Errors
/// [`RenderError::NoTiles`] for no tiles or no columns; [`RenderError::Tile`] naming the file
/// of a tile that cannot be read; [`RenderError::Output`] when `out` cannot be written.
pub fn contact_sheet(
    tiles: &[(String, PathBuf)],
    columns: usize,
    out: &Path,
) -> Result<(), RenderError> {
    if tiles.is_empty() || columns == 0 {
        return Err(RenderError::NoTiles);
    }
    let images = tiles
        .iter()
        .map(|(_, path)| read_tile(path))
        .collect::<Result<Vec<_>, _>>()?;
    let factor = images
        .iter()
        .map(|t| t.width.max(t.height).div_ceil(TILE_MAX_PX))
        .max()
        .unwrap_or(1)
        .max(1);
    let cell_w = images.iter().map(|t| t.width / factor).max().unwrap_or(0);
    let cell_h = images.iter().map(|t| t.height / factor).max().unwrap_or(0);
    let rows = tiles.len().div_ceil(columns);
    let width = columns * (cell_w + PAD_PX) + PAD_PX;
    let height = rows * (LABEL_BAND_PX + cell_h + PAD_PX) + PAD_PX;
    let size = RenderError::Size {
        rows: height,
        cols: width,
    };
    let (w, h) = (u32::try_from(width), u32::try_from(height));
    let mut sheet = match (w, h) {
        (Ok(w), Ok(h)) => Pixmap::new(w, h).ok_or(size)?,
        _ => return Err(size),
    };
    sheet.fill(Color::from_rgba8(
        BACKGROUND[0],
        BACKGROUND[1],
        BACKGROUND[2],
        u8::MAX,
    ));
    let face = &fonts()?.inter;
    for (n, ((label, _), tile)) in tiles.iter().zip(&images).enumerate() {
        let x0 = PAD_PX + (n % columns) * (cell_w + PAD_PX);
        let y0 = PAD_PX + (n / columns) * (LABEL_BAND_PX + cell_h + PAD_PX);
        let origin = [to_f32(to_f64(x0)), to_f32(to_f64(y0 + LABEL_BAND_PX - 8))];
        let size = text::fit(face, label, LABEL_PX, to_f32(max(to_f64(cell_w), 1.0)));
        text::draw(&mut sheet, face, label, size, origin, INK)?;
        let top = y0 + LABEL_BAND_PX;
        let rows = canvas::rows_mut(&mut sheet)
            .skip(top)
            .take(tile.height / factor);
        for (ty, row) in rows.enumerate() {
            for (tx, px) in row[x0..x0 + tile.width / factor].iter_mut().enumerate() {
                canvas::put(px, block_mean(tile, tx, ty, factor));
            }
        }
    }
    let bytes = canvas::encode_png(&sheet)?;
    std::fs::write(out, bytes).map_err(|source| RenderError::Output {
        path: out.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::decode;

    /// Writes a `w` x `h` PNG filled with `colour` as `name` in `dir`.
    fn tile(dir: &Path, name: &str, w: u32, h: u32, colour: Rgb) -> (String, PathBuf) {
        let mut pixmap = Pixmap::new(w, h).unwrap();
        pixmap.fill(Color::from_rgba8(colour[0], colour[1], colour[2], 255));
        let path = dir.join(format!("{name}.png"));
        std::fs::write(&path, canvas::encode_png(&pixmap).unwrap()).unwrap();
        (name.to_owned(), path)
    }

    #[test]
    fn the_contact_sheet_tiles_in_rows() {
        let dir = tempfile::tempdir().unwrap();
        let colours = [
            [200, 0, 0],
            [0, 200, 0],
            [0, 0, 200],
            [200, 200, 0],
            [0, 200, 200],
        ];
        let tiles: Vec<_> = colours
            .iter()
            .enumerate()
            .map(|(i, c)| tile(dir.path(), &format!("t{i}"), 20, 10, *c))
            .collect();
        let out = dir.path().join("sheet.png");
        contact_sheet(&tiles, 2, &out).unwrap();
        let sheet = decode(&std::fs::read(&out).unwrap());
        // Five tiles in two columns: three rows.
        assert_eq!(
            usize::try_from(sheet.width).unwrap(),
            2 * (20 + PAD_PX) + PAD_PX
        );
        assert_eq!(
            usize::try_from(sheet.height).unwrap(),
            3 * (LABEL_BAND_PX + 10 + PAD_PX) + PAD_PX
        );
        let at = |col: usize, row: usize| {
            let x = PAD_PX + col * (20 + PAD_PX) + 3;
            let y = PAD_PX + row * (LABEL_BAND_PX + 10 + PAD_PX) + LABEL_BAND_PX + 3;
            sheet.rgb(x, y)
        };
        assert_eq!(at(0, 0), colours[0]);
        assert_eq!(at(1, 0), colours[1]);
        assert_eq!(at(0, 2), colours[4]);
        // The label is inked above the first tile.
        let band_y = PAD_PX + 8;
        assert!((PAD_PX..PAD_PX + 20).any(|x| sheet.rgb(x, band_y) != BACKGROUND));
    }

    #[test]
    fn large_tiles_are_shrunk_by_a_whole_factor() {
        let dir = tempfile::tempdir().unwrap();
        let tiles = vec![tile(dir.path(), "big", 1100, 40, [10, 20, 30])];
        let out = dir.path().join("sheet.png");
        contact_sheet(&tiles, 1, &out).unwrap();
        let sheet = decode(&std::fs::read(&out).unwrap());
        // 1100 px wide needs a factor of 3: 366 px wide, 13 px high.
        assert_eq!(usize::try_from(sheet.width).unwrap(), 366 + 2 * PAD_PX);
        assert_eq!(
            usize::try_from(sheet.height).unwrap(),
            LABEL_BAND_PX + 13 + 2 * PAD_PX
        );
        assert_eq!(
            sheet.rgb(PAD_PX + 5, PAD_PX + LABEL_BAND_PX + 5),
            [10, 20, 30]
        );
    }

    #[test]
    fn errors_name_the_failing_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("sheet.png");
        let good = tile(dir.path(), "good", 8, 8, [1, 2, 3]);
        let missing = ("gone".to_owned(), dir.path().join("gone.png"));
        let err = contact_sheet(&[good.clone(), missing], 2, &out).unwrap_err();
        assert!(err.to_string().contains("gone.png"), "{err}");
        let bad_path = dir.path().join("bad.png");
        std::fs::write(&bad_path, b"not a png").unwrap();
        let err =
            contact_sheet(&[good.clone(), ("bad".to_owned(), bad_path)], 2, &out).unwrap_err();
        assert!(
            matches!(&err, RenderError::Tile { path, .. } if path.ends_with("bad.png")),
            "{err}"
        );
        assert!(matches!(
            contact_sheet(&[], 2, &out),
            Err(RenderError::NoTiles)
        ));
        assert!(matches!(
            contact_sheet(std::slice::from_ref(&good), 0, &out),
            Err(RenderError::NoTiles)
        ));
        let err = contact_sheet(&[good], 1, dir.path()).unwrap_err();
        assert!(matches!(err, RenderError::Output { .. }), "{err}");
    }
}
