//! Finds the callouts in the shared events and reads their titles through the registry.

use pulldown_cmark::{Event, Tag};

use super::title;
use super::{Callout, CalloutTitle, ContentBlock, Fold, TitleGrammar, TitleInput, grammar};
use crate::body::events::Events;
use crate::body::links::Link;
use crate::body::malformed::{Malformed, covers_link_fault};
use crate::text::{Span, TextIndex};

/// The parts of a `[!type]+` marker, as offsets into the record text.
struct Marker {
    name_span: Span,
    fold: Option<Fold>,
    title_start: usize,
}

/// Reads `[!type]`, an optional fold sign and one optional space at `start`.
fn marker(source: &str, start: usize) -> Option<Marker> {
    let rest = source.get(start..)?.strip_prefix("[!")?;
    let name_len = rest
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_alphabetic() || i > 0 && (c.is_ascii_digit() || c == '-')))
        .map_or(rest.len(), |(i, _)| i);
    if name_len == 0 || !rest[name_len..].starts_with(']') {
        return None;
    }
    let name_start = start + 2;
    let mut at = name_start + name_len + 1;
    let fold = match source.as_bytes().get(at) {
        Some(b'+') => Some(Fold::Open),
        Some(b'-') => Some(Fold::Closed),
        _ => None,
    };
    at += usize::from(fold.is_some());
    at += usize::from(source.as_bytes().get(at) == Some(&b' '));
    Some(Marker {
        name_span: Span::new(name_start, name_start + name_len),
        fold,
        title_start: at,
    })
}

/// The span `start..end` of `source` without the spaces and tabs at both ends.
fn trimmed(source: &str, start: usize, end: usize) -> Span {
    let text = source.get(start..end).unwrap_or_default();
    let head = text.len() - text.trim_start_matches([' ', '\t']).len();
    let kept = text.trim_matches([' ', '\t']).len();
    Span::new(start + head, start + head + kept)
}

/// The index of the `End` that closes the `Start` at `open`.
fn closing(events: &Events<'_>, open: usize) -> usize {
    let mut depth = 0usize;
    for (i, (event, _)) in events.items.iter().enumerate().skip(open) {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    events.items.len().saturating_sub(1)
}

/// The blocks that are direct children of the quote opened at `open`, in document order.
fn child_blocks(events: &Events<'_>, open: usize, close: usize) -> Vec<ContentBlock> {
    let mut depth = 0usize;
    let mut found = Vec::new();
    for (event, span) in &events.items[open + 1..close] {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    found.push(ContentBlock {
                        span: *span,
                        paragraph: matches!(tag, Tag::Paragraph),
                    });
                }
                depth += 1;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Rule if depth == 0 => found.push(ContentBlock {
                span: *span,
                paragraph: false,
            }),
            _ => {}
        }
    }
    found
}

/// The caption: the last line of `last` when it starts with `— ` and is not the title line.
fn caption(source: &str, last: Span, is_title_paragraph: bool) -> Option<Span> {
    let text = last.get(source)?.trim_end_matches('\n');
    let line_start = text.rfind('\n').map_or(0, |at| at + 1);
    if is_title_paragraph && line_start == 0 {
        return None;
    }
    let line = &text[line_start..];
    let shown = line.trim_start_matches(['>', ' ']);
    shown.starts_with("— ").then(|| {
        let start = last.start + line_start + (line.len() - shown.len());
        Span::new(start, last.start + text.len())
    })
}

/// The content blocks: the child blocks without the title line and the caption line.
/// `caption` is the paragraph that ends in the caption line with that line.
fn content_blocks(
    source: &str,
    blocks: &[ContentBlock],
    first: Span,
    title_end: usize,
    caption: Option<(Span, Span)>,
) -> Vec<ContentBlock> {
    blocks
        .iter()
        .filter_map(|block| {
            let mut span = block.span;
            if span == first {
                span.start = (title_end + 1).min(span.end);
            }
            if let Some((captioned, caption)) = caption
                && block.span == captioned
                && let Some(line_start) = source[..caption.start].rfind('\n').map(|at| at + 1)
            {
                span.end = line_start.min(span.end);
            }
            (span.start < span.end).then_some(ContentBlock { span, ..*block })
        })
        .collect()
}

/// Recognises every callout, parses its title and reports the malformed titles.
pub(crate) fn scan(
    index: &TextIndex,
    events: &Events<'_>,
    links: &[Link],
    reported: &[Malformed],
) -> (Vec<Callout>, Vec<Malformed>) {
    let source = events.source;
    let mut callouts = Vec::new();
    let mut bad = Vec::new();
    let mut depth = 0usize;
    for (i, (event, quote)) in events.items.iter().enumerate() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => depth += 1,
            Event::End(pulldown_cmark::TagEnd::BlockQuote(_)) => {
                depth = depth.saturating_sub(1);
                continue;
            }
            _ => continue,
        }
        let Some((Event::Start(Tag::Paragraph), first)) = events.items.get(i + 1) else {
            continue;
        };
        let Some(marker) = marker(source, first.start) else {
            continue;
        };
        let close = closing(events, i);
        let line_end = first
            .get(source)
            .and_then(|p| p.find('\n'))
            .map_or_else(|| first.end, |at| first.start + at);
        let title_span = trimmed(source, marker.title_start.min(line_end), line_end);
        let content_start = (line_end + 1).min(quote.end);
        let blocks = child_blocks(events, i, close);
        let name = marker
            .name_span
            .get(source)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let registered = grammar(&name);
        let last_paragraph = blocks.iter().rev().find(|block| block.paragraph);
        let caption = last_paragraph
            .filter(|_| registered.is_some_and(TitleGrammar::captioned))
            .and_then(|last| {
                caption(source, last.span, last.span == *first).map(|line| (last.span, line))
            });
        let caption_span = caption.map(|(_, line)| line);
        let content = content_blocks(source, &blocks, *first, line_end, caption);
        let inside: Vec<Link> = links
            .iter()
            .filter(|l| title_span.start <= l.span.start && l.span.end <= title_span.end)
            .cloned()
            .collect();
        let title = match registered {
            None => Some(CalloutTitle::Other),
            Some(grammar) => {
                let tokens = title::split(index, title_span, &inside);
                let input = TitleInput {
                    index,
                    title_span,
                    tokens: &tokens,
                    links: &inside,
                    content: &content,
                    depth,
                };
                match grammar.parse(&input) {
                    Ok(title) => Some(title),
                    Err(report) => {
                        if !covers_link_fault(reported, report.span) {
                            bad.push(report);
                        }
                        None
                    }
                }
            }
        };
        callouts.push(Callout {
            span: *quote,
            depth,
            name,
            name_span: marker.name_span,
            fold: marker.fold,
            title_span,
            content_span: Span::new(content_start, quote.end),
            caption_span,
            title,
        });
    }
    (callouts, bad)
}
