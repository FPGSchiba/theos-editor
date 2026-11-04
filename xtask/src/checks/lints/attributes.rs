//! Rule 3 of `lints`: no `allow` attribute, and no `expect` or `ignore` without a reason where
//! clippy does not look.
//!
//! The lint levels forbid `#[allow]` and require a reason on `#[expect]` and `#[ignore]`, but
//! clippy reads only the syntax tree: it does not see an attribute that is written in the tokens
//! of a macro call or of a `macro_rules!` body. The rule reads both, through `syn` and
//! `proc-macro2`:
//!
//! - in the syntax tree (outer and inner attributes at every depth): every `allow` is a finding,
//!   with or without a reason, alone or inside `cfg_attr`; `expect` and `ignore` are left to
//!   clippy;
//! - in the tokens of macro calls and macro definitions, at every depth: each `#` [`!`] bracket
//!   group is read as an attribute, judged the same way, and an `expect(…)` without
//!   `reason = …` and a bare `ignore` are findings too (`ignore = "…"` passes). A group that does
//!   not parse as an attribute (`#[$m]`, as in `$(#[$m:meta])*`) is judged by its identifiers:
//!   an `allow` in it is a finding, anything else is left to the reviews (Ruling 9);
//! - everywhere, in the tree and in macro tokens, with or without a reason, an `allow` or `expect`
//!   that names one of the lints that guard the others (`unexpected_cfgs`,
//!   `clippy::allow_attributes`, `clippy::allow_attributes_without_reason`,
//!   `clippy::ignore_without_reason`) is a finding (spec §5.2): `forbid` on a lint stops rustc
//!   from honouring such an attribute, but clippy's guard lints are only `deny`.
//!
//! A string or a comment is one token that is never an attribute. An attribute that a macro
//! forwards through a metavariable is judged where the invocation writes it.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::Meta;
use syn::Token;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::cfg_names::split_args;
use super::{Inputs, Rule};
use crate::verdict::Finding;
use crate::view::{Source, line_of};

/// The rule, as `RULES` lists it.
pub const RULE: Rule = Rule {
    name: "attributes",
    run: |inputs: &Inputs<'_>| Ok(inputs.view.sources().flat_map(findings).collect()),
};

/// What an `allow` is told.
const ALLOW: &str = "an `allow` attribute: use `#[expect(…, reason = \"…\")]`, or fix the code";

/// What an `expect` without a reason is told, in macro tokens.
const EXPECT: &str = "an `#[expect]` without a reason, inside a macro where clippy does not see it";

/// What an `ignore` without a reason is told, in macro tokens.
const IGNORE: &str = "an `#[ignore]` without a reason, inside a macro where clippy does not see it";

/// The lints that guard the others: `unexpected_cfgs` is `forbid`, the other three are `deny`.
const GUARDS: [&str; 4] = [
    "unexpected_cfgs",
    "clippy::allow_attributes",
    "clippy::allow_attributes_without_reason",
    "clippy::ignore_without_reason",
];

/// Where an attribute was found.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// In the syntax tree, where clippy judges `expect` and `ignore`.
    Tree,
    /// In the tokens of a macro, where clippy sees nothing.
    Macro,
}

/// The findings of one file, in line order.
#[must_use]
pub fn findings(source: &Source) -> Vec<Finding> {
    let Some(file) = &source.file else {
        return Vec::new();
    };
    let mut walker = Walker {
        path: &source.path,
        findings: Vec::new(),
    };
    walker.visit_file(file);
    walker.findings.sort_by_key(|finding| finding.line);
    walker.findings
}

/// What is wrong with the attribute `meta`: one message per problem.
fn problems(meta: &Meta, place: Place) -> Vec<String> {
    let Some(name) = meta.path().get_ident().map(ToString::to_string) else {
        return Vec::new();
    };
    match (name.as_str(), meta) {
        ("cfg_attr", Meta::List(list)) => cfg_attr_problems(&list.tokens, place),
        ("allow", Meta::List(list)) => {
            let guards = guard_messages(&list.tokens);
            if guards.is_empty() {
                vec![ALLOW.to_owned()]
            } else {
                guards
            }
        }
        ("allow", _) => vec![ALLOW.to_owned()],
        ("expect", Meta::List(list)) => {
            let mut found = guard_messages(&list.tokens);
            if place == Place::Macro && !has_reason(&list.tokens) {
                found.push(EXPECT.to_owned());
            }
            found
        }
        ("ignore", Meta::Path(_)) if place == Place::Macro => vec![IGNORE.to_owned()],
        _ => Vec::new(),
    }
}

/// The problems of the attributes that `cfg_attr(predicate, attribute…)` applies, whose tokens are
/// `tokens`. Arguments that do not parse (a metavariable) are judged by their identifiers.
fn cfg_attr_problems(tokens: &TokenStream, place: Place) -> Vec<String> {
    let parsed = syn::parse::Parser::parse2(
        Punctuated::<Meta, Token![,]>::parse_terminated,
        tokens.clone(),
    );
    match parsed {
        Ok(args) => args
            .iter()
            .skip(1)
            .flat_map(|meta| problems(meta, place))
            .collect(),
        Err(_) => unparsed_problems(tokens),
    }
}

/// An attribute that `syn` cannot read is judged by its identifiers (Ruling 9): an `allow`
/// anywhere in it is a finding.
fn unparsed_problems(tokens: &TokenStream) -> Vec<String> {
    if has_ident(tokens, "allow") {
        vec![ALLOW.to_owned()]
    } else {
        Vec::new()
    }
}

/// Whether the identifier `word` is anywhere in `tokens`.
fn has_ident(tokens: &TokenStream, word: &str) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => ident == word,
        TokenTree::Group(group) => has_ident(&group.stream(), word),
        _ => false,
    })
}

/// Whether the arguments `tokens` of an `expect(…)` have a `reason = …` of their own.
fn has_reason(tokens: &TokenStream) -> bool {
    split_args(tokens).iter().any(|arg| {
        let mut trees = arg.clone().into_iter();
        matches!(trees.next(), Some(TokenTree::Ident(ident)) if ident == "reason")
            && matches!(trees.next(), Some(TokenTree::Punct(punct)) if punct.as_char() == '=')
    })
}

/// A message for each guard lint that the lint list `tokens` names.
fn guard_messages(tokens: &TokenStream) -> Vec<String> {
    split_args(tokens)
        .iter()
        .filter_map(lint_path)
        .filter(|lint| GUARDS.contains(&lint.as_str()))
        .map(|lint| {
            format!(
                "an `allow` or `expect` of `{lint}`, one of the lints that guard the others \
                 (spec §5.2)"
            )
        })
        .collect()
}

/// The lint `tokens` name, as `a::b`, when they are nothing but a path.
fn lint_path(tokens: &TokenStream) -> Option<String> {
    let mut path = String::new();
    for tree in tokens.clone() {
        match tree {
            TokenTree::Ident(ident) => path.push_str(&ident.to_string()),
            TokenTree::Punct(punct) if punct.as_char() == ':' => path.push(':'),
            _ => return None,
        }
    }
    (!path.is_empty()).then_some(path)
}

/// Walks one file and collects the findings of its attributes.
struct Walker<'a> {
    path: &'a str,
    findings: Vec<Finding>,
}

impl Walker<'_> {
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
                        self.judge_group(line_of(pound.span()), &group.stream());
                    }
                }
                TokenTree::Group(group) => self.scan_tokens(&group.stream()),
                _ => {}
            }
        }
    }

    /// Judges the content of a bracket group that follows a `#` in macro tokens.
    fn judge_group(&mut self, line: u32, content: &TokenStream) {
        let found = match syn::parse2::<Meta>(content.clone()) {
            Ok(meta) => problems(&meta, Place::Macro),
            Err(_) => unparsed_problems(content),
        };
        for message in found {
            self.findings.push(Finding::at(self.path, line, message));
        }
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        let line = line_of(attribute.pound_token.span());
        for message in problems(&attribute.meta, Place::Tree) {
            self.findings.push(Finding::at(self.path, line, message));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.scan_tokens(&mac.tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::parse_source;

    /// The line and message of every finding for the source `text`.
    fn judged(text: &str) -> Vec<(u32, String)> {
        let (source, finding) = parse_source("crates/a/src/lib.rs", text.as_bytes());
        assert!(finding.is_none(), "{finding:?}");
        findings(&source)
            .into_iter()
            .map(|finding| (finding.line.unwrap_or(0), finding.message))
            .collect()
    }

    fn lines(text: &str) -> Vec<u32> {
        judged(text).into_iter().map(|(line, _)| line).collect()
    }

    #[test]
    fn outer_and_inner_allow_are_findings_with_their_line() {
        let text = "#![allow(missing_docs)]\n\n#[allow(dead_code)]\nfn a() {}\n\
                    fn b() { #[allow(unused)] let _x = 1; }\n\
                    #[allow(\n    clippy::too_many_lines,\n    dead_code\n)]\nfn c() {}\n";
        let found = judged(text);
        assert_eq!(
            found.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
            [1, 3, 5, 6]
        );
        assert!(
            found.iter().all(|(_, message)| message == ALLOW),
            "{found:?}"
        );
    }

    #[test]
    fn allow_inside_cfg_attr_is_a_finding() {
        assert_eq!(lines("#[cfg_attr(test, allow(unused))]\nfn a() {}\n"), [1]);
        assert_eq!(
            lines(
                "#[cfg_attr(\n    test,\n    derive(Debug),\n    allow(dead_code)\n)]\nstruct S;\n"
            ),
            [1]
        );
        assert_eq!(
            lines("#[cfg_attr(a, cfg_attr(b, allow(unused)))]\nfn a() {}\n"),
            [1]
        );
        assert_eq!(lines("#![cfg_attr(test, allow(unused))]\n"), [1]);
    }

    #[test]
    fn an_inner_allow_with_a_reason_in_a_one_line_module_is_a_finding() {
        assert_eq!(
            lines("mod m { #![allow(unused, reason = \"in one line\") ] }\n"),
            [1]
        );
        assert_eq!(
            lines("#[allow(dead_code, reason = \"why\")]\nfn a() {}\n"),
            [1]
        );
    }

    #[test]
    fn allow_in_a_macro_invocation_and_in_a_macro_rules_body_are_findings() {
        let text = "macro_rules! q { () => { #[allow(dead_code)] fn a() {} } }\n\
                    q! { }\n\
                    fwd! { #[allow(dead_code)] fn c() }\n\
                    fn f() { m!(nested { #![allow(unused)] }); }\n";
        assert_eq!(lines(text), [1, 3, 4]);
    }

    #[test]
    fn expect_and_ignore_without_reason_in_macro_tokens_are_findings_and_with_a_reason_pass() {
        let text = "macro_rules! q { () => { #[expect(dead_code)] fn a() {} \
                    #[test] #[ignore] fn b() {} } }\n";
        let found = judged(text);
        assert_eq!(
            found,
            [(1, EXPECT.to_owned()), (1, IGNORE.to_owned())],
            "{found:?}"
        );
        let passing = "macro_rules! q { () => { #[expect(dead_code, reason = \"kept\")] fn a() {} \
                       #[test] #[ignore = \"slow\"] fn b() {} } }\n\
                       m! { #[cfg_attr(test, expect(unused, reason = \"x\"))] fn c() }\n";
        assert_eq!(judged(passing), []);
        // Inside `cfg_attr` the same judgement applies, and a blank-looking list is not a reason.
        let nested = "m! { #[cfg_attr(test, expect(unused))] #[cfg_attr(test, ignore)] fn c() }\n";
        assert_eq!(
            judged(nested),
            [(1, EXPECT.to_owned()), (1, IGNORE.to_owned())]
        );
    }

    #[test]
    fn expect_and_ignore_in_the_tree_are_left_to_clippy() {
        let text = "#[expect(dead_code)]\nfn a() {}\n#[test]\n#[ignore]\nfn b() {}\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn a_forwarded_attribute_is_not_judged() {
        let text = "macro_rules! fwd {\n    ($(#[$m:meta])* fn $n:ident()) => { $(#[$m])* fn $n() {} };\n    \
                    ($(#[$a:meta])+ struct $s:ident;) => { #[$a] struct $s; };\n}\n";
        assert_eq!(judged(text), []);
        // An `allow` ident in an attribute that cannot be read is still caught (Ruling 9).
        assert_eq!(
            lines("macro_rules! m { ($x:ty) => { #[allow($x)] fn a() {} } }\n"),
            [1]
        );
    }

    #[test]
    fn derives_doc_comments_and_cfg_attr_derive_pass() {
        let text = "//! Docs.\n/// Docs.\n#[derive(Debug, Clone)]\n#[cfg_attr(test, derive(PartialEq))]\n\
                    #[doc = \"allow\"]\n#[rustfmt::skip]\nstruct S;\n\
                    #[cfg(test)]\nmod tests {}\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn text_in_strings_and_comments_is_not_an_attribute() {
        let text = "const A: &str = \"#[allow(dead_code)]\";\n\
                    const B: &str = r#\"\n#![allow(dead_code)]\n\"#;\n\
                    // #[allow(dead_code)]\n/* #[allow(dead_code)] */\n/// #[allow(dead_code)]\n\
                    /* /* */ \" */ fn f() {} // \"\n\
                    fn g() -> char { '#' }\n\
                    m! { \"#[allow(x)]\" }\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn an_allow_or_expect_of_a_guard_lint_is_a_finding() {
        for lint in GUARDS {
            let message = format!(
                "an `allow` or `expect` of `{lint}`, one of the lints that guard the others \
                 (spec §5.2)"
            );
            for text in [
                format!("#[allow({lint}, reason = \"r\")]\nfn a() {{}}\n"),
                format!("#![allow({lint}, reason = \"r\")]\n"),
                format!("#[expect({lint}, reason = \"r\")]\nfn a() {{}}\n"),
                format!("#![expect({lint}, reason = \"r\")]\n"),
                format!("#[cfg_attr(test, expect({lint}, reason = \"r\"))]\nfn a() {{}}\n"),
                format!("#[cfg_attr(test, allow({lint}, reason = \"r\"))]\nfn a() {{}}\n"),
                format!(
                    "macro_rules! m {{ () => {{ #[expect({lint}, reason = \"r\")] fn a() {{}} }} }}\n"
                ),
                format!("m! {{ #[allow({lint}, reason = \"r\")] fn a() }}\n"),
                format!("#[expect(dead_code, {lint}, reason = \"r\")]\nfn a() {{}}\n"),
            ] {
                assert_eq!(judged(&text), [(1, message.clone())], "{text}");
            }
        }
        let other = "#[expect(clippy::unwrap_used, reason = \"r\")]\nfn a() {}\n\
                     m! { #[expect(clippy::unwrap_used, reason = \"r\")] fn b() }\n\
                     #[expect(my::unexpected_cfgs, reason = \"r\")]\nfn c() {}\n";
        assert_eq!(judged(other), []);
    }

    #[test]
    fn findings_are_in_line_order_and_a_file_that_did_not_parse_has_none() {
        let text = "m! { #[allow(a)] fn a() }\n#[allow(b)]\nfn b() {}\nm! { #[allow(c)] fn c() }\n";
        assert_eq!(lines(text), [1, 2, 4]);
        let (source, _) = parse_source("a.rs", b"fn {");
        assert_eq!(findings(&source), []);
    }

    #[test]
    fn a_byte_order_mark_does_not_move_the_line() {
        let (source, _) = parse_source("a.rs", "\u{feff}#[allow(a)]\nfn a() {}\n".as_bytes());
        assert_eq!(findings(&source)[0].line, Some(1));
    }
}
