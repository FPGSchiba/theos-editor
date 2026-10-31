//! The umbrella spec (`docs/specs/00-umbrella.md`): the phases, the repository each one is built
//! in and the acceptance criteria each one has.
//!
//! The phase table of §3 gives the name and the repository of every phase. The acceptance
//! criteria are the list items that start with an id (`- CE-08.6 …`) under the heading
//! `### PNN · Name` of their phase; an id is `CE-`, two digits, an optional capital letter, a
//! dot and a number. A list item that starts `- CE-` anywhere else in a phase section must be
//! such an id, so a typo in the spec is an error and not a criterion that silently does not
//! exist. `ac-trace` and `commit-msg` both read their ids from here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::branch::Phase;
use crate::{Error, Result};

/// Where the umbrella spec is, relative to the repository root.
pub const PATH: &str = "docs/specs/00-umbrella.md";

/// One phase of the build.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PhaseInfo {
    /// The phase number.
    pub id: Phase,
    /// The name in the phase table, such as `Foundation and gate`.
    pub name: String,
    /// The repository the phase is built in, such as `theos-editor` or `tower-of-theos`.
    pub repo: String,
    /// The acceptance criteria of the phase, in the order the spec lists them.
    pub acs: Vec<String>,
}

/// The phases of the umbrella spec.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Umbrella {
    phases: Vec<PhaseInfo>,
}

impl Umbrella {
    /// Every acceptance criterion of every phase.
    #[must_use]
    pub fn all_ids(&self) -> BTreeSet<String> {
        self.phases
            .iter()
            .flat_map(|phase| phase.acs.iter().cloned())
            .collect()
    }

    /// The phase `phase`, or `None` when the spec has no such phase.
    #[must_use]
    pub fn phase(&self, phase: Phase) -> Option<&PhaseInfo> {
        self.phases.iter().find(|info| info.id == phase)
    }
}

/// Reads and parses the umbrella spec of the repository at `root`.
///
/// # Errors
///
/// Fails when the file cannot be read or does not parse, and with [`Error::Config`] when it lists
/// no acceptance criterion at all: every id would then be unknown.
pub fn load(root: &Path) -> Result<Umbrella> {
    let path = root.join(PATH);
    let text = fs::read_to_string(&path).map_err(|source| Error::Io { path, source })?;
    let umbrella = parse(&text)?;
    if umbrella.all_ids().is_empty() {
        return Err(Error::Config {
            file: PATH.to_owned(),
            message: "lists no acceptance criteria (`- CE-NN.N …` items under `### PNN ·` \
                      headings)"
                .to_owned(),
        });
    }
    Ok(umbrella)
}

/// Parses the text of the umbrella spec.
///
/// # Errors
///
/// Fails, naming the line, when the phase table has no `Name` and `Repo` column, a row of it is
/// too short or names a phase twice, a phase has two sections, a section has no row in the table,
/// an item starting `- CE-` does not start with an id, or an id is listed twice.
pub fn parse(text: &str) -> Result<Umbrella> {
    let mut columns: Option<Columns> = None;
    let mut rows: Vec<(Phase, String, String)> = Vec::new();
    let mut sections: BTreeMap<Phase, Vec<String>> = BTreeMap::new();
    let mut current: Option<Phase> = None;
    let mut seen: BTreeSet<String> = BTreeSet::new();

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some(title) = heading(line) {
            current = section_phase(title);
            if let Some(phase) = current
                && sections.insert(phase, Vec::new()).is_some()
            {
                return Err(error(format!("{phase} has two sections (line {number})")));
            }
        } else if let Some(cells) = table_cells(line) {
            if cells.first() == Some(&"Phase") {
                columns = Some(Columns::of(&cells).ok_or_else(|| {
                    error(format!(
                        "the phase table (line {number}) has no `Name` and `Repo` column"
                    ))
                })?);
            } else if let (Some(Ok(phase)), Some(columns)) =
                (cells.first().map(|cell| cell.parse::<Phase>()), &columns)
            {
                let (Some(name), Some(repo)) = (cells.get(columns.name), cells.get(columns.repo))
                else {
                    return Err(error(format!(
                        "the row of {phase} in the phase table (line {number}) is too short"
                    )));
                };
                if rows.iter().any(|(known, _, _)| *known == phase) {
                    return Err(error(format!(
                        "{phase} is in the phase table twice (line {number})"
                    )));
                }
                rows.push((phase, (*name).to_owned(), (*repo).to_owned()));
            }
        } else if let (Some(item), Some(phase)) = (line.strip_prefix("- CE-"), current) {
            let id = format!("CE-{}", item.split_whitespace().next().unwrap_or_default());
            if !is_ac_id(&id) {
                return Err(error(format!(
                    "line {number} is a list item that starts with `- CE-` but not with an \
                     acceptance criterion id (`CE-NN.N`, `CE-NNR.N`)"
                )));
            }
            if !seen.insert(id.clone()) {
                return Err(error(format!("{id} is listed twice (line {number})")));
            }
            sections.entry(phase).or_default().push(id);
        }
    }

    if rows.is_empty() {
        return Err(error(
            "there is no phase table (a table with the columns `Phase`, `Name` and `Repo`)"
                .to_owned(),
        ));
    }
    let mut phases = Vec::with_capacity(rows.len());
    for (id, name, repo) in rows {
        let acs = sections.remove(&id).unwrap_or_default();
        phases.push(PhaseInfo {
            id,
            name,
            repo,
            acs,
        });
    }
    if let Some(phase) = sections.keys().next() {
        return Err(error(format!(
            "{phase} has a section but no row in the phase table"
        )));
    }
    Ok(Umbrella { phases })
}

/// Where the columns the check needs are in the phase table.
struct Columns {
    name: usize,
    repo: usize,
}

impl Columns {
    fn of(header: &[&str]) -> Option<Self> {
        let position = |title: &str| header.iter().position(|cell| *cell == title);
        Some(Self {
            name: position("Name")?,
            repo: position("Repo")?,
        })
    }
}

fn error(message: String) -> Error {
    Error::Parse {
        what: "the umbrella spec".to_owned(),
        message,
    }
}

/// The text of a heading of level 1 to 3 (`#`, `##` or `###`, a space, then the text).
///
/// Deeper headings stay inside the section they are in.
fn heading(line: &str) -> Option<&str> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=3).contains(&hashes) {
        line[hashes..].strip_prefix(' ')
    } else {
        None
    }
}

/// The phase of a heading `PNN · Name`; `None` for any other heading.
fn section_phase(title: &str) -> Option<Phase> {
    let (word, rest) = title.split_once(' ')?;
    let phase = word.parse().ok()?;
    rest.trim_start().starts_with('·').then_some(phase)
}

/// The trimmed cells of a table row, without the bars at both ends.
fn table_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.trim().strip_prefix('|')?;
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    Some(inner.split('|').map(str::trim).collect())
}

/// `CE-`, two digits, an optional capital letter, a dot and at least one digit.
fn is_ac_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("CE-") else {
        return false;
    };
    let Some((group, number)) = rest.split_once('.') else {
        return false;
    };
    let group = group
        .strip_suffix(|c: char| c.is_ascii_uppercase())
        .unwrap_or(group);
    group.len() == 2
        && group.bytes().all(|b| b.is_ascii_digit())
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    const REAL: &str = include_str!("../../docs/specs/00-umbrella.md");

    fn phase(text: &str) -> TestResult<Phase> {
        Ok(text.parse()?)
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    /// A small umbrella: a table of two phases and the sections `body` holds.
    fn excerpt(body: &str) -> String {
        format!(
            "## 3. Phases\n\n\
             | Phase | Name | Repo | Slice | Depends on |\n\
             |---|---|---|---|---|\n\
             | P90 | Fixture phase | theos-editor | S2 | – |\n\
             | P91 | Elsewhere | tower-of-theos | S1 | – |\n\n\
             {body}"
        )
    }

    // AC: CE-08.6
    #[test]
    fn parses_phases_repos_and_ids() -> TestResult {
        let umbrella = parse(REAL)?;

        let p00 = umbrella.phase(phase("P00")?).ok_or("no P00")?;
        let expected: Vec<String> = (1..=12).map(|n| format!("CE-08.{n}")).collect();
        assert_eq!(p00.acs, expected);
        assert_eq!(p00.name, "Foundation and gate");
        assert_eq!(p00.repo, "theos-editor");

        let p05 = umbrella.phase(phase("P05")?).ok_or("no P05")?;
        assert!(p05.acs.iter().any(|id| id == "CE-08R.1"), "{:?}", p05.acs);

        let p01 = umbrella.phase(phase("P01")?).ok_or("no P01")?;
        assert_eq!(p01.repo, "tower-of-theos");
        let p08 = umbrella.phase(phase("P08")?).ok_or("no P08")?;
        assert_eq!(p08.repo, "tower-of-theos");

        // The ids of the work outside the build (§4) are not criteria.
        assert!(!umbrella.all_ids().contains("CE-02"));
        assert!(umbrella.all_ids().contains("CE-06.1"));
        assert!(umbrella.phase(phase("P19")?).is_none());
        Ok(())
    }

    #[test]
    fn every_phase_of_the_real_umbrella_has_criteria() -> TestResult {
        let umbrella = parse(REAL)?;
        for number in 0..=18 {
            let id = phase(&format!("P{number:02}"))?;
            let info = umbrella.phase(id).ok_or(format!("no {id}"))?;
            assert!(!info.acs.is_empty(), "{id} has no criteria");
        }
        Ok(())
    }

    #[test]
    fn criteria_belong_to_the_section_they_are_in() -> TestResult {
        let text = excerpt(
            "### P90 · Fixture phase\n\n\
             **CE-90 Group**\n\
             - CE-90.1 First.\n\
             - CE-90R.2 Second, with a letter.\n\
             #### A deeper heading stays in the section\n\
             - CE-90.3 Third.\n\n\
             ### P91 · Elsewhere (tower-of-theos)\n\
             - CE-91.1 Other.\n\n\
             ## 4. Work outside the build\n\
             - CE-92.1 Not in any phase.\n",
        );
        let umbrella = parse(&text)?;
        let p90 = umbrella.phase(phase("P90")?).ok_or("no P90")?;
        assert_eq!(p90.acs, ids(&["CE-90.1", "CE-90R.2", "CE-90.3"]));
        let p91 = umbrella.phase(phase("P91")?).ok_or("no P91")?;
        assert_eq!(p91.acs, ids(&["CE-91.1"]));
        assert_eq!(
            umbrella.all_ids().into_iter().collect::<Vec<_>>(),
            ids(&["CE-90.1", "CE-90.3", "CE-90R.2", "CE-91.1"])
        );
        Ok(())
    }

    #[test]
    fn table_columns_may_come_in_any_order() -> TestResult {
        let text = "| Phase | Repo | Name |\n|---|---|---|\n| P90 | theos-editor | A phase |\n";
        let umbrella = parse(text)?;
        let info = umbrella.phase(phase("P90")?).ok_or("no P90")?;
        assert_eq!(
            (info.name.as_str(), info.repo.as_str()),
            ("A phase", "theos-editor")
        );
        Ok(())
    }

    #[test]
    fn a_list_item_that_is_not_an_id_is_an_error() {
        for item in [
            "CE-8.1 One digit.",
            "CE-081.1 Three digits.",
            "CE-90. None.",
            "CE-90.1: x",
        ] {
            let text = excerpt(&format!("### P90 · Fixture phase\n- {item}\n"));
            let result = parse(&text);
            assert!(
                matches!(result, Err(Error::Parse { .. })),
                "{item}: {result:?}"
            );
        }
    }

    #[test]
    fn a_broken_spec_is_an_error() {
        let two_sections = excerpt("### P90 · A\n- CE-90.1 x\n### P90 · B\n- CE-90.2 y\n");
        let no_row = excerpt("### P92 · No row\n- CE-92.1 x\n");
        let twice = excerpt("### P90 · A\n- CE-90.1 x\n- CE-90.1 y\n");
        let twice_elsewhere = excerpt("### P90 · A\n- CE-90.1 x\n### P91 · B\n- CE-90.1 y\n");
        let no_table = "### P90 · A\n- CE-90.1 x\n".to_owned();
        let no_repo = "| Phase | Name |\n|---|---|\n| P90 | A |\n".to_owned();
        let short_row = "| Phase | Name | Repo |\n|---|---|---|\n| P90 | A |\n".to_owned();
        let table_twice = excerpt("| P90 | Again | theos-editor |\n");
        for (name, text) in [
            ("two sections", two_sections),
            ("no row", no_row),
            ("id twice", twice),
            ("id in two phases", twice_elsewhere),
            ("no table", no_table),
            ("no repo column", no_repo),
            ("short row", short_row),
            ("phase twice in the table", table_twice),
        ] {
            let result = parse(&text);
            assert!(
                matches!(result, Err(Error::Parse { .. })),
                "{name}: {result:?}"
            );
        }
    }

    #[test]
    fn crlf_line_ends_parse_like_lf() -> TestResult {
        let text = excerpt("### P90 · A\n- CE-90.1 x\n").replace('\n', "\r\n");
        let umbrella = parse(&text)?;
        let info = umbrella.phase(phase("P91")?).ok_or("no P91")?;
        assert_eq!(info.repo, "tower-of-theos");
        assert_eq!(umbrella.all_ids().len(), 1);
        Ok(())
    }

    #[test]
    fn load_reads_the_umbrella_of_a_repository() -> TestResult {
        let dir = tempfile::tempdir()?;
        let result = load(dir.path());
        assert!(matches!(result, Err(Error::Io { .. })), "{result:?}");

        let specs = dir.path().join("docs").join("specs");
        fs::create_dir_all(&specs)?;
        let file = specs.join("00-umbrella.md");
        fs::write(&file, excerpt("### P90 · A\n- CE-90.1 x\n"))?;
        assert!(load(dir.path())?.all_ids().contains("CE-90.1"));

        // With no criterion at all every id would be unknown, so that is a broken spec.
        fs::write(&file, excerpt("### P90 · A\nNothing yet.\n"))?;
        let result = load(dir.path());
        assert!(matches!(result, Err(Error::Config { .. })), "{result:?}");
        Ok(())
    }
}
