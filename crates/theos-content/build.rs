//! Fingerprints the sources of this crate so the index cache of a developer build is rebuilt
//! whenever they change; a stale cache would serve findings of older rules.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Collects every file below `dir` with its path relative to `root`, written with `/`.
fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, out)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(std::io::Error::other)?
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push((relative, path));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("src");
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, path) in &files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(std::fs::read(path)?);
        hasher.update([0]);
    }
    let mut hex = String::new();
    for byte in hasher.finalize() {
        write!(hex, "{byte:02x}")?;
    }
    let mut out = std::io::stdout().lock();
    writeln!(out, "cargo::rerun-if-changed=src")?;
    writeln!(out, "cargo::rustc-env=THEOS_CONTENT_SOURCE_HASH={hex}")?;
    Ok(())
}
