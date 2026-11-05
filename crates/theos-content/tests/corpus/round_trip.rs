//! Round trips over the whole corpus: through the patcher (front matter) and through span edits
//! built from the model (bodies). A replacement is never read from the span it replaces.

use std::error::Error;
use std::fs;

use theos_content::yaml::Value;
use theos_content::{
    Attribute, AttributeValue, CalloutTitle, Link, Order, PatchOp, Record, Span, TextEdit,
    apply_edits, decode,
};

use crate::common::project_records;

/// The order of a record's own fields and entries, and the ops that set each of them to its
/// current value.
struct OwnOps {
    order: Order,
    fields: Vec<PatchOp>,
    entries: Vec<PatchOp>,
}

fn own_ops(record: &Record) -> Result<OwnOps, Box<dyn Error>> {
    let front = record.front_matter().map_err(ToString::to_string)?;
    let mut order = Order::new(front.fields.iter().map(|f| f.key.clone()));
    let mut field_ops = Vec::new();
    let mut entry_ops = Vec::new();
    for field in &front.fields {
        field_ops.push(PatchOp::Set {
            key: field.key.clone(),
            value: field.value.to_raw(),
        });
        if let Value::Map(map) = &field.value {
            order = order.with_entries(&field.key, map.entries.iter().map(|e| e.key.clone()));
            for entry in &map.entries {
                entry_ops.push(PatchOp::SetEntry {
                    key: field.key.clone(),
                    entry: entry.key.clone(),
                    value: entry.value.to_raw(),
                });
            }
        }
    }
    Ok(OwnOps {
        order,
        fields: field_ops,
        entries: entry_ops,
    })
}

// AC: CE-09.1
#[test]
fn corpus_records_round_trip_through_the_patcher() -> Result<(), Box<dyn Error>> {
    let files = project_records()?;
    assert!(!files.is_empty(), "the corpus has records");
    for file in files {
        let name = file.display().to_string();
        let bytes = fs::read(&file)?;
        let record = Record::parse(decode(&bytes)?)?;
        assert_eq!(record.text().as_bytes(), bytes.as_slice(), "{name}");
        let own = own_ops(&record)?;
        for op in own.fields.iter().chain(&own.entries) {
            let out = record.patch(std::slice::from_ref(op), &own.order)?;
            assert_eq!(out.text(), record.text(), "{name}: {op:?}");
        }
        let all: Vec<PatchOp> = own.fields.into_iter().chain(own.entries).collect();
        let out = record.patch(&all, &own.order)?;
        assert_eq!(out.text(), record.text(), "{name}: all ops");
        let again = Record::parse(out.text())?;
        assert_eq!(again.front_matter(), record.front_matter(), "{name}");
        assert_eq!(again.body(), record.body(), "{name}");
    }
    Ok(())
}

/// The model's edits of a body: each replaces a span with text built from the model.
fn body_edits(record: &Record) -> Vec<TextEdit> {
    let body = record.body();
    let mut edits = Vec::new();
    let mut put = |span: Span, replacement: String| edits.push(TextEdit { span, replacement });
    let link = |put: &mut dyn FnMut(Span, String), link: &Link| {
        put(link.target_span, link.target.clone());
        if let (Some(span), Some(text)) = (link.fragment_span, &link.fragment) {
            put(span, text.clone());
        }
        if let (Some(span), Some(text)) = (link.shown_span, &link.shown) {
            put(span, text.clone());
        }
        put(link.span, link.spelling());
    };
    for l in body.links() {
        link(&mut put, l);
    }
    for callout in body.callouts() {
        match &callout.title {
            Some(CalloutTitle::Note(note)) => {
                put(note.state_span, note.state.spelling());
                put(note.id.span, note.id.text.clone());
            }
            Some(CalloutTitle::Voice(voice)) => {
                if let Some(tier) = &voice.tier {
                    put(tier.span, tier.text.clone());
                }
            }
            Some(CalloutTitle::Design(design)) => {
                if let Some(text) = &design.text {
                    put(text.span, text.text.clone());
                }
            }
            _ => {}
        }
    }
    for beat in body.beats() {
        put(beat.number_span, beat.number.to_string());
        put(beat.name_span, beat.name.clone());
        for Attribute {
            value, value_span, ..
        } in &beat.attributes
        {
            put(*value_span, value.spelling());
            if let AttributeValue::Links(links) | AttributeValue::Raw { links, .. } = value {
                for l in links {
                    link(&mut put, l);
                }
            }
        }
    }
    edits
}

/// A subset of `edits` that touch no common bytes: the greedy pick in `order`.
fn disjoint(edits: &[TextEdit], order: &[usize]) -> Vec<TextEdit> {
    let mut picked: Vec<TextEdit> = Vec::new();
    for &i in order {
        let e = &edits[i];
        let clash = picked
            .iter()
            .any(|p| e.span.start < p.span.end && p.span.start < e.span.end || e.span == p.span);
        if !clash {
            picked.push(e.clone());
        }
    }
    picked
}

// AC: CE-10.4
#[test]
fn corpus_bodies_round_trip_through_span_edits() -> Result<(), Box<dyn Error>> {
    let mut edit_count = 0;
    for file in project_records()? {
        let name = file.display().to_string();
        let record = Record::parse(decode(&fs::read(&file)?)?)?;
        let text = record.text();
        let body = record.body();
        let mut cursor = body.span().start;
        for block in body.blocks() {
            assert!(block.span.start >= cursor, "{name}: blocks overlap");
            let gap = Span::new(cursor, block.span.start).get(text).unwrap_or("?");
            assert!(gap.trim().is_empty(), "{name}: gap {gap:?}");
            cursor = block.span.end;
        }
        let tail = Span::new(cursor, body.span().end).get(text).unwrap_or("?");
        assert!(tail.trim().is_empty(), "{name}: tail {tail:?}");

        let edits = body_edits(&record);
        edit_count += edits.len();
        for edit in &edits {
            let out = apply_edits(text, std::slice::from_ref(edit))?;
            assert_eq!(out, text, "{name}: {edit:?}");
        }
        let mut by_start: Vec<usize> = (0..edits.len()).collect();
        by_start.sort_by_key(|&i| (edits[i].span.start, std::cmp::Reverse(edits[i].span.end)));
        let mut by_size = by_start.clone();
        by_size.sort_by_key(|&i| edits[i].span.len());
        for order in [by_start, by_size] {
            let together = disjoint(&edits, &order);
            let out = apply_edits(text, &together)?;
            assert_eq!(out, text, "{name}: together");
        }
    }
    assert!(edit_count > 0, "the corpus bodies have something to edit");
    Ok(())
}
