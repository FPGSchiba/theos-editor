//! The user's settings: where the binaries folder is, read from `settings.toml` in the user's
//! `theos` config folder.

use std::path::{Path, PathBuf};

use crate::StudioError;

/// The environment variable that names the config folder instead of the platform's.
const FOLDER_VAR: &str = "THEOS_CONFIG_DIR";
/// The key of the binaries folder in the settings file.
const KEY: &str = "binaries_root";

/// The folder of the user settings: `THEOS_CONFIG_DIR`, else the platform's config folder joined
/// with `theos`. `None` when the platform has no config folder.
#[must_use]
pub fn folder() -> Option<PathBuf> {
    match std::env::var_os(FOLDER_VAR) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => dirs::config_dir().map(|dir| dir.join("theos")),
    }
}

/// The binaries folder the user settings name, as written (a relative path stays relative).
///
/// # Errors
///
/// [`StudioError::UserSettings`] when the file is not valid TOML or the value is not text, and
/// [`StudioError::Io`] when the file cannot be read.
pub fn binaries_root() -> Result<Option<PathBuf>, StudioError> {
    folder().map_or(Ok(None), |folder| binaries_root_in(&folder))
}

fn binaries_root_in(folder: &Path) -> Result<Option<PathBuf>, StudioError> {
    let path = folder.join("settings.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(StudioError::Io { path, source }),
    };
    let table: toml::Table = toml::from_str(&text).map_err(|error| StudioError::UserSettings {
        path: path.clone(),
        message: error.to_string(),
    })?;
    match table.get(KEY) {
        None => Ok(None),
        Some(toml::Value::String(root)) => Ok(Some(PathBuf::from(root))),
        Some(other) => Err(StudioError::UserSettings {
            path,
            message: format!("`{KEY}` must be text, not {}", other.type_str()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(content: &str) -> Result<tempfile::TempDir, std::io::Error> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("settings.toml"), content)?;
        Ok(dir)
    }

    #[test]
    fn no_file_means_no_folder() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        assert_eq!(binaries_root_in(dir.path())?, None);
        Ok(())
    }

    #[test]
    fn a_file_without_the_key_means_no_folder() -> Result<(), Box<dyn std::error::Error>> {
        let dir = settings("other = 1\n")?;
        assert_eq!(binaries_root_in(dir.path())?, None);
        Ok(())
    }

    #[test]
    fn the_key_gives_the_folder() -> Result<(), Box<dyn std::error::Error>> {
        let dir = settings("binaries_root = \"rel/binaries\"\n")?;
        assert_eq!(
            binaries_root_in(dir.path())?,
            Some(PathBuf::from("rel/binaries"))
        );
        Ok(())
    }

    #[test]
    fn invalid_toml_names_the_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = settings("binaries_root = \n")?;
        let error = binaries_root_in(dir.path()).err().ok_or("no error")?;
        assert!(matches!(error, StudioError::UserSettings { .. }));
        let path = dir.path().join("settings.toml");
        assert!(
            error.to_string().starts_with(&path.display().to_string()),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn a_value_that_is_not_text_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let dir = settings("binaries_root = 5\n")?;
        let error = binaries_root_in(dir.path()).err().ok_or("no error")?;
        assert!(matches!(error, StudioError::UserSettings { .. }));
        assert!(error.to_string().contains("binaries_root"), "{error}");
        Ok(())
    }

    #[test]
    fn a_settings_path_that_cannot_be_read_is_an_io_error() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("settings.toml"))?;
        let error = binaries_root_in(dir.path()).err().ok_or("no error")?;
        assert!(matches!(error, StudioError::Io { .. }), "{error}");
        Ok(())
    }
}
