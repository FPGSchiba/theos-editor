//! `cargo xtask needs <check>,…`: whether a run needs any of some checks.
//!
//! CI asks before it installs what a job's checks need (Linux packages, Node and Playwright), so
//! a pull request that changes no code does not pay for them. The answer is the one `gate` gives:
//! the scope of the run ([`scope::runs`]), and for `ui` also whether `editor-ui/` exists. The
//! Linux packages of the GUI ask one thing more: that `Cargo.lock` names the package that links
//! them (`--requires-package`), because until the lock names it nothing needs them.

use std::io::Write;
use std::path::Path;

use crate::checks::ui::{self, UiPlan};
use crate::ci::CiEvent;
use crate::scope::{self, Scope};
use crate::verdict::Check;
use crate::{Error, Result};

/// The environment variable GitHub Actions gives the file of the step outputs in.
const OUTPUT_FILE: &str = "GITHUB_OUTPUT";

/// Whether any of `checks` runs under `scope` in the repository at `root`.
#[must_use]
pub fn needed_under(scope: &Scope, root: &Path, checks: &[Check]) -> bool {
    checks.iter().any(|&check| {
        scope::runs(scope, check)
            && !(check == Check::Ui && matches!(ui::plan(root), UiPlan::NotNeeded(_)))
    })
}

/// Whether `Cargo.lock` of `root` names a package `name`. A lock that cannot be read or parsed
/// counts as naming it: every doubt adds work, none removes it.
#[must_use]
pub fn lock_names(root: &Path, name: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.lock")) else {
        return true;
    };
    let Ok(lock) = text.parse::<toml::Table>() else {
        return true;
    };
    let Some(packages) = lock.get("package").and_then(toml::Value::as_array) else {
        return true;
    };
    packages
        .iter()
        .any(|package| package.get("name").and_then(toml::Value::as_str) == Some(name))
}

/// [`run_with`] with the step output file that GitHub Actions names in `GITHUB_OUTPUT`.
///
/// # Errors
///
/// The errors of [`run_with`].
pub fn run(
    root: &Path,
    ci: Option<&CiEvent>,
    checks: &[Check],
    requires_package: Option<&str>,
    out: &mut dyn Write,
) -> Result<bool> {
    let output_file = std::env::var_os(OUTPUT_FILE);
    run_with(
        root,
        ci,
        checks,
        requires_package,
        output_file.as_deref(),
        out,
    )
}

/// Writes `true` or `false` to `out` and, under GitHub Actions, `needed=<answer>` to
/// `output_file`, the file of the step outputs. The answer is true when any of `checks` is needed
/// ([`needed_under`]) and, with `requires_package`, `Cargo.lock` names that package
/// ([`lock_names`]). The command itself succeeds whatever the answer, so it returns `true`.
///
/// # Errors
///
/// Fails when `out` or the step output file cannot be written, and under GitHub Actions when
/// there is no step output file: the steps that wait for the answer would all be skipped, and a
/// required job would end green without running its check.
pub fn run_with(
    root: &Path,
    ci: Option<&CiEvent>,
    checks: &[Check],
    requires_package: Option<&str>,
    output_file: Option<&std::ffi::OsStr>,
    out: &mut dyn Write,
) -> Result<bool> {
    let answer = needed_under(&scope::for_run(root, ci), root, checks)
        && requires_package.is_none_or(|name| lock_names(root, name));
    writeln!(out, "{answer}").map_err(|source| Error::Io {
        path: "stdout".into(),
        source,
    })?;
    if ci.is_some() {
        let file = output_file.ok_or_else(|| Error::Missing {
            what: format!("the file of the step outputs: ${OUTPUT_FILE} is not set"),
        })?;
        append(Path::new(file), answer)?;
    }
    Ok(true)
}

/// Appends the output line to the step output file.
fn append(file: &Path, answer: bool) -> Result<()> {
    let io = |source| Error::Io {
        path: file.to_path_buf(),
        source,
    };
    let mut handle = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
        .map_err(io)?;
    writeln!(handle, "needed={answer}").map_err(io)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::ci::{PullRequestEvent, PushEvent};
    use crate::git::testing::{TestResult, commit, git, init_repo};

    const REAL_LISTS: &str = include_str!("../../gate/code-paths.toml");

    fn write(dir: &Path, path: &str, text: &str) -> TestResult {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, text)?;
        Ok(())
    }

    /// A repository with `origin/main` at a first commit and a branch `docs/x` that adds
    /// `docs/a.md`; the base and head commits.
    fn docs_branch() -> TestResult<(tempfile::TempDir, String, String)> {
        let dir = init_repo()?;
        let path = dir.path();
        write(path, "gate/code-paths.toml", REAL_LISTS)?;
        write(path, "README.md", "x\n")?;
        let base = commit(path, "chore: initial")?;
        git(path, &["update-ref", "refs/remotes/origin/main", &base])?;
        git(path, &["switch", "-q", "-c", "docs/x"])?;
        write(path, "docs/a.md", "a\n")?;
        let head = commit(path, "docs: a")?;
        Ok((dir, base, head))
    }

    fn pull_request(action: &str, base: &str, head: &str) -> CiEvent {
        CiEvent::PullRequest(PullRequestEvent {
            action: action.to_owned(),
            number: 1,
            title: "docs: a".to_owned(),
            body: None,
            base_sha: base.to_owned(),
            head_sha: head.to_owned(),
            head_ref: "docs/x".to_owned(),
            label: (action == "labeled").then(|| "windows".to_owned()),
        })
    }

    fn answer(root: &Path, ci: Option<&CiEvent>, checks: &[Check]) -> TestResult<String> {
        // Under CI the step output file is needed; the answer is read from stdout.
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("output");
        let mut out = Vec::new();
        assert!(run_with(
            root,
            ci,
            checks,
            None,
            Some(file.as_os_str()),
            &mut out
        )?);
        Ok(String::from_utf8(out)?)
    }

    // AC: CE-08C.2
    #[test]
    fn a_docs_only_pull_request_needs_no_check_but_commit_msg() -> TestResult {
        let (dir, base, head) = docs_branch()?;
        let event = pull_request("opened", &base, &head);
        for checks in [
            vec![Check::Clippy, Check::Docs],
            vec![Check::Test],
            vec![Check::Coverage],
            vec![Check::Ui],
        ] {
            assert_eq!(answer(dir.path(), Some(&event), &checks)?, "false\n");
        }
        assert_eq!(
            answer(dir.path(), Some(&event), &[Check::Clippy, Check::CommitMsg])?,
            "true\n"
        );
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_pull_request_that_changes_code_needs_the_checks() -> TestResult {
        let (dir, base, _) = docs_branch()?;
        let path = dir.path();
        write(path, "crates/a/src/lib.rs", "// a\n")?;
        let head = commit(path, "feat: a")?;
        let event = pull_request("opened", &base, &head);
        assert_eq!(answer(path, Some(&event), &[Check::Clippy])?, "true\n");
        assert_eq!(answer(path, Some(&event), &[Check::Test])?, "true\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_push_a_manual_run_and_a_label_need_every_check() -> TestResult {
        let (dir, base, head) = docs_branch()?;
        let path = dir.path();
        let push = CiEvent::Push(PushEvent {
            before: base.clone(),
            after: head.clone(),
        });
        assert_eq!(answer(path, Some(&push), &[Check::Clippy])?, "true\n");
        assert_eq!(
            answer(path, Some(&CiEvent::Other), &[Check::Test])?,
            "true\n"
        );
        let labeled = pull_request("labeled", &base, &head);
        assert_eq!(answer(path, Some(&labeled), &[Check::Clippy])?, "true\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_run_outside_ci_decides_from_the_working_tree() -> TestResult {
        let (dir, _, _) = docs_branch()?;
        let path = dir.path();
        assert_eq!(answer(path, None, &[Check::Clippy])?, "false\n");
        write(path, "crates/a/src/b.rs", "// b\n")?;
        assert_eq!(answer(path, None, &[Check::Clippy])?, "true\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn ui_is_not_needed_without_editor_ui_even_when_code_changed() -> TestResult {
        let (dir, _, _) = docs_branch()?;
        let path = dir.path();
        write(path, "crates/a/src/b.rs", "// b\n")?;
        assert_eq!(answer(path, None, &[Check::Ui])?, "false\n");
        // Any other check in the list keeps the answer true.
        assert_eq!(answer(path, None, &[Check::Ui, Check::Test])?, "true\n");
        write(path, "editor-ui/package.json", "{}\n")?;
        assert_eq!(answer(path, None, &[Check::Ui])?, "true\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn the_packages_wait_for_a_lock_that_names_the_package_that_links_them() -> TestResult {
        let (dir, _, _) = docs_branch()?;
        let path = dir.path();
        write(path, "crates/a/src/b.rs", "// b\n")?;
        let asked = |path: &Path| -> TestResult<String> {
            let mut out = Vec::new();
            assert!(run(
                path,
                None,
                &[Check::Clippy],
                Some("webkit2gtk"),
                &mut out
            )?);
            Ok(String::from_utf8(out)?)
        };
        // No lock, or one that cannot be read: doubt adds the install.
        assert_eq!(asked(path)?, "true\n");
        write(path, "Cargo.lock", "this is not toml [")?;
        assert_eq!(asked(path)?, "true\n");
        // A lock without the package: nothing links the libraries yet.
        write(
            path,
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n",
        )?;
        assert_eq!(asked(path)?, "false\n");
        // A lock with it, and a code change: install.
        write(
            path,
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"webkit2gtk\"\nversion = \"2.0.0\"\n",
        )?;
        assert_eq!(asked(path)?, "true\n");
        // The scope still decides first: a docs-only change installs nothing.
        git(path, &["checkout", "-q", "--", "."])?;
        git(path, &["clean", "-fdq"])?;
        assert_eq!(asked(path)?, "false\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn under_ci_a_missing_step_output_file_is_an_error() -> TestResult {
        let (dir, base, head) = docs_branch()?;
        let event = pull_request("opened", &base, &head);
        let mut out = Vec::new();
        let error = run_with(
            dir.path(),
            Some(&event),
            &[Check::Test],
            None,
            None,
            &mut out,
        )
        .err()
        .ok_or("no error")?;
        assert!(error.to_string().contains("GITHUB_OUTPUT"), "{error}");

        // With the file, the same run writes the answer there; outside CI no file is needed.
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("output");
        let mut out = Vec::new();
        run_with(
            dir.path(),
            Some(&event),
            &[Check::Test],
            None,
            Some(file.as_os_str()),
            &mut out,
        )?;
        assert_eq!(fs::read_to_string(&file)?, "needed=false\n");
        assert!(run_with(
            dir.path(),
            None,
            &[Check::Test],
            None,
            None,
            &mut out
        )?);
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn the_answer_goes_to_the_step_outputs() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("output");
        append(&file, true)?;
        append(&file, false)?;
        assert_eq!(fs::read_to_string(&file)?, "needed=true\nneeded=false\n");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn an_output_that_cannot_be_written_is_an_error() {
        assert!(append(Path::new("no-such-folder").join("output").as_path(), true).is_err());
    }
}
