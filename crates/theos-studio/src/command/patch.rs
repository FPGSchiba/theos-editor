//! The command that patches the front matter of a record.

use theos_content::yaml::Value;
use theos_content::{KindRegistry, Order, PatchOp, Record};

use super::edit::parse;
use super::{Base, Command, CommandError, Plan, PlanContext, Reads, project_path};

/// Applies `ops` to the front matter of the record `path`. New fields go where the order of the
/// record's kind puts them; a record of an unregistered kind keeps the order of its fields.
/// Operations that leave the text as it is plan nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    /// Project-relative path.
    pub path: String,
    /// What the caller saw of the file.
    pub base: Base,
    /// The operations.
    pub ops: Vec<PatchOp>,
}

impl Command for Patch {
    fn label(&self) -> String {
        format!("patch {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let path = project_path(&self.path)?;
        let before = ctx.read_based(&path, &self.base)?;
        let record = parse(&path, &before)?;
        let patched = record
            .patch(&self.ops, &order_of(&record, ctx.registry()))
            .map_err(|source| CommandError::Patch {
                path: path.clone(),
                source,
            })?;
        Ok(Plan::text(path, before, patched.text()))
    }
}

/// The order of the record's kind, or the order of its own fields when the kind is not
/// registered (or the front matter does not parse; the patch then fails by itself).
fn order_of(record: &Record, registry: &KindRegistry) -> Order {
    let Ok(front) = record.front_matter() else {
        return Order::default();
    };
    let kind = front.get("kind").and_then(|field| match &field.value {
        Value::Scalar(scalar) => registry.kind(&scalar.text),
        _ => None,
    });
    kind.map_or_else(
        || Order::new(front.fields.iter().map(|field| field.key.clone())),
        theos_content::KindSpec::order,
    )
}

#[cfg(test)]
mod tests {
    use theos_content::yaml::RawValue;

    use super::*;
    use crate::test_support::{Fixture, TRUTH, TRUTH_PATH, TestResult};

    fn set(key: &str, value: &str) -> PatchOp {
        PatchOp::Set {
            key: key.to_owned(),
            value: RawValue::Text(value.to_owned()),
        }
    }

    fn patch(text: &str, ops: Vec<PatchOp>) -> Patch {
        Patch {
            path: TRUTH_PATH.to_owned(),
            base: Base::of(text.as_bytes()),
            ops,
        }
    }

    #[test]
    fn patch_keeps_the_kind_order() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        let plan = patch(TRUTH, vec![set("status", "draft")]).plan(&fixture.ctx())?;
        let after = String::from_utf8(plan.changes[0].after.clone().ok_or("deleted")?)?;
        // The kind puts `status` after `title` and before `target`.
        let expected = TRUTH.replace("target:", "status: draft\ntarget:");
        assert_eq!(after, expected);
        Ok(())
    }

    #[test]
    fn a_patch_that_changes_nothing_is_an_empty_plan() -> TestResult {
        for text in [TRUTH.to_owned(), TRUTH.replace('\n', "\r\n")] {
            let fixture = Fixture::new(&[(TRUTH_PATH, text.as_bytes())])?;
            for ops in [Vec::new(), vec![set("title", "Old")]] {
                let plan = patch(&text, ops).plan(&fixture.ctx())?;
                assert_eq!(plan, Plan::default(), "{text:?}");
            }
        }
        Ok(())
    }

    #[test]
    fn patch_of_an_unknown_kind_keeps_the_order_of_its_fields() -> TestResult {
        let text = "---\nid: W-1\nkind: widget\ntitle: Old\n---\nBody\n";
        let fixture = Fixture::new(&[(TRUTH_PATH, text.as_bytes())])?;
        let plan = patch(text, vec![set("title", "New")]).plan(&fixture.ctx())?;
        assert_eq!(
            plan.changes[0].after.as_deref(),
            Some(text.replace("Old", "New").as_bytes())
        );
        let result = patch(text, vec![set("extra", "1")]).plan(&fixture.ctx());
        assert!(matches!(result, Err(CommandError::Patch { .. })));
        Ok(())
    }

    #[test]
    fn patch_of_a_file_that_is_not_a_record_is_an_error() -> TestResult {
        let fixture = Fixture::new(&[("a.md", b"plain\n")])?;
        let command = Patch {
            path: "a.md".to_owned(),
            base: Base::of(b"plain\n"),
            ops: vec![set("title", "x")],
        };
        assert!(matches!(
            command.plan(&fixture.ctx()),
            Err(CommandError::Record { .. })
        ));
        Ok(())
    }
}
