//! `create`: a new record from a template or a skeleton, judged before it is written.

use theos_studio::create::{CreateError, Created, NewRequest, create};
use theos_studio::{StudioError, Workspace};

use crate::common::{Error, TempProject, TestResult, put};
use crate::repo::variant;

const SYNTHETIC: &str = "# synthetic fixture (P05), not canon\n";

/// A copy of the `staged` fixture project, opened, with `files` added.
fn project(files: &[(&str, String)]) -> Result<(TempProject, Workspace), Error> {
    let temp = TempProject::copy_of_studio("projects/staged")?;
    for (path, text) in files {
        put(temp.root(), path, text.as_bytes())?;
    }
    let workspace = Workspace::open(temp.root())?;
    Ok((temp, workspace))
}

fn request<'a>(kind: &'a str, id: Option<&'a str>, set: &'a [(String, String)]) -> NewRequest<'a> {
    NewRequest {
        kind,
        id,
        title: None,
        set,
    }
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// A people record and a faction under it, so an npc has a parent.
fn people() -> Vec<(&'static str, String)> {
    vec![
        (
            "people/synthetic-people.md",
            format!(
                "---\n{SYNTHETIC}id: synthetic-people\nkind: people\ntitle: Synthetic people\n---\n"
            ),
        ),
        (
            "people/synthetic-people/synthetic-faction.md",
            format!(
                "---\n{SYNTHETIC}id: synthetic-faction\nkind: faction\ntitle: Synthetic faction\n\
                 parent: synthetic-people\n---\n"
            ),
        ),
    ]
}

fn created(result: Result<Created, StudioError>) -> Result<Created, Error> {
    Ok(result?)
}

fn create_error(result: Result<Created, StudioError>) -> Result<CreateError, Error> {
    match result {
        Err(StudioError::Create(error)) => Ok(error),
        other => Err(format!("expected a create error, got {other:?}").into()),
    }
}

fn clue_fields(reveals: &str) -> Vec<(String, String)> {
    pairs(&[
        ("reveals", reveals),
        ("moves_to", "hinted"),
        ("where", "[synthetic-hall]"),
        ("gate", "T1"),
        ("vector", "synthetic"),
    ])
}

#[test]
fn a_truth_is_numbered_from_the_index_and_written() -> TestResult {
    let (temp, mut workspace) = project(&[])?;
    let set = pairs(&[("target", "hinted")]);
    let result = created(create(
        &mut workspace,
        &NewRequest {
            title: Some("Synthetic statement B"),
            ..request("truth", None, &set)
        },
    ))?;
    assert_eq!(result.id, "T-902");
    assert_eq!(result.path, "lore/truths/T-902.md");
    assert!(result.written());
    let text = std::fs::read_to_string(temp.root().join("lore/truths/T-902.md"))?;
    assert_eq!(
        text,
        "---\nid: T-902\nkind: truth\ntitle: Synthetic statement B\nstatus: draft\ntarget: hinted\n---\n"
    );
    assert_eq!(workspace.index().record("T-902")?.len(), 1);
    Ok(())
}

#[test]
fn a_template_is_used_with_its_title_filled() -> TestResult {
    let template = format!(
        "---\n{SYNTHETIC}id: {{{{title}}}}\nkind: truth\ntitle: {{{{title}}}}\nstatus: draft\n\
         target: hinted\n---\nKeep {{{{speaker}}}}.\n"
    );
    let (temp, mut workspace) = project(&[("templates/records/truth.template.md", template)])?;
    let result = created(create(
        &mut workspace,
        &request("truth", Some("T-950"), &[]),
    ))?;
    assert!(result.written());
    let text = std::fs::read_to_string(temp.root().join("lore/truths/T-950.md"))?;
    assert!(text.contains("id: T-950\n"), "{text}");
    assert!(text.contains("title: T-950\n"), "{text}");
    assert!(text.contains("Keep {{speaker}}."), "{text}");
    assert!(!text.contains('\r'));
    Ok(())
}

#[test]
fn a_standing_error_elsewhere_does_not_refuse() -> TestResult {
    // T-900 is lowered, so the standing clue C-900 overshoots it. A new clue that reveals T-900
    // revalidates C-900, so the standing error is in the findings.
    let (temp, mut workspace) = project(&[("lore/truths/T-900.md", variant("T-900-lowered.md")?)])?;
    let mut set = clue_fields("T-900");
    set[1].1 = "suspected".to_owned();
    let result = created(create(&mut workspace, &request("clue", None, &set)))?;
    assert!(result.written(), "{:?}", result.introduced);
    assert!(temp.root().join("lore/clues/C-901.md").is_file());
    assert!(
        result.findings.iter().any(
            |finding| finding.rule == "lore/no-overshoot" && finding.path.ends_with("C-900.md")
        ),
        "{:?}",
        result.findings
    );
    assert_eq!(result.introduced.len(), 0);
    Ok(())
}

#[test]
fn a_clue_revealing_a_never_truth_is_refused() -> TestResult {
    let (temp, mut workspace) = project(&[])?;
    let set = clue_fields("T-901");
    let result = created(create(&mut workspace, &request("clue", None, &set)))?;
    assert_eq!(result.path, "lore/clues/C-901.md");
    assert!(!result.written());
    assert!(
        result
            .introduced
            .iter()
            .any(|finding| finding.rule == "lore/never-confirm"),
        "{:?}",
        result.introduced
    );
    assert!(!temp.root().join("lore/clues/C-901.md").exists());
    assert_eq!(workspace.index().record("C-901")?.len(), 0);
    Ok(())
}

#[test]
fn an_existing_id_is_never_overwritten() -> TestResult {
    let (temp, mut workspace) = project(&[])?;
    let before = std::fs::read(temp.root().join("lore/truths/T-900.md"))?;
    // The same target as the existing T-900, so the preview finds nothing to refuse.
    let set = pairs(&[("target", "confirmed")]);
    let error = create_error(create(
        &mut workspace,
        &request("truth", Some("T-900"), &set),
    ))?;
    assert!(
        matches!(&error, CreateError::Exists { path } if path == "lore/truths/T-900.md"),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(temp.root().join("lore/truths/T-900.md"))?,
        before
    );
    Ok(())
}

#[test]
fn an_id_in_another_case_is_refused() -> TestResult {
    let error = {
        let (_temp, mut workspace) = project(&[])?;
        create_error(create(
            &mut workspace,
            &request("truth", Some("t-900"), &[]),
        ))?
    };
    assert!(
        matches!(error, CreateError::IdDoesNotFit { .. }),
        "{error:?}"
    );

    let existing = format!("---\n{SYNTHETIC}id: Synthetic-Case\nkind: people\ntitle: Case\n---\n");
    let (temp, mut workspace) = project(&[("people/Synthetic-Case.md", existing)])?;
    match create(
        &mut workspace,
        &request("people", Some("synthetic-case"), &[]),
    ) {
        Err(StudioError::Create(CreateError::Exists { .. })) => {}
        Ok(result) => {
            assert!(!result.written());
            assert!(
                result
                    .introduced
                    .iter()
                    .any(|finding| finding.rule == "index/duplicate-id"),
                "{:?}",
                result.introduced
            );
        }
        Err(other) => return Err(other.into()),
    }
    let same: Vec<String> = std::fs::read_dir(temp.root().join("people"))?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_lowercase())
        .filter(|name| name == "synthetic-case.md")
        .collect();
    assert_eq!(same.len(), 1, "{same:?}");
    Ok(())
}

#[test]
fn a_missing_parent_is_parent_missing() -> TestResult {
    let (_temp, mut workspace) = project(&[])?;
    let set = pairs(&[("parent", "nobody")]);
    let error = create_error(create(
        &mut workspace,
        &request("npc", Some("mother-ilse"), &set),
    ))?;
    assert!(
        matches!(&error, CreateError::ParentMissing { id } if id == "nobody"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_npc_without_a_parent_value_has_no_parent() -> TestResult {
    let (_temp, mut workspace) = project(&[])?;
    let error = create_error(create(
        &mut workspace,
        &request("npc", Some("mother-ilse"), &[]),
    ))?;
    assert!(
        matches!(&error, CreateError::NoParent { field } if field == "parent"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_npc_is_placed_under_its_parent() -> TestResult {
    let (temp, mut workspace) = project(&people())?;
    let set = pairs(&[("parent", "synthetic-faction")]);
    let result = created(create(
        &mut workspace,
        &request("npc", Some("mother-ilse"), &set),
    ))?;
    assert_eq!(
        result.path,
        "people/synthetic-people/synthetic-faction/mother-ilse.md"
    );
    assert!(result.written(), "{:?}", result.introduced);
    assert!(temp.root().join(&result.path).is_file());
    Ok(())
}

#[test]
fn a_value_the_restricted_yaml_rejects_is_a_value_error() -> TestResult {
    let (temp, mut workspace) = project(&[])?;
    let set = pairs(&[("title", "a: b")]);
    let error = create_error(create(&mut workspace, &request("truth", None, &set)))?;
    assert!(
        matches!(&error, CreateError::Value { field, .. } if field == "title"),
        "{error:?}"
    );
    assert!(!temp.root().join("lore/truths/T-902.md").exists());
    let set = pairs(&[("title", "one\ntwo")]);
    let error = create_error(create(&mut workspace, &request("truth", None, &set)))?;
    assert!(matches!(error, CreateError::Value { .. }), "{error:?}");
    Ok(())
}

#[test]
fn unknown_kinds_fields_and_ids_are_refused() -> TestResult {
    let (_temp, mut workspace) = project(&[])?;
    let error = create(&mut workspace, &request("nonsense", None, &[]));
    assert!(
        matches!(error, Err(StudioError::UnknownKind { .. })),
        "{error:?}"
    );
    let set = pairs(&[("colour", "red")]);
    let error = create_error(create(&mut workspace, &request("truth", None, &set)))?;
    assert!(
        matches!(&error, CreateError::UnknownField { kind, field } if kind == "truth" && field == "colour"),
        "{error:?}"
    );
    let error = create_error(create(&mut workspace, &request("truth", Some("x-1"), &[])))?;
    assert!(
        matches!(error, CreateError::IdDoesNotFit { .. }),
        "{error:?}"
    );
    let error = create_error(create(&mut workspace, &request("people", None, &[])))?;
    assert!(
        matches!(&error, CreateError::NeedsId { kind } if kind == "people"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_template_that_is_not_a_record_is_a_template_error() -> TestResult {
    let (temp, mut workspace) = project(&[(
        "templates/records/truth.template.md",
        "no front matter\n".to_owned(),
    )])?;
    let error = create_error(create(&mut workspace, &request("truth", None, &[])))?;
    assert!(
        matches!(&error, CreateError::Template { path, .. } if path == "templates/records/truth.template.md"),
        "{error:?}"
    );
    assert!(!temp.root().join("lore/truths/T-902.md").exists());
    Ok(())
}
