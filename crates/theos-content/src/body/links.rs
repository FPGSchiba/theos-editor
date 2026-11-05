//! Wikilinks `[[target#fragment|shown]]` with exact spans.

use pulldown_cmark::{Event, LinkType, Tag};

use super::events::Events;
use super::malformed::{Malformed, MalformedKind};
use crate::text::{Span, TextIndex};

/// A well-formed wikilink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The whole `[[…]]`.
    pub span: Span,
    /// The record id it points to.
    pub target: String,
    /// The bytes of `target`.
    pub target_span: Span,
    /// The digits after `#`, if any.
    pub fragment: Option<String>,
    /// The bytes of `fragment`.
    pub fragment_span: Option<Span>,
    /// The text after `|`, if any.
    pub shown: Option<String>,
    /// The bytes of `shown`.
    pub shown_span: Option<Span>,
    /// Index of the top-level block the link sits in.
    pub block: usize,
}

impl Link {
    /// A link that only occupies `span`; it stands in for a reported fault in a shape check and is
    /// never part of a result.
    fn stand_in(span: Span) -> Link {
        Link {
            span,
            target: String::new(),
            target_span: span,
            fragment: None,
            fragment_span: None,
            shown: None,
            shown_span: None,
            block: 0,
        }
    }

    /// The fragment as a beat number.
    #[must_use]
    pub fn beat(&self) -> Option<u32> {
        self.fragment.as_deref().and_then(|f| f.parse().ok())
    }

    /// The link written out again: `[[target#fragment|shown]]`.
    #[must_use]
    pub fn spelling(&self) -> String {
        let mut text = format!("[[{}", self.target);
        if let Some(fragment) = &self.fragment {
            text.push('#');
            text.push_str(fragment);
        }
        if let Some(shown) = &self.shown {
            text.push('|');
            text.push_str(shown);
        }
        text.push_str("]]");
        text
    }
}

/// The links of `inside` with a stand-in for every fault in `faults`, in document order: the shape
/// check of a grammar then treats a link that is already reported as a well-formed one.
pub(crate) fn with_stand_ins(inside: &[Link], faults: &[Span]) -> Vec<Link> {
    let mut all: Vec<Link> = inside
        .iter()
        .cloned()
        .chain(faults.iter().map(|&span| Link::stand_in(span)))
        .collect();
    all.sort_by_key(|link| link.span.start);
    all
}

fn valid_target(target: &str) -> bool {
    !target.is_empty()
        && target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn valid_fragment(fragment: &str) -> bool {
    !fragment.is_empty()
        && fragment.chars().all(|c| c.is_ascii_digit())
        && fragment.parse::<u32>().is_ok()
}

/// Reads the wikilinks of a body: the well-formed ones and a report for each bad target.
pub(crate) fn scan(index: &TextIndex, events: &Events<'_>) -> (Vec<Link>, Vec<Malformed>) {
    let mut links = Vec::new();
    let mut bad = Vec::new();
    for ((event, span), &block) in events.items.iter().zip(&events.block_of) {
        let Event::Start(Tag::Link {
            link_type: LinkType::WikiLink { has_pothole },
            dest_url,
            ..
        }) = event
        else {
            continue;
        };
        let (target, fragment) = match dest_url.split_once('#') {
            Some((target, fragment)) => (target, Some(fragment)),
            None => (&**dest_url, None),
        };
        if !valid_target(target) || !fragment.is_none_or(valid_fragment) {
            bad.push(Malformed::new(
                index,
                MalformedKind::LinkTarget,
                format!("`{dest_url}` is not a record id with an optional beat number"),
                *span,
            ));
            continue;
        }
        let target_start = span.start + 2;
        let target_span = Span::new(target_start, target_start + target.len());
        let fragment_span = fragment.map(|f| {
            let start = target_span.end + 1;
            Span::new(start, start + f.len())
        });
        let dest_end = target_start + dest_url.len();
        let shown_span = has_pothole.then(|| Span::new(dest_end + 1, span.end.saturating_sub(2)));
        let shown = shown_span
            .and_then(|s| s.get(events.source))
            .map(str::to_owned);
        links.push(Link {
            span: *span,
            target: target.to_owned(),
            target_span,
            fragment: fragment.map(str::to_owned),
            fragment_span,
            shown,
            shown_span,
            block,
        });
    }
    (links, bad)
}
