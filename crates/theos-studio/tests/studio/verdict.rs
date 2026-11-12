//! CE-20.1, the verdict: a command that introduces an error is rejected and changes nothing;
//! standing errors and warnings do not block.

use theos_content::{Finding, Severity};
use theos_studio::{Command, CommandError, Create, Workspace, Write};

use crate::common::{
    CLUE, Error, SCRIPT, SECOND_CLUE, STANDING_SCRIPT, TRUTH, TempProject, TestResult, append,
    base_of, holds, open, refused, set,
};

const SETTINGS: &str = ".theos/settings.toml";

const STANDING_TRUTH: &str = "lore/truths/T-980.md";
const STANDING_CLUE: &str = "lore/clues/C-980.md";
const CLUELESS_TRUTH: &str = "lore/truths/T-981.md";

/// A write of `text` over the settings file.
fn settings(workspace: &Workspace, text: &str) -> Result<Write, Error> {
    Ok(Write {
        path: SETTINGS.to_owned(),
        base: base_of(workspace, SETTINGS)?,
        text: text.to_owned(),
    })
}

/// Runs `command`, which must be rejected, and checks that it changed nothing, as [`refused`]
/// does. Returns the blocking findings.
fn rejected(
    project: &TempProject,
    workspace: &mut Workspace,
    command: &dyn Command,
) -> Result<Vec<Finding>, Error> {
    match refused(project, workspace, command)? {
        CommandError::Rejected { blocking, .. } => Ok(blocking),
        other => Err(format!("expected Rejected, got {other:?}").into()),
    }
}

/// Whether `findings` hold one of `rule` on `path` with `severity`.
fn holds_as(findings: &[Finding], path: &str, rule: &str, severity: Severity) -> bool {
    findings
        .iter()
        .any(|f| f.path == path && f.rule == rule && f.severity == severity)
}

/// The message of the finding of `rule` on `path`.
fn message_of(findings: &[Finding], path: &str, rule: &str) -> Result<String, Error> {
    let finding = findings
        .iter()
        .find(|f| f.path == path && f.rule == rule)
        .ok_or_else(|| format!("no {rule} on {path} in {findings:#?}"))?;
    Ok(finding.message.clone())
}

// AC: CE-20.1
#[test]
fn an_invalid_value_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "target", "maybe")?;
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(holds(&blocking, TRUTH, "schema/value"), "{blocking:#?}");
    Ok(())
}

// AC: CE-20.1
#[test]
fn a_dangling_link_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = append(
        &workspace,
        SCRIPT,
        "\nSynthetic text with [[synthetic-nowhere]].\n",
    )?;
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(holds(&blocking, SCRIPT, "link/dangling"), "{blocking:#?}");
    Ok(())
}

// AC: CE-20.1
#[test]
fn a_clue_that_reveals_a_never_truth_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, SECOND_CLUE, "reveals", "T-961")?;
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(
        holds(&blocking, SECOND_CLUE, "lore/never-confirm"),
        "{blocking:#?}"
    );
    Ok(())
}

// AC: CE-20.1
#[test]
fn a_neighbour_that_overshoots_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "target", "hinted")?;
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(holds(&blocking, CLUE, "lore/no-overshoot"), "{blocking:#?}");
    Ok(())
}

// AC: CE-20.1
#[test]
fn a_new_record_with_an_error_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let path = "lore/truths/T-962.md";
    let command = Create {
        path: path.to_owned(),
        text: "---\n# synthetic fixture (P06), not canon\nid: T-962\nkind: truth\n\
               title: Synthetic truth without a target\nstatus: draft\n---\n\
               Synthetic statement.\n"
            .to_owned(),
    };
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(holds(&blocking, path, "schema/required"), "{blocking:#?}");
    Ok(())
}

// AC: CE-20.1
#[test]
fn standing_errors_do_not_block() -> TestResult {
    let (_project, mut workspace) = open("standing")?;

    // A command on a record with a standing error.
    let command = set(
        &workspace,
        STANDING_SCRIPT,
        "title",
        "Synthetic script, retitled",
    )?;
    let outcome = workspace.run(&command)?;
    assert_eq!(outcome.changed, [STANDING_SCRIPT]);
    assert!(
        holds_as(
            &outcome.findings,
            STANDING_SCRIPT,
            "link/dangling",
            Severity::Error
        ),
        "{:#?}",
        outcome.findings
    );

    // A command whose neighbour has a standing error.
    let command = set(&workspace, STANDING_TRUTH, "target", "suspected")?;
    let outcome = workspace.run(&command)?;
    assert_eq!(outcome.changed, [STANDING_TRUTH]);
    assert!(
        outcome
            .update
            .revalidated
            .iter()
            .any(|path| path == STANDING_CLUE),
        "{:#?}",
        outcome.update
    );
    assert!(
        holds_as(
            &outcome.findings,
            STANDING_CLUE,
            "lore/no-overshoot",
            Severity::Error
        ),
        "{:#?}",
        outcome.findings
    );
    assert!(holds_as(
        &workspace.index().findings_for(STANDING_CLUE)?,
        STANDING_CLUE,
        "lore/no-overshoot",
        Severity::Error
    ));
    assert_eq!(workspace.undo_entries().len(), 2);
    Ok(())
}

// AC: CE-20.1
#[test]
fn a_changing_standing_message_does_not_block() -> TestResult {
    let (_project, mut workspace) = open("standing")?;
    let rule = "lore/three-clue";
    let before = workspace.index().findings_for(CLUELESS_TRUTH)?;
    assert!(holds_as(&before, CLUELESS_TRUTH, rule, Severity::Error));
    let path = "lore/clues/C-981.md";
    let command = Create {
        path: path.to_owned(),
        text: "---\n# synthetic fixture (P06), not canon\nid: C-981\nkind: clue\n\
               title: Synthetic clue of the clueless truth\nstatus: draft\nreveals: T-981\n\
               moves_to: hinted\nwhere: [f04-synthetic]\ngate: T1\nvector: synthetic vector\n\
               ---\nSynthetic clue text.\n"
            .to_owned(),
    };
    let outcome = workspace.run(&command)?;
    assert_eq!(outcome.changed, [path]);
    let message_before = message_of(&before, CLUELESS_TRUTH, rule)?;
    let message_after = message_of(&outcome.findings, CLUELESS_TRUTH, rule)?;
    assert!(message_before.starts_with("No clue"), "{message_before}");
    assert!(
        message_after.starts_with("Only one clue"),
        "{message_after}"
    );
    assert!(holds_as(
        &outcome.findings,
        CLUELESS_TRUTH,
        rule,
        Severity::Error
    ));
    Ok(())
}

// AC: CE-20.1
#[test]
fn lowering_a_guardrail_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = settings(
        &workspace,
        "[validate.severity]\n\"lore/never-confirm\" = \"warning\"\n",
    )?;
    let blocking = rejected(&project, &mut workspace, &command)?;
    assert!(
        holds(&blocking, SETTINGS, "settings/guardrail"),
        "{blocking:#?}"
    );
    Ok(())
}

// AC: CE-20.1
#[test]
fn raising_a_rule_to_error_is_applied() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let rule = "lore/three-clue";
    assert!(holds_as(
        &workspace.index().findings_for(TRUTH)?,
        TRUTH,
        rule,
        Severity::Warning
    ));
    let command = settings(
        &workspace,
        "[validate.severity]\n\"lore/three-clue\" = \"error\"\n",
    )?;
    let outcome = workspace.run(&command)?;
    assert_eq!(outcome.changed, [SETTINGS]);
    assert!(holds_as(
        &workspace.index().findings_for(TRUTH)?,
        TRUTH,
        rule,
        Severity::Error
    ));
    assert_eq!(workspace.undo_entries().len(), 1);
    Ok(())
}
