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
    use crate::test_support::{Fixture, TRUTH, TRUTH_PATH, TestResult};

    #[test]
    fn delete_plans_a_removal() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        let command = Delete {
            path: TRUTH_PATH.to_owned(),
            base: Base::of(TRUTH.as_bytes()),
        };
        let plan = command.plan(&fixture.ctx())?;
        assert_eq!(
            plan.changes,
            vec![FileChange {
                path: TRUTH_PATH.to_owned(),
                before: Some(TRUTH.as_bytes().to_vec()),
                after: None,
            }]
        );
        assert_eq!(plan.paths(), vec![TRUTH_PATH.to_owned()]);
        assert!(fixture.root().join(TRUTH_PATH).exists());
        Ok(())
    }
}
