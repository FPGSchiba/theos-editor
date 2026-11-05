//! Text primitives shared by every grammar: spans, positions, line endings and edits.

mod edit;
mod index;
mod normalize;
mod span;

pub use edit::{EditError, TextEdit, apply_edits};
pub use index::{Position, PositionError, TextIndex};
pub use normalize::normalize;
pub use span::Span;
