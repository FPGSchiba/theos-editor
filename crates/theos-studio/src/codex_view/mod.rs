//! The Codex-only views of the editor's Codex screen (P11 §5.8, §5.10), built on the shared
//! [`codex`](crate::codex) queries: the tree tabs ([`tree()`]), the record page ([`record()`])
//! and the search prefixes ([`query()`]). Each takes the workspace as it is; the caller syncs the
//! index first. The editor maps these plain types to its IPC types.
//!
//! One module per job: [`mod@tree`] builds the tabs, [`mod@record`] assembles the page from
//! [`body`] (the block tree with resolved links), [`front_matter`] (the rows in file order),
//! [`lore`] (the lore table), [`pictures`] and [`mod@relation`] (the "Used by" relations),
//! [`mod@query`] parses the prefixes, and [`resolve`] turns an id into the record it names.

pub mod body;
pub mod front_matter;
pub mod lore;
pub mod pictures;
pub mod query;
pub mod record;
pub mod relation;
pub mod resolve;
pub mod tree;

pub use body::{BeatRow, ViewInline, ViewNode};
pub use front_matter::{FieldRow, FieldValue};
pub use lore::LoreRow;
pub use pictures::PictureRow;
pub use query::{QueryView, query};
pub use record::{Header, RecordPage, record};
pub use relation::{UsedBy, relation};
pub use resolve::ViewLink;
pub use tree::{TABS, TabShape, TabTable, TreeNode, TreeTab, table, tree};
