//! `codex_graph`: the records around one record and the links between them.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::codex::{self, Direction};
use theos_studio::views::codex::GraphView;

use super::run;
use crate::server::Server;

/// The parameters of `codex_graph`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// The record id to start from.
    id: String,
    /// How many link steps to walk, 1 to 3; default 1.
    depth: Option<u8>,
    /// Which links to follow; default `both`.
    direction: Option<Direction>,
}

#[tool_router(router = codex_graph_router, vis = "pub(super)")]
impl Server {
    /// The link graph around a record.
    #[tool(
        name = "codex_graph",
        input_schema = super::input_schema::<Params>(),
        description = concat!("Return the records within a few links of one record, by distance, and the links between them. ", marks_note!(), " ", read_only_note!()),
        annotations(read_only_hint = true)
    )]
    fn codex_graph(
        &self,
        Parameters(params): Parameters<Params>,
    ) -> Result<Json<GraphView>, String> {
        let depth = params.depth.unwrap_or(1);
        let direction = params.direction.unwrap_or(Direction::Both);
        run(self, |ws| codex::graph(ws, &params.id, depth, direction))
    }
}
