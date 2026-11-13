//! `check_staged` against real repositories: the commit is judged, not the working tree.

use theos_studio::{StagedCheck, StudioError, Workspace, check_staged};

use crate::common::{Error, TempProject, TestResult, put};
use crate::repo::{Repo, variant};

const C901: &str = "lore/clues/C-901.md";
const C900: &str = "lore/clues/C-900.md";
const T900: &str = "lore/truths/T-900.md";
const SETTINGS: &str = ".theos/settings.toml";

/// Opens the project at `place` below the top of `repo` and checks the staged snapshot.
fn check(repo: &Repo, place: &str) -> Result<StagedCheck, Error> {
    let mut workspace = Workspace::open(&repo.top().join(place))?;
    Ok(check_staged(&mut workspace, &repo.git()?)?)
}

/// Writes `text` to `path` below `place`.
fn write(repo: &Repo, place: &str, path: &str, text: &str) -> Result<(), Error> {
    put(&repo.top().join(place), path, text.as_bytes())?;
    Ok(())
}

/// Writes `text` to `path` below `place` and stages it.
fn stage(repo: &Repo, place: &str, path: &str, text: &str) -> Result<(), Error> {
    write(repo, place, path, text)?;
    repo.run(&["add", "--", &format!("{place}{path}")])?;
    Ok(())
}

fn on_rule(check: &StagedCheck, rule: &str, path: &str) -> bool {
    check
        .findings
        .iter()
        .any(|finding| finding.rule == rule && finding.path == path)
}

/// Asserts that the check ran and found no error: the clean fixture's known warning is there.
fn assert_checked(check: &StagedCheck) {
    assert!(!check.blocks(), "{:?}", check.findings);
    assert!(
        on_rule(check, "lore/three-clue", T900),
        "the check did not run: {:?}",
        check.findings
    );
}

#[test]
fn the_clean_fixture_has_no_error() -> TestResult {
    let repo = Repo::committed("")?;
    let result = check(&repo, "")?;
    assert_checked(&result);
    assert_eq!(result.errors(), 0);
    assert_eq!(result.changed.len(), 0);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_staged_never_confirm_violation_blocks() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, "", C901, &variant("C-901.md")?)?;
    let result = check(&repo, "")?;
    assert!(result.blocks());
    assert!(on_rule(&result, "lore/never-confirm", C901));
    assert_eq!(result.findings[0].path, C901);
    assert_eq!(result.changed, [C901]);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_fix_only_in_the_working_tree_does_not_unblock() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, "", C901, &variant("C-901.md")?)?;
    std::fs::remove_file(repo.top().join(C901))?;
    let result = check(&repo, "")?;
    assert!(result.blocks());
    assert!(on_rule(&result, "lore/never-confirm", C901));
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_violation_only_in_the_working_tree_does_not_block() -> TestResult {
    let repo = Repo::committed("")?;
    let violating = variant("C-901.md")?;
    write(&repo, "", C901, &violating)?;
    assert_checked(&check(&repo, "")?);
    let clean = violating.replace("reveals: T-901", "reveals: T-900");
    stage(&repo, "", C901, &clean)?;
    write(&repo, "", C901, &violating)?;
    let result = check(&repo, "")?;
    assert_checked(&result);
    assert_eq!(result.changed, [C901]);
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_lowered_target_blocks_through_the_committed_clue() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, "", T900, &variant("T-900-lowered.md")?)?;
    let result = check(&repo, "")?;
    assert!(result.blocks());
    assert!(on_rule(&result, "lore/no-overshoot", C900));
    assert!(!result.changed.iter().any(|path| path == C900));
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_staged_guardrail_lowering_blocks() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, "", SETTINGS, &variant("settings-lowering.toml")?)?;
    let result = check(&repo, "")?;
    assert!(result.blocks());
    assert!(on_rule(&result, "settings/guardrail", SETTINGS));
    Ok(())
}

#[test]
fn a_locally_excluded_record_does_not_block() -> TestResult {
    let repo = Repo::committed("")?;
    write(&repo, "", ".git/info/exclude", &format!("{C901}\n"))?;
    write(&repo, "", C901, &variant("C-901.md")?)?;
    assert_checked(&check(&repo, "")?);
    Ok(())
}

#[test]
fn a_staged_deletion_of_a_file_on_disk_is_checked_as_gone() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["rm", "-q", "--cached", T900])?;
    assert!(repo.top().join(T900).is_file());
    let result = check(&repo, "")?;
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.path == C900 && finding.message.contains("T-900")),
        "{:?}",
        result.findings
    );
    assert_eq!(result.changed, [T900]);
    Ok(())
}

#[test]
fn links_and_submodules_at_record_paths_are_not_records() -> TestResult {
    let repo = Repo::committed("")?;
    write(&repo, "", C901, &variant("C-901.md")?)?;
    write(&repo, "", "link-target.txt", T900)?;
    let blob = repo
        .run(&["hash-object", "-w", "link-target.txt"])?
        .trim()
        .to_owned();
    repo.run(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("120000,{blob},{C901}"),
    ])?;
    assert_checked(&check(&repo, "")?);
    let head = repo.run(&["rev-parse", "HEAD"])?.trim().to_owned();
    repo.run(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{head},{C901}"),
    ])?;
    assert_checked(&check(&repo, "")?);
    Ok(())
}

#[test]
fn a_project_in_a_subfolder_maps_its_paths() -> TestResult {
    let place = "[draft] Inhalt ä/";
    let repo = Repo::committed(place)?;
    stage(&repo, "", "other/C-901.md", &variant("C-901.md")?)?;
    stage(&repo, place, C901, &variant("C-901.md")?)?;
    let result = check(&repo, place)?;
    assert!(result.blocks());
    assert!(on_rule(&result, "lore/never-confirm", C901));
    assert_eq!(result.changed, [C901]);
    assert!(result.findings.iter().all(|f| !f.path.contains("other")));
    Ok(())
}

#[test]
fn staged_settings_decide_the_snapshot_severities() -> TestResult {
    let repo = Repo::committed("")?;
    let original = std::fs::read_to_string(repo.top().join(SETTINGS))?;
    stage(&repo, "", SETTINGS, &variant("settings-raising.toml")?)?;
    write(&repo, "", SETTINGS, &original)?;
    let result = check(&repo, "")?;
    assert!(result.blocks());
    assert!(on_rule(&result, "lore/three-clue", T900));
    assert!(!result.changed.iter().any(|path| path == T900));
    Ok(())
}

#[test]
fn a_force_added_ignored_record_is_not_checked() -> TestResult {
    let repo = Repo::committed("")?;
    let ignore = std::fs::read_to_string(repo.top().join(".gitignore"))?;
    write(&repo, "", ".gitignore", &format!("{ignore}{C901}\n"))?;
    repo.commit("ignore")?;
    write(&repo, "", C901, &variant("C-901.md")?)?;
    repo.run(&["add", "-f", "--", C901])?;
    assert_checked(&check(&repo, "")?);
    Ok(())
}

/// Makes git compare paths without case, as `git init` does on Windows and macOS.
fn ignore_case(repo: &Repo) -> TestResult {
    repo.run(&["config", "core.ignorecase", "true"])?;
    Ok(())
}

// On a case-insensitive file system git still finds the file under its own spelling and the cache
// holds the disk's; on a case-sensitive one git reports its spelling as gone from the working
// tree. Both must check the staged bytes at git's spelling.
// AC: CE-17.2
#[test]
fn a_case_only_rename_of_a_record_is_checked_in_git_spelling() -> TestResult {
    let repo = Repo::committed("")?;
    ignore_case(&repo)?;
    let committed = std::fs::read_to_string(repo.top().join(C900))?;
    let violating = committed.replace("reveals: T-900", "reveals: T-901");
    stage(&repo, "", C900, &violating)?;
    std::fs::rename(
        repo.top().join(C900),
        repo.top().join("lore/clues/c-900.md"),
    )?;
    let result = check(&repo, "")?;
    assert!(result.blocks(), "{:?}", result.findings);
    assert!(
        on_rule(&result, "lore/never-confirm", C900),
        "{:?}",
        result.findings
    );
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_case_only_rename_of_the_project_folder_is_checked() -> TestResult {
    let repo = Repo::committed("content/")?;
    ignore_case(&repo)?;
    stage(&repo, "content/", C901, &variant("C-901.md")?)?;
    std::fs::rename(repo.top().join("content"), repo.top().join("Content"))?;
    let result = check(&repo, "Content")?;
    assert!(result.blocks(), "{:?}", result.findings);
    assert!(
        on_rule(&result, "lore/never-confirm", C901),
        "{:?}",
        result.findings
    );
    Ok(())
}

#[test]
fn a_folder_outside_git_is_not_in_a_repository() -> TestResult {
    let repo = Repo::committed("")?;
    let project = TempProject::copy_of_studio("projects/staged")?;
    let mut workspace = Workspace::open(project.root())?;
    assert!(matches!(
        check_staged(&mut workspace, &repo.git()?),
        Err(StudioError::NotInRepository { .. })
    ));
    Ok(())
}
