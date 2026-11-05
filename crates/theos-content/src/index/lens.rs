//! Effective lenses (spec §5.12): the lens of a people, a faction or an npc with the layer each
//! value comes from. A child's valid entry replaces its parent's; an invalid entry never does,
//! so it cannot withhold or reveal what the parent says.

use std::collections::BTreeSet;

use rusqlite::params;

use crate::finding::Finding;
use crate::index::query::offset;
use crate::index::rows::touches;
use crate::index::slots::Slot;
use crate::index::{Index, IndexError};
use crate::kinds::{FieldType, LENS_ENTRIES, LensEntry};
use crate::text::{Position, Span};

/// One record of the chain: the holder or one of its parents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    /// The id (the file stem).
    pub id: String,
    /// The kind.
    pub kind: String,
    /// The project-relative path with `/`.
    pub path: String,
}

/// Where a value is written: the layer and the value's place in its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// The layer whose lens holds the value.
    pub layer: Layer,
    /// The scalar, or the whole list.
    pub span: Span,
    /// Line and column of the span start.
    pub position: Position,
}

/// The value of a lens entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensData {
    /// A text or an enum value.
    Text(String),
    /// The ids of a list of links, in order; the list may be empty.
    Links(Vec<String>),
}

/// A value of a farther layer that a nearer one replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overridden {
    /// The replaced value.
    pub value: LensData,
    /// Where it is written.
    pub origin: Origin,
}

/// One entry of the effective lens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensValue {
    /// The entry key, such as `stance`.
    pub field: String,
    /// The value of the nearest layer that sets the entry.
    pub value: LensData,
    /// Where that value is written.
    pub origin: Origin,
    /// The values of the farther layers that this one replaces, nearest first. A value equal to
    /// the winner still counts.
    pub overrides: Vec<Overridden>,
}

/// An entry that does not count, because its value has a `schema/value` problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoredEntry {
    /// The entry key, or `lens` when the lens itself is not a map.
    pub field: String,
    /// Where the value is written.
    pub origin: Origin,
    /// The `schema/value` findings that overlap the value.
    pub findings: Vec<Finding>,
}

/// A belief held by a layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldBelief {
    /// The belief id.
    pub belief: String,
    /// The path of the belief record.
    pub path: String,
    /// The layer in whose `held_by` the belief names it.
    pub through: Layer,
}

/// The lens of a holder with everything it is made of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveLens {
    /// The holder, then its parents up to the top.
    pub chain: Vec<Layer>,
    /// The entries some layer sets, in the order of the lens entries.
    pub fields: Vec<LensValue>,
    /// The entries that do not count, by layer from the holder up.
    pub ignored: Vec<IgnoredEntry>,
    /// The beliefs of each layer, holder first, then by belief id.
    pub beliefs: Vec<HeldBelief>,
}

/// The shape of a stored value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Scalar,
    List,
    Map,
}

/// One `field_values` row of a lens.
struct Node {
    entry: Option<String>,
    item: bool,
    shape: Shape,
    text: Option<String>,
    span: Span,
    position: Position,
    valid: bool,
}

/// What one layer's lens says.
#[derive(Default)]
struct Written {
    set: Vec<(&'static str, LensData, Origin)>,
    ignored: Vec<IgnoredEntry>,
}

impl Index {
    /// The effective lens of a people, a faction or an npc: its chain of parents, the value of
    /// each entry with the layer it comes from and the values it replaces, the entries that do
    /// not count and the beliefs of each layer. `None` when `holder` is no id of a record whose
    /// kind has a lens. When the id is duplicated, the first record by path counts.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn lens(&self, holder: &str) -> Result<Option<EffectiveLens>, IndexError> {
        let holders = self.holder_kinds();
        let Some(first) = self.first_of(holder, &holders)? else {
            return Ok(None);
        };
        let chain = self.chain(first, &holders)?;
        let mut writes = Vec::new();
        for layer in &chain {
            writes.push(self.written(layer)?);
        }
        let fields = LENS_ENTRIES
            .iter()
            .filter_map(|entry| merge(entry.key, &writes))
            .collect();
        let ignored = writes.iter().flat_map(|w| w.ignored.clone()).collect();
        let beliefs = self.beliefs(&chain)?;
        Ok(Some(EffectiveLens {
            chain,
            fields,
            ignored,
            beliefs,
        }))
    }

    /// The kinds whose spec has a lens field.
    fn holder_kinds(&self) -> Vec<String> {
        self.registry
            .kinds()
            .filter(|spec| spec.field("lens").is_some_and(|f| f.ty == FieldType::Lens))
            .map(|spec| spec.name.to_owned())
            .collect()
    }

    /// The first record by path that has `id` and one of `kinds`.
    fn first_of(&self, id: &str, kinds: &[String]) -> Result<Option<Layer>, IndexError> {
        let mut stmt = self
            .store
            .conn
            .prepare_cached("SELECT path, kind FROM records WHERE id = ?1 ORDER BY path")?;
        let rows = stmt.query_map([id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        for row in rows {
            let (path, kind) = row?;
            if let Some(kind) = kind.filter(|kind| kinds.contains(kind)) {
                return Ok(Some(Layer {
                    id: id.to_owned(),
                    kind,
                    path,
                }));
            }
        }
        Ok(None)
    }

    /// The holder and its parents: each step follows the `parent` link to a holder of a kind the
    /// field allows, and ends at a dangling or wrong-kind parent or at a cycle.
    fn chain(&self, first: Layer, holders: &[String]) -> Result<Vec<Layer>, IndexError> {
        let mut chain = vec![first];
        loop {
            let Some(layer) = chain.last() else {
                return Ok(chain);
            };
            let allowed: Vec<String> = match self
                .registry
                .kind(&layer.kind)
                .and_then(|spec| spec.field("parent"))
                .map(|field| field.ty)
            {
                Some(FieldType::Link(targets)) => targets
                    .iter()
                    .map(|target| target.kind.to_owned())
                    .filter(|kind| holders.contains(kind))
                    .collect(),
                _ => return Ok(chain),
            };
            let parent = self
                .links_from(&layer.path)?
                .into_iter()
                .find(|link| link.slot == Slot::Field("parent".to_owned()));
            let Some(parent) = parent else {
                return Ok(chain);
            };
            if chain.iter().any(|seen| seen.id == parent.target) {
                return Ok(chain);
            }
            match self.first_of(&parent.target, &allowed)? {
                Some(next) => chain.push(next),
                None => return Ok(chain),
            }
        }
    }

    /// The `lens` rows of a record, in file order.
    fn lens_nodes(&self, path: &str) -> Result<Vec<Node>, IndexError> {
        let mut stmt = self.store.conn.prepare_cached(
            "SELECT entry, item, shape, text, span_start, span_end, line, col, valid \
             FROM field_values WHERE path = ?1 AND field = 'lens' ORDER BY rowid",
        )?;
        let rows = stmt.query_map(params![path], |row| {
            let shape: String = row.get(2)?;
            let at = |column| offset(row, column).map(Option::unwrap_or_default);
            Ok(Node {
                entry: row.get(0)?,
                item: row.get::<_, Option<i64>>(1)?.is_some(),
                shape: match shape.as_str() {
                    "map" => Shape::Map,
                    "list" => Shape::List,
                    _ => Shape::Scalar,
                },
                text: row.get(3)?,
                span: Span {
                    start: at(4)?,
                    end: at(5)?,
                },
                position: Position {
                    line: at(6)?,
                    column: at(7)?,
                },
                valid: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// What the lens of one layer sets and what it ignores.
    fn written(&self, layer: &Layer) -> Result<Written, IndexError> {
        let nodes = self.lens_nodes(&layer.path)?;
        let mut written = Written::default();
        let Some(top) = nodes.iter().find(|n| n.entry.is_none() && !n.item) else {
            return Ok(written);
        };
        if top.shape != Shape::Map {
            written.ignored.push(self.ignore("lens", layer, top)?);
            return Ok(written);
        }
        for entry in LENS_ENTRIES {
            let key = Some(entry.key);
            let Some(node) = nodes.iter().find(|n| n.entry.as_deref() == key && !n.item) else {
                continue;
            };
            let items = nodes
                .iter()
                .filter(|n| n.entry.as_deref() == key && n.item)
                .map(|n| n.text.clone().unwrap_or_default());
            match data(entry, node, items) {
                Some(value) => written.set.push((entry.key, value, origin(layer, node))),
                None => written.ignored.push(self.ignore(entry.key, layer, node)?),
            }
        }
        Ok(written)
    }

    fn ignore(&self, field: &str, layer: &Layer, node: &Node) -> Result<IgnoredEntry, IndexError> {
        let findings = self
            .findings_for(&layer.path)?
            .into_iter()
            .filter(|f| f.rule == "schema/value")
            .filter(|f| f.span.is_some_and(|span| touches(span, node.span)))
            .collect();
        Ok(IgnoredEntry {
            field: field.to_owned(),
            origin: origin(layer, node),
            findings,
        })
    }

    /// The beliefs that name a layer in `held_by`, by layer and belief id.
    fn beliefs(&self, chain: &[Layer]) -> Result<Vec<HeldBelief>, IndexError> {
        let beliefs: BTreeSet<String> = self
            .records_of_kind("belief")?
            .into_iter()
            .map(|record| record.path)
            .collect();
        let held_by = Slot::Field("held_by".to_owned());
        let mut held = Vec::new();
        for layer in chain {
            let mut through: Vec<HeldBelief> = self
                .backlinks(&layer.id)?
                .into_iter()
                .filter(|link| link.slot == held_by && beliefs.contains(&link.source_path))
                .map(|link| HeldBelief {
                    belief: link.source_id,
                    path: link.source_path,
                    through: layer.clone(),
                })
                .collect();
            through.sort_by(|a, b| (&a.belief, &a.path).cmp(&(&b.belief, &b.path)));
            through.dedup();
            held.extend(through);
        }
        Ok(held)
    }
}

fn origin(layer: &Layer, node: &Node) -> Origin {
    Origin {
        layer: layer.clone(),
        span: node.span,
        position: node.position,
    }
}

/// The value of a valid entry node of the right shape; `None` when it does not count.
fn data(entry: &LensEntry, node: &Node, items: impl Iterator<Item = String>) -> Option<LensData> {
    if !node.valid {
        return None;
    }
    match (entry.ty, node.shape) {
        (FieldType::Links(_), Shape::List) => Some(LensData::Links(items.collect())),
        (FieldType::Links(_), _) | (_, Shape::List | Shape::Map) => None,
        (_, Shape::Scalar) => node.text.clone().map(LensData::Text),
    }
}

/// The value of `key` over all layers: the nearest wins, the farther ones are overridden.
fn merge(key: &str, writes: &[Written]) -> Option<LensValue> {
    let mut found = writes
        .iter()
        .filter_map(|w| w.set.iter().find(|(set, ..)| *set == key));
    let (_, value, origin) = found.next()?;
    Some(LensValue {
        field: key.to_owned(),
        value: value.clone(),
        origin: origin.clone(),
        overrides: found
            .map(|(_, value, origin)| Overridden {
                value: value.clone(),
                origin: origin.clone(),
            })
            .collect(),
    })
}
