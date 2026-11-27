//! The JSON views of `project_info` and `ledger_check`.

use schemars::JsonSchema;
use serde::Serialize;

use crate::guides::Guide;
use crate::marks::MarkView;
use crate::views::check::FindingView;
use crate::views::ledger::TruthView;

/// Where the index cache lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CacheKind {
    /// In `.theos/cache`.
    File,
    /// In memory, because another process holds the cache.
    Memory,
}

/// A guide of the content project, as JSON.
pub type GuideView = Guide;

/// How many records a kind has.
#[derive(Debug, Serialize, JsonSchema)]
pub struct KindCountView {
    /// The kind name.
    pub name: String,
    /// The editor part that registered it.
    pub part: String,
    /// The records of the kind.
    pub records: usize,
}

/// The effective severity of a rule.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RuleSeverityView {
    /// The rule id.
    pub id: String,
    /// The severity the project's settings give the rule, else its default.
    pub severity: String,
}

/// What `project_info` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ProjectInfoView {
    /// The project folder.
    pub root: String,
    /// The schema version this build speaks.
    pub schema_version: u32,
    /// Where the index cache lives.
    pub cache: CacheKind,
    /// The records of the project.
    pub records: usize,
    /// The registered kinds with their record counts.
    pub kinds: Vec<KindCountView>,
    /// Every rule with its effective severity.
    pub rules: Vec<RuleSeverityView>,
    /// The errors among the findings.
    pub errors: usize,
    /// The warnings among the findings.
    pub warnings: usize,
    /// The guides of `tools/guides`.
    pub guides: Vec<GuideView>,
    /// The tools of the server that answers.
    pub tools: Vec<String>,
}

/// A truth with its marks.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MarkedTruthView {
    /// The truth and the clues that reveal it.
    #[serde(flatten)]
    pub truth: TruthView,
    /// The marks of the truth.
    pub mark: MarkView,
}

/// What `ledger_check` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LedgerCheckView {
    /// The truths, with their marks.
    pub truths: Vec<MarkedTruthView>,
    /// The `lore/` findings on the truths and their clues.
    pub findings: Vec<FindingView>,
    /// The errors among them.
    pub errors: usize,
    /// The warnings among them.
    pub warnings: usize,
}
