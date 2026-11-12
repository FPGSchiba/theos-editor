//! Helpers of the studio tests, each in one place: a temporary content project and its files.

use std::collections::BTreeMap;
use std::path::Path;

/// The error of a test or a test helper.
pub type Error = Box<dyn std::error::Error>;

/// The result of a test.
pub type TestResult = Result<(), Error>;

/// The files of a project: their bytes by path with `/` separators.
pub type Files = BTreeMap<String, Vec<u8>>;

/// A content project in a temporary folder.
pub struct TempProject {
    dir: tempfile::TempDir,
}

impl TempProject {
    /// A project in a temporary folder with the files of the fixture project `name` of
    /// `theos-content`.
    pub fn copy_of(name: &str) -> Result<TempProject, Error> {
        TempProject::copied(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("theos-content")
                .join("tests")
                .join("fixtures")
                .join("projects")
                .join(name),
        )
    }

    /// A project in a temporary folder with the files of the P06 fixture project `name` of
    /// `theos-studio` (`tests/fixtures/<name>`).
    pub fn copy_of_studio(name: &str) -> Result<TempProject, Error> {
        TempProject::copied(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join(name),
        )
    }

    /// A project in a temporary folder with the files under `from`, `.theos/cache` left out.
    fn copied(from: &Path) -> Result<TempProject, Error> {
        let dir = tempfile::tempdir()?;
        for (path, bytes) in files(from)? {
            put(dir.path(), &path, &bytes)?;
        }
        Ok(TempProject { dir })
    }

    /// The project folder.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }
}

/// Every file under `root` except `.theos/cache`, by path with `/` separators.
pub fn files(root: &Path) -> std::io::Result<Files> {
    let mut found = BTreeMap::new();
    collect(root, root, &mut found)?;
    Ok(found)
}

fn collect(root: &Path, dir: &Path, found: &mut Files) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel: Vec<String> = path
            .strip_prefix(root)
            .map_err(std::io::Error::other)?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if rel.join("/") == ".theos/cache" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect(root, &path, found)?;
        } else {
            found.insert(rel.join("/"), std::fs::read(&path)?);
        }
    }
    Ok(())
}

/// Writes the file `path` (with `/`) under `root` with `bytes`, and the folders above it.
pub fn put(root: &Path, path: &str, bytes: &[u8]) -> std::io::Result<()> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, bytes)
}
