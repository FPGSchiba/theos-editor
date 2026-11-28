//! The tree tabs of the Codex (P11 §5.8): Where and Who nest records by `parent`; Lore and Things
//! group records by kind. A kind the registry does not know yet is left out, so a kind that a
//! later phase registers appears with no change here.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use theos_content::{FieldType, RecordSummary, read_record};

use crate::views::codex::RecordRef;
use crate::{Marks, StudioError, Workspace};

/// The kinds of the Who tab, from the top down; `@` in a query keeps to them too.
pub(crate) const WHO: &[&str] = &["people", "faction", "npc"];

/// The kinds of the Where tab, from the top down.
const WHERE: &[&str] = &["tower", "band", "floor", "location", "spawn"];

/// The groups of the Lore tab: label and kind.
const LORE: &[(&str, &str)] = &[
    ("Topics", "topic"),
    ("Truths", "truth"),
    ("Beliefs", "belief"),
    ("Clues", "clue"),
    ("Equations", "equation"),
];

/// The groups of the Things tab: label and kind.
const THINGS: &[(&str, &str)] = &[("Things", "thing"), ("Enemies", "enemy")];

/// How a tab orders its records.
#[derive(Clone, Copy, Debug)]
pub enum TabShape {
    /// Nested by `parent`; the kinds from the top down.
    Nested(&'static [&'static str]),
    /// Grouped by kind; each group's label and kind.
    Groups(&'static [(&'static str, &'static str)]),
}

/// One tab of the tree (spec §5.8's table): its kinds, the folder new records of them go in, and
/// whether its rows show the id before the title.
#[derive(Clone, Copy, Debug)]
pub struct TabTable {
    /// The tab.
    pub tab: TreeTab,
    /// How the tab orders its records.
    pub shape: TabShape,
    /// The folder of the tab's records, such as `world/`.
    pub folder: &'static str,
    /// Whether the tab's kinds show their id before the title (the lore kinds).
    pub id_first: bool,
}

impl TabTable {
    /// The tab's kinds, from the top down or in group order.
    #[must_use]
    pub fn kinds(&self) -> Vec<&'static str> {
        match self.shape {
            TabShape::Nested(kinds) => kinds.to_vec(),
            TabShape::Groups(groups) => groups.iter().map(|(_, kind)| *kind).collect(),
        }
    }

    /// The kind at the top of a nested tab; `None` for a grouped tab.
    #[must_use]
    pub fn root(&self) -> Option<&'static str> {
        match self.shape {
            TabShape::Nested(kinds) => kinds.first().copied(),
            TabShape::Groups(_) => None,
        }
    }
}

/// The Where tab.
const WHERE_TAB: TabTable = TabTable {
    tab: TreeTab::Where,
    shape: TabShape::Nested(WHERE),
    folder: "world/",
    id_first: false,
};

/// The Who tab.
const WHO_TAB: TabTable = TabTable {
    tab: TreeTab::Who,
    shape: TabShape::Nested(WHO),
    folder: "people/",
    id_first: false,
};

/// The Lore tab.
const LORE_TAB: TabTable = TabTable {
    tab: TreeTab::Lore,
    shape: TabShape::Groups(LORE),
    folder: "lore/",
    id_first: true,
};

/// The Things tab.
const THINGS_TAB: TabTable = TabTable {
    tab: TreeTab::Things,
    shape: TabShape::Groups(THINGS),
    folder: "things/",
    id_first: false,
};

/// The tabs of the tree, in order: the one table that `tree` and the UI read.
pub const TABS: &[TabTable] = &[WHERE_TAB, WHO_TAB, LORE_TAB, THINGS_TAB];

/// The table entry of `tab`.
#[must_use]
pub fn table(tab: TreeTab) -> &'static TabTable {
    match tab {
        TreeTab::Where => &WHERE_TAB,
        TreeTab::Who => &WHO_TAB,
        TreeTab::Lore => &LORE_TAB,
        TreeTab::Things => &THINGS_TAB,
    }
}

/// The slot of a `parent` link.
const PARENT: &str = "field:parent";

/// A tab of the Codex tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
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

/// A row of the tree.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TreeNode {
    /// A record with the records filed under it.
    Record {
        /// The record.
        record: RecordRef,
        /// A band's floor range (`1–20`), or the record's `status`.
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

/// The rows of `tab`. The caller syncs the index first.
///
/// # Errors
/// [`StudioError::Index`] when the index fails.
pub fn tree(ws: &Workspace, tab: TreeTab) -> Result<Vec<TreeNode>, StudioError> {
    match table(tab).shape {
        TabShape::Nested(kinds) => nested(ws, kinds),
        TabShape::Groups(list) => groups(ws, list),
    }
}

/// The records of `kinds` that the registry knows, ordered by path.
fn records_of(ws: &Workspace, kinds: &[&str]) -> Result<Vec<RecordSummary>, StudioError> {
    let mut records = Vec::new();
    for kind in kinds {
        if ws.registry().kind(kind).is_some() {
            records.extend(ws.index().records_of_kind(kind)?);
        }
    }
    Ok(records)
}

fn groups(ws: &Workspace, groups: &[(&str, &str)]) -> Result<Vec<TreeNode>, StudioError> {
    let marks = Marks::load(ws.index())?;
    let mut nodes = Vec::new();
    for (label, kind) in groups {
        let mut records = records_of(ws, &[kind])?;
        if records.is_empty() {
            continue;
        }
        records.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
        nodes.push(TreeNode::Group {
            label: (*label).to_owned(),
            children: records
                .iter()
                .map(|record| TreeNode::Record {
                    record: RecordRef::from_summary(record, &marks),
                    tag: record.status.clone(),
                    children: Vec::new(),
                })
                .collect(),
        });
    }
    Ok(nodes)
}

/// The records of one nested tab and what each needs to be placed and drawn.
struct Nest {
    marks: Marks,
    records: Vec<RecordSummary>,
    /// The parent id of each record path, when a record of the tab has that id.
    parent: HashMap<String, String>,
    /// The path of each id; the first by path for a duplicated id.
    path_of: HashMap<String, String>,
    /// The order key of each path: `number`, or a band's first floor.
    number: HashMap<String, i64>,
    /// A band's floor range by path.
    range: HashMap<String, String>,
}

fn nested(ws: &Workspace, kinds: &[&str]) -> Result<Vec<TreeNode>, StudioError> {
    let nest = Nest::load(ws, kinds)?;
    let mut top: Vec<&RecordSummary> = nest
        .records
        .iter()
        .filter(|record| !nest.parent.contains_key(&record.path) || nest.on_cycle(record))
        .collect();
    nest.sort(&mut top);
    let mut seen = HashSet::new();
    Ok(top
        .into_iter()
        .filter_map(|record| nest.node(record, &mut seen))
        .collect())
}

impl Nest {
    fn load(ws: &Workspace, kinds: &[&str]) -> Result<Nest, StudioError> {
        let records = records_of(ws, kinds)?;
        let mut path_of = HashMap::new();
        for record in &records {
            path_of
                .entry(record.id.clone())
                .or_insert_with(|| record.path.clone());
        }
        let mut parent = HashMap::new();
        for record in &records {
            let named = ws
                .index()
                .links_from(&record.path)?
                .into_iter()
                .find(|link| link.slot.as_db() == PARENT)
                .map(|link| link.target);
            if let Some(id) = named.filter(|id| path_of.contains_key(id)) {
                parent.insert(record.path.clone(), id);
            }
        }
        let mut number: HashMap<String, i64> = ws
            .index()
            .field_texts("number")?
            .into_iter()
            .filter_map(|row| row.text.parse().ok().map(|n| (row.path, n)))
            .collect();
        let mut range = HashMap::new();
        for record in &records {
            if let Some((first, last)) = floor_range(ws, record) {
                number.entry(record.path.clone()).or_insert(first);
                range.insert(record.path.clone(), format!("{first}–{last}"));
            }
        }
        Ok(Nest {
            marks: Marks::load(ws.index())?,
            records,
            parent,
            path_of,
            number,
            range,
        })
    }

    /// Whether the `parent` chain of `record` leads back to it.
    fn on_cycle(&self, record: &RecordSummary) -> bool {
        let mut seen = HashSet::new();
        let mut at = record.path.as_str();
        while let Some(id) = self.parent.get(at) {
            if *id == record.id {
                return true;
            }
            let Some(next) = self.path_of.get(id) else {
                return false;
            };
            if !seen.insert(next.as_str()) {
                return false;
            }
            at = next;
        }
        false
    }

    /// `records` by number, then title, then id.
    fn sort(&self, records: &mut [&RecordSummary]) {
        records.sort_by(|a, b| {
            let number = |r: &RecordSummary| self.number.get(&r.path).copied().unwrap_or(i64::MAX);
            number(a)
                .cmp(&number(b))
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
                .then_with(|| a.path.cmp(&b.path))
        });
    }

    /// The row of `record` with the records filed under it; `None` when it was drawn already.
    fn node<'a>(
        &'a self,
        record: &'a RecordSummary,
        seen: &mut HashSet<&'a str>,
    ) -> Option<TreeNode> {
        if !seen.insert(record.path.as_str()) {
            return None;
        }
        let mut under: Vec<&RecordSummary> = self
            .records
            .iter()
            .filter(|child| {
                self.parent.get(&child.path) == Some(&record.id) && !self.on_cycle(child)
            })
            .collect();
        self.sort(&mut under);
        Some(TreeNode::Record {
            record: RecordRef::from_summary(record, &self.marks),
            tag: self
                .range
                .get(&record.path)
                .cloned()
                .or_else(|| record.status.clone()),
            children: under
                .into_iter()
                .filter_map(|child| self.node(child, seen))
                .collect(),
        })
    }
}

/// The first and last floor of a record whose kind has a floor-range field, read from its file.
fn floor_range(ws: &Workspace, record: &RecordSummary) -> Option<(i64, i64)> {
    let spec = ws.registry().kind(record.kind.as_deref()?)?;
    let key = spec
        .fields
        .iter()
        .find(|field| field.ty == FieldType::FloorRange)?
        .key;
    let file = ws.project().root().join(&record.path);
    let parsed = match read_record(&file) {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::warn!(path = %record.path, %error, "a record of the tree cannot be read");
            return None;
        }
    };
    let front = parsed.front_matter().ok()?;
    let theos_content::yaml::Value::List(list) = &front.get(key)?.value else {
        return None;
    };
    match list.items.as_slice() {
        [low, high] => Some((
            low.scalar.text.parse().ok()?,
            high.scalar.text.parse().ok()?,
        )),
        _ => None,
    }
}
