//! Per-item edits of a list: the items an item diff keeps are not touched, so their bytes, comments
//! and spacing survive; only removed and new items produce edits.

use std::ops::Range;

use similar::{Algorithm, DiffOp, capture_diff_slices};

use super::PatchError;
use crate::text::{Span, TextEdit};
use crate::yaml::{Item, List, Slot, encode};

/// What changes between two kept items: the old items that go and the new items that come.
///
/// A gap follows the kept item `anchor` (the one before the first kept item when `None`).
struct Gap {
    anchor: Option<usize>,
    removed: Range<usize>,
    inserted: Range<usize>,
}

impl Gap {
    fn after(anchor: Option<usize>, old: usize, new: usize) -> Gap {
        Gap {
            anchor,
            removed: old..old,
            inserted: new..new,
        }
    }

    fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.inserted.is_empty()
    }
}

/// The gaps between the kept items of `old` and `new`; `Replace` is a delete plus an insert.
fn gaps(old: &[&str], new: &[&str]) -> Vec<Gap> {
    let mut found = Vec::new();
    let mut gap = Gap::after(None, 0, 0);
    for op in capture_diff_slices(Algorithm::Myers, old, new) {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                let next = Gap::after(Some(old_index + len - 1), old_index + len, new_index + len);
                found.push(std::mem::replace(&mut gap, next));
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => gap.removed.end = old_index + old_len,
            DiffOp::Insert {
                new_index, new_len, ..
            } => gap.inserted.end = new_index + new_len,
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                gap.removed.end = old_index + old_len;
                gap.inserted.end = new_index + new_len;
            }
        }
    }
    found.push(gap);
    found.retain(|gap| !gap.is_empty());
    found
}

/// The refusal for a list whose model lacks a span the edits need; the model builds them all.
fn missing(what: &str) -> PatchError {
    PatchError::Verify {
        reason: format!("the list model has no {what}"),
    }
}

fn encode_all(new: &[String], range: &Range<usize>, slot: Slot) -> Result<Vec<String>, PatchError> {
    let texts = new.get(range.clone()).ok_or_else(|| missing("new item"))?;
    Ok(texts
        .iter()
        .map(|text| encode(text, None, slot))
        .collect::<Result<_, _>>()?)
}

/// Every text of `parts` with `before` in front of it and `after` behind it.
fn wrapped(parts: &[String], before: &str, after: &str) -> String {
    parts
        .iter()
        .flat_map(|part| [before, part.as_str(), after])
        .collect()
}

fn item(list: &List, index: usize) -> Result<&Item, PatchError> {
    list.items.get(index).ok_or_else(|| missing("item"))
}

fn texts<'a>(list: &'a List, new: &'a [String]) -> (Vec<&'a str>, Vec<&'a str>) {
    let old = list.items.iter().map(|i| i.scalar.text.as_str()).collect();
    (old, new.iter().map(String::as_str).collect())
}

/// The edits that turn the flow `list` into `new`; nothing outside `[` … `]` changes.
pub(super) fn flow_edits(list: &List, new: &[String]) -> Result<Vec<TextEdit>, PatchError> {
    let (old_texts, new_texts) = texts(list, new);
    let gaps = gaps(&old_texts, &new_texts);
    let all_replaced = gaps
        .first()
        .is_some_and(|gap| gap.removed.len() == list.items.len());
    if all_replaced {
        let spelled = encode_all(new, &(0..new.len()), Slot::FlowItem)?;
        return Ok(vec![TextEdit {
            span: list.span,
            replacement: format!("[{}]", spelled.join(", ")),
        }]);
    }
    gaps.iter().map(|gap| flow_gap(list, new, gap)).collect()
}

fn flow_gap(list: &List, new: &[String], gap: &Gap) -> Result<TextEdit, PatchError> {
    let spelled = encode_all(new, &gap.inserted, Slot::FlowItem)?;
    if let Some(kept) = gap.anchor {
        // after a kept item: the removed items go with the separator before them
        let start = item(list, kept)?.scalar.span.end;
        let end = if gap.removed.is_empty() {
            start
        } else {
            item(list, gap.removed.end - 1)?.scalar.span.end
        };
        return Ok(TextEdit {
            span: Span::new(start, end),
            replacement: wrapped(&spelled, ", ", ""),
        });
    }
    // before the first kept item: the removed items go with the separator after them
    let start = item(list, 0)?.scalar.span.start;
    let end = if gap.removed.is_empty() {
        start
    } else {
        item(list, gap.removed.end)?.scalar.span.start
    };
    Ok(TextEdit {
        span: Span::new(start, end),
        replacement: wrapped(&spelled, "", ", "),
    })
}

/// The edits that turn the block `list` into `new` (at least one item): removed items lose their
/// lines, new items are lines of the first item's indent; comment and blank lines stay.
pub(super) fn block_edits(
    text: &str,
    list: &List,
    new: &[String],
) -> Result<Vec<TextEdit>, PatchError> {
    let first = item(list, 0)?.lines.ok_or_else(|| missing("item line"))?;
    let line = first.get(text).ok_or_else(|| missing("item line"))?;
    let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
    let (old_texts, new_texts) = texts(list, new);
    let mut edits = Vec::new();
    for gap in gaps(&old_texts, &new_texts) {
        for index in gap.removed.clone() {
            let lines = item(list, index)?
                .lines
                .ok_or_else(|| missing("item line"))?;
            edits.push(TextEdit {
                span: lines,
                replacement: String::new(),
            });
        }
        let at = match gap.anchor {
            Some(kept) => {
                item(list, kept)?
                    .lines
                    .ok_or_else(|| missing("item line"))?
                    .end
            }
            None => first.start,
        };
        let spelled = encode_all(new, &gap.inserted, Slot::BlockItem)?;
        let lines = wrapped(&spelled, &format!("{indent}- "), "\n");
        if !lines.is_empty() {
            edits.push(TextEdit {
                span: Span::new(at, at),
                replacement: lines,
            });
        }
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use crate::record::Record;
    use crate::test_support::record;
    use crate::yaml::RawValue;
    use crate::yaml::patch::Order;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The record text after writing `items` over the list `l` of `front`.
    fn written(front: &str, items: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let rec: Record = record(front, "");
        let list = RawValue::List(items.iter().map(|item| (*item).to_owned()).collect());
        Ok(rec.set("l", list, &Order::default())?.text().to_owned())
    }

    /// Asserts that writing `items` over `front` gives `want` (both are front-matter lines).
    fn check(front: &str, items: &[&str], want: &str) -> TestResult {
        assert_eq!(
            written(front, items)?,
            format!("---\n{want}---\n"),
            "{front:?} -> {items:?}"
        );
        Ok(())
    }

    // AC: CE-09.2
    #[test]
    fn flow_lists_change_per_item() -> TestResult {
        check("l: [a, b, c]\n", &["a", "c"], "l: [a, c]\n")?;
        check("l: [a, b, c]\n", &["b", "c"], "l: [b, c]\n")?;
        check("l: [a, b, c]\n", &["a", "b"], "l: [a, b]\n")?;
        check("l: [a,b]\n", &["a", "b", "c"], "l: [a,b, c]\n")?;
        check("l: [ a ]\n", &["b", "a"], "l: [ b, a ]\n")?;
        check("l: ['a', \"b\"]\n", &["a", "b"], "l: ['a', \"b\"]\n")?;
        check("l: [a, b,]\n", &["a"], "l: [a,]\n")?;
        check("l: [a, b,]\n", &["a", "b", "c"], "l: [a, b, c,]\n")?;
        check("l: []\n", &["a", "b"], "l: [a, b]\n")?;
        check("l: [a]\n", &[], "l: []\n")?;
        check("l: [a, b, c]\n", &["c", "a", "b"], "l: [c, a, b]\n")?;
        check("l: [a]\n", &["a", "x, y"], "l: [a, \"x, y\"]\n")?;
        check("l: [a, b]  # c\n", &["a"], "l: [a]  # c\n")?;
        Ok(())
    }

    // AC: CE-09.2
    #[test]
    fn block_lists_change_per_item() -> TestResult {
        let abc = "l:\n  - a\n  - b\n  - c\n";
        check(abc, &["a", "c"], "l:\n  - a\n  - c\n")?;
        check(
            abc,
            &["a", "d", "b", "c"],
            "l:\n  - a\n  - d\n  - b\n  - c\n",
        )?;
        check(
            abc,
            &["z", "a", "b", "c"],
            "l:\n  - z\n  - a\n  - b\n  - c\n",
        )?;
        check("l:\n- a\n- b\n", &["a", "c", "b"], "l:\n- a\n- c\n- b\n")?;
        check(
            "l:\n  - a  # x\n  - b  # y\n  - c\n",
            &["c", "a", "b"],
            "l:\n  - c\n  - a  # x\n  - b  # y\n",
        )?;
        check("l:   # k\n  - a\n  - b\n", &[], "l: [] # k\n")?;
        Ok(())
    }

    #[test]
    fn block_list_comments_stay_put() -> TestResult {
        let front = "l:\n  - a  # first\n  # between\n  - b\n";
        check(front, &["b"], "l:\n  # between\n  - b\n")?;
        check(
            front,
            &["a", "x", "b"],
            "l:\n  - a  # first\n  - x\n  # between\n  - b\n",
        )?;
        Ok(())
    }

    #[test]
    fn duplicates_and_replacements() -> TestResult {
        check("l: [a, a, b]\n", &["a", "b"], "l: [a, b]\n")?;
        check("l: [a, b]\n", &["x", "y"], "l: [x, y]\n")?;
        check(
            "l:\n  - a\n  - a\n  - b\n",
            &["a", "b"],
            "l:\n  - a\n  - b\n",
        )?;
        Ok(())
    }
}
