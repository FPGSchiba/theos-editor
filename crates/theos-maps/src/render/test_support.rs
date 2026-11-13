//! A synthetic world for the render tests (synthetic fixture, not canon) and a PNG reader.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::ops::Range;

use tiny_skia::Pixmap;

use super::{RenderError, RenderInput, encode_png, render};
use crate::features::{Feature, FeatureSet, Geometry};
use crate::grid::Grid;
use crate::progress::CancelToken;
use crate::stages::terrain::hydrology::{RiverLine, WaterFeatures};
use crate::stages::terrain::rocks::Rock;
use crate::world::{Layer, MapFrame, World};

/// What the render tests return.
pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The ground of a flat test world, m.
pub(crate) const GROUND_M: f64 = 500.0;

/// A world with every layer `base` draws, and its features.
pub(crate) struct TestWorld {
    /// The world.
    pub world: World,
    /// The features.
    pub features: FeatureSet,
    rivers: Vec<RiverLine>,
}

impl TestWorld {
    /// A flat world of `rows` x `cols` cells of `cell_m` metres at [`GROUND_M`]: sandstone, bare,
    /// no snow, no lake, no river, no settlement.
    pub(crate) fn flat(rows: usize, cols: usize, cell_m: f64) -> Self {
        let mut world = World::new(MapFrame { rows, cols, cell_m });
        let f64s = [("height", GROUND_M), ("vegetation", 0.0), ("snow", 0.0)];
        for (name, value) in f64s {
            world
                .insert(name, Layer::F64(Grid::filled(rows, cols, value)))
                .unwrap();
        }
        world
            .insert(
                "rock",
                Layer::U8(Grid::filled(rows, cols, Rock::Sandstone.code())),
            )
            .unwrap();
        world
            .insert("lake", Layer::Mask(Grid::filled(rows, cols, false)))
            .unwrap();
        let mut w = Self {
            world,
            features: FeatureSet::default(),
            rivers: Vec::new(),
        };
        w.store_rivers();
        w
    }

    fn store_rivers(&mut self) {
        let water = WaterFeatures {
            rivers: self.rivers.clone(),
            lakes: Vec::new(),
        };
        self.world
            .insert("water_features", Layer::Value(Box::new(water)))
            .unwrap();
    }

    /// Sets every cell of the float raster `name` to `value(row, col)`.
    pub(crate) fn set(&mut self, name: &str, value: impl Fn(usize, usize) -> f64) {
        let frame = *self.world.frame();
        let mut g = Grid::filled(frame.rows, frame.cols, 0.0);
        for r in 0..frame.rows {
            for c in 0..frame.cols {
                g.set(r, c, value(r, c));
            }
        }
        self.world.insert(name, Layer::F64(g)).unwrap();
    }

    /// Makes the cells in `rows` x `cols` a lake.
    pub(crate) fn lake(&mut self, rows: Range<usize>, cols: Range<usize>) {
        let mut lake = self.world.mask("lake").unwrap().clone();
        for r in rows {
            for c in cols.clone() {
                lake.set(r, c, true);
            }
        }
        self.world.insert("lake", Layer::Mask(lake)).unwrap();
    }

    /// Adds a river through the points `[x, y]` (metres), `width_m` wide everywhere.
    pub(crate) fn river(&mut self, points: &[[f64; 2]], width_m: f64) {
        let n = points.len();
        self.rivers.push(RiverLine {
            points: points.iter().map(|p| [p[0], p[1], GROUND_M]).collect(),
            width_m: vec![width_m; n],
            depth_m: vec![1.0; n],
            discharge_m3s: vec![10.0; n],
            order: 1,
        });
        self.store_rivers();
    }

    /// Adds a settlement of type `ty` at `at` (metres).
    pub(crate) fn settlement(&mut self, ty: &str, at: [f64; 2]) {
        self.features.add(Feature {
            kind: "settlement",
            ty: ty.to_owned(),
            geometry: Geometry::Point(at),
            props: BTreeMap::new(),
        });
    }

    /// The input of a render of this world with `cancel`, no options and no title.
    pub(crate) fn input<'a>(&'a self, cancel: &'a CancelToken) -> RenderInput<'a> {
        RenderInput {
            world: &self.world,
            features: &self.features,
            options: None,
            cancel,
            title: String::new(),
        }
    }

    /// Draws the render `name` with `cancel`.
    pub(crate) fn draw(&self, name: &str, cancel: &CancelToken) -> Result<Pixmap, RenderError> {
        let draw = render(name).unwrap();
        draw(&self.input(cancel))
    }

    /// The render `name` as PNG bytes.
    pub(crate) fn png(&self, name: &str) -> Result<Vec<u8>, RenderError> {
        encode_png(&self.draw(name, &CancelToken::new())?)
    }
}

/// A decoded 8-bit RGBA image.
pub(crate) struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    data: Vec<u8>,
}

impl Image {
    /// The pixel in column `x`, row `y`.
    pub(crate) fn rgba(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * usize::try_from(self.width).unwrap() + x) * 4;
        self.data[i..i + 4].try_into().unwrap()
    }

    /// The colour of the pixel in column `x`, row `y`.
    pub(crate) fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let [red, green, blue, _] = self.rgba(x, y);
        [red, green, blue]
    }
}

/// Decodes `png` (8-bit RGBA, as `encode_png` writes it).
pub(crate) fn decode(png: &[u8]) -> Image {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgba, png::BitDepth::Eight)
    );
    data.truncate(info.buffer_size());
    Image {
        width: info.width,
        height: info.height,
        data,
    }
}

/// The colour of the pixel in column `x`, row `y` of `png`.
pub(crate) fn pixel(png: &[u8], x: usize, y: usize) -> [u8; 3] {
    decode(png).rgb(x, y)
}
