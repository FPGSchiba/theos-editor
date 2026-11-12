//! The package writer: a ZIP with fixed headers, so equal packages give equal bytes
//! (`docs/map-package.md` §5).

use std::io::{Seek, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, System, ZipWriter};

use crate::manifest::{FORMAT, FORMAT_VERSION};
use crate::package::label;
use crate::{DType, FormatError, LayerKey, Manifest, Package, json, sha256_hex};

/// Name of the manifest entry.
const MANIFEST_FILE: &str = "manifest.json";
/// Name of the feature entry.
const FEATURES_FILE: &str = "features.geojson";

/// True when an entry of `len` bytes needs ZIP64 fields (above 4 GiB, `docs/map-package.md` §8).
pub(crate) fn needs_zip64(len: u64) -> bool {
    len > u64::from(u32::MAX)
}

fn extension(dtype: DType) -> &'static str {
    match dtype {
        DType::F32 => "f32",
        DType::U8 => "u8",
        DType::U16 => "u16",
    }
}

fn layer_path(key: &LayerKey, dtype: DType) -> String {
    let ext = extension(dtype);
    match &key.level {
        Some(level) => format!("layers/{level}/{}.{ext}", key.name),
        None => format!("layers/{}.{ext}", key.name),
    }
}

/// A file of the package: its name and bytes.
type PackageFile = (String, Vec<u8>);

/// The manifest with everything the writer owns filled in, and the bytes of each layer file.
fn complete(package: &Package) -> Result<(Manifest, Vec<PackageFile>), FormatError> {
    package.validate()?;
    let mut manifest = package.manifest.clone();
    FORMAT.clone_into(&mut manifest.format);
    FORMAT_VERSION.clone_into(&mut manifest.format_version);
    FEATURES_FILE.clone_into(&mut manifest.features);
    manifest.requires.sort();
    manifest.requires.dedup();
    let grid = manifest.frame.grid;
    let mut files = Vec::with_capacity(manifest.layers.len());
    for entry in &mut manifest.layers {
        let key = LayerKey {
            level: entry.level.clone(),
            name: entry.name.clone(),
        };
        let data = package
            .layers
            .get(&key)
            .ok_or_else(|| FormatError::MissingData { layer: label(&key) })?;
        let bytes = data.to_bytes()?;
        entry.dtype = data.dtype();
        entry.file = layer_path(&key, entry.dtype);
        entry.shape = grid;
        entry.sha256 = sha256_hex(&bytes);
        let range = data.decoded_range(entry.scale, entry.offset);
        entry.min = range.map(|(lo, _)| lo);
        entry.max = range.map(|(_, hi)| hi);
        files.push((entry.file.clone(), bytes));
    }
    Ok((manifest, files))
}

/// The manifest as [`fn@write`] stores it: `format`, `format_version`, `features` and `requires`
/// set, and every layer entry completed with its file, dtype, shape, hash and range.
///
/// # Errors
///
/// The errors of [`Package::validate`].
pub fn written_manifest(package: &Package) -> Result<Manifest, FormatError> {
    complete(package).map(|(manifest, _)| manifest)
}

/// Writes `package` as a map package ZIP to `out` and returns `out`.
///
/// # Errors
///
/// The errors of [`written_manifest`], and [`FormatError::Zip`] or [`FormatError::Io`] when the
/// output fails.
pub fn write<W: Write + Seek>(package: &Package, out: W) -> Result<W, FormatError> {
    write_cancellable(package, out, &|| false)
}

/// Like [`fn@write`], and asks `cancelled` before each entry.
///
/// # Errors
///
/// [`FormatError::Cancelled`] when `cancelled` returns true, and the errors of [`fn@write`].
pub fn write_cancellable<W: Write + Seek>(
    package: &Package,
    out: W,
    cancelled: &dyn Fn() -> bool,
) -> Result<W, FormatError> {
    let (manifest, layer_files) = complete(package)?;
    let mut entries = vec![
        (
            MANIFEST_FILE.to_owned(),
            json::to_canonical_vec(&serde_json::to_value(&manifest)?)?,
        ),
        (
            FEATURES_FILE.to_owned(),
            json::to_canonical_vec(&serde_json::to_value(&package.features)?)?,
        ),
    ];
    entries.extend(layer_files);

    let mut zip = ZipWriter::new(out);
    for (name, bytes) in entries {
        if cancelled() {
            return Err(FormatError::Cancelled);
        }
        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6))
            .last_modified_time(DateTime::DEFAULT)
            .system(System::Unix)
            .unix_permissions(0o644)
            .large_file(needs_zip64(len));
        zip.start_file(name, options)?;
        zip.write_all(&bytes)?;
    }
    Ok(zip.finish()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip64_is_used_only_above_4_gib() {
        assert!(!needs_zip64(u64::from(u32::MAX)));
        assert!(needs_zip64(u64::from(u32::MAX) + 1));
    }

    #[test]
    fn layer_paths_follow_dtype_and_level() {
        let flat = LayerKey {
            level: None,
            name: "height".to_owned(),
        };
        assert_eq!(layer_path(&flat, DType::F32), "layers/height.f32");
        let deep = LayerKey {
            level: Some("f1".to_owned()),
            name: "walls".to_owned(),
        };
        assert_eq!(layer_path(&deep, DType::U16), "layers/f1/walls.u16");
        assert_eq!(layer_path(&deep, DType::U8), "layers/f1/walls.u8");
    }
}
