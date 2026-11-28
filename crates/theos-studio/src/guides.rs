//! The guides of the content project: `tools/guides/*.md`, which the MCP server serves as prompts.

use std::path::Path;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use schemars::JsonSchema;
use serde::Serialize;

use crate::StudioError;

const FOLDER: &str = "tools/guides";

/// One guide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Guide {
    /// The file name without `.md`.
    pub name: String,
    /// The text of the first heading, else the name.
    pub title: String,
    /// The project-relative path, written with `/`.
    pub path: String,
}

/// The guides of the project in `root`, sorted by name. A file that is not UTF-8 is skipped; a
/// missing folder gives an empty list.
#[must_use]
pub fn list(root: &Path) -> Vec<Guide> {
    let Ok(entries) = std::fs::read_dir(root.join(FOLDER)) else {
        return Vec::new();
    };
    let mut guides = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "skipping a guide that cannot be read");
                continue;
            }
        };
        guides.push(Guide {
            title: title_of(&text).unwrap_or_else(|| name.to_owned()),
            path: format!("{FOLDER}/{name}.md"),
            name: name.to_owned(),
        });
    }
    guides.sort_by(|a, b| a.name.cmp(&b.name));
    guides
}

/// The text of guide `name` with LF line ends; `None` for an unknown name or one that is not a
/// plain file name.
///
/// # Errors
/// [`StudioError::Io`] when the file exists and cannot be read.
pub fn read(root: &Path, name: &str) -> Result<Option<String>, StudioError> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") {
        return Ok(None);
    }
    let path = root.join(FOLDER).join(format!("{name}.md"));
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text.replace("\r\n", "\n"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StudioError::Io { path, source }),
    }
}

/// The text of the first heading of any level.
fn title_of(text: &str) -> Option<String> {
    let mut title: Option<String> = None;
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Heading { .. }) => title = Some(String::new()),
            Event::Text(part) | Event::Code(part) => {
                if let Some(title) = title.as_mut() {
                    title.push_str(&part);
                }
            }
            Event::End(TagEnd::Heading(_)) => return title.filter(|t| !t.trim().is_empty()),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    // AC: CE-18.1
    #[test]
    fn guides_edge_cases() -> TestResult {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join(FOLDER);
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("crlf.md"), "# Title\r\nline\r\n")?;
        std::fs::write(folder.join("bad.md"), [0xff_u8, 0xfe, 0x00])?;
        let guides = list(dir.path());
        assert_eq!(guides.len(), 1);
        assert_eq!(guides[0].title, "Title");
        assert_eq!(
            read(dir.path(), "crlf")?.as_deref(),
            Some("# Title\nline\n")
        );
        assert_eq!(read(dir.path(), "../theos")?, None);
        assert_eq!(read(dir.path(), "missing")?, None);

        let other = tempfile::tempdir()?;
        std::fs::create_dir_all(other.path().join("tools"))?;
        std::fs::write(other.path().join(FOLDER), "a file")?;
        assert_eq!(list(other.path()).len(), 0);
        Ok(())
    }
}
