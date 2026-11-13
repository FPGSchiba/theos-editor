//! `theos schema`.

use serde_json::Value;
use theos_content::{parts, rules};

use crate::common::{Sandbox, TestResult, content_project, stderr, stdout};

/// The lines of `text` with every run of whitespace collapsed to one space.
fn collapsed(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn names(value: &Value, key: &str, field: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item[field].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

// AC: CE-17.1
#[test]
fn theos_schema_lists_every_kind_and_rule() -> TestResult {
    let sandbox = Sandbox::new()?;
    let registry = parts::builtin()?;

    let output = sandbox.run(&["--json", "schema"], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let view: Value = serde_json::from_str(&stdout(&output))?;
    let kinds: Vec<String> = registry.kinds().map(|spec| spec.name.to_owned()).collect();
    assert_eq!(names(&view, "kinds", "name"), kinds);
    let ids: Vec<String> = rules::all(&registry)
        .iter()
        .map(|rule| rule.id.to_owned())
        .collect();
    assert_eq!(names(&view, "rules", "id"), ids);
    for rule in view["rules"].as_array().ok_or("rules")? {
        assert!(rule["configured"].is_null(), "{rule}");
    }

    let output = sandbox.run(&["schema"], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let lines = collapsed(&stdout(&output));
    assert!(
        lines.iter().any(|line| line.starts_with("truth ")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("lore/never-confirm ")),
        "{lines:?}"
    );
    assert!(
        lines.iter().all(|line| !line.contains("configured")),
        "{lines:?}"
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_schema_shows_the_configured_severities() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("settings"), "settings")?;
    let output = sandbox.run(&["--json", "schema"], &project)?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let view: Value = serde_json::from_str(&stdout(&output))?;
    let rules = view["rules"].as_array().ok_or("rules")?;
    assert!(
        rules
            .iter()
            .any(|rule| rule["class"] == "configurable" && rule["configured"] != rule["severity"]),
        "{rules:?}"
    );
    let guardrails: Vec<&Value> = rules
        .iter()
        .filter(|rule| rule["class"] == "guardrail")
        .collect();
    assert!(guardrails.len() >= 3, "{guardrails:?}");
    for rule in guardrails {
        assert_eq!(rule["configured"], "error", "{rule}");
    }

    let output = sandbox.run(&["schema"], &project)?;
    let lines = collapsed(&stdout(&output));
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("lore/never-confirm ")
                && line.ends_with("configured error")),
        "{lines:?}"
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_schema_of_a_kind_lists_fields_and_body() -> TestResult {
    let sandbox = Sandbox::new()?;
    let output = sandbox.run(&["--json", "schema", "truth"], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let truth: Value = serde_json::from_str(&stdout(&output))?;
    assert_eq!(truth["name"], "truth");
    assert_eq!(
        truth["placement"],
        serde_json::json!({ "folder": "lore/truths" })
    );
    let registry = parts::builtin()?;
    let spec = registry.kind("truth").ok_or("truth")?;
    let keys: Vec<String> = spec.fields.iter().map(|f| f.key.to_owned()).collect();
    assert_eq!(names(&truth, "fields", "key"), keys);
    for (view, field) in truth["fields"]
        .as_array()
        .ok_or("fields")?
        .iter()
        .zip(&spec.fields)
    {
        assert_eq!(view["required"], field.required);
        assert_eq!(view["type"], field.ty.to_string());
    }
    assert!(truth["body"]["beats"].is_null());

    let output = sandbox.run(&["--json", "schema", "script"], &sandbox.work())?;
    let script: Value = serde_json::from_str(&stdout(&output))?;
    let beats = script["body"]["beats"].as_array().ok_or("beats")?;
    assert!(beats.len() >= 2, "{beats:?}");
    assert!(
        beats
            .iter()
            .all(|beat| beat["targets"].as_array().is_some_and(|t| !t.is_empty()))
    );

    let output = sandbox.run(&["schema", "script"], &sandbox.work())?;
    let lines = collapsed(&stdout(&output));
    for prefix in ["id:", "placement:", "fields:", "body:"] {
        assert!(
            lines.iter().any(|line| line.starts_with(prefix)),
            "{prefix} {lines:?}"
        );
    }
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_schema_of_a_kind_names_its_note_targets() -> TestResult {
    let sandbox = Sandbox::new()?;
    let registry = parts::builtin()?;
    let spec = registry.kind("concept").ok_or("concept")?;
    let notes: Vec<String> = spec
        .body
        .notes
        .ok_or("concept has no notes")?
        .iter()
        .map(ToString::to_string)
        .collect();
    let output = sandbox.run(&["schema", "concept"], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let text = stdout(&output);
    let line = format!("  notes link to: {}", notes.join(", "));
    assert!(text.lines().any(|found| found == line), "{line} in {text}");

    let output = sandbox.run(&["--json", "schema", "concept"], &sandbox.work())?;
    let view: Value = serde_json::from_str(&stdout(&output))?;
    assert_eq!(view["body"]["notes"], serde_json::json!(notes));
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_schema_project_names_the_project_folder() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("settings"), "settings")?;
    let path = project.to_str().ok_or("the project path is not UTF-8")?;
    let output = sandbox.run(&["--json", "schema", "--project", path], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));
    let view: Value = serde_json::from_str(&stdout(&output))?;
    let rules = view["rules"].as_array().ok_or("rules")?;
    assert!(rules.iter().all(|rule| rule["configured"].is_string()));
    let output = sandbox.run(&["schema", "truth", "--project", path], &sandbox.work())?;
    assert!(output.status.success(), "{output:?} {}", stderr(&output));

    // A folder without theos.toml is no project, also when one lies above it, with and
    // without a kind.
    let inner = project.join("lore");
    std::fs::create_dir_all(&inner)?;
    for folder in [inner, sandbox.work()] {
        let folder = folder.to_str().ok_or("the folder path is not UTF-8")?;
        for args in [
            ["schema", "--project", folder].as_slice(),
            ["schema", "truth", "--project", folder].as_slice(),
        ] {
            let output = sandbox.run(args, &sandbox.work())?;
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            assert_eq!(stdout(&output), "");
            assert!(
                stderr(&output).contains("is not a content project"),
                "{}",
                stderr(&output)
            );
        }
    }
    Ok(())
}

#[test]
fn theos_schema_of_an_unknown_kind_exits_2_and_lists_the_kinds() -> TestResult {
    let sandbox = Sandbox::new()?;
    let output = sandbox.run(&["schema", "nope"], &sandbox.work())?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(stdout(&output), "");
    let error = stderr(&output);
    assert!(
        error.contains("unknown kind `nope`; the kinds are: "),
        "{error}"
    );
    assert!(error.contains("truth"), "{error}");
    Ok(())
}
