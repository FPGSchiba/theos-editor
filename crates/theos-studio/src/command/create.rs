//! The command that creates a file.

use theos_content::text::normalize;

use super::{Command, CommandError, FileChange, Plan, PlanContext, Reads, project_path};

/// Creates the file `path` with `text`, which is written with LF line ends. The file must not
/// exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Create {
    /// Project-relative path.
    pub path: String,
    /// The text of the file.
    pub text: String,
}

impl Command for Create {
    fn label(&self) -> String {
        format!("create {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let path = project_path(&self.path)?;
        // The disk decides what exists: a case-insensitive disk also finds another case.
        match std::fs::symlink_metadata(ctx.root().join(&path)) {
            Ok(_) => return Err(CommandError::Exists { path }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(CommandError::read_io(path, source)),
        }
        let after = normalize(&self.text).0.into_owned().into_bytes();
        Ok(Plan {
            changes: vec![FileChange {
                path,
                before: None,
                after: Some(after),
            }],
            ..Plan::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::test_support::{Fixture, TRUTH, TestResult};

    fn create(path: &str, text: &str) -> Create {
        Create {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn create_plans_a_new_file_with_lf() -> TestResult {
        let fixture = Fixture::new(&[])?;
        let command = create("lore/truths/T-961.md", "a\r\nb\r\n");
        let plan = command.plan(&fixture.ctx())?;
        assert_eq!(
            plan.changes,
            vec![FileChange {
                path: "lore/truths/T-961.md".to_owned(),
                before: None,
                after: Some(b"a\nb\n".to_vec()),
            }]
        );
        assert_eq!(
            command.reads(),
            Reads::Paths(vec!["lore/truths/T-961.md".to_owned()])
        );
        Ok(())
    }

    #[test]
    fn create_refuses_an_existing_file() -> TestResult {
        let fixture = Fixture::new(&[("lore/truths/T-960.md", TRUTH.as_bytes())])?;
        let result = create("lore/truths/T-960.md", "x").plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::Exists { .. })));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn create_refuses_another_case_of_the_name_on_windows() -> TestResult {
        let fixture = Fixture::new(&[("lore/truths/T-960.md", TRUTH.as_bytes())])?;
        let result = create("lore/truths/t-960.MD", "x").plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::Exists { .. })));
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn create_allows_another_case_of_the_name_on_linux() -> TestResult {
        let fixture = Fixture::new(&[("lore/truths/T-960.md", TRUTH.as_bytes())])?;
        assert!(
            create("lore/truths/t-960.md", "x")
                .plan(&fixture.ctx())
                .is_ok()
        );
        Ok(())
    }

    #[test]
    fn create_refuses_a_bad_path() -> TestResult {
        let fixture = Fixture::new(&[])?;
        let result = create("../x.md", "x").plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::Path { .. })));
        Ok(())
    }
}
