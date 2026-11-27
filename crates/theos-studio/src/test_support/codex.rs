//! The `codex` fixture: a synthetic project every P48 test reads, copied to a temporary folder.

use std::path::Path;

use tempfile::TempDir;

use crate::Workspace;

/// Copies `from` to `to`, folders and files.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The `codex` fixture copied to a temporary folder, opened. The folder is dropped with the
/// `TempDir`, so the cache never lands in the repository.
pub(crate) fn codex_workspace() -> Result<(TempDir, Workspace), Box<dyn std::error::Error>> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/projects/codex");
    let dir = tempfile::tempdir()?;
    copy_tree(&source, dir.path())?;
    let workspace = Workspace::open(dir.path())?;
    Ok((dir, workspace))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{TestResult, codex_workspace};

    // AC: CE-18.2
    #[test]
    fn codex_fixture_indexes_as_expected() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let index = workspace.index();
        let lens = index
            .lens("mother-ilse")?
            .ok_or("mother-ilse has no lens")?;
        let chain: Vec<&str> = lens.chain.iter().map(|layer| layer.id.as_str()).collect();
        assert_eq!(chain, ["mother-ilse", "fates-cult", "hearth-folk"]);
        let beliefs: Vec<(&str, &str)> = lens
            .beliefs
            .iter()
            .map(|held| (held.belief.as_str(), held.through.id.as_str()))
            .collect();
        assert_eq!(beliefs, [("B-912", "mother-ilse"), ("B-911", "fates-cult")]);
        let findings: Vec<String> = index
            .findings()?
            .iter()
            .map(|finding| format!("{} {}", finding.rule, finding.path))
            .collect();
        // Exactly these: the id twice, and the unresolved mention. Every other record is clean.
        assert_eq!(
            findings,
            [
                "link/dangling story/scripts/S-004.md",
                "index/duplicate-id things/dup-911.md",
                "index/duplicate-id world/synthetic-band/f04-synthetic/dup-911.md",
            ]
        );
        Ok(())
    }
}
