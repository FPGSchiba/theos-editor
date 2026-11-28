//! Word-level diff of two texts, the base of the prose diff.

use std::time::{Duration, Instant};

use similar::{Algorithm, DiffOp, capture_diff_slices_deadline};

use crate::text::Span;

/// The word-level difference between two texts.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WordDiff {
    /// The operations in text order; neighbouring operations are never of the same kind.
    pub ops: Vec<WordOp>,
}

/// One run of words that is kept, removed or added. Spans are byte ranges into the texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordOp {
    /// Words present in both texts.
    Equal {
        /// The words in the old text.
        old: Span,
        /// The words in the new text.
        new: Span,
    },
    /// Words only in the old text.
    Delete {
        /// The words in the old text.
        old: Span,
    },
    /// Words only in the new text.
    Insert {
        /// The words in the new text.
        new: Span,
    },
}

/// How long the diff may search for a minimal result before it settles for a coarser one.
const DEADLINE: Duration = Duration::from_secs(2);

/// Git's `[:space:]`: a no-break space is part of a word.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// The words of `text` with their byte spans.
fn tokens(text: &str) -> Vec<(&str, Span)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (is_space(c), start) {
            (true, Some(s)) => {
                out.push((&text[s..i], Span::new(s, i)));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((&text[s..], Span::new(s, text.len())));
    }
    out
}

/// The span from the first to the last of `toks[index..index + len]`; `len` is at least 1.
fn cover(toks: &[(&str, Span)], index: usize, len: usize) -> Span {
    Span::new(toks[index].1.start, toks[index + len - 1].1.end)
}

/// Appends `op`, merging it into the last one when both are of the same kind.
fn push(ops: &mut Vec<WordOp>, op: WordOp) {
    match (ops.last_mut(), op) {
        (Some(WordOp::Equal { old, new }), WordOp::Equal { old: o, new: n }) => {
            old.end = o.end;
            new.end = n.end;
        }
        (Some(WordOp::Delete { old }), WordOp::Delete { old: o }) => old.end = o.end,
        (Some(WordOp::Insert { new }), WordOp::Insert { new: n }) => new.end = n.end,
        _ => ops.push(op),
    }
}

/// Diffs `old` against `new` word by word.
///
/// A word is a maximal run of characters that are not git's `[:space:]`. Whitespace is never
/// part of a span, so a re-wrapped paragraph shows no change. The search stops after a deadline
/// and then returns a correct but coarser diff.
#[must_use]
pub fn words(old: &str, new: &str) -> WordDiff {
    let (old_toks, new_toks) = (tokens(old), tokens(new));
    let old_words: Vec<&str> = old_toks.iter().map(|t| t.0).collect();
    let new_words: Vec<&str> = new_toks.iter().map(|t| t.0).collect();
    let diff_ops = capture_diff_slices_deadline(
        Algorithm::Myers,
        &old_words,
        &new_words,
        Some(Instant::now() + DEADLINE),
    );
    let mut ops = Vec::new();
    for op in diff_ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } if len > 0 => push(
                &mut ops,
                WordOp::Equal {
                    old: cover(&old_toks, old_index, len),
                    new: cover(&new_toks, new_index, len),
                },
            ),
            DiffOp::Delete {
                old_index, old_len, ..
            } if old_len > 0 => {
                push(
                    &mut ops,
                    WordOp::Delete {
                        old: cover(&old_toks, old_index, old_len),
                    },
                );
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } if new_len > 0 => {
                push(
                    &mut ops,
                    WordOp::Insert {
                        new: cover(&new_toks, new_index, new_len),
                    },
                );
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } if old_len > 0 && new_len > 0 => {
                push(
                    &mut ops,
                    WordOp::Delete {
                        old: cover(&old_toks, old_index, old_len),
                    },
                );
                push(
                    &mut ops,
                    WordOp::Insert {
                        new: cover(&new_toks, new_index, new_len),
                    },
                );
            }
            _ => {}
        }
    }
    WordDiff { ops }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{TestError, TestRunner};

    /// The texts behind each op: `(kind, old text, new text)`.
    fn render<'a>(diff: &WordDiff, old: &'a str, new: &'a str) -> Vec<(char, &'a str, &'a str)> {
        let get = |text: &'a str, span: Span| span.get(text).unwrap_or("<bad span>");
        diff.ops
            .iter()
            .map(|op| match *op {
                WordOp::Equal { old: o, new: n } => ('=', get(old, o), get(new, n)),
                WordOp::Delete { old: o } => ('-', get(old, o), ""),
                WordOp::Insert { new: n } => ('+', "", get(new, n)),
            })
            .collect()
    }

    // AC: CE-21.2
    #[test]
    fn one_changed_word_is_one_delete_and_one_insert() {
        let (old, new) = ("under an open sky.", "under an opened sky.");
        let got = render(&words(old, new), old, new);
        assert_eq!(
            got,
            vec![
                ('=', "under an", "under an"),
                ('-', "open", ""),
                ('+', "", "opened"),
                ('=', "sky.", "sky."),
            ]
        );
    }

    // AC: CE-21.2
    #[test]
    fn a_rewrapped_paragraph_has_no_change() {
        let (old, new) = ("one two three four", "one two\nthree\nfour");
        let diff = words(old, new);
        assert_eq!(diff.ops.len(), 1);
        assert!(matches!(diff.ops[0], WordOp::Equal { .. }));
        assert_eq!(render(&diff, old, new)[0], ('=', old, new));
    }

    // AC: CE-21.2
    #[test]
    fn a_no_break_space_joins_words() {
        let (old, new) = ("a\u{a0}b", "a\u{a0}c");
        let got = render(&words(old, new), old, new);
        assert_eq!(got, vec![('-', "a\u{a0}b", ""), ('+', "", "a\u{a0}c")]);
    }

    // AC: CE-21.2
    #[test]
    fn vertical_tab_and_form_feed_split_words() {
        let (old, new) = ("a\u{b}b\u{c}c", "a\u{b}x\u{c}c");
        let got = render(&words(old, new), old, new);
        assert_eq!(
            got,
            vec![
                ('=', "a", "a"),
                ('-', "b", ""),
                ('+', "", "x"),
                ('=', "c", "c")
            ]
        );
    }

    // AC: CE-21.2
    #[test]
    fn empty_and_whitespace_edges() {
        let text = "two words";
        assert_eq!(render(&words("", text), "", text), vec![('+', "", text)]);
        assert_eq!(render(&words(text, ""), text, ""), vec![('-', text, "")]);
        assert_eq!(words("", ""), WordDiff::default());
        assert_eq!(words(" \n", "\t"), WordDiff::default());
    }

    fn text_strategy() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec!["a", "b", "c", "ab", "d.", "é"]);
        let ws = prop::sample::select(vec![" ", "\n", "  ", "\t", "\r\n", "\u{b}", "\u{c}"]);
        (
            prop::option::of(ws.clone()),
            prop::collection::vec((word, ws), 0..12),
        )
            .prop_map(|(lead, parts)| {
                let mut out = lead.unwrap_or("").to_string();
                for (w, s) in parts {
                    out.push_str(w);
                    out.push_str(s);
                }
                out
            })
    }

    /// Rebuilds `text` from the spans, taking the gaps from `text`; checks the spans on the way.
    fn rebuild(text: &str, spans: &[Span]) -> String {
        let is_ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r');
        let mut out = String::new();
        let mut pos = 0;
        for span in spans {
            assert!(span.start >= pos, "spans overlap or are unordered");
            let word = span.get(text).expect("span on char boundaries");
            assert_ne!(word, "");
            assert!(!word.starts_with(is_ws) && !word.ends_with(is_ws));
            let gap = &text[pos..span.start];
            assert!(gap.chars().all(is_ws), "gap is not whitespace");
            out.push_str(gap);
            out.push_str(word);
            pos = span.end;
        }
        let tail = &text[pos..];
        assert!(tail.chars().all(is_ws));
        out.push_str(tail);
        out
    }

    // AC: CE-21.2
    #[test]
    fn reconstruction() -> Result<(), TestError<(String, String)>> {
        let config = ProptestConfig {
            source_file: Some(file!()),
            ..ProptestConfig::default()
        };
        let mut runner = TestRunner::new(config);
        runner.run(&(text_strategy(), text_strategy()), |(old, new)| {
            let diff = words(&old, &new);
            let new_spans: Vec<Span> = diff
                .ops
                .iter()
                .filter_map(|op| match *op {
                    WordOp::Equal { new, .. } | WordOp::Insert { new } => Some(new),
                    WordOp::Delete { .. } => None,
                })
                .collect();
            let old_spans: Vec<Span> = diff
                .ops
                .iter()
                .filter_map(|op| match *op {
                    WordOp::Equal { old, .. } | WordOp::Delete { old } => Some(old),
                    WordOp::Insert { .. } => None,
                })
                .collect();
            prop_assert_eq!(rebuild(&new, &new_spans), new);
            prop_assert_eq!(rebuild(&old, &old_spans), old);
            for pair in diff.ops.windows(2) {
                prop_assert_ne!(
                    std::mem::discriminant(&pair[0]),
                    std::mem::discriminant(&pair[1])
                );
            }
            Ok(())
        })
    }
}
