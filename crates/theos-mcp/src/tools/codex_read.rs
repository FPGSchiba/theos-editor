//! `codex_read`: one record with its text, links, lens and ledger.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::codex::{self, ReadTarget};
use theos_studio::views::codex::ReadView;

use super::run;
use crate::server::Server;

/// The parameters of `codex_read`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The record id. Give this or `path`.
    id: Option<String>,
    /// The project-relative path of the record. Give this or `id`.
    path: Option<String>,
}

impl Params {
    /// What to read; exactly one of `id` and `path` must be given.
    fn target(self) -> Result<ReadTarget, String> {
        match (self.id, self.path) {
            (Some(id), None) => Ok(ReadTarget::Id(id)),
            (None, Some(path)) => Ok(ReadTarget::Path(path)),
            _ => Err("give exactly one of `id` and `path`".to_owned()),
        }
    }
}

#[tool_router(router = codex_read_router, vis = "pub(super)")]
impl Server {
    /// One record in full.
    #[tool(
        name = "codex_read",
        description = concat!("Read one record by id or path: its full text, links, backlinks, children, findings, effective lens with beliefs and, for a truth, its reveal ledger. ", marks_note!()),
        annotations(read_only_hint = true)
    )]
    fn codex_read(&self, Parameters(params): Parameters<Params>) -> Result<Json<ReadView>, String> {
        let target = params.target()?;
        run(self, |ws| codex::read(ws, &target))
    }
}
