//! Developer words (spec §5.11): the words in-world text cannot use. Built in are the ids of the
//! numbered kinds and floors, the tier tokens and the names of the order rules, matched
//! case-sensitively; the project adds words, matched as whole words in any case. The allowlist
//! exempts a word everywhere.

use std::collections::HashSet;
use std::ops::Range;

use regex::Regex;

use crate::settings::ValidationSettings;

/// The built-in developer words (spec §5.11, Jann's ruling §4.1): ids of the numbered kinds with
/// three or more digits, `E` with a number, floor ids, the tiers and the rule names.
pub(crate) const BUILTIN: &str = r"\b(?:[TBCSQ]-[0-9]{3,}|E[0-9]+|T[1-3]|f[0-9]{2}(?:-[a-z0-9]+)*|never-confirm|no-overshoot|inworld-vocab|reveal-order|three-clue)\b";

/// The developer words of a project, ready to match text.
#[derive(Debug)]
pub(crate) struct DeveloperWords {
    builtin: Regex,
    /// None without project words: an empty alternation would match everywhere.
    project: Option<Regex>,
    allow_exact: HashSet<String>,
    allow_folded: HashSet<String>,
}

impl DeveloperWords {
    /// The built-in patterns plus the words and the allowlist of `settings`.
    ///
    /// # Errors
    /// [`regex::Error`] when the patterns do not compile, which only a very large word list
    /// causes.
    pub(crate) fn new(settings: &ValidationSettings) -> Result<DeveloperWords, regex::Error> {
        let words: Vec<String> = settings
            .words
            .iter()
            .filter(|word| !word.is_empty())
            .map(|word| regex::escape(word))
            .collect();
        let project = if words.is_empty() {
            None
        } else {
            Some(Regex::new(&format!(r"(?i)\b(?:{})\b", words.join("|")))?)
        };
        Ok(DeveloperWords {
            builtin: Regex::new(BUILTIN)?,
            project,
            allow_exact: settings.allow.iter().map(|a| a.word.clone()).collect(),
            allow_folded: settings
                .allow
                .iter()
                .map(|a| a.word.to_lowercase())
                .collect(),
        })
    }

    /// The developer words in `text` with their byte ranges, sorted and without overlaps: where
    /// a built-in and a project match overlap, the longer one stays.
    pub(crate) fn find<'t>(&self, text: &'t str) -> Vec<(Range<usize>, &'t str)> {
        let mut found: Vec<(Range<usize>, &'t str)> = self
            .builtin
            .find_iter(text)
            .filter(|m| !self.allow_exact.contains(m.as_str()))
            .map(|m| (m.range(), m.as_str()))
            .collect();
        if let Some(project) = &self.project {
            found.extend(
                project
                    .find_iter(text)
                    .filter(|m| !self.allow_folded.contains(&m.as_str().to_lowercase()))
                    .map(|m| (m.range(), m.as_str())),
            );
        }
        found.sort_by_key(|(range, _)| (range.start, std::cmp::Reverse(range.end)));
        let mut kept: Vec<(Range<usize>, &'t str)> = Vec::with_capacity(found.len());
        for hit in found {
            match kept.last_mut() {
                Some(last) if hit.0.start < last.0.end => {
                    if hit.0.len() > last.0.len() {
                        *last = hit;
                    }
                }
                _ => kept.push(hit),
            }
        }
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Allowed;

    fn settings(words: &[&str], allow: &[&str]) -> ValidationSettings {
        ValidationSettings {
            words: words.iter().map(|w| (*w).to_owned()).collect(),
            allow: allow
                .iter()
                .map(|w| Allowed {
                    word: (*w).to_owned(),
                    reason: "synthetic reason".to_owned(),
                })
                .collect(),
            ..ValidationSettings::default()
        }
    }

    fn words(settings: &ValidationSettings, text: &str) -> Vec<String> {
        let built = DeveloperWords::new(settings);
        assert!(built.is_ok(), "{built:?}");
        built
            .map(|w| {
                w.find(text)
                    .into_iter()
                    .map(|(_, word)| word.to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn built_in_patterns_match_whole_words_case_sensitively() {
        let none = ValidationSettings::default();
        assert_eq!(
            words(&none, "T-950, T2, f04-hall, E12 and never-confirm."),
            ["T-950", "T2", "f04-hall", "E12", "never-confirm"]
        );
        assert_eq!(
            words(&none, "T4 xT2 T22 t2 T-95 the Eel"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn project_words_match_whole_words_in_any_case() {
        let ledger = settings(&["ledger"], &[]);
        assert_eq!(
            words(&ledger, "The Ledger, ledger’s end"),
            ["Ledger", "ledger"]
        );
        assert_eq!(words(&ledger, "a ledgerless hall"), Vec::<String>::new());
        let phrase = settings(&["two words", "a.b"], &[]);
        assert_eq!(
            words(&phrase, "Two Words and a.b but not axb"),
            ["Two Words", "a.b"]
        );
    }

    #[test]
    fn the_allowlist_exempts_by_exact_text_or_case_insensitively() {
        let allowed = settings(&["Ledger"], &["T1", "ledger"]);
        assert_eq!(words(&allowed, "T1 T2 ledger LEDGER"), ["T2"]);
        // A built-in match needs its exact text in the allowlist.
        let lower = settings(&[], &["t2"]);
        assert_eq!(words(&lower, "T2"), ["T2"]);
    }

    #[test]
    fn overlapping_matches_give_one_word_the_longer() {
        let both = settings(&["t-950 and more", "T-950"], &[]);
        assert_eq!(words(&both, "see T-950 and more"), ["T-950 and more"]);
        assert_eq!(words(&both, "see T-950."), ["T-950"]);
    }
}
