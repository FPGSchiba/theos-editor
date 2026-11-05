//! The `records/` corpus parses, its ids are unique, and `crlf/` inputs read like their LF twins.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;

use serde::Deserialize;
use theos_content::yaml::Value;
use theos_content::{Order, PatchOp, Record, decode};

use crate::common::{corpus, md_files, project_records};

fn parse(bytes: &[u8]) -> Result<Record, Box<dyn Error>> {
    Ok(Record::parse(decode(bytes)?)?)
}

#[test]
fn corpus_records_parse() -> Result<(), Box<dyn Error>> {
    let files = project_records()?;
    assert!(!files.is_empty(), "the corpus has records");
    for file in files {
        let record = parse(&fs::read(&file)?).map_err(|e| format!("{}: {e}", file.display()))?;
        if let Err(error) = record.front_matter() {
            return Err(format!("{}: {error}", file.display()).into());
        }
    }
    Ok(())
}

#[test]
fn corpus_ids_are_unique_and_match_file_names() -> Result<(), Box<dyn Error>> {
    let mut seen = BTreeSet::new();
    for file in project_records()? {
        let record = parse(&fs::read(&file)?)?;
        let front = record
            .front_matter()
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let id = match front.get("id").map(|field| &field.value) {
            Some(Value::Scalar(scalar)) => scalar.text.clone(),
            _ => return Err(format!("{} has no scalar id", file.display()).into()),
        };
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        assert_eq!(id, stem, "id of {}", file.display());
        assert!(seen.insert(id.clone()), "id {id} appears twice");
    }
    Ok(())
}

#[test]
fn every_fixture_says_it_is_synthetic() -> Result<(), Box<dyn Error>> {
    for file in md_files(&corpus())? {
        let bytes = fs::read(&file)?;
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("synthetic fixture"),
            "{} does not say it is synthetic",
            file.display()
        );
    }
    Ok(())
}

#[derive(Deserialize)]
struct Twin {
    input: String,
    lf: String,
}

#[derive(Deserialize)]
struct Twins {
    twin: Vec<Twin>,
}

// AC: CE-09.4
#[test]
fn crlf_inputs_read_like_their_lf_twins() -> Result<(), Box<dyn Error>> {
    let dir = corpus().join("crlf");
    let listing = fs::read_to_string(dir.join("twins.toml"))?;
    let twins: Twins = toml::from_str(&listing)?;
    assert!(!twins.twin.is_empty(), "twins.toml lists twins");
    for twin in twins.twin {
        let input = fs::read(dir.join(&twin.input))?;
        let lf = fs::read(dir.join(&twin.lf))?;
        let has = |bytes: &[u8], byte: u8| bytes.contains(&byte);
        if twin.input == "lone-cr.md" {
            assert!(has(&input, b'\r') && !has(&input, b'\n'), "{}", twin.input);
        } else {
            assert!(input.windows(2).any(|w| w == b"\r\n"), "{}", twin.input);
        }
        let read = parse(&input)?;
        let want = parse(&lf)?;
        assert_eq!(read.text(), want.text(), "text of {}", twin.input);
        assert!(read.was_normalized(), "{} was normalized", twin.input);
        let (read_front, want_front) = (
            read.front_matter().map_err(ToString::to_string)?,
            want.front_matter().map_err(ToString::to_string)?,
        );
        assert_eq!(read_front, want_front, "front matter of {}", twin.input);
        for (a, b) in read_front.fields.iter().zip(&want_front.fields) {
            assert_eq!(
                read.index().position(a.key_span.start)?,
                want.index().position(b.key_span.start)?,
                "position of {} in {}",
                a.key,
                twin.input
            );
        }
    }
    Ok(())
}

// AC: CE-09.4
#[test]
fn crlf_writes_return_lf() -> Result<(), Box<dyn Error>> {
    let dir = corpus().join("crlf");
    let twins: Twins = toml::from_str(&fs::read_to_string(dir.join("twins.toml"))?)?;
    for twin in twins.twin {
        let record = parse(&fs::read(dir.join(&twin.input))?)?;
        let lf = parse(&fs::read(dir.join(&twin.lf))?)?;
        let front = record.front_matter().map_err(ToString::to_string)?;
        assert!(!front.fields.is_empty(), "{} has fields", twin.input);
        let ops: Vec<PatchOp> = front
            .fields
            .iter()
            .map(|field| PatchOp::Set {
                key: field.key.clone(),
                value: field.value.to_raw(),
            })
            .collect();
        let written = record.patch(&ops, &Order::default())?;
        assert!(!written.text().contains('\r'), "{} writes LF", twin.input);
        assert_eq!(written.text(), lf.text(), "{}", twin.input);
    }
    Ok(())
}
