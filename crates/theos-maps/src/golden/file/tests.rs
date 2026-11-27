use std::collections::BTreeMap;

use super::*;
use crate::golden::{GoldenRun, GoldenStage};

fn file(seed: u64) -> GoldenFile {
    GoldenFile {
        generator: "theos-maps@0.0.1".to_owned(),
        run: vec![GoldenRun {
            recipe: "fixture-small".to_owned(),
            seed,
            features: "f".to_owned(),
            package: "p".to_owned(),
            stages: vec![
                GoldenStage {
                    key: "grid#0".to_owned(),
                    layers: BTreeMap::from([("height".to_owned(), "h".to_owned())]),
                },
                GoldenStage {
                    key: "travel#0".to_owned(),
                    layers: BTreeMap::new(),
                },
            ],
        }],
    }
}

#[test]
fn a_recorded_file_reads_back_and_its_folder_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden").join("0.0.1.toml");
    record(&path, &file(1)).unwrap();
    assert_eq!(load(&path).unwrap(), file(1));
}

#[test]
fn the_file_is_toml_with_lf_line_ends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0.0.1.toml");
    record(&path, &file(1)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains('\r'), "{text:?}");
    assert!(text.ends_with('\n'), "{text:?}");
    assert!(text.contains("[[run.stage]]"), "{text}");
}

#[test]
fn a_missing_file_names_the_command_that_records_it() {
    let dir = tempfile::tempdir().unwrap();
    let error = load(&dir.path().join("1.0.0.toml")).unwrap_err();
    assert!(matches!(error, GoldenError::Missing { .. }), "{error:?}");
    assert!(
        error
            .to_string()
            .contains("run `cargo run -p theos-maps --example bless`"),
        "{error}"
    );
}

#[test]
fn a_file_that_is_not_a_golden_file_is_a_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("1.0.0.toml");
    std::fs::write(&path, "generator = 3\n").unwrap();
    let error = load(&path).unwrap_err();
    assert!(matches!(error, GoldenError::Parse { .. }), "{error:?}");
    assert!(error.to_string().contains("is not valid"), "{error}");
}

#[test]
fn a_folder_in_place_of_the_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let error = load(dir.path()).unwrap_err();
    assert!(matches!(error, GoldenError::Io { .. }), "{error:?}");
}

#[test]
fn an_existing_file_is_refused_with_the_instruction_to_raise_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0.0.1.toml");
    std::fs::write(&path, "not ours").unwrap();
    let error = record(&path, &file(1)).unwrap_err();
    assert!(matches!(error, GoldenError::Exists { .. }), "{error:?}");
    assert!(error.to_string().contains("raise `VERSION`"), "{error}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not ours");
}

#[test]
fn every_u64_seed_reads_back() {
    // TOML integers are signed 64-bit, but the `toml` crate writes and reads all of `u64`.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0.0.1.toml");
    record(&path, &file(u64::MAX)).unwrap();
    assert_eq!(load(&path).unwrap(), file(u64::MAX));
}

#[test]
fn a_folder_that_cannot_be_created_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("golden");
    std::fs::write(&blocker, "a file where the folder should be").unwrap();
    let error = record(&blocker.join("0.0.1.toml"), &file(1)).unwrap_err();
    assert!(matches!(error, GoldenError::Io { .. }), "{error:?}");
    assert!(error.to_string().contains("golden"), "{error}");
}
