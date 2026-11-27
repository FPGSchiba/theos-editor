//! The marks every result of the read tools carries (CE-18.3): whether a record is a truth with
//! target `never`, and why a record is locked.

use std::collections::{BTreeSet, HashMap};

use schemars::JsonSchema;
use serde::Serialize;
use theos_content::Index;

use crate::StudioError;

/// Why a record is locked, in the order a mark lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LockReason {
    /// Its `status` is `canon`.
    Canon,
    /// It is a truth with target `never`.
    Never,
    /// It is in the generated formalism mirror.
    Formalism,
    /// It is a style whose `state` is `locked`.
    Style,
}

/// The marks of one record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
pub struct MarkView {
    /// Whether the record is a truth with target `never`.
    pub never: bool,
    /// Why the record is locked; empty when it is not.
    pub locked: Vec<LockReason>,
}

/// The marks of every record, built once from the index.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    never: BTreeSet<String>,
    canon: BTreeSet<String>,
    formalism: BTreeSet<String>,
    style: BTreeSet<String>,
    paths_of_id: HashMap<String, Vec<String>>,
}

/// The path prefix of the generated formalism mirror: the folder of `theos_content::formalism`
/// and a `/`, so a sibling folder such as `canon/formalism-notes/` does not match.
fn formalism_prefix() -> String {
    format!("{}/", theos_content::formalism::FOLDER)
}

impl Marks {
    /// Reads the marks from `index`.
    ///
    /// # Errors
    /// [`StudioError::Index`] when the index fails.
    pub fn load(index: &Index) -> Result<Marks, StudioError> {
        let mut marks = Marks::default();
        let formalism = formalism_prefix();
        let mut kinds: HashMap<String, Option<String>> = HashMap::new();
        for record in index.records()? {
            if record.path.starts_with(&formalism) {
                marks.formalism.insert(record.path.clone());
            }
            marks
                .paths_of_id
                .entry(record.id)
                .or_default()
                .push(record.path.clone());
            kinds.insert(record.path, record.kind);
        }
        let kind_is = |path: &str, kind: &str| {
            kinds
                .get(path)
                .is_some_and(|found| found.as_deref() == Some(kind))
        };
        for row in index.field_texts("target")? {
            if row.text == "never" && kind_is(&row.path, "truth") {
                marks.never.insert(row.path);
            }
        }
        for row in index.field_texts("status")? {
            if row.text == "canon" && kinds.contains_key(&row.path) {
                marks.canon.insert(row.path);
            }
        }
        for row in index.field_texts("state")? {
            if row.text == "locked" && kind_is(&row.path, "style") {
                marks.style.insert(row.path);
            }
        }
        Ok(marks)
    }

    /// The marks of the record at `path`.
    #[must_use]
    pub fn of_path(&self, path: &str) -> MarkView {
        let never = self.never.contains(path);
        let reasons = [
            (self.canon.contains(path), LockReason::Canon),
            (never, LockReason::Never),
            (self.formalism.contains(path), LockReason::Formalism),
            (self.style.contains(path), LockReason::Style),
        ];
        MarkView {
            never,
            locked: reasons
                .into_iter()
                .filter_map(|(held, reason)| held.then_some(reason))
                .collect(),
        }
    }

    /// The marks of the records with `id`: the union over all of them, so a duplicate loses no
    /// mark. An id no record has gives the default.
    #[must_use]
    pub fn of_id(&self, id: &str) -> MarkView {
        let mut never = false;
        let mut locked = BTreeSet::new();
        for path in self.paths_of_id.get(id).into_iter().flatten() {
            let view = self.of_path(path);
            never |= view.never;
            locked.extend(view.locked);
        }
        MarkView {
            never,
            locked: locked.into_iter().collect(),
        }
    }

    /// Whether the record at `path` is a truth with target `never`.
    #[must_use]
    pub fn is_never(&self, path: &str) -> bool {
        self.never.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestResult, codex_workspace};
    use theos_content::kinds::IdRule;
    use theos_content::{BodyGrammar, FieldSpec, FieldType, KindSpec, Placement, Project, parts};

    fn mark(never: bool, locked: &[LockReason]) -> MarkView {
        MarkView {
            never,
            locked: locked.to_vec(),
        }
    }

    /// A project in a temporary folder: `theos.toml` and the given files.
    fn project(
        files: &[(&str, &str)],
    ) -> Result<(tempfile::TempDir, Project), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("theos.toml"),
            "[project]\nname = \"synthetic\"\nkind = \"content\"\nschema_version = 1\n",
        )?;
        for (path, text) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full, text)?;
        }
        let project = Project::open(dir.path())?;
        Ok((dir, project))
    }

    // AC: CE-18.3
    #[test]
    fn never_truth_is_never_and_locked() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let marks = Marks::load(workspace.index())?;
        assert_eq!(
            marks.of_path("lore/truths/T-911.md"),
            mark(true, &[LockReason::Never])
        );
        assert!(marks.is_never("lore/truths/T-911.md"));
        assert!(!marks.is_never("lore/truths/T-912.md"));
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn canon_style_and_mirror_are_locked() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let marks = Marks::load(workspace.index())?;
        assert_eq!(
            marks.of_path("people/hearth-folk/fates-cult.md"),
            mark(false, &[LockReason::Canon])
        );
        assert_eq!(
            marks.of_path("canon/formalism/E91.md"),
            mark(false, &[LockReason::Formalism])
        );

        // No style kind is registered before P26, so the style lock has its own project.
        let mut registry = parts::builtin()?;
        registry.register(
            "test",
            KindSpec {
                name: "style",
                id: IdRule {
                    pattern: "^[a-z0-9]+(-[a-z0-9]+)*$",
                    says: "a slug",
                },
                placement: Placement::Flat("style"),
                fields: vec![
                    FieldSpec {
                        key: "id",
                        ty: FieldType::Id,
                        required: true,
                    },
                    FieldSpec {
                        key: "kind",
                        ty: FieldType::KindName,
                        required: true,
                    },
                    FieldSpec {
                        key: "state",
                        ty: FieldType::Enum(&["draft", "locked"]),
                        required: true,
                    },
                ],
                body: BodyGrammar::PROSE,
                checks: Vec::new(),
            },
        )?;
        let (_dir, project) = project(&[
            (
                "style/voice.md",
                "---\nid: voice\nkind: style\nstate: locked\n---\nText\n",
            ),
            (
                "style/draft.md",
                "---\nid: draft\nkind: style\nstate: draft\n---\nText\n",
            ),
        ])?;
        let mut index = Index::in_memory(&project, &registry)?;
        index.sync()?;
        let marks = Marks::load(&index)?;
        assert_eq!(
            marks.of_path("style/voice.md"),
            mark(false, &[LockReason::Style])
        );
        assert_eq!(marks.of_path("style/draft.md"), MarkView::default());
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn duplicated_id_is_marked_by_any_record() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let marks = Marks::load(workspace.index())?;
        assert_eq!(marks.of_id("dup-911"), mark(false, &[LockReason::Canon]));
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn invalid_values_give_no_mark() -> TestResult {
        let registry = parts::builtin()?;
        let (_dir, project) = project(&[
            (
                "lore/truths/T-901.md",
                "---\nid: T-901\nkind: truth\ntitle: Synthetic\ntarget: Never\nstatus: Canon\n---\nBody\n",
            ),
            (
                "lore/truths/T-902.md",
                "---\nid: T-902\nkind: truth\ntitle: Synthetic\ntarget: never\nstatus: canon\n---\nBody\n",
            ),
        ])?;
        let mut index = Index::in_memory(&project, &registry)?;
        index.sync()?;
        let marks = Marks::load(&index)?;
        assert_eq!(marks.of_path("lore/truths/T-901.md"), MarkView::default());
        // The valid spelling of the same project does mark, so the test is not vacuous.
        assert_eq!(
            marks.of_path("lore/truths/T-902.md"),
            mark(true, &[LockReason::Canon, LockReason::Never])
        );
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn unknown_id_has_no_mark() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let marks = Marks::load(workspace.index())?;
        assert_eq!(marks.of_id("ghost-911"), MarkView::default());
        Ok(())
    }
}
