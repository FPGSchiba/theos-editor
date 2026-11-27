//! `codex::graph`: a bounded breadth-first walk of the links around one record.

use std::collections::{BTreeSet, HashMap};

use schemars::JsonSchema;
use serde::Deserialize;

use super::lookup::Lookup;
use crate::views::codex::{EdgeView, GraphView, NodeView};
use crate::{StudioError, Workspace};

/// The most nodes a graph lists.
pub const MAX_NODES: usize = 200;

/// Which links the walk follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Links the records contain.
    Out,
    /// Links that name the records.
    In,
    /// Both.
    Both,
}

/// The records within `depth` links of `id`.
///
/// Breadth-first: out-edges are the links of every record with the id, in-edges the backlinks
/// and children. A node is ordered by distance, then path (a missing id by id). The walk stops at
/// the end of the distance that would pass [`MAX_NODES`], dropping that distance's last nodes, and
/// sets `truncated`. A missing id is a node, never walked. An edge is listed only when both its
/// ends are nodes.
///
/// # Errors
/// [`StudioError::InvalidFilter`] when `depth` is not 1 to 3; [`StudioError::UnknownRecord`] when
/// no record has `id`; [`StudioError::Index`] when the index fails.
pub fn graph(
    ws: &Workspace,
    id: &str,
    depth: u8,
    direction: Direction,
) -> Result<GraphView, StudioError> {
    if !(1..=3).contains(&depth) {
        return Err(StudioError::InvalidFilter {
            message: format!("depth is {depth}; it must be 1, 2 or 3"),
        });
    }
    if ws.index().record(id)?.is_empty() {
        return Err(StudioError::UnknownRecord {
            what: id.to_owned(),
        });
    }
    let lookup = Lookup::load(ws)?;
    let mut distance: HashMap<String, u8> = HashMap::from([(id.to_owned(), 0)]);
    let mut order = vec![id.to_owned()];
    let mut edges = BTreeSet::new();
    let mut frontier = vec![id.to_owned()];
    let mut truncated = false;
    for step in 0..=depth {
        let mut reached = BTreeSet::new();
        for node in &frontier {
            for edge in edges_of(ws, &lookup, node, direction)? {
                reached.insert(if edge.from == *node {
                    edge.to.clone()
                } else {
                    edge.from.clone()
                });
                edges.insert(edge);
            }
        }
        if step == depth || truncated {
            break;
        }
        let mut next: Vec<String> = reached
            .into_iter()
            .filter(|reached| !distance.contains_key(reached))
            .collect();
        next.sort_by_cached_key(|reached| sort_key(&lookup, reached));
        let room = MAX_NODES.saturating_sub(order.len());
        if next.len() > room {
            next.truncate(room);
            truncated = true;
        }
        for reached in &next {
            distance.insert(reached.clone(), step + 1);
            order.push(reached.clone());
        }
        frontier = next;
    }
    let nodes = order
        .iter()
        .map(|node| NodeView {
            target: lookup.target(node),
            distance: distance.get(node).copied().unwrap_or_default(),
        })
        .collect();
    Ok(GraphView {
        root: id.to_owned(),
        depth,
        nodes,
        edges: edges
            .into_iter()
            .filter(|e| distance.contains_key(&e.from) && distance.contains_key(&e.to))
            .collect(),
        truncated,
    })
}

/// A node sorts by its path; a missing id by the id.
fn sort_key(lookup: &Lookup, id: &str) -> String {
    lookup
        .by_id
        .get(id)
        .map_or_else(|| id.to_owned(), |summary| summary.path.clone())
}

/// The edges of `id` the direction follows. A missing id has none: it is never walked.
fn edges_of(
    ws: &Workspace,
    lookup: &Lookup,
    id: &str,
    direction: Direction,
) -> Result<Vec<EdgeView>, StudioError> {
    let index = ws.index();
    let mut links = Vec::new();
    if matches!(direction, Direction::Out | Direction::Both) {
        for summary in index.record(id)? {
            links.extend(index.links_from(&summary.path)?);
        }
    }
    if lookup.by_id.contains_key(id) && matches!(direction, Direction::In | Direction::Both) {
        links.extend(index.backlinks(id)?);
        links.extend(index.children(id)?);
    }
    Ok(links
        .into_iter()
        .map(|link| EdgeView {
            from: link.source_id,
            to: link.target,
            slot: link.slot.as_db(),
            fragment: link.fragment,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{Fixture, TestResult, codex_workspace};

    fn node<'a>(view: &'a GraphView, id: &str) -> Option<&'a NodeView> {
        view.nodes.iter().find(|n| n.target.record.id == id)
    }

    fn has_edge(view: &GraphView, from: &str, to: &str, slot: &str) -> bool {
        view.edges
            .iter()
            .any(|e| e.from == from && e.to == to && e.slot == slot)
    }

    // AC: CE-18.1
    #[test]
    fn graph_depth_one_both() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = graph(&ws, "mother-ilse", 1, Direction::Both)?;
        assert_eq!(view.root, "mother-ilse");
        assert_eq!(node(&view, "mother-ilse").ok_or("root")?.distance, 0);
        for id in ["fates-cult", "B-912", "S-004"] {
            assert_eq!(node(&view, id).ok_or(id)?.distance, 1, "{id}");
        }
        assert!(has_edge(&view, "S-004", "mother-ilse", "beat:with"));
        assert!(!view.truncated);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn graph_out_only() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = graph(&ws, "mother-ilse", 1, Direction::Out)?;
        assert!(has_edge(&view, "mother-ilse", "fates-cult", "field:parent"));
        assert!(node(&view, "S-004").is_none());
        assert!(node(&view, "B-912").is_none());
        assert!(view.edges.iter().all(|e| e.from == "mother-ilse"));
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn graph_in_only() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = graph(&ws, "mother-ilse", 1, Direction::In)?;
        assert!(has_edge(&view, "S-004", "mother-ilse", "beat:with"));
        assert!(has_edge(&view, "B-912", "mother-ilse", "field:held_by"));
        assert!(node(&view, "fates-cult").is_none());
        assert!(view.edges.iter().all(|e| e.to == "mother-ilse"));
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn graph_marks_never_node() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = graph(&ws, "mother-ilse", 2, Direction::Both)?;
        let truth = node(&view, "T-911").ok_or("T-911")?;
        assert_eq!(truth.distance, 2);
        assert!(truth.target.record.mark.never);
        let json = serde_json::to_value(truth)?;
        assert_eq!(json["never"], true);
        assert_eq!(json["distance"], 2);
        assert_eq!(json["exists"], true);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn graph_missing_node_not_walked() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = graph(&ws, "S-004", 3, Direction::Both)?;
        let ghost = node(&view, "ghost-911").ok_or("ghost")?;
        assert!(!ghost.target.exists);
        assert!(
            view.edges
                .iter()
                .any(|e| e.from == "S-004" && e.to == "ghost-911")
        );
        assert!(view.edges.iter().all(|e| e.from != "ghost-911"));
        Ok(())
    }

    /// A synthetic belief record (path, bytes).
    fn belief(id: &str, held_by: &str) -> (String, Vec<u8>) {
        let text = format!(
            "---\n# synthetic fixture (P48), not canon\nid: {id}\nkind: belief\ntitle: Synthetic\n\
             status: draft\nheld_by: [{held_by}]\n---\nSynthetic.\n"
        );
        (format!("lore/beliefs/{id}.md"), text.into_bytes())
    }

    fn project(files: &[(String, Vec<u8>)]) -> Result<Fixture, Box<dyn std::error::Error>> {
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect();
        Fixture::new(&refs)
    }

    // AC: CE-18.1
    #[test]
    fn graph_cycle_terminates() -> TestResult {
        let fixture = project(&[belief("B-801", "B-802"), belief("B-802", "B-801")])?;
        let view = graph(fixture.workspace(), "B-801", 3, Direction::Both)?;
        let mut ids: Vec<&str> = view
            .nodes
            .iter()
            .map(|n| n.target.record.id.as_str())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, ["B-801", "B-802"]);
        assert_eq!(view.edges.len(), 2);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn graph_caps_at_max_nodes() -> TestResult {
        let mut files = vec![belief("B-900", "B-900")];
        for n in 0..250 {
            files.push(belief(&format!("B-{}", 1000 + n), "B-900"));
        }
        let fixture = project(&files)?;
        let view = graph(fixture.workspace(), "B-900", 1, Direction::Both)?;
        assert_eq!(view.nodes.len(), MAX_NODES);
        assert!(view.truncated);
        // The cut keeps path order: the first paths stay.
        assert!(node(&view, "B-1000").is_some());
        assert!(node(&view, "B-1249").is_none());
        // Edges only join nodes in the result.
        assert!(
            view.edges
                .iter()
                .all(|e| node(&view, &e.from).is_some() && node(&view, &e.to).is_some())
        );
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn graph_depth_bounds() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        for depth in [0, 4] {
            let error = graph(&ws, "mother-ilse", depth, Direction::Both)
                .err()
                .ok_or("accepted")?;
            assert!(
                matches!(error, StudioError::InvalidFilter { .. }),
                "{error:?}"
            );
        }
        let error = graph(&ws, "nope", 1, Direction::Both)
            .err()
            .ok_or("accepted")?;
        assert!(
            matches!(error, StudioError::UnknownRecord { .. }),
            "{error:?}"
        );
        Ok(())
    }
}
