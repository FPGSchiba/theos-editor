//! The front-matter patcher: setting, inserting and removing whole fields as span edits, each
//! result verified by parsing it again. Per-item list edits (`list.rs`) and per-entry map edits (`map.rs`) build on this.

mod field;
mod list;
mod map;
mod verify;

use std::collections::BTreeMap;

use super::{EncodeError, RawEntry, RawValue, YamlError};
use crate::record::{Record, RecordEditError};

/// One change to the front matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchOp {
    /// Write `value` as the field `key`: replaces the field when present, inserts it otherwise.
    Set {
        /// The field.
        key: String,
        /// The raw value to write.
        value: RawValue,
    },
    /// Delete the field `key`.
    Remove {
        /// The field.
        key: String,
    },
    /// Write `value` as the entry `entry` of the map field `key`: replaces the entry when present,
    /// inserts it by the entry order otherwise, and inserts the field when it is missing.
    SetEntry {
        /// The map field.
        key: String,
        /// The entry of the map.
        entry: String,
        /// The raw value to write.
        value: RawEntry,
    },
    /// Delete the entry `entry` of the map field `key`; the field goes with its last entry.
    RemoveEntry {
        /// The map field.
        key: String,
        /// The entry of the map.
        entry: String,
    },
}

/// The order the caller wants fields (and, for a map field, its entries) written in. Insertions
/// follow it; fields and entries it does not name keep their place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Order {
    fields: Vec<String>,
    entries: BTreeMap<String, Vec<String>>,
}

impl Order {
    /// An order of top-level fields.
    #[must_use]
    pub fn new<I, S>(fields: I) -> Order
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Order {
            fields: fields.into_iter().map(Into::into).collect(),
            entries: BTreeMap::new(),
        }
    }

    /// Adds the order of the entries of the map field `key`.
    #[must_use]
    pub fn with_entries<I, S>(mut self, key: &str, entries: I) -> Order
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.entries.insert(
            key.to_owned(),
            entries.into_iter().map(Into::into).collect(),
        );
        self
    }

    /// The entry order of the map field `key`; empty when none was given.
    #[must_use]
    pub fn entries_of(&self, key: &str) -> &[String] {
        self.entries.get(key).map_or(&[], Vec::as_slice)
    }

    /// The place of the field `key` in the order.
    pub(crate) fn position(&self, key: &str) -> Option<usize> {
        self.fields.iter().position(|name| name == key)
    }
}

/// What is wrong with a map that [`Record::set`] was asked to write.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MapProblem {
    /// The map has no entry; a field is removed with [`Record::remove`].
    #[error("a map needs at least one entry")]
    Empty,
    /// The map names an entry twice.
    #[error("the entry `{0}` occurs twice")]
    DuplicateEntry(String),
}

/// Why a patch produced no record.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    /// The front matter of the record does not parse, so there are no spans to edit.
    #[error("the front matter is invalid: {0}")]
    FrontMatter(YamlError),
    /// A key that does not match `^[a-z][a-z0-9_]*$`.
    #[error("`{key}` is not a valid key")]
    Key {
        /// The refused key.
        key: String,
    },
    /// The field (or the entry of it) does not exist.
    #[error("no such field or entry: {key}{}", entry.as_ref().map(|e| format!(".{e}")).unwrap_or_default())]
    NotFound {
        /// The field.
        key: String,
        /// The entry of the field, for entry edits.
        entry: Option<String>,
    },
    /// An entry edit on a field that is not a map.
    #[error("`{key}` is not a map")]
    NotAMap {
        /// The field.
        key: String,
    },
    /// A new field that the caller's order does not name.
    #[error("`{key}` is not in the given order, so there is no place to insert it")]
    NotInOrder {
        /// The field.
        key: String,
    },
    /// A map value the front matter cannot hold; remove a field with `remove`, not an empty map.
    #[error("`{key}` cannot be written as a map: {reason}")]
    Map {
        /// The field.
        key: String,
        /// What is wrong with the map.
        reason: MapProblem,
    },
    /// A value the front matter cannot hold.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// The edits do not apply or do not leave a record.
    #[error(transparent)]
    Edit(#[from] RecordEditError),
    /// The patched record does not read back as asked; nothing was returned.
    #[error("the patched record failed verification: {reason}")]
    Verify {
        /// What differs.
        reason: String,
    },
}

impl Record {
    /// Applies `ops` one after the other; each op makes its edits, the edits are applied, the
    /// result is verified, and the next op runs on that record. The result is an LF record.
    ///
    /// # Errors
    ///
    /// The [`PatchError`] of the first op that fails; no text is returned then.
    /// [`PatchError::Map`] when a `Set` writes a map with no entry or a repeated entry name.
    pub fn patch(&self, ops: &[PatchOp], order: &Order) -> Result<Record, PatchError> {
        let mut current = self.clone();
        for op in ops {
            let edits = field::edits(&current, op, order)?;
            let next = current.with_edits(&edits)?;
            verify::verify(&current, &next, op)?;
            current = next;
        }
        Ok(current)
    }

    /// Sets the field `key` to `value`, inserting it by `order` when it is missing.
    ///
    /// # Errors
    ///
    /// See [`Record::patch`].
    pub fn set(&self, key: &str, value: RawValue, order: &Order) -> Result<Record, PatchError> {
        let op = PatchOp::Set {
            key: key.to_owned(),
            value,
        };
        self.patch(&[op], order)
    }

    /// Removes the field `key`.
    ///
    /// # Errors
    ///
    /// See [`Record::patch`]; [`PatchError::NotFound`] when the field is absent.
    pub fn remove(&self, key: &str) -> Result<Record, PatchError> {
        let op = PatchOp::Remove {
            key: key.to_owned(),
        };
        self.patch(&[op], &Order::default())
    }

    /// Sets the entry `entry` of the map field `key`; a new entry goes where `order` puts it.
    ///
    /// # Errors
    ///
    /// See [`Record::patch`]; [`PatchError::NotAMap`] when the field is not a map.
    pub fn set_entry(
        &self,
        key: &str,
        entry: &str,
        value: RawEntry,
        order: &Order,
    ) -> Result<Record, PatchError> {
        let op = PatchOp::SetEntry {
            key: key.to_owned(),
            entry: entry.to_owned(),
            value,
        };
        self.patch(&[op], order)
    }

    /// Removes the entry `entry` of the map field `key`; the field goes with its last entry.
    ///
    /// # Errors
    ///
    /// See [`Record::patch`]; [`PatchError::NotFound`] when the field or the entry is absent and
    /// [`PatchError::NotAMap`] when the field is not a map.
    pub fn remove_entry(&self, key: &str, entry: &str) -> Result<Record, PatchError> {
        let op = PatchOp::RemoveEntry {
            key: key.to_owned(),
            entry: entry.to_owned(),
        };
        self.patch(&[op], &Order::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;

    const F04: &str =
        include_str!("../../../tests/fixtures/corpus/records/world/hearth/f04-choir-hall.md");
    const B901: &str = include_str!("../../../tests/fixtures/corpus/records/lore/beliefs/B-901.md");

    fn text(value: &str) -> RawValue {
        RawValue::Text(value.to_owned())
    }

    fn parse(input: &str) -> Result<Record, Box<dyn std::error::Error>> {
        Ok(Record::parse(input)?)
    }

    fn none() -> Order {
        Order::default()
    }

    // AC: CE-09.2
    #[test]
    fn setting_a_scalar_changes_only_its_value() -> Result<(), Box<dyn std::error::Error>> {
        let f04 = parse(F04)?;
        let set = f04.set("title", text("The Choir Hall"), &none())?;
        assert_eq!(
            set.text(),
            F04.replace("title: The Choir-Hall", "title: The Choir Hall")
        );

        let b901 = parse(B901)?;
        let set = b901.set("status", text("canon"), &none())?;
        assert_eq!(
            set.text(),
            B901.replace("status: draft   # a comment", "status: canon   # a comment")
        );

        let set = f04.set("title", text("a: b"), &none())?;
        assert_eq!(
            set.text(),
            F04.replace("title: The Choir-Hall", "title: \"a: b\"")
        );

        let set = b901.set("accuracy", text("false"), &none())?;
        assert_eq!(
            set.text(),
            B901.replace("accuracy: 'partial'", "accuracy: 'false'")
        );

        let same = f04.set("title", text("The Choir-Hall"), &none())?;
        assert_eq!(same.text(), F04);
        Ok(())
    }

    // AC: CE-09.2
    #[test]
    fn inserting_follows_the_callers_order() -> Result<(), Box<dyn std::error::Error>> {
        let order = Order::new(["id", "kind", "title", "status", "tags", "parent", "number"]);
        let f04 = parse(F04)?;
        let set = f04.set("tags", RawValue::List(vec!["a".into(), "b".into()]), &order)?;
        assert_eq!(
            set.text(),
            F04.replace("status: draft\n", "status: draft\ntags: [a, b]\n")
        );

        let order = Order::new(["zero", "id", "kind", "title", "status"]);
        let set = f04.set("zero", text("x"), &order)?;
        assert_eq!(set.text(), F04.replace("id: f04", "zero: x\nid: f04"));

        let empty = parse("---\n---\n")?;
        let set = empty.set("a", text("x"), &Order::new(["a"]))?;
        assert_eq!(set.text(), "---\na: x\n---\n");

        let commented = parse("---\na: 1\n# c\nb: 2\n---\n")?;
        let set = commented.set("aa", text("x"), &Order::new(["a", "aa", "b"]))?;
        assert_eq!(set.text(), "---\na: 1\naa: x\n# c\nb: 2\n---\n");

        let map = RawValue::Map(vec![("e".into(), crate::yaml::RawEntry::Text("v".into()))]);
        let set = empty.set("m", map, &Order::new(["m"]))?;
        assert_eq!(set.text(), "---\nm:\n  e: v\n---\n");
        Ok(())
    }

    // AC: CE-09.2
    #[test]
    fn removing_a_field_removes_its_lines() -> Result<(), Box<dyn std::error::Error>> {
        let removed = parse(F04)?.remove("parent")?;
        assert_eq!(removed.text(), F04.replace("parent: hearth\n", ""));
        Ok(())
    }

    #[test]
    fn own_line_comments_stay_put() -> Result<(), Box<dyn std::error::Error>> {
        let rec = record("a: 1\n# about b\nb: 2\n# after b\nc: 3\n", "");
        let removed = rec.remove("b")?;
        assert_eq!(
            removed.text(),
            "---\na: 1\n# about b\n# after b\nc: 3\n---\n"
        );
        let order = Order::new(["a", "b", "bb", "c"]);
        let set = rec.set("bb", text("x"), &order)?;
        assert_eq!(
            set.text(),
            "---\na: 1\n# about b\nb: 2\nbb: x\n# after b\nc: 3\n---\n"
        );
        Ok(())
    }

    #[test]
    fn shape_changes_rewrite_the_field() -> Result<(), Box<dyn std::error::Error>> {
        let rec = record("role: x   # c\nother: 1\n", "");
        let list = RawValue::List(vec!["a".into(), "b".into()]);
        let set = rec.set("role", list, &none())?;
        assert_eq!(set.text(), "---\nrole: [a, b] # c\nother: 1\n---\n");
        let back = set.set("role", text("y"), &none())?;
        assert_eq!(back.text(), "---\nrole: y # c\nother: 1\n---\n");
        Ok(())
    }

    #[test]
    fn refusals_return_no_text() {
        let rec = record("a: 1\n", "");
        let order = Order::new(["a", "b"]);
        assert!(matches!(
            rec.set("a", text("x\ny"), &order),
            Err(PatchError::Encode(_))
        ));
        assert!(matches!(
            rec.set("Bad", text("x"), &order),
            Err(PatchError::Key { .. })
        ));
        assert!(matches!(
            rec.set("c", text("x"), &order),
            Err(PatchError::NotInOrder { .. })
        ));
        assert!(matches!(rec.remove("zz"), Err(PatchError::NotFound { .. })));
    }

    #[test]
    fn maps_the_front_matter_cannot_hold_are_refused_up_front() {
        use crate::yaml::RawEntry;
        let rec = record("m:\n  e: v\na: 1\n", "");
        let order = Order::new(["m", "a", "n"]);
        let twice = vec![
            ("e".to_owned(), RawEntry::Text("1".into())),
            ("e".to_owned(), RawEntry::Text("2".into())),
        ];
        for key in ["m", "n"] {
            let empty = rec.set(key, RawValue::Map(vec![]), &order);
            assert!(
                matches!(
                    &empty,
                    Err(PatchError::Map {
                        reason: MapProblem::Empty,
                        ..
                    })
                ),
                "{key}: {empty:?}"
            );
            let dup = rec.set(key, RawValue::Map(twice.clone()), &order);
            assert!(
                matches!(&dup, Err(PatchError::Map { reason: MapProblem::DuplicateEntry(name), .. }) if name == "e"),
                "{key}: {dup:?}"
            );
        }
        let message = rec.set("n", RawValue::Map(vec![]), &order).map(|_| ());
        assert_eq!(
            message.map_err(|e| e.to_string()),
            Err("`n` cannot be written as a map: a map needs at least one entry".to_owned())
        );
    }

    #[test]
    fn patching_a_broken_front_matter_fails() {
        let rec = record("a: &x 1\n", "");
        assert!(matches!(
            rec.set("a", text("2"), &none()),
            Err(PatchError::FrontMatter(_))
        ));
    }

    // AC: CE-09.4
    #[test]
    fn writes_turn_crlf_into_lf() -> Result<(), Box<dyn std::error::Error>> {
        let crlf = parse("---\r\ntitle: a\r\n---\r\nb\r\n")?;
        let lf = parse("---\ntitle: a\n---\nb\n")?;
        let set = crlf.set("title", text("a"), &none())?;
        assert!(!set.text().contains('\r'));
        assert_eq!(set.text(), lf.text());
        Ok(())
    }
}
