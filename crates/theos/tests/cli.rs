//! Behaviour of the built `theos` binary.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../theos-content/tests/fixtures/projects")
        .join(name)
}

/// Copies a fixture project to a fresh temp folder; the index writes `.theos/cache` there.
fn copy_of(name: &str) -> io::Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    copy_dir(&fixture(name), temp.path())?;
    Ok(temp)
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn theos(command: &str, folder: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_theos"))
        .arg(command)
        .arg(folder)
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

// AC: CE-15.3
#[test]
fn theos_ledger_prints_the_ledger() -> io::Result<()> {
    let project = copy_of("lore")?;
    let output = theos("ledger", project.path())?;
    assert!(output.status.success(), "{output:?}");
    let expected = std::fs::read_to_string(fixture("lore").join("expected-ledger.txt"))?;
    assert_eq!(stdout(&output), expected);
    Ok(())
}

#[test]
fn theos_formalism_writes_the_mirror_once() -> io::Result<()> {
    let project = copy_of("formalism")?;
    let first = theos("formalism", project.path())?;
    assert!(first.status.success(), "{first:?}");
    let first = stdout(&first);
    assert_eq!(first.lines().count(), 18, "{first}");
    let second = theos("formalism", project.path())?;
    assert!(second.status.success(), "{second:?}");
    assert_eq!(stdout(&second), "nothing to write\n");
    Ok(())
}

#[test]
fn a_folder_without_theos_toml_exits_with_1() -> io::Result<()> {
    let empty = tempfile::tempdir()?;
    for command in ["ledger", "formalism"] {
        let output = theos(command, empty.path())?;
        assert_eq!(output.status.code(), Some(1), "{command}: {output:?}");
        assert_eq!(output.stdout, Vec::<u8>::new(), "{command}");
        assert_ne!(output.stderr, Vec::<u8>::new(), "{command}");
    }
    Ok(())
}
