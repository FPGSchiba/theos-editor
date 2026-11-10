//! The in-world text of a record (spec §5.11): the parts of a record that the lore step reads as
//! spoken or believed by someone in the story, as spans of the record.
//!
//! A segment is a span of the record. Its text is the record's text in that span; what does not
//! count (design notes, voice titles, headings of a dialogue, link markup) is left out by cutting
//! the span in two, never by editing the text.

use crate::body::{BlockKind, CalloutTitle, Link};
use crate::index::rows::{SegmentRow, ValueRow};
use crate::kinds::KindSpec;
use crate::record::Record;
use crate::text::Span;

/// The in-world segments of `record`, sorted, merged and non-empty. `values` are the front-matter
/// rows of the record. A record of an unknown kind is judged as prose, so its voice blocks count; a
/// verbatim grammar has none.
pub(crate) fn segments(
    record: &Record,
    spec: Option<&KindSpec>,
    values: &[ValueRow],
) -> Vec<SegmentRow> {
    if spec.is_some_and(|spec| spec.body.verbatim) {
        return Vec::new();
    }
    let kind = spec.map_or("", |spec| spec.name);
    let body = record.body();
    let mut included: Vec<Span> = body
        .callouts()
        .iter()
        .filter(|callout| callout.name == "voice")
        .map(|callout| callout.content_span)
        .collect();
    match kind {
        "dialogue" => included.push(record.body_span()),
        "topic" if is_in_world_track(values) => included.push(record.body_span()),
        "belief" => {
            if let Some(title) = top_value(values, "title").filter(|v| v.text.is_some()) {
                included.push(title.span);
            }
        }
        _ => {}
    }

    let mut holes: Vec<Span> = body.design_notes().iter().map(|note| note.span).collect();
    for callout in body.callouts() {
        if callout.name == "voice" && matches!(callout.title, Some(CalloutTitle::Voice(_))) {
            holes.push(Span::new(callout.span.start, callout.content_span.start));
        }
    }
    if kind == "dialogue" {
        holes.extend(
            body.blocks()
                .iter()
                .filter(|block| block.kind == BlockKind::Heading)
                .filter(|block| {
                    block
                        .span
                        .get(record.text())
                        .is_some_and(is_level_two_heading)
                })
                .map(|block| block.span),
        );
    }
    for link in body.links() {
        holes.extend(link_markup(link));
    }

    cut(&merge(included), &merge(holes))
        .into_iter()
        .filter_map(|span| {
            let text = span.get(record.text())?.to_owned();
            let position = record.index().position(span.start).ok()?;
            Some(SegmentRow {
                span,
                position,
                text,
            })
        })
        .collect()
}

/// The two spans of a link that are markup: for a link with shown text everything around the
/// shown text, for a bare link only `[[` and `]]`.
fn link_markup(link: &Link) -> [Span; 2] {
    let Link {
        span, shown_span, ..
    } = *link;
    match shown_span {
        Some(shown) => [
            Span::new(span.start, shown.start),
            Span::new(shown.end, span.end),
        ],
        None => [
            Span::new(span.start, (span.start + 2).min(span.end)),
            Span::new(span.end.saturating_sub(2).max(span.start), span.end),
        ],
    }
}

fn top_value<'a>(values: &'a [ValueRow], field: &str) -> Option<&'a ValueRow> {
    values
        .iter()
        .find(|v| v.field == field && v.entry.is_none() && v.item.is_none())
}

/// Whether the `track` of the record is a valid `in-world`.
fn is_in_world_track(values: &[ValueRow]) -> bool {
    top_value(values, "track").is_some_and(|v| v.valid && v.text.as_deref() == Some("in-world"))
}

/// Whether a heading block is level 2: ATX `## `, or setext with a `-` underline.
fn is_level_two_heading(text: &str) -> bool {
    let first = text
        .lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches(' ');
    if let Some(rest) = first.strip_prefix("##") {
        return rest.is_empty() || rest.starts_with([' ', '\t']);
    }
    text.lines()
        .nth(1)
        .map(str::trim)
        .is_some_and(|line| !line.is_empty() && line.chars().all(|c| c == '-'))
}

/// The spans sorted, empty ones dropped, overlapping and adjacent ones joined.
fn merge(mut spans: Vec<Span>) -> Vec<Span> {
    spans.retain(|span| !span.is_empty());
    spans.sort();
    let mut merged: Vec<Span> = Vec::new();
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

/// The parts of `included` that no span of `holes` covers; both merged.
fn cut(included: &[Span], holes: &[Span]) -> Vec<Span> {
    let mut out = Vec::new();
    for span in included {
        let mut start = span.start;
        for hole in holes
            .iter()
            .filter(|hole| hole.end > span.start && hole.start < span.end)
        {
            if hole.start > start {
                out.push(Span::new(start, hole.start));
            }
            start = start.max(hole.end);
        }
        if start < span.end {
            out.push(Span::new(start, span.end));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::check::check_record;
    use crate::parts::builtin;

    type Res = Result<(), Box<dyn std::error::Error>>;

    /// The texts of the segments of a record of `kind` with the given front-matter lines.
    fn texts(
        path: &str,
        kind: &str,
        rest: &str,
        body: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let registry = builtin()?;
        let text = format!(
            "---\n# synthetic fixture (P04), not canon\nid: x\nkind: {kind}\n{rest}---\n{body}"
        );
        let checked = check_record(&registry, path, text.as_bytes());
        let found = crate::index::rows::rows(path, checked, &registry);
        for segment in &found.inworld {
            assert_eq!(segment.span.get(&text), Some(segment.text.as_str()));
        }
        Ok(found.inworld.into_iter().map(|s| s.text).collect())
    }

    #[test]
    fn segments_hold_voice_content_and_caption_but_not_the_title() -> Res {
        let got = texts(
            "story/scripts/x.md",
            "script",
            "title: t\n",
            "> [!voice] [[synthetic-speaker]] · T1\n> Spoken words.\n>\n> *caption*\n\nplain prose\n",
        )?;
        assert_eq!(got.len(), 1);
        assert!(got[0].contains("Spoken words.") && got[0].contains("caption"));
        assert!(!got[0].contains("[!voice]") && !got[0].contains("plain prose"));
        Ok(())
    }

    #[test]
    fn segments_skip_design_notes_inside_a_voice_block() -> Res {
        let got = texts(
            "story/scripts/x.md",
            "script",
            "title: t\n",
            "> [!voice] [[synthetic-speaker]] · T1\n> Before.\n>\n> Design note: hidden remark.\n",
        )?;
        let joined = got.join("|");
        assert!(joined.contains("Before."));
        assert!(!joined.contains("hidden remark") && !joined.contains("Design note"));
        Ok(())
    }

    #[test]
    fn segments_cover_an_in_world_topic_body_once() -> Res {
        let got = texts(
            "codex/x-in-world.md",
            "topic",
            "title: t\ntrack: in-world\n",
            "Intro line.\n\n> [!voice] [[synthetic-speaker]] · T1\n> Voiced.\n",
        )?;
        assert_eq!(got.iter().filter(|t| t.contains("Voiced.")).count(), 1);
        assert!(got.iter().any(|t| t.contains("Intro line.")));
        assert!(!got.iter().any(|t| t.contains("[!voice]")));
        let explaining = texts(
            "codex/x-explaining.md",
            "topic",
            "title: t\ntrack: explaining\n",
            "Plain.\n",
        )?;
        assert_eq!(explaining, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn segments_hold_a_belief_title() -> Res {
        let got = texts(
            "codex/x.md",
            "belief",
            "title: The synthetic belief\n",
            "Body text.\n",
        )?;
        assert_eq!(got, ["The synthetic belief"]);
        Ok(())
    }

    #[test]
    fn segments_skip_dialogue_level_two_headings() -> Res {
        let got = texts(
            "story/dialogues/x.md",
            "dialogue",
            "title: t\n",
            "Opening.\n\n## Atx heading\n\nMiddle.\n\nSetext heading\n---\n\n### Level three\n\nEnd.\n",
        )?;
        let joined = got.join("|");
        assert!(
            joined.contains("Opening.") && joined.contains("Middle.") && joined.contains("End.")
        );
        assert!(!joined.contains("Atx heading") && !joined.contains("Setext heading"));
        assert!(joined.contains("Level three"));
        Ok(())
    }

    #[test]
    fn segments_keep_shown_text_and_bare_ids_but_not_link_markup() -> Res {
        let got = texts(
            "story/dialogues/x.md",
            "dialogue",
            "title: t\n",
            "See [[synthetic-a|the shown]] and [[synthetic-b]] now.\n",
        )?;
        assert_eq!(got.concat(), "See the shown and synthetic-b now.\n");
        Ok(())
    }

    #[test]
    fn an_unknown_kind_keeps_its_voice_segments() -> Res {
        let body = "> [!voice] [[synthetic-speaker]] · T1
> Spoken words.
>
> Design note: hidden.

plain
";
        let got = texts(
            "codex/x.md",
            "nonsense",
            "title: t
",
            body,
        )?;
        assert_eq!(got.len(), 1);
        assert!(got[0].contains("Spoken words.") && !got[0].contains("hidden"));
        assert!(!got[0].contains("[!voice]") && !got[0].contains("plain"));
        let broken = texts(
            "codex/x.md",
            "npc",
            "title: &a b
",
            body,
        )?;
        assert_eq!(broken.len(), 1);
        assert!(broken[0].contains("Spoken words."));
        Ok(())
    }

    #[test]
    fn a_verbatim_body_has_no_segments() -> Res {
        let got = texts(
            "codex/x.md",
            "equation",
            "title: t\n",
            "> [!voice] [[synthetic-speaker]] · T1\n> Words.\n",
        )?;
        assert_eq!(got, Vec::<String>::new());
        Ok(())
    }
}
