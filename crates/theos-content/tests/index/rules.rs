//! Every rule P04 adds has a finding in some fixture project, and every finding carries a rule, a
//! severity, a path and a position (spec §5.18 CE-14.2), also those on the input files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use theos_content::rules::{self, RuleInfo};
use theos_content::{Finding, Index, Project, formalism, parts};

use crate::common::{TempProject, fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The rule groups of P04 (spec §5.2). Of `link/`, only the parent cycle is P04's.
const GROUPS: [&str; 8] = [
    "body",
    "world",
    "story",
    "lore",
    "version",
    "binary",
    "formalism",
    "settings",
];
const PARENT_CYCLE: &str = "link/parent-cycle";

const SETTINGS: &str = ".theos/settings.toml";
const SOURCE: &str = "canon/formalism/_source.json";
const E7: &str = "canon/formalism/E7.md";

fn is_p04(rule: &RuleInfo) -> bool {
    GROUPS.contains(&rule.group()) || rule.id == PARENT_CYCLE
}

/// The findings of a fresh index over the folder `root`.
fn findings_in(root: &Path) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&Project::open(root)?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index.findings()?)
}

/// The binaries folder of the `binaries/` fixture project.
fn binaries_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("binaries")
}

/// The stored findings of the `binaries/` fixture and those `binary_findings` gives on request.
fn binaries() -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("binaries"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    let mut found = index.findings()?;
    found.extend(index.binary_findings(&binaries_folder())?.findings);
    Ok(found)
}

/// The findings of the `formalism/` fixture after regeneration, then after a hand edit of E7, a
/// stray E99 and a missing E2, then with a source that is not valid JSON.
fn formalism_mirror() -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("formalism")?;
    formalism::regenerate(project.root())?;
    let mut found = findings_in(project.root())?;
    let e7 = std::fs::read_to_string(project.root().join(E7))?;
    project.write(
        E7,
        &e7.replace("Synthetic equation 7", "Synthetic equation seven"),
    )?;
    project.write("canon/formalism/E99.md", &e7.replace("E7", "E99"))?;
    project.remove("canon/formalism/E2.md")?;
    found.extend(findings_in(project.root())?);
    project.write(SOURCE, "{\n  \"format\": 1,\n  oops\n}\n")?;
    found.extend(findings_in(project.root())?);
    Ok(found)
}

/// The findings of the `settings/` fixture with a settings file that is not valid TOML: the one
/// row of spec §5.12 that the fixture's own file cannot hold.
fn broken_settings() -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("settings")?;
    project.write(SETTINGS, "[validate.severity\n")?;
    findings_in(project.root())
}

/// The findings of every fixture project that holds P04's findings: the P04 projects, the input
/// file cases above, and P02's corpus, whose `malformed/` records hold the malformed link and
/// beat forms.
fn every_finding() -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    for name in ["body", "lore", "world", "story", "versions", "settings"] {
        found.extend(findings_in(&fixture(name))?);
    }
    found.extend(binaries()?);
    found.extend(formalism_mirror()?);
    found.extend(broken_settings()?);
    found.extend(findings_in(TempProject::corpus()?.root())?);
    Ok(found)
}

// AC: CE-14.2
#[test]
fn every_rule_has_a_finding_with_rule_severity_path_and_line() -> TestResult {
    let found = every_finding()?;

    let seen: BTreeSet<&str> = found.iter().map(|f| f.rule.as_str()).collect();
    let missing: Vec<&str> = rules::all(&parts::builtin()?)
        .iter()
        .filter(|rule| is_p04(rule))
        .map(|rule| rule.id)
        .filter(|id| !seen.contains(id))
        .collect();
    assert_eq!(missing, Vec::<&str>::new(), "P04 rules without a finding");

    let incomplete: Vec<String> = found
        .iter()
        .filter(|f| {
            f.rule.split_once('/').is_none()
                || f.path.is_empty()
                || !f.position.is_some_and(|at| at.line >= 1 && at.column >= 1)
        })
        .map(|f| format!("{f:?}"))
        .collect();
    assert_eq!(incomplete, Vec::<String>::new(), "findings without a place");

    // The input files are no records, and their findings are placed like any other.
    for input in [SETTINGS, SOURCE] {
        assert!(
            found.iter().any(|f| f.path == input),
            "no finding on {input}"
        );
    }
    Ok(())
}
