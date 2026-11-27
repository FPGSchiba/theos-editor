//! `ledger_check`: the reveal ledger of the truths and the findings of the reveal rules.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use theos_studio::codex;
use theos_studio::views::project::LedgerCheckView;

use super::run;
use crate::server::Server;

/// The parameters of `ledger_check`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// Check only this truth; default every truth.
    id: Option<String>,
}

#[tool_router(router = ledger_check_router, vis = "pub(super)")]
impl Server {
    /// The ledger of the truths with their clues, and the `lore/` findings on them.
    #[tool(
        name = "ledger_check",
        description = concat!("Return the reveal ledger (each truth with the clues that reveal it) and the findings of the lore rules on those records. ", marks_note!()),
        annotations(read_only_hint = true)
    )]
    fn ledger_check(
        &self,
        Parameters(params): Parameters<Params>,
    ) -> Result<Json<LedgerCheckView>, String> {
        run(self, |ws| codex::ledger_check(ws, params.id.as_deref()))
    }
}
