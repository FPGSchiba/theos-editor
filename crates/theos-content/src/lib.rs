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
//! Every span is a byte range into the LF text of the whole record; a CRLF record is read as its
//! LF text and a write returns LF.
//!
//! Later phases use it so: P03 (index, registry) the record model, field and entry spans, the
//! order injection of the patcher and the body links; P04 (validators) the malformed items,
//! design notes, voice spans and note states; P05 the positions of errors; P06 (editing) the
//! patcher, [`TextEdit`], [`apply_edits`], link target spans and [`Record::was_normalized`]; P09
//! (notes) the note id, version, state span and text; P12 (UI) the body model and the UTF-16
//! conversion of [`TextIndex`].

pub mod body;
pub mod finding;
pub mod kinds;
pub mod project;
pub mod record;
pub mod text;
pub mod yaml;

#[cfg(test)]
mod test_support;

pub use body::{
    Attribute, AttributeValue, Beat, Block, BlockKind, Body, Callout, CalloutTitle, Design,
    DesignNote, DesignNoteForm, Fold, Link, Malformed, MalformedKind, Note, NoteState, Token,
    Voice,
};
pub use finding::{Finding, Severity};
pub use kinds::{
    BodyGrammar, FieldSpec, FieldType, KindRegistry, KindSpec, Placement, RegistryError, Target,
};
pub use project::{Project, ProjectError, SCHEMA_VERSION};
pub use record::{ReadError, Record, RecordEditError, RecordError, decode, read_record};
pub use text::{Position, Span, TextEdit, TextIndex, apply_edits};
pub use yaml::{MapProblem, Order, PatchError, PatchOp, YamlError, YamlRule};
