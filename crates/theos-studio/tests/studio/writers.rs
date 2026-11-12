//! Two processes append to the same record through the command layer: the repo lock and the base
//! check together never lose a line.

use std::time::Duration;

use theos_content::Record;
use theos_studio::Workspace;

use crate::common::{TempProject, spawn_helper};
use crate::helper::{HelperTask, append_lines};

const SCRIPT: &str = "story/scripts/S-960.md";
const LINES: usize = 20;

// AC: CE-22.2
#[test]
fn two_processes_append_without_losing_a_line() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::copy_of_studio("commands")?;
    let ready = project.root().join("helper.ready");
    let mut helper = spawn_helper(&HelperTask::Append {
        root: project.root().to_path_buf(),
        path: SCRIPT.to_owned(),
        tag: "helper".to_owned(),
        count: LINES,
        ready: ready.clone(),
    })?;
    helper.wait_for(&ready)?;

    let mut workspace = Workspace::open(project.root())?;
    workspace.set_lock_wait(Duration::from_secs(10));
    append_lines(&mut workspace, SCRIPT, "test", LINES)?;

    let status = helper.wait()?;
    assert!(status.success(), "the helper exited with {status}");

    let text = std::fs::read_to_string(project.root().join(SCRIPT))?;
    Record::parse(&text)?;
    for tag in ["helper", "test"] {
        for n in 1..=LINES {
            let line = format!("{tag} {n}");
            let count = text.lines().filter(|l| *l == line).count();
            assert_eq!(count, 1, "the line {line:?} is in the record {count} times");
        }
    }
    let mut leftovers = Vec::new();
    find_tmp(project.root(), &mut leftovers)?;
    assert!(
        leftovers.is_empty(),
        "temporary files are left: {leftovers:?}"
    );
    Ok(())
}

/// Collects every `.theos-*.tmp` file under `dir`.
fn find_tmp(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            find_tmp(&path, found)?;
        } else {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".theos-")
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp"))
            {
                found.push(path);
            }
        }
    }
    Ok(())
}
