//! The validation settings of a project (spec §5.12): the `[validate]` table of
//! `.theos/settings.toml`. [`parse`] never fails: every fault is a [`SettingsProblem`] with the
//! rule that reports it and the span it sits at, and has no effect on the [`ValidationSettings`].
//!
//! The file is read with `toml`'s span-keeping parser and walked by hand, so that each problem
//! has its own position (L14). Other tables of the file belong to other parts and are not read.

mod rules;

use std::collections::BTreeMap;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

pub(crate) use rules::RULES;

use crate::finding::Severity;
use crate::lore::vocab::{self, DeveloperWords};
use crate::rules::{RuleClass, RuleInfo, is_guardrail};
use crate::text::Span;

/// The project-relative path of the settings file.
pub const SETTINGS: &str = ".theos/settings.toml";

/// What `[validate]` configures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationSettings {
    /// The severity per configurable rule. Never holds a guardrail or a fixed rule.
    pub severities: BTreeMap<String, Severity>,
    /// The project's developer words, one word each.
    pub words: Vec<String>,
    /// The words that are allowed in in-world text, each with its reason.
    pub allow: Vec<Allowed>,
}

/// One allowlisted word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    /// The word.
    pub word: String,
    /// Why it is allowed; never empty.
    pub reason: String,
}

/// One fault of the settings file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsProblem {
    /// The `settings/…` rule that reports it.
    pub rule: &'static str,
    /// Where it sits in the file.
    pub span: Span,
    /// What is wrong.
    pub message: String,
}

/// The result of reading a settings file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// What the valid entries configure.
    pub settings: ValidationSettings,
    /// The faults; none of them changed `settings`.
    pub problems: Vec<SettingsProblem>,
}

const TOML: &str = "settings/toml";
const UNKNOWN_KEY: &str = "settings/unknown-key";
const GUARDRAIL: &str = "settings/guardrail";
const FIXED: &str = "settings/fixed";
const UNKNOWN_RULE: &str = "settings/unknown-rule";
const SEVERITY: &str = "settings/severity";
const ALLOW_REASON: &str = "settings/allow-reason";
const WORD: &str = "settings/word";

/// Reads the settings in `text`, which is none when the file is not UTF-8 (L17). `rules` is the
/// rule table: it tells which rule a `severity` entry may name.
///
/// A bad entry gives a problem and no setting. A file that is not valid TOML, or a value of the
/// wrong type, gives empty settings (`settings/toml`).
#[must_use]
pub fn parse(text: Option<&str>, rules: &[RuleInfo]) -> Parsed {
    parse_with(text, rules, vocab::SIZE_LIMIT)
}

/// [`parse`] with the size limit of the compiled word list, which a test can lower.
fn parse_with(text: Option<&str>, rules: &[RuleInfo], word_limit: usize) -> Parsed {
    let mut walk = Walk {
        rules,
        word_limit,
        out: Parsed::default(),
    };
    match text {
        None => walk.problem(
            TOML,
            Span::default(),
            "the file is not valid UTF-8 text.".to_owned(),
        ),
        Some(text) => match DeTable::parse(text) {
            Err(error) => {
                let range = error.span().unwrap_or(0..0);
                let span = Span {
                    start: range.start,
                    end: range.end,
                };
                walk.problem(TOML, span, error.message().to_owned());
            }
            Ok(root) => walk.root(root.get_ref()),
        },
    }
    // A value of the wrong shape makes `[validate]` empty (spec 5.12); the other problems only
    // drop their own entry.
    if walk.out.problems.iter().any(|problem| problem.rule == TOML) {
        walk.out.settings = ValidationSettings::default();
    }
    // `toml` keeps a table's keys sorted, not in file order.
    walk.out.problems.sort_by_key(|problem| problem.span.start);
    walk.out
}

/// The state of one walk over the parsed file.
struct Walk<'r> {
    rules: &'r [RuleInfo],
    word_limit: usize,
    out: Parsed,
}

type Entry<'a> = (
    &'a Spanned<std::borrow::Cow<'a, str>>,
    &'a Spanned<DeValue<'a>>,
);

impl Walk<'_> {
    fn problem(&mut self, rule: &'static str, span: Span, message: String) {
        self.out.problems.push(SettingsProblem {
            rule,
            span,
            message,
        });
    }

    /// `[validate]`, if the file has it.
    fn root(&mut self, root: &DeTable<'_>) {
        let Some(validate) = root.get("validate") else {
            return;
        };
        let Some(table) = validate.get_ref().as_table() else {
            self.problem(
                TOML,
                span_of(validate),
                "[validate] must be a table.".to_owned(),
            );
            return;
        };
        for (key, value) in table {
            self.validate_entry((key, value));
        }
    }

    fn validate_entry(&mut self, (key, value): Entry<'_>) {
        match key.get_ref().as_ref() {
            "severity" => self.severity_table(value),
            "inworld-vocab" => self.vocab_table(value),
            other => self.unknown_key(key, other, "[validate]", "severity and inworld-vocab"),
        }
    }

    /// `name` is not a key of `owner`, whose keys are `known`.
    fn unknown_key(
        &mut self,
        key: &Spanned<std::borrow::Cow<'_, str>>,
        name: &str,
        owner: &str,
        known: &str,
    ) {
        self.problem(
            UNKNOWN_KEY,
            span_of(key),
            format!("`{name}` is not a key of {owner}; its keys are {known}."),
        );
    }

    /// `[validate.severity]`: rule name to severity.
    fn severity_table(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(table) = value.get_ref().as_table() else {
            self.shape(
                value,
                "[validate.severity] must be a table of rule = severity.",
            );
            return;
        };
        for (key, value) in table {
            self.severity_entry(key, value);
        }
    }

    fn severity_entry(
        &mut self,
        key: &Spanned<std::borrow::Cow<'_, str>>,
        value: &Spanned<DeValue<'_>>,
    ) {
        let name = key.get_ref().as_ref();
        let class = self.class_of(name);
        match class {
            Some(RuleClass::Guardrail) => {
                self.problem(
                    GUARDRAIL,
                    span_of(key),
                    format!("{name} is a guardrail: it always blocks, and no setting lowers it."),
                );
            }
            Some(RuleClass::Fixed) => self.problem(
                FIXED,
                span_of(key),
                format!("{name} has no setting: it keeps its severity."),
            ),
            None => self.problem(UNKNOWN_RULE, span_of(key), format!("{name} is not a rule.")),
            Some(RuleClass::Configurable) => {
                let Some(text) = value.get_ref().as_str() else {
                    self.shape(value, &format!("the severity of {name} must be a string."));
                    return;
                };
                if let Some(severity) = Severity::parse(text) {
                    self.out
                        .settings
                        .severities
                        .insert(name.to_owned(), severity);
                } else {
                    self.problem(
                        SEVERITY,
                        span_of(value),
                        format!("the severity of {name} must be \"error\" or \"warning\"."),
                    );
                }
            }
        }
    }

    /// The class a `severity` entry for `name` meets: a guardrail by name even when its rule is
    /// not registered, none for a name that is no rule.
    fn class_of(&self, name: &str) -> Option<RuleClass> {
        if is_guardrail(name) {
            return Some(RuleClass::Guardrail);
        }
        self.rules
            .iter()
            .find(|rule| rule.id == name)
            .map(|rule| rule.class)
    }

    /// `[validate.inworld-vocab]`: `words` and `allow`.
    fn vocab_table(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(table) = value.get_ref().as_table() else {
            self.shape(value, "[validate.inworld-vocab] must be a table.");
            return;
        };
        for (key, value) in table {
            match key.get_ref().as_ref() {
                "words" => self.words(value),
                "allow" => self.allow(value),
                other => {
                    self.unknown_key(key, other, "[validate.inworld-vocab]", "words and allow");
                }
            }
        }
    }

    fn words(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(items) = value.get_ref().as_array() else {
            self.shape(value, "words must be an array of strings.");
            return;
        };
        for item in items {
            match item.get_ref().as_str() {
                None => self.shape(item, "a word must be a string."),
                Some(text) => {
                    if let Some(word) = self.one_word(text, item) {
                        self.out.settings.words.push(word);
                    }
                }
            }
        }
        self.check_word_list(value);
    }

    /// The words must compile into one pattern; a list too large for it is a `settings/word`
    /// problem at the list. The words stay: the lore step falls back to the built-in words.
    fn check_word_list(&mut self, at: &Spanned<DeValue<'_>>) {
        let only_words = ValidationSettings {
            words: self.out.settings.words.clone(),
            ..ValidationSettings::default()
        };
        if let Err(error) = DeveloperWords::with_size_limit(&only_words, self.word_limit) {
            tracing::error!(%error, "the project's word list does not compile");
            self.problem(
                WORD,
                span_of(at),
                "the word list is too large to match; split or shorten it. Until then only the built-in words are checked."
                    .to_owned(),
            );
        }
    }

    /// `text` trimmed when it is one word, else a `settings/word` problem at `at`.
    fn one_word(&mut self, text: &str, at: &Spanned<DeValue<'_>>) -> Option<String> {
        let mut parts = text.split_whitespace();
        if let (Some(word), None) = (parts.next(), parts.next()) {
            let edge = |c: char| c.is_alphanumeric();
            if word.chars().next().is_some_and(edge) && word.chars().next_back().is_some_and(edge) {
                return Some(word.to_owned());
            }
            self.problem(
                WORD,
                span_of(at),
                format!("\"{word}\" cannot be matched as a whole word: a word starts and ends with a letter or a digit."),
            );
            return None;
        }
        self.problem(
            WORD,
            span_of(at),
            format!("\"{text}\" is not one word: a word is not empty and has no spaces."),
        );
        None
    }

    fn allow(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(items) = value.get_ref().as_array() else {
            self.shape(value, "allow must be an array of { word, reason } tables.");
            return;
        };
        for item in items {
            self.allow_entry(item);
        }
    }

    fn allow_entry(&mut self, item: &Spanned<DeValue<'_>>) {
        let Some(table) = item.get_ref().as_table() else {
            self.shape(item, "an allow entry must be a { word, reason } table.");
            return;
        };
        let mut word = None;
        let mut reason = None;
        let mut unknown = false;
        for (key, value) in table {
            match key.get_ref().as_ref() {
                "word" => word = Some(value),
                "reason" => reason = Some(value),
                other => {
                    unknown = true;
                    self.unknown_key(key, other, "an allow entry", "word and reason");
                }
            }
        }
        let word = match word {
            None => {
                self.problem(
                    WORD,
                    span_of(item),
                    "an allow entry needs a word.".to_owned(),
                );
                None
            }
            Some(value) => {
                if let Some(text) = value.get_ref().as_str() {
                    self.one_word(text, value)
                } else {
                    self.shape(value, "the word of an allow entry must be a string.");
                    None
                }
            }
        };
        let reason = match reason.map(|value| (value, value.get_ref().as_str())) {
            None => None,
            Some((_, Some(text))) => Some(text.trim()).filter(|text| !text.is_empty()),
            Some((value, None)) => {
                // A shape problem: no second finding for the same value.
                self.shape(value, "the reason of an allow entry must be a string.");
                return;
            }
        };
        if reason.is_none() {
            self.problem(
                ALLOW_REASON,
                span_of(item),
                "an allow entry needs a reason that says why the word is allowed.".to_owned(),
            );
        }
        if unknown {
            return;
        }
        if let (Some(word), Some(reason)) = (word, reason) {
            self.out.settings.allow.push(Allowed {
                word,
                reason: reason.to_owned(),
            });
        }
    }

    /// A value of the wrong shape: `settings/toml` at the value.
    fn shape(&mut self, at: &Spanned<DeValue<'_>>, message: &str) {
        self.problem(TOML, span_of(at), message.to_owned());
    }
}

/// The span of a spanned item.
fn span_of<T>(item: &Spanned<T>) -> Span {
    let range = item.span();
    Span {
        start: range.start,
        end: range.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::GUARDRAILS;

    fn table() -> Vec<RuleInfo> {
        let rule = |id, severity, class| RuleInfo {
            id,
            severity,
            class,
        };
        vec![
            rule("lore/never-confirm", Severity::Error, RuleClass::Guardrail),
            rule(
                "lore/three-clue",
                Severity::Warning,
                RuleClass::Configurable,
            ),
            rule(
                "lore/reveal-order",
                Severity::Error,
                RuleClass::Configurable,
            ),
            rule("link/dangling", Severity::Error, RuleClass::Fixed),
        ]
    }

    fn read(text: &str) -> Parsed {
        parse(Some(text), &table())
    }

    /// The `(rule, text at the span)` of each problem.
    fn at(text: &str) -> Vec<(&'static str, String)> {
        read(text)
            .problems
            .iter()
            .map(|p| (p.rule, text[p.span.start..p.span.end].to_owned()))
            .collect()
    }

    #[test]
    fn a_valid_file_configures_and_has_no_problem() {
        let parsed = read(
            "[other]\nx = 1\n[validate.severity]\n\"lore/three-clue\" = \"error\"\n\
             \"lore/reveal-order\" = \"warning\"\n[validate.inworld-vocab]\nwords = [\" w1 \", \"w2\"]\n\
             allow = [{ word = \"T1\", reason = \" because \" }]\n",
        );
        assert_eq!(parsed.problems, []);
        assert_eq!(
            parsed.settings.severities,
            BTreeMap::from([
                ("lore/three-clue".to_owned(), Severity::Error),
                ("lore/reveal-order".to_owned(), Severity::Warning),
            ])
        );
        assert_eq!(parsed.settings.words, ["w1", "w2"]);
        assert_eq!(
            parsed.settings.allow,
            [Allowed {
                word: "T1".to_owned(),
                reason: "because".to_owned()
            }]
        );
    }

    #[test]
    fn no_validate_table_is_empty_settings() {
        assert_eq!(read(""), Parsed::default());
        assert_eq!(read("[generators]\nx = 1\n"), Parsed::default());
    }

    #[test]
    fn invalid_toml_is_one_problem_at_its_position_and_empty_settings() {
        let text = "[validate.severity]\n\"lore/three-clue\" = \n";
        let parsed = read(text);
        assert_eq!(parsed.settings, ValidationSettings::default());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].rule, "settings/toml");
        assert!(parsed.problems[0].span.start >= 20, "{:?}", parsed.problems);
    }

    #[test]
    fn a_file_that_is_not_text_is_a_problem_at_the_start() {
        let parsed = parse(None, &table());
        assert_eq!(parsed.settings, ValidationSettings::default());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].rule, "settings/toml");
        assert_eq!(parsed.problems[0].span, Span::default());
    }

    #[test]
    fn a_validate_that_is_not_a_table_is_a_toml_problem() {
        assert_eq!(at("validate = 3\n"), [("settings/toml", "3".to_owned())]);
    }

    #[test]
    fn wrongly_shaped_values_are_toml_problems() {
        let text = "[validate]\nseverity = 1\ninworld-vocab = 2\n";
        assert_eq!(
            at(text),
            [
                ("settings/toml", "1".to_owned()),
                ("settings/toml", "2".to_owned())
            ]
        );
        let text = "[validate.inworld-vocab]\nwords = \"a\"\nallow = 4\n";
        assert_eq!(
            at(text),
            [
                ("settings/toml", "\"a\"".to_owned()),
                ("settings/toml", "4".to_owned())
            ]
        );
        let text = "[validate.inworld-vocab]\nwords = [1]\nallow = [2]\n";
        assert_eq!(
            at(text),
            [
                ("settings/toml", "1".to_owned()),
                ("settings/toml", "2".to_owned())
            ]
        );
    }

    #[test]
    fn a_shape_problem_empties_the_settings() {
        // Each case is next to a valid entry that would otherwise stay.
        let valid = "[validate.severity]\n\"lore/three-clue\" = \"error\"\n";
        let cases = [
            "[validate.inworld-vocab]\nwords = \"a\"\n",
            "[validate.inworld-vocab]\nwords = [1]\n",
            "[validate.inworld-vocab]\nallow = 4\n",
            "[validate.inworld-vocab]\nallow = [2]\n",
            "[validate.inworld-vocab]\nallow = [{ word = 1, reason = \"r\" }]\n",
            "[validate.inworld-vocab]\nallow = [{ word = \"a\", reason = 1 }]\n",
            "[validate.inworld-vocab]\nwords = [\"ok\"]\nallow = [{ word = \"a\", reason = 1 }]\n",
            "[validate]\ninworld-vocab = 2\n",
            "[validate.severity]\n\"lore/reveal-order\" = 3\n",
        ];
        for case in cases {
            let parsed = read(&format!("{valid}{case}"));
            assert_eq!(parsed.settings, ValidationSettings::default(), "{case}");
            assert!(
                parsed.problems.iter().any(|p| p.rule == "settings/toml"),
                "{case}: {:?}",
                parsed.problems
            );
        }
        for text in [
            "validate = 3\n",
            "[validate]\nseverity = 1\n",
            "[validate\n",
        ] {
            let parsed = read(text);
            assert_eq!(parsed.settings, ValidationSettings::default(), "{text}");
            assert_eq!(parsed.problems[0].rule, "settings/toml", "{text}");
        }
        // The other rows drop only their own entry.
        let parsed = read(&format!(
            "{valid}[validate.inworld-vocab]\nwords = [\"ok\", \"a b\"]\n"
        ));
        assert_eq!(parsed.settings.severities.len(), 1);
        assert_eq!(parsed.settings.words, ["ok"]);
    }

    #[test]
    fn a_word_must_start_and_end_with_a_letter_or_a_digit() {
        let text = "[validate.inworld-vocab]
words = [\"c++\", \"-x\", \"ok-1\", \"ünï\"]
                    allow = [{ word = \"x-\", reason = \"r\" }, { word = \"T1\", reason = \"r\" }]
";
        assert_eq!(
            at(text),
            [
                ("settings/word", "\"c++\"".to_owned()),
                ("settings/word", "\"-x\"".to_owned()),
                ("settings/word", "\"x-\"".to_owned()),
            ]
        );
        let parsed = read(text);
        assert_eq!(parsed.settings.words, ["ok-1", "ünï"]);
        assert_eq!(parsed.settings.allow.len(), 1);
    }

    #[test]
    fn a_word_list_too_large_to_match_is_a_problem_at_the_list() {
        let text = "[validate.inworld-vocab]
words = [\"alpha\", \"beta\"]
";
        let parsed = parse_with(Some(text), &table(), 10);
        assert_eq!(parsed.problems.len(), 1, "{:?}", parsed.problems);
        assert_eq!(parsed.problems[0].rule, "settings/word");
        assert!(parsed.problems[0].message.contains("too large"));
        let at = &text[parsed.problems[0].span.start..parsed.problems[0].span.end];
        assert!(at.starts_with('[') && at.ends_with(']'));
        // A small list is fine.
        let small = read(
            "[validate.inworld-vocab]
words = [\"a\"]
",
        );
        assert_eq!(small.problems, vec![]);
    }

    #[test]
    fn an_unknown_key_is_reported_at_the_key() {
        let text = "[validate]\nfoo = 1\n[validate.inworld_vocab]\nwords = [\"a\"]\n\
                    [validate.inworld-vocab]\nbar = 1\nallow = [{ word = \"a\", reason = \"r\", why = 1 }]\n";
        assert_eq!(
            at(text),
            [
                ("settings/unknown-key", "foo".to_owned()),
                ("settings/unknown-key", "inworld_vocab".to_owned()),
                ("settings/unknown-key", "bar".to_owned()),
                ("settings/unknown-key", "why".to_owned()),
            ]
        );
        assert_eq!(read(text).settings.words, Vec::<String>::new());
        // An allow entry with an unknown key is a bad entry: its word is not allowed.
        assert_eq!(read(text).settings.allow, Vec::new());
    }

    #[test]
    fn a_guardrail_is_reported_at_the_key_and_has_no_effect() {
        let text = "[validate.severity]\n\"lore/never-confirm\" = \"warning\"\n\
                    \"lore/inworld-vocab\" = \"warning\"\n";
        let parsed = read(text);
        // `lore/inworld-vocab` is not in the table, yet a guardrail by name.
        assert_eq!(
            at(text),
            [
                ("settings/guardrail", "\"lore/never-confirm\"".to_owned()),
                ("settings/guardrail", "\"lore/inworld-vocab\"".to_owned()),
            ]
        );
        assert_eq!(
            parsed.problems[0].message,
            "lore/never-confirm is a guardrail: it always blocks, and no setting lowers it."
        );
        assert!(parsed.settings.severities.is_empty());
    }

    #[test]
    fn guardrails_never_enter_the_severities() {
        // One file per value: duplicate keys are invalid TOML.
        let mut all = Parsed::default();
        for name in GUARDRAILS {
            for value in ["error", "warning", "off"] {
                let one = read(&format!("[validate.severity]\n\"{name}\" = \"{value}\"\n"));
                assert_eq!(one.problems.len(), 1, "{name} = {value}");
                assert_eq!(one.problems[0].rule, "settings/guardrail");
                all.settings.severities.extend(one.settings.severities);
            }
        }
        assert!(all.settings.severities.is_empty());
    }

    #[test]
    fn a_fixed_rule_has_no_setting() {
        let text = "[validate.severity]\n\"link/dangling\" = \"warning\"\n";
        assert_eq!(
            at(text),
            [("settings/fixed", "\"link/dangling\"".to_owned())]
        );
        assert!(read(text).settings.severities.is_empty());
    }

    #[test]
    fn an_unknown_rule_is_reported_at_the_key() {
        let text = "[validate.severity]\n\"lore/nothing\" = \"warning\"\n";
        assert_eq!(
            at(text),
            [("settings/unknown-rule", "\"lore/nothing\"".to_owned())]
        );
        assert!(read(text).settings.severities.is_empty());
    }

    #[test]
    fn a_value_other_than_error_or_warning_is_reported_at_the_value() {
        let text = "[validate.severity]\n\"lore/three-clue\" = \"off\"\n\"lore/reveal-order\" = \"warning\"\n";
        assert_eq!(at(text), [("settings/severity", "\"off\"".to_owned())]);
        // The other entry is valid and stays.
        assert_eq!(
            read(text).settings.severities,
            BTreeMap::from([("lore/reveal-order".to_owned(), Severity::Warning)])
        );
    }

    #[test]
    fn an_allow_entry_without_a_reason_is_reported_at_the_entry() {
        let text = "[validate.inworld-vocab]\nallow = [\n  { word = \"a\" },\n  \
                    { word = \"b\", reason = \"  \" },\n]\n";
        assert_eq!(
            at(text),
            [
                ("settings/allow-reason", "{ word = \"a\" }".to_owned()),
                (
                    "settings/allow-reason",
                    "{ word = \"b\", reason = \"  \" }".to_owned()
                ),
            ]
        );
        assert_eq!(read(text).settings.allow, Vec::new());
    }

    #[test]
    fn a_bad_word_is_reported_at_the_item_and_dropped() {
        let text = "[validate.inworld-vocab]\nwords = [\"two words\", \"\", \"ok\"]\n\
                    allow = [{ word = \"a b\", reason = \"r\" }, { reason = \"r\" }]\n";
        assert_eq!(
            at(text),
            [
                ("settings/word", "\"two words\"".to_owned()),
                ("settings/word", "\"\"".to_owned()),
                ("settings/word", "\"a b\"".to_owned()),
                ("settings/word", "{ reason = \"r\" }".to_owned()),
            ]
        );
        let settings = read(text).settings;
        assert_eq!(settings.words, ["ok"]);
        assert_eq!(settings.allow, Vec::new());
    }
}
