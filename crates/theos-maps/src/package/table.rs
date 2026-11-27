//! The layers a terrain package holds, in order, and where each one's values come from
//! (spec §5.10, `docs/map-package.md` §4.4).

use theos_map_format::DType;

/// Where the values of a package layer come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// The world layer of the same name.
    Raster,
    /// The classes of `river`, `big_river` and `lake` (ruling L19).
    Water,
    /// `climate.fields.precipitation`, computed from the height when the climate holds no
    /// fields (ruling P2).
    Precipitation,
}

/// A layer of a terrain package: how it is stored and described in the manifest.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PackageLayer {
    /// The layer name, in the package and in the world.
    pub name: &'static str,
    /// The stored sample type.
    pub dtype: DType,
    /// The units of the values.
    pub units: Option<&'static str>,
    /// The multiplier of an integer layer that holds a unit-range density.
    pub scale: Option<f64>,
    /// The meaning of each value of a categorical layer, as `(value, meaning)`.
    pub legend: Option<&'static [(&'static str, &'static str)]>,
    /// What the layer holds.
    pub description: &'static str,
    /// Where the values come from.
    pub source: Source,
}

/// The `scale` of a unit-range density stored as a byte.
pub(crate) const UNIT_SCALE: f64 = 1.0 / 255.0;

const fn float(
    name: &'static str,
    units: &'static str,
    description: &'static str,
    source: Source,
) -> PackageLayer {
    PackageLayer {
        name,
        dtype: DType::F32,
        units: Some(units),
        scale: None,
        legend: None,
        description,
        source,
    }
}

const fn density(name: &'static str, description: &'static str) -> PackageLayer {
    PackageLayer {
        name,
        dtype: DType::U8,
        units: None,
        scale: Some(UNIT_SCALE),
        legend: None,
        description,
        source: Source::Raster,
    }
}

const fn classes(
    name: &'static str,
    legend: &'static [(&'static str, &'static str)],
    description: &'static str,
    source: Source,
) -> PackageLayer {
    PackageLayer {
        name,
        dtype: DType::U8,
        units: None,
        scale: None,
        legend: Some(legend),
        description,
        source,
    }
}

/// The legend of `flow_dir` (ruling L19): the neighbour codes run N, NE, E, SE, S, SW, W, NW.
const FLOW_DIR_LEGEND: &[(&str, &str)] = &[
    ("0", "outlet or terminal lake bottom"),
    ("1", "north"),
    ("2", "north-east"),
    ("3", "east"),
    ("4", "south-east"),
    ("5", "south"),
    ("6", "south-west"),
    ("7", "west"),
    ("8", "north-west"),
];

/// The legend of `water` (ruling L19).
const WATER_LEGEND: &[(&str, &str)] =
    &[("0", "dry"), ("1", "stream"), ("2", "river"), ("3", "lake")];

/// The legend of `rock`: the names of `Rock::ALL` by code.
pub(crate) const ROCK_LEGEND: &[(&str, &str)] = &[
    ("0", "granite"),
    ("1", "quartzite"),
    ("2", "basalt"),
    ("3", "limestone"),
    ("4", "sandstone"),
    ("5", "shale"),
    ("6", "alluvium"),
];

/// The legend of `roads`: the codes of `RoadClass`.
pub(crate) const ROADS_LEGEND: &[(&str, &str)] = &[
    ("0", "none"),
    ("1", "trail"),
    ("2", "track"),
    ("3", "road"),
    ("4", "main"),
];

/// The terrain layers in package order; `omega` follows them when the world holds it.
pub(crate) const TERRAIN_LAYERS: &[PackageLayer] = &[
    float("height", "m", "Terrain height", Source::Raster),
    float(
        "water_surface",
        "m",
        "The lake level where there is a lake, else the ground height",
        Source::Raster,
    ),
    classes(
        "flow_dir",
        FLOW_DIR_LEGEND,
        "D8 code of the steepest downhill neighbour on the depression-filled surface",
        Source::Raster,
    ),
    float(
        "drainage_area",
        "m2",
        "Upstream area draining through the cell over multiple flow directions",
        Source::Raster,
    ),
    float(
        "erosion",
        "m",
        "Thickness removed by all processes over the run",
        Source::Raster,
    ),
    float(
        "deposition",
        "m",
        "Thickness added by all processes over the run",
        Source::Raster,
    ),
    float("discharge", "m3/s", "River discharge", Source::Raster),
    float(
        "precipitation",
        "mm/yr",
        "Annual precipitation",
        Source::Precipitation,
    ),
    float(
        "temperature",
        "degC",
        "Mean annual temperature",
        Source::Raster,
    ),
    classes("water", WATER_LEGEND, "Water class", Source::Water),
    classes("rock", ROCK_LEGEND, "Bedrock class", Source::Raster),
    density("vegetation", "Vegetation density"),
    density("snow", "Snow cover density"),
    classes("roads", ROADS_LEGEND, "Road class", Source::Raster),
    density("element_water", "Ambient water element affinity"),
    density("element_wood", "Ambient wood element affinity"),
    density("element_earth", "Ambient earth element affinity"),
    density("element_metal", "Ambient metal element affinity"),
    density("element_fire", "Ambient fire element affinity"),
];

/// The layer `omega`, written only when the recipe ran the detuning stage.
pub(crate) const OMEGA_LAYER: PackageLayer = density("omega", "Detuning omega");
