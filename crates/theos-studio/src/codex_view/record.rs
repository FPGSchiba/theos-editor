//! The record page of the Codex (P11 §5.8): the header, the body, the lore table, the pictures
//! and the side column, assembled from `codex::read` and P02's record model.

use serde::Serialize;
use theos_content::read_record;

use super::resolve::Resolver;
use super::{FieldRow, LoreRow, PictureRow, UsedBy, ViewNode};
use super::{body, front_matter, lore, pictures, relation};
use crate::codex::{ReadTarget, read, saturate};
use crate::views::check::FindingView;
use crate::views::codex::{LinkView, RecordRef, RecordView};
use crate::{StudioError, Workspace};

/// The slot of a `parent` link.
const PARENT: &str = "field:parent";

/// Everything the record window and the side column show of one record.
#[derive(Debug, Serialize)]
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
    pub links_in: Vec<LinkView>,
    /// The backlinks grouped by source, with their relations.
    pub used_by: Vec<UsedBy>,
    /// The findings of the record's file.
    pub findings: Vec<FindingView>,
}

/// The header of a record page.
#[derive(Clone, Debug, Serialize)]
pub struct Header {
    /// The record: id, kind, title, status, path and marks.
    pub record: RecordRef,
    /// The record its `parent` names, when that record exists.
    pub parent: Option<RecordRef>,
    /// How many links name the record.
    pub links_in: u32,
    /// How many records are filed under it.
    pub children: u32,
}

/// The page of the record `id`; for a duplicated id, the first record by path. The caller syncs
/// the index first; nothing is written.
///
/// # Errors
/// [`StudioError::UnknownRecord`] when no record has the id; [`StudioError::Read`] when its file
/// cannot be read; [`StudioError::Index`] when the index fails.
pub fn record(ws: &Workspace, id: &str) -> Result<RecordPage, StudioError> {
    let view = read(ws, &ReadTarget::Id(id.to_owned()))?
        .records
        .into_iter()
        .next()
        .ok_or_else(|| StudioError::UnknownRecord {
            what: id.to_owned(),
        })?;
    let path = view.record.path.clone().unwrap_or_default();
    let file = ws.project().root().join(&path);
    let parsed = read_record(&file).map_err(|source| StudioError::Read { path: file, source })?;
    let resolver = Resolver::load(ws)?;
    let front_matter = parsed
        .front_matter()
        .map(|front| {
            front_matter::rows(front, view.record.kind.as_deref(), ws.registry(), &resolver)
        })
        .unwrap_or_default();
    Ok(RecordPage {
        header: header(&view),
        body: body::nodes(&theos_content::body::tree::tree(&parsed), &resolver),
        lore: lore::rows(ws, &resolver, &path, &view.backlinks)?,
        pictures: pictures::rows(ws, &view.backlinks)?,
        front_matter,
        used_by: relation::used_by(&view.backlinks),
        links_in: view.backlinks,
        findings: view.findings,
    })
}

fn header(view: &RecordView) -> Header {
    let parent = view
        .links
        .iter()
        .find(|link| link.slot == PARENT && link.target.exists)
        .map(|link| link.target.record.clone());
    Header {
        record: view.record.clone(),
        parent,
        links_in: saturate(view.backlinks.len()),
        children: saturate(view.children.len()),
    }
}
