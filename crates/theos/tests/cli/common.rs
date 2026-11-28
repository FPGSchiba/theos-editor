//! Helpers of the CLI tests, each in one place: isolated runs of `theos`, fixture copies and an
//! isolated git repository.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The error of a test or a test helper.
pub type Error = Box<dyn std::error::Error>;

/// The result of a test.
pub type TestResult = Result<(), Error>;

/// A fixture folder below `crates/<relative>`.
pub fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative)
}

/// A P04 fixture project of `theos-content`.
pub fn content_project(name: &str) -> PathBuf {
    fixture("theos-content/tests/fixtures/projects").join(name)
}

/// The P05 fixture project `staged` of `theos-studio`.
pub fn staged_project() -> PathBuf {
    fixture("theos-studio/tests/fixtures/projects/staged")
}

/// The text of a file of `theos-studio/tests/fixtures/staged-variants`.
pub fn variant(name: &str) -> io::Result<String> {
    std::fs::read_to_string(fixture("theos-studio/tests/fixtures/staged-variants").join(name))
}

/// A temp folder with the settings and git isolation every run of `theos` and git needs.
pub struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    /// A new sandbox with an empty `theos` config folder and an empty git configuration.
    pub fn new() -> Result<Sandbox, Error> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("config"))?;
        std::fs::write(dir.path().join("gitconfig"), "")?;
        std::fs::create_dir(dir.path().join("work"))?;
        Ok(Sandbox { dir })
    }

    /// The `theos` config folder (`THEOS_CONFIG_DIR`).
    pub fn config(&self) -> PathBuf {
        self.dir.path().join("config")
    }

    /// The empty git configuration file (`GIT_CONFIG_GLOBAL`).
    pub fn gitconfig(&self) -> PathBuf {
        self.dir.path().join("gitconfig")
    }

    /// A folder for projects and repositories.
    pub fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// Runs `theos` in `cwd` with `args` and the isolation.
    pub fn run(&self, args: &[&str], cwd: &Path) -> io::Result<Output> {
        self.run_binary(Path::new(env!("CARGO_BIN_EXE_theos")), args, cwd)
    }

    /// Runs `theos` in `cwd` with `args` and the isolation, with `PATH` replaced by `path`.
    pub fn run_with_path(&self, args: &[&str], cwd: &Path, path: &OsStr) -> io::Result<Output> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_theos"));
        command.args(args).current_dir(cwd);
        self.isolate(&mut command);
        command.env("PATH", path);
        command.output()
    }

    /// Runs the `theos` at `binary`, which may be a copy of the built one, like [`Sandbox::run`].
    pub fn run_binary(&self, binary: &Path, args: &[&str], cwd: &Path) -> io::Result<Output> {
        let mut command = Command::new(binary);
        command.args(args).current_dir(cwd);
        self.isolate(&mut command);
        command.output()
    }

    /// Runs git in `cwd` with the isolation and returns the whole output, a failure included.
    /// `env` is set after the isolation, for git and the hooks it runs: a test hands git a `PATH`
    /// or a `GIT_WORK_TREE` this way on purpose.
    pub fn git_output(
        &self,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &OsStr)],
    ) -> io::Result<Output> {
        let mut command = Command::new("git");
        command.args(args).current_dir(cwd);
        self.isolate(&mut command);
        command.envs(env.iter().copied());
        command.output()
    }

    /// Runs git in `cwd` with the isolation; fails unless it succeeds. Returns stdout.
    pub fn git(&self, args: &[&str], cwd: &Path) -> Result<String, Error> {
        let mut command = Command::new("git");
        command.args(args).current_dir(cwd);
        self.isolate(&mut command);
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    fn isolate(&self, command: &mut Command) {
        command
            .env("THEOS_CONFIG_DIR", self.config())
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
    }

    /// A copy of the project `from` in a fresh folder of the sandbox, named `name`.
    pub fn copy_project(&self, from: &Path, name: &str) -> io::Result<PathBuf> {
        let to = self.work().join(name);
        std::fs::create_dir_all(&to)?;
        copy_dir(from, &to)?;
        Ok(to)
    }

    /// Makes `folder` a repository with everything in it committed.
    pub fn commit_all(&self, folder: &Path) -> Result<(), Error> {
        self.git(&["init", "-q", "-b", "main"], folder)?;
        self.git(&["add", "-A"], folder)?;
        self.git(
            &[
                "-c",
                "user.name=Synthetic",
                "-c",
                "user.email=synthetic@example.invalid",
                "commit",
                "-q",
                "-m",
                "fixture",
            ],
            folder,
        )?;
        Ok(())
    }
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The standard output of a run as text.
pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The standard error of a run as text.
pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
