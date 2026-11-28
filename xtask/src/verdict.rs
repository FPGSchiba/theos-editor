//! What a check returns: a verdict made of findings, and the order the gate runs the checks in.
//!
//! A check is a pure core (inputs in, [`Verdict`] out) with the real git, cargo and GitHub wired
//! in elsewhere. A verdict passes exactly when it has no findings.

/// A check of `docs/gate.md` §1. Each one is a `cargo xtask <name>` subcommand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Check {
    /// The tool versions match `gate/tools.toml`.
    Tools,
    /// `cargo fmt --all --check`.
    Fmt,
    /// `cargo clippy` with warnings denied.
    Clippy,
    /// The tests, through nextest, and the doc tests; the slow tests are left out.
    Test,
    /// The slow tests (a segment of the name starts with `slow_`), through nextest.
    SlowTests,
    /// `cargo doc` with warnings denied.
    Docs,
    /// `cargo deny check`.
    Deny,
    /// No engine crate is a dependency without the feature `engine`.
    EngineFree,
    /// Line coverage per crate against floors and the baseline on `main`.
    Coverage,
    /// The lint list, the tool configuration and the attribute scan.
    Lints,
    /// Every acceptance criterion of the phase has a tagged test and every tag names one.
    AcTrace,
    /// The checks of `editor-ui/`: install, svelte-check, eslint, vitest and playwright.
    Ui,
    /// Every commit of the PR is a Conventional Commit.
    CommitMsg,
    /// The PR title is a Conventional Commit that matches the branch.
    PrTitle,
    /// The PR description has every section of the template.
    PrBody,
    /// A PR that touches a protected path carries the label `approved`.
    Approval,
    /// The rulesets, settings and labels of the repository on GitHub are those of `gate/`. Not a
    /// check of the gate: it needs GitHub and is run on its own.
    Github,
}

impl Check {
    /// The name of the check: its subcommand, its name in the verdict table and its row in
    /// `docs/gate.md` §1. In CI it runs inside a job that may have another name: `gate` runs most
    /// checks, `pr` runs `pr-title` and `pr-body`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Fmt => "fmt",
            Self::Clippy => "clippy",
            Self::Test => "test",
            Self::SlowTests => "slow-tests",
            Self::Docs => "docs",
            Self::Deny => "deny",
            Self::EngineFree => "engine-free",
            Self::Coverage => "coverage",
            Self::Lints => "lints",
            Self::AcTrace => "ac-trace",
            Self::Ui => "ui",
            Self::CommitMsg => "commit-msg",
            Self::PrTitle => "pr-title",
            Self::PrBody => "pr-body",
            Self::Approval => "approval",
            Self::Github => "github",
        }
    }

    /// The check called `name`: the inverse of [`Check::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "tools" => Self::Tools,
            "fmt" => Self::Fmt,
            "clippy" => Self::Clippy,
            "test" => Self::Test,
            "slow-tests" => Self::SlowTests,
            "docs" => Self::Docs,
            "deny" => Self::Deny,
            "engine-free" => Self::EngineFree,
            "coverage" => Self::Coverage,
            "lints" => Self::Lints,
            "ac-trace" => Self::AcTrace,
            "ui" => Self::Ui,
            "commit-msg" => Self::CommitMsg,
            "pr-title" => Self::PrTitle,
            "pr-body" => Self::PrBody,
            "approval" => Self::Approval,
            "github" => Self::Github,
            _ => return None,
        })
    }
}

/// The checks `cargo xtask gate` runs, in order: cheap and local ones first. `ac-trace` comes after
/// them, because it reads the reports of `test` and `slow-tests`; `ui` is last and needs Node.
pub const GATE_ORDER: [Check; 13] = [
    Check::Tools,
    Check::Fmt,
    Check::Lints,
    Check::EngineFree,
    Check::CommitMsg,
    Check::Clippy,
    Check::Docs,
    Check::Deny,
    Check::Test,
    Check::Coverage,
    Check::SlowTests,
    Check::AcTrace,
    Check::Ui,
];

/// The checks that need a pull request, which the gate leaves out.
pub const PR_ONLY: [Check; 3] = [Check::PrTitle, Check::PrBody, Check::Approval];

/// One thing a check found wrong, with where it is when that is known.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    /// The file, repo-relative with `/`.
    pub file: Option<String>,
    /// The line in the file, counting from 1.
    pub line: Option<u32>,
    /// What is wrong, and how to fix it.
    pub message: String,
}

impl Finding {
    /// A finding that is not tied to a place.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            file: None,
            line: None,
            message: message.into(),
        }
    }

    /// A finding at `line` of `file`.
    #[must_use]
    pub fn at(file: impl Into<String>, line: u32, message: impl Into<String>) -> Self {
        Self {
            file: Some(file.into()),
            line: Some(line),
            message: message.into(),
        }
    }
}

/// The outcome of one check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    /// The check that ran.
    pub check: Check,
    /// What it found wrong; empty when it passed.
    pub findings: Vec<Finding>,
    /// Remarks that do not fail the check, such as a crate with no instrumented lines.
    pub notes: Vec<String>,
    /// Why the check did not run because the change does not need it; such a verdict passes.
    pub not_needed: Option<String>,
}

impl Verdict {
    /// A verdict with nothing to report.
    #[must_use]
    pub const fn pass(check: Check) -> Self {
        Self {
            check,
            findings: Vec::new(),
            notes: Vec::new(),
            not_needed: None,
        }
    }

    /// A verdict with these findings. It passes if the list is empty.
    #[must_use]
    pub const fn with(check: Check, findings: Vec<Finding>) -> Self {
        Self {
            check,
            findings,
            notes: Vec::new(),
            not_needed: None,
        }
    }

    /// A verdict for a check the change does not need, with the reason. It passes.
    #[must_use]
    pub fn not_needed(check: Check, reason: impl Into<String>) -> Self {
        Self {
            not_needed: Some(reason.into()),
            ..Self::pass(check)
        }
    }

    /// True when the check was left out because the change does not need it.
    #[must_use]
    pub const fn is_not_needed(&self) -> bool {
        self.not_needed.is_some()
    }

    /// True when there are no findings.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Check; 17] = [
        Check::Tools,
        Check::Fmt,
        Check::Clippy,
        Check::Test,
        Check::SlowTests,
        Check::Docs,
        Check::Deny,
        Check::EngineFree,
        Check::Coverage,
        Check::Lints,
        Check::AcTrace,
        Check::Ui,
        Check::CommitMsg,
        Check::PrTitle,
        Check::PrBody,
        Check::Approval,
        Check::Github,
    ];

    fn names(checks: &[Check]) -> Vec<&'static str> {
        checks.iter().map(|check| check.name()).collect()
    }

    #[test]
    fn verdict_passed_iff_no_findings() {
        assert!(Verdict::pass(Check::Fmt).passed());
        assert!(Verdict::with(Check::Fmt, Vec::new()).passed());
        assert!(!Verdict::with(Check::Fmt, vec![Finding::new("drifted")]).passed());
    }

    #[test]
    fn check_names_round_trip() {
        for check in ALL {
            assert_eq!(Check::from_name(check.name()), Some(check));
        }
        assert_eq!(Check::from_name("nope"), None);
    }

    #[test]
    fn a_not_needed_verdict_passes_and_knows_its_reason() {
        let verdict = Verdict::not_needed(Check::Test, "docs only");
        assert!(verdict.passed());
        assert!(verdict.is_not_needed());
        assert_eq!(verdict.not_needed.as_deref(), Some("docs only"));
        assert!(!Verdict::pass(Check::Test).is_not_needed());
    }

    #[test]
    fn notes_do_not_fail_a_verdict() {
        let mut verdict = Verdict::pass(Check::Coverage);
        verdict
            .notes
            .push("theos-vcs has no instrumented lines".to_owned());
        assert!(verdict.passed());
    }

    #[test]
    fn findings_know_where_they_are() {
        let anywhere = Finding::new("broken");
        assert_eq!(anywhere.file, None);
        assert_eq!(anywhere.line, None);
        assert_eq!(anywhere.message, "broken");
        let here = Finding::at("src/lib.rs", 7, "broken");
        assert_eq!(here.file.as_deref(), Some("src/lib.rs"));
        assert_eq!(here.line, Some(7));
        assert_eq!(here.message, "broken");
    }

    #[test]
    fn check_names_are_the_subcommand_names() {
        assert_eq!(
            names(&ALL),
            [
                "tools",
                "fmt",
                "clippy",
                "test",
                "slow-tests",
                "docs",
                "deny",
                "engine-free",
                "coverage",
                "lints",
                "ac-trace",
                "ui",
                "commit-msg",
                "pr-title",
                "pr-body",
                "approval",
                "github",
            ]
        );
    }

    #[test]
    fn gate_runs_cheap_checks_first_and_leaves_out_the_pr_checks() {
        assert_eq!(
            names(&GATE_ORDER),
            [
                "tools",
                "fmt",
                "lints",
                "engine-free",
                "commit-msg",
                "clippy",
                "docs",
                "deny",
                "test",
                "coverage",
                "slow-tests",
                "ac-trace",
                "ui",
            ]
        );
        assert_eq!(names(&PR_ONLY), ["pr-title", "pr-body", "approval"]);
        for check in ALL {
            let in_gate = GATE_ORDER.contains(&check);
            let pr_only = PR_ONLY.contains(&check);
            if check == Check::Github {
                // The settings of the repository are compared with `cargo xtask github check`.
                assert!(!in_gate && !pr_only, "github is in a list of the gate");
                continue;
            }
            assert!(
                in_gate != pr_only,
                "{} is in both lists or neither",
                check.name()
            );
        }
    }
}
