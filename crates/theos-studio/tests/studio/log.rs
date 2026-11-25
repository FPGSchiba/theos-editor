//! The log of a command (spec §5.11, P04 ruling L21): the line of its reindex names the command,
//! and every line of the run carries its label, under any level filter.

use std::io;
use std::sync::{Arc, Mutex};

use tracing::Level;

use theos_studio::{Applied, CommandError, EntryId, Route, Workspace};

use crate::common::{Error, TRUTH, TestResult, open, set};

/// A log writer that keeps what it is given.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    /// What the log has written so far.
    fn text(&self) -> Result<String, Error> {
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

/// Runs `f` under a subscriber that lets the lines up to `level` through; what `f` returned and
/// the log it wrote.
fn logged<T>(level: Level, f: impl FnOnce() -> T) -> Result<(T, String), Error> {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    Ok((value, captured.text()?))
}

#[test]
fn the_reindex_line_names_its_command() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let (outcome, log) = logged(Level::DEBUG, || workspace.run(&command))?;
    outcome?;

    let line = log
        .lines()
        .find(|line| line.contains("reindexed"))
        .ok_or_else(|| format!("no reindex line in:\n{log}"))?;
    assert!(line.contains(&format!("patch {TRUTH}")), "{line}");
    assert!(line.contains("revalidated="), "{line}");
    Ok(())
}

/// The line every [`LeveledRoute`] logs.
const INSIDE: &str = "a line inside the run";

/// A route that logs one line at its level inside the run, as theos-content logs errors and the
/// rollback and the retries log warnings.
struct LeveledRoute(Level);

impl Route for LeveledRoute {
    fn holds_lock(&self) -> bool {
        false
    }

    fn finish(
        &mut self,
        _workspace: &mut Workspace,
        _applied: Applied,
    ) -> Result<Option<EntryId>, CommandError> {
        match self.0 {
            Level::ERROR => tracing::error!("{INSIDE}"),
            Level::WARN => tracing::warn!("{INSIDE}"),
            Level::INFO => tracing::info!("{INSIDE}"),
            Level::DEBUG => tracing::debug!("{INSIDE}"),
            _ => tracing::trace!("{INSIDE}"),
        }
        Ok(None)
    }
}

/// Ruling R14: the span `command` is on under every level filter, so a line of the run that the
/// filter lets through carries the label of its command, an error under an ERROR filter too.
#[test]
fn a_line_of_the_run_carries_the_command_label_under_every_level_filter() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    for level in [
        Level::ERROR,
        Level::WARN,
        Level::INFO,
        Level::DEBUG,
        Level::TRACE,
    ] {
        let title = format!("Synthetic truth, retitled under {level}");
        let command = set(&workspace, TRUTH, "title", &title)?;
        let label = format!("command{{label=patch {TRUTH}}}");
        let (outcome, log) = logged(level, || {
            workspace.run_with(&command, &mut LeveledRoute(level))
        })?;
        outcome?;

        let line = log
            .lines()
            .find(|line| line.contains(INSIDE))
            .ok_or_else(|| format!("no line under {level} in:\n{log}"))?;
        assert!(line.contains(&label), "{level}: {line}");
    }
    Ok(())
}
