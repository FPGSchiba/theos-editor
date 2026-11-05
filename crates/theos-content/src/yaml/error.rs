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
    /// A control character other than a tab anywhere in the front matter: scalars, comments and
    /// blank lines.
    ControlCharacter,
    /// A scalar that spans more than one line.
    OneLineScalar,
    /// An escape sequence in a double-quoted scalar other than a double backslash
    /// and a backslash-quote.
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
    /// A flow list that spans more than one line, or a block list as the value of a map entry.
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
    /// A key with no value, or a list item (`- `) with no value.
    EmptyValue,
    /// An error of the YAML crate itself, or the reader's own "the front matter ends early here"
    /// when the scanner ended the stream before the end of the front matter.
    Syntax,
}

impl YamlRule {
    /// Every rule, in the order of the phase spec.
    pub const ALL: [YamlRule; 21] = [
        YamlRule::Anchor,
        YamlRule::Alias,
        YamlRule::Tag,
        YamlRule::BlockScalar,
        YamlRule::ControlCharacter,
        YamlRule::OneLineScalar,
        YamlRule::Escape,
        YamlRule::FlowMap,
        YamlRule::Layout,
        YamlRule::ListOfMaps,
        YamlRule::NestedMap,
        YamlRule::ListInList,
        YamlRule::OneLineList,
        YamlRule::TopLevelMap,
        YamlRule::TabIndent,
        YamlRule::DuplicateKey,
        YamlRule::Key,
        YamlRule::ComplexKey,
        YamlRule::DocumentMarker,
        YamlRule::EmptyValue,
        YamlRule::Syntax,
    ];

    /// The kebab-case name of the rule, as the phase spec lists it (`syntax` for the crate).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.id().strip_prefix("yaml/").unwrap_or_default()
    }

    /// The rule id of a finding, `yaml/<name>`.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            YamlRule::Anchor => "yaml/anchor",
            YamlRule::Alias => "yaml/alias",
            YamlRule::Tag => "yaml/tag",
            YamlRule::BlockScalar => "yaml/block-scalar",
            YamlRule::ControlCharacter => "yaml/control-character",
            YamlRule::OneLineScalar => "yaml/one-line-scalar",
            YamlRule::Escape => "yaml/escape",
            YamlRule::FlowMap => "yaml/flow-map",
            YamlRule::Layout => "yaml/layout",
            YamlRule::ListOfMaps => "yaml/list-of-maps",
            YamlRule::NestedMap => "yaml/nested-map",
            YamlRule::ListInList => "yaml/list-in-list",
            YamlRule::OneLineList => "yaml/one-line-list",
            YamlRule::TopLevelMap => "yaml/top-level-map",
            YamlRule::TabIndent => "yaml/tab-indent",
            YamlRule::DuplicateKey => "yaml/duplicate-key",
            YamlRule::Key => "yaml/key",
            YamlRule::ComplexKey => "yaml/complex-key",
            YamlRule::DocumentMarker => "yaml/document-marker",
            YamlRule::EmptyValue => "yaml/empty-value",
            YamlRule::Syntax => "yaml/syntax",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustive, so a new rule must be added here and then to [`YamlRule::ALL`].
    fn index(rule: YamlRule) -> usize {
        match rule {
            YamlRule::Anchor => 0,
            YamlRule::Alias => 1,
            YamlRule::Tag => 2,
            YamlRule::BlockScalar => 3,
            YamlRule::ControlCharacter => 4,
            YamlRule::OneLineScalar => 5,
            YamlRule::Escape => 6,
            YamlRule::FlowMap => 7,
            YamlRule::Layout => 8,
            YamlRule::ListOfMaps => 9,
            YamlRule::NestedMap => 10,
            YamlRule::ListInList => 11,
            YamlRule::OneLineList => 12,
            YamlRule::TopLevelMap => 13,
            YamlRule::TabIndent => 14,
            YamlRule::DuplicateKey => 15,
            YamlRule::Key => 16,
            YamlRule::ComplexKey => 17,
            YamlRule::DocumentMarker => 18,
            YamlRule::EmptyValue => 19,
            YamlRule::Syntax => 20,
        }
    }

    #[test]
    fn all_lists_every_rule_once_and_names_follow_ids() {
        for (at, rule) in YamlRule::ALL.iter().enumerate() {
            assert_eq!(index(*rule), at);
            assert_eq!(rule.id(), format!("yaml/{}", rule.as_str()));
        }
        assert_eq!(YamlRule::Syntax.as_str(), "syntax");
    }
}
