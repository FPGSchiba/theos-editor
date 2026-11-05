//! Patching front matter of corpus records: only the touched field's lines change.

use std::error::Error;
use std::fs;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use theos_content::yaml::{RawEntry, RawValue, Value};
use theos_content::{Order, PatchError, Record, Span};

use crate::common::{corpus, cut, only_region_changed};

fn check(file: &str, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let path = corpus().join("records").join(file);
    let old = Record::parse(&fs::read_to_string(path)?)?;
    let raw = RawValue::Text(value.to_owned());
    let new = old.set(key, raw.clone(), &Order::default())?;
    let old_field = old
        .front_matter()
        .map_err(ToString::to_string)?
        .get(key)
        .ok_or("field missing")?;
    let new_field = new
        .front_matter()
        .map_err(ToString::to_string)?
        .get(key)
        .ok_or("field missing after set")?;
    assert_eq!(
        cut(old.text(), Some(old_field.lines)),
        cut(new.text(), Some(new_field.lines)),
        "{file} {key}"
    );
    assert_eq!(new_field.value.to_raw(), raw, "{file} {key}");
    Ok(())
}

// AC: CE-09.2
#[test]
fn scalar_edits_change_only_the_field_lines() -> Result<(), Box<dyn Error>> {
    check("lore/clues/C-031.md", "title", "Another plain title")?;
    check("lore/clues/C-031.md", "title", "needs: quotes # here")?;
    check("lore/beliefs/B-901.md", "status", "canon")?;
    Ok(())
}

fn check_list(
    file: &str,
    key: &str,
    change: impl Fn(&[String]) -> Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let path = corpus().join("records").join(file);
    let old = Record::parse(&fs::read_to_string(path)?)?;
    let old_field = old
        .front_matter()
        .map_err(ToString::to_string)?
        .get(key)
        .ok_or("field missing")?;
    let RawValue::List(items) = old_field.value.to_raw() else {
        return Err(format!("{file} {key} is not a list").into());
    };
    let wanted = change(&items);
    let raw = RawValue::List(wanted);
    let new = old.set(key, raw.clone(), &Order::default())?;
    let new_field = new
        .front_matter()
        .map_err(ToString::to_string)?
        .get(key)
        .ok_or("field missing after set")?;
    assert_eq!(
        cut(old.text(), Some(old_field.lines)),
        cut(new.text(), Some(new_field.lines)),
        "{file} {key}"
    );
    assert_eq!(new_field.value.to_raw(), raw, "{file} {key}");
    Ok(())
}

fn with(items: &[String], extra: &str) -> Vec<String> {
    let mut all = items.to_vec();
    all.push(extra.to_owned());
    all
}

fn without_first(items: &[String]) -> Vec<String> {
    items.iter().skip(1).cloned().collect()
}

fn reversed(items: &[String]) -> Vec<String> {
    items.iter().rev().cloned().collect()
}

// AC: CE-09.2
#[test]
fn list_edits_change_only_the_field_lines() -> Result<(), Box<dyn Error>> {
    check_list("lore/clues/C-031.md", "where", |items| {
        with(items, "f04-choir-hall")
    })?;
    check_list("lore/clues/C-031.md", "where", without_first)?;
    check_list("lore/beliefs/B-901.md", "held_by", |items| {
        with(items, "new-kin")
    })?;
    check_list("lore/beliefs/B-901.md", "held_by", without_first)?;
    check_list("lore/beliefs/B-901.md", "held_by", reversed)?;
    let comments = "things/synthetic-yaml-comments.md";
    check_list(comments, "held_by", |items| with(items, "new-kin"))?;
    check_list(comments, "held_by", without_first)?;
    // a reorder can move the comment lines behind the last item, so the old field's lines are
    // the region: every byte outside it is unchanged and the comment and blank line survive in it
    let path = corpus().join("records").join(comments);
    let old = Record::parse(&fs::read_to_string(path)?)?;
    let region = old
        .front_matter()
        .map_err(ToString::to_string)?
        .get("held_by")
        .ok_or("field missing")?
        .lines;
    let raw = RawValue::List(vec!["hearth-folk".to_owned(), "throat-kin".to_owned()]);
    let new = old.set("held_by", raw.clone(), &Order::default())?;
    assert!(only_region_changed(old.text(), new.text(), region));
    let new_region = Span::new(
        region.start,
        new.text().len() - (old.text().len() - region.end),
    );
    let inside = new_region
        .get(new.text())
        .ok_or("region outside the new text")?;
    assert!(inside.contains("  # a comment between block items\n\n"));
    let field = new
        .front_matter()
        .map_err(ToString::to_string)?
        .get("held_by");
    assert_eq!(field.map(|f| f.value.to_raw()), Some(raw));
    Ok(())
}

/// The lines of the field `key` and its raw value, or an error.
fn field_of(record: &Record, key: &str) -> Result<(Span, RawValue), Box<dyn Error>> {
    let field = record
        .front_matter()
        .map_err(ToString::to_string)?
        .get(key)
        .ok_or("field missing")?;
    Ok((field.lines, field.value.to_raw()))
}

fn entry_order(key: &str, entries: &[&str]) -> Order {
    Order::new([key]).with_entries(key, entries.iter().copied())
}

// AC: CE-09.2
#[test]
fn map_edits_change_only_the_field_lines() -> Result<(), Box<dyn Error>> {
    let ilse = corpus().join("records/people/hearth-folk/fates-cult/mother-ilse.md");
    let old = Record::parse(&fs::read_to_string(ilse)?)?;
    let (old_lines, _) = field_of(&old, "lens")?;
    let wary = RawEntry::Text("wary".to_owned());
    let order = entry_order("lens", &["stance", "speaks_to"]);
    let added = old.set_entry("lens", "stance", wary.clone(), &order)?;
    let changed = added.set_entry(
        "lens",
        "speaks_to",
        RawEntry::Text("all".to_owned()),
        &order,
    )?;
    let removed = changed.remove_entry("lens", "speaks_to")?;
    for new in [&added, &changed, &removed] {
        let (new_lines, _) = field_of(new, "lens")?;
        assert_eq!(
            cut(old.text(), Some(old_lines)),
            cut(new.text(), Some(new_lines))
        );
    }
    let (_, raw) = field_of(&removed, "lens")?;
    assert_eq!(raw, RawValue::Map(vec![("stance".to_owned(), wary)]));

    let comments = "things/synthetic-yaml-comments.md";
    let path = corpus().join("records").join(comments);
    let old = Record::parse(&fs::read_to_string(path)?)?;
    let (region, _) = field_of(&old, "lens")?;
    let order = entry_order("lens", &["speaks_to", "stance", "role"]);
    let added = old.set_entry("lens", "stance", RawEntry::Text("calm".to_owned()), &order)?;
    let edited = added.set_entry("lens", "role", RawEntry::Text("warden".to_owned()), &order)?;
    let removed = edited.remove_entry("lens", "speaks_to")?;
    for new in [&added, &edited, &removed] {
        assert!(only_region_changed(old.text(), new.text(), region));
    }
    // the comments between the entries survive in the untouched entries' neighbourhood
    assert!(added.text().contains(
        "  speaks_to: children   # a comment after an entry\n  stance: calm\n  # a comment between entries\n  role: keeper\n"
    ));
    assert!(
        edited
            .text()
            .contains("  role: warden\n  # a comment after the last entry\n")
    );
    assert!(removed.text().contains(
        "  # a comment before the first entry\n  stance: calm\n  # a comment between entries\n"
    ));
    Ok(())
}

fn load(file: &str) -> Result<Record, Box<dyn Error>> {
    Ok(Record::parse(&fs::read_to_string(
        corpus().join("records").join(file),
    )?)?)
}

/// The keys of the fields in file order.
fn keys(record: &Record) -> Result<Vec<String>, Box<dyn Error>> {
    let front = record.front_matter().map_err(ToString::to_string)?;
    Ok(front.fields.iter().map(|f| f.key.clone()).collect())
}

/// Inserts `key` by `order`: only the new field's lines are added, it reads back as `value`, and
/// the fields come in the order `expected`.
fn check_insert(
    file: &str,
    key: &str,
    order: &[&str],
    expected: &[&str],
) -> Result<(), Box<dyn Error>> {
    let old = load(file)?;
    let raw = RawValue::Text("x".to_owned());
    let new = old.set(key, raw.clone(), &Order::new(order.iter().copied()))?;
    let (lines, read) = field_of(&new, key)?;
    assert_eq!(
        cut(new.text(), Some(lines)),
        old.text(),
        "{file} insert {key}"
    );
    assert_eq!(read, raw, "{file} insert {key}");
    assert_eq!(keys(&new)?, expected, "{file} insert {key}");
    Ok(())
}

const F04: &str = "world/hearth/f04-choir-hall.md";

// AC: CE-09.2
#[test]
fn edit_matrix_changes_only_the_field_lines() -> Result<(), Box<dyn Error>> {
    let comments = "things/synthetic-yaml-comments.md";
    // a scalar: plain to plain, to a spelling that needs quotes, each with its trailing comment
    check(comments, "type", "tool")?;
    check(comments, "type", "a: b # c")?;
    check(comments, "status", "canon")?;
    check(F04, "title", "The Choir Hall")?;
    // a flow list, then block lists with comments inside
    check_list(comments, "where", |items| with(items, "f04-terrain"))?;
    check_list(comments, "where", without_first)?;
    check_list(comments, "held_by", |items| with(items, "new-kin"))?;
    check_list(comments, "held_by", without_first)?;
    check_list(comments, "requires", reversed)?;
    check_list("lore/beliefs/B-901.md", "held_by", reversed)?;
    // a block list reordered with comments: the comments move inside the field's lines
    let old = load(comments)?;
    let (region, _) = field_of(&old, "held_by")?;
    let raw = RawValue::List(vec!["hearth-folk".to_owned(), "throat-kin".to_owned()]);
    let new = old.set("held_by", raw.clone(), &Order::default())?;
    assert!(only_region_changed(old.text(), new.text(), region));
    assert_eq!(field_of(&new, "held_by")?.1, raw);
    // lens.stance: a new entry, only that entry's lines are added
    let old = load("people/hearth-folk/fates-cult/mother-ilse.md")?;
    let order = entry_order("lens", &["stance", "speaks_to"]);
    let new = old.set_entry("lens", "stance", RawEntry::Text("wary".to_owned()), &order)?;
    let lens = new
        .front_matter()
        .map_err(ToString::to_string)?
        .get("lens")
        .ok_or("no lens")?;
    let Value::Map(map) = &lens.value else {
        return Err("lens is not a map".into());
    };
    assert_eq!(map.entries[0].key, "stance");
    assert_eq!(cut(new.text(), Some(map.entries[0].lines)), old.text());
    // inserting a missing field at the start, in the middle, at the end, and tags after status
    let own = ["id", "kind", "title", "parent", "number", "status"];
    check_insert(
        F04,
        "zero",
        &["zero", "id", "kind", "title", "parent", "number", "status"],
        &["zero", "id", "kind", "title", "parent", "number", "status"],
    )?;
    check_insert(
        F04,
        "mid",
        &["id", "kind", "mid", "title", "parent", "number", "status"],
        &["id", "kind", "mid", "title", "parent", "number", "status"],
    )?;
    check_insert(
        F04,
        "last",
        &["id", "kind", "title", "parent", "number", "status", "last"],
        &["id", "kind", "title", "parent", "number", "status", "last"],
    )?;
    let old = load(F04)?;
    assert_eq!(keys(&old)?, own);
    let order = Order::new(["id", "kind", "title", "parent", "number", "status", "tags"]);
    let tags = RawValue::List(vec!["a".to_owned(), "b".to_owned()]);
    let new = old.set("tags", tags.clone(), &order)?;
    let (lines, read) = field_of(&new, "tags")?;
    assert_eq!(cut(new.text(), Some(lines)), old.text());
    assert_eq!(read, tags);
    let found = keys(&new)?;
    let at = |key: &str| found.iter().position(|name| name == key);
    assert_eq!(at("tags"), at("status").map(|i| i + 1));
    // removing a field and a map entry: only their lines are gone
    let (lines, _) = field_of(&old, "parent")?;
    let new = old.remove("parent")?;
    assert_eq!(new.text(), cut(old.text(), Some(lines)));
    assert!(!keys(&new)?.contains(&"parent".to_owned()));
    let old = load(comments)?;
    let lens = old
        .front_matter()
        .map_err(ToString::to_string)?
        .get("lens")
        .ok_or("no lens")?;
    let Value::Map(map) = &lens.value else {
        return Err("lens is not a map".into());
    };
    let new = old.remove_entry("lens", "speaks_to")?;
    assert_eq!(new.text(), cut(old.text(), Some(map.entries[0].lines)));
    let left = RawValue::Map(vec![(
        "role".to_owned(),
        RawEntry::Text("keeper".to_owned()),
    )]);
    assert_eq!(field_of(&new, "lens")?.1, left);
    Ok(())
}

fn fail(error: &dyn ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// Sets `raw` on `key` of `file`: a text without a line break changes only the field's lines and
/// reads back; a text with one is refused as `Encode`.
fn property(file: &str, key: &str, raw: RawValue, has_break: bool) -> Result<(), TestCaseError> {
    let old = load(file).map_err(|e| fail(&e))?;
    match old.set(key, raw.clone(), &Order::default()) {
        Ok(new) => {
            prop_assert!(!has_break, "a line break was written");
            let (old_lines, _) = field_of(&old, key).map_err(|e| fail(&e))?;
            let (new_lines, read) = field_of(&new, key).map_err(|e| fail(&e))?;
            prop_assert_eq!(
                cut(old.text(), Some(old_lines)),
                cut(new.text(), Some(new_lines))
            );
            prop_assert_eq!(read, raw);
        }
        Err(PatchError::Encode(_)) => {
            prop_assert!(has_break, "a text without a line break was refused");
        }
        Err(other) => return Err(fail(&other)),
    }
    Ok(())
}

// Texts the encoder accepts: no control character but the tab (a line break is one, and is
// tested on its own below).
const PRINTABLE_24: &str = "([^\\p{Cc}]|\t){0,24}";
const PRINTABLE_12: &str = "([^\\p{Cc}]|\t){0,12}";

proptest! {
    #[test]
    fn random_scalars_change_only_their_field(text in PRINTABLE_24) {
        property(F04, "title", RawValue::Text(text), false)?;
    }

    #[test]
    fn scalars_with_a_line_break_are_refused(
        a in "[^\n\r]{0,6}",
        b in "[^\n\r]{0,6}",
        cr in any::<bool>(),
    ) {
        let text = format!("{a}{}{b}", if cr { '\r' } else { '\n' });
        property(F04, "title", RawValue::Text(text), true)?;
    }

    #[test]
    fn random_lists_change_only_their_field(
        items in proptest::collection::vec(PRINTABLE_12, 0..5),
    ) {
        property("lore/clues/C-031.md", "where", RawValue::List(items), false)?;
    }

    #[test]
    fn lists_with_a_line_break_are_refused(a in "[^\n\r]{0,6}", b in "[^\n\r]{0,6}") {
        let items = vec!["x".to_owned(), format!("{a}\n{b}")];
        property("lore/clues/C-031.md", "where", RawValue::List(items), true)?;
    }
}
