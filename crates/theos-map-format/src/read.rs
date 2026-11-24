//! The package reader: lenient towards what a later minor version adds, strict about integrity
//! (`docs/map-package.md` §6).

use std::collections::BTreeMap;
use std::io::{Read, Seek};

use serde_json::Value;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::manifest::FORMAT;
use crate::package::check_name;
use crate::write::{MANIFEST_FILE, layer_path};
use crate::{DType, FormatError, LayerData, LayerEntry, LayerKey, Manifest, Package, sha256_hex};

/// The capabilities this reader implements; format 1.0 defines none.
pub const IMPLEMENTED_CAPABILITIES: &[&str] = &[];

/// The major version this reader reads.
const SUPPORTED_MAJOR: u32 = 1;

/// The dtype names of [`DType`]; a layer with any other dtype is skipped.
const KNOWN_DTYPES: [&str; 3] = ["f32", "u8", "u16"];

/// Reads the bytes of the entry `name`, at most `limit` bytes.
///
/// Returns one byte more than `limit` when the entry is longer, so that the caller sees the
/// size is wrong without the whole entry being held in memory.
fn read_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, FormatError> {
    let file = archive.by_name(name).map_err(|e| match e {
        ZipError::FileNotFound => FormatError::MissingFile {
            name: name.to_owned(),
        },
        other => FormatError::Zip(other),
    })?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The largest manifest or feature file the reader accepts (1 GiB).
const MAX_JSON_BYTES: u64 = 1 << 30;

/// Checks `format`, `format_version` and `requires` of a manifest given as JSON.
fn check_header(manifest: &Value) -> Result<(), FormatError> {
    let text = |key: &str| {
        manifest
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    if text("format") != FORMAT {
        return Err(FormatError::NotAPackage {
            found: text("format").to_owned(),
        });
    }
    let version = text("format_version");
    let major = version
        .split('.')
        .next()
        .and_then(|m| m.parse::<u32>().ok());
    if major != Some(SUPPORTED_MAJOR) {
        return Err(FormatError::UnknownMajor {
            found: version.to_owned(),
            supported: "1.x",
        });
    }
    for capability in manifest
        .get("requires")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !IMPLEMENTED_CAPABILITIES.contains(&capability) {
            return Err(FormatError::UnsupportedCapability {
                capability: capability.to_owned(),
            });
        }
    }
    Ok(())
}

/// Drops the layer entries whose dtype this reader does not know.
fn drop_unknown_dtypes(manifest: &mut Value) {
    if let Some(layers) = manifest.get_mut("layers").and_then(Value::as_array_mut) {
        layers.retain(|entry| {
            entry
                .get("dtype")
                .and_then(Value::as_str)
                .is_none_or(|dtype| KNOWN_DTYPES.contains(&dtype))
        });
    }
}

/// The data of one indexed layer, checked against its entry.
fn read_layer<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    grid: [u32; 2],
    entry: &LayerEntry,
) -> Result<(LayerKey, LayerData), FormatError> {
    check_name("layer", &entry.name)?;
    if let Some(level) = &entry.level {
        check_name("level", level)?;
    }
    let key = LayerKey {
        level: entry.level.clone(),
        name: entry.name.clone(),
    };
    let expected = layer_path(&key, entry.dtype);
    if entry.file != expected {
        return Err(FormatError::LayerFile {
            name: entry.name.clone(),
            file: entry.file.clone(),
            expected,
        });
    }
    if entry.shape != grid {
        return Err(FormatError::LayerShape {
            name: entry.name.clone(),
        });
    }
    let cells = u64::from(entry.shape[0]) * u64::from(entry.shape[1]);
    let width = match entry.dtype {
        DType::F32 => 4,
        DType::U8 => 1,
        DType::U16 => 2,
    };
    let bytes = read_entry(archive, &entry.file, cells.saturating_mul(width))?;
    let cells = usize::try_from(cells).unwrap_or(usize::MAX);
    let data = LayerData::from_bytes(entry.dtype, &bytes, cells)?;
    if sha256_hex(&bytes) != entry.sha256 {
        return Err(FormatError::LayerHash {
            name: entry.name.clone(),
        });
    }
    Ok((key, data))
}

/// Reads a map package.
///
/// Refuses a package of another format or major version, one that requires a capability this
/// reader lacks, an index entry whose file, shape or name is wrong, a missing file, a layer whose
/// size or SHA-256 differs from the manifest, a float layer value that is not finite
/// ([`FormatError::NonFinite`], before the rules), and the rules of [`Package::validate`]. Ignores unknown manifest fields, entries the index does not list, layers
/// with an unknown dtype (they are left out of the result) and unknown feature kinds (kept as
/// they are).
///
/// # Errors
///
/// A [`FormatError`] naming the first refusal, or the JSON, ZIP or I/O error that stopped the
/// read.
pub fn read<R: Read + Seek>(input: R) -> Result<Package, FormatError> {
    let mut archive = ZipArchive::new(input)?;
    let manifest_bytes = read_entry(&mut archive, MANIFEST_FILE, MAX_JSON_BYTES)?;
    let mut manifest_json: Value = serde_json::from_slice(&manifest_bytes)?;
    check_header(&manifest_json)?;
    drop_unknown_dtypes(&mut manifest_json);
    let manifest: Manifest = serde_json::from_value(manifest_json)?;

    let features_bytes = read_entry(&mut archive, &manifest.features, MAX_JSON_BYTES)?;
    let features: geojson::FeatureCollection = serde_json::from_slice(&features_bytes)?;

    let mut layers = BTreeMap::new();
    for entry in &manifest.layers {
        let (key, data) = read_layer(&mut archive, manifest.frame.grid, entry)?;
        layers.insert(key, data);
    }
    let package = Package {
        manifest,
        features,
        layers,
    };
    package.validate()?;
    Ok(package)
}
