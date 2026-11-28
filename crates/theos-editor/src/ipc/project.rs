//! Opening a project, the last project, and syncing the open one with its folder.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;
use theos_content::Severity;
use theos_studio::Workspace;
use theos_studio::parts;

use super::error::IpcError;
use super::sync_files::{self, FileMove, Files, RecordFile};
use super::{logged, saturate, timed};
use crate::app_state::{AppState, lock};
use crate::state::EditorState;

/// One button of the rail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct RailEntry {
    /// The part the button opens.
    pub id: String,
    /// The label.
    pub label: String,
    /// The glyph.
    pub glyph: String,
    /// Position on the rail, ascending.
    pub order: u32,
    /// The phase that builds the part's view, or `null` when it is built.
    pub phase: Option<String>,
}

impl From<parts::RailEntry> for RailEntry {
    fn from(entry: parts::RailEntry) -> RailEntry {
        RailEntry {
            id: entry.id.to_owned(),
            label: entry.label.to_owned(),
            glyph: entry.glyph.to_owned(),
            order: u32::from(entry.order),
            phase: entry.phase.map(str::to_owned),
        }
    }
}

/// The open project as the shell shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct ProjectView {
    /// `[project] name`, else the name of the folder.
    pub name: String,
    /// The project folder.
    pub root: String,
    /// `[project] kind`, when it is set.
    pub kind: Option<String>,
    /// The rail: the enabled parts in rail order, then Code and Settings.
    pub rail: Vec<RailEntry>,
    /// How many records the index holds.
    pub records: u32,
    /// How many findings are errors.
    pub errors: u32,
    /// How many findings are warnings.
    pub warnings: u32,
    /// Whether the index lives in memory, because another program holds the cache.
    pub in_memory: bool,
}

impl ProjectView {
    /// The view of `workspace`.
    ///
    /// # Errors
    ///
    /// [`IpcError::Index`] when the index cannot be read.
    pub fn of(workspace: &Workspace) -> Result<ProjectView, IpcError> {
        let project = workspace.project();
        let index = workspace.index();
        let records = index.records()?.len();
        let findings = index.findings()?;
        let count = |severity| findings.iter().filter(|f| f.severity == severity).count();
        Ok(ProjectView {
            name: project
                .name()
                .map_or_else(|| folder_name(project.root()), str::to_owned),
            root: project.root().display().to_string(),
            kind: project.kind().map(str::to_owned),
            rail: parts::rail(project.enabled_parts())
                .into_iter()
                .map(RailEntry::from)
                .collect(),
            records: saturate(records),
            errors: saturate(count(Severity::Error)),
            warnings: saturate(count(Severity::Warning)),
            in_memory: workspace.is_in_memory(),
        })
    }
}

/// What a sync did: the project afterwards and the record files whose views changed, are gone or
/// moved. Files are named by path, since two files may carry one id; see
/// [`sync_files`](super::sync_files) for the rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct SyncView {
    /// The project after the sync.
    pub project: ProjectView,
    /// The files read again, relinked or checked again, sorted by path; a moved file's new path
    /// is among them.
    pub changed: Vec<RecordFile>,
    /// The files that are gone and did not move, sorted by path.
    pub removed: Vec<RecordFile>,
    /// The files that moved with their id, sorted by id, then old path.
    pub moved: Vec<FileMove>,
}

/// Opens the project in `path` and syncs its index. The open project stays open when this one
/// does not open.
///
/// # Errors
///
/// [`IpcError::NotAProject`] when `path` has no `theos.toml` or it is not valid, else as the
/// index fails.
#[tauri::command]
#[specta::specta]
pub async fn project_open(
    state: State<'_, AppState>,
    path: String,
) -> Result<ProjectView, IpcError> {
    let started = Instant::now();
    logged("project_open", started, open(&state, Path::new(&path)))
}

fn open(state: &AppState, path: &Path) -> Result<ProjectView, IpcError> {
    let root = std::path::absolute(path).map_err(|error| IpcError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    let workspace = Workspace::open(&root).map_err(|error| IpcError::opening(&root, error))?;
    let view = ProjectView::of(&workspace)?;
    for part in parts::unknown(workspace.project().enabled_parts()) {
        tracing::warn!(part, "unknown editor part in `enabled`; skipped");
    }
    tracing::info!(
        project = %view.root,
        records = view.records,
        errors = view.errors,
        warnings = view.warnings,
        in_memory = view.in_memory,
        "project opened"
    );
    *lock(&state.workspace) = Some(workspace);
    // Opening synced the index; the first sync after it still reads the folder, so a focus event
    // right after a folder dialog never hides a change.
    *lock(&state.last_sync) = None;
    remember(state, root);
    Ok(view)
}

/// Writes `root` as the last project. A failure only costs the user a step next time, so it is
/// logged and the command goes on.
fn remember(state: &AppState, root: PathBuf) {
    let mut saved = state.state_file.load();
    saved.last_project = Some(root);
    if let Err(error) = state.state_file.save(&saved) {
        tracing::warn!(
            file = ?state.state_file.path(),
            %error,
            "cannot write the state file"
        );
    }
}

/// The folder of the project opened last, or `null`.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State and AppHandle by value"
)]
#[must_use]
pub fn project_last(state: State<'_, AppState>) -> Option<String> {
    timed("project_last", || {
        let EditorState { last_project } = state.state_file.load();
        last_project.map(|path| path.display().to_string())
    })
}

/// The folder named on the command line (`theos-editor <path>`), or `null`. The UI opens it at
/// start, before the last project (spec §5.9).
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State and AppHandle by value"
)]
#[must_use]
pub fn project_argument(state: State<'_, AppState>) -> Option<String> {
    timed("project_argument", || {
        state
            .argument
            .as_ref()
            .map(|path| path.display().to_string())
    })
}

/// Why the UI syncs: the window got the focus back, or the user asked (Reload, Ctrl+R).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    /// The window got the focus back; held back within [`SYNC_INTERVAL`] of the last sync.
    Focus,
    /// The user asked; always syncs.
    Reload,
}

/// How long after a sync a focus sync answers the open view without reading the folder. The
/// window syncs each time it gets the focus back (spec §5.9), and focus can come and go fast.
pub const SYNC_INTERVAL: Duration = Duration::from_secs(1);

/// Syncs the open project's index with its folder. A [`SyncReason::Focus`] sync within
/// [`SYNC_INTERVAL`] of the last sync answers the open project's view with no changed or removed
/// records, which the sync before already reported; a [`SyncReason::Reload`] always syncs.
///
/// # Errors
///
/// [`IpcError::NoProject`] without an open project, else as the index fails.
#[tauri::command]
#[specta::specta]
pub async fn project_sync(
    state: State<'_, AppState>,
    reason: SyncReason,
) -> Result<SyncView, IpcError> {
    let started = Instant::now();
    logged("project_sync", started, sync(&state, reason))
}

fn sync(state: &AppState, reason: SyncReason) -> Result<SyncView, IpcError> {
    let mut guard = lock(&state.workspace);
    let workspace = guard.as_mut().ok_or(IpcError::NoProject)?;
    let mut last_sync = lock(&state.last_sync);
    let recent = last_sync.is_some_and(|at| at.elapsed() < SYNC_INTERVAL);
    if reason == SyncReason::Focus && recent {
        tracing::debug!("synced less than a second ago; the open view answers");
        return Ok(SyncView {
            project: ProjectView::of(workspace)?,
            changed: Vec::new(),
            removed: Vec::new(),
            moved: Vec::new(),
        });
    }
    let before = files(workspace)?;
    let update = workspace.write(theos_content::Index::sync)?;
    *last_sync = Some(Instant::now());
    let after = files(workspace)?;
    let sync_files::FileChanges {
        changed,
        removed,
        moved,
    } = sync_files::changes(
        &before,
        &after,
        update
            .reparsed
            .iter()
            .chain(&update.relinked)
            .chain(&update.revalidated),
    );
    let project = ProjectView::of(workspace)?;
    tracing::info!(
        project = %project.root,
        changed = changed.len(),
        removed = removed.len(),
        moved = moved.len(),
        records = project.records,
        errors = project.errors,
        warnings = project.warnings,
        "project synced"
    );
    Ok(SyncView {
        project,
        changed,
        removed,
        moved,
    })
}

/// The index's record files of `workspace`: id by path.
fn files(workspace: &Workspace) -> Result<Files, IpcError> {
    Ok(workspace
        .index()
        .records()?
        .into_iter()
        .map(|record| (record.path, record.id))
        .collect())
}

fn folder_name(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_without_a_name_takes_its_folder_name() {
        assert_eq!(folder_name(Path::new("synthetic/Projekt Ä")), "Projekt Ä");
        // A root folder has no name of its own: the path stands for it.
        assert_eq!(folder_name(Path::new("/")), "/");
    }
}
