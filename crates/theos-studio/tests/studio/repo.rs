//! An isolated git repository around a copy of the staged fixture project.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use theos_vcs::Git;

use crate::common::{Error, TempProject, files, put};

/// A temp folder with an empty git configuration and one repository in it.
pub struct Repo {
    /// Kept for its drop, which deletes the folder.
    _dir: TempDir,
    config: PathBuf,
    /// The top folder of the work tree: the temp folder itself, or a folder in it for a clone.
    top: PathBuf,
}

impl Repo {
    /// A repository whose first commit holds the fixture project `projects/staged` below
    /// `place` (empty for the top of the repository).
    pub fn committed(place: &str) -> Result<Repo, Error> {
        let dir = tempfile::tempdir()?;
        let config = dir.path().join("gitconfig");
        std::fs::write(&config, "")?;
        let top = dir.path().to_path_buf();
        let repo = Repo {
            _dir: dir,
            config,
            top,
        };
        repo.run(&["init", "-q", "-b", "main"])?;
        let fixture = TempProject::copy_of_studio("projects/staged")?;
        let top = repo.top().join(place);
        for (path, bytes) in files(fixture.root())? {
            put(&top, &path, &bytes)?;
        }
        repo.commit("fixture")?;
        Ok(repo)
    }

    /// A clone of the repository at `source` (a local bare repository, say), with a synthetic
    /// identity in its own configuration so that git can commit and merge in it.
    pub fn clone_of(source: &Path) -> Result<Repo, Error> {
        let dir = tempfile::tempdir()?;
        let config = dir.path().join("gitconfig");
        std::fs::write(&config, "")?;
        let top = dir.path().join("clone");
        let output = Command::new("git")
            .args(["clone", "-q", "--"])
            .arg(source)
            .arg(&top)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let repo = Repo {
            _dir: dir,
            config,
            top,
        };
        repo.run(&["config", "user.name", "Synthetic"])?;
        repo.run(&["config", "user.email", "synthetic@example.invalid"])?;
        Ok(repo)
    }

    /// The top folder of the repository.
    pub fn top(&self) -> &Path {
        &self.top
    }

    /// A runner that ignores the user and system git configuration.
    pub fn git(&self) -> Result<Git, Error> {
        Ok(Git::find()?.with_env(&[
            (
                "GIT_CONFIG_GLOBAL",
                self.config.to_str().ok_or("config path")?,
            ),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ]))
    }

    /// Runs git in the top folder with the isolation of the repository and returns stdout.
    pub fn run(&self, args: &[&str]) -> Result<String, Error> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.top())
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    /// Stages everything and commits it with a synthetic identity.
    pub fn commit(&self, message: &str) -> Result<(), Error> {
        self.run(&["add", "-A"])?;
        self.run(&[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "commit",
            "-q",
            "-m",
            message,
        ])?;
        Ok(())
    }
}

/// The text of the file `name` in `tests/fixtures/staged-variants`.
pub fn variant(name: &str) -> Result<String, Error> {
    Ok(std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("staged-variants")
            .join(name),
    )?)
}
