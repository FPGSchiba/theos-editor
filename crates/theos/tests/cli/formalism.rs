//! `theos formalism`, and the exit code of a folder that is no project.

use serde_json::Value;

use crate::common::{Sandbox, TestResult, content_project, stderr, stdout};

#[test]
fn theos_formalism_writes_the_mirror_once() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("formalism"), "formalism")?;
    let first = sandbox.run(&["formalism", project.to_str().ok_or("path")?], &project)?;
    assert!(first.status.success(), "{first:?}");
    let first = stdout(&first);
    assert_eq!(first.lines().count(), 18, "{first}");
    let second = sandbox.run(&["formalism", project.to_str().ok_or("path")?], &project)?;
    assert!(second.status.success(), "{second:?}");
    assert_eq!(stdout(&second), "nothing to write\n");
    Ok(())
}

#[test]
fn theos_formalism_json_lists_the_written_paths() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("formalism"), "formalism")?;
    let first = sandbox.run(&["formalism", "--json"], &project)?;
    assert!(first.status.success(), "{first:?}");
    let value: Value = serde_json::from_str(&stdout(&first))?;
    assert_eq!(value["written"].as_array().ok_or("written")?.len(), 18);
    let second = sandbox.run(&["--json", "formalism"], &project)?;
    let value: Value = serde_json::from_str(&stdout(&second))?;
    assert_eq!(value["written"], Value::Array(Vec::new()));
    Ok(())
}

#[test]
fn a_folder_without_theos_toml_exits_with_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let empty = tempfile::tempdir()?;
    for command in ["ledger", "formalism"] {
        let output = sandbox.run(
            &[command, empty.path().to_str().ok_or("path")?],
            empty.path(),
        )?;
        assert_eq!(output.status.code(), Some(2), "{command}: {output:?}");
        assert_eq!(output.stdout, Vec::<u8>::new(), "{command}");
        assert!(stderr(&output).starts_with("error: "), "{command}");
    }
    Ok(())
}

#[test]
fn a_source_error_is_printed_once() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("formalism"), "formalism")?;
    let source = project.join("canon").join("formalism").join("_source.json");
    std::fs::write(&source, "{ not json")?;
    let output = sandbox.run(&["formalism"], &project)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let message = stderr(&output);
    // The last cause of the chain is the JSON parser's text; it must not repeat.
    let detail = message.trim().rsplit(": ").next().ok_or("a message")?;
    assert_eq!(message.matches(detail).count(), 1, "{message}");
    Ok(())
}

#[test]
fn theos_formalism_finds_the_project_above_the_current_folder() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("formalism"), "formalism")?;
    let sub = project.join("canon");
    let output = sandbox.run(&["formalism"], &sub)?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    assert_eq!(stdout(&output).lines().count(), 18);
    assert!(
        project.join("canon").join("formalism").is_dir(),
        "written in the project, not in the subfolder"
    );
    Ok(())
}
