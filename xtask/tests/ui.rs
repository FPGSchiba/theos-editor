//! The `ui` check: its plan of steps, and how the verdict is made from their results. The tests
//! inject the runner, so they need no Node.

use std::error::Error;
use std::fs;
use std::process::{ExitStatus, Output};

use xtask::checks::ui::{self, Step, UiPlan};
use xtask::verdict::Check;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn exit_status(success: bool) -> ExitStatus {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(u32::from(!success))
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(i32::from(!success) << 8)
    }
}

fn output(success: bool, text: &str) -> Output {
    Output {
        status: exit_status(success),
        stdout: text.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

fn root_with_editor_ui() -> TestResult<tempfile::TempDir> {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("editor-ui"))?;
    fs::write(root.path().join("editor-ui").join("package.json"), "{}")?;
    Ok(root)
}

#[test]
fn not_needed_without_editor_ui() -> TestResult {
    let root = tempfile::tempdir()?;
    assert_eq!(ui::plan(root.path()), UiPlan::NotNeeded("no editor-ui yet"));
    Ok(())
}

#[test]
fn steps_in_order() -> TestResult {
    let root = root_with_editor_ui()?;
    let UiPlan::Steps(steps) = ui::plan(root.path()) else {
        return Err("expected steps".into());
    };
    let found: Vec<(&str, &str, Vec<String>)> = steps
        .iter()
        .map(|step| (step.name, step.program, step.args.clone()))
        .collect();
    let expect = |name, args: &[&str]| {
        (
            name,
            "pnpm",
            args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
        )
    };
    assert_eq!(
        found,
        [
            expect("install", &["install", "--frozen-lockfile"]),
            expect(
                "svelte-check",
                &["exec", "svelte-check", "--fail-on-warnings"]
            ),
            expect("eslint", &["exec", "eslint", "."]),
            expect("vitest", &["exec", "vitest", "run"]),
            expect("playwright", &["exec", "playwright", "test"]),
        ]
    );
    Ok(())
}

#[test]
fn every_step_runs_and_each_failure_is_a_finding() -> TestResult {
    let root = root_with_editor_ui()?;
    let UiPlan::Steps(steps) = ui::plan(root.path()) else {
        return Err("expected steps".into());
    };
    let mut seen = Vec::new();
    let verdict = ui::execute(&steps, |step: &Step| {
        seen.push(step.name);
        let failed = matches!(step.name, "eslint" | "vitest");
        Ok(output(!failed, &format!("output of {}\n", step.name)))
    });
    assert_eq!(
        seen,
        ["install", "svelte-check", "eslint", "vitest", "playwright"]
    );
    assert_eq!(verdict.check, Check::Ui);
    assert_eq!(verdict.findings.len(), 2);
    assert!(verdict.findings[0].message.contains("eslint"));
    assert!(verdict.findings[0].message.contains("output of eslint"));
    assert!(verdict.findings[1].message.contains("vitest"));
    Ok(())
}

#[test]
fn a_failure_shows_the_last_forty_lines_only() {
    let lines: Vec<String> = (1..=100).map(|n| format!("line {n}")).collect();
    let text = lines.join("\n") + "\n";
    let tail = ui::tail(&text, 40);
    assert!(tail.starts_with("line 61\n"), "{tail}");
    assert!(tail.ends_with("line 100"), "{tail}");
}

#[test]
fn a_step_that_cannot_start_is_a_finding() -> TestResult {
    let root = root_with_editor_ui()?;
    let UiPlan::Steps(steps) = ui::plan(root.path()) else {
        return Err("expected steps".into());
    };
    let verdict = ui::execute(&steps, |_| {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no pnpm"))
    });
    assert_eq!(verdict.findings.len(), 5);
    assert!(verdict.findings[0].message.contains("no pnpm"));
    Ok(())
}

#[test]
fn check_name_round_trips() {
    assert_eq!(Check::from_name("ui"), Some(Check::Ui));
    assert_eq!(Check::Ui.name(), "ui");
}
