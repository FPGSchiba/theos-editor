//! The log of a command (spec §5.11, P04 ruling L21): the line of its reindex names the command.

use std::io;
use std::sync::{Arc, Mutex};

use crate::common::{open, set};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TRUTH: &str = "lore/truths/T-960.md";

/// A log writer that keeps what it is given.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    /// What the log has written so far.
    fn text(&self) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.0.lock().map_err(|_| "the log buffer is poisoned")?;
        Ok(String::from_utf8(bytes.clone())?)
    }
}

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes = self
            .0
            .lock()
            .map_err(|_| io::Error::other("the log buffer is poisoned"))?;
        bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_reindex_line_names_its_command() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || workspace.run(&command))?;

    let log = captured.text()?;
    let line = log
        .lines()
        .find(|line| line.contains("reindexed"))
        .ok_or_else(|| format!("no reindex line in:\n{log}"))?;
    assert!(line.contains(&format!("patch {TRUTH}")), "{line}");
    assert!(line.contains("revalidated="), "{line}");
    Ok(())
}
