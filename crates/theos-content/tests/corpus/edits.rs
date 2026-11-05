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
