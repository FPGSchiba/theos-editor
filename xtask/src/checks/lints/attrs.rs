//! The attributes of a Rust file, for the rules of `lints` that judge them (rules 2 and 3).
//!
//! `syn` reads the syntax tree and `proc-macro2` the tokens; nothing here lexes Rust. Every
//! attribute of a file comes out once, with its line and where it was found: in the syntax tree
//! (outer and inner, at every depth), or in the tokens of a macro call or `macro_rules!` body (each
//! `#` [`!`] bracket group, at every depth). A rule keeps only its judgement and its messages.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::Meta;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::view::line_of;

/// Where an attribute was found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    /// In the syntax tree, where clippy judges `expect` and `ignore`.
    Tree,
    /// In the tokens of a macro, where clippy sees nothing.
    Macro,
}

/// What an attribute says, as far as `syn` can read it.
#[derive(Clone, Debug)]
pub enum Body {
    /// The content of the brackets, read.
    Meta(Box<Meta>),
    /// The content of the brackets of a macro-token attribute that `syn` cannot read (a
    /// metavariable such as `#[$m]`): the tokens as they are.
    Unparsed(TokenStream),
}

/// One attribute of a file.
#[derive(Clone, Debug)]
pub struct Attribute {
    /// The line of its `#`.
    pub line: u32,
    /// Where it was found.
    pub place: Place,
    /// What it says.
    pub body: Body,
}

/// Every attribute of `file`, in the order the tree and its macros are walked.
#[must_use]
pub fn attributes(file: &syn::File) -> Vec<Attribute> {
    let mut collector = Collector::default();
    collector.visit_file(file);
    collector.found
}

/// The comma-separated arguments of a list, each as its own tokens; a trailing comma adds none.
/// Commas inside a group belong to the group, so nesting needs no parser of ours.
#[must_use]
pub fn split_args(tokens: &TokenStream) -> Vec<TokenStream> {
    let mut args = Vec::new();
    let mut current = TokenStream::new();
    for tree in tokens.clone() {
        match &tree {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                args.push(std::mem::take(&mut current));
            }
            _ => current.extend([tree]),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

#[derive(Default)]
struct Collector {
    found: Vec<Attribute>,
}

impl Collector {
    /// The `#` `[`…`]` and `#` `!` `[`…`]` groups of the tokens of a macro, at any depth.
    fn scan_tokens(&mut self, tokens: &TokenStream) {
        let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
        for (index, tree) in trees.iter().enumerate() {
            match tree {
                TokenTree::Punct(pound) if pound.as_char() == '#' => {
                    let mut rest = trees[index + 1..].iter();
                    let mut next = rest.next();
                    if matches!(next, Some(TokenTree::Punct(bang)) if bang.as_char() == '!') {
                        next = rest.next();
                    }
                    if let Some(TokenTree::Group(group)) = next
                        && group.delimiter() == Delimiter::Bracket
                    {
                        let content = group.stream();
                        let body = syn::parse2::<Meta>(content.clone())
                            .map_or(Body::Unparsed(content), |meta| Body::Meta(Box::new(meta)));
                        self.found.push(Attribute {
                            line: line_of(pound.span()),
                            place: Place::Macro,
                            body,
                        });
                    }
                }
                TokenTree::Group(group) => self.scan_tokens(&group.stream()),
                _ => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        self.found.push(Attribute {
            line: line_of(attribute.pound_token.span()),
            place: Place::Tree,
            body: Body::Meta(Box::new(attribute.meta.clone())),
        });
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.scan_tokens(&mac.tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::parse_source;

    fn read(text: &str) -> Vec<Attribute> {
        let (source, finding) = parse_source("a.rs", text.as_bytes());
        assert!(finding.is_none(), "{finding:?}");
        source.file.as_ref().map(attributes).unwrap_or_default()
    }

    #[test]
    fn tree_and_macro_attributes_come_with_line_and_place() {
        let found = read(
            "#![a]\n#[b]\nfn f() { #![c] }\nm! { #[d] x { #![e] } }\nmacro_rules! q { () => { #[f] } }\n",
        );
        let places: Vec<(u32, Place)> = found.iter().map(|a| (a.line, a.place)).collect();
        assert_eq!(
            places,
            [
                (1, Place::Tree),
                (2, Place::Tree),
                (3, Place::Tree),
                (4, Place::Macro),
                (4, Place::Macro),
                (5, Place::Macro),
            ]
        );
    }

    #[test]
    fn a_macro_attribute_that_does_not_parse_is_kept_as_tokens() {
        let found = read("macro_rules! q { ($m:meta) => { #[$m] #[ok] fn a() {} } }\n");
        assert!(matches!(found[0].body, Body::Unparsed(_)), "{found:?}");
        assert!(matches!(found[1].body, Body::Meta(_)), "{found:?}");
    }

    #[test]
    fn arguments_split_at_top_level_commas_only() {
        let tokens: TokenStream = "a, f(b, c), d,".parse().unwrap_or_default();
        assert_eq!(split_args(&tokens).len(), 3);
    }
}
