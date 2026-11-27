//! `project_info`: the folder, schema, cache, counts, rules and guides in one answer.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::project;
use theos_studio::views::project::ProjectInfoView;

use super::{names, run};
use crate::server::Server;

/// The parameters of `project_info`: none, and an argument is an error.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {}

#[tool_router(router = project_info_router, vis = "pub(super)")]
impl Server {
    /// What the project is: folder, schema version, cache, records per kind, rules, findings.
    #[tool(
        name = "project_info",
        input_schema = super::input_schema::<Params>(),
        description = concat!("Describe the open Theos project: its folder, schema version, index cache, record counts per kind, rule severities, finding counts and guides. Start here. ", read_only_note!()),
        annotations(read_only_hint = true)
    )]
    fn project_info(
        &self,
        Parameters(Params {}): Parameters<Params>,
    ) -> Result<Json<ProjectInfoView>, String> {
        let tools = names();
        let tools: Vec<&str> = tools.iter().map(String::as_str).collect();
        run(self, |ws| project::info(ws, &tools))
    }
}
