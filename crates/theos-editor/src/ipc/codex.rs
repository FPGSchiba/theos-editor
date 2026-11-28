//! The Codex commands: the tree tabs, the record page and the find field. Each reads the open
//! project as the last sync left it; `project_sync` brings it up to date.

use std::time::Instant;

use tauri::State;
use theos_studio::codex_view;
use theos_studio::{StudioError, Workspace};

use super::error::IpcError;
use super::views::{QueryResult, RecordPage, TreeNode, TreeTab, TreeTabInfo};
use super::{logged, timed};
use crate::app_state::{AppState, lock};

/// Runs `read` on the open project.
fn with_workspace<T>(
    state: &AppState,
    read: impl FnOnce(&Workspace) -> Result<T, IpcError>,
) -> Result<T, IpcError> {
    let guard = lock(&state.workspace);
    read(guard.as_ref().ok_or(IpcError::NoProject)?)
}

/// The tabs of the Codex tree: the kinds of each, their folder, the kinds shown id first and the
/// kind at the top. It is the table `codex_tree` builds its rows from.
#[tauri::command]
#[specta::specta]
#[must_use]
pub fn codex_tabs() -> Vec<TreeTabInfo> {
    timed("codex_tabs", || {
        codex_view::TABS.iter().map(TreeTabInfo::from).collect()
    })
}

/// The rows of a tab of the Codex tree.
///
/// # Errors
///
/// [`IpcError::NoProject`] without an open project, else as the index fails.
#[tauri::command]
#[specta::specta]
pub async fn codex_tree(
    state: State<'_, AppState>,
    tab: TreeTab,
) -> Result<Vec<TreeNode>, IpcError> {
    let started = Instant::now();
    let result = with_workspace(&state, |ws| {
        Ok(codex_view::tree(ws, tab.into())?
            .into_iter()
            .map(TreeNode::from)
            .collect())
    });
    logged("codex_tree", started, result)
}

/// The page of the record `id`: header, body, lore, pictures, front matter and backlinks.
///
/// # Errors
///
/// [`IpcError::NoProject`] without an open project, [`IpcError::NotFound`] when no record has
/// the id, else as the file or the index fails.
#[tauri::command]
#[specta::specta]
pub async fn record_view(state: State<'_, AppState>, id: String) -> Result<RecordPage, IpcError> {
    let started = Instant::now();
    let result = with_workspace(&state, |ws| match codex_view::record(ws, &id) {
        Ok(page) => Ok(RecordPage::from(page)),
        Err(StudioError::UnknownRecord { .. }) => Err(IpcError::NotFound { id: id.clone() }),
        Err(error) => Err(error.into()),
    });
    logged("record_view", started, result)
}

/// The records the find field's `query` finds, at most `limit`.
///
/// # Errors
///
/// [`IpcError::NoProject`] without an open project, else as the index fails.
#[tauri::command]
#[specta::specta]
pub async fn codex_search(
    state: State<'_, AppState>,
    query: String,
    limit: u32,
) -> Result<QueryResult, IpcError> {
    let started = Instant::now();
    let result = with_workspace(
        &state,
        |ws| Ok(codex_view::query(ws, &query, limit)?.into()),
    );
    logged("codex_search", started, result)
}
