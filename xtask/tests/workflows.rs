//! The shape of the GitHub Actions workflows and of the setup action under `.github/`.
//!
//! CI is thin over xtask (P00 spec D1): a job checks out, sets up and runs `cargo xtask <check>`,
//! and the workflow files hold triggers, runners, caching and permissions and nothing else. These
//! tests parse the real files, so a change that breaks a rule of `docs/gate.md` §1 or of the
//! spec's §5.6 fails here, before it fails on GitHub.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use xtask::gate::{PR_CHECKS, Selection};
use xtask::github::settings::required_contexts_from_gate_md;
use xtask::markdown;
use xtask::verdict::{Check, GATE_ORDER, PR_ONLY};
use yaml_rust2::{Yaml, YamlLoader};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The workflows whose jobs are the required status checks of `main`.
const PR_WORKFLOWS: [&str; 3] = ["ci.yml", "pr.yml", "approval.yml"];

/// Every workflow file.
const ALL_WORKFLOWS: [&str; 5] = [
    "ci.yml",
    "pr.yml",
    "approval.yml",
    "platforms.yml",
    "settings.yml",
];

const SETUP_ACTION: &str = ".github/actions/setup/action.yml";

/// The step that runs the setup action, as a job names it.
const SETUP_USES: &str = "./.github/actions/setup";

const XTASK_ACTION: &str = ".github/actions/xtask/action.yml";

/// The step that runs the xtask action: the binary `main` built and cached, or a fresh build.
const XTASK_USES: &str = "./.github/actions/xtask";

/// The binary the xtask action leaves behind, which `pr`, `approval` and `settings` run.
const XTASK_BIN: &str = "target/xtask-bin/debug/xtask";

/// The condition of the one step of the xtask action that has one: build only on a cache miss.
const BUILD_ON_MISS_IF: &str = "steps.restore.outputs.cache-hit != 'true'";

/// The expression that is the workflow token.
const TOKEN: &str = "${{ github.token }}";

fn root() -> TestResult<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent folder")?
        .to_path_buf())
}

/// The text of a file of the repository, given by its path from the root with `/`.
fn read(relative: &str) -> TestResult<String> {
    Ok(fs::read_to_string(root()?.join(relative))?)
}

/// The one YAML document of a file of the repository.
fn parse(relative: &str) -> TestResult<Yaml> {
    let mut documents = YamlLoader::load_from_str(&read(relative)?)?;
    if documents.len() != 1 {
        return Err(format!("{relative} has {} YAML documents, not one", documents.len()).into());
    }
    Ok(documents.remove(0))
}

fn workflow_path(file: &str) -> String {
    format!(".github/workflows/{file}")
}

fn workflow(file: &str) -> TestResult<Yaml> {
    parse(&workflow_path(file))
}

/// A job of a workflow: its key, its name (which is its status check) and its definition.
struct Job {
    id: String,
    name: String,
    node: Yaml,
}

impl Job {
    fn runs_on(&self) -> TestResult<&str> {
        self.node["runs-on"]
            .as_str()
            .ok_or_else(|| format!("job {} has no runs-on", self.id).into())
    }

    fn steps(&self) -> TestResult<&[Yaml]> {
        self.node["steps"]
            .as_vec()
            .map(Vec::as_slice)
            .ok_or_else(|| format!("job {} has no steps", self.id).into())
    }

    /// The `run:` values of the job, one per `run:` step.
    fn commands(&self) -> TestResult<Vec<String>> {
        Ok(run_values(self.steps()?))
    }

    /// The `cargo xtask gate` command of the job: its last `run:` step.
    fn gate_command(&self) -> TestResult<String> {
        self.commands()?
            .pop()
            .ok_or_else(|| format!("job {} runs no command", self.name).into())
    }
}

fn jobs(workflow: &Yaml) -> TestResult<Vec<Job>> {
    let table = workflow["jobs"]
        .as_hash()
        .ok_or("the workflow has no jobs")?;
    table
        .iter()
        .map(|(id, node)| -> TestResult<Job> {
            let id = id.as_str().ok_or("a job key is not a string")?.to_owned();
            let name = node["name"]
                .as_str()
                .ok_or_else(|| format!("job {id} has no name"))?
                .to_owned();
            Ok(Job {
                id,
                name,
                node: node.clone(),
            })
        })
        .collect()
}

fn index_of(jobs: &[Job], name: &str) -> TestResult<usize> {
    jobs.iter()
        .position(|job| job.name == name)
        .ok_or_else(|| format!("no job named {name}").into())
}

/// Every job of every workflow, with the file it is in.
fn all_jobs() -> TestResult<Vec<(&'static str, Job)>> {
    let mut all = Vec::new();
    for file in ALL_WORKFLOWS {
        for job in jobs(&workflow(file)?)? {
            all.push((file, job));
        }
    }
    Ok(all)
}

fn run_values(steps: &[Yaml]) -> Vec<String> {
    steps
        .iter()
        .filter_map(|step| step["run"].as_str().map(str::to_owned))
        .collect()
}

/// A string, number or boolean as the text it was written as.
fn scalar(node: &Yaml) -> Option<String> {
    match node {
        Yaml::String(text) => Some(text.clone()),
        Yaml::Integer(number) => Some(number.to_string()),
        Yaml::Boolean(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn strings(node: &Yaml) -> TestResult<Vec<String>> {
    node.as_vec()
        .ok_or("not a list")?
        .iter()
        .map(|item| scalar(item).ok_or_else(|| "a list item is not a scalar".into()))
        .collect()
}

fn keys(node: &Yaml) -> TestResult<BTreeSet<String>> {
    node.as_hash()
        .ok_or("not a mapping")?
        .keys()
        .map(|key| scalar(key).ok_or_else(|| "a key is not a scalar".into()))
        .collect()
}

fn string_map(node: &Yaml) -> TestResult<BTreeMap<String, String>> {
    node.as_hash()
        .ok_or("not a mapping")?
        .iter()
        .map(|(key, value)| {
            let key = scalar(key).ok_or("a key is not a scalar")?;
            let value = scalar(value).ok_or("a value is not a scalar")?;
            Ok((key, value))
        })
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// One job of `platforms.yml`: one per OS, which runs its checks through `cargo xtask gate
/// --only`, so an OS is set up and built once.
struct PlatformJob {
    name: &'static str,
    runner: &'static str,
    command: &'static str,
    /// The commands that run before `command`: the UI's browser, for the jobs that run `ui`.
    prelude: &'static [&'static str],
    timeout: i64,
    /// The label of a pull request that starts the job, if any.
    label: Option<&'static str>,
    /// The `workflow_dispatch` input that starts the job.
    input: &'static str,
    /// The condition the job spells.
    condition: &'static str,
}

/// The jobs of `platforms.yml`.
const PLATFORM_JOBS: [PlatformJob; 3] = [
    PlatformJob {
        name: "windows",
        runner: "windows-latest",
        command: "cargo xtask gate --only clippy,test,slow-tests,lints,ac-trace,ui",
        prelude: &[NEEDS_UI_CARGO, UI_BROWSER_LOCAL],
        timeout: 150,
        label: Some("windows"),
        input: "windows",
        condition: "github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' \
                    && inputs.windows) || (github.event_name == 'pull_request' && \
                    github.event.label.name == 'windows')",
    },
    PlatformJob {
        name: "macos",
        runner: "macos-latest",
        command: "cargo xtask gate --only clippy,test,slow-tests,lints,ac-trace,ui",
        prelude: &[NEEDS_UI_CARGO, UI_BROWSER_LOCAL],
        timeout: 150,
        label: Some("macos"),
        input: "macos",
        condition: "github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' \
                    && inputs.macos) || (github.event_name == 'pull_request' && \
                    github.event.label.name == 'macos')",
    },
    PlatformJob {
        name: "macos-intel",
        runner: "macos-15-intel",
        command: "cargo xtask gate --only test,slow-tests",
        prelude: &[],
        timeout: 150,
        label: None,
        input: "macos_intel",
        condition: "github.event_name == 'workflow_dispatch' && inputs.macos_intel",
    },
];

/// What the jobs that run `ui` install before it: the packages and Chromium of Playwright.
/// `pnpm install` comes first because `pnpm exec playwright` needs the packages; `ui` installs
/// them again, which finds them there. The Linux job adds the system libraries of the browser.
const UI_BROWSER_LOCAL: &str = "pnpm --dir editor-ui install --frozen-lockfile && pnpm --dir \
                                editor-ui exec playwright install chromium";
const UI_BROWSER_CI: &str = "pnpm --dir editor-ui install --frozen-lockfile && pnpm --dir \
                             editor-ui exec playwright install --with-deps chromium";

/// The condition of every step that sets up Node, pnpm and the browser: the run needs `ui`
/// (`xtask needs ui`, which is the scope of `gate`) and `editor-ui/` exists, so a pull request
/// that changes no code pays nothing and the jobs stay green until the folder does.
const UI_EXISTS_IF: &str =
    "steps.needs.outputs.needed == 'true' && hashFiles('editor-ui/package.json') != ''";

/// The condition of the step of the `ui` job that runs the check: the run needs it.
const UI_RUN_IF: &str = "steps.needs.outputs.needed == 'true'";

/// The step that asks xtask whether the run needs `ui`, and leaves the answer as the step output
/// `needed`. The `ui` job asks the cached binary; the platform jobs have built xtask already.
const NEEDS_UI_CACHED: &str = "target/xtask-bin/debug/xtask --root . needs ui";
const NEEDS_UI_CARGO: &str = "cargo xtask needs ui";

/// The check of the `ui` job: the `ui` subcommand of the cached binary. Not `gate --only ui`,
/// because `gate` always runs `tools` first, which fails in a job that installed none of the
/// pinned Rust tools, and `cargo xtask` would rebuild xtask on a toolchain the job never set up.
const UI_CACHED: &str = "target/xtask-bin/debug/xtask --root . ui";

/// The `platforms.yml` job of this name in [`PLATFORM_JOBS`].
fn platform_job(name: &str) -> Option<&'static PlatformJob> {
    PLATFORM_JOBS.iter().find(|job| job.name == name)
}

/// The expression that is true on the schedule only: the one case where `platforms` saves a build
/// cache.
const ON_SCHEDULE: &str = "${{ github.event_name == 'schedule' }}";

/// The runner of a job of `file`.
fn runner_for(file: &str, name: &str) -> &'static str {
    if file == "platforms.yml" {
        platform_job(name).map_or("", |job| job.runner)
    } else {
        ci_job(name)
            .filter(|_| file == "ci.yml")
            .map_or("ubuntu-latest", |job| job.runner)
    }
}

/// One job of `ci.yml`: its name (the required status check), its runner, its one command and
/// its timeout in minutes.
struct CiJob {
    name: &'static str,
    runner: &'static str,
    command: &'static str,
    /// The commands that run before `command`.
    prelude: &'static [&'static str],
    timeout: i64,
}

/// The jobs of `ci.yml`, side by side: the gate without the two long checks and `ui`, and each of
/// those on a runner of its own.
const CI_JOBS: [CiJob; 4] = [
    CiJob {
        name: "gate",
        runner: "ubuntu-latest",
        command: "cargo xtask gate --skip test,coverage,ui",
        prelude: &[],
        timeout: 150,
    },
    CiJob {
        name: "test",
        runner: "ubuntu-latest",
        command: "cargo xtask gate --only test",
        prelude: &[],
        timeout: 60,
    },
    CiJob {
        name: "coverage",
        runner: "ubuntu-latest",
        command: "cargo xtask gate --only coverage",
        prelude: &[],
        timeout: 60,
    },
    CiJob {
        name: "ui",
        runner: "ubuntu-latest",
        command: UI_CACHED,
        prelude: &[NEEDS_UI_CACHED, UI_BROWSER_CI],
        timeout: 30,
    },
];

/// The jobs that build Rust with the Tauri crates, which need the Linux packages of
/// `gate/linux-packages.txt`, and the checks of each that build: the value of `gui-packages`,
/// which the setup action asks `xtask needs` about.
const GUI_JOBS: [(&str, &str); 3] = [
    ("gate", "clippy,docs,slow-tests"),
    ("test", "test"),
    ("coverage", "coverage"),
];

/// The value of the setup input `gui-packages` of a job: its checks that need the packages.
fn gui_packages_of(job: &str) -> Option<&'static str> {
    GUI_JOBS
        .iter()
        .find(|(name, _)| *name == job)
        .map(|(_, checks)| *checks)
}

/// The `ci.yml` job of this name in [`CI_JOBS`].
fn ci_job(name: &str) -> Option<&'static CiJob> {
    CI_JOBS.iter().find(|job| job.name == name)
}

/// The commands a job runs, in order: the prelude and the one gate command of [`CI_JOBS`] for
/// `ci.yml`, of [`PLATFORM_JOBS`] for `platforms.yml`, for `pr`, `approval` and `settings` the
/// cached xtask binary.
fn commands_of(file: &str, name: &str) -> Vec<String> {
    let with_prelude = |prelude: &[&str], command: &str| {
        prelude
            .iter()
            .chain([&command])
            .map(|command| (*command).to_owned())
            .collect()
    };
    if file == "ci.yml" {
        ci_job(name).map_or_else(Vec::new, |job| with_prelude(job.prelude, job.command))
    } else if file == "platforms.yml" {
        platform_job(name).map_or_else(Vec::new, |job| with_prelude(job.prelude, job.command))
    } else {
        cached_xtask_command(name).map_or_else(Vec::new, |command| vec![command.to_owned()])
    }
}

/// The command of a job that runs the xtask binary `main` cached, by the name of the job.
fn cached_xtask_command(name: &str) -> Option<&'static str> {
    match name {
        "pr" => Some("target/xtask-bin/debug/xtask --root . pr"),
        "approval" => Some("target/xtask-bin/debug/xtask --root . approval"),
        "settings" => Some("target/xtask-bin/debug/xtask --root . github check"),
        _ => None,
    }
}

/// The selection a `cargo xtask gate` command spells with `--only` and `--skip`, read from the
/// command itself so the test does not repeat what the workflow says.
fn selection_of(command: &str) -> TestResult<Selection> {
    if command == UI_CACHED {
        return Ok(Selection {
            only: vec![Check::Ui],
            skip: Vec::new(),
        });
    }
    let rest = command
        .strip_prefix("cargo xtask gate")
        .ok_or_else(|| format!("`{command}` is not `cargo xtask gate`"))?;
    let mut selection = Selection::default();
    let mut words = rest.split_whitespace();
    while let Some(flag) = words.next() {
        let value = words
            .next()
            .ok_or_else(|| format!("`{command}`: {flag} has no value"))?;
        let checks = value
            .split(',')
            .map(|name| {
                Check::from_name(name).ok_or_else(|| format!("`{command}`: {name} is no check"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match flag {
            "--only" => selection.only.extend(checks),
            "--skip" => selection.skip.extend(checks),
            _ => return Err(format!("`{command}`: unknown flag {flag}").into()),
        }
    }
    Ok(selection)
}

/// The checks the `ci.yml` jobs run between them, `tools` (which every job runs) left out.
fn ci_checks() -> TestResult<Vec<Check>> {
    let mut checks = Vec::new();
    for job in jobs(&workflow("ci.yml")?)? {
        checks.extend(
            selection_of(&job.gate_command()?)?
                .checks()
                .into_iter()
                .filter(|check| *check != Check::Tools),
        );
    }
    Ok(checks)
}

/// What a step is, for the shape of a job: the checkout, the setup action, the cache, or a command.
fn step_kind(step: &Yaml) -> TestResult<&'static str> {
    if let Some(uses) = step["uses"].as_str() {
        return if uses.starts_with("actions/checkout@") {
            Ok("checkout")
        } else if uses == SETUP_USES {
            Ok("setup")
        } else if uses == XTASK_USES {
            Ok("xtask")
        } else if uses.starts_with("pnpm/action-setup@") {
            Ok("pnpm")
        } else if uses.starts_with("actions/setup-node@") {
            Ok("node")
        } else if uses.starts_with("actions/cache@") {
            Ok("cache")
        } else if uses.starts_with("actions/cache/save@") {
            Ok("cache-save")
        } else if uses.starts_with("actions/cache/restore@") {
            Ok("cache-restore")
        } else {
            Err(format!("a step uses {uses}, which no job needs").into())
        };
    }
    if step["run"].as_str().is_some() {
        Ok("run")
    } else {
        Err("a step neither uses an action nor runs a command".into())
    }
}

// AC: CE-08.1
#[test]
fn ci_builds_and_tests_on_linux() -> TestResult {
    let checks = ci_checks()?;
    for check in [Check::Clippy, Check::Test, Check::SlowTests] {
        assert!(checks.contains(&check), "no ci job runs {}", check.name());
    }
    for job in jobs(&workflow("ci.yml")?)? {
        assert_eq!(job.runs_on()?, "ubuntu-latest", "{}", job.name);
    }
    Ok(())
}

// AC: CE-08C.1
#[test]
fn ci_runs_four_jobs_side_by_side() -> TestResult {
    let jobs = jobs(&workflow("ci.yml")?)?;
    let found: Vec<(&str, &str)> = jobs
        .iter()
        .map(|job| (job.id.as_str(), job.name.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("gate", "gate"),
            ("test", "test"),
            ("coverage", "coverage"),
            ("ui", "ui")
        ]
    );
    for job in &jobs {
        let expected = ci_job(&job.name).ok_or("an unexpected job")?;
        assert_eq!(job.runs_on()?, expected.runner, "{}", job.name);
        assert_eq!(
            job.commands()?,
            commands_of("ci.yml", &job.name),
            "{}",
            job.name
        );
        assert_eq!(job.gate_command()?, expected.command, "{}", job.name);
        // Side by side: no job waits for another, so the wall time is the slowest job.
        assert_eq!(job.node["needs"], Yaml::BadValue, "{} has needs", job.name);
    }
    Ok(())
}

// AC: CE-08.4
#[test]
fn ci_jobs_run_the_gate_runner() -> TestResult {
    // Every `ci` command is `cargo xtask gate` with `--only` or `--skip`, and between them the
    // jobs run every check of the gate once.
    let mut checks = ci_checks()?;
    checks.sort_by_key(|check| GATE_ORDER.iter().position(|other| other == check));
    let gate: Vec<Check> = GATE_ORDER
        .iter()
        .copied()
        .filter(|check| *check != Check::Tools)
        .collect();
    assert_eq!(
        checks, gate,
        "the ci jobs and cargo xtask gate run different checks, or one twice"
    );

    // The pull-request-only checks have their jobs in the other workflows: `pr` runs the title
    // and the body, `approval` the approval.
    let mut pr_jobs = BTreeSet::new();
    for file in ["pr.yml", "approval.yml"] {
        for job in jobs(&workflow(file)?)? {
            pr_jobs.insert(job.name);
        }
    }
    for pr_only in PR_ONLY {
        let job = if pr_only == Check::Approval {
            "approval"
        } else {
            "pr"
        };
        assert!(
            pr_jobs.contains(job),
            "{} is a PR-only check without a job",
            pr_only.name()
        );
    }
    Ok(())
}

// AC: CE-08.1
#[test]
fn platforms_builds_and_tests_on_windows_and_macos() -> TestResult {
    // The Windows and the Apple silicon job build with clippy and run the tests and the slow
    // tests; each runs them in the one gate command of its job.
    let jobs = jobs(&workflow("platforms.yml")?)?;
    for (name, runner) in [("windows", "windows-latest"), ("macos", "macos-latest")] {
        let job = &jobs[index_of(&jobs, name)?];
        assert_eq!(job.runs_on()?, runner, "{name}");
        let checks = selection_of(&job.gate_command()?)?.checks();
        for check in [Check::Clippy, Check::Test, Check::SlowTests] {
            assert!(
                checks.contains(&check),
                "{name} does not run {}",
                check.name()
            );
        }
    }
    Ok(())
}

// AC: CE-08C.6
#[test]
fn platforms_runs_one_job_per_os() -> TestResult {
    let jobs = jobs(&workflow("platforms.yml")?)?;
    let found: Vec<(&str, &str)> = jobs
        .iter()
        .map(|job| (job.id.as_str(), job.name.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("windows", "windows"),
            ("macos", "macos"),
            ("macos-intel", "macos-intel")
        ]
    );
    for job in &jobs {
        let expected = platform_job(&job.name).ok_or("an unexpected job")?;
        assert_eq!(job.runs_on()?, expected.runner, "{}", job.name);
        assert_eq!(
            job.commands()?,
            commands_of("platforms.yml", &job.name),
            "{}",
            job.name
        );
        assert_eq!(job.node["needs"], Yaml::BadValue, "{} has needs", job.name);
    }
    Ok(())
}

/// What starts a run of `platforms`: the event, the label of a `labeled` pull request, and the
/// `workflow_dispatch` inputs that are on.
struct Event {
    name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
}

/// Whether a job runs for an event, by its condition. Only the conditions of [`PLATFORM_JOBS`]
/// are known, and an unknown one fails the test, so a changed condition is a changed test.
fn runs(job: &PlatformJob, condition: &str, event: &Event) -> TestResult<bool> {
    if condition != job.condition {
        return Err(format!("job {} has the unknown condition `{condition}`", job.name).into());
    }
    let on_demand = event.name == "workflow_dispatch" && event.inputs.contains(&job.input);
    Ok(match job.label {
        // The scheduled run, the manual run that asks for it, and its own label.
        Some(label) => {
            event.name == "schedule"
                || on_demand
                || (event.name == "pull_request" && event.label == label)
        }
        // Only on demand: the Intel runner is dear and its tests rarely differ.
        None => on_demand,
    })
}

/// The condition of a job as the workflow spells it.
fn condition_of(job: &Job) -> TestResult<&str> {
    job.node["if"]
        .as_str()
        .ok_or_else(|| format!("job {} has no `if:`", job.name).into())
}

/// What follows each occurrence of `marker` in `text` up to the closing `quote` or the end of the
/// identifier: the labels after `label.name == '` and the inputs after `inputs.`.
fn after_marker(text: &str, marker: &str, stop: fn(char) -> bool) -> Vec<String> {
    text.split(marker)
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| !stop(*c)).collect())
        .collect()
}

/// Pins the text of the condition of a job as the workflow spells it, independent of the table's
/// model of it: the labels and inputs it names, and the events it mentions. A condition that
/// names another job's label or input, or an event it should not start on, fails here.
fn assert_condition_text(job: &Job) -> TestResult {
    let expected = platform_job(&job.name).ok_or("an unexpected job")?;
    let condition = condition_of(job)?;
    let labels = after_marker(condition, "github.event.label.name == '", |c| c == '\'');
    let inputs = after_marker(condition, "inputs.", |c| {
        !(c.is_ascii_alphanumeric() || c == '_')
    });
    let name = &job.name;
    assert_eq!(
        inputs,
        [expected.input],
        "{name}: the inputs its condition names"
    );
    let events = |event: &str| condition.contains(&format!("github.event_name == '{event}'"));
    if let Some(label) = expected.label {
        // The schedule, its own manual input and its own label, and no other.
        assert_eq!(labels, [label], "{name}: the labels its condition names");
        assert!(events("schedule"), "{name}: not run by the schedule");
        assert!(events("workflow_dispatch"), "{name}: not run on demand");
        assert!(events("pull_request"), "{name}: not run by its label");
    } else {
        // On demand only: no schedule, no label, no pull request.
        assert!(events("workflow_dispatch"), "{name}: not run on demand");
        for word in ["schedule", "label", "pull_request"] {
            assert!(
                !condition.contains(word),
                "{name}: the condition mentions {word}, but the job runs only on demand"
            );
        }
    }
    Ok(())
}

// AC: CE-08C.6, CE-08.12
#[test]
fn macos_runs_weekly_on_demand_and_by_label_on_apple_silicon() -> TestResult {
    let platforms = workflow("platforms.yml")?;
    let on = &platforms["on"];
    assert_eq!(
        keys(on)?,
        set(&["schedule", "workflow_dispatch", "pull_request"])
    );
    let schedule = on["schedule"].as_vec().ok_or("schedule is not a list")?;
    assert_eq!(schedule.len(), 1, "one schedule");
    // Monday, 04:00 UTC.
    assert_eq!(schedule[0]["cron"].as_str(), Some("0 4 * * 1"));
    assert_eq!(strings(&on["pull_request"]["types"])?, ["labeled"]);

    let inputs = &on["workflow_dispatch"]["inputs"];
    assert_eq!(keys(inputs)?, set(&["windows", "macos", "macos_intel"]));
    for (input, default) in [
        ("windows", "true"),
        ("macos", "true"),
        ("macos_intel", "false"),
    ] {
        assert_eq!(inputs[input]["type"].as_str(), Some("boolean"), "{input}");
        assert_eq!(
            scalar(&inputs[input]["default"]).as_deref(),
            Some(default),
            "{input}"
        );
    }

    let jobs = jobs(&platforms)?;
    let macos = &jobs[index_of(&jobs, "macos")?];
    assert_eq!(macos.runs_on()?, "macos-latest");
    assert_condition_text(macos)?;
    let job = platform_job("macos").ok_or("no macos job")?;
    for (event, expected) in [
        (
            Event {
                name: "schedule",
                label: "",
                inputs: &[],
            },
            true,
        ),
        (
            Event {
                name: "workflow_dispatch",
                label: "",
                inputs: &["macos"],
            },
            true,
        ),
        (
            Event {
                name: "workflow_dispatch",
                label: "",
                inputs: &["windows"],
            },
            false,
        ),
        (
            Event {
                name: "pull_request",
                label: "macos",
                inputs: &[],
            },
            true,
        ),
    ] {
        assert_eq!(
            runs(job, condition_of(macos)?, &event)?,
            expected,
            "{} {} {:?}",
            event.name,
            event.label,
            event.inputs
        );
    }
    Ok(())
}

// AC: CE-08C.6
#[test]
fn intel_runs_only_on_demand() -> TestResult {
    let jobs = jobs(&workflow("platforms.yml")?)?;
    let intel = &jobs[index_of(&jobs, "macos-intel")?];
    assert_eq!(intel.runs_on()?, "macos-15-intel");
    assert_condition_text(intel)?;
    let job = platform_job("macos-intel").ok_or("no macos-intel job")?;
    for (event, expected) in [
        (
            Event {
                name: "schedule",
                label: "",
                inputs: &[],
            },
            false,
        ),
        (
            Event {
                name: "pull_request",
                label: "macos",
                inputs: &[],
            },
            false,
        ),
        (
            Event {
                name: "pull_request",
                label: "macos-intel",
                inputs: &[],
            },
            false,
        ),
        (
            Event {
                name: "workflow_dispatch",
                label: "",
                inputs: &["macos"],
            },
            false,
        ),
        (
            Event {
                name: "workflow_dispatch",
                label: "",
                inputs: &["macos_intel"],
            },
            true,
        ),
    ] {
        assert_eq!(
            runs(job, condition_of(intel)?, &event)?,
            expected,
            "{} {} {:?}",
            event.name,
            event.label,
            event.inputs
        );
    }
    // Its input is off unless asked for.
    let on = &workflow("platforms.yml")?["on"];
    assert_eq!(
        scalar(&on["workflow_dispatch"]["inputs"]["macos_intel"]["default"]).as_deref(),
        Some("false")
    );
    Ok(())
}

// AC: CE-08C.6
#[test]
fn a_newer_label_run_cancels_the_older_one() -> TestResult {
    for job in jobs(&workflow("platforms.yml")?)? {
        let concurrency = &job.node["concurrency"];
        // Per pull request and per job, so labels of two OSes do not cancel each other.
        assert_eq!(
            concurrency["group"].as_str(),
            Some(
                format!(
                    "platforms-${{{{ github.event.pull_request.number || github.run_id }}}}-{}",
                    job.id
                )
                .as_str()
            ),
            "{}",
            job.name
        );
        // Only a pull request run is cancelled: a schedule or manual run has its own run id.
        assert_eq!(
            concurrency["cancel-in-progress"].as_str(),
            Some("${{ github.event_name == 'pull_request' }}"),
            "{}",
            job.name
        );
    }
    Ok(())
}

// AC: CE-08C.6
#[test]
fn platforms_starts_no_job_for_another_label() -> TestResult {
    let labels = ["windows", "macos", "macos-intel", "needs-jann", "approved"];
    for job in jobs(&workflow("platforms.yml")?)? {
        let expected = platform_job(&job.name).ok_or("an unexpected job")?;
        assert_condition_text(&job)?;
        for label in labels {
            let event = Event {
                name: "pull_request",
                label,
                inputs: &[],
            };
            assert_eq!(
                runs(expected, condition_of(&job)?, &event)?,
                expected.label == Some(label),
                "{}: label {label}",
                job.name
            );
        }
    }
    Ok(())
}

// AC: CE-08.2
#[test]
fn engine_free_runs_on_every_pr() -> TestResult {
    let ci = workflow("ci.yml")?;
    assert!(
        keys(&ci["on"])?.contains("pull_request"),
        "ci.yml does not run on pull requests"
    );
    assert!(
        ci_checks()?.contains(&Check::EngineFree),
        "no ci job runs engine-free"
    );
    Ok(())
}

// AC: CE-08.3
#[test]
fn required_contexts_are_pr_jobs() -> TestResult {
    let mut contexts = BTreeSet::new();
    for file in PR_WORKFLOWS {
        for job in jobs(&workflow(file)?)? {
            assert!(
                contexts.insert(job.name.clone()),
                "{file}: the job name {} is used twice, and a status check has one job",
                job.name
            );
        }
    }

    let required = required_contexts_from_gate_md(&read("docs/gate.md")?);
    assert_eq!(
        contexts, required,
        "the jobs are not the checks of docs/gate.md §1"
    );

    let ruleset: serde_json::Value = serde_json::from_str(&read("gate/ruleset-main.json")?)?;
    let rules = ruleset["rules"]
        .as_array()
        .ok_or("the ruleset has no rules")?;
    let status_checks = rules
        .iter()
        .find(|rule| rule["type"] == "required_status_checks")
        .ok_or("the ruleset has no required_status_checks rule")?;
    let checks = status_checks["parameters"]["required_status_checks"]
        .as_array()
        .ok_or("required_status_checks is not a list")?;
    let in_ruleset: BTreeSet<String> = checks
        .iter()
        .map(|check| {
            let context = check["context"].as_str().ok_or("a check has no context")?;
            Ok(context.to_owned())
        })
        .collect::<TestResult<_>>()?;
    assert_eq!(
        contexts, in_ruleset,
        "the jobs are not the contexts of gate/ruleset-main.json"
    );
    Ok(())
}

/// The check and the job of every row of the first table of `docs/gate.md` §1, read through
/// `xtask::markdown`.
fn gate_md_check_rows() -> TestResult<Vec<(String, String)>> {
    let gate_md = read("docs/gate.md")?;
    let sections = markdown::sections(&gate_md, 2);
    let section = sections
        .iter()
        .find(|section| section.heading.text.starts_with("1."))
        .ok_or("docs/gate.md has no §1")?;
    let tables = markdown::tables(section.body);
    let table = tables.first().ok_or("docs/gate.md §1 has no table")?;
    let column = |name: &str| {
        table
            .header
            .iter()
            .position(|cell| cell == name)
            .ok_or_else(|| format!("the table of docs/gate.md §1 has no column {name}"))
    };
    let (check, job) = (column("Check")?, column("Job")?);
    table
        .rows
        .iter()
        .map(|row| match (row.get(check), row.get(job)) {
            (Some(check), Some(job)) => Ok((check.clone(), job.clone())),
            _ => Err(format!("a row of docs/gate.md §1 has no check or job: {row:?}").into()),
        })
        .collect()
}

// AC: CE-08.3
#[test]
fn gate_md_names_the_job_that_runs_each_check() -> TestResult {
    let rows = gate_md_check_rows()?;

    // The table lists every check of the gate and every pull-request check once, and nothing
    // else.
    let mut listed: Vec<String> = rows.iter().map(|(check, _)| check.clone()).collect();
    let mut expected: Vec<String> = GATE_ORDER
        .iter()
        .filter(|check| **check != Check::Tools)
        .chain(PR_ONLY.iter())
        .map(|check| check.name().to_owned())
        .collect();
    listed.sort();
    expected.sort();
    assert_eq!(
        listed, expected,
        "the Check column of docs/gate.md §1 is not the checks of the gate and the PR checks"
    );

    // The job of each row runs that check: a `ci.yml` job by the selection of its command, `pr`
    // by the checks of `cargo xtask pr`, `approval` only the approval.
    let ci = jobs(&workflow("ci.yml")?)?;
    for (name, job_name) in &rows {
        let check = Check::from_name(name).ok_or_else(|| format!("`{name}` is no check"))?;
        let runs = match job_name.as_str() {
            "pr" => PR_CHECKS.contains(&check),
            "approval" => check == Check::Approval,
            _ => {
                let job = &ci[index_of(&ci, job_name)?];
                selection_of(&job.gate_command()?)?
                    .checks()
                    .contains(&check)
            }
        };
        assert!(
            runs,
            "docs/gate.md §1 says the job {job_name} runs {name}, and it does not"
        );
    }
    Ok(())
}

// AC: CE-08.4
#[test]
fn every_job_runs_only_its_command_on_its_runner() -> TestResult {
    for (file, job) in all_jobs()? {
        let name = &job.name;
        assert_eq!(
            job.commands()?,
            commands_of(file, name),
            "{file}: job {name} runs more or other than its check"
        );
        let runner = match cached_xtask_command(name) {
            Some(_) => "ubuntu-latest",
            None => runner_for(file, name),
        };
        assert_eq!(job.runs_on()?, runner, "{file}: job {name}");

        let kinds = job
            .steps()?
            .iter()
            .map(step_kind)
            .collect::<TestResult<Vec<_>>>()?;
        let expected: &[&str] = match (file, name.as_str()) {
            (_, "approval") => &["checkout", "checkout", "xtask", "setup", "run"],
            (_, "pr" | "settings") => &["checkout", "xtask", "run"],
            ("ci.yml", "coverage") => &["checkout", "setup", "cache", "run"],
            // The question whether `ui` is needed, then Node and pnpm, the browser cache and the
            // browser, before `ui`.
            ("ci.yml", "ui") => &[
                "checkout", "xtask", "run", "pnpm", "node", "cache", "run", "run",
            ],
            ("platforms.yml", "windows" | "macos") => {
                &["checkout", "setup", "run", "pnpm", "node", "run", "run"]
            }
            _ => &["checkout", "setup", "run"],
        };
        assert_eq!(kinds, expected, "{file}: the steps of job {name}");
    }

    // The two checkouts of `approval` (D3): the base commit with its history, and, only while
    // the base has no xtask, the head of the pull request.
    let approval = jobs(&workflow("approval.yml")?)?;
    let steps = approval[index_of(&approval, "approval")?].steps()?;
    let base = &steps[0]["with"];
    assert_eq!(
        base["ref"].as_str(),
        Some("${{ github.event.pull_request.base.sha }}")
    );
    assert_eq!(base["fetch-depth"].as_i64(), Some(0));
    assert_eq!(
        steps[0]["if"],
        Yaml::BadValue,
        "the base checkout always runs"
    );
    let head = &steps[1]["with"];
    assert_eq!(
        head["ref"].as_str(),
        Some("${{ github.event.pull_request.head.sha }}")
    );
    assert_eq!(head["fetch-depth"].as_i64(), Some(0));
    assert_eq!(steps[1]["if"].as_str(), Some(APPROVAL_FALLBACK_IF));
    // Then xtask: the cached-xtask action where the checkout has one, else a build through the
    // checkout's own setup action, which bases older than P50 have.
    assert_eq!(steps[2]["uses"].as_str(), Some(XTASK_USES));
    assert_eq!(steps[2]["if"].as_str(), Some(APPROVAL_XTASK_IF));
    assert_eq!(steps[3]["uses"].as_str(), Some(SETUP_USES));
    assert_eq!(steps[3]["if"].as_str(), Some(APPROVAL_BUILD_IF));
    // `cargo xtask approval` fetches the pull request refs with `git fetch origin`, which needs
    // the credentials the checkout leaves behind.
    for step in &steps[..2] {
        assert_eq!(
            step["with"]["persist-credentials"],
            Yaml::BadValue,
            "a checkout of the approval job changes persist-credentials"
        );
    }
    Ok(())
}

/// What the workflow files may spell at their top level. A `defaults:` or an `env:` there would
/// change every command below it.
const WORKFLOW_KEYS: [&str; 5] = ["name", "on", "permissions", "concurrency", "jobs"];

/// What a job may spell. Left out on purpose: `continue-on-error` (a failed check passes),
/// `defaults` (another shell or folder), `strategy` and `needs` (a job that is skipped or
/// tolerated), `container`, `services` and `concurrency` (a job that a newer run cancels).
const JOB_KEYS: [&str; 6] = [
    "name",
    "runs-on",
    "timeout-minutes",
    "permissions",
    "if",
    "steps",
];

/// What a job of `platforms.yml` may spell: the keys of [`JOB_KEYS`] and `concurrency`, the
/// cancelling of an older label run, which `a_newer_label_run_cancels_the_older_one` pins. Those
/// jobs are not required checks.
const PLATFORM_JOB_KEYS: [&str; 7] = [
    "name",
    "runs-on",
    "timeout-minutes",
    "permissions",
    "if",
    "steps",
    "concurrency",
];

/// What a step of a job may spell. `env` is the token of the two steps that call GitHub, which
/// `token_reaches_only_the_steps_that_call_github` pins. `id` names the step whose output the
/// conditions of the `ui` steps read. Left out on purpose: `continue-on-error`,
/// `working-directory`, `shell` and `timeout-minutes`.
const STEP_KEYS: [&str; 6] = ["uses", "with", "run", "if", "env", "id"];

/// What a step of the setup action may spell on top of the keys of a job's step. `env` hands the
/// `gui-packages` input to the step that asks `xtask needs`, which `setup_installs_packages_from_the_list`
/// pins.
const SETUP_STEP_KEYS: [&str; 8] = ["name", "id", "uses", "with", "run", "shell", "if", "env"];

/// The condition of the step of the setup action that saves the built xtask.
const SAVE_XTASK_IF: &str = "inputs.save-xtask == 'true'";

/// The condition of the step of the setup action that asks whether the run needs the packages:
/// the job passes the checks that build.
const ASK_IF: &str = "inputs.gui-packages != 'false'";

/// The condition of the steps of the setup action that install the Linux packages of the GUI: the
/// job asks for them and `xtask needs` says the run needs its checks.
const GUI_IF: &str = "inputs.gui-packages != 'false' && steps.needs.outputs.needed == 'true'";

/// The condition of the step that saves the packages: the job saves its build cache, and the
/// restore found nothing.
const GUI_SAVE_IF: &str = "inputs.gui-packages != 'false' && steps.needs.outputs.needed == 'true' \
                           && inputs.save-cache == 'true' && steps.packages.outputs.cache-hit != \
                           'true'";

/// The condition of the checkout of the head of the pull request in `approval`, which runs only
/// while the base commit has no xtask.
const APPROVAL_FALLBACK_IF: &str = "hashFiles('xtask/Cargo.toml') == ''";

/// The condition of the step of `approval` that takes the cached xtask: only where the checkout
/// has the cached-xtask action.
const APPROVAL_XTASK_IF: &str = "hashFiles('.github/actions/xtask/action.yml') != ''";

/// The condition of the step of `approval` that builds xtask with the checkout's setup action
/// instead: bases older than P50 have no cached-xtask action.
const APPROVAL_BUILD_IF: &str = "hashFiles('.github/actions/xtask/action.yml') == ''";

/// The conditions of the steps of the workflows that have an `if:`, by the position of the step.
/// In `approval`: the head checkout, then exactly one of the two ways to get xtask. In the jobs
/// that run `ui`: the steps that set up Node, pnpm and the browser, which wait for `editor-ui/`.
/// Every other step of every job has none, so no condition can skip a check.
fn step_condition(file: &str, job: &str, position: usize) -> Option<&'static str> {
    match (file, job, position) {
        ("approval.yml", "approval", 1) => Some(APPROVAL_FALLBACK_IF),
        ("approval.yml", "approval", 2) => Some(APPROVAL_XTASK_IF),
        ("approval.yml", "approval", 3) => Some(APPROVAL_BUILD_IF),
        // pnpm, Node, the browser cache and the browser after the question; in the platform jobs
        // pnpm, Node and the browser. The `ui` job also runs its check only when it is needed.
        ("ci.yml", "ui", 3..=6) | ("platforms.yml", "windows" | "macos", 3..=5) => {
            Some(UI_EXISTS_IF)
        }
        ("ci.yml", "ui", 7) => Some(UI_RUN_IF),
        _ => None,
    }
}

/// The keys of a mapping that are not in `allowed`.
fn unexpected_keys(node: &Yaml, allowed: &[&str]) -> TestResult<Vec<String>> {
    Ok(keys(node)?
        .into_iter()
        .filter(|key| !allowed.contains(&key.as_str()))
        .collect())
}

// AC: CE-08.3, CE-08.4
#[test]
fn no_key_lets_a_job_pass_while_xtask_fails() -> TestResult {
    for file in ALL_WORKFLOWS {
        let workflow = workflow(file)?;
        assert_eq!(
            unexpected_keys(&workflow, &WORKFLOW_KEYS)?,
            Vec::<String>::new(),
            "{file}: a key at the top of the workflow that no check needs"
        );
        let allowed: &[&str] = if file == "platforms.yml" {
            &PLATFORM_JOB_KEYS
        } else {
            &JOB_KEYS
        };
        for job in jobs(&workflow)? {
            let name = &job.name;
            assert_eq!(
                unexpected_keys(&job.node, allowed)?,
                Vec::<String>::new(),
                "{file}: job {name} has keys that can make it pass while xtask fails, or run \
                 somewhere else than the root of the repository"
            );
            // The only job-level conditions are the label and event conditions of the
            // `platforms` jobs, which are not required checks.
            assert_eq!(
                job.node["if"].is_badvalue(),
                file != "platforms.yml",
                "{file}: job {name}: only the platforms jobs have an `if:`"
            );
            // The token goes to two steps, never to a job: see the token test.
            for (position, step) in job.steps()?.iter().enumerate() {
                assert_eq!(
                    unexpected_keys(step, &STEP_KEYS)?,
                    Vec::<String>::new(),
                    "{file}: job {name}, step {position}: keys that can make the step pass while \
                     xtask fails, or run it somewhere else than the root of the repository"
                );
                assert_eq!(
                    step["if"].as_str(),
                    step_condition(file, name, position),
                    "{file}: job {name}, step {position}: only the head checkout of approval and \
                     its two ways to get xtask have an `if:`"
                );
            }
        }
    }
    Ok(())
}

/// The local actions, and the steps of each that carry a condition, by position, with that
/// condition. Every other step of them runs always: a condition anywhere else could skip a check.
const LOCAL_ACTIONS: [(&str, &[(usize, &str)]); 2] = [
    (
        SETUP_ACTION,
        &[
            (4, SAVE_XTASK_IF),
            (5, ASK_IF),
            (6, GUI_IF),
            (7, GUI_IF),
            (8, GUI_IF),
            (9, GUI_IF),
            (10, GUI_SAVE_IF),
        ],
    ),
    (XTASK_ACTION, &[(1, BUILD_ON_MISS_IF)]),
];

// AC: CE-08.3, CE-08.4
#[test]
fn no_key_lets_a_step_of_a_local_action_fail_silently() -> TestResult {
    for (file, _) in LOCAL_ACTIONS {
        let action = parse(file)?;
        // The inputs of the setup action are pinned by
        // `setup_action_passes_the_cache_inputs_to_rust_cache`; the xtask action has none.
        assert_eq!(
            unexpected_keys(&action, &["name", "description", "inputs", "runs"])?,
            Vec::<String>::new(),
            "{file} has a key it does not need"
        );
        assert_eq!(
            unexpected_keys(&action["runs"], &["using", "steps"])?,
            Vec::<String>::new(),
            "{file}: the runs has a key it does not need"
        );
        for (position, step) in action["runs"]["steps"]
            .as_vec()
            .ok_or_else(|| format!("{file} has no steps"))?
            .iter()
            .enumerate()
        {
            assert_eq!(
                unexpected_keys(step, &SETUP_STEP_KEYS)?,
                Vec::<String>::new(),
                "{file}, step {position}: keys that can make the step pass when it fails (such as \
                 `continue-on-error`), skip it or run it elsewhere than the root of the repository"
            );
        }
    }
    Ok(())
}

// AC: CE-08.3
#[test]
fn no_step_condition_except_the_cache_ones() -> TestResult {
    for (file, conditions) in LOCAL_ACTIONS {
        let parsed = parse(file)?;
        let steps = parsed["runs"]["steps"]
            .as_vec()
            .ok_or_else(|| format!("{file} has no steps"))?;
        for (position, step) in steps.iter().enumerate() {
            let expected = conditions
                .iter()
                .find(|(at, _)| *at == position)
                .map(|(_, condition)| *condition);
            assert_eq!(
                step["if"].as_str(),
                expected,
                "{file}, step {position}: only the steps that save the xtask, build it on a miss \
                 or install the Linux packages have an `if:`"
            );
        }
        for (position, _) in conditions {
            assert!(
                position < &steps.len(),
                "{file} has no step {position} for its condition"
            );
        }
    }
    Ok(())
}

/// The minutes a job may run before GitHub cancels it. The default is six hours. The jobs of
/// `ci.yml` and `platforms.yml` have their own: a platform job builds once and runs its checks,
/// slow tests included, one after the other.
fn timeout_of(file: &str, name: &str) -> TestResult<i64> {
    let known = match (file, name) {
        ("ci.yml", _) => ci_job(name).map(|job| job.timeout),
        ("platforms.yml", _) => platform_job(name).map(|job| job.timeout),
        // These run a binary `main` cached, so a label change is checked in seconds.
        ("pr.yml", "pr") | ("approval.yml", "approval") | ("settings.yml", "settings") => Some(15),
        _ => None,
    };
    known.ok_or_else(|| format!("{file}: job {name} has no expected timeout").into())
}

// AC: CE-08.3, CE-08.4
#[test]
fn every_job_has_a_timeout() -> TestResult {
    for (file, job) in all_jobs()? {
        let expected = timeout_of(file, &job.name)?;
        assert_eq!(
            job.node["timeout-minutes"].as_i64(),
            Some(expected),
            "{file}: job {} has no `timeout-minutes` of {expected}, and a hung job runs for six \
             hours",
            job.name
        );
    }
    Ok(())
}

#[test]
fn required_jobs_always_report() -> TestResult {
    for file in PR_WORKFLOWS {
        let workflow = workflow(file)?;
        let on = &workflow["on"];
        assert!(
            keys(on)?.contains("pull_request"),
            "{file} has no pull_request trigger"
        );
        if on["pull_request"].is_hash() {
            let found = keys(&on["pull_request"])?;
            for filter in [
                "paths",
                "paths-ignore",
                "branches",
                "branches-ignore",
                "tags",
                "tags-ignore",
            ] {
                assert!(
                    !found.contains(filter),
                    "{file}: pull_request filters on {filter}, so a required check can be missing"
                );
            }
        }
        for job in jobs(&workflow)? {
            assert_eq!(
                job.node["if"],
                Yaml::BadValue,
                "{file}: job {} can be skipped, and a skipped check is not a passed one",
                job.name
            );
            // The workflow's own `concurrency` decides which run is cancelled (see
            // `triggers_and_concurrency_follow_the_spec`); a job-level one could cancel a required
            // check that the workflow keeps.
            assert_eq!(
                job.node["concurrency"],
                Yaml::BadValue,
                "{file}: job {} has a `concurrency` of its own",
                job.name
            );
        }
    }
    Ok(())
}

#[test]
fn no_pull_request_target_and_least_privilege() -> TestResult {
    for file in ALL_WORKFLOWS {
        let workflow = workflow(file)?;
        let triggers = keys(&workflow["on"])?;
        assert!(
            !triggers.contains("pull_request_target"),
            "{file} runs on pull_request_target, with a token and secrets for fork code"
        );
        assert_eq!(
            string_map(&workflow["permissions"])?,
            map(&[("contents", "read")]),
            "{file}: the top-level permissions"
        );
        for job in jobs(&workflow)? {
            if file == "approval.yml" && job.name == "approval" {
                // It labels and comments on the pull request, and nothing else.
                assert_eq!(
                    string_map(&job.node["permissions"])?,
                    map(&[
                        ("contents", "read"),
                        ("pull-requests", "write"),
                        ("issues", "write"),
                    ]),
                    "the permissions of the approval job"
                );
            } else if file == "settings.yml" && job.name == "settings" {
                // A job-level `permissions:` replaces the top-level one and sets every scope it
                // does not name to none. The drift check lists the labels of the repository,
                // which needs `issues: read` in a private repository. It reads, and writes
                // nothing.
                assert_eq!(
                    string_map(&job.node["permissions"])?,
                    map(&[("contents", "read"), ("issues", "read")]),
                    "the permissions of the settings job"
                );
            } else {
                assert_eq!(
                    job.node["permissions"],
                    Yaml::BadValue,
                    "{file}: job {} sets permissions, and only approval and settings may",
                    job.name
                );
            }
        }
    }
    Ok(())
}

/// The step of each job that calls GitHub, the only one that gets the token.
const TOKEN_STEPS: [(&str, &str); 2] = [
    ("approval", "target/xtask-bin/debug/xtask --root . approval"),
    (
        "settings",
        "target/xtask-bin/debug/xtask --root . github check",
    ),
];

#[test]
fn token_reaches_only_the_steps_that_call_github() -> TestResult {
    for (file, job) in all_jobs()? {
        assert_eq!(
            job.node["env"],
            Yaml::BadValue,
            "{file}: job {} has an env, and the token is given to the steps that need it",
            job.name
        );
        for (position, step) in job.steps()?.iter().enumerate() {
            let calls_github = TOKEN_STEPS
                .iter()
                .any(|(name, run)| job.name == *name && step["run"].as_str() == Some(run));
            if calls_github {
                assert_eq!(
                    string_map(&step["env"])?,
                    map(&[("GH_TOKEN", TOKEN)]),
                    "{file}: job {}, step {position}: the env of a step that calls GitHub",
                    job.name
                );
            } else {
                assert_eq!(
                    step["env"],
                    Yaml::BadValue,
                    "{file}: job {}, step {position} has an env, and only the steps that call \
                     GitHub need the token",
                    job.name
                );
            }
        }
    }
    Ok(())
}

// AC: CE-08.5
#[test]
fn only_the_approval_checkouts_keep_the_git_credentials() -> TestResult {
    // Only `cargo xtask approval` talks to git with credentials (it fetches the pull request
    // refs); every other job leaves them out of `.git/config`, away from the later actions.
    for (file, job) in all_jobs()? {
        for (position, step) in job.steps()?.iter().enumerate() {
            if step_kind(step)? != "checkout" {
                continue;
            }
            let persisted = &step["with"]["persist-credentials"];
            if job.name == "approval" {
                assert_eq!(
                    *persisted,
                    Yaml::BadValue,
                    "{file}: job {}, step {position}: approval needs the default",
                    job.name
                );
            } else {
                assert_eq!(
                    persisted.as_bool(),
                    Some(false),
                    "{file}: job {}, step {position}: the checkout must not persist credentials",
                    job.name
                );
            }
        }
    }
    Ok(())
}

/// Every `uses:` of the workflows and of the setup action, as the files spell it.
fn uses_lines() -> TestResult<Vec<(String, String)>> {
    let mut files: Vec<String> = ALL_WORKFLOWS
        .iter()
        .map(|file| workflow_path(file))
        .collect();
    files.push(SETUP_ACTION.to_owned());
    files.push(XTASK_ACTION.to_owned());
    let mut found = Vec::new();
    for file in files {
        for line in read(&file)?.lines() {
            let line = line.trim_start();
            let line = line.strip_prefix("- ").unwrap_or(line);
            if let Some(value) = line.strip_prefix("uses:") {
                found.push((file.clone(), value.trim().to_owned()));
            }
        }
    }
    Ok(found)
}

#[test]
fn actions_pinned_by_sha() -> TestResult {
    let found = uses_lines()?;

    // The text scan above must see every action the parsed files use.
    let mut parsed = 0;
    for (_, job) in all_jobs()? {
        parsed += job
            .steps()?
            .iter()
            .filter(|step| step["uses"].as_str().is_some())
            .count();
    }
    for (file, _) in LOCAL_ACTIONS {
        for step in parse(file)?["runs"]["steps"]
            .as_vec()
            .ok_or_else(|| format!("{file} has no steps"))?
        {
            parsed += usize::from(step["uses"].as_str().is_some());
        }
    }
    assert_eq!(
        found.len(),
        parsed,
        "a `uses:` is written in a way the scan misses"
    );

    for (file, value) in found {
        if value.starts_with("./") {
            continue;
        }
        let (action, comment) = value
            .split_once(" # ")
            .ok_or_else(|| format!("{file}: {value} has no version comment"))?;
        let (name, sha) = action
            .split_once('@')
            .ok_or_else(|| format!("{file}: {value} is not pinned"))?;
        assert!(
            name.contains('/') && !name.contains(' '),
            "{file}: {value} is not an owner/repo action"
        );
        assert!(
            sha.len() == 40 && sha.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "{file}: {value} is not pinned by a full commit SHA"
        );
        assert!(
            comment.starts_with('v') && comment[1..].starts_with(|c: char| c.is_ascii_digit()),
            "{file}: {value} does not say its version in the comment"
        );
    }
    Ok(())
}

#[test]
fn no_expressions_in_run_steps() -> TestResult {
    for (file, job) in all_jobs()? {
        for command in job.commands()? {
            assert!(
                !command.contains("${{"),
                "{file}: job {} has an expression in `run: {command}`, which a pull request can inject",
                job.name
            );
        }
    }
    for (file, _) in LOCAL_ACTIONS {
        let action = parse(file)?;
        let steps = action["runs"]["steps"].as_vec().ok_or("no steps")?;
        for command in run_values(steps) {
            assert!(
                !command.contains("${{"),
                "{file} has an expression in `run: {command}`"
            );
        }
    }
    Ok(())
}

/// The checks of `platforms.yml` that read history: the commits of a pull request, or the merge
/// base with its base. `slow-tests` reads the changes since the merge base to decide whether its
/// tests run.
const READ_HISTORY: [Check; 3] = [Check::CommitMsg, Check::Coverage, Check::SlowTests];

#[test]
fn history_is_fetched_where_commits_or_the_merge_base_are_read() -> TestResult {
    // Every `ci.yml` job reads the changed paths since the merge base to decide whether the
    // change needs its checks.
    for job in jobs(&workflow("ci.yml")?)? {
        let depth = &job.steps()?[0]["with"]["fetch-depth"];
        assert_eq!(
            depth.as_i64(),
            Some(0),
            "ci.yml: {} reads history",
            job.name
        );
    }
    for job in jobs(&workflow("platforms.yml")?)? {
        let depth = &job.steps()?[0]["with"]["fetch-depth"];
        let reads_history = selection_of(&job.gate_command()?)?
            .checks()
            .iter()
            .any(|check| READ_HISTORY.contains(check));
        if reads_history {
            assert_eq!(
                depth.as_i64(),
                Some(0),
                "platforms.yml: {} reads history",
                job.name
            );
        } else {
            assert_eq!(
                *depth,
                Yaml::BadValue,
                "platforms.yml: {} fetches more than the commit it checks",
                job.name
            );
        }
    }
    Ok(())
}

#[test]
fn coverage_restores_and_saves_the_baseline_cache() -> TestResult {
    let ci = jobs(&workflow("ci.yml")?)?;
    let coverage = &ci[index_of(&ci, "coverage")?];
    let cache = coverage
        .steps()?
        .iter()
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/cache@"))
        })
        .ok_or("coverage has no cache step")?;
    assert_eq!(
        cache["with"]["path"].as_str(),
        Some("target/xtask/coverage/cache")
    );
    assert_eq!(
        cache["with"]["key"].as_str(),
        Some(
            "coverage-${{ github.event.pull_request.base.sha || github.sha }}-${{ \
             hashFiles('rust-toolchain.toml', 'gate/tools.toml') }}"
        )
    );
    assert_eq!(
        cache["with"]["restore-keys"].as_str().map(str::trim),
        Some("coverage-")
    );
    Ok(())
}

#[test]
fn triggers_and_concurrency_follow_the_spec() -> TestResult {
    let ci = workflow("ci.yml")?;
    assert_eq!(
        keys(&ci["on"])?,
        set(&["pull_request", "push", "workflow_dispatch"])
    );
    assert_eq!(
        strings(&ci["on"]["pull_request"]["types"])?,
        ["opened", "synchronize", "reopened"]
    );
    assert_eq!(strings(&ci["on"]["push"]["branches"])?, ["main"]);

    let pr = workflow("pr.yml")?;
    assert_eq!(keys(&pr["on"])?, set(&["pull_request"]));
    assert_eq!(
        strings(&pr["on"]["pull_request"]["types"])?,
        ["opened", "edited", "synchronize", "reopened"]
    );

    let approval = workflow("approval.yml")?;
    assert_eq!(keys(&approval["on"])?, set(&["pull_request"]));
    assert_eq!(
        strings(&approval["on"]["pull_request"]["types"])?,
        [
            "opened",
            "edited",
            "synchronize",
            "reopened",
            "labeled",
            "unlabeled"
        ]
    );

    let settings = workflow("settings.yml")?;
    assert_eq!(
        keys(&settings["on"])?,
        set(&["schedule", "workflow_dispatch"])
    );
    let schedule = settings["on"]["schedule"]
        .as_vec()
        .ok_or("schedule is not a list")?;
    assert_eq!(schedule.len(), 1, "one schedule");
    // Monday, 05:00 UTC.
    assert_eq!(schedule[0]["cron"].as_str(), Some("0 5 * * 1"));

    // Outdated runs of the CI and the PR checks are cancelled; an approval run never is, so a
    // label change is not cut off halfway.
    let group = "${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}";
    for file in ["ci.yml", "pr.yml"] {
        let concurrency = &workflow(file)?["concurrency"];
        assert_eq!(concurrency["group"].as_str(), Some(group), "{file}");
        assert_eq!(
            concurrency["cancel-in-progress"].as_str(),
            Some("${{ github.event_name == 'pull_request' }}"),
            "{file}"
        );
    }
    let concurrency = &approval["concurrency"];
    assert_eq!(
        concurrency["group"].as_str(),
        Some("${{ github.workflow }}-${{ github.event.pull_request.number }}")
    );
    assert_eq!(concurrency["cancel-in-progress"].as_bool(), Some(false));
    Ok(())
}

/// The action a step uses, without its version.
fn action_name(step: &Yaml) -> Option<&str> {
    step["uses"]
        .as_str()
        .and_then(|uses| uses.split_once('@'))
        .map(|(name, _)| name)
}

#[test]
fn setup_action_installs_the_toolchain_the_cache_and_the_pinned_tools() -> TestResult {
    let action = parse(SETUP_ACTION)?;
    assert_eq!(action["runs"]["using"].as_str(), Some("composite"));
    let steps = action["runs"]["steps"]
        .as_vec()
        .ok_or("the action has no steps")?;
    // The five steps below, then the six of the Linux packages (see
    // `setup_installs_packages_from_the_list`).
    assert_eq!(steps.len(), 11);

    // Every command step names its shell: a composite action has no default.
    for step in steps.iter().filter(|step| step["run"].as_str().is_some()) {
        assert_eq!(step["shell"].as_str(), Some("bash"));
    }

    // The toolchain comes from `rust-toolchain.toml`. Since rustup 1.28 only `toolchain
    // install` installs it; `rustup show` does not.
    assert_eq!(
        steps[0]["run"].as_str().map(str::trim),
        Some("rustup toolchain install")
    );

    assert_eq!(action_name(&steps[1]), Some("Swatinem/rust-cache"));

    // The spec line of `gate/tools.toml` goes to the install action through a step output.
    assert_eq!(steps[2]["id"].as_str(), Some("tools"));
    let command = steps[2]["run"]
        .as_str()
        .ok_or("the tools step has no command")?;
    assert!(
        command.contains("cargo xtask tools --print-install-spec"),
        "{command}"
    );
    assert!(
        command.contains("spec=") && command.contains("$GITHUB_OUTPUT"),
        "{command}"
    );
    assert_eq!(action_name(&steps[3]), Some("taiki-e/install-action"));
    assert_eq!(
        steps[3]["with"]["tool"].as_str(),
        Some("${{ steps.tools.outputs.spec }}")
    );

    // The last step saves the xtask the tools step built, keyed by what goes into it, and only
    // when the job asks for it. xtask is a member of the workspace, so the root `Cargo.toml` goes
    // into it too: its edition, the versions and features of its dependencies, its profiles.
    assert_eq!(action_name(&steps[4]), Some("actions/cache/save"));
    assert_eq!(steps[4]["if"].as_str(), Some(SAVE_XTASK_IF));
    assert_eq!(
        string_map(&steps[4]["with"])?,
        map(&[
            ("path", "target/xtask-bin/debug/xtask"),
            (
                "key",
                "xtask-bin-${{ runner.os }}-${{ hashFiles('xtask/**', 'Cargo.toml', \
                 'Cargo.lock', 'rust-toolchain.toml', '.cargo/**') }}"
            ),
        ])
    );
    Ok(())
}

/// The inputs of the setup action: the key of jobs that build the same thing and share one cache,
/// and whether the job saves the cache, reach rust-cache and nothing else; `save-xtask` reaches
/// only the step that saves the xtask binary. Their defaults keep rust-cache's own behaviour, a
/// cache per job that every run saves, and save no xtask.
#[test]
fn setup_action_passes_the_cache_inputs_to_rust_cache() -> TestResult {
    let action = parse(SETUP_ACTION)?;
    let inputs = &action["inputs"];
    assert_eq!(
        keys(inputs)?,
        set(&["cache-key", "save-cache", "save-xtask", "gui-packages"])
    );
    for (input, default) in [
        ("cache-key", ""),
        ("save-cache", "true"),
        ("save-xtask", "false"),
        ("gui-packages", "false"),
    ] {
        assert_eq!(
            scalar(&inputs[input]["default"]).as_deref(),
            Some(default),
            "{input}"
        );
        assert_eq!(inputs[input]["required"].as_bool(), Some(false), "{input}");
    }
    assert_eq!(
        string_map(&action["runs"]["steps"][1]["with"])?,
        map(&[
            ("shared-key", "${{ inputs.cache-key }}"),
            ("save-if", "${{ inputs.save-cache }}"),
        ]),
        "the rust-cache step"
    );
    Ok(())
}

/// The expression that is true on `main` only.
const ON_MAIN: &str = "${{ github.ref == 'refs/heads/main' }}";

/// The inputs a job passes to its setup step, or `None` when the job has no setup step (the jobs
/// that run the xtask action).
fn setup_inputs(job: &Job) -> TestResult<Option<BTreeMap<String, String>>> {
    let Some(setup) = job
        .steps()?
        .iter()
        .find(|step| step["uses"].as_str() == Some(SETUP_USES))
    else {
        return Ok(None);
    };
    if setup["with"].is_badvalue() {
        Ok(Some(BTreeMap::new()))
    } else {
        string_map(&setup["with"]).map(Some)
    }
}

/// The value of an input of the setup action: what the step passes, else the default of the
/// action, so a step that leaves an input out is not taken to save nothing.
fn setup_input(inputs: &BTreeMap<String, String>, input: &str) -> TestResult<String> {
    if let Some(value) = inputs.get(input) {
        return Ok(value.clone());
    }
    let action = parse(SETUP_ACTION)?;
    scalar(&action["inputs"][input]["default"])
        .ok_or_else(|| format!("the setup action has no default for {input}").into())
}

/// The jobs of the workflows that run on `pull_request`, which may not save a build cache.
fn pull_request_jobs() -> TestResult<Vec<(&'static str, Job)>> {
    let mut found = Vec::new();
    for (file, job) in all_jobs()? {
        if keys(&workflow(file)?["on"])?.contains("pull_request") {
            found.push((file, job));
        }
    }
    Ok(found)
}

// AC: CE-08C.5
#[test]
fn no_pull_request_job_saves_a_build_cache() -> TestResult {
    // A pull request is searched first, so a cache it saved would win over the one of `main`.
    for (file, job) in pull_request_jobs()? {
        let Some(inputs) = setup_inputs(&job)? else {
            continue;
        };
        // `platforms` runs on a label as well as on the schedule: it saves on the schedule only.
        let cache_ok = if file == "platforms.yml" {
            ON_SCHEDULE
        } else {
            ON_MAIN
        };
        let value = setup_input(&inputs, "save-cache")?;
        assert!(
            value == cache_ok || (value == "false" && file != "platforms.yml"),
            "{file}: job {} saves with save-cache: {value}, which a pull request does",
            job.name
        );
        let value = setup_input(&inputs, "save-xtask")?;
        assert!(
            value == "false" || value == ON_MAIN,
            "{file}: job {} saves with save-xtask: {value}, which a pull request does",
            job.name
        );
    }
    Ok(())
}

// AC: CE-08C.5
#[test]
fn only_gate_on_main_saves_the_cached_xtask() -> TestResult {
    for (file, job) in all_jobs()? {
        let inputs = setup_inputs(&job)?.unwrap_or_default();
        let saves = inputs
            .get("save-xtask")
            .is_some_and(|value| value != "false");
        assert_eq!(
            saves,
            file == "ci.yml" && job.name == "gate",
            "{file}: job {}: only the gate job saves the xtask binary",
            job.name
        );
        if saves {
            assert_eq!(inputs["save-xtask"], ON_MAIN);
        }
    }
    Ok(())
}

/// The inputs each job passes to the setup action. The jobs that run the xtask action have no
/// setup step of their own, but `approval`, whose base may predate the xtask action, builds xtask
/// there as the action's fallback does: restoring `main`'s `gate` cache and saving nothing.
#[test]
fn setup_inputs_of_every_job() -> TestResult {
    let long = |key: &str| {
        let mut with = map(&[("cache-key", key), ("save-cache", ON_MAIN)]);
        if let Some(checks) = gui_packages_of(key) {
            with.insert("gui-packages".to_owned(), checks.to_owned());
        }
        with
    };
    let mut gate = long("gate");
    gate.insert("save-xtask".to_owned(), ON_MAIN.to_owned());
    let platform = |key: &str| map(&[("cache-key", key), ("save-cache", ON_SCHEDULE)]);
    let shared = [
        ("ci.yml", "gate", gate),
        ("ci.yml", "test", long("test")),
        ("ci.yml", "coverage", long("coverage")),
        ("platforms.yml", "windows", platform("windows")),
        ("platforms.yml", "macos", platform("macos")),
        ("platforms.yml", "macos-intel", platform("macos-intel")),
        (
            "approval.yml",
            "approval",
            map(&[("cache-key", "gate"), ("save-cache", "false")]),
        ),
    ];
    for (file, job) in all_jobs()? {
        let name = &job.name;
        let xtask_only = (cached_xtask_command(name).is_some() && name != "approval")
            || (file == "ci.yml" && name == "ui");
        let expected = if xtask_only {
            None
        } else {
            Some(
                shared
                    .iter()
                    .find(|(in_file, job_name, _)| *in_file == file && job_name == name)
                    .map(|(_, _, with)| with.clone())
                    .unwrap_or_default(),
            )
        };
        assert_eq!(setup_inputs(&job)?, expected, "{file}: job {name}");
    }
    Ok(())
}

/// The one key of the cached xtask binary, as a step spells it.
fn xtask_key(step: &Yaml) -> TestResult<String> {
    step["with"]["key"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the step has no key".into())
}

/// The steps of the xtask action.
fn xtask_action_steps() -> TestResult<Vec<Yaml>> {
    Ok(parse(XTASK_ACTION)?["runs"]["steps"]
        .as_vec()
        .ok_or("the xtask action has no steps")?
        .clone())
}

// AC: CE-08C.5
#[test]
fn pr_and_approval_take_xtask_from_main_s_cache() -> TestResult {
    // The xtask action restores the binary by its exact key, without `restore-keys`, so it never
    // takes a stale one, and saves nothing: `main` is the only job that saves it.
    let steps = xtask_action_steps()?;
    let restore = &steps[0];
    assert_eq!(action_name(restore), Some("actions/cache/restore"));
    assert_eq!(restore["id"].as_str(), Some("restore"));
    assert_eq!(
        string_map(&restore["with"])?,
        map(&[("path", XTASK_BIN), ("key", &xtask_key(restore)?)]),
        "the restore step has `restore-keys` or other inputs"
    );
    assert!(
        steps
            .iter()
            .all(|step| action_name(step) != Some("actions/cache/save")),
        "the xtask action saves a cache"
    );

    // On a miss it builds xtask as before, and saves nothing, so a pull request never writes the
    // cache `main` is searched for.
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[1]["if"].as_str(), Some(BUILD_ON_MISS_IF));
    assert_eq!(steps[1]["uses"].as_str(), Some(SETUP_USES));
    assert_eq!(
        string_map(&steps[1]["with"])?,
        map(&[("cache-key", "gate"), ("save-cache", "false")]),
        "the fallback build of xtask"
    );

    for (file, name) in [("pr.yml", "pr"), ("approval.yml", "approval")] {
        let jobs = jobs(&workflow(file)?)?;
        let job = &jobs[index_of(&jobs, name)?];
        let kinds = job
            .steps()?
            .iter()
            .map(step_kind)
            .collect::<TestResult<Vec<_>>>()?;
        assert!(kinds.contains(&"xtask"), "{file}: {name} has no xtask step");
        assert!(
            !kinds.iter().any(|kind| kind.starts_with("cache")),
            "{file}: {name} touches a cache itself"
        );
    }
    Ok(())
}

// AC: CE-08C.5
#[test]
fn the_cached_xtask_has_one_key() -> TestResult {
    let setup = parse(SETUP_ACTION)?;
    let saved = setup["runs"]["steps"]
        .as_vec()
        .ok_or("the setup action has no steps")?
        .iter()
        .find(|step| action_name(step) == Some("actions/cache/save"))
        .ok_or("the setup action saves no xtask")?;
    let restored = &xtask_action_steps()?[0];
    assert_eq!(
        xtask_key(saved)?,
        xtask_key(restored)?,
        "the key that saves the xtask binary is not the key that restores it"
    );
    assert_eq!(saved["with"]["path"], restored["with"]["path"]);
    Ok(())
}

// AC: CE-08.5
#[test]
fn approval_keys_the_cached_xtask_on_the_base_checkout() -> TestResult {
    // The key hashes `xtask/**`, so the xtask step comes after every checkout: it then hashes the
    // checkout `approval` runs, the base commit (D3), and a head checkout (only while the base
    // has no xtask) is hashed too. The build that stands in for it on a base without the xtask
    // action comes after every checkout as well, so it builds the xtask that runs.
    let approval = jobs(&workflow("approval.yml")?)?;
    let steps = approval[index_of(&approval, "approval")?].steps()?;
    let position_of = |uses: &str| {
        steps
            .iter()
            .position(|step| step["uses"].as_str() == Some(uses))
            .ok_or_else(|| format!("approval has no step that uses {uses}"))
    };
    let xtask = position_of(XTASK_USES)?.min(position_of(SETUP_USES)?);
    for (position, step) in steps.iter().enumerate() {
        if step_kind(step)? == "checkout" {
            assert!(
                position < xtask,
                "step {position}: a checkout after a step that gets xtask changes what the key \
                 hashed or what is built"
            );
            // The checkouts that always run are the base commit; the head only on the fallback.
            if step["if"].is_badvalue() {
                assert_eq!(
                    step["with"]["ref"].as_str(),
                    Some("${{ github.event.pull_request.base.sha }}"),
                    "step {position}: a checkout that always runs is not the base commit"
                );
            }
        }
    }
    assert!(xtask > 0, "the xtask step is the first step");
    Ok(())
}

// AC: CE-08C.1
#[test]
fn pr_runs_both_checks_in_one_job() -> TestResult {
    let jobs = jobs(&workflow("pr.yml")?)?;
    assert_eq!(jobs.len(), 1, "pr.yml has more than the one job `pr`");
    let job = &jobs[0];
    assert_eq!((job.id.as_str(), job.name.as_str()), ("pr", "pr"));
    assert_eq!(job.commands()?, commands_of("pr.yml", "pr"));
    assert_eq!(job.node["timeout-minutes"].as_i64(), Some(15));
    Ok(())
}

/// The package list the setup action installs, as `gate/linux-packages.txt` writes it.
const LINUX_PACKAGES: [&str; 3] = ["build-essential", "pkg-config", "libwebkit2gtk-4.1-dev"];

/// The step of a job that uses `action`, by the part of its `uses:` before the version.
fn step_of<'a>(job: &'a Job, action: &str) -> TestResult<&'a Yaml> {
    job.steps()?
        .iter()
        .find(|step| action_name(step) == Some(action))
        .ok_or_else(|| format!("job {} has no step that uses {action}", job.name).into())
}

/// A job of `ci.yml`, by name.
fn ci_named(name: &str) -> TestResult<Job> {
    let all = jobs(&workflow("ci.yml")?)?;
    let at = index_of(&all, name)?;
    all.into_iter()
        .nth(at)
        .ok_or_else(|| format!("no job named {name}").into())
}

/// The setup of Node and pnpm that the jobs running `ui` share: pnpm at the version of
/// `editor-ui/package.json`, then Node 24 with the pnpm store cached by the lockfile.
fn assert_node_and_pnpm(job: &Job) -> TestResult {
    let name = &job.name;
    let pnpm = step_of(job, "pnpm/action-setup")?;
    assert_eq!(
        string_map(&pnpm["with"])?,
        map(&[("package_json_file", "editor-ui/package.json")]),
        "{name}: pnpm takes its version from packageManager in editor-ui"
    );
    let node = step_of(job, "actions/setup-node")?;
    assert_eq!(
        string_map(&node["with"])?,
        map(&[
            ("node-version", "24"),
            ("cache", "pnpm"),
            ("cache-dependency-path", "editor-ui/pnpm-lock.yaml"),
        ]),
        "{name}: Node"
    );
    let steps = job.steps()?;
    let position = |step: &Yaml| steps.iter().position(|other| other == step);
    assert!(
        position(pnpm) < position(node),
        "{name}: setup-node reads the pnpm store, so pnpm comes first"
    );
    Ok(())
}

// AC: CE-08.3
#[test]
fn ci_has_a_ui_job() -> TestResult {
    let job = ci_named("ui")?;
    assert_eq!(job.id, "ui");
    assert_eq!(job.runs_on()?, "ubuntu-latest");
    let steps = job.steps()?;
    assert_eq!(steps[1]["uses"].as_str(), Some(XTASK_USES));
    // The cached binary answers whether the run needs `ui`, before anything is installed.
    assert_eq!(steps[2]["id"].as_str(), Some("needs"));
    assert_eq!(steps[2]["run"].as_str(), Some(NEEDS_UI_CACHED));
    assert_eq!(
        steps[2]["if"],
        Yaml::BadValue,
        "the question is always asked"
    );
    assert_node_and_pnpm(&job)?;

    // The browsers of Playwright are cached by the lockfile.
    let cache = step_of(&job, "actions/cache")?;
    assert_eq!(
        string_map(&cache["with"])?,
        map(&[
            ("path", "~/.cache/ms-playwright"),
            (
                "key",
                "playwright-${{ runner.os }}-${{ hashFiles('editor-ui/pnpm-lock.yaml') }}"
            ),
        ])
    );

    // The last step is the check, from the cached binary and only when it is needed; the step
    // before it installs the browser with its system libraries. Every step after the question
    // waits for the answer.
    let last = steps.last().ok_or("the ui job has no steps")?;
    assert_eq!(last["run"].as_str(), Some(UI_CACHED));
    assert_eq!(last["if"].as_str(), Some(UI_RUN_IF));
    for step in &steps[3..] {
        let condition = step["if"].as_str().unwrap_or_default();
        assert!(
            condition.starts_with("steps.needs.outputs.needed == 'true'"),
            "a step of the ui job runs without asking whether ui is needed: {condition:?}"
        );
    }
    assert_eq!(job.commands()?, commands_of("ci.yml", "ui"));
    Ok(())
}

// AC: CE-08.3
#[test]
fn rust_jobs_install_the_gui_packages() -> TestResult {
    for (name, checks) in GUI_JOBS {
        let job = ci_named(name)?;
        let inputs = setup_inputs(&job)?.ok_or("no setup step")?;
        assert_eq!(
            inputs.get("gui-packages").map(String::as_str),
            Some(checks),
            "ci.yml: job {name} builds the Tauri crates without their Linux packages, or asks \
             about other checks than the ones that build"
        );
    }
    // Only the jobs that build the GUI install them: they cost time and the others do not need
    // them.
    for (file, job) in all_jobs()? {
        if file == "ci.yml" && gui_packages_of(&job.name).is_some() {
            continue;
        }
        let inputs = setup_inputs(&job)?.unwrap_or_default();
        assert!(
            !inputs.contains_key("gui-packages"),
            "{file}: job {} installs the GUI packages",
            job.name
        );
    }
    Ok(())
}

// AC: CE-08.3
#[test]
fn setup_installs_packages_from_the_list() -> TestResult {
    let list = read("gate/linux-packages.txt")?;
    assert_eq!(list.lines().collect::<Vec<_>>(), LINUX_PACKAGES);
    assert!(!list.contains('\r'), "the list has CRLF line ends");

    let action = parse(SETUP_ACTION)?;
    let steps = action["runs"]["steps"]
        .as_vec()
        .ok_or("the action has no steps")?;
    // The tail of the action: the question, the image, the folder, the restore, the install and
    // the save. The question is asked only for a job that passes checks, and answers through the
    // step output `needed`, which every later step reads.
    let ask = &steps[5];
    assert_eq!(ask["id"].as_str(), Some("needs"));
    assert_eq!(
        string_map(&ask["env"])?,
        map(&[("NEEDS", "${{ inputs.gui-packages }}")])
    );
    assert_eq!(
        ask["run"].as_str(),
        Some("cargo xtask needs --requires-package webkit2gtk \"$NEEDS\"")
    );
    let image = &steps[6];
    assert_eq!(image["id"].as_str(), Some("image"));
    let image_command = image["run"].as_str().ok_or("the image step runs nothing")?;
    for variable in ["ImageOS", "ImageVersion"] {
        assert!(image_command.contains(variable), "{image_command}");
    }
    assert!(image_command.contains("$GITHUB_OUTPUT"), "{image_command}");

    // The apt cache folder belongs to root; the cache action runs as the runner user.
    let folder = steps[7]["run"].as_str().ok_or("no folder step")?;
    assert!(folder.contains("chown") && folder.contains("/var/cache/apt/archives"));

    // The cache: the downloaded packages, by the runner image and the list.
    let restore = &steps[8];
    assert_eq!(action_name(restore), Some("actions/cache/restore"));
    assert_eq!(restore["id"].as_str(), Some("packages"));
    let key = "linux-packages-${{ runner.os }}-${{ steps.image.outputs.image }}-${{ \
               hashFiles('gate/linux-packages.txt') }}";
    assert_eq!(
        string_map(&restore["with"])?,
        map(&[("path", "/var/cache/apt/archives/*.deb"), ("key", key)])
    );

    // The install takes the packages from the file and no recommended ones.
    let install = steps[9]["run"].as_str().ok_or("no install step")?;
    assert!(
        install.contains("apt-get install --no-install-recommends"),
        "{install}"
    );
    assert!(install.contains("gate/linux-packages.txt"), "{install}");

    // The save runs only when the job saves its caches, with the key and path of the restore.
    let save = &steps[10];
    // Only the step that asks has `env`.
    assert_eq!(
        steps
            .iter()
            .filter(|step| !step["env"].is_badvalue())
            .count(),
        1
    );
    assert_eq!(action_name(save), Some("actions/cache/save"));
    assert_eq!(save["if"].as_str(), Some(GUI_SAVE_IF));
    assert_eq!(string_map(&save["with"])?, string_map(&restore["with"])?);
    Ok(())
}

// AC: CE-08C.6
#[test]
fn platforms_run_ui_on_windows_and_macos() -> TestResult {
    let jobs = jobs(&workflow("platforms.yml")?)?;
    for name in ["windows", "macos"] {
        let job = &jobs[index_of(&jobs, name)?];
        assert!(
            selection_of(&job.gate_command()?)?
                .checks()
                .contains(&Check::Ui),
            "{name} does not run ui"
        );
        assert_node_and_pnpm(job)?;
        // The question comes before the setup of Node, and every step of it waits for the answer.
        let steps = job.steps()?;
        assert_eq!(steps[2]["id"].as_str(), Some("needs"));
        assert_eq!(steps[2]["run"].as_str(), Some(NEEDS_UI_CARGO));
        for step in &steps[3..6] {
            assert_eq!(step["if"].as_str(), Some(UI_EXISTS_IF), "{name}");
        }
    }
    // Intel macOS runs the tests only.
    let intel = &jobs[index_of(&jobs, "macos-intel")?];
    assert!(
        !selection_of(&intel.gate_command()?)?
            .checks()
            .contains(&Check::Ui)
    );
    Ok(())
}

// AC: CE-08.3
#[test]
fn ruleset_requires_ui() -> TestResult {
    let ruleset: serde_json::Value = serde_json::from_str(&read("gate/ruleset-main.json")?)?;
    let rules = ruleset["rules"].as_array().ok_or("no rules")?;
    let status_checks = rules
        .iter()
        .find(|rule| rule["type"] == "required_status_checks")
        .ok_or("no required_status_checks rule")?;
    let checks = status_checks["parameters"]["required_status_checks"]
        .as_array()
        .ok_or("required_status_checks is not a list")?;
    assert!(
        checks.iter().any(|check| check["context"] == "ui"),
        "the ruleset does not require ui"
    );
    Ok(())
}
