//! `codex::list`: the records of the project, filtered and paged, with their marks.

use theos_content::{FieldSpec, FieldType};

use crate::views::codex::{ListView, RecordRef};
use crate::{Marks, StudioError, Workspace};

/// The most records one page holds.
const MAX_LIMIT: usize = 500;

/// Which records `list` returns, and which page of them.
#[derive(Clone, Debug)]
pub struct ListFilter {
    /// Only records of this registered kind.
    pub kind: Option<String>,
    /// Only records with this status, one the registry allows (`draft` or `canon`).
    pub status: Option<String>,
    /// Only records under this project-relative folder, written with `/`.
    pub folder: Option<String>,
    /// Records to skip.
    pub offset: usize,
    /// Records per page, 1 to 500.
    pub limit: usize,
}

/// The records that pass `filter`, ordered by path, one page of them, each with its marks.
///
/// # Errors
/// [`StudioError::InvalidFilter`] for a `status`, `folder` or `limit` that is not valid;
/// [`StudioError::UnknownKind`] for a `kind` that is not registered; [`StudioError::Index`] when
/// the index fails.
pub fn list(ws: &Workspace, filter: &ListFilter) -> Result<ListView, StudioError> {
    if !(1..=MAX_LIMIT).contains(&filter.limit) {
        return Err(invalid(format!(
            "limit must be between 1 and {MAX_LIMIT}, not {}",
            filter.limit
        )));
    }
    if let Some(kind) = &filter.kind
        && !ws.registry().kinds().any(|spec| spec.name == kind)
    {
        return Err(StudioError::unknown_kind(ws.registry(), kind));
    }
    if let Some(status) = &filter.status {
        let allowed = status_values(ws);
        if !allowed.contains(&status.as_str()) {
            let names: Vec<String> = allowed.iter().map(|name| format!("`{name}`")).collect();
            return Err(invalid(format!(
                "status must be one of {}, not `{status}`",
                names.join(", ")
            )));
        }
    }
    let prefix = filter.folder.as_deref().map(folder_prefix).transpose()?;

    let marks = Marks::load(ws.index())?;
    let matching: Vec<_> = ws
        .index()
        .records()?
        .into_iter()
        .filter(|record| filter.kind.is_none() || record.kind == filter.kind)
        .filter(|record| filter.status.is_none() || record.status == filter.status)
        .filter(|record| {
            prefix
                .as_deref()
                .is_none_or(|prefix| record.path.starts_with(prefix))
        })
        .collect();
    let total = matching.len();
    let records: Vec<RecordRef> = matching
        .iter()
        .skip(filter.offset)
        .take(filter.limit)
        .map(|record| RecordRef::from_summary(record, &marks))
        .collect();
    Ok(ListView {
        truncated: filter.offset.saturating_add(records.len()) < total,
        records,
        total,
        offset: filter.offset,
    })
}

/// The values the registry allows for `status`, in registration order, without repeats.
fn status_values(ws: &Workspace) -> Vec<&'static str> {
    let mut values: Vec<&'static str> = Vec::new();
    for spec in ws.registry().kinds() {
        if let Some(FieldSpec {
            ty: FieldType::Enum(allowed),
            ..
        }) = spec.field("status")
        {
            for value in *allowed {
                if !values.contains(value) {
                    values.push(value);
                }
            }
        }
    }
    values
}

/// The path prefix of a folder filter: the folder with one trailing `/`.
fn folder_prefix(folder: &str) -> Result<String, StudioError> {
    let trimmed = folder.strip_suffix('/').unwrap_or(folder);
    let bad = trimmed.is_empty()
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.starts_with('/')
        || trimmed
            .split('/')
            .any(|part| part.is_empty() || part == "..");
    if bad {
        return Err(invalid(format!(
            "folder must be relative, written with `/` and without `..`, not `{folder}`"
        )));
    }
    Ok(format!("{trimmed}/"))
}

fn invalid(message: String) -> StudioError {
    StudioError::InvalidFilter { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StudioError;
    use crate::test_support::{TestResult, codex_workspace};

    fn filter() -> ListFilter {
        ListFilter {
            kind: None,
            status: None,
            folder: None,
            offset: 0,
            limit: 500,
        }
    }

    fn ids(view: &crate::views::codex::ListView) -> Vec<&str> {
        view.records.iter().map(|r| r.id.as_str()).collect()
    }

    // AC: CE-18.1
    // AC: CE-18.3
    #[test]
    fn list_filters_combine() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let beliefs = list(
            &ws,
            &ListFilter {
                kind: Some("belief".into()),
                ..filter()
            },
        )?;
        assert_eq!(ids(&beliefs), ["B-911", "B-912"]);
        let canon = list(
            &ws,
            &ListFilter {
                status: Some("canon".into()),
                ..filter()
            },
        )?;
        assert_eq!(canon.records.len(), 2, "{:?}", ids(&canon));
        assert!(
            canon
                .records
                .iter()
                .all(|r| r.mark.locked.contains(&crate::LockReason::Canon))
        );
        let truths = list(
            &ws,
            &ListFilter {
                folder: Some("lore/truths".into()),
                ..filter()
            },
        )?;
        assert_eq!(ids(&truths), ["T-911", "T-912"]);
        assert!(truths.records[0].mark.never);
        let slash = list(
            &ws,
            &ListFilter {
                folder: Some("lore/truths/".into()),
                ..filter()
            },
        )?;
        assert_eq!(ids(&slash), ["T-911", "T-912"]);
        let prefix = list(
            &ws,
            &ListFilter {
                folder: Some("lore/truth".into()),
                ..filter()
            },
        )?;
        assert!(prefix.records.is_empty());
        assert_eq!(prefix.total, 0);

        let all = list(&ws, &filter())?;
        let page = list(
            &ws,
            &ListFilter {
                offset: 1,
                limit: 2,
                ..filter()
            },
        )?;
        assert_eq!(page.total, all.total);
        assert_eq!(page.offset, 1);
        assert_eq!(ids(&page), ids(&all)[1..3]);
        assert!(page.truncated);
        assert!(!all.truncated);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn list_rejects_bad_filters() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let bad = [
            ListFilter {
                kind: Some("nope".into()),
                ..filter()
            },
            ListFilter {
                status: Some("Canon".into()),
                ..filter()
            },
            ListFilter {
                folder: Some("../x".into()),
                ..filter()
            },
            ListFilter {
                folder: Some(r"a\b".into()),
                ..filter()
            },
            ListFilter {
                folder: Some("/a".into()),
                ..filter()
            },
            ListFilter {
                limit: 0,
                ..filter()
            },
            ListFilter {
                limit: 501,
                ..filter()
            },
        ];
        for filter in &bad[1..] {
            let error = list(&ws, filter).err().ok_or("accepted")?;
            assert!(
                matches!(error, StudioError::InvalidFilter { .. }),
                "{error:?}"
            );
        }
        let error = list(&ws, &bad[0]).err().ok_or("accepted")?;
        assert!(
            matches!(error, StudioError::UnknownKind { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("npc"), "{error}");
        Ok(())
    }
}
