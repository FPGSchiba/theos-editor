//! `codex_list`: the records of the project, filtered and paged.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::codex::{self, ListFilter};
use theos_studio::views::codex::ListView;

use super::run;
use crate::server::Server;

/// The parameters of `codex_list`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// Only records of this kind.
    kind: Option<String>,
    /// Only records with this status, `draft` or `canon`.
    status: Option<String>,
    /// Only records under this project-relative folder, written with `/`.
    folder: Option<String>,
    /// Records to skip; default 0.
    offset: Option<usize>,
    /// Records per page, 1 to 500; default 100.
    limit: Option<usize>,
}

#[tool_router(router = codex_list_router, vis = "pub(super)")]
impl Server {
    /// The records that match every given filter, ordered by path.
    #[tool(
        name = "codex_list",
        input_schema = super::input_schema::<Params>(),
        description = concat!("List the records of the Theos project that match all given filters (kind, status, folder), ordered by path, one page at a time. ", marks_note!()),
        annotations(read_only_hint = true)
    )]
    fn codex_list(&self, Parameters(params): Parameters<Params>) -> Result<Json<ListView>, String> {
        let filter = ListFilter {
            kind: params.kind,
            status: params.status,
            folder: params.folder,
            offset: params.offset.unwrap_or(0),
            limit: params.limit.unwrap_or(100),
        };
        run(self, |ws| codex::list(ws, &filter))
    }
}
