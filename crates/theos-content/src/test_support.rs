//! Helpers shared by unit tests.

use crate::index::Index;
use crate::project::Project;
use crate::record::Record;

/// The `theos.toml` of data model §13.
const THEOS_TOML: &str = "[project]\nname = \"synthetic\"\nkind = \"content\"\nschema_version = 1\n\n[parts]\nenabled = [\"codex\", \"changes\", \"scripts\", \"graph\", \"concepts\", \"maps\", \"review\", \"mcp\"]\n";

/// A project in a temporary folder with `files` (path, text) and a `theos.toml`, indexed in
/// memory with the builtin kinds. The folder lives as long as the returned guard.
pub(crate) fn indexed(
    files: &[(&str, &str)],
) -> Result<(tempfile::TempDir, Index), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("theos.toml"), THEOS_TOML)?;
    for (path, text) in files {
        let file = dir.path().join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, text)?;
    }
    let mut index = Index::in_memory(&Project::open(dir.path())?, &crate::parts::builtin()?)?;
    index.sync()?;
    Ok((dir, index))
}

/// Indexes `files` as [`indexed`] does and runs `f` on a lore context over the store: the one
/// the lore step builds, which fetches facts by id.
pub(crate) fn with_context<R>(
    files: &[(&str, &str)],
    f: impl FnOnce(&mut crate::lore::Context<'_>) -> R,
) -> Result<R, Box<dyn std::error::Error>> {
    let (_dir, index) = indexed(files)?;
    let mut ctx = index.lore_context();
    Ok(f(&mut ctx))
}

/// Parses `---\n{front}---\n{body}`; panics on error (tests only).
pub(crate) fn record(front: &str, body: &str) -> Record {
    match Record::parse(&format!("---\n{front}---\n{body}")) {
        Ok(record) => record,
        Err(error) => panic!("test record does not parse: {error}"),
    }
}

/// The rule name and `line:column` of the first front-matter error of `front` (tests only).
pub(crate) fn front_error(front: &str) -> Option<(&'static str, String)> {
    record(front, "")
        .front_matter_error()
        .map(|error| (error.rule.as_str(), error.position.to_string()))
}
