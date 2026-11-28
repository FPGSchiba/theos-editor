//! A sandbox for tests: a temp folder, an isolated git configuration and small git helpers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;
use theos_vcs::Git;

/// A temp folder holding an empty git configuration file.
pub struct Sandbox {
    dir: TempDir,
    config: PathBuf,
}

impl Sandbox {
    /// Makes the folder and the empty configuration file.
    pub fn new() -> Result<Sandbox, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let config = dir.path().join("gitconfig");
        std::fs::write(&config, "")?;
        Ok(Sandbox { dir, config })
    }

    /// The sandbox folder.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A runner that ignores the user's and the system's git configuration.
    pub fn git(&self) -> Result<Git, Box<dyn std::error::Error>> {
        Ok(Git::find()?.with_env(&[
            (
                "GIT_CONFIG_GLOBAL",
                self.config.to_str().ok_or("config path")?,
            ),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ]))
    }

    /// Creates `rel` below the sandbox and runs `git init -q -b main` in it.
    pub fn init(&self, rel: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let folder = self.dir.path().join(rel);
        std::fs::create_dir_all(&folder)?;
        self.run(&folder, &["init", "-q", "-b", "main"])?;
        Ok(folder)
    }

    /// Writes a file, creating its folders.
    pub fn write(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Runs git in `cwd` with the isolation of the sandbox and returns stdout.
    pub fn run(&self, cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        self.run_env(cwd, args, &[])
    }

    /// Like [`Sandbox::run`], with `input` on git's stdin.
    pub fn run_input(
        &self,
        cwd: &Path,
        args: &[&str],
        input: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut child = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child.stdin.take().ok_or("git stdin")?.write_all(input)?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    /// Like [`Sandbox::run`], with more variables set after the isolation.
    pub fn run_env(
        &self,
        cwd: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .envs(env.iter().copied())
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

    /// Runs a git command that must fail and returns its trimmed stderr.
    pub fn failing_stderr(
        &self,
        cwd: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let error = self
            .run_env(cwd, args, env)
            .err()
            .ok_or("the command succeeded")?
            .to_string();
        // `run_env` words its error "git [...] failed: <stderr>".
        let (_, stderr) = error.split_once("failed: ").ok_or("error text")?;
        Ok(stderr.trim().to_owned())
    }

    /// Stages everything and commits it with a synthetic identity.
    pub fn commit(&self, cwd: &Path, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.run(cwd, &["add", "-A"])?;
        self.run(
            cwd,
            &[
                "-c",
                "user.name=Synthetic",
                "-c",
                "user.email=synthetic@example.invalid",
                "commit",
                "-q",
                "-m",
                message,
            ],
        )?;
        Ok(())
    }
}
