//! Rule 2 of `lints`: `cfg` and `cfg_attr` attributes may use only the names of
//! `gate/cfg-names.toml`.
//!
//! Each check builds the workspace under its own cfgs: clippy sets `clippy`, cargo-llvm-cov sets
//! `coverage`, a profile sets `debug_assertions` and `panic`, `miri` sets `miri`, and a `build.rs`
//! can set any name. Code under `#[cfg(not(clippy))]` is never seen by clippy and
//! `#[cfg(not(coverage))]` is never counted, while both ship. `unexpected_cfgs` at `forbid` already
//! refuses every name that rustc and cargo do not know; this rule refuses the known names that
//! differ between the builds the checks make. It is an allowlist: the bare `names` of the file
//! (`test`, `doc`, `unix`, `windows`) and the `keys` that take a value (`feature`, `target_os`, …,
//! which rustc checks), combined with `all`, `any` and `not`. A predicate of any other shape,
//! such as a literal (`cfg(false)` hides code from every check) or a path with `::`, is a
//! finding too.
//!
//! The attributes come from `syn`: the syntax tree holds every attribute that is code, including
//! the `cfg` nested in a `cfg_attr`, and the tokens of a macro invocation or definition are read
//! for a `#` `[` … `]` group (inner or outer) that parses as a `cfg` or `cfg_attr` attribute. A
//! predicate with a macro metavariable (`#[cfg($p)]`) is not judged (Ruling 6): `unexpected_cfgs`
//! refuses unknown names, and the invocation is judged where it writes the predicate itself.
//!
//! `cfg!(…)` in an expression is not an attribute: both of its branches are compiled and
//! measured, so it is out of scope (Ruling 7).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use serde::Deserialize;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprLit, Ident, Lit, Meta, MetaList, Token};

use super::{Inputs, Rule};
use crate::verdict::Finding;
use crate::view::line_of;
use crate::{Error, Result};

/// The file that lists the names.
const FILE: &str = "gate/cfg-names.toml";

/// The longest word that a finding repeats.
const WORD_LIMIT: usize = 40;

/// The rule, as `RULES` lists it.
pub const RULE: Rule = Rule {
    name: "cfg-names",
    run: |inputs| Ok(findings(inputs)),
};

/// The cfg names that code may test, from `gate/cfg-names.toml`.
#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CfgNames {
    /// Names that may stand alone: `#[cfg(test)]`.
    pub names: BTreeSet<String>,
    /// Names that take a value: `#[cfg(target_os = "windows")]`.
    pub keys: BTreeSet<String>,
}

impl CfgNames {
    /// Reads `gate/cfg-names.toml` of the repository at `root`.
    ///
    /// # Errors
    ///
    /// Fails with [`Error::Config`] when the file is missing or is not what is expected.
    pub fn load(root: &Path) -> Result<Self> {
        let config = |message: String| Error::Config {
            file: FILE.to_owned(),
            message,
        };
        let text = fs::read_to_string(root.join(FILE))
            .map_err(|error| config(format!("cannot be read: {error}")))?;
        toml::from_str(&text).map_err(|error| {
            // The error quotes the text; the line is all that is shown.
            let place = error.span().map_or_else(String::new, |span| {
                let line = text
                    .get(..span.start)
                    .unwrap_or(&text)
                    .matches('\n')
                    .count()
                    + 1;
                format!(" near line {line}")
            });
            config(format!(
                "is not a table of `names` and `keys`, two arrays of strings{place}"
            ))
        })
    }
}

fn findings(inputs: &Inputs<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for source in inputs.view.sources() {
        let Some(file) = &source.file else { continue };
        let mut walker = Walker {
            names: inputs.cfg_names,
            file: &source.path,
            findings: Vec::new(),
        };
        walker.visit_file(file);
        findings.extend(walker.findings);
    }
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    findings
}

/// Judges the predicate of a `cfg(…)`: a bare name that is listed, a listed key with a string,
/// or `all`/`any`/`not` over those.
///
/// # Errors
///
/// The message says what is refused. It repeats a rejected word as ASCII letters, digits and `_`
/// (every other character as `?`), at most 40 characters of it.
pub fn judge(predicate: &Meta, names: &CfgNames) -> std::result::Result<(), String> {
    match predicate {
        Meta::Path(path) => match path.get_ident() {
            Some(ident) if names.names.contains(&ident.to_string()) => Ok(()),
            Some(ident) => Err(refused_word(&ident.to_string())),
            None => Err(unreadable()),
        },
        Meta::NameValue(pair) => {
            let is_string = matches!(
                &pair.value,
                Expr::Lit(ExprLit {
                    lit: Lit::Str(_),
                    ..
                })
            );
            match pair.path.get_ident() {
                Some(ident) if names.keys.contains(&ident.to_string()) && is_string => Ok(()),
                Some(ident) if is_string => Err(refused_word(&ident.to_string())),
                _ => Err(unreadable()),
            }
        }
        Meta::List(list) => {
            let head = list.path.get_ident().map(ToString::to_string);
            match head.as_deref() {
                Some(combinator @ ("all" | "any" | "not")) => {
                    let inner = parse_args(list).map_err(|_| unreadable())?;
                    if combinator == "not" && inner.len() != 1 {
                        return Err(unreadable());
                    }
                    inner.iter().try_for_each(|arg| judge_arg(arg, names))
                }
                Some(other) => Err(refused_word(other)),
                None => Err(unreadable()),
            }
        }
    }
}

/// One argument of a list: a meta item, or a keyword (`false`, `true`) that `Meta` cannot hold
/// but that `cfg(false)` writes.
enum Arg {
    Meta(Box<Meta>),
    Keyword(String),
}

impl Parse for Arg {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Ident::peek_any) && !input.peek(Ident) {
            Ok(Self::Keyword(Ident::parse_any(input)?.to_string()))
        } else {
            input.parse().map(|meta| Self::Meta(Box::new(meta)))
        }
    }
}

/// The comma-separated arguments of `list`.
fn parse_args(list: &MetaList) -> syn::Result<Vec<Arg>> {
    list.parse_args_with(Punctuated::<Arg, Token![,]>::parse_terminated)
        .map(|args| args.into_iter().collect())
}

fn judge_arg(arg: &Arg, names: &CfgNames) -> std::result::Result<(), String> {
    match arg {
        Arg::Meta(meta) => judge(meta, names),
        Arg::Keyword(word) => Err(refused_word(word)),
    }
}

/// What is wrong with the attribute `meta`, when it is a `cfg` or `cfg_attr` that breaks the
/// rule; `None` for every other attribute.
fn attribute_problem(meta: &Meta, names: &CfgNames) -> Option<String> {
    let path = meta.path();
    let is_cfg = path.is_ident("cfg");
    if !is_cfg && !path.is_ident("cfg_attr") {
        return None;
    }
    let Meta::List(list) = meta else {
        return Some(unreadable());
    };
    let Ok(args) = parse_args(list) else {
        return Some(unreadable());
    };
    let mut args = args.into_iter();
    let Some(predicate) = args.next() else {
        return Some(unreadable());
    };
    if is_cfg && args.next().is_some() {
        return Some(unreadable());
    }
    if let Err(message) = judge_arg(&predicate, names) {
        return Some(message);
    }
    // `cfg_attr(predicate, attribute…)`: the attributes it applies are judged the same way.
    args.find_map(|nested| match nested {
        Arg::Meta(nested) => attribute_problem(&nested, names),
        Arg::Keyword(_) => None,
    })
}

/// The message for a refused name or key.
fn refused_word(word: &str) -> String {
    format!(
        "`{}` in a `cfg(…)` or `cfg_attr(…)` predicate is not a cfg name of \
         `gate/cfg-names.toml`: use only those, with `all`, `any` and `not`, so that every check \
         builds and measures the same code; another cfg needs a gate change approved by Jann",
        echo(word)
    )
}

/// The message for a predicate of a shape that is not judged.
fn unreadable() -> String {
    "a `cfg(…)` or `cfg_attr(…)` predicate that cannot be read: only a name, `key = \"value\"` \
     and `all`, `any` and `not` over those are allowed, with the names of \
     `gate/cfg-names.toml`; another cfg needs a gate change approved by Jann"
        .to_owned()
}

/// `word` as a finding repeats it: ASCII letters, digits and `_` as they are, every other
/// character as `?`, cut at 40 characters. No format character, bidi control or `::` reaches the
/// output.
fn echo(word: &str) -> String {
    word.chars()
        .take(WORD_LIMIT)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '?'
            }
        })
        .collect()
}

/// Walks one file and collects the findings of its attributes.
struct Walker<'a> {
    names: &'a CfgNames,
    file: &'a str,
    findings: Vec<Finding>,
}

impl Walker<'_> {
    fn report(&mut self, line: u32, message: String) {
        self.findings.push(Finding::at(self.file, line, message));
    }

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

    /// Judges the content of a bracket group when it is a `cfg` or `cfg_attr` attribute without
    /// a macro metavariable.
    fn judge_group(&mut self, line: u32, content: &TokenStream) {
        if has_dollar(content) {
            return;
        }
        let Ok(meta) = syn::parse2::<Meta>(content.clone()) else {
            return;
        };
        if let Some(message) = attribute_problem(&meta, self.names) {
            self.report(line, message);
        }
    }
}

/// Whether a `$` is anywhere in `tokens`.
fn has_dollar(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Punct(punct) => punct.as_char() == '$',
        TokenTree::Group(group) => has_dollar(&group.stream()),
        _ => false,
    })
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if let Some(message) = attribute_problem(&attribute.meta, self.names) {
            self.report(line_of(attribute.pound_token.span()), message);
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

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn names() -> CfgNames {
        let set = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
        CfgNames {
            names: set(&["test", "doc", "unix", "windows"]),
            keys: set(&["feature", "target_os", "target_env"]),
        }
    }

    /// The messages and lines of the rule over the Rust text `text`.
    fn judged(text: &str) -> Vec<(u32, String)> {
        let (source, finding) = parse_source("a.rs", text.as_bytes());
        assert!(finding.is_none(), "{finding:?}");
        let names = names();
        let mut walker = Walker {
            names: &names,
            file: "a.rs",
            findings: Vec::new(),
        };
        if let Some(file) = &source.file {
            walker.visit_file(file);
        }
        walker
            .findings
            .into_iter()
            .map(|finding| (finding.line.unwrap_or(0), finding.message))
            .collect()
    }

    fn lines(text: &str) -> Vec<u32> {
        judged(text).into_iter().map(|(line, _)| line).collect()
    }

    #[test]
    fn ordinary_predicates_pass() {
        let text = "#![cfg(doc)]\n#[cfg(test)]\nmod t {}\n#[cfg(not(test))]\nfn a() {}\n\
                    #[cfg(windows)]\nfn b() {}\n#[cfg(target_os = \"macos\")]\nfn c() {}\n\
                    #[cfg(all(unix, not(target_env = \"musl\")))]\nfn d() {}\n\
                    #[cfg(feature = \"extra\")]\nfn e() {}\n#[cfg(any())]\nfn f() {}\n\
                    #[cfg_attr(test, derive(Debug))]\nstruct S;\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn a_literal_or_a_path_with_segments_is_refused() {
        for predicate in [
            "false",
            "true",
            "a::b",
            "target_os = 1",
            "feature",
            "target_os",
            "not(test, doc)",
            "not()",
            "foo(test)",
            "all(test, miri)",
        ] {
            let text = format!("#[cfg({predicate})]\nfn a() {{}}\n");
            assert_eq!(lines(&text), [1], "{predicate}");
        }
        assert_eq!(lines("#[cfg = \"x\"]\nfn a() {}\n"), [1]);
        assert_eq!(lines("#[cfg(\"x\")]\nfn a() {}\n"), [1]);
    }

    #[test]
    fn nested_cfg_attr_and_combinators_are_judged() {
        let text = "#[cfg_attr(test, cfg(not(clippy)))]\nfn a() {}\n\
                    #[cfg_attr(not(clippy), allow(dead_code))]\nfn b() {}\n\
                    #[cfg_attr(test, cfg_attr(miri, inline))]\nfn c() {}\n\
                    #[cfg_attr(test, derive(Debug), cfg(any(unix, docsrs)))]\nstruct D;\n\
                    #[cfg(any(windows, all(unix, not(coverage))))]\nfn e() {}\n";
        let found = judged(text);
        let words: Vec<&str> = ["clippy", "clippy", "miri", "docsrs", "coverage"]
            .into_iter()
            .collect();
        assert_eq!(found.len(), words.len(), "{found:?}");
        for ((line, message), word) in found.iter().zip(words) {
            assert!(message.contains(&format!("`{word}`")), "{line}: {message}");
        }
        let at: Vec<u32> = found.iter().map(|(line, _)| *line).collect();
        assert_eq!(at, [1, 3, 5, 7, 9]);
    }

    #[test]
    fn macro_bodies_are_judged_where_they_write_the_attribute() {
        let text = "macro_rules! m { () => { #[cfg(not(clippy))] fn g() {} } }\n\
                    macro_rules! n { () => { #![cfg(miri)] } }\n\
                    m! { #[cfg(docsrs)] fn h() {} }\n\
                    fn f() { m!(nested { #[cfg_attr(coverage, inline)] }); }\n";
        assert_eq!(lines(text), [1, 2, 3, 4]);
    }

    #[test]
    fn a_metavariable_in_a_predicate_is_not_judged() {
        let text = "macro_rules! p { ($p:meta) => { #[cfg($p)] fn k() {} } }\n\
                    macro_rules! q { ($p:meta) => { #[cfg_attr($p, inline)] fn k() {} } }\n\
                    macro_rules! r { ($(#[$m:meta])* $v:vis fn $n:ident()) => { $(#[$m])* $v fn $n() {} } }\n\
                    macro_rules! s { ($m:meta) => { #[$m] fn k() {} } }\n\
                    p!(not(clippy));\nr! { #[inline] fn h() }\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn a_name_in_a_string_or_a_comment_is_not_judged() {
        let text = "// #[cfg(not(clippy))]\n/// #[cfg(miri)]\n/* #[cfg(coverage)] */\n\
                    const S: &str = \"#[cfg(not(clippy))]\";\n\
                    const T: &str = r#\"#![cfg(miri)]\"#;\nfn f() -> bool { cfg!(clippy) }\n";
        assert_eq!(judged(text), []);
    }

    #[test]
    fn the_echoed_word_is_ascii_and_short() -> TestResult {
        let predicate: Meta = syn::parse_str("test\u{e9}")?;
        let message = judge(&predicate, &names()).err().ok_or("no refusal")?;
        assert!(message.contains("`test?`"), "{message}");

        let long = "a".repeat(100);
        let predicate: Meta = syn::parse_str(&long)?;
        let message = judge(&predicate, &names()).err().ok_or("no refusal")?;
        assert!(
            message.contains(&format!("`{}`", "a".repeat(40))),
            "{message}"
        );
        assert!(!message.contains(&"a".repeat(41)), "{message}");

        let predicate: Meta = syn::parse_str("r#try")?;
        let message = judge(&predicate, &names()).err().ok_or("no refusal")?;
        assert!(message.contains("`r?try`"), "{message}");
        Ok(())
    }

    #[test]
    fn the_names_file_must_be_a_table_of_two_arrays() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("gate"))?;
        let load = |text: &str| -> TestResult<Result<CfgNames>> {
            fs::write(dir.path().join(FILE), text)?;
            Ok(CfgNames::load(dir.path()))
        };
        let loaded = load("names = [\"test\"]\nkeys = [\"feature\"]\n")??;
        assert!(loaded.names.contains("test") && loaded.keys.contains("feature"));
        for bad in [
            "names = [\"test\"]\n",
            "names = 1\nkeys = []\n",
            "x = = 1\n",
        ] {
            assert!(matches!(load(bad)?, Err(Error::Config { .. })), "{bad}");
        }
        let missing = tempfile::tempdir()?;
        assert!(matches!(
            CfgNames::load(missing.path()),
            Err(Error::Config { .. })
        ));
        Ok(())
    }
}
