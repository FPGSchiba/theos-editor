//! `theos new`: a record of a kind from its template or a skeleton, judged against the canon rules
//! and written only when it introduces no error (spec §5.9).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};

use theos_content::yaml::{RawValue, Value};
use theos_content::{Change, Finding, KindSpec, PatchOp, Placement, Record, RecordError, Update};

use crate::error::StudioError;
use crate::introduced::{FindingCounts, introduced};
use crate::report::report;
use crate::workspace::Workspace;

/// What `theos new` was asked for.
#[derive(Clone, Copy, Debug)]
pub struct NewRequest<'a> {
    /// The kind of the new record.
    pub kind: &'a str,
    /// The id; for a numbered kind, the next number when it is `None`.
    pub id: Option<&'a str>,
    /// The title, replacing the template's.
    pub title: Option<&'a str>,
    /// Fields to set, as `(field, value)`; a value is read as front matter reads it.
    pub set: &'a [(String, String)],
}

/// The result of [`create`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    /// The project-relative path of the record, with `/`.
    pub path: String,
    /// The id of the record.
    pub id: String,
    /// The findings the new record leaves on everything it touches.
    pub findings: Vec<Finding>,
    /// The errors the new record introduces; the file is not written when there are any.
    pub introduced: Vec<Finding>,
}

impl Created {
    /// Whether the file was written: the record introduces no error.
    #[must_use]
    pub fn written(&self) -> bool {
        self.introduced.is_empty()
    }
}

/// Why no record was made.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    /// The id does not match the id rule of the kind.
    #[error("`{id}` is not an id of a {kind}: it must be {says}")]
    IdDoesNotFit {
        /// The kind.
        kind: String,
        /// The id.
        id: String,
        /// The rule in plain words.
        says: String,
    },
    /// No id was given and the kind is not numbered.
    #[error("a {kind} has no number to count on; give an id")]
    NeedsId {
        /// The kind.
        kind: String,
    },
    /// The kind has no such field.
    #[error("a {kind} has no field `{field}`")]
    UnknownField {
        /// The kind.
        kind: String,
        /// The field.
        field: String,
    },
    /// The field is not the request's to set.
    #[error("`{field}` cannot be set: the id is the argument of the command and the kind is fixed")]
    ReservedField {
        /// The field.
        field: String,
    },
    /// A value cannot be written as the field.
    #[error("{field}: {message}")]
    Value {
        /// The field.
        field: String,
        /// What is wrong.
        message: String,
    },
    /// The template of the kind cannot be used.
    #[error("{path}: {message}")]
    Template {
        /// The template, project-relative.
        path: String,
        /// What is wrong.
        message: String,
    },
    /// The kind goes under a parent and the record does not name one.
    #[error("give the parent with --set {field}=<id>")]
    NoParent {
        /// The field that names the parent.
        field: String,
    },
    /// The parent is not in the index.
    #[error("there is no record `{id}` to place it under")]
    ParentMissing {
        /// The parent's id.
        id: String,
    },
    /// The file exists; a record is never replaced.
    #[error("`{path}` exists already; `theos new` never replaces a file")]
    Exists {
        /// The project-relative path.
        path: String,
    },
}

/// Makes a record of `request.kind`, judges it with `Index::preview`, and writes it unless it
/// introduces an error. Nothing is written when [`Created::written`] is false.
///
/// # Errors
///
/// [`StudioError::UnknownKind`] for an unknown kind; a [`CreateError`] when the request or the
/// template is wrong, the parent is missing or the file exists; otherwise the index or the file
/// system failed.
pub fn create(workspace: &mut Workspace, request: &NewRequest<'_>) -> Result<Created, StudioError> {
    let spec = workspace
        .registry()
        .kind(request.kind)
        .ok_or_else(|| StudioError::unknown_kind(workspace.registry(), request.kind))?
        .clone();
    let id = choose_id(workspace, &spec, request.id)?;
    let record = fill(workspace, &spec, &id, request)?;
    let path = place(workspace, &spec, &id, &record)?;
    let text = record.text().to_owned();
    if workspace.project().root().join(&path).exists() {
        return Err(CreateError::Exists { path }.into());
    }

    let preview = workspace.write(|index| index.preview(&[Change::text(&path, &text)]))?;
    let before = standing(workspace, &path, &preview.update)?;
    let introduced = introduced(
        &FindingCounts::of(&before, &BTreeMap::new()),
        &preview.findings,
    );
    let created = Created {
        path,
        id,
        findings: preview.findings,
        introduced,
    };
    if created.written() {
        write_new(workspace, &created.path, &text)?;
    }
    Ok(created)
}

/// The given id when it fits, else the next number of a numbered kind.
fn choose_id(
    workspace: &Workspace,
    spec: &KindSpec,
    given: Option<&str>,
) -> Result<String, StudioError> {
    if let Some(id) = given {
        if !workspace.registry().id_fits(spec.name, id) {
            return Err(CreateError::IdDoesNotFit {
                kind: spec.name.to_owned(),
                id: id.to_owned(),
                says: spec.id.says.to_owned(),
            }
            .into());
        }
        return Ok(id.to_owned());
    }
    let prefix = spec
        .id
        .number_prefix()
        .ok_or_else(|| CreateError::NeedsId {
            kind: spec.name.to_owned(),
        })?;
    let records = workspace.index().records()?;
    Ok(next_id(records.iter().map(|r| r.id.as_str()), prefix))
}

/// One above the highest number among `ids` that are `prefix` and digits, padded to the widest of
/// them and at least three digits; `<prefix>001` when there is none.
fn next_id<'a>(ids: impl Iterator<Item = &'a str>, prefix: &str) -> String {
    let mut highest = 0_u64;
    let mut width = 3;
    for number in ids.filter_map(|id| id.strip_prefix(prefix)) {
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(value) = number.parse::<u64>() {
            highest = highest.max(value);
            width = width.max(number.len());
        }
    }
    format!("{prefix}{:0width$}", highest + 1)
}

/// The template text with each `{{title}}` replaced by `id`; other placeholders stay.
fn fill_title(template: &str, id: &str) -> String {
    template.replace("{{title}}", id)
}

/// The text of a record without a template: `id`, `kind`, `title` (the id) and `status: draft`
/// where the kind has them, and an empty body.
fn skeleton(spec: &KindSpec, id: &str) -> String {
    let mut lines = vec![format!("id: {id}"), format!("kind: {}", spec.name)];
    if spec.field("title").is_some() {
        lines.push(format!("title: {id}"));
    }
    if spec.field("status").is_some() {
        lines.push("status: draft".to_owned());
    }
    format!("---\n{}\n---\n", lines.join("\n"))
}

/// The record of the template or skeleton with the title and the fields of the request.
fn fill(
    workspace: &Workspace,
    spec: &KindSpec,
    id: &str,
    request: &NewRequest<'_>,
) -> Result<Record, StudioError> {
    let mut record = base(workspace, spec, id)?;
    let order = spec.order();
    let mut ops = Vec::new();
    if let Some(title) = request.title {
        ops.push((
            "title",
            PatchOp::Set {
                key: "title".to_owned(),
                value: RawValue::Text(title.to_owned()),
            },
        ));
    }
    for (key, value) in request.set {
        if key == "id" || key == "kind" {
            return Err(CreateError::ReservedField { field: key.clone() }.into());
        }
        if spec.field(key).is_none() {
            return Err(CreateError::UnknownField {
                kind: spec.name.to_owned(),
                field: key.clone(),
            }
            .into());
        }
        ops.push((
            key.as_str(),
            PatchOp::Set {
                key: key.clone(),
                value: read_value(key, value)?,
            },
        ));
    }
    for (field, op) in ops {
        record = record
            .patch(&[op], &order)
            .map_err(|error| CreateError::Value {
                field: field.to_owned(),
                message: report(&error),
            })?;
    }
    Ok(record)
}

/// The record of the kind's template when the project has one, else the skeleton.
fn base(workspace: &Workspace, spec: &KindSpec, id: &str) -> Result<Record, StudioError> {
    let relative = format!("templates/records/{}.template.md", spec.name);
    let template = |message: String| CreateError::Template {
        path: relative.clone(),
        message,
    };
    let text = match std::fs::read_to_string(workspace.project().root().join(&relative)) {
        Ok(template_text) => fill_title(&template_text, id),
        Err(error) if error.kind() == ErrorKind::NotFound => skeleton(spec, id),
        Err(error) => return Err(template(error.to_string()).into()),
    };
    let record = Record::parse(&text).map_err(|error: RecordError| template(error.to_string()))?;
    if let Some(error) = record.front_matter_error() {
        return Err(template(error.to_string()).into());
    }
    Ok(record)
}

/// The value of `--set field=value`, read as the front matter reads `field: value`.
fn read_value(field: &str, value: &str) -> Result<RawValue, CreateError> {
    let bad = |message: String| CreateError::Value {
        field: field.to_owned(),
        message,
    };
    if value.contains(['\n', '\r']) {
        return Err(bad("a value cannot hold a line break".to_owned()));
    }
    let record = Record::parse(&format!("---\n{field}: {value}\n---\n"))
        .map_err(|error| bad(error.to_string()))?;
    let front = record
        .front_matter()
        .map_err(|error| bad(error.to_string()))?;
    match front.fields.as_slice() {
        [only] if only.key == field => Ok(only.value.to_raw()),
        _ => Err(bad("not one value".to_owned())),
    }
}

/// The project-relative path of the record: in its kind's folder, or under its parent.
fn place(
    workspace: &Workspace,
    spec: &KindSpec,
    id: &str,
    record: &Record,
) -> Result<String, StudioError> {
    match spec.placement {
        Placement::Flat("") => Ok(format!("{id}.md")),
        Placement::Flat(folder) => Ok(format!("{folder}/{id}.md")),
        Placement::UnderParent(field) => {
            let parent = record
                .front_matter()
                .ok()
                .and_then(|front| front.get(field))
                .and_then(|found| match &found.value {
                    Value::Scalar(scalar) if !scalar.text.is_empty() => Some(scalar.text.clone()),
                    _ => None,
                })
                .ok_or_else(|| CreateError::NoParent {
                    field: field.to_owned(),
                })?;
            let parents = workspace.index().record(&parent)?;
            let first = parents
                .first()
                .ok_or(CreateError::ParentMissing { id: parent.clone() })?;
            Ok(format!(
                "{}/{id}.md",
                Placement::under_parent(&first.path, &parent)
            ))
        }
    }
}

/// The findings the index holds now for every path the preview touched and for the new path.
fn standing(
    workspace: &Workspace,
    path: &str,
    update: &Update,
) -> Result<Vec<Finding>, StudioError> {
    let paths: BTreeSet<&str> = [path]
        .into_iter()
        .chain(update.reparsed.iter().map(String::as_str))
        .chain(update.relinked.iter().map(String::as_str))
        .chain(update.revalidated.iter().map(String::as_str))
        .chain(update.removed.iter().map(String::as_str))
        .collect();
    let mut before = Vec::new();
    for each in paths {
        before.extend(workspace.index().findings_for(each)?);
    }
    Ok(before)
}

/// Creates the file, which must not exist, and tells the index.
fn write_new(workspace: &mut Workspace, path: &str, text: &str) -> Result<(), StudioError> {
    let full = workspace.project().root().join(path);
    let io = |source| StudioError::Io {
        path: full.clone(),
        source,
    };
    if let Some(folder) = full.parent() {
        std::fs::create_dir_all(folder).map_err(io)?;
    }
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&full) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(CreateError::Exists {
                path: path.to_owned(),
            }
            .into());
        }
        Err(error) => return Err(io(error)),
    };
    file.write_all(text.as_bytes()).map_err(io)?;
    workspace.write(|index| index.update(&[path]))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    #[test]
    fn next_id_is_one_above_the_highest() {
        let next = |ids: &[&str], prefix: &str| next_id(ids.iter().copied(), prefix);
        assert_eq!(next(&["T-001", "T-012", "T-003"], "T-"), "T-013");
        assert_eq!(next(&["T-999"], "T-"), "T-1000");
        assert_eq!(next(&["T-0099"], "T-"), "T-0100");
        assert_eq!(next(&[], "T-"), "T-001");
        assert_eq!(next(&["C-040", "x"], "T-"), "T-001");
        assert_eq!(next(&["T-01a", "T-", "T-002"], "T-"), "T-003");
    }

    #[test]
    fn fill_title_replaces_only_title() {
        assert_eq!(
            fill_title("id: {{title}}\n{{speaker}}: {{title}}", "T-001"),
            "id: T-001\n{{speaker}}: T-001"
        );
    }

    #[test]
    fn the_truth_skeleton() -> TestResult {
        let registry = theos_content::parts::builtin()?;
        let spec = registry.kind("truth").ok_or("no truth kind")?;
        assert_eq!(
            skeleton(spec, "T-001"),
            "---\nid: T-001\nkind: truth\ntitle: T-001\nstatus: draft\n---\n"
        );
        Ok(())
    }

    #[test]
    fn a_value_is_read_as_front_matter_reads_it() {
        assert_eq!(
            read_value("tags", "[a, b]").ok(),
            Some(RawValue::List(vec!["a".to_owned(), "b".to_owned()]))
        );
        assert_eq!(
            read_value("title", "plain text").ok(),
            Some(RawValue::Text("plain text".to_owned()))
        );
        assert!(read_value("title", "a: b").is_err());
        assert!(read_value("title", "a\nb: c").is_err());
    }
}
