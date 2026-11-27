//! The tools: one module each, registered in [`router`], and the shared [`run`] path.

/// The sentence every record-returning tool ends its description with. A macro, because
/// `concat!` takes literals and not constants.
macro_rules! marks_note {
    () => {
        "Records carry `never` (a truth Claude must never confirm) and `locked` (reasons it cannot be changed over MCP)."
    };
}

mod codex_graph;
mod codex_list;
mod codex_read;
mod codex_search;
mod ledger_check;
mod project_info;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Json;
use serde::Serialize;
use theos_studio::{StudioError, Workspace};

use crate::server::Server;

/// Every tool. A new tool is one module and one `+` here.
pub fn router() -> ToolRouter<Server> {
    Server::project_info_router()
        + Server::codex_search_router()
        + Server::codex_read_router()
        + Server::codex_list_router()
        + Server::codex_graph_router()
        + Server::ledger_check_router()
}

/// The names of all tools.
pub fn names() -> Vec<String> {
    router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect()
}

/// The path every tool takes: lock the state, sync the index, run `f`, report an error as
/// text. The lock is never held across an `.await`; a tool is synchronous.
///
/// # Errors
/// The `report` of the error; rmcp turns an `Err(String)` into a result with `isError: true`.
pub fn run<T: Serialize>(
    server: &Server,
    f: impl FnOnce(&mut Workspace) -> Result<T, StudioError>,
) -> Result<Json<T>, String> {
    let mut state = server
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.with(f).map(Json)
}
