//! Ruling R21: no command changes which files are records where the verdict cannot see it. The
//! preview decides that from the disk as it is before the write, so the pipeline refuses a
//! change of a `.gitignore`, the replacement of a hidden file on Windows, and a path that the
//! disk spells otherwise or that passes a symbolic link to a folder. Each refusal leaves the
//! files, the index cache and the journal as they were.

use theos_studio::{Command, CommandError, Create, Delete, Rename, Workspace, Write};

use crate::common::{
    Error, TRUTH, TempProject, TestResult, base_of, cold_dump, new_truth, put, refused, set,
};

/// The `.gitignore` of `lore/` in [`with_ignored_truth`].
const LORE_GITIGNORE: &str = "lore/.gitignore";

/// The reason a `.gitignore` is refused.
const GITIGNORE: &str =
    "is a .gitignore, which decides which files are records; edit it outside the editor";

/// Whether `error` refuses the path `path` for `reason`.
fn refuses(error: &CommandError, path: &str, reason: &str) -> bool {
    matches!(error, CommandError::Path { path: p, reason: r } if p == path && *r == reason)
}

/// A copy of `commands/` with a truth that the `.gitignore` of `lore/` keeps out, and a workspace
/// on it.
fn with_ignored_truth() -> Result<(TempProject, Workspace), Error> {
    let project = TempProject::copy_of_studio("commands")?;
    let ignored = new_truth("T-962");
    put(project.root(), &ignored.path, ignored.text.as_bytes())?;
    put(project.root(), LORE_GITIGNORE, b"truths/T-962.md\n")?;
    let workspace = Workspace::open(project.root())?;
    assert_eq!(workspace.index().record("T-962")?, Vec::new());
    Ok((project, workspace))
}

/// Ruling R21: a `.gitignore` decides which files below its folder are records, and the preview
/// does not see a change of it, so no command writes, creates or removes one, whatever the case
/// of its name.
#[test]
fn a_gitignore_is_never_written_created_or_removed() -> TestResult {
    let (project, mut workspace) = with_ignored_truth()?;
    let base = base_of(&workspace, LORE_GITIGNORE)?;
    // Each would make the ignored truth a record, or keep records out, unjudged.
    let write = Write {
        path: LORE_GITIGNORE.to_owned(),
        base: base.clone(),
        text: String::new(),
    };
    let delete = Delete {
        path: LORE_GITIGNORE.to_owned(),
        base,
    };
    let create = Create {
        path: "world/.gitignore".to_owned(),
        text: "synthetic-band/\n".to_owned(),
    };
    let create_cased = Create {
        path: "story/.GitIgnore".to_owned(),
        text: "scripts/\n".to_owned(),
    };
    for (command, path) in [
        (&write as &dyn Command, LORE_GITIGNORE),
        (&delete, LORE_GITIGNORE),
        (&create, "world/.gitignore"),
        (&create_cased, "story/.GitIgnore"),
    ] {
        let error = refused(&project, &mut workspace, command)?;
        assert!(
            refuses(&error, path, GITIGNORE),
            "{}: {error:?}",
            command.label()
        );
        assert_eq!(error.to_string(), format!("the path `{path}` {GITIGNORE}"));
    }
    Ok(())
}

/// Ruling R21: commands that touch no `.gitignore` still pass beside one, and the index matches
/// a cold build after each.
#[test]
fn ordinary_commands_beside_a_gitignore_still_pass() -> TestResult {
    let (project, mut workspace) = with_ignored_truth()?;
    let retitle = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let rename = Rename {
        id: "T-960".to_owned(),
        to: "T-970".to_owned(),
    };
    for command in [&retitle as &dyn Command, &rename] {
        let outcome = workspace.run(command)?;
        assert!(!outcome.changed.is_empty(), "{}", command.label());
        assert_eq!(
            workspace.index().dump()?,
            cold_dump(project.root())?,
            "{}",
            command.label()
        );
    }
    assert_eq!(workspace.index().record("T-970")?.len(), 1);
    assert_eq!(workspace.undo_entries().len(), 2);
    Ok(())
}

/// A copy of `commands/` with a truth that is hidden on Windows, and a workspace on it.
#[cfg(windows)]
fn with_hidden_truth() -> Result<(TempProject, Workspace, Create), Error> {
    let project = TempProject::copy_of_studio("commands")?;
    let hidden = new_truth("T-962");
    crate::common::put_hidden(project.root(), &hidden.path, hidden.text.as_bytes())?;
    let workspace = Workspace::open(project.root())?;
    assert_eq!(workspace.index().record("T-962")?, Vec::new());
    Ok((project, workspace, hidden))
}

/// Ruling R21, on Windows: a write swaps in a new file, which does not carry the hidden
/// attribute, so replacing a hidden file would make it a record unjudged.
#[cfg(windows)]
#[test]
fn a_hidden_file_is_never_replaced() -> TestResult {
    let (project, mut workspace, hidden) = with_hidden_truth()?;
    let write = Write {
        path: hidden.path.clone(),
        base: base_of(&workspace, &hidden.path)?,
        text: hidden
            .text
            .replace("Synthetic statement.", "Synthetic statement, changed."),
    };
    let error = refused(&project, &mut workspace, &write)?;
    assert!(refuses(&error, &hidden.path, "is hidden"), "{error:?}");
    assert_eq!(workspace.index().record("T-962")?, Vec::new());
    Ok(())
}

/// Ruling R21, on Windows: removing a hidden file takes no record away, so it is applied.
#[cfg(windows)]
#[test]
fn a_hidden_file_may_be_deleted() -> TestResult {
    let (project, mut workspace, hidden) = with_hidden_truth()?;
    let delete = Delete {
        path: hidden.path.clone(),
        base: base_of(&workspace, &hidden.path)?,
    };
    let outcome = workspace.run(&delete)?;
    assert_eq!(outcome.changed, [hidden.path.as_str()]);
    assert!(!project.root().join(&hidden.path).exists());
    assert_eq!(workspace.index().dump()?, cold_dump(project.root())?);
    assert_eq!(workspace.undo_entries().len(), 1);
    Ok(())
}

/// The other side of ruling R21 on Unix: a name that starts with `.` hides a file, and a write
/// keeps the name, so writing a hidden file is applied and it stays hidden.
#[cfg(unix)]
#[test]
fn a_hidden_file_stays_hidden_when_written() -> TestResult {
    let project = TempProject::copy_of_studio("commands")?;
    let hidden = "lore/truths/.T-962.md";
    let text = new_truth("T-962").text;
    put(project.root(), hidden, text.as_bytes())?;
    let mut workspace = Workspace::open(project.root())?;
    let write = Write {
        path: hidden.to_owned(),
        base: base_of(&workspace, hidden)?,
        text: text.replace("Synthetic statement.", "Synthetic statement, changed."),
    };
    let outcome = workspace.run(&write)?;
    assert_eq!(outcome.changed, [hidden]);
    assert_eq!(workspace.index().record("T-962")?, Vec::new());
    assert_eq!(workspace.index().dump()?, cold_dump(project.root())?);
    Ok(())
}

/// The reason a path is refused whose part on disk is spelled otherwise.
#[cfg(any(windows, target_os = "macos"))]
const MISSPELLED: &str = "is spelled unlike the file or folder on disk";

/// Ruling R21, where the disk ignores case: a path that names a folder or file in another case
/// reaches the file the walk lists under its own spelling, so the preview would judge the wrong
/// path. Deleting the truth so would remove a record unjudged, and creating one so would add a
/// record the index does not list.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn a_path_spelled_unlike_the_disk_is_refused() -> TestResult {
    let (project, mut workspace) = crate::common::open("commands")?;
    let other = "lore/truths/T-961.md";
    let cased = "LORE/truths/T-961.md";
    let delete = Delete {
        path: cased.to_owned(),
        base: base_of(&workspace, other)?,
    };
    let created = new_truth("T-962");
    let create = Create {
        path: created.path.replace("truths", "Truths"),
        text: created.text,
    };
    for (command, path) in [(&delete as &dyn Command, cased), (&create, &create.path)] {
        let error = refused(&project, &mut workspace, command)?;
        assert!(
            refuses(&error, path, MISSPELLED),
            "{}: {error:?}",
            command.label()
        );
    }
    assert_eq!(workspace.index().record("T-961")?.len(), 1);
    Ok(())
}

/// Ruling R21: a path that passes a symbolic link to a folder reaches a file the walk lists
/// under another path, or none, since the walk does not follow links.
#[cfg(unix)]
#[test]
fn a_path_through_a_symbolic_link_is_refused() -> TestResult {
    let (project, mut workspace) = crate::common::open("commands")?;
    std::os::unix::fs::symlink(
        project.root().join("lore").join("truths"),
        project.root().join("lore").join("linked"),
    )?;
    let created = new_truth("T-962");
    let create = Create {
        path: created.path.replace("truths", "linked"),
        text: created.text,
    };
    let error = refused(&project, &mut workspace, &create)?;
    assert!(
        refuses(&error, &create.path, "passes a symbolic link to a folder"),
        "{error:?}"
    );
    assert!(!project.root().join("lore/truths/T-962.md").exists());
    Ok(())
}
