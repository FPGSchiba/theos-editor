//! `cargo xtask gate`: every local check of `docs/gate.md` §1, in a fixed order, and one table.
//!
//! The gate calls the same per-check runners as the subcommands do, with no override flags, so a
//! check gives the same verdict here as in its CI job (CE-08.4); only where the inputs come from
//! differs (the repository instead of the event payload). It leaves out the checks that need a
//! pull request (`pr-title`, `pr-body`, `approval`) and the settings check, which needs GitHub.
//! `slow-tests` always runs its tests here, as on a push to `main`; only on a pull request that
//! cannot change their outcome does it pass without running them ([`slow`]), so the gate is never
//! more lenient than CI.
//!
//! Versions decide verdicts, so a gate whose tools are not the pinned ones would give verdicts
//! that CI does not. When the `tools` check fails, nothing else runs.

use std::io::Write;
use std::path::Path;

use crate::checks::{
    ac_trace, cargo_cmds, commit_msg, coverage, engine_free, lints, pr_body, pr_title, slow,
};
use crate::ci::CiEvent;
use crate::scope::{self, Scope};
use crate::verdict::{Check, Finding, GATE_ORDER, PR_ONLY, Verdict};
use crate::view::Shared;
use crate::{Error, Result, report, tools};

/// Which checks of [`GATE_ORDER`] a run covers: the CI jobs split the gate with this.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Selection {
    /// Run only these checks (and `tools`); empty means every check.
    pub only: Vec<Check>,
    /// Leave these checks out.
    pub skip: Vec<Check>,
}

impl Selection {
    /// The checks to run, in order: `tools` first, then [`GATE_ORDER`] without it, each kept when
    /// `only` is empty or names it, and dropped when `skip` names it.
    #[must_use]
    pub fn checks(&self) -> Vec<Check> {
        let rest = GATE_ORDER
            .iter()
            .copied()
            .filter(|&check| check != Check::Tools)
            .filter(|check| self.only.is_empty() || self.only.contains(check))
            .filter(|check| !self.skip.contains(check));
        std::iter::once(Check::Tools).chain(rest).collect()
    }
}

/// Parses a check name of `--only` or `--skip`: the checks of the gate, not `tools` (it always
/// runs) and not the checks that need a pull request or GitHub.
///
/// # Errors
///
/// Returns the message to show when `name` is not a check or is not selectable.
pub fn parse_selectable(name: &str) -> std::result::Result<Check, String> {
    let check = Check::from_name(name).ok_or_else(|| format!("`{name}` is not a check"))?;
    if check == Check::Tools {
        return Err("`tools` always runs first and cannot be selected".to_owned());
    }
    if PR_ONLY.contains(&check) || check == Check::Github {
        return Err(format!(
            "`{name}` is not part of the gate: it needs a pull request or GitHub"
        ));
    }
    Ok(check)
}

/// Runs the checks of `selection` in `root`, and writes the verdict table to `out` after the
/// output of the checks themselves. Returns the verdicts; the gate passes when all of them do.
///
/// The scope is decided once ([`scope::for_run`]): when the change touches only paths no check
/// reads, every selected check but `commit-msg` is not needed and is not run. A check that fails
/// does not stop the others, and a check that cannot run is a failing verdict that carries the
/// error. Only a failing `tools` check stops the gate, as the module documentation says: the
/// checks after it are not run, and its verdict notes which.
///
/// # Errors
///
/// Fails only when `out` cannot be written.
pub fn run(
    root: &Path,
    ci: Option<&CiEvent>,
    selection: &Selection,
    out: &mut dyn Write,
) -> Result<Vec<Verdict>> {
    // The checks that read code share one view, built when the first of them asks for it.
    let shared = Shared::new();
    run_scoped(
        &selection.checks(),
        &scope::for_run(root, ci),
        |check, out| run_check(check, root, ci, &shared, out),
        out,
    )
}

/// The checks that need a pull request and are run by `cargo xtask pr`.
pub const PR_CHECKS: [Check; 2] = [Check::PrTitle, Check::PrBody];

/// Runs `pr-title` and `pr-body` and writes one table. Needs no `tools` check and has no scope.
///
/// # Errors
///
/// Fails only when `out` cannot be written.
pub fn run_pr(root: &Path, ci: Option<&CiEvent>, out: &mut dyn Write) -> Result<Vec<Verdict>> {
    run_checks(
        &PR_CHECKS,
        |check, out| match check {
            Check::PrTitle => pr_title::run(root, ci, out),
            _ => pr_body::run(root, ci, out),
        },
        out,
    )
}

/// Applies `scope` to `run_one`, then runs the checks. See [`run`].
fn run_scoped(
    order: &[Check],
    scope: &Scope,
    mut run_one: impl FnMut(Check, &mut dyn Write) -> Result<Verdict>,
    out: &mut dyn Write,
) -> Result<Vec<Verdict>> {
    if let Scope::Every { note: Some(note) } = scope {
        writeln!(out, "note: every check runs: {note}").map_err(report::output_error)?;
    }
    let docs_only = *scope == Scope::DocsOnly;
    run_checks(
        order,
        |check, out| {
            if docs_only && check != Check::CommitMsg {
                Ok(Verdict::not_needed(check, scope::NOT_NEEDED_REASON))
            } else {
                run_one(check, out)
            }
        },
        out,
    )
}

/// The runner of one gate check: the function its subcommand calls.
///
/// There is no wildcard arm, so a new [`Check`] does not compile until it is decided here.
fn run_check(
    check: Check,
    root: &Path,
    ci: Option<&CiEvent>,
    shared: &Shared,
    out: &mut dyn Write,
) -> Result<Verdict> {
    match check {
        Check::Tools => tools::run(root, ci, out),
        Check::Fmt | Check::Clippy | Check::Test | Check::Docs | Check::Deny => {
            cargo_cmds::run(check, root, ci, out)
        }
        Check::Lints => lints::run(root, ci, shared, out),
        Check::EngineFree => engine_free::run(root, ci, out),
        Check::AcTrace => ac_trace::run(root, ci, shared, out),
        Check::CommitMsg => commit_msg::run(root, ci, out),
        Check::Coverage => coverage::run(root, ci, out),
        Check::SlowTests => slow::run(root, ci, out),
        Check::PrTitle | Check::PrBody | Check::Approval | Check::Github => Err(Error::Missing {
            what: format!(
                "a way to run the `{}` check in the gate: it needs a pull request or GitHub",
                check.name()
            ),
        }),
    }
}

/// Runs the checks of `order` with `run_one`, then writes the table. See [`run`].
fn run_checks(
    order: &[Check],
    mut run_one: impl FnMut(Check, &mut dyn Write) -> Result<Verdict>,
    out: &mut dyn Write,
) -> Result<Vec<Verdict>> {
    let mut verdicts = Vec::new();
    for (position, &check) in order.iter().enumerate() {
        // Cargo writes to the terminal on its own, so the heading goes out before it starts.
        writeln!(out, "==> {}", check.name()).map_err(report::output_error)?;
        out.flush().map_err(report::output_error)?;
        let mut verdict = run_one(check, out).unwrap_or_else(|error| could_not_run(check, &error));
        let stop = check == Check::Tools && !verdict.passed();
        if stop && position + 1 < order.len() {
            let skipped: Vec<&str> = order[position + 1..]
                .iter()
                .map(|skipped| skipped.name())
                .collect();
            verdict.notes.push(format!(
                "not run: {}; install the pinned tools first",
                skipped.join(", ")
            ));
        }
        verdicts.push(verdict);
        if stop {
            break;
        }
    }
    writeln!(out, "==> gate").map_err(report::output_error)?;
    report::write_table(&verdicts, out).map_err(report::output_error)?;
    out.flush().map_err(report::output_error)?;
    Ok(verdicts)
}

/// The verdict of a check that stopped with an error: one finding with the error text.
fn could_not_run(check: Check, error: &Error) -> Verdict {
    Verdict::with(check, vec![Finding::new(format!("could not run: {error}"))])
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Where a check runs.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Place {
        /// In the gate, after the version check.
        Gate,
        /// Only on a pull request; the gate leaves it out.
        PrOnly,
        /// Neither: the version check that opens the gate, and the settings check that needs
        /// GitHub.
        Apart,
    }

    /// Lists every check once, and builds `ALL` and `place` from that list. `place` has no
    /// wildcard arm, so a new `Check` does not compile here until it is placed.
    macro_rules! placed {
        ($($check:ident => $place:ident),+ $(,)?) => {
            const ALL: &[Check] = &[$(Check::$check),+];

            fn place(check: Check) -> Place {
                match check {
                    $(Check::$check => Place::$place),+
                }
            }
        };
    }

    placed! {
        Tools => Apart,
        Fmt => Gate,
        Clippy => Gate,
        Test => Gate,
        SlowTests => Gate,
        Docs => Gate,
        Deny => Gate,
        EngineFree => Gate,
        Coverage => Gate,
        Lints => Gate,
        AcTrace => Gate,
        CommitMsg => Gate,
        PrTitle => PrOnly,
        PrBody => PrOnly,
        Approval => PrOnly,
        Github => Apart,
    }

    fn names(checks: &[Check]) -> Vec<&'static str> {
        checks.iter().map(|check| check.name()).collect()
    }

    // AC: CE-08.4
    #[test]
    fn gate_runs_every_ci_check_except_pr_only() {
        let after_tools: Vec<Check> = GATE_ORDER
            .iter()
            .copied()
            .filter(|check| *check != Check::Tools)
            .collect();
        assert_eq!(
            GATE_ORDER.first(),
            Some(&Check::Tools),
            "tools opens the gate"
        );
        for &check in ALL {
            let expected = match place(check) {
                Place::Gate => (true, false),
                Place::PrOnly => (false, true),
                Place::Apart => (false, false),
            };
            assert_eq!(
                (after_tools.contains(&check), PR_ONLY.contains(&check)),
                expected,
                "{} is not where `place` says",
                check.name()
            );
        }
        // No duplicates in or between the lists, and nothing in a list that is not a check.
        let apart: Vec<Check> = ALL
            .iter()
            .copied()
            .filter(|check| place(*check) == Place::Apart)
            .collect();
        assert_eq!(names(&apart), ["tools", "github"]);
        assert_eq!(
            after_tools.len() + PR_ONLY.len() + apart.len(),
            ALL.len(),
            "every check is in exactly one of the gate, the PR-only checks and the set apart"
        );
    }

    fn fail(check: Check) -> Verdict {
        Verdict::with(
            check,
            vec![Finding::new(format!("{} is wrong", check.name()))],
        )
    }

    /// Runs the orchestration over `order` with `decide` as every check, and returns the checks
    /// that were run, the verdicts and everything written to the output.
    fn run_with(
        order: &[Check],
        mut decide: impl FnMut(Check) -> Result<Verdict>,
    ) -> TestResult<(Vec<Check>, Vec<Verdict>, String)> {
        let mut ran = Vec::new();
        let mut out = Vec::new();
        let verdicts = run_checks(
            order,
            |check, out| {
                ran.push(check);
                writeln!(out, "output of {}", check.name()).map_err(|source| Error::Io {
                    path: "<test>".into(),
                    source,
                })?;
                decide(check)
            },
            &mut out,
        )?;
        Ok((ran, verdicts, String::from_utf8(out)?))
    }

    fn checks_of(verdicts: &[Verdict]) -> Vec<Check> {
        verdicts.iter().map(|verdict| verdict.check).collect()
    }

    fn only_failing(failing: &'static [Check]) -> impl FnMut(Check) -> Result<Verdict> {
        move |check| {
            Ok(if failing.contains(&check) {
                fail(check)
            } else {
                Verdict::pass(check)
            })
        }
    }

    #[test]
    fn checks_run_in_gate_order_and_all_pass() -> TestResult {
        let (ran, verdicts, _) = run_with(&GATE_ORDER, only_failing(&[]))?;
        assert_eq!(ran, GATE_ORDER);
        assert_eq!(checks_of(&verdicts), GATE_ORDER);
        assert!(verdicts.iter().all(Verdict::passed));
        Ok(())
    }

    #[test]
    fn a_failing_check_does_not_stop_the_rest() -> TestResult {
        let (ran, verdicts, _) = run_with(&GATE_ORDER, only_failing(&[Check::Fmt, Check::Clippy]))?;
        assert_eq!(ran, GATE_ORDER);
        let failed: Vec<&str> = verdicts
            .iter()
            .filter(|verdict| !verdict.passed())
            .map(|verdict| verdict.check.name())
            .collect();
        assert_eq!(failed, ["fmt", "clippy"]);
        Ok(())
    }

    #[test]
    fn a_failing_tools_check_skips_the_rest() -> TestResult {
        let (ran, verdicts, out) = run_with(&GATE_ORDER, only_failing(&[Check::Tools]))?;
        assert_eq!(ran, [Check::Tools]);
        assert_eq!(checks_of(&verdicts), [Check::Tools]);
        assert!(!verdicts[0].passed());
        let [note] = verdicts[0].notes.as_slice() else {
            return Err(format!("expected one note, got {:?}", verdicts[0].notes).into());
        };
        assert!(note.contains("install the pinned tools first"), "{note}");
        for skipped in &GATE_ORDER[1..] {
            assert!(
                note.contains(skipped.name()),
                "{} missing from {note}",
                skipped.name()
            );
        }
        assert!(out.contains("FAIL  tools"), "{out}");
        assert!(!out.contains("output of fmt"), "{out}");
        Ok(())
    }

    #[test]
    fn an_erroring_tools_check_skips_the_rest_too() -> TestResult {
        let (ran, verdicts, _) = run_with(&GATE_ORDER, |check| {
            if check == Check::Tools {
                Err(Error::Missing {
                    what: "gate/tools.toml".to_owned(),
                })
            } else {
                Ok(Verdict::pass(check))
            }
        })?;
        assert_eq!(ran, [Check::Tools]);
        assert!(!verdicts[0].passed());
        assert!(
            verdicts[0].notes[0].contains("install the pinned tools first"),
            "{:?}",
            verdicts[0].notes
        );
        Ok(())
    }

    #[test]
    fn an_error_becomes_a_failing_verdict_with_its_text() -> TestResult {
        let (ran, verdicts, out) = run_with(&GATE_ORDER, |check| {
            if check == Check::Lints {
                Err(Error::Missing {
                    what: "the lint list".to_owned(),
                })
            } else {
                Ok(Verdict::pass(check))
            }
        })?;
        assert_eq!(ran, GATE_ORDER, "the checks after an error still run");
        let failed: Vec<&Verdict> = verdicts
            .iter()
            .filter(|verdict| !verdict.passed())
            .collect();
        let [lints] = failed.as_slice() else {
            return Err(format!("expected one failure, got {failed:?}").into());
        };
        assert_eq!(lints.check, Check::Lints);
        let [finding] = lints.findings.as_slice() else {
            return Err(format!("expected one finding, got {:?}", lints.findings).into());
        };
        assert!(
            finding.message.contains("missing the lint list"),
            "{}",
            finding.message
        );
        assert!(out.contains("FAIL  lints"), "{out}");
        Ok(())
    }

    #[test]
    fn each_checks_output_comes_first_and_the_table_last() -> TestResult {
        let (_, verdicts, out) = run_with(&GATE_ORDER, only_failing(&[Check::Docs]))?;
        let mut table = Vec::new();
        report::write_table(&verdicts, &mut table)?;
        let table = String::from_utf8(table)?;
        assert!(out.ends_with(&table), "{out}");
        let before_table = &out[..out.len() - table.len()];
        for check in GATE_ORDER {
            assert!(
                before_table.contains(&format!("output of {}\n", check.name())),
                "{} wrote no output before the table: {out}",
                check.name()
            );
        }
        assert!(table.contains("FAIL  docs\n  - docs is wrong\n"), "{table}");
        Ok(())
    }

    #[test]
    fn a_check_that_needs_a_pull_request_is_not_run_by_the_gate() {
        let mut out = Vec::new();
        for check in PR_ONLY.into_iter().chain([Check::Github]) {
            let result = run_check(check, Path::new("."), None, &Shared::new(), &mut out);
            assert!(
                matches!(result, Err(Error::Missing { .. })),
                "{check:?}: {result:?}"
            );
        }
        assert_eq!(
            out,
            Vec::<u8>::new(),
            "nothing is written for a check that is not run"
        );
    }

    fn selection(only: &[Check], skip: &[Check]) -> Selection {
        Selection {
            only: only.to_vec(),
            skip: skip.to_vec(),
        }
    }

    // AC: CE-08C.1
    #[test]
    fn only_and_skip_choose_the_checks_after_tools() {
        assert_eq!(Selection::default().checks(), GATE_ORDER);
        assert_eq!(
            selection(&[Check::Test], &[]).checks(),
            [Check::Tools, Check::Test]
        );
        let expected: Vec<Check> = GATE_ORDER
            .iter()
            .copied()
            .filter(|check| !matches!(check, Check::Test | Check::Coverage))
            .collect();
        assert_eq!(
            selection(&[], &[Check::Test, Check::Coverage]).checks(),
            expected
        );
        assert_eq!(expected.first(), Some(&Check::Tools));
    }

    // AC: CE-08C.1
    #[test]
    fn the_three_ci_jobs_cover_the_gate_once() {
        let jobs = [
            selection(&[], &[Check::Test, Check::Coverage]),
            selection(&[Check::Test], &[]),
            selection(&[Check::Coverage], &[]),
        ];
        let mut covered: Vec<Check> = jobs
            .iter()
            .flat_map(Selection::checks)
            .filter(|check| *check != Check::Tools)
            .collect();
        let mut expected: Vec<Check> = GATE_ORDER
            .iter()
            .copied()
            .filter(|check| *check != Check::Tools)
            .collect();
        covered.sort_by_key(|check| check.name());
        expected.sort_by_key(|check| check.name());
        assert_eq!(covered, expected);
    }

    // AC: CE-08C.2
    #[test]
    fn docs_only_runs_commit_msg_and_marks_the_rest_not_needed() -> TestResult {
        let mut ran = Vec::new();
        let mut out = Vec::new();
        let verdicts = run_scoped(
            &GATE_ORDER,
            &Scope::DocsOnly,
            |check, _| {
                ran.push(check);
                Ok(Verdict::pass(check))
            },
            &mut out,
        )?;
        assert_eq!(ran, [Check::CommitMsg]);
        assert_eq!(checks_of(&verdicts), GATE_ORDER);
        for verdict in &verdicts {
            assert_eq!(
                verdict.is_not_needed(),
                verdict.check != Check::CommitMsg,
                "{}",
                verdict.check.name()
            );
        }
        assert!(String::from_utf8(out)?.contains("not needed"));
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn docs_only_in_a_test_job_runs_nothing() -> TestResult {
        let mut out = Vec::new();
        let verdicts = run_scoped(
            &selection(&[Check::Test], &[]).checks(),
            &Scope::DocsOnly,
            |check, _| {
                Err(Error::Missing {
                    what: format!("a run of {}", check.name()),
                })
            },
            &mut out,
        )?;
        assert_eq!(checks_of(&verdicts), [Check::Tools, Check::Test]);
        assert!(verdicts.iter().all(Verdict::is_not_needed));
        Ok(())
    }

    #[test]
    fn a_scope_with_a_note_prints_it_before_the_table() -> TestResult {
        let scope = Scope::Every {
            note: Some("HEAD is detached".to_owned()),
        };
        let mut out = Vec::new();
        let verdicts = run_scoped(
            &[Check::Fmt],
            &scope,
            |check, _| Ok(Verdict::pass(check)),
            &mut out,
        )?;
        let out = String::from_utf8(out)?;
        assert_eq!(checks_of(&verdicts), [Check::Fmt]);
        assert!(
            out.starts_with("note: every check runs: HEAD is detached\n"),
            "{out}"
        );
        Ok(())
    }

    // AC: CE-08C.1
    #[test]
    fn pr_runs_title_and_body_in_one_table() -> TestResult {
        // Outside a pull request neither check can read its input: both are failing verdicts.
        let mut out = Vec::new();
        let verdicts = run_pr(Path::new("."), None, &mut out)?;
        assert_eq!(checks_of(&verdicts), PR_CHECKS);
        let out = String::from_utf8(out)?;
        assert!(
            out.contains("==> pr-title") && out.contains("==> pr-body"),
            "{out}"
        );
        assert_eq!(out.matches("==> gate").count(), 1, "{out}");
        Ok(())
    }

    #[test]
    fn unknown_or_pr_only_names_are_rejected() {
        assert_eq!(parse_selectable("test"), Ok(Check::Test));
        for name in ["approval", "pr-title", "pr-body", "github", "tools"] {
            assert!(parse_selectable(name).is_err(), "{name}");
        }
        assert_eq!(
            parse_selectable("nope"),
            Err("`nope` is not a check".to_owned())
        );
    }

    /// A writer that refuses everything.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn an_output_that_cannot_be_written_is_an_error() {
        let result = run_checks(
            &GATE_ORDER,
            |check, _| Ok(Verdict::pass(check)),
            &mut Closed,
        );
        assert!(matches!(result, Err(Error::Io { .. })), "{result:?}");
    }
}
