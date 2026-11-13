//! The fonts of the renders and their licences (spec §5.11).
//!
//! Inter and Spectral come from the design system (`design/theos-ui/fonts`) as WOFF2, embedded
//! with `include_bytes!`, and are decoded once per process: `wuff` turns WOFF2 into OpenType,
//! which `skrifa` reads. `skrifa` is built with `libm`, so glyph outlines are the same on every
//! platform; the renders turn them into `tiny-skia` paths (`text.rs`), so text and shapes go
//! through one rasterizer. Both fonts are under the SIL Open Font License 1.1, whose texts are
//! embedded too.

use std::sync::OnceLock;

use skrifa::FontRef;

use super::RenderError;

/// Every embedded font with its licence text.
const LICENCES: &[(&str, &str)] = &[
    (
        "Inter",
        include_str!("../../../../design/theos-ui/licenses/Inter-OFL.txt"),
    ),
    (
        "Spectral",
        include_str!("../../../../design/theos-ui/licenses/Spectral-OFL.txt"),
    ),
];

/// Inter, variable, as WOFF2; decoded on first use.
static INTER: Embedded = Embedded::new(
    "Inter",
    include_bytes!("../../../../design/theos-ui/fonts/InterVariable.woff2"),
);
/// Spectral Regular, as WOFF2; decoded on first use.
static SPECTRAL: Embedded = Embedded::new(
    "Spectral",
    include_bytes!("../../../../design/theos-ui/fonts/Spectral-Regular.woff2"),
);

/// An embedded font that is not what it should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the embedded font {font} {problem}")]
pub struct FontError {
    /// The font's name.
    pub font: &'static str,
    /// What is wrong with it, such as "is not valid WOFF2".
    pub problem: &'static str,
}

/// An embedded font: its name, its WOFF2 bytes and, once decoded, its OpenType bytes.
struct Embedded {
    name: &'static str,
    woff2: &'static [u8],
    otf: OnceLock<Result<Vec<u8>, FontError>>,
}

impl Embedded {
    /// The font `name` from `woff2`, not decoded yet.
    const fn new(name: &'static str, woff2: &'static [u8]) -> Self {
        Self {
            name,
            woff2,
            otf: OnceLock::new(),
        }
    }

    /// The font, decoding it on the first call.
    fn face(&'static self) -> Result<Face, FontError> {
        let otf = self
            .otf
            .get_or_init(|| decode(self.name, self.woff2))
            .as_ref()
            .map_err(|e| *e)?;
        let font = FontRef::new(otf).map_err(|_| FontError {
            font: self.name,
            problem: "is not a valid OpenType font",
        })?;
        Ok(Face {
            name: self.name,
            font,
        })
    }
}

/// `woff2`, the font `font`, as OpenType bytes.
fn decode(font: &'static str, woff2: &[u8]) -> Result<Vec<u8>, FontError> {
    wuff::decompress_woff2(woff2).map_err(|_| FontError {
        font,
        problem: "is not valid WOFF2",
    })
}

/// A decoded font with its name, for the text of the renders (`text.rs`).
///
/// `font` is a `skrifa` font over the decoded OpenType bytes, which live as long as the
/// process: `font.charmap()` maps characters to glyphs, `font.glyph_metrics(size, location)`
/// gives advance widths, and `font.outline_glyphs()` draws outlines into an `OutlinePen`. The
/// renders use the default instance of a variable font (`LocationRef::default()`) and unhinted
/// outlines.
#[derive(Clone)]
pub(crate) struct Face {
    /// The font's name, for errors.
    pub name: &'static str,
    /// The font.
    pub font: FontRef<'static>,
}

/// The decoded fonts: Inter for `truth`, legends and the contact sheet, Spectral for `inworld`.
pub(crate) struct Fonts {
    /// Inter (variable, default instance: Regular).
    pub inter: Face,
    /// Spectral Regular.
    pub spectral: Face,
}

/// The fonts, decoded on the first call; later calls return the same fonts (or error).
pub(crate) fn fonts() -> Result<&'static Fonts, RenderError> {
    static FONTS: OnceLock<Result<Fonts, FontError>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            Ok(Fonts {
                inter: INTER.face()?,
                spectral: SPECTRAL.face()?,
            })
        })
        .as_ref()
        .map_err(|e| RenderError::Font(*e))
}

/// The fonts the renders embed, each with the full text of its licence (SIL OFL 1.1), so the
/// binaries that embed this crate can ship them.
#[must_use]
pub fn font_licences() -> &'static [(&'static str, &'static str)] {
    LICENCES
}

#[cfg(test)]
mod tests {
    use skrifa::MetadataProvider;
    use skrifa::instance::{LocationRef, Size};
    use skrifa::outline::{DrawSettings, OutlinePen};

    use super::*;

    /// Counts the drawing commands of an outline.
    struct Count(usize);

    impl OutlinePen for Count {
        fn move_to(&mut self, _: f32, _: f32) {
            self.0 += 1;
        }
        fn line_to(&mut self, _: f32, _: f32) {
            self.0 += 1;
        }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1;
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1;
        }
        fn close(&mut self) {
            self.0 += 1;
        }
    }

    #[test]
    fn fonts_decode_and_licences_are_exposed() {
        let fonts = fonts().unwrap();
        for (face, name) in [(&fonts.inter, "Inter"), (&fonts.spectral, "Spectral")] {
            assert_eq!(face.name, name);
            let a = face.font.charmap().map('A').unwrap();
            assert_ne!(a.to_u32(), 0, "{name} maps 'A'");
            let size = Size::new(24.0);
            let advance = face
                .font
                .glyph_metrics(size, LocationRef::default())
                .advance_width(a)
                .unwrap();
            assert!(advance > 5.0 && advance < 24.0, "{name}: {advance}");
            let glyph = face.font.outline_glyphs().get(a).unwrap();
            let mut pen = Count(0);
            glyph
                .draw(
                    DrawSettings::unhinted(size, LocationRef::default()),
                    &mut pen,
                )
                .unwrap();
            assert!(pen.0 > 3, "{name} draws an outline for 'A'");
        }
        // Decoded once: every call hands out the same fonts.
        assert!(std::ptr::eq(fonts, super::fonts().unwrap()));

        let licences = font_licences();
        let names: Vec<&str> = licences.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["Inter", "Spectral"]);
        for (name, text) in licences {
            assert!(text.contains("SIL OPEN FONT LICENSE Version 1.1"), "{name}");
            assert!(
                text.contains(&format!("The {name} Project Authors")),
                "{name}"
            );
        }
    }

    #[test]
    fn a_broken_font_is_named() {
        static BROKEN: Embedded = Embedded::new("Broken", b"wOF2 but not really");
        let err = decode("Broken", b"not a font").unwrap_err();
        assert_eq!(
            err,
            FontError {
                font: "Broken",
                problem: "is not valid WOFF2"
            }
        );
        assert_eq!(BROKEN.face().err(), Some(err));
        assert_eq!(
            RenderError::from(err).to_string(),
            "the embedded font Broken is not valid WOFF2"
        );
    }
}
