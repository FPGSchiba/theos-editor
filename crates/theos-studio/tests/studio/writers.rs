//! Two processes append to the same record through the command layer: the repo lock and the base
//! check together never lose a line, and a reader never sees a file half-written.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use theos_content::Record;
use theos_studio::Workspace;

use crate::common::{TempProject, spawn_helper};
use crate::helper::{HelperTask, WRITER_WAIT, append_lines};

type Error = Box<dyn std::error::Error>;

const SCRIPT: &str = "story/scripts/S-960.md";
const LINES: usize = 20;

// AC: CE-22.2
#[test]
fn two_processes_append_without_losing_a_line() -> Result<(), Error> {
    let project = TempProject::copy_of_studio("commands")?;
    let original = std::fs::read_to_string(project.root().join(SCRIPT))?;
    let ready = project.root().join("helper.ready");
    let mut helper = spawn_helper(&HelperTask::Append {
        root: project.root().to_path_buf(),
        path: SCRIPT.to_owned(),
        tag: "helper".to_owned(),
        count: LINES,
        ready: ready.clone(),
    })?;
    helper.wait_for(&ready)?;

    let reader = Reader::start(project.root().join(SCRIPT), original.clone());
    let mut workspace = Workspace::open(project.root())?;
    workspace.set_lock_wait(WRITER_WAIT);
    append_lines(&mut workspace, SCRIPT, "test", LINES)?;
    let status = helper.wait()?;
    assert!(status.success(), "the helper exited with {status}");
    // Every intermediate state the reader saw was a whole record.
    let checked = reader.finish()?;
    assert!(checked > 0, "the reader read nothing");

    let text = std::fs::read_to_string(project.root().join(SCRIPT))?;
    whole(&original, &text)?;
    for tag in ["helper", "test"] {
        for n in 1..=LINES {
            let line = format!("{tag} {n}");
            let count = text.lines().filter(|l| *l == line).count();
            assert_eq!(count, 1, "the line {line:?} is in the record {count} times");
        }
    }
    let mut leftovers = Vec::new();
    find_tmp(project.root(), &mut leftovers)?;
    assert!(
        leftovers.is_empty(),
        "temporary files are left: {leftovers:?}"
    );
    Ok(())
}

#[test]
fn a_torn_text_is_not_whole() {
    let original = "---\nid: S-1\nkind: script\n---\nBody.\n";
    let good = format!("{original}helper 1\ntest 1\n");
    assert_eq!(whole(original, &good), Ok(()));
    let unwritten = good.replace("helper 1", "helper 0");
    for torn in [
        &original[..original.len() - 3],
        &good[..good.len() - 1],
        &good[..good.len() - 3],
        unwritten.as_str(),
        "",
    ] {
        assert!(whole(original, torn).is_err(), "{torn:?}");
    }
}

/// A thread that reads the record over and over while the writers run, and checks that each read
/// is whole.
struct Reader {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<usize, String>>>,
}

impl Reader {
    /// Starts reading the record at `path`, whose text was `original` before the writers began.
    fn start(path: PathBuf, original: String) -> Reader {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut reads = 0;
            while !stopped.load(Ordering::Relaxed) {
                // A read can race the rename that replaces the file (sharing on Windows): that is
                // no state of the file, so it is read again. Text that is read is never let pass
                // unchecked.
                if let Ok(bytes) = std::fs::read(&path) {
                    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
                    whole(&original, &text)?;
                    reads += 1;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Ok(reads)
        });
        Reader {
            stop,
            thread: Some(thread),
        }
    }

    /// Stops the reader; how many whole reads it made, or the first read that was not whole.
    fn finish(mut self) -> Result<usize, Error> {
        self.stop.store(true, Ordering::Relaxed);
        let thread = self
            .thread
            .take()
            .ok_or("the reader has finished already")?;
        let reads = thread.join().map_err(|_| "the reader panicked")??;
        Ok(reads)
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Whether `text` is a whole state of the record that held `original`: it parses as a record, it
/// starts with `original`, and every line after that is a whole `helper <n>` or `test <n>`.
fn whole(original: &str, text: &str) -> Result<(), String> {
    Record::parse(text).map_err(|error| format!("not a record ({error}):\n{text}"))?;
    let added = text
        .strip_prefix(original)
        .ok_or_else(|| format!("the original text is not whole:\n{text}"))?;
    if !added.is_empty() && !added.ends_with('\n') {
        return Err(format!("the last line is torn:\n{text}"));
    }
    for line in added.lines() {
        let written = line.split_once(' ').is_some_and(|(tag, n)| {
            matches!(tag, "helper" | "test")
                && n.parse::<usize>().is_ok_and(|n| (1..=LINES).contains(&n))
        });
        if !written {
            return Err(format!("{line:?} is no line a writer wrote:\n{text}"));
        }
    }
    Ok(())
}

/// Collects every `.theos-*.tmp` file under `dir`.
fn find_tmp(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            find_tmp(&path, found)?;
        } else {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".theos-")
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp"))
            {
                found.push(path);
            }
        }
    }
    Ok(())
}
