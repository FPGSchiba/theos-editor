//! `codex_search`: the records that match a query.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::codex;
use theos_studio::views::codex::SearchView;

use super::run;
use crate::server::Server;

/// The parameters of `codex_search`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The words to look for in titles, bodies and fields.
    query: String,
    /// The most hits to return, 1 to 100; default 20.
    limit: Option<usize>,
}

#[tool_router(router = codex_search_router, vis = "pub(super)")]
impl Server {
    /// The records that match words, best first.
    #[tool(
        name = "codex_search",
        description = concat!("Search the records of the Theos project for words and return the best hits with where they match. ", marks_note!()),
        annotations(read_only_hint = true)
    )]
    fn codex_search(
        &self,
        Parameters(params): Parameters<Params>,
    ) -> Result<Json<SearchView>, String> {
        run(self, |ws| {
            codex::search(ws, &params.query, params.limit.unwrap_or(20))
        })
    }
}
