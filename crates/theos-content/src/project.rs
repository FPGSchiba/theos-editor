//! The project: `theos.toml` and the data model version this build reads.

use crate::text::{Position, TextIndex, normalize};
use serde::Deserialize;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use toml::Spanned;
use toml::Value;

/// The data model version this build reads.
pub const SCHEMA_VERSION: i64 = 1;

/// Why a project does not open.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// The folder has no `theos.toml`.
    #[error("`{}` is not a content project: it has no theos.toml", root.display())]
    NotAProject {
        /// The folder that was opened.
        root: PathBuf,
    },
    /// `theos.toml` could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file that failed.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// `theos.toml` is not valid TOML (or has a wrongly typed key we read).
    #[error("theos.toml: {message}")]
    Toml {
        /// The `toml` crate's message, with line and column.
        message: String,
    },
    /// `[project] schema_version` is missing.
    #[error(
        "theos.toml: [project] has no schema_version, but this build of theos reads schema_version {v}.",
        v = SCHEMA_VERSION
    )]
    NoSchemaVersion,
    /// `schema_version` is not an integer.
    #[error(
        "theos.toml:{position}: schema_version is {found}, but it must be the integer {v} for this build of theos (schema_version {v}).",
        v = SCHEMA_VERSION
    )]
    NotAnInteger {
        /// Line and column of the value.
        position: Position,
        /// The value as written in TOML.
        found: String,
    },
    /// The project's data model is newer than this build.
    #[error(
        "theos.toml:{position}: this project uses data model schema_version {found}, but this build of theos reads schema_version {v}. Update theos to open it.",
        v = SCHEMA_VERSION
    )]
    Newer {
        /// Line and column of the value.
        position: Position,
        /// The version found.
        found: i64,
    },
    /// The project's data model is older than this build.
    #[error(
        "theos.toml:{position}: this project uses data model schema_version {found}, but this build of theos reads only schema_version {v}.",
        v = SCHEMA_VERSION
    )]
    Older {
        /// Line and column of the value.
        position: Position,
        /// The version found.
        found: i64,
    },
}

#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    project: RawProject,
    #[serde(default)]
    parts: RawParts,
}

#[derive(Default, Deserialize)]
struct RawProject {
    name: Option<Value>,
    kind: Option<Value>,
    schema_version: Option<Spanned<Value>>,
}

#[derive(Default, Deserialize)]
struct RawParts {
    #[serde(default)]
    enabled: Vec<String>,
}

/// A content project: its root and what `theos.toml` says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    root: PathBuf,
    name: Option<String>,
    kind: Option<String>,
    enabled_parts: Vec<String>,
}

impl Project {
    /// Reads `<root>/theos.toml` and checks its `schema_version`.
    ///
    /// # Errors
    ///
    /// [`ProjectError`] when there is no `theos.toml`, it cannot be read or is not valid TOML,
    /// or `schema_version` is missing, not an integer, or not [`SCHEMA_VERSION`].
    pub fn open(root: &Path) -> Result<Project, ProjectError> {
        let path = root.join("theos.toml");
        let bytes = std::fs::read(&path).map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                ProjectError::NotAProject {
                    root: root.to_path_buf(),
                }
            } else {
                ProjectError::Io {
                    path: path.clone(),
                    source,
                }
            }
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let (text, _) = normalize(&text);
        let raw: RawFile = toml::from_str(&text).map_err(|e| ProjectError::Toml {
            message: e.to_string(),
        })?;
        let version = raw
            .project
            .schema_version
            .ok_or(ProjectError::NoSchemaVersion)?;
        let position = TextIndex::new(text.to_string())
            .position(version.span().start)
            .map_err(|e| ProjectError::Toml {
                message: e.to_string(),
            })?;
        let Value::Integer(found) = *version.get_ref() else {
            return Err(ProjectError::NotAnInteger {
                position,
                found: version.get_ref().to_string(),
            });
        };
        match found.cmp(&SCHEMA_VERSION) {
            std::cmp::Ordering::Greater => return Err(ProjectError::Newer { position, found }),
            std::cmp::Ordering::Less => return Err(ProjectError::Older { position, found }),
            std::cmp::Ordering::Equal => {}
        }
        let text_of = |v: Option<Value>| match v {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        Ok(Project {
            root: root.to_path_buf(),
            name: text_of(raw.project.name),
            kind: text_of(raw.project.kind),
            enabled_parts: raw.parts.enabled,
        })
    }

    /// The project folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `[project] name`, when it is a string.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// `[project] kind`, when it is a string.
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// `[parts] enabled`, in file order.
    #[must_use]
    pub fn enabled_parts(&self) -> &[String] {
        &self.enabled_parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const BASE: &str = "[project]\nname           = \"tower-of-theos\"\nkind           = \"content\"\nschema_version = 1\n\n[parts]\nenabled = [\"codex\", \"changes\", \"scripts\", \"graph\", \"concepts\", \"maps\", \"review\", \"mcp\"]\n";

    fn open_text(text: &str) -> Result<Project, ProjectError> {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("theos.toml"), text).expect("write");
        Project::open(dir.path())
    }

    fn with_version(v: &str) -> String {
        BASE.replace("schema_version = 1", &format!("schema_version = {v}"))
    }

    fn message(r: Result<Project, ProjectError>) -> String {
        r.expect_err("must fail").to_string()
    }

    // AC: CE-11.3
    #[test]
    fn schema_version_mismatch_is_clear() {
        assert_eq!(
            message(open_text(&with_version("2"))),
            "theos.toml:4:18: this project uses data model schema_version 2, but this build of theos reads schema_version 1. Update theos to open it."
        );
        assert_eq!(
            message(open_text(&with_version("0"))),
            "theos.toml:4:18: this project uses data model schema_version 0, but this build of theos reads only schema_version 1."
        );
    }

    // AC: CE-11.3
    #[test]
    fn schema_version_must_be_an_integer_and_present() {
        let err = open_text(&with_version("\"1\"")).expect_err("string");
        assert!(matches!(err, ProjectError::NotAnInteger { .. }));
        let m = err.to_string();
        assert!(
            m.contains("theos.toml:4:18") && m.contains("\"1\"") && m.contains("schema_version 1"),
            "{m}"
        );
        let err = open_text(&BASE.replace("schema_version = 1\n", "")).expect_err("missing");
        assert!(matches!(err, ProjectError::NoSchemaVersion));
        let m = err.to_string();
        assert!(
            m.contains("theos.toml") && m.contains("schema_version 1"),
            "{m}"
        );
    }

    // AC: CE-11.3
    #[test]
    fn version_one_opens() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("theos.toml"), BASE).expect("write");
        let p = Project::open(dir.path()).expect("opens");
        assert_eq!(p.root(), dir.path());
        assert_eq!(p.name(), Some("tower-of-theos"));
        assert_eq!(p.kind(), Some("content"));
        assert_eq!(
            p.enabled_parts(),
            [
                "codex", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp"
            ]
        );
    }

    #[test]
    fn not_a_project_without_theos_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Project::open(dir.path()).expect_err("no file");
        assert_eq!(
            err.to_string(),
            format!(
                "`{}` is not a content project: it has no theos.toml",
                dir.path().display()
            )
        );
    }

    #[test]
    fn invalid_toml_reports_where() {
        let err = open_text("[project\n").expect_err("bad toml");
        assert!(matches!(err, ProjectError::Toml { .. }));
        assert!(err.to_string().contains("line 1"), "{err}");
    }

    #[test]
    fn crlf_theos_toml_reports_the_same_position() {
        let text = with_version("2").replace('\n', "\r\n");
        assert!(message(open_text(&text)).starts_with("theos.toml:4:18: "));
    }

    #[test]
    fn io_error_names_the_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("theos.toml")).expect("dir");
        let err = Project::open(dir.path()).expect_err("directory");
        assert!(matches!(err, ProjectError::Io { .. }));
        assert!(err.to_string().contains("theos.toml"));
    }

    #[test]
    fn wrong_types_of_name_and_kind_are_ignored() {
        let p = open_text("[project]\nname = 3\nkind = true\nschema_version = 1\n").expect("opens");
        assert_eq!(
            (p.name(), p.kind(), p.enabled_parts().len()),
            (None, None, 0)
        );
    }
}
