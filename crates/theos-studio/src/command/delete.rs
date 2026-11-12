//! The command that deletes a file.

use super::{Base, Command, CommandError, FileChange, Plan, PlanContext, Reads, project_path};

/// Deletes the file `path`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delete {
    /// Project-relative path.
    pub path: String,
    /// What the caller saw of the file.
    pub base: Base,
}

impl Command for Delete {
    fn label(&self) -> String {
        format!("delete {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let path = project_path(&self.path)?;
        let before = ctx.read_based(&path, &self.base)?;
        Ok(Plan {
            changes: vec![FileChange {
                path,
                before: Some(before),
                after: None,
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

    #[test]
    fn a_base_that_differs_is_stale() -> TestResult {
        let fixture = Fixture::new(&[(PATH, TRUTH.as_bytes())])?;
        let command = Delete {
            path: PATH.to_owned(),
            base: Base::of(b"other"),
        };
        assert!(matches!(
            command.plan(&fixture.ctx()),
            Err(CommandError::Stale { .. })
        ));
        Ok(())
    }

    #[test]
    fn delete_plans_a_removal() -> TestResult {
        let fixture = Fixture::new(&[(PATH, TRUTH.as_bytes())])?;
        let command = Delete {
            path: PATH.to_owned(),
            base: Base::of(TRUTH.as_bytes()),
        };
        let plan = command.plan(&fixture.ctx())?;
        assert_eq!(
            plan.changes,
            vec![FileChange {
                path: PATH.to_owned(),
                before: Some(TRUTH.as_bytes().to_vec()),
                after: None,
            }]
        );
        assert_eq!(plan.paths(), vec![PATH.to_owned()]);
        assert!(fixture.root().join(PATH).exists());
        Ok(())
    }
}
