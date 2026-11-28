//! The guides of the project (`tools/guides/*.md`) as MCP prompts.
//!
//! Each call reads the folder, so a guide written while the server runs shows up at once.

use std::sync::{Mutex, PoisonError};

use rmcp::ErrorData;
use rmcp::model::{GetPromptResult, Prompt, PromptMessage, Role};
use theos_studio::{guides, report};

use crate::state::State;

/// One prompt per guide: the name is the file stem, the description the guide's title. Without
/// a project the list is empty.
pub fn list(state: &Mutex<State>) -> Vec<Prompt> {
    let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
    let root = match state.root() {
        Ok(root) => root,
        Err(reason) => {
            tracing::warn!(%reason, "no project, so no prompts");
            return Vec::new();
        }
    };
    guides::list(&root)
        .into_iter()
        .map(|guide| Prompt::new(guide.name, Some(guide.title), None))
        .collect()
}

/// The guide `name` as one user message.
///
/// # Errors
/// `invalid_params` when the project cannot be opened, the guide is unknown or unreadable.
pub fn get(state: &Mutex<State>, name: &str) -> Result<GetPromptResult, ErrorData> {
    let root = state
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .root()
        .map_err(|reason| ErrorData::invalid_params(reason, None))?;
    let text = guides::read(&root, name)
        .map_err(|error| ErrorData::invalid_params(report(&error), None))?
        .ok_or_else(|| ErrorData::invalid_params(format!("no guide named `{name}`"), None))?;
    Ok(GetPromptResult::new(vec![PromptMessage::new_text(
        Role::User,
        text,
    )]))
}
