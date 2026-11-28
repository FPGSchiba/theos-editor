//! The error every command returns, as the UI sees it.

use std::path::Path;

use serde::Serialize;
use theos_content::{IndexError, ProjectError, ReadError};
use theos_studio::StudioError;

/// Why a command failed. It is serialised flat as `{ "kind": …, "message": … }`: `kind` is the
/// variant in `snake_case` and `message` its text, which names the path or id. The UI shows the
/// message, never a stack.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(into = "wire::IpcError")]
pub enum IpcError {
    /// The command needs an open project, and none is open.
    #[error("no project is open")]
    NoProject,
    /// The folder is not a content project, or its `theos.toml` cannot be read.
    #[error("{message}")]
    NotAProject {
        /// The folder.
        path: String,
        /// What is wrong and what to do, as the studio says it, naming the folder.
        message: String,
    },
    /// No record has the id.
    #[error("no record has the id `{id}`")]
    NotFound {
        /// The id asked for.
        id: String,
    },
    /// The index failed.
    #[error("{message}")]
    Index {
        /// The studio's message.
        message: String,
    },
    /// A file or folder could not be read or written.
    #[error("{path}: {message}")]
    Io {
        /// The file or folder.
        path: String,
        /// What went wrong.
        message: String,
    },
}

/// The shape the UI receives, which the bindings type.
pub mod wire {
    use serde::Serialize;

    /// A failed command: what kind of failure, and the text to show.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
    pub struct IpcError {
        /// What kind of failure.
        pub kind: IpcErrorKind,
        /// The text to show, naming the path or id.
        pub message: String,
    }

    /// The kinds of [`IpcError`], one per variant of [`crate::IpcError`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
    #[serde(rename_all = "snake_case")]
    pub enum IpcErrorKind {
        /// See [`crate::IpcError::NoProject`].
        NoProject,
        /// See [`crate::IpcError::NotAProject`].
        NotAProject,
        /// See [`crate::IpcError::NotFound`].
        NotFound,
        /// See [`crate::IpcError::Index`].
        Index,
        /// See [`crate::IpcError::Io`].
        Io,
    }
}

impl IpcError {
    /// The kind of the error.
    #[must_use]
    pub fn kind(&self) -> wire::IpcErrorKind {
        match self {
            IpcError::NoProject => wire::IpcErrorKind::NoProject,
            IpcError::NotAProject { .. } => wire::IpcErrorKind::NotAProject,
            IpcError::NotFound { .. } => wire::IpcErrorKind::NotFound,
            IpcError::Index { .. } => wire::IpcErrorKind::Index,
            IpcError::Io { .. } => wire::IpcErrorKind::Io,
        }
    }

    /// The error of opening the project in `path`.
    #[must_use]
    pub fn opening(path: &Path, error: StudioError) -> IpcError {
        match error {
            StudioError::Project(error) => IpcError::not_a_project(path, &error),
            other => IpcError::from(other),
        }
    }

    fn not_a_project(path: &Path, error: &ProjectError) -> IpcError {
        match error {
            ProjectError::Io { path, source } => IpcError::Io {
                path: path.display().to_string(),
                message: source.to_string(),
            },
            other => IpcError::NotAProject {
                path: path.display().to_string(),
                message: naming(path, &other.to_string()),
            },
        }
    }
}

/// `message`, prefixed with `path` unless it names it already.
fn naming(path: &Path, message: &str) -> String {
    let path = path.display().to_string();
    if message.contains(&path) {
        message.to_owned()
    } else {
        format!("{path}: {message}")
    }
}

/// The bindings type an error as what it serialises to, [`wire::IpcError`], under that one name.
impl specta::Type for IpcError {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        wire::IpcError::definition(types)
    }
}

impl From<IpcError> for wire::IpcError {
    fn from(error: IpcError) -> wire::IpcError {
        wire::IpcError {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

impl From<StudioError> for IpcError {
    fn from(error: StudioError) -> IpcError {
        match error {
            StudioError::Io { path, source } => IpcError::Io {
                path: path.display().to_string(),
                message: source.to_string(),
            },
            StudioError::Read { path, source } => IpcError::Io {
                path: path.display().to_string(),
                message: match source {
                    // The path is the file's already; its own message would name it twice.
                    ReadError::Io { source, .. } => source.to_string(),
                    other => other.to_string(),
                },
            },
            ref refused @ StudioError::NotAProject { ref start } => IpcError::NotAProject {
                message: refused.to_string(),
                path: start.display().to_string(),
            },
            other => IpcError::Index {
                message: other.to_string(),
            },
        }
    }
}

impl From<IndexError> for IpcError {
    fn from(error: IndexError) -> IpcError {
        IpcError::Index {
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn every_variant() -> [IpcError; 5] {
        [
            IpcError::NoProject,
            IpcError::NotAProject {
                path: "synthetic".to_owned(),
                message: "`synthetic` is not a content project: it has no theos.toml".to_owned(),
            },
            IpcError::NotFound {
                id: "f04-choir-hall".to_owned(),
            },
            IpcError::Index {
                message: "synthetic index failure".to_owned(),
            },
            IpcError::Io {
                path: "synthetic.md".to_owned(),
                message: "synthetic failure".to_owned(),
            },
        ]
    }

    #[test]
    fn serialised_flat_with_kind_and_message() -> Result<(), serde_json::Error> {
        let error = IpcError::NotFound {
            id: "f04-choir-hall".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&error)?,
            serde_json::json!({
                "kind": "not_found",
                "message": "no record has the id `f04-choir-hall`"
            })
        );
        assert_eq!(
            serde_json::to_value(IpcError::NoProject)?,
            serde_json::json!({ "kind": "no_project", "message": "no project is open" })
        );
        Ok(())
    }

    #[test]
    fn every_variant_has_a_kind_and_a_non_empty_message() -> Result<(), serde_json::Error> {
        let kinds = ["no_project", "not_a_project", "not_found", "index", "io"];
        for (error, kind) in every_variant().into_iter().zip(kinds) {
            let value = serde_json::to_value(&error)?;
            let object = value.as_object().map(serde_json::Map::len);
            assert_eq!(object, Some(2), "{value}");
            assert_eq!(value["kind"], kind, "{value}");
            let message = value["message"].as_str().unwrap_or_default();
            assert!(!message.is_empty(), "{value}");
        }
        Ok(())
    }

    #[test]
    fn the_bindings_type_is_the_flat_shape() {
        use specta::Type;
        let mut types = specta::Types::default();
        let error = IpcError::definition(&mut types);
        assert_eq!(error, wire::IpcError::definition(&mut types));
        let names: Vec<_> = types
            .into_sorted_iter()
            .map(|named| named.name.to_string())
            .collect();
        let count = |name: &str| names.iter().filter(|n| *n == name).count();
        assert_eq!(
            (count("IpcError"), count("IpcErrorKind")),
            (1, 1),
            "{names:?}"
        );
    }

    #[test]
    fn a_message_without_the_folder_gets_it() {
        let path = PathBuf::from("synthetic");
        let error = IpcError::opening(&path, StudioError::Project(ProjectError::NoSchemaVersion));
        assert_eq!(error.kind(), wire::IpcErrorKind::NotAProject);
        assert!(
            error.to_string().starts_with("synthetic: theos.toml:"),
            "{error}"
        );
    }

    #[test]
    fn a_missing_theos_toml_is_not_a_project() {
        let path = PathBuf::from("synthetic");
        let error = IpcError::opening(
            &path,
            StudioError::Project(ProjectError::NotAProject { root: path.clone() }),
        );
        assert!(
            matches!(&error, IpcError::NotAProject { message, .. } if message.contains("has no theos.toml")),
            "{error:?}"
        );
    }

    #[test]
    fn io_errors_keep_their_path() {
        let error = IpcError::from(StudioError::Io {
            path: PathBuf::from("synthetic.md"),
            source: std::io::Error::other("synthetic failure"),
        });
        assert_eq!(error.to_string(), "synthetic.md: synthetic failure");
        let error = IpcError::opening(
            Path::new("synthetic"),
            StudioError::Project(ProjectError::Io {
                path: PathBuf::from("synthetic/theos.toml"),
                source: std::io::Error::other("synthetic failure"),
            }),
        );
        assert!(matches!(error, IpcError::Io { .. }), "{error:?}");
    }

    #[test]
    fn a_folder_outside_any_project_is_not_a_project() {
        let error = IpcError::from(StudioError::NotAProject {
            start: PathBuf::from("synthetic"),
        });
        assert!(matches!(error, IpcError::NotAProject { .. }), "{error:?}");
    }

    #[test]
    fn a_record_that_cannot_be_read_is_an_io_error_naming_its_file_once() {
        let file = PathBuf::from("synthetic.md");
        let error = IpcError::from(StudioError::Read {
            path: file.clone(),
            source: ReadError::Io {
                path: file.clone(),
                source: std::io::Error::other("synthetic failure"),
            },
        });
        assert_eq!(
            error,
            IpcError::Io {
                path: "synthetic.md".to_owned(),
                message: "synthetic failure".to_owned(),
            }
        );
        let error = IpcError::from(StudioError::Read {
            path: file,
            source: ReadError::Utf8 {
                position: theos_content::Position { line: 1, column: 2 },
            },
        });
        assert!(
            matches!(&error, IpcError::Io { message, .. } if message.contains("invalid UTF-8")),
            "{error:?}"
        );
    }

    #[test]
    fn opening_fails_as_the_studio_error_says_when_it_is_not_the_project() {
        let error = IpcError::opening(
            Path::new("synthetic"),
            StudioError::NotAProject {
                start: PathBuf::from("synthetic"),
            },
        );
        assert_eq!(error.kind(), wire::IpcErrorKind::NotAProject);
    }

    #[test]
    fn index_errors_are_index_errors() {
        let error = IpcError::from(IndexError::Busy {
            path: PathBuf::from("synthetic.sqlite"),
        });
        assert!(
            matches!(&error, IpcError::Index { message } if message.contains("synthetic.sqlite")),
            "{error:?}"
        );
    }

    #[test]
    fn other_studio_errors_are_index_errors() {
        let error = IpcError::from(StudioError::UserSettings {
            path: PathBuf::from("settings.toml"),
            message: "synthetic".to_owned(),
        });
        assert!(matches!(error, IpcError::Index { .. }), "{error:?}");
    }
}
