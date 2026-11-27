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
fn the_probe_findings_come_before_the_step_findings() -> TestResult {
    let root = root_with_editor_ui()?;
    let UiPlan::Steps(steps) = ui::plan(root.path()) else {
        return Err("expected steps".into());
    };
    let steps = ui::execute(&steps, |_| Ok(output(false, "boom")));
    let probe = xtask::ui_tools::judge("{}", Some("v22.0.0"), Some("1.0.0"));
    assert_eq!(probe.len(), 2, "wrong node and no packageManager pin");
    let verdict = ui::with_probe(probe, steps);
    assert_eq!(verdict.findings.len(), 7);
    assert!(
        verdict.findings[0]
            .message
            .contains("node must be major version 24")
    );
    assert!(verdict.findings[1].message.contains("packageManager"));
    assert!(verdict.findings[2].message.contains("install"));
    assert!(!verdict.passed());
    Ok(())
}

// AC: CE-08C.2
#[test]
fn a_docs_only_scope_is_not_needed_and_runs_nothing() -> TestResult {
    use xtask::scope::{NOT_NEEDED_REASON, Scope};
    // With an `editor-ui/` that would otherwise run its steps: nothing is probed and no pnpm
    // starts, so the run needs no Node.
    let root = root_with_editor_ui()?;
    let verdict = ui::verdict_under(
        &Scope::DocsOnly,
        root.path(),
        || panic!("the probe ran for a change that touches no code"),
        |_| panic!("a step ran for a change that touches no code"),
    );
    assert!(verdict.passed());
    assert_eq!(verdict.not_needed.as_deref(), Some(NOT_NEEDED_REASON));
    assert_eq!(verdict.check, Check::Ui);

    // Any other scope runs the steps; without `editor-ui/` it is not needed for its own reason.
    let every = Scope::Every { note: None };
    let mut ran = 0;
    let verdict = ui::verdict_under(&every, root.path(), Vec::new, |_| {
        ran += 1;
        Ok(output(true, ""))
    });
    assert_eq!(ran, 5);
    assert!(verdict.passed() && verdict.not_needed.is_none());
    let empty = tempfile::tempdir()?;
    let verdict = ui::verdict_under(&every, empty.path(), Vec::new, |_| {
        panic!("a step ran without editor-ui")
    });
    assert_eq!(verdict.not_needed.as_deref(), Some("no editor-ui yet"));
    Ok(())
}

#[test]
fn check_name_round_trips() {
    assert_eq!(Check::from_name("ui"), Some(Check::Ui));
    assert_eq!(Check::Ui.name(), "ui");
}

// AC: CE-08C.2
#[test]
fn the_summary_says_why_every_check_runs() -> TestResult {
    // A push runs every check for a reason other than a change to code; `gate.md` §1 has the job
    // summary say why above the table.
    let root = tempfile::tempdir()?;
    let event = root.path().join("event.json");
    fs::write(&event, r#"{"before":"a","after":"b"}"#)?;
    let summary = root.path().join("summary.md");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("ui")
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_EVENT_NAME", "push")
        .env("GITHUB_EVENT_PATH", &event)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    let text = fs::read_to_string(&summary)?;
    assert!(
        text.contains(
            "Why every check ran: this is a push, which has no pull request to narrow by"
        ),
        "{text}"
    );
    Ok(())
}
