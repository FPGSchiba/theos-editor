//! `theos ledger`.

use serde_json::Value;

use crate::common::{Sandbox, TestResult, content_project, stderr, stdout};

// AC: CE-15.3
#[test]
fn theos_ledger_prints_the_ledger() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("lore"), "lore")?;
    let output = sandbox.run(
        &["ledger", project.to_str().ok_or("path")?],
        &sandbox.work(),
    )?;
    assert!(output.status.success(), "{output:?}");
    let expected = std::fs::read_to_string(content_project("lore").join("expected-ledger.txt"))?;
    assert_eq!(stdout(&output), expected);
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_ledger_json_is_the_ledger() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("lore"), "lore")?;
    let output = sandbox.run(&["--json", "ledger"], &project)?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let actual: Value = serde_json::from_str(&stdout(&output))?;
    let expected: toml::Table = toml::from_str(&std::fs::read_to_string(
        content_project("lore").join("expected-ledger.toml"),
    )?)?;
    // Compare as JSON values: the TOML file's `truth` and `entry` arrays are the view's arrays,
    // and a value the TOML file leaves out is `null` in the view.
    let expected: Value = serde_json::to_value(&expected)?;
    let truths = expected["truth"].as_array().ok_or("truth array")?;
    let actual = actual.as_array().ok_or("ledger array")?;
    assert_eq!(actual.len(), truths.len());
    for (view, truth) in actual.iter().zip(truths) {
        for key in ["id", "path", "title", "target"] {
            assert_eq!(view[key], truth[key], "{key}");
        }
        // A truth without a clue has no `entry` table in the TOML file.
        let entries = truth.get("entry").map_or(Ok(&[][..]), |e| {
            e.as_array().map(Vec::as_slice).ok_or("entry array")
        })?;
        let view_entries = view["entries"].as_array().ok_or("entries array")?;
        assert_eq!(view_entries.len(), entries.len());
        for (view, entry) in view_entries.iter().zip(entries) {
            assert_eq!(view.as_object().ok_or("entry object")?.len(), 7, "{view}");
            for key in [
                "state",
                "floor",
                "place",
                "gate",
                "clue",
                "clue_path",
                "placed_by",
            ] {
                let want = entry.get(key).unwrap_or(&Value::Null);
                assert_eq!(&view[key], want, "{key}");
            }
        }
    }
    Ok(())
}
