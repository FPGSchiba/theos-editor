//! Patching front matter of corpus records: only the touched field's lines change.

use std::error::Error;
use std::fs;

use theos_content::yaml::RawValue;
use theos_content::{Order, Record};

use crate::common::{corpus, cut};

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
    // a reorder can move the comment lines out of the field's lines, so only their survival and
    // the read-back are checked
    let path = corpus().join("records").join(comments);
    let old = Record::parse(&fs::read_to_string(path)?)?;
    let raw = RawValue::List(vec!["hearth-folk".to_owned(), "throat-kin".to_owned()]);
    let new = old.set("held_by", raw.clone(), &Order::default())?;
    assert!(new.text().contains(
        "  # a comment between block items
"
    ));
    let field = new
        .front_matter()
        .map_err(ToString::to_string)?
        .get("held_by");
    assert_eq!(field.map(|f| f.value.to_raw()), Some(raw));
    Ok(())
}
