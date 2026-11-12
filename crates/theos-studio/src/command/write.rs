//! The command that replaces a file's text.

use theos_content::text::normalize;

use super::{Base, Command, CommandError, FileChange, Plan, PlanContext, Reads, project_path};

/// Replaces the text of the file `path` with `text`, written with LF line ends. `base` is the
/// base of the bytes the caller saw. A text equal to the file's own text with LF line ends
/// plans nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    /// Project-relative path.
    pub path: String,
    /// What the caller saw of the file.
    pub base: Base,
    /// The new text.
    pub text: String,
}

impl Command for Write {
    fn label(&self) -> String {
        format!("write {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let path = project_path(&self.path)?;
        let before = ctx.read_based(&path, &self.base)?;
        let after = normalize(&self.text).0.into_owned();
        let unchanged = std::str::from_utf8(&before)
            .is_ok_and(|current| normalize(current).0.as_ref() == after);
        if unchanged {
            return Ok(Plan::default());
        }
        Ok(Plan {
            changes: vec![FileChange {
                path,
                before: Some(before),
                after: Some(after.into_bytes()),
            }],
            ..Plan::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::test_support::{Fixture, TRUTH, TestResult};

    const PATH: &str = "lore/truths/T-960.md";

    fn write(base: Base, text: &str) -> Write {
        Write {
            path: PATH.to_owned(),
            base,
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_base_that_differs_is_stale() -> TestResult {
        let fixture = Fixture::new(&[(PATH, TRUTH.as_bytes())])?;
        let result = write(Base::of(b"other"), TRUTH).plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::Stale { path }) if path == PATH));
        Ok(())
    }

    #[test]
    fn a_missing_file_is_not_found() -> TestResult {
        let fixture = Fixture::new(&[])?;
        let result = write(Base::of(b""), "x").plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::NotFound { .. })));
        Ok(())
    }

    #[test]
    fn write_turns_line_ends_into_lf() -> TestResult {
        let fixture = Fixture::new(&[(PATH, TRUTH.as_bytes())])?;
        let plan = write(Base::of(TRUTH.as_bytes()), "a\r\nb\r\n").plan(&fixture.ctx())?;
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].before.as_deref(), Some(TRUTH.as_bytes()));
        assert_eq!(plan.changes[0].after.as_deref(), Some(&b"a\nb\n"[..]));
        assert!(!plan.is_empty());
        Ok(())
    }

    #[test]
    fn an_unchanged_save_of_a_crlf_record_is_an_empty_plan() -> TestResult {
        let crlf = TRUTH.replace('\n', "\r\n");
        let fixture = Fixture::new(&[(PATH, crlf.as_bytes())])?;
        let plan = write(Base::of(crlf.as_bytes()), TRUTH).plan(&fixture.ctx())?;
        assert!(plan.is_empty());
        let plan = write(Base::of(crlf.as_bytes()), &crlf).plan(&fixture.ctx())?;
        assert!(plan.is_empty());
        Ok(())
    }

    #[test]
    fn write_takes_a_file_that_is_not_a_record() -> TestResult {
        let fixture = Fixture::new(&[("notes.txt", &[0xff, 0xfe][..])])?;
        let command = Write {
            path: "notes.txt".to_owned(),
            base: Base::of(&[0xff, 0xfe]),
            text: "text".to_owned(),
        };
        assert!(!command.plan(&fixture.ctx())?.is_empty());
        Ok(())
    }
}
