//! Which paths no check reads: `gate/code-paths.toml` (`docs/specs/phases/P50-leaner-ci.md` §5.3).
//!
//! A path is non-code when it matches `non_code` and not `always_code`. Everything else is code,
//! so a new path counts as code until someone lists it. `non_code` matches exactly, `always_code`
//! ignores letter case, because the tool configs it names are found in any case.
//!
//! Errors never quote the file; a bad glob is named by its list and place in it.

use std::fs;
use std::path::Path;

use globset::GlobSet;
use serde::Deserialize;

use crate::checks::lints;
use crate::globs;
use crate::{Error, Result};

/// The lists file, relative to the root of the checkout the gate runs from.
pub const FILE: &str = "gate/code-paths.toml";

/// The two lists of `gate/code-paths.toml`.
#[derive(Debug)]
pub struct CodePaths {
    non_code: Vec<String>,
    always_code: Vec<String>,
    non_code_set: GlobSet,
    always_code_set: GlobSet,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lists {
    non_code: Vec<String>,
    always_code: Vec<String>,
}

impl CodePaths {
    /// Reads `gate/code-paths.toml` of the checkout at `root`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read, is not TOML of the expected shape or has a pattern that
    /// is not a glob.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(FILE);
        let text = fs::read_to_string(&path).map_err(|source| Error::Io { path, source })?;
        Self::parse(&text)
    }

    /// Reads the lists from the text of `gate/code-paths.toml`.
    ///
    /// # Errors
    ///
    /// The same as [`CodePaths::load`], but for reading the file.
    pub fn parse(text: &str) -> Result<Self> {
        // The TOML error would quote the line, so only its place is passed on.
        let table = lints::parse(FILE, text).map_err(|error| match error {
            Error::Parse { message, .. } => config_error(message),
            other => other,
        })?;
        let lists: Lists = table.try_into().map_err(|_| {
            config_error(
                "has an unexpected shape: `non_code` and `always_code` are lists of patterns",
            )
        })?;
        let set = |list: &str, patterns: &[String], ignore_case: bool| {
            globs::glob_set(list, patterns, ignore_case)
                .map_err(|error| globs::in_file(error, FILE))
        };
        Ok(Self {
            non_code_set: set("non_code", &lists.non_code, false)?,
            always_code_set: set("always_code", &lists.always_code, true)?,
            non_code: lists.non_code,
            always_code: lists.always_code,
        })
    }

    /// Whether no check reads `path` (repo-relative, with `/`): it matches `non_code` and not
    /// `always_code`. The lists file itself is always code, so a pull request cannot make its
    /// own change to the lists skip the tests that guard them.
    #[must_use]
    pub fn is_non_code(&self, path: &str) -> bool {
        path != FILE && self.non_code_set.is_match(path) && !self.always_code_set.is_match(path)
    }

    /// The `non_code` patterns, in file order.
    #[must_use]
    pub fn non_code(&self) -> &[String] {
        &self.non_code
    }

    /// The `always_code` patterns, in file order.
    #[must_use]
    pub fn always_code(&self) -> &[String] {
        &self.always_code
    }
}

fn config_error(message: impl Into<String>) -> Error {
    Error::Config {
        file: FILE.to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> Result<CodePaths> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
        CodePaths::load(root.unwrap_or_else(|| Path::new(".")))
    }

    // AC: CE-08C.3
    #[test]
    fn every_input_a_check_reads_is_code() -> Result<()> {
        let paths = real()?;
        for path in [
            "docs/specs/00-umbrella.md",
            "docs/gate.md",
            "CONTRIBUTING.md",
            ".github/workflows/ci.yml",
            ".github/pull_request_template.md",
            "research/app/src/x.ts",
            "docs/x/Cargo.toml",
            "research/Cargo.lock",
            "docs/CLIPPY.TOML",
            "docs/.clippy.toml",
            "docs/x/rustfmt.toml",
            "research/.rustfmt.toml",
            "research/rust-toolchain",
            "design/rust-toolchain.toml",
            "docs/Rust-Toolchain",
            "design/.cargo/config.toml",
            "docs/.gitignore",
            "research/.gitattributes",
            "crates/theos/src/main.rs",
            "xtask/src/main.rs",
            "gate/tools.toml",
            "gate/code-paths.toml",
            "Cargo.toml",
            "editor-ui/src/App.svelte",
        ] {
            assert!(!paths.is_non_code(path), "{path}");
        }
        Ok(())
    }

    #[test]
    fn docs_research_design_and_notes_are_non_code() -> Result<()> {
        let paths = real()?;
        for path in [
            "docs/status.md",
            "docs/specs/phases/P50-leaner-ci.md",
            "docs/plans/P50-leaner-ci.md",
            "research/notes.md",
            "design/notes/ideas.md",
            "README.md",
            "CLAUDE.md",
            ".claude/skills/theos-phase/SKILL.md",
            "docs/ui/screens/01-start.png",
        ] {
            assert!(paths.is_non_code(path), "{path}");
        }
        Ok(())
    }

    #[test]
    fn the_design_system_the_ui_imports_is_code() -> Result<()> {
        let paths = real()?;
        for path in [
            "design/theos-ui/tokens.css",
            "design/theos-ui/components/bundle.css",
            "design/theos-ui/fonts/inter.woff2",
            "design/theos-ui/README.md",
        ] {
            assert!(!paths.is_non_code(path), "{path}");
        }
        Ok(())
    }

    #[test]
    fn non_code_matches_exactly() -> Result<()> {
        let paths = real()?;
        assert!(!paths.is_non_code("Docs/status.md"));
        assert!(!paths.is_non_code("readme.md"));
        Ok(())
    }

    // AC: CE-08C.3
    #[test]
    fn a_ts_file_in_docs_is_code() -> Result<()> {
        assert!(!real()?.is_non_code("docs/x.ts"));
        Ok(())
    }

    // AC: CE-08C.3
    #[test]
    fn a_list_cannot_make_its_own_file_non_code() -> Result<()> {
        let paths = CodePaths::parse(
            "non_code = [\"gate/**\", \"**\"]
always_code = []",
        )?;
        assert!(paths.is_non_code("gate/tools.toml"));
        assert!(!paths.is_non_code(FILE));
        Ok(())
    }

    #[test]
    fn the_lists_are_kept_in_file_order() -> Result<()> {
        let paths = CodePaths::parse("non_code = [\"b/**\", \"a\"]\nalways_code = [\"z\", \"y\"]")?;
        assert_eq!(paths.non_code(), ["b/**", "a"]);
        assert_eq!(paths.always_code(), ["z", "y"]);
        Ok(())
    }

    #[test]
    fn a_bad_glob_is_named_by_its_list() {
        let message = |text: &str| match CodePaths::parse(text) {
            Err(Error::Config { file, message }) => {
                assert_eq!(file, FILE);
                message
            }
            other => panic!("expected a config error for {text:?}, got {other:?}"),
        };
        let bad = message("non_code = [\"ok\", \"SECRET[\"]\nalways_code = []");
        assert!(bad.contains("`non_code` pattern 2"), "{bad}");
        assert!(!bad.contains("SECRET"), "{bad}");
        let bad = message("non_code = []\nalways_code = [\"SECRET{a\"]");
        assert!(bad.contains("`always_code` pattern 1"), "{bad}");
        assert!(!bad.contains("SECRET"), "{bad}");
    }

    #[test]
    fn missing_keys_are_an_error() {
        for text in [
            "non_code = [\"a\"]",
            "always_code = [\"a\"]",
            "",
            "non_code = [\"a\"]\nalways_code = []\nother = 1",
            "non_code = \"a\"\nalways_code = []",
        ] {
            let result = CodePaths::parse(text);
            assert!(
                matches!(&result, Err(Error::Config { message, .. }) if message.contains("unexpected shape")),
                "{text:?}: {result:?}"
            );
        }
        let result = CodePaths::parse("this is not toml");
        assert!(
            matches!(&result, Err(Error::Config { message, .. }) if message.contains("near line 1")),
            "{result:?}"
        );
    }

    #[test]
    fn a_missing_file_is_an_io_error() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|source| Error::Io {
            path: "temporary directory".into(),
            source,
        })?;
        let result = CodePaths::load(dir.path());
        assert!(matches!(result, Err(Error::Io { .. })), "{result:?}");
        Ok(())
    }
}
