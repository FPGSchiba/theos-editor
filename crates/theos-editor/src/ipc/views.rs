//! The view types of the Codex commands: specta mirrors of the plain types of
//! `theos_studio::codex_view` and of the P48 types they embed. Each mirror keeps the serde names
//! of its original, so the JSON the studio emits and the JSON the app returns are the same, and
//! has a `From` impl. Counts are `u32`; a larger value saturates.

use serde::{Deserialize, Serialize};
use specta::Type;
use theos_studio::codex_view as studio;
use theos_studio::marks;
use theos_studio::views::check;
use theos_studio::views::codex as p48;

use super::saturate;

fn all<S, T: From<S>>(items: impl IntoIterator<Item = S>) -> Vec<T> {
    items.into_iter().map(T::from).collect()
}

/// A tab of the Codex tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TreeTab {
    /// Places: tower, band, floor, location, spawn.
    Where,
    /// People, factions and npcs.
    Who,
    /// Topics, truths, beliefs, clues and equations.
    Lore,
    /// Things and enemies.
    Things,
}

impl From<TreeTab> for studio::TreeTab {
    fn from(tab: TreeTab) -> Self {
        match tab {
            TreeTab::Where => studio::TreeTab::Where,
            TreeTab::Who => studio::TreeTab::Who,
            TreeTab::Lore => studio::TreeTab::Lore,
            TreeTab::Things => studio::TreeTab::Things,
        }
    }
}

impl From<studio::TreeTab> for TreeTab {
    fn from(tab: studio::TreeTab) -> Self {
        match tab {
            studio::TreeTab::Where => TreeTab::Where,
            studio::TreeTab::Who => TreeTab::Who,
            studio::TreeTab::Lore => TreeTab::Lore,
            studio::TreeTab::Things => TreeTab::Things,
        }
    }
}

/// A tab of the Codex tree as the UI needs it: its kinds, their folder, the kinds whose rows
/// show the id first, and the kind at the top of a nested tab.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct TreeTabInfo {
    /// The tab.
    pub tab: TreeTab,
    /// The tab's kinds, from the top down or in group order.
    pub kinds: Vec<String>,
    /// The folder of the tab's records, such as `world/`.
    pub folder: String,
    /// The kinds whose rows and chips show the id before the title.
    pub id_first: Vec<String>,
    /// The kind at the top of a nested tab; `null` for a grouped tab.
    pub root: Option<String>,
}

impl From<&studio::TabTable> for TreeTabInfo {
    fn from(entry: &studio::TabTable) -> Self {
        let kinds: Vec<String> = entry.kinds().into_iter().map(str::to_owned).collect();
        TreeTabInfo {
            tab: entry.tab.into(),
            id_first: if entry.id_first {
                kinds.clone()
            } else {
                Vec::new()
            },
            kinds,
            folder: entry.folder.to_owned(),
            root: entry.root().map(str::to_owned),
        }
    }
}

/// Why a record is locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum LockReason {
    /// Its `status` is `canon`.
    Canon,
    /// It is a truth with target `never`.
    Never,
    /// It is in the generated formalism mirror.
    Formalism,
    /// It is a style whose `state` is `locked`.
    Style,
}

impl From<marks::LockReason> for LockReason {
    fn from(reason: marks::LockReason) -> Self {
        match reason {
            marks::LockReason::Canon => LockReason::Canon,
            marks::LockReason::Never => LockReason::Never,
            marks::LockReason::Formalism => LockReason::Formalism,
            marks::LockReason::Style => LockReason::Style,
        }
    }
}

/// The marks of one record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct Mark {
    /// Whether the record is a truth with target `never`.
    pub never: bool,
    /// Why the record is locked; empty when it is not.
    pub locked: Vec<LockReason>,
}

impl From<marks::MarkView> for Mark {
    fn from(mark: marks::MarkView) -> Self {
        Mark {
            never: mark.never,
            locked: all(mark.locked),
        }
    }
}

/// A record named in a result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct RecordRef {
    /// The id (the file stem).
    pub id: String,
    /// The project-relative path with `/`; `null` when no record has the id.
    pub path: Option<String>,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The text of the `title` scalar.
    pub title: Option<String>,
    /// The text of the `status` scalar.
    pub status: Option<String>,
    /// `never` and `locked`, flattened next to the id.
    #[serde(flatten)]
    pub mark: Mark,
}

impl From<p48::RecordRef> for RecordRef {
    fn from(record: p48::RecordRef) -> Self {
        RecordRef {
            id: record.id,
            path: record.path,
            kind: record.kind,
            title: record.title,
            status: record.status,
            mark: record.mark.into(),
        }
    }
}

/// An id a link names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct TargetRef {
    /// The record, or the bare id when none has it.
    #[serde(flatten)]
    pub record: RecordRef,
    /// Whether a record has the id.
    pub exists: bool,
}

impl From<p48::TargetRef> for TargetRef {
    fn from(target: p48::TargetRef) -> Self {
        TargetRef {
            record: target.record.into(),
            exists: target.exists,
        }
    }
}

/// One link with both ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct LinkRef {
    /// The slot text, such as `field:parent`, or `mention` for a link in the body.
    pub slot: String,
    /// The text after `#` in a body link.
    pub fragment: Option<String>,
    /// The line of the link, from 1.
    pub line: u32,
    /// The column of the link, from 1.
    pub column: u32,
    /// The record the link sits in.
    pub source: RecordRef,
    /// The record it names.
    pub target: TargetRef,
}

impl From<p48::LinkView> for LinkRef {
    fn from(link: p48::LinkView) -> Self {
        LinkRef {
            slot: link.slot,
            fragment: link.fragment,
            line: link.line,
            column: link.column,
            source: link.source.into(),
            target: link.target.into(),
        }
    }
}

/// A byte range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct Span {
    /// The first byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

impl From<check::SpanView> for Span {
    fn from(span: check::SpanView) -> Self {
        Span {
            start: saturate(span.start),
            end: saturate(span.end),
        }
    }
}

/// How bad a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The record is wrong.
    Error,
    /// The record is suspicious.
    Warning,
}

impl From<theos_content::Severity> for Severity {
    fn from(severity: theos_content::Severity) -> Self {
        match severity {
            theos_content::Severity::Error => Severity::Error,
            theos_content::Severity::Warning => Severity::Warning,
        }
    }
}

/// A finding of a record's file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct Finding {
    /// The rule that raised it.
    pub rule: String,
    /// How bad it is.
    pub severity: Severity,
    /// The project-relative path of the file.
    pub path: String,
    /// The id of the record, when the finding names one.
    pub id: Option<String>,
    /// The line, from 1.
    pub line: Option<u32>,
    /// The column, from 1.
    pub column: Option<u32>,
    /// The byte range.
    pub span: Option<Span>,
    /// What is wrong.
    pub message: String,
}

impl From<check::FindingView> for Finding {
    fn from(finding: check::FindingView) -> Self {
        Finding {
            rule: finding.rule,
            severity: finding.severity.into(),
            path: finding.path,
            id: finding.id,
            line: finding.line.map(saturate),
            column: finding.column.map(saturate),
            span: finding.span.map(Into::into),
            message: finding.message,
        }
    }
}

/// One place where a query matches a record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct Match {
    /// `title`, `body`, `field:<key>` or `field:<key>.<entry>`.
    pub column: String,
    /// The line of the match, from 1.
    pub line: u32,
    /// The column of the match, from 1, in Unicode scalar values.
    pub column_number: u32,
    /// The record text under the match.
    #[serde(rename = "match")]
    pub r#match: String,
    /// The line of the record that holds the match.
    pub text: String,
}

impl From<p48::MatchView> for Match {
    fn from(found: p48::MatchView) -> Self {
        Match {
            column: found.column,
            line: found.line,
            column_number: found.column_number,
            r#match: found.r#match,
            text: found.text,
        }
    }
}

/// One record that matches a query.
#[derive(Clone, Debug, PartialEq, Serialize, Type)]
pub struct Hit {
    /// The record.
    #[serde(flatten)]
    pub record: RecordRef,
    /// The relevance: higher is better.
    pub score: f64,
    /// Where the words match.
    pub matches: Vec<Match>,
}

impl From<p48::HitView> for Hit {
    fn from(hit: p48::HitView) -> Self {
        Hit {
            record: hit.record.into(),
            score: hit.score,
            matches: all(hit.matches),
        }
    }
}

/// The result of a search.
#[derive(Clone, Debug, PartialEq, Serialize, Type)]
pub struct QueryResult {
    /// The hits: best first for words, by id for filters alone.
    pub hits: Vec<Hit>,
    /// Whether more records match than `hits` holds.
    pub truncated: bool,
    /// A line for the user, such as "no kind `x`".
    pub note: Option<String>,
}

impl From<studio::QueryView> for QueryResult {
    fn from(query: studio::QueryView) -> Self {
        QueryResult {
            hits: all(query.hits),
            truncated: query.truncated,
            note: query.note,
        }
    }
}

/// A row of the tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TreeNode {
    /// A record with the records filed under it.
    Record {
        /// The record.
        record: RecordRef,
        /// A band's floor range, or the record's `status`.
        tag: Option<String>,
        /// The records under it.
        children: Vec<TreeNode>,
    },
    /// A group of records of one kind.
    Group {
        /// The group's name, such as `Truths`.
        label: String,
        /// The records of the group.
        children: Vec<TreeNode>,
    },
}

impl From<studio::TreeNode> for TreeNode {
    fn from(node: studio::TreeNode) -> Self {
        match node {
            studio::TreeNode::Record {
                record,
                tag,
                children,
            } => TreeNode::Record {
                record: record.into(),
                tag,
                children: all(children),
            },
            studio::TreeNode::Group { label, children } => TreeNode::Group {
                label,
                children: all(children),
            },
        }
    }
}

/// A link as the Codex draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct ViewLink {
    /// The id the link names.
    pub id: String,
    /// The text after `#`, if any.
    pub fragment: Option<String>,
    /// The text after `|`, if any.
    pub shown: Option<String>,
    /// The record the id names; `null` when no record has it.
    pub target: Option<RecordRef>,
}

impl From<studio::ViewLink> for ViewLink {
    fn from(link: studio::ViewLink) -> Self {
        ViewLink {
            id: link.id,
            fragment: link.fragment,
            shown: link.shown,
            target: link.target.map(Into::into),
        }
    }
}

/// A run of text inside a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewInline {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// Emphasised text.
    Emphasis {
        /// The text inside.
        inlines: Vec<ViewInline>,
    },
    /// Strong text.
    Strong {
        /// The text inside.
        inlines: Vec<ViewInline>,
    },
    /// Inline code.
    Code {
        /// The code.
        text: String,
    },
    /// A hard line break.
    Break,
    /// A wikilink chip.
    Link(ViewLink),
    /// A Markdown link or image.
    External {
        /// The shown text.
        text: String,
        /// The URL as written.
        url: String,
    },
}

impl From<studio::ViewInline> for ViewInline {
    fn from(inline: studio::ViewInline) -> Self {
        use studio::ViewInline as I;
        match inline {
            I::Text { text } => ViewInline::Text { text },
            I::Emphasis { inlines } => ViewInline::Emphasis {
                inlines: all(inlines),
            },
            I::Strong { inlines } => ViewInline::Strong {
                inlines: all(inlines),
            },
            I::Code { text } => ViewInline::Code { text },
            I::Break => ViewInline::Break,
            I::Link(link) => ViewInline::Link(link.into()),
            I::External { text, url } => ViewInline::External { text, url },
        }
    }
}

/// One attribute line of a beat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct BeatRow {
    /// The key before `::`.
    pub key: String,
    /// The value.
    pub value: Vec<ViewInline>,
}

impl From<studio::BeatRow> for BeatRow {
    fn from(row: studio::BeatRow) -> Self {
        BeatRow {
            key: row.key,
            value: all(row.value),
        }
    }
}

/// A block of a record body, with resolved links.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewNode {
    /// A heading.
    Heading {
        /// The level, 1 to 6.
        level: u8,
        /// The text of the heading.
        inlines: Vec<ViewInline>,
    },
    /// A paragraph.
    Paragraph {
        /// The text of the paragraph.
        inlines: Vec<ViewInline>,
    },
    /// A bullet or numbered list.
    List {
        /// Whether the list is numbered.
        ordered: bool,
        /// The items, each a list of blocks.
        items: Vec<Vec<ViewNode>>,
    },
    /// A block quote.
    Quote {
        /// The blocks inside the quote.
        nodes: Vec<ViewNode>,
    },
    /// A code block.
    Code {
        /// The code as written.
        text: String,
    },
    /// A thematic break.
    Rule,
    /// A `[!voice]` callout.
    Voice {
        /// Who speaks.
        speaker: Box<ViewLink>,
        /// The beliefs voiced.
        beliefs: Vec<ViewLink>,
        /// The tier, when the title names one.
        tier: Option<String>,
        /// The speech.
        nodes: Vec<ViewNode>,
        /// The caption line, if any.
        caption: Option<Vec<ViewInline>>,
    },
    /// A `[!design]` callout.
    Design {
        /// The remark.
        nodes: Vec<ViewNode>,
    },
    /// A `[!note]` callout.
    Note {
        /// The state as written.
        state: Option<String>,
        /// The title before the state.
        title: Vec<ViewInline>,
        /// The note's text.
        nodes: Vec<ViewNode>,
    },
    /// A callout of a type without a title grammar.
    Callout {
        /// The type in lower case.
        name: String,
        /// The title after the marker.
        title: Vec<ViewInline>,
        /// The blocks after the title line.
        nodes: Vec<ViewNode>,
    },
    /// A beat of a script.
    Beat {
        /// The heading text.
        heading: Vec<ViewInline>,
        /// The attribute lines that parse, in order.
        attributes: Vec<BeatRow>,
        /// The blocks of the beat.
        nodes: Vec<ViewNode>,
    },
    /// Source text shown as it is.
    Raw {
        /// The characters as written.
        text: String,
    },
}

impl From<studio::ViewNode> for ViewNode {
    fn from(node: studio::ViewNode) -> Self {
        use studio::ViewNode as N;
        match node {
            N::Heading { level, inlines } => ViewNode::Heading {
                level,
                inlines: all(inlines),
            },
            N::Paragraph { inlines } => ViewNode::Paragraph {
                inlines: all(inlines),
            },
            N::List { ordered, items } => ViewNode::List {
                ordered,
                items: items.into_iter().map(all).collect(),
            },
            N::Quote { nodes } => ViewNode::Quote { nodes: all(nodes) },
            N::Code { text } => ViewNode::Code { text },
            N::Rule => ViewNode::Rule,
            N::Voice {
                speaker,
                beliefs,
                tier,
                nodes,
                caption,
            } => ViewNode::Voice {
                speaker: Box::new((*speaker).into()),
                beliefs: all(beliefs),
                tier,
                nodes: all(nodes),
                caption: caption.map(all),
            },
            N::Design { nodes } => ViewNode::Design { nodes: all(nodes) },
            N::Note {
                state,
                title,
                nodes,
            } => ViewNode::Note {
                state,
                title: all(title),
                nodes: all(nodes),
            },
            N::Callout { name, title, nodes } => ViewNode::Callout {
                name,
                title: all(title),
                nodes: all(nodes),
            },
            N::Beat {
                heading,
                attributes,
                nodes,
            } => ViewNode::Beat {
                heading: all(heading),
                attributes: all(attributes),
                nodes: all(nodes),
            },
            N::Raw { text } => ViewNode::Raw { text },
        }
    }
}

/// One front-matter field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct FieldRow {
    /// The key.
    pub key: String,
    /// The typed value.
    pub value: FieldValue,
}

impl From<studio::FieldRow> for FieldRow {
    fn from(row: studio::FieldRow) -> Self {
        FieldRow {
            key: row.key,
            value: row.value.into(),
        }
    }
}

/// A front-matter value, typed by its field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    /// Plain text.
    Text(String),
    /// A link chip.
    Link(ViewLink),
    /// A list of values.
    List(Vec<FieldValue>),
    /// A tier value.
    Tier(String),
    /// A map of rows.
    Map(Vec<FieldRow>),
}

impl From<studio::FieldValue> for FieldValue {
    fn from(value: studio::FieldValue) -> Self {
        use studio::FieldValue as V;
        match value {
            V::Text(text) => FieldValue::Text(text),
            V::Link(link) => FieldValue::Link(link.into()),
            V::List(items) => FieldValue::List(all(items)),
            V::Tier(tier) => FieldValue::Tier(tier),
            V::Map(rows) => FieldValue::Map(all(rows)),
        }
    }
}

/// A row of the lore table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct LoreRow {
    /// The truth, belief or clue.
    pub record: RecordRef,
    /// How it relates to what it touches.
    pub verb: Option<String>,
    /// The truth or topic it touches.
    pub touches: Option<RecordRef>,
    /// Its tier.
    pub tier: Option<String>,
}

impl From<studio::LoreRow> for LoreRow {
    fn from(row: studio::LoreRow) -> Self {
        LoreRow {
            record: row.record.into(),
            verb: row.verb,
            touches: row.touches.map(Into::into),
            tier: row.tier,
        }
    }
}

/// A concept for the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct PictureRow {
    /// The concept.
    pub concept: RecordRef,
    /// Its picked version.
    pub picked: Option<String>,
}

impl From<studio::PictureRow> for PictureRow {
    fn from(row: studio::PictureRow) -> Self {
        PictureRow {
            concept: row.concept.into(),
            picked: row.picked,
        }
    }
}

/// One source that links a record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct UsedBy {
    /// The record that holds the links.
    pub source: RecordRef,
    /// The relations, in the order the links come.
    pub relations: Vec<String>,
}

impl From<studio::UsedBy> for UsedBy {
    fn from(used: studio::UsedBy) -> Self {
        UsedBy {
            source: used.source.into(),
            relations: used.relations,
        }
    }
}

/// The header of a record page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
pub struct Header {
    /// The record.
    pub record: RecordRef,
    /// The record its `parent` names, when that record exists.
    pub parent: Option<RecordRef>,
    /// How many links name the record.
    pub links_in: u32,
    /// How many records are filed under it.
    pub children: u32,
}

impl From<studio::Header> for Header {
    fn from(header: studio::Header) -> Self {
        Header {
            record: header.record.into(),
            parent: header.parent.map(Into::into),
            links_in: header.links_in,
            children: header.children,
        }
    }
}

/// Everything the record window and the side column show of one record.
#[derive(Clone, Debug, PartialEq, Serialize, Type)]
pub struct RecordPage {
    /// The top lines.
    pub header: Header,
    /// The body as a block tree.
    pub body: Vec<ViewNode>,
    /// The truths, beliefs and clues that link to the record.
    pub lore: Vec<LoreRow>,
    /// The concepts for the record.
    pub pictures: Vec<PictureRow>,
    /// The front-matter fields in file order.
    pub front_matter: Vec<FieldRow>,
    /// Every backlink, one row each.
    pub links_in: Vec<LinkRef>,
    /// The backlinks grouped by source.
    pub used_by: Vec<UsedBy>,
    /// The findings of the record's file.
    pub findings: Vec<Finding>,
}

impl From<studio::RecordPage> for RecordPage {
    fn from(page: studio::RecordPage) -> Self {
        RecordPage {
            header: page.header.into(),
            body: all(page.body),
            lore: all(page.lore),
            pictures: all(page.pictures),
            front_matter: all(page.front_matter),
            links_in: all(page.links_in),
            used_by: all(page.used_by),
            findings: all(page.findings),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), serde_json::Error>;

    /// Whether the mirror of `studio` serialises as `studio` does.
    fn same<S: Serialize, T: Serialize + From<S>>(studio: S) -> Result<bool, serde_json::Error> {
        let expected = serde_json::to_value(&studio)?;
        Ok(serde_json::to_value(T::from(studio))? == expected)
    }

    fn text(text: &str) -> studio::ViewInline {
        studio::ViewInline::Text {
            text: text.to_owned(),
        }
    }

    #[test]
    fn every_lock_reason_keeps_its_name() -> TestResult {
        for reason in [
            marks::LockReason::Canon,
            marks::LockReason::Never,
            marks::LockReason::Formalism,
            marks::LockReason::Style,
        ] {
            assert!(same::<_, LockReason>(reason)?, "{reason:?}");
        }
        Ok(())
    }

    fn link() -> studio::ViewLink {
        studio::ViewLink {
            id: "synthetic-dangling".to_owned(),
            fragment: None,
            shown: Some("synthetic".to_owned()),
            target: None,
        }
    }

    fn paragraph() -> studio::ViewNode {
        studio::ViewNode::Paragraph {
            inlines: vec![text("synthetic")],
        }
    }

    // Every variant, in this one test binary, so each conversion runs whole in one build.
    #[test]
    fn every_inline_serialises_like_the_studio() -> TestResult {
        use studio::ViewInline as I;
        for inline in [
            text("synthetic"),
            I::Emphasis {
                inlines: vec![text("synthetic")],
            },
            I::Strong {
                inlines: vec![text("synthetic")],
            },
            I::Code {
                text: "synthetic".to_owned(),
            },
            I::Break,
            I::Link(link()),
            I::External {
                text: "synthetic".to_owned(),
                url: "https://example.invalid".to_owned(),
            },
        ] {
            assert!(same::<_, ViewInline>(inline)?);
        }
        Ok(())
    }

    #[test]
    fn every_block_serialises_like_the_studio() -> TestResult {
        use studio::ViewNode as N;
        for node in [
            N::Heading {
                level: 2,
                inlines: vec![text("synthetic")],
            },
            paragraph(),
            N::List {
                ordered: true,
                items: vec![vec![paragraph()]],
            },
            N::Quote {
                nodes: vec![paragraph()],
            },
            N::Code {
                text: "synthetic".to_owned(),
            },
            N::Rule,
            N::Voice {
                speaker: Box::new(link()),
                beliefs: vec![link()],
                tier: Some("T1".to_owned()),
                nodes: vec![paragraph()],
                caption: Some(vec![text("synthetic")]),
            },
            N::Design {
                nodes: vec![paragraph()],
            },
            N::Note {
                state: Some("open".to_owned()),
                title: vec![text("synthetic")],
                nodes: vec![paragraph()],
            },
            N::Callout {
                name: "tip".to_owned(),
                title: vec![text("synthetic")],
                nodes: vec![paragraph()],
            },
            N::Beat {
                heading: vec![text("Beat 1 — synthetic")],
                attributes: vec![studio::BeatRow {
                    key: "at".to_owned(),
                    value: vec![studio::ViewInline::Link(link())],
                }],
                nodes: vec![paragraph()],
            },
            N::Raw {
                text: "<div>synthetic</div>".to_owned(),
            },
        ] {
            assert!(same::<_, ViewNode>(node)?);
        }
        Ok(())
    }

    #[test]
    fn severities_are_the_studio_names() -> TestResult {
        for (studio, severity) in [
            (theos_content::Severity::Error, Severity::Error),
            (theos_content::Severity::Warning, Severity::Warning),
        ] {
            assert_eq!(Severity::from(studio), severity);
            assert_eq!(serde_json::to_value(severity)?, studio.as_str());
        }
        Ok(())
    }
}
