//! The `ui` check: the checks of `editor-ui/` (Svelte 5, TypeScript, pnpm): install, type check,
//! lint, unit tests and end-to-end tests, in that order.
//!
//! [`plan`] decides what to run from the repository alone: no `editor-ui/package.json` is not
//! needed, otherwise five [`Step`]s. [`execute`] runs them through a runner it is given, so the
//! tests need no Node, and every step runs even after one has failed, so one run shows all that
//! is wrong. [`run`] wires in `std::process::Command`, with `pnpm` resolved through `which`
//! (on Windows it is `pnpm.cmd`). Its first findings are the probe of node and pnpm
//! ([`crate::ui_tools`]): only a job that runs `ui` needs them, so the `tools` check leaves them
//! alone.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

use crate::Result;
use crate::ci::CiEvent;
use crate::report;
use crate::scope::{self, Scope};
use crate::ui_tools;
use crate::verdict::{Check, Finding, Verdict};

/// The folder of the UI, relative to the repository root.
const UI_DIR: &str = "editor-ui";

/// How many lines of a failed step's output a finding shows.
const TAIL_LINES: usize = 40;

/// One command of the check, run in `editor-ui/`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Step {
    /// The name in findings: `install`, `svelte-check`, `eslint`, `vitest` or `playwright`.
    pub name: &'static str,
    /// The program, resolved through the search path when it runs.
    pub program: &'static str,
    /// Its arguments.
    pub args: Vec<String>,
}

/// What the check has to do in a repository.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UiPlan {
    /// There is nothing to check; the reason is the one the verdict shows.
    NotNeeded(&'static str),
    /// The steps to run, in order.
    Steps(Vec<Step>),
}

fn pnpm(name: &'static str, args: &[&str]) -> Step {
    Step {
        name,
        program: "pnpm",
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
    }
}

/// The plan for the repository at `root`: [`UiPlan::NotNeeded`] without `editor-ui/package.json`,
/// else the five steps.
#[must_use]
pub fn plan(root: &Path) -> UiPlan {
    if !root.join(UI_DIR).join("package.json").is_file() {
        return UiPlan::NotNeeded("no editor-ui yet");
    }
    UiPlan::Steps(vec![
        pnpm("install", &["install", "--frozen-lockfile"]),
        pnpm(
            "svelte-check",
            &["exec", "svelte-check", "--fail-on-warnings"],
        ),
        pnpm("eslint", &["exec", "eslint", "."]),
        pnpm("vitest", &["exec", "vitest", "run"]),
        pnpm("playwright", &["exec", "playwright", "test"]),
    ])
}

/// The last `lines` lines of `text`, without a final line break.
#[must_use]
pub fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// The stdout and stderr of a step as one text.
fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Runs every step through `runner`, even after one has failed. A step that exits non-zero is one
/// finding with the last 40 lines of its output; a step that cannot start is one with the reason.
pub fn execute(
    steps: &[Step],
    mut runner: impl FnMut(&Step) -> std::io::Result<Output>,
) -> Verdict {
    let mut findings = Vec::new();
    for step in steps {
        let command = format!("{} {}", step.program, step.args.join(" "));
        match runner(step) {
            Ok(output) if output.status.success() => {}
            Ok(output) => findings.push(Finding::new(format!(
                "`{command}` ({}) failed:\n{}",
                step.name,
                tail(&combined(&output), TAIL_LINES)
            ))),
            Err(error) => findings.push(Finding::new(format!(
                "`{command}` ({}) could not run: {error}",
                step.name
            ))),
        }
    }
    Verdict::with(Check::Ui, findings)
}

/// `verdict` with the findings of the probe of node and pnpm in front of the findings of the
/// steps.
#[must_use]
pub fn with_probe(probe: Vec<Finding>, mut verdict: Verdict) -> Verdict {
    verdict.findings.splice(0..0, probe);
    verdict
}

/// The verdict of `ui` in `root` under `scope`: not needed (with the reason of the gate) when the
/// scope does not run it, or when there is no `editor-ui/`; else the probe of node and pnpm, then
/// the steps. Neither `probe` nor `runner` is called when the check is not needed, so such a run
/// needs no Node and starts no pnpm.
#[must_use]
pub fn verdict_under(
    scope: &Scope,
    root: &Path,
    probe: impl FnOnce() -> Vec<Finding>,
    runner: impl FnMut(&Step) -> std::io::Result<Output>,
) -> Verdict {
    if !scope::runs(scope, Check::Ui) {
        return Verdict::not_needed(Check::Ui, scope::NOT_NEEDED_REASON);
    }
    match plan(root) {
        UiPlan::NotNeeded(reason) => Verdict::not_needed(Check::Ui, reason),
        UiPlan::Steps(steps) => with_probe(probe(), execute(&steps, runner)),
    }
}

/// Runs the steps of the `ui` plan as real `pnpm` processes in `editor-ui/`.
fn run_step(root: &Path, step: &Step) -> std::io::Result<Output> {
    let program = which::which(step.program)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::NotFound, error))?;
    Command::new(program)
        .args(&step.args)
        .current_dir(root.join(UI_DIR))
        .output()
}

/// The `ui` check in `root`, for the gate, which has applied the scope already: the probe of node
/// and pnpm, then the steps. `_ci` and `_out` are not used.
///
/// # Errors
///
/// Never fails today: a step that does not run is a finding. The signature is that of every
/// check.
pub fn run(root: &Path, _ci: Option<&CiEvent>, _out: &mut dyn Write) -> Result<Verdict> {
    let every = Scope::Every { note: None };
    Ok(verdict_under(
        &every,
        root,
        || ui_tools::check(root),
        |step| run_step(root, step),
    ))
}

/// The `ui` check as its own subcommand, `cargo xtask ui`: the scope of the run decides first, as
/// it does for `cargo xtask gate`, so a pull request that changes no code gets the verdict "not
/// needed" and the job summary lists it. Returns the verdict and why every check runs, when the
/// scope says so; the note goes to `out` as the gate prints it, and the summary shows it.
///
/// # Errors
///
/// Fails only when `out` cannot be written.
pub fn run_scoped(
    root: &Path,
    ci: Option<&CiEvent>,
    out: &mut dyn Write,
) -> Result<(Verdict, Option<String>)> {
    let scope = scope::for_run(root, ci);
    let every_because = match &scope {
        Scope::Every { note } => note.clone(),
        Scope::DocsOnly => None,
    };
    if let Some(note) = &every_because {
        report::write_note(out, &format!("every check runs: {note}"))
            .map_err(report::output_error)?;
    }
    let verdict = verdict_under(
        &scope,
        root,
        || ui_tools::check(root),
        |step| run_step(root, step),
    );
    Ok((verdict, every_because))
}
