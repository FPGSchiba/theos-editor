//! The state file: what the app remembers between runs, `editor-state.json` in the data folder.
//!
//! It is app state, not a setting: the settings layers (P14) stay untouched (P11 ruling 8). It is
//! the only file the app writes.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The name of the state file in the data folder.
pub const FILE_NAME: &str = "editor-state.json";

/// The data folder: the platform's data folder joined with `theos`. `None` when the platform
/// has no data folder.
#[must_use]
pub fn folder() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("theos"))
}

/// What the state file holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorState {
    /// The folder of the project opened last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_project: Option<PathBuf>,
}

/// The state file in one data folder, or none at all.
#[derive(Clone, Debug)]
pub struct StateFile {
    folder: Option<PathBuf>,
}

impl StateFile {
    /// The state file in `folder`; with `None`, nothing is read or written.
    #[must_use]
    pub fn in_folder(folder: Option<PathBuf>) -> StateFile {
        StateFile { folder }
    }

    /// The file, when there is a data folder.
    #[must_use]
    pub fn path(&self) -> Option<PathBuf> {
        self.folder.as_deref().map(|folder| folder.join(FILE_NAME))
    }

    /// What the file holds. A missing file is the empty state; an unreadable or damaged one is
    /// too, with a warning, since the state only saves the user a step.
    #[must_use]
    pub fn load(&self) -> EditorState {
        let Some(path) = self.path() else {
            return EditorState::default();
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return EditorState::default();
            }
            Err(error) => {
                tracing::warn!(file = %path.display(), %error, "cannot read the state file");
                return EditorState::default();
            }
        };
        serde_json::from_str(&text).unwrap_or_else(|error| {
            tracing::warn!(file = %path.display(), %error, "the state file is damaged; ignored");
            EditorState::default()
        })
    }

    /// Writes `state`, creating the data folder. It writes a sibling file first and renames it,
    /// so a crash never leaves half a file.
    ///
    /// # Errors
    ///
    /// When the folder or the file cannot be written.
    pub fn save(&self, state: &EditorState) -> std::io::Result<()> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        std::fs::create_dir_all(folder)?;
        let path = folder.join(FILE_NAME);
        let mut text = serde_json::to_string_pretty(state)?;
        text.push('\n');
        let partial = path.with_extension("json.partial");
        std::fs::write(&partial, text)?;
        std::fs::rename(&partial, &path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn the_folder_is_theos_in_the_platform_data_folder() {
        assert_eq!(folder(), dirs::data_dir().map(|dir| dir.join("theos")));
    }

    #[test]
    fn no_folder_reads_empty_and_writes_nothing() -> TestResult {
        let file = StateFile::in_folder(None);
        assert_eq!(file.path(), None);
        assert_eq!(file.load(), EditorState::default());
        file.save(&EditorState {
            last_project: Some(PathBuf::from("synthetic")),
        })?;
        Ok(())
    }

    #[test]
    fn a_missing_file_reads_empty() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = StateFile::in_folder(Some(dir.path().join("missing")));
        assert_eq!(
            file.path(),
            Some(dir.path().join("missing").join(FILE_NAME))
        );
        assert_eq!(file.load(), EditorState::default());
        Ok(())
    }

    #[test]
    fn a_damaged_file_reads_empty() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(FILE_NAME), "{ not json")?;
        let file = StateFile::in_folder(Some(dir.path().to_path_buf()));
        assert_eq!(file.load(), EditorState::default());
        Ok(())
    }

    #[test]
    fn an_unreadable_file_reads_empty_and_a_blocked_folder_does_not_save() -> TestResult {
        let dir = tempfile::tempdir()?;
        // A folder where the file should be cannot be read as one.
        std::fs::create_dir(dir.path().join(FILE_NAME))?;
        let file = StateFile::in_folder(Some(dir.path().to_path_buf()));
        assert_eq!(file.load(), EditorState::default());
        // A file where the data folder should be cannot hold the state file.
        let blocked = dir.path().join("synthetic-file");
        std::fs::write(&blocked, "")?;
        let file = StateFile::in_folder(Some(blocked.join("theos")));
        assert!(file.save(&EditorState::default()).is_err());
        Ok(())
    }

    #[test]
    fn saved_state_reads_back_with_lf() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = StateFile::in_folder(Some(dir.path().join("nested")));
        let state = EditorState {
            last_project: Some(PathBuf::from("synthetic project")),
        };
        file.save(&state)?;
        assert_eq!(file.load(), state);
        let text = std::fs::read_to_string(dir.path().join("nested").join(FILE_NAME))?;
        assert!(text.ends_with("}\n") && !text.contains('\r'), "{text:?}");
        Ok(())
    }
}
