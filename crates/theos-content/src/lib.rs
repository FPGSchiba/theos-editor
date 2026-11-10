//! Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry, index,
//! search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no UI.
//!
//! After P02 the crate provides:
//!
//! - [`text`]: byte spans, line and column positions, UTF-16 offsets, line-ending normalisation
//!   and span-based text edits;
//! - [`record`]: reading a record and splitting it into front matter and body;
//! - [`yaml`]: the restricted YAML of the data model (one rule per forbidden form, each reporting
//!   its line and column), the front-matter model with spans, and the patcher that sets, inserts
//!   and removes fields, list items and map entries as span edits;
//! - [`body`]: blocks, wikilinks, callouts (voice, design, note), design notes, beats and
//!   malformed constructs.
//!
//! After P03 it also provides:
//!
//! - [`project`]: [`Project`] reads `theos.toml`; [`SCHEMA_VERSION`];
//! - [`finding`]: [`Finding`] and [`Severity`], the one shape of every problem found;
//! - [`kinds`]: the [`KindRegistry`] of [`KindSpec`]s (declarative field tables, id rules,
//!   [`Placement`], body grammars) and the generic checker behind [`FieldSpec`];
//! - [`parts`]: the 20 builtin kinds, one module per kind and one `register` per part (codex,
//!   scripts, concepts, maps); [`parts::builtin`] returns the registry;
//! - [`check`]: the checks of one record ([`check::check_record`]) and their list
//!   ([`check::rules`]);
//! - [`walk`]: the walk of a project's record files ([`walk::walk`], [`is_record_path`]);
//! - [`index`]: the persistent index. [`Index::open`] opens or builds the cache at
//!   `.theos/cache/index.sqlite` and syncs it with the files. `sync`, `update(paths)`,
//!   `apply(changes)` and `preview(changes)` keep it current, from disk or from staged texts.
//!   Queries give findings, records, links, backlinks and children; `search` ranks matches
//!   with SQLite FTS5; `lens` gives the effective lens of a holder with its layers; `dump`
//!   writes the whole state as text.
//!
//! The cache is valid for one identity: the layout format, the crate version, the project
//! `schema_version`, the registry, the rule set and a fingerprint of the crate sources. When any
//! of them differs, the cache is rebuilt; when nothing changed, nothing is parsed again. Every
//! write is one immediate transaction.
//!
//! Every span is a byte range into the LF text of the whole record; a CRLF record is read as its
//! LF text and a write returns LF.
//!
//! Later phases use it so: P03 (index, registry) the record model, field and entry spans, the
//! order injection of the patcher and the body links; P04 (validators) the malformed items,
//! design notes, voice spans and note states, and from P03 the findings, the rule list, the
//! relink step and the same store for the ledger; P05 the positions of errors, `Project::open`,
//! `findings`, `preview`, `search`, `lens` and [`KindSpec`]; P06 (editing) the patcher,
//! [`TextEdit`], [`apply_edits`], link target spans and [`Record::was_normalized`], and `preview`
//! before and `update` after each write, backlink spans and [`Placement`]; P07 one index per
//! agent worktree; P09 (notes) the note id, version, state span and text; P11 and P12 (UI) the
//! body model, the UTF-16 conversion of [`TextIndex`], `search`, backlinks and slot targets; P15
//! `lens` and the links for the graph; P17 `preview` for drafts and the same store for
//! embeddings; P19 the same store for comment lists.

pub mod body;
pub mod check;
pub mod finding;
pub mod formalism;
pub mod index;
mod inworld;
pub mod kinds;
pub mod lore;
pub mod parts;
pub mod project;
pub mod record;
pub mod rules;
pub mod settings;
pub mod text;
pub mod walk;
pub mod yaml;

#[cfg(test)]
mod test_support;

pub use body::{
    Attribute, AttributeValue, Beat, Block, BlockKind, Body, Callout, CalloutTitle, Design,
    DesignNote, DesignNoteForm, Fold, Link, Malformed, MalformedKind, Note, NoteState, Token,
    Voice,
};
pub use finding::{Finding, Severity};
pub use index::{
    BeatRef, BinaryReport, Change, Column, EffectiveLens, HeldBelief, Hit, IgnoredEntry, Index,
    IndexError, Layer, LedgerEntry, LensData, LensValue, LinkRef, Match, Origin, Overridden,
    Preview, RecordSummary, Slot, TruthLedger, Update,
};
pub use kinds::{
    BodyGrammar, FieldSpec, FieldType, KindRegistry, KindSpec, Placement, RegistryError, Target,
};
pub use project::{Project, ProjectError, SCHEMA_VERSION};
pub use record::{ReadError, Record, RecordEditError, RecordError, decode, read_record};
pub use text::{Position, Span, TextEdit, TextIndex, apply_edits};
pub use walk::is_record_path;
pub use yaml::{MapProblem, Order, PatchError, PatchOp, YamlError, YamlRule};
