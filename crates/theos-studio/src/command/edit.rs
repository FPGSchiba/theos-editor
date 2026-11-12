//! The command that applies span edits to a record.

use theos_content::{Record, TextEdit, decode};

use super::{Base, Command, CommandError, Plan, PlanContext, Reads, project_path};

/// Applies `edits` to the record `path`. The spans are offsets into the record's text with LF
/// line ends; the result is written with LF. Edits that leave the text as it is plan nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Project-relative path.
    pub path: String,
    /// What the caller saw of the file.
    pub base: Base,
    /// The edits.
    pub edits: Vec<TextEdit>,
}

impl Command for Edit {
    fn label(&self) -> String {
        format!("edit {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let path = project_path(&self.path)?;
        let before = ctx.read_based(&path, &self.base)?;
        let record = parse(&path, &before)?;
        let edited = record
            .with_edits(&self.edits)
            .map_err(|source| CommandError::Edit {
                path: path.clone(),
                source,
            })?;
        Ok(Plan::text(path, before, edited.text()))
    }
}

/// The record in `bytes`.
pub(super) fn parse(path: &str, bytes: &[u8]) -> Result<Record, CommandError> {
    let text = decode(bytes).map_err(|source| CommandError::Read {
        path: path.to_owned(),
        source,
    })?;
    Record::parse(text).map_err(|source| CommandError::Record {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use theos_content::Span;

    use super::*;
    use crate::test_support::{Fixture, TRUTH, TRUTH_PATH, TestResult};

    #[test]
    fn edit_applies_span_edits() -> TestResult {
        let crlf = TRUTH.replace('\n', "\r\n");
        let fixture = Fixture::new(&[(TRUTH_PATH, crlf.as_bytes())])?;
        let start = TRUTH.find("Old").ok_or("no Old")?;
        let command = Edit {
            path: TRUTH_PATH.to_owned(),
            base: Base::of(crlf.as_bytes()),
            edits: vec![TextEdit {
                span: Span {
                    start,
                    end: start + 3,
                },
                replacement: "New".to_owned(),
            }],
        };
        let plan = command.plan(&fixture.ctx())?;
        assert_eq!(
            plan.changes[0].after.as_deref(),
            Some(TRUTH.replace("Old", "New").as_bytes())
        );
        assert_eq!(plan.changes[0].before.as_deref(), Some(crlf.as_bytes()));
        Ok(())
    }

    #[test]
    fn an_edit_that_changes_nothing_is_an_empty_plan() -> TestResult {
        for text in [TRUTH.to_owned(), TRUTH.replace('\n', "\r\n")] {
            let fixture = Fixture::new(&[(TRUTH_PATH, text.as_bytes())])?;
            let start = TRUTH.find("Old").ok_or("no Old")?;
            for edits in [
                Vec::new(),
                vec![TextEdit {
                    span: Span {
                        start,
                        end: start + 3,
                    },
                    replacement: "Old".to_owned(),
                }],
            ] {
                let command = Edit {
                    path: TRUTH_PATH.to_owned(),
                    base: Base::of(text.as_bytes()),
                    edits,
                };
                assert_eq!(command.plan(&fixture.ctx())?, Plan::default(), "{text:?}");
            }
        }
        Ok(())
    }

    #[test]
    fn edit_of_a_file_that_is_not_a_record_is_an_error() -> TestResult {
        let fixture = Fixture::new(&[("a.md", b"no front matter\n"), ("b.md", &[0xff][..])])?;
        for (path, bytes) in [("a.md", &b"no front matter\n"[..]), ("b.md", &[0xff][..])] {
            let command = Edit {
                path: path.to_owned(),
                base: Base::of(bytes),
                edits: Vec::new(),
            };
            let error = command.plan(&fixture.ctx()).err().ok_or("planned")?;
            assert!(
                matches!(
                    error,
                    CommandError::Record { .. } | CommandError::Read { .. }
                ),
                "{error}"
            );
        }
        Ok(())
    }

    #[test]
    fn edit_names_a_failing_edit() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        let command = Edit {
            path: TRUTH_PATH.to_owned(),
            base: Base::of(TRUTH.as_bytes()),
            edits: vec![TextEdit {
                span: Span {
                    start: 9_000,
                    end: 9_001,
                },
                replacement: String::new(),
            }],
        };
        assert!(matches!(
            command.plan(&fixture.ctx()),
            Err(CommandError::Edit { .. })
        ));
        Ok(())
    }
}
