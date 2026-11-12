//! Plans built by hand, for the write and the pipeline.

use crate::{Command, CommandError, FileChange, Plan, PlanContext, Reads};

/// A change of `path` from the text `before` to the text `after`; `None` for a file that is
/// absent.
pub(crate) fn change(path: &str, before: Option<&str>, after: Option<&str>) -> FileChange {
    FileChange {
        path: path.to_owned(),
        before: before.map(|text| text.as_bytes().to_vec()),
        after: after.map(|text| text.as_bytes().to_vec()),
    }
}

/// A plan of `changes` that moves no file.
pub(crate) fn plan(changes: Vec<FileChange>) -> Plan {
    Plan {
        changes,
        ..Plan::default()
    }
}

/// A command whose plan is given, as a third-party command may build it; it reads nothing.
pub(crate) struct Planned(pub(crate) Plan);

impl Command for Planned {
    fn label(&self) -> String {
        "planned".to_owned()
    }

    fn reads(&self) -> Reads {
        Reads::Paths(Vec::new())
    }

    fn plan(&self, _ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        Ok(self.0.clone())
    }
}
