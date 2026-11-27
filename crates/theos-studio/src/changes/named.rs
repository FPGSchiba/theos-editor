//! Which changed file is a record, and its name: decided from the file's bytes alone.

use theos_content::yaml::{FrontMatter, Value};
use theos_content::{KindRegistry, Record, is_record_path};

/// What names a record in a list of changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Named {
    /// The file name without `.md` (the data model: the id equals the file name).
    pub id: String,
    /// The kind, one the registry knows.
    pub kind: String,
    /// The text of the `title` scalar, when there is one.
    pub title: Option<String>,
}

/// The name of the record at the project path `path` with the content `bytes`, or `None` when it
/// is no record: the path is not a record path ([`is_record_path`]: settings, templates, tools),
/// the bytes are not UTF-8 or no record ([`Record::parse`]), the front matter does not parse, or
/// its `kind` is not a scalar the registry knows.
pub(super) fn named(path: &str, bytes: &[u8], registry: &KindRegistry) -> Option<Named> {
    if !is_record_path(path) {
        return None;
    }
    let record = Record::parse(std::str::from_utf8(bytes).ok()?).ok()?;
    let front = record.front_matter().ok()?;
    let kind = scalar(front, "kind").filter(|kind| registry.kind(kind).is_some())?;
    let name = path.rsplit('/').next()?;
    Some(Named {
        id: name.strip_suffix(".md")?.to_owned(),
        kind: kind.to_owned(),
        title: scalar(front, "title").map(str::to_owned),
    })
}

/// The text of the scalar field `key` of `front`.
fn scalar<'a>(front: &'a FrontMatter, key: &str) -> Option<&'a str> {
    match &front.get(key)?.value {
        Value::Scalar(scalar) => Some(&scalar.text),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use theos_content::parts;

    use super::*;
    use crate::test_support::TestResult;

    const TRUTH: &str = "---\n# synthetic fixture (P46), not canon\nid: T-930\nkind: truth\n\
        title: Synthetic truth\nstatus: draft\n---\nSynthetic statement.\n";

    #[test]
    fn a_record_of_a_known_kind_is_named_by_its_file_name() -> TestResult {
        let registry = parts::builtin()?;
        assert_eq!(
            named("lore/truths/T-930.md", TRUTH.as_bytes(), &registry),
            Some(Named {
                id: "T-930".to_owned(),
                kind: "truth".to_owned(),
                title: Some("Synthetic truth".to_owned()),
            })
        );
        let untitled = TRUTH.replace("title: Synthetic truth\n", "");
        let found = named("lore/truths/T-930.md", untitled.as_bytes(), &registry);
        assert_eq!(found.map(|named| named.title), Some(None));
        Ok(())
    }

    #[test]
    fn everything_else_is_no_record() -> TestResult {
        let registry = parts::builtin()?;
        let unknown = TRUTH.replace("kind: truth", "kind: synthetic-kind");
        let listed = TRUTH.replace("kind: truth", "kind: [truth]");
        let broken = TRUTH.replace("title: Synthetic truth", "title: &a b");
        for (path, bytes) in [
            ("lore/notes.md", b"Synthetic notes.\n".as_slice()),
            ("lore/truths/T-930.md", unknown.as_bytes()),
            ("lore/truths/T-930.md", listed.as_bytes()),
            ("lore/truths/T-930.md", broken.as_bytes()),
            ("lore/truths/T-930.md", b"---\nkind: truth\n---\n\xff\n"),
            ("templates/records/truth.template.md", TRUTH.as_bytes()),
            (".theos/settings.toml", TRUTH.as_bytes()),
            ("lore/truths/T-930.txt", TRUTH.as_bytes()),
        ] {
            assert_eq!(named(path, bytes, &registry), None, "{path}: {bytes:?}");
        }
        Ok(())
    }
}
