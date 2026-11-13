//! One line of text as a `tiny-skia` path, for the labels, legends and titles of the renders.
//!
//! The glyphs come from `skrifa`: the character map, the advance widths and the unhinted
//! outlines of the font's default instance. They are laid out left to right by their advance
//! widths (no kerning, no shaping) and collected into one path, which the caller fills with
//! `canvas::fill` like any other shape. A character the font does not map draws the font's
//! `.notdef` glyph, so a missing glyph shows instead of vanishing.

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia::{Path, PathBuilder};

use super::fonts::{Face, FontError};

/// Collects outlines into a path: font coordinates (pixels, y up) from the pen position
/// `(x, y)` (pixels, y down).
struct Pen {
    path: PathBuilder,
    x: f32,
    y: f32,
}

impl Pen {
    /// The point `(x, y)` of an outline in pixels of the pixmap.
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x, self.y - y)
    }
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.path.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.path.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let ((cx0, cy0), (x, y)) = (self.at(cx0, cy0), self.at(x, y));
        self.path.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let ((cx0, cy0), (cx1, cy1), (x, y)) =
            (self.at(cx0, cy0), self.at(cx1, cy1), self.at(x, y));
        self.path.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.path.close();
    }
}

/// The glyph of `c` in `face`, `.notdef` when the font has none.
fn glyph(face: &Face, c: char) -> GlyphId {
    face.font.charmap().map(c).unwrap_or(GlyphId::NOTDEF)
}

/// The advance width of `text` in `face` at `size_px` (the em, in pixels).
pub(crate) fn width(face: &Face, text: &str, size_px: f32) -> f32 {
    let metrics = face
        .font
        .glyph_metrics(Size::new(size_px), LocationRef::default());
    text.chars()
        .map(|c| metrics.advance_width(glyph(face, c)).unwrap_or(0.0))
        .fold(0.0, |sum, w| sum + w)
}

/// `text` in `face` at `size_px` (the em, in pixels), its baseline starting at `origin`
/// (pixels, y down), as one path; `None` when no glyph has an outline (an empty or blank text).
///
/// # Errors
/// A [`FontError`] naming the font when a glyph outline cannot be drawn.
pub(crate) fn line(
    face: &Face,
    text: &str,
    size_px: f32,
    origin: [f32; 2],
) -> Result<Option<Path>, FontError> {
    let size = Size::new(size_px);
    let settings = || DrawSettings::unhinted(size, LocationRef::default());
    let metrics = face.font.glyph_metrics(size, LocationRef::default());
    let outlines = face.font.outline_glyphs();
    let mut pen = Pen {
        path: PathBuilder::new(),
        x: origin[0],
        y: origin[1],
    };
    for c in text.chars() {
        let id = glyph(face, c);
        if let Some(outline) = outlines.get(id) {
            outline.draw(settings(), &mut pen).map_err(|_| FontError {
                font: face.name,
                problem: "has a glyph outline that cannot be drawn",
            })?;
        }
        pen.x += metrics.advance_width(id).unwrap_or(0.0);
    }
    Ok(pen.path.finish())
}

#[cfg(test)]
mod tests {
    use theos_map_format::sha256_hex;
    use tiny_skia::Pixmap;

    use super::*;
    use crate::render::canvas::{encode_png, fill};
    use crate::render::fonts::fonts;
    use crate::render::test_support::decode;

    /// "Theos 0123 Ag" in Inter on the first line and in Spectral on the second, dark on white.
    fn sample() -> Vec<u8> {
        let fonts = fonts().unwrap();
        let mut pixmap = Pixmap::new(170, 56).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        for (face, baseline) in [(&fonts.inter, 22.0), (&fonts.spectral, 48.0)] {
            let path = line(face, "Theos 0123 Ag", 20.0, [4.0, baseline])
                .unwrap()
                .unwrap();
            fill(&mut pixmap, &path, [0x1d, 0x14, 0x0a]);
        }
        encode_png(&pixmap).unwrap()
    }

    #[test]
    fn a_line_of_text_draws_the_same_pixels_twice() {
        let png = sample();
        assert_eq!(sample(), png);
        // Recorded on Windows x64 (Task 39); text goes through the scalar tiny-skia pipeline.
        assert_eq!(
            sha256_hex(&png),
            "c6337f9e9bb3d463ba577fb8dccfa14c425fe2e6fca2e4199ee06d134bd3e419"
        );
        // Ink lies between the cap height and the descender of each line, and only there.
        let image = decode(&png);
        let inked = |y: usize| (0..170).any(|x| image.rgb(x, y) != [255; 3]);
        assert!(
            inked(10) && inked(20) && inked(24),
            "Inter: cap, x-height, descender"
        );
        assert!(inked(36) && inked(46) && inked(50), "Spectral");
        assert!(!inked(1) && !inked(29) && !inked(55));
    }

    #[test]
    fn widths_follow_the_advances_and_blanks_draw_nothing() {
        let inter = &fonts().unwrap().inter;
        let one = width(inter, "Theos", 20.0);
        let two = width(inter, "TheosTheos", 20.0);
        assert!(one > 20.0 && (two - 2.0 * one).abs() < 1e-3, "{one} {two}");
        assert!((width(inter, "", 20.0)).abs() < f32::EPSILON);
        assert!(line(inter, "", 20.0, [0.0, 0.0]).unwrap().is_none());
        assert!(line(inter, "   ", 20.0, [0.0, 0.0]).unwrap().is_none());
        // A character without a glyph draws the .notdef box rather than nothing.
        assert_eq!(glyph(inter, '\u{4E2D}'), GlyphId::NOTDEF);
        assert!(
            line(inter, "\u{4E2D}", 20.0, [0.0, 20.0])
                .unwrap()
                .is_some()
        );
    }
}
