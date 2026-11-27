//! `project_info`: the folder, schema, cache, counts, rules and guides in one answer.

use rmcp::handler::server::wrapper::Json;
use rmcp::{tool, tool_router};
use theos_studio::project;
use theos_studio::views::project::ProjectInfoView;

use super::{names, run};
use crate::server::Server;

#[tool_router(router = project_info_router, vis = "pub(super)")]
impl Server {
    /// What the project is: folder, schema version, cache, records per kind, rules, findings.
    #[tool(
        name = "project_info",
        description = "Describe the open Theos project: its folder, schema version, index cache, record counts per kind, rule severities, finding counts and guides. Start here.",
        annotations(read_only_hint = true)
    )]
    fn project_info(&self) -> Result<Json<ProjectInfoView>, String> {
        let tools = names();
        let tools: Vec<&str> = tools.iter().map(String::as_str).collect();
        run(self, |ws| project::info(ws, &tools))
    }
}
