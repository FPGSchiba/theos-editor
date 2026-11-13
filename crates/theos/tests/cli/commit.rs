//! Real `git commit`s through the hook that `theos hook install` writes (spec §5.7, §5.8): the
//! staged snapshot is judged, and a refused commit leaves the history as it was.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::common::{Error, Sandbox, TestResult, staged_project, stderr, variant};

const C901: &str = "lore/clues/C-901.md";
const C900: &str = "lore/clues/C-900.md";
const T900: &str = "lore/truths/T-900.md";
const SETTINGS: &str = ".theos/settings.toml";
const NOTES: &str = "notes.txt";
const SUBFOLDER: &str = "[draft] Inhalt ä";

/// A repository with the `staged` fixture project at `place` below its top.
struct Repo<'a> {
    sandbox: &'a Sandbox,
    top: PathBuf,
    project: PathBuf,
}

impl<'a> Repo<'a> {
    /// The project copied and `git init`ed, nothing committed and no hook yet.
    fn init(sandbox: &'a Sandbox, place: &str) -> Result<Repo<'a>, Error> {
        let name = if place.is_empty() {
            "repo".to_owned()
        } else {
            format!("repo/{place}")
        };
        let project = sandbox.copy_project(&staged_project(), &name)?;
        let top = sandbox.work().join("repo");
        sandbox.git(&["init", "-q", "-b", "main"], &top)?;
        Ok(Repo {
            sandbox,
            top,
            project,
        })
    }

    /// The fixture committed before the hook exists, then the hook installed with the built
    /// `theos`.
    fn with_hook(sandbox: &'a Sandbox, place: &str) -> Result<Repo<'a>, Error> {
        let repo = Repo::init(sandbox, place)?;
        repo.git_ok(&["add", "-A"])?;
        repo.commit(&["-m", "fixture"], &[])?.require_success()?;
        repo.install(None)?;
        Ok(repo)
    }

    /// Runs `theos hook install` with `binary`, or the built one.
    fn install(&self, binary: Option<&Path>) -> TestResult {
        let binary = binary.unwrap_or(Path::new(env!("CARGO_BIN_EXE_theos")));
        let output = self
            .sandbox
            .run_binary(binary, &["hook", "install"], &self.project)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!("hook install failed: {}", stderr(&output)).into())
        }
    }

    fn git_ok(&self, args: &[&str]) -> Result<String, Error> {
        self.sandbox.git(args, &self.project)
    }

    /// Writes `text` to `path` below the project.
    fn put(&self, path: &str, text: &str) -> Result<(), Error> {
        let file = self.project.join(path);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(file, text)?;
        Ok(())
    }

    /// Writes `text` to `path` and stages it.
    fn stage(&self, path: &str, text: &str) -> TestResult {
        self.put(path, text)?;
        self.git_ok(&["add", "--", path])?;
        Ok(())
    }

    /// `git commit` with `config` as `-c` options and `rest` after `commit`.
    fn commit(&self, rest: &[&str], config: &[&str]) -> Result<Committed, Error> {
        self.commit_with_path(rest, config, None)
    }

    fn commit_with_path(
        &self,
        rest: &[&str],
        config: &[&str],
        path: Option<&OsString>,
    ) -> Result<Committed, Error> {
        let before = self.count()?;
        let mut args = vec![
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
        ];
        for option in config {
            args.extend(["-c", option]);
        }
        args.extend(["commit", "-q"]);
        args.extend(rest);
        let output =
            self.sandbox
                .git_output(&args, &self.project, path.map(OsString::as_os_str))?;
        Ok(Committed {
            output,
            before,
            after: self.count()?,
        })
    }

    /// The number of commits on `HEAD`. It is 0 only when `git rev-parse --verify -q HEAD` exits
    /// with exactly 1, which is an unborn `HEAD`; any other non-zero exit (128 for a fatal error)
    /// fails the test with git's stderr.
    fn count(&self) -> Result<u32, Error> {
        let head = self.sandbox.git_output(
            &["rev-parse", "--verify", "-q", "HEAD"],
            &self.project,
            None,
        )?;
        match head.status.code() {
            Some(0) => {}
            Some(1) => return Ok(0),
            _ => return Err(format!("git rev-parse HEAD failed: {}", stderr(&head)).into()),
        }
        let count = self.git_ok(&["rev-list", "--count", "HEAD"])?;
        Ok(count.trim().parse()?)
    }

    /// `git add -A` with `config` as `-c` options.
    fn add_all(&self, config: &[&str]) -> TestResult {
        let mut args = Vec::new();
        for option in config {
            args.extend(["-c", option]);
        }
        args.extend(["add", "-A"]);
        self.git_ok(&args)?;
        Ok(())
    }

    /// Stages a clean change, so there is something to commit.
    fn stage_notes(&self) -> TestResult {
        self.stage(NOTES, "Synthetic note.\n")
    }
}

/// The outcome of a `git commit` and the commit count around it.
struct Committed {
    output: Output,
    before: u32,
    after: u32,
}

impl Committed {
    fn require_success(self) -> Result<Committed, Error> {
        if self.output.status.success() {
            Ok(self)
        } else {
            Err(format!("git commit failed: {}", stderr(&self.output)).into())
        }
    }

    /// Asserts that the commit failed, named `rule` and `file`, and left the history alone.
    fn assert_refused(&self, rule: &str, file: &str) {
        let text = stderr(&self.output);
        assert!(!self.output.status.success(), "{text}");
        assert!(text.contains(rule), "{rule} not named: {text}");
        assert!(text.contains(file), "{file} not named: {text}");
        assert!(text.contains("commit refused"), "{text}");
        assert_eq!(self.before, self.after, "the history grew: {text}");
    }

    /// Asserts that the commit succeeded and added one commit.
    fn assert_commits(&self) {
        let text = stderr(&self.output);
        assert!(self.output.status.success(), "{text}");
        assert_eq!(self.before + 1, self.after, "{text}");
    }
}

fn read(path: &Path) -> Result<String, Error> {
    Ok(std::fs::read_to_string(path)?)
}

/// The fixture's C-900 reveals the never-confirm truth T-901 instead of T-900.
fn clue_revealing_never(repo: &Repo<'_>) -> Result<String, Error> {
    Ok(read(&repo.project.join(C900))?.replace("reveals: T-900", "reveals: T-901"))
}

// AC: CE-17.2
#[test]
fn a_staged_never_confirm_violation_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.stage(C901, &variant("C-901.md")?)?;
    repo.commit(&["-m", "violation"], &[])?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_fix_only_in_the_working_tree_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.stage(C901, &variant("C-901.md")?)?;
    let fixed = variant("C-901.md")?.replace("reveals: T-901", "reveals: T-900");
    repo.put(C901, &fixed)?;
    repo.commit(&["-m", "violation"], &[])?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_violation_only_in_the_working_tree_commits() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.stage_notes()?;
    repo.put(C901, &variant("C-901.md")?)?;
    repo.put(C900, &clue_revealing_never(&repo)?)?;
    repo.commit(&["-m", "clean"], &[])?.assert_commits();
    Ok(())
}

// AC: CE-17.2
#[test]
fn commit_a_and_commit_path_check_their_own_snapshots() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;

    // commit -a takes the tracked edit, so the violation is in the commit.
    repo.put(C900, &clue_revealing_never(&repo)?)?;
    repo.commit(&["-a", "-m", "edited"], &[])?
        .assert_refused("lore/never-confirm", C900);
    repo.git_ok(&["checkout", "-q", "--", C900])?;

    // commit <clean path> leaves the staged violation out and commits.
    repo.stage(C901, &variant("C-901.md")?)?;
    repo.stage_notes()?;
    repo.commit(&["-m", "notes", "--", NOTES], &[])?
        .assert_commits();

    // commit <violating path> is refused.
    repo.commit(&["-m", "clue", "--", C901], &[])?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_lowered_target_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.stage(T900, &variant("T-900-lowered.md")?)?;
    repo.commit(&["-m", "lowered"], &[])?
        .assert_refused("lore/no-overshoot", C900);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_staged_guardrail_lowering_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.stage(SETTINGS, &variant("settings-lowering.toml")?)?;
    repo.commit(&["-m", "lowering"], &[])?
        .assert_refused("settings/guardrail", SETTINGS);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_locally_excluded_record_does_not_block() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    std::fs::write(
        repo.top.join(".git").join("info").join("exclude"),
        format!("{C901}\n"),
    )?;
    repo.stage_notes()?;
    repo.put(C901, &variant("C-901.md")?)?;
    repo.commit(&["-m", "clean"], &[])?.assert_commits();
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_project_in_a_subfolder_commits_through_its_hook() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, SUBFOLDER)?;
    repo.stage_notes()?;
    repo.commit(&["-m", "clean"], &[])?.assert_commits();
    repo.stage(C901, &variant("C-901.md")?)?;
    repo.commit(&["-m", "violation"], &[])?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

// The cache holds the disk's spelling `c-900.md`, git's index `C-900.md`; the staged bytes at
// git's spelling are judged, on a case-insensitive file system and on a case-sensitive one.
// AC: CE-17.2
#[test]
fn a_case_only_rename_cannot_hide_a_violation() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    repo.git_ok(&["config", "core.ignorecase", "true"])?;
    repo.stage(C900, &clue_revealing_never(&repo)?)?;
    std::fs::rename(
        repo.project.join(C900),
        repo.project.join("lore").join("clues").join("c-900.md"),
    )?;
    repo.commit(&["-m", "renamed"], &[])?
        .assert_refused("lore/never-confirm", C900);
    Ok(())
}

#[test]
fn the_first_commit_is_checked() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::init(&sandbox, "")?;
    repo.install(None)?;
    repo.put(C901, &variant("C-901.md")?)?;
    repo.git_ok(&["add", "-A"])?;
    let committed = repo.commit(&["-m", "first"], &[])?;
    committed.assert_refused("lore/never-confirm", C901);
    assert_eq!(committed.after, 0);
    Ok(())
}

#[test]
fn a_commit_with_autocrlf_is_checked() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::with_hook(&sandbox, "")?;
    let autocrlf = ["core.autocrlf=true"];
    // CRLF working files of the whole fixture lore, unchanged apart from the line ends.
    for path in [C900, T900, "lore/truths/T-901.md"] {
        let text = read(&repo.project.join(path))?
            .replace("\r\n", "\n")
            .replace('\n', "\r\n");
        repo.put(path, &text)?;
    }
    repo.put(NOTES, "Synthetic note.\r\n")?;
    repo.add_all(&autocrlf)?;
    repo.commit(&["-m", "clean"], &autocrlf)?.assert_commits();

    let violation = variant("C-901.md")?.replace('\n', "\r\n");
    repo.put(C901, &violation)?;
    repo.add_all(&autocrlf)?;
    repo.commit(&["-m", "violation"], &autocrlf)?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

/// A copy of the built `theos` in `<work>/bin dir/it's/`.
fn copy_theos(sandbox: &Sandbox) -> Result<PathBuf, Error> {
    let folder = sandbox.work().join("bin dir").join("it's");
    std::fs::create_dir_all(&folder)?;
    let copy = folder.join(format!("theos{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_theos"), &copy)?;
    Ok(copy)
}

#[test]
fn a_theos_path_with_a_space_and_a_quote_runs() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::init(&sandbox, "")?;
    repo.git_ok(&["add", "-A"])?;
    repo.commit(&["-m", "fixture"], &[])?.require_success()?;
    let copy = copy_theos(&sandbox)?;
    repo.install(Some(&copy))?;
    let hook = read(&repo.top.join(".git").join("hooks").join("pre-commit"))?;
    assert!(hook.contains("it'\\''s"), "{hook}");
    repo.stage(C901, &variant("C-901.md")?)?;
    repo.commit(&["-m", "violation"], &[])?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}

#[test]
fn a_moved_theos_falls_back_to_the_path() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = Repo::init(&sandbox, "")?;
    repo.git_ok(&["add", "-A"])?;
    repo.commit(&["-m", "fixture"], &[])?.require_success()?;
    let copy = copy_theos(&sandbox)?;
    repo.install(Some(&copy))?;
    let hook = read(&repo.top.join(".git").join("hooks").join("pre-commit"))?;
    let named = copy
        .to_str()
        .ok_or("the copy's path is not UTF-8")?
        .replace('\\', "/")
        .replace('\'', "'\\''");
    assert!(hook.contains(&named), "{named} not in {hook}");
    assert!(named.contains("bin dir/it"), "{named}");
    std::fs::remove_file(&copy)?;
    assert!(!copy.exists());
    repo.stage(C901, &variant("C-901.md")?)?;

    let built = PathBuf::from(env!("CARGO_BIN_EXE_theos"));
    let mut dirs = vec![built.parent().map(Path::to_path_buf).ok_or("no folder")?];
    dirs.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let path = std::env::join_paths(dirs)?;
    repo.commit_with_path(&["-m", "violation"], &[], Some(&path))?
        .assert_refused("lore/never-confirm", C901);
    Ok(())
}
