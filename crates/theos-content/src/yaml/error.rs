//! The rules of the restricted YAML subset and the error a violation reports.

use crate::text::{Position, Span};

/// The rule of the data model's restricted YAML that a front matter breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YamlRule {
    /// An anchor (`&x`).
    Anchor,
    /// An alias (`*x`).
    Alias,
    /// A YAML tag (`!t`).
    Tag,
    /// A block scalar (`|` or `>`).
    BlockScalar,
    /// A control character other than a tab in a scalar.
    ControlCharacter,
    /// A scalar that spans more than one line.
    OneLineScalar,
    /// An escape sequence in a double-quoted scalar.
    Escape,
    /// A flow map (`{a: 1}`).
    FlowMap,
    /// A spelling of a layout that the reader refuses: an indented map, a space before a colon,
    /// a list item on the line after its dash.
    Layout,
    /// A list of maps.
    ListOfMaps,
    /// A map nested deeper than the model allows.
    NestedMap,
    /// A list inside a list.
    ListInList,
    /// A flow list that spans more than one line.
    OneLineList,
    /// A front matter that is not a map.
    TopLevelMap,
    /// A tab used for indentation.
    TabIndent,
    /// A key that occurs twice in one map.
    DuplicateKey,
    /// A key that is not a plain key of the key pattern.
    Key,
    /// A complex key (`? a`).
    ComplexKey,
    /// A document marker or directive (`...`, `%YAML`).
    DocumentMarker,
    /// A key with no value.
    EmptyValue,
    /// An error of the YAML crate itself.
    Syntax,
}

impl YamlRule {
    /// The kebab-case name of the rule, as the phase spec lists it (`syntax` for the crate).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            YamlRule::Anchor => "anchor",
            YamlRule::Alias => "alias",
            YamlRule::Tag => "tag",
            YamlRule::BlockScalar => "block-scalar",
            YamlRule::ControlCharacter => "control-character",
            YamlRule::OneLineScalar => "one-line-scalar",
            YamlRule::Escape => "escape",
            YamlRule::FlowMap => "flow-map",
            YamlRule::Layout => "layout",
            YamlRule::ListOfMaps => "list-of-maps",
            YamlRule::NestedMap => "nested-map",
            YamlRule::ListInList => "list-in-list",
            YamlRule::OneLineList => "one-line-list",
            YamlRule::TopLevelMap => "top-level-map",
            YamlRule::TabIndent => "tab-indent",
            YamlRule::DuplicateKey => "duplicate-key",
            YamlRule::Key => "key",
            YamlRule::ComplexKey => "complex-key",
            YamlRule::DocumentMarker => "document-marker",
            YamlRule::EmptyValue => "empty-value",
            YamlRule::Syntax => "syntax",
        }
    }
}

/// A front matter that breaks the restricted YAML, with where it does.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{position}: {message}")]
pub struct YamlError {
    /// The rule that was broken.
    pub rule: YamlRule,
    /// What is wrong, for the user.
    pub message: String,
    /// The offending text, in record coordinates.
    pub span: Span,
    /// Line and column of the start of `span`, in record coordinates.
    pub position: Position,
}
