use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn roles(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

/// Every file directly in `dir` by name, with its bytes.
fn files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, std::io::Error> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        out.insert(name, std::fs::read(entry.path())?);
    }
    Ok(out)
}

#[test]
fn names_follow_seed_and_role() {
    assert_eq!(file_name(1, "base"), "s1-base.png");
    assert_eq!(file_name(12, "omega"), "s12-omega.png");
    assert_eq!(file_name(7, "features"), "s7-features.geojson");
    assert_eq!(file_name(7, "package"), "s7.theosmap");
}

#[test]
fn parts_are_renamed_only_at_the_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut outputs = Outputs::claim(dir.path(), 4, &roles(&["base", "package"]))?;
    outputs.write("base", b"png")?;
    outputs.write("package", b"zip")?;
    let parts = files(dir.path())?;
    assert_eq!(
        parts.keys().collect::<Vec<_>>(),
        [".s4-base.png.part", ".s4.theosmap.part"]
    );
    let names = outputs.commit()?;
    assert_eq!(names["base"], "s4-base.png");
    assert_eq!(names["package"], "s4.theosmap");
    let done = files(dir.path())?;
    assert_eq!(done.len(), 2);
    assert_eq!(done["s4-base.png"], b"png");
    assert_eq!(done["s4.theosmap"], b"zip");
    Ok(())
}

#[test]
fn an_output_that_is_dropped_before_its_commit_leaves_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("keep.txt"), b"mine")?;
    let mut outputs = Outputs::claim(dir.path(), 1, &roles(&["base", "truth"]))?;
    outputs.write("base", b"png")?;
    drop(outputs);
    let left = files(dir.path())?;
    assert_eq!(left.keys().collect::<Vec<_>>(), ["keep.txt"]);
    assert_eq!(left["keep.txt"], b"mine");
    Ok(())
}

#[test]
fn a_panic_after_a_write_leaves_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let caught = catch_unwind(AssertUnwindSafe(|| -> Result<(), RunError> {
        let mut outputs = Outputs::claim(dir.path(), 1, &roles(&["base"]))?;
        outputs.write("base", b"png")?;
        panic!("a render blew up")
    }));
    assert!(caught.is_err());
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn existing_names_and_parts_are_refused_before_anything_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("s1-truth.png"), b"old")?;
    match Outputs::claim(dir.path(), 1, &roles(&["base", "truth"])) {
        Err(RunError::OutputExists { path }) => assert_eq!(path, dir.path().join("s1-truth.png")),
        Err(other) => return Err(format!("expected OutputExists, got {other}").into()),
        Ok(_) => return Err("the clash was not found".into()),
    }
    std::fs::write(dir.path().join(".s2-base.png.part"), b"old part")?;
    match Outputs::claim(dir.path(), 2, &roles(&["base"])) {
        Err(RunError::OutputExists { path }) => {
            assert_eq!(path, dir.path().join(".s2-base.png.part"));
        }
        Err(other) => return Err(format!("expected OutputExists, got {other}").into()),
        Ok(_) => return Err("the leftover part was not found".into()),
    }
    let left = files(dir.path())?;
    assert_eq!(left.len(), 2);
    assert_eq!(left["s1-truth.png"], b"old");
    Ok(())
}

#[test]
fn a_file_that_appears_during_the_run_is_not_replaced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut outputs = Outputs::claim(dir.path(), 1, &roles(&["base", "truth"]))?;
    outputs.write("base", b"png")?;
    outputs.write("truth", b"png")?;
    std::fs::write(dir.path().join("s1-truth.png"), b"someone else's")?;
    match outputs.commit() {
        Err(RunError::OutputExists { path }) => assert_eq!(path, dir.path().join("s1-truth.png")),
        Err(other) => return Err(format!("expected OutputExists, got {other}").into()),
        Ok(_) => return Err("the late file was replaced".into()),
    }
    let left = files(dir.path())?;
    assert_eq!(left.keys().collect::<Vec<_>>(), ["s1-truth.png"]);
    assert_eq!(left["s1-truth.png"], b"someone else's");
    Ok(())
}

#[test]
fn a_role_that_was_not_claimed_or_not_written_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut outputs = Outputs::claim(dir.path(), 1, &roles(&["base", "truth"]))?;
    assert!(matches!(
        outputs.write("omega", b"png"),
        Err(RunError::Io { .. })
    ));
    outputs.write("base", b"png")?;
    match outputs.commit() {
        Err(RunError::Io { path, source }) => {
            assert_eq!(path, dir.path().join("s1-truth.png"));
            assert!(source.to_string().contains("never written"), "{source}");
        }
        Err(other) => return Err(format!("expected an I/O error, got {other}").into()),
        Ok(_) => return Err("a commit with a missing output succeeded".into()),
    }
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn a_folder_that_is_missing_or_a_file_is_an_io_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("missing");
    assert!(matches!(
        Outputs::claim(&missing, 1, &roles(&["base"])),
        Err(RunError::Io { path, .. }) if path == missing
    ));
    let file = dir.path().join("file");
    std::fs::write(&file, b"x")?;
    assert!(matches!(
        Outputs::claim(&file, 1, &roles(&["base"])),
        Err(RunError::Io { path, .. }) if path == file
    ));
    Ok(())
}
