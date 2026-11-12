//! The error type of the crate.

/// Everything that can go wrong while reading or writing a map package.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// A JSON value could not be written or read.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A float layer holds NaN or an infinity (`docs/map-package.md` §4.2).
    #[error("layer value at index {index} is not finite (NaN and infinities are not allowed)")]
    NonFinite {
        /// Index of the first offending value.
        index: usize,
    },
    /// A layer file does not have `cells · size of dtype` bytes.
    #[error("layer has {found} bytes, expected {expected}")]
    LayerSize {
        /// The byte count the shape asks for.
        expected: usize,
        /// The byte count found.
        found: usize,
    },
    /// A layer entry has no data in the package.
    #[error("layer {layer} has an entry in the manifest but no data")]
    MissingData {
        /// The layer, as `name` or `level/name`.
        layer: String,
    },
    /// A layer has data but no entry in the manifest.
    #[error("layer {layer} has data but no entry in the manifest")]
    MissingEntry {
        /// The layer, as `name` or `level/name`.
        layer: String,
    },
    /// A layer does not hold rows x columns values.
    #[error("layer {layer} has {found} values, the grid needs {expected}")]
    LayerLength {
        /// The layer, as `name` or `level/name`.
        layer: String,
        /// Rows x columns.
        expected: usize,
        /// The number of values found.
        found: usize,
    },
    /// Two layer entries share a name within one level.
    #[error("layer {layer} is listed more than once")]
    DuplicateLayer {
        /// The layer, as `name` or `level/name`.
        layer: String,
    },
    /// A float layer holds NaN or an infinity.
    #[error("layer {layer} has a value at index {index} that is not finite")]
    LayerNotFinite {
        /// The layer, as `name` or `level/name`.
        layer: String,
        /// Index of the first offending value.
        index: usize,
    },
    /// A number of the manifest's frame is NaN or an infinity.
    #[error("manifest field {field} is not finite")]
    ManifestNotFinite {
        /// The field, such as `frame.cell_m`.
        field: String,
    },
    /// A map with levels has a layer or feature without a level.
    #[error("{owner} has no level, but the frame lists levels")]
    LevelMissing {
        /// What lacks the level, such as `layer height` or `feature 2`.
        owner: String,
    },
    /// A layer or feature names a level the frame does not list.
    #[error("{owner} is on level {level}, which the frame does not list")]
    LevelUnknown {
        /// What names the level.
        owner: String,
        /// The level it names.
        level: String,
    },
    /// A map without levels has a layer or feature that carries one.
    #[error("{owner} carries level {level}, but the frame has no levels")]
    LevelUnexpected {
        /// What carries the level.
        owner: String,
        /// The level it carries.
        level: String,
    },
    /// The manifest's `format` is not `theos-map`.
    #[error("not a map package: format is {found:?}, expected \"theos-map\"")]
    NotAPackage {
        /// The `format` value found.
        found: String,
    },
    /// The manifest's major version is not one this reader reads.
    #[error("format_version {found} is not supported; this reader reads {supported}")]
    UnknownMajor {
        /// The `format_version` found.
        found: String,
        /// The versions this reader reads, such as `1.x`.
        supported: &'static str,
    },
    /// The package requires a capability this reader does not implement.
    #[error("the package requires `{capability}`, which this reader does not implement")]
    UnsupportedCapability {
        /// The capability named in `requires`.
        capability: String,
    },
    /// A layer file does not match the SHA-256 of its index entry.
    #[error("layer {name} does not match the SHA-256 in the manifest")]
    LayerHash {
        /// The layer's name.
        name: String,
    },
    /// A layer index entry names a file other than the one its name, dtype and level give.
    #[error("layer {name} lists the file {file:?}, expected {expected:?}")]
    LayerFile {
        /// The layer's name.
        name: String,
        /// The file the index lists.
        file: String,
        /// The file the format expects.
        expected: String,
    },
    /// A layer index entry's shape differs from the frame's grid.
    #[error("layer {name} has a shape that is not the frame's grid")]
    LayerShape {
        /// The layer's name.
        name: String,
    },
    /// The package lacks a file it needs.
    #[error("the package has no file {name}")]
    MissingFile {
        /// The file's name.
        name: String,
    },
    /// A layer name or level id cannot be part of an entry path.
    #[error(
        "{kind} name {name:?} is not allowed: it must not be empty, `.` or `..`, or contain `/`, `\\` or NUL"
    )]
    InvalidName {
        /// `layer` or `level`.
        kind: &'static str,
        /// The offending name.
        name: String,
    },
    /// The caller asked to stop while a package was written.
    #[error("writing the package was cancelled")]
    Cancelled,
    /// The ZIP container could not be written.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// The output could not be written.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
