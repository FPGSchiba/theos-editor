//! Opening, syncing, updating and applying: what is read, what is re-parsed, what is kept.

use std::time::{Duration, SystemTime};

use theos_content::kinds::{BodyGrammar, IdRule, KindSpec};
use theos_content::{Change, FieldSpec, FieldType, Index, Placement, Update, parts};

use crate::common::{TempProject, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn thing(id: &str, title: &str) -> String {
    format!(
        "---\n# synthetic fixture (P03), not canon\nid: {id}\nkind: thing\ntitle: {title}\ntype: synthetic\n---\nsynthetic body\n"
    )
}

fn paths(list: &[&str]) -> Vec<String> {
    list.iter().map(|p| (*p).to_owned()).collect()
}

fn three(project: &TempProject) -> TestResult {
    for name in ["a", "b", "c"] {
        project.write(
            &format!("things/{name}.md"),
            &thing(name, "synthetic title"),
        )?;
    }
    Ok(())
}

fn widget() -> KindSpec {
    KindSpec {
        name: "synthetic-widget",
        id: IdRule {
            pattern: "^[a-z0-9]+(-[a-z0-9]+)*$",
            says: "a slug",
        },
        placement: Placement::Flat("widgets"),
        fields: vec![
            FieldSpec {
                key: "id",
                ty: FieldType::Id,
                required: true,
            },
            FieldSpec {
                key: "kind",
                ty: FieldType::KindName,
                required: true,
            },
            FieldSpec {
                key: "colour",
                ty: FieldType::Text,
                required: true,
            },
        ],
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}

// AC: CE-12.2
#[test]
fn a_second_open_reads_no_file() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let registry = parts::builtin()?;
    let (first, update) = Index::open(&project.project()?, &registry)?;
    let all = paths(&["things/a.md", "things/b.md", "things/c.md"]);
    assert_eq!(update.read, all);
    assert_eq!(update.reparsed, all);
    assert_eq!(update.removed.len(), 0);
    drop(first);
    assert!(project.root().join(".theos/cache/index.sqlite").is_file());
    let (_second, update) = Index::open(&project.project()?, &registry)?;
    assert_eq!(update, Update::default());
    Ok(())
}

// AC: CE-12.2
#[test]
fn changing_one_file_reads_only_that_file() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    project.write("things/b.md", &thing("b", "a much longer synthetic title"))?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&["things/b.md"]));
    assert_eq!(update.reparsed, paths(&["things/b.md"]));
    assert_eq!(update.removed.len(), 0);
    let record = index.record("b")?;
    assert_eq!(
        record.first().and_then(|r| r.title.as_deref()),
        Some("a much longer synthetic title")
    );
    Ok(())
}

// AC: CE-11.5
#[test]
fn a_test_part_kind_indexes_and_checks() -> TestResult {
    let project = TempProject::new()?;
    let mut registry = parts::builtin()?;
    registry.register("test", widget())?;
    let mut index = Index::in_memory(&project.project()?, &registry)?;
    let without = "---\nid: w1\nkind: synthetic-widget\n---\n";
    index.apply(&[Change::text("widgets/w1.md", without)])?;
    assert_eq!(
        keys(&index.findings()?),
        vec![(
            "widgets/w1.md".to_owned(),
            "schema/required".to_owned(),
            "error".to_owned(),
            3,
            7
        )]
    );
    let with = "---\nid: w1\nkind: synthetic-widget\ncolour: red\n---\n";
    index.apply(&[Change::text("widgets/w1.md", with)])?;
    assert_eq!(index.findings()?.len(), 0);
    assert_eq!(index.records_of_kind("synthetic-widget")?.len(), 1);

    let mut plain = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    plain.apply(&[Change::text("widgets/w1.md", with)])?;
    let rules: Vec<String> = plain.findings()?.into_iter().map(|f| f.rule).collect();
    assert_eq!(rules, vec!["schema/kind".to_owned()]);
    Ok(())
}

#[test]
fn racy_files_are_read_again() -> TestResult {
    let project = TempProject::new()?;
    project.write_fresh("things/a.md", &thing("a", "synthetic one"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    let update = index.sync()?;
    assert_eq!(update.reparsed, paths(&["things/a.md"]));
    let stored = project.time("things/a.md")?;
    project.write_fresh("things/a.md", &thing("a", "synthetic two"))?;
    project.set_time("things/a.md", stored)?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed, paths(&["things/a.md"]));
    project.set_time("things/a.md", SystemTime::now() - Duration::from_secs(60))?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed.len(), 0);
    assert_eq!(index.sync()?, Update::default());
    Ok(())
}

#[test]
fn a_time_in_the_future_stays_racy() -> TestResult {
    let project = TempProject::new()?;
    project.write("things/a.md", &thing("a", "synthetic one"))?;
    project.set_time("things/a.md", SystemTime::now() + Duration::from_secs(3600))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    assert_eq!(index.sync()?.reparsed, paths(&["things/a.md"]));
    for _ in 0..2 {
        let update = index.sync()?;
        assert_eq!(update.read, paths(&["things/a.md"]));
        assert_eq!(update.reparsed.len(), 0);
    }
    Ok(())
}

#[test]
fn an_older_copy_is_read() -> TestResult {
    let project = TempProject::new()?;
    project.write("things/a.md", &thing("a", "synthetic one"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    project.write("things/a.md", &thing("a", "synthetic two"))?;
    project.set_time("things/a.md", SystemTime::now() - Duration::from_secs(3600))?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed, paths(&["things/a.md"]));
    Ok(())
}

#[test]
fn same_bytes_only_update_the_times() -> TestResult {
    let project = TempProject::new()?;
    project.write("things/a.md", &thing("a", "synthetic one"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    project.set_time("things/a.md", SystemTime::now() - Duration::from_secs(600))?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed.len(), 0);
    assert_eq!(index.sync()?, Update::default());
    Ok(())
}

#[test]
fn apply_gives_the_dump_of_the_files() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let registry = parts::builtin()?;
    let mut synced = Index::in_memory(&project.project()?, &registry)?;
    synced.sync()?;
    let mut applied = Index::in_memory(&project.project()?, &registry)?;
    let changes: Vec<Change> = ["a", "b", "c"]
        .iter()
        .map(|n| Change::text(format!("things/{n}.md"), thing(n, "synthetic title")))
        .collect();
    let update = applied.apply(&changes)?;
    assert_eq!(update.read.len(), 0);
    assert_eq!(update.reparsed.len(), 3);
    assert_eq!(applied.dump()?, synced.dump()?);
    assert_ne!(synced.dump()?.len(), 0);
    let again = applied.apply(&changes)?;
    assert_eq!(again.reparsed.len(), 0);
    Ok(())
}

#[test]
fn not_records_are_ignored_by_update_and_apply() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    project.write("README.md", "# synthetic\n")?;
    project.write("templates/records/x.md", "synthetic\n")?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let before = index.dump()?;
    let update = index.update(&["README.md", "templates/records/x.md"])?;
    assert_eq!(update, Update::default());
    let update = index.apply(&[Change::text("templates/records/x.md", "---\n---\n")])?;
    assert_eq!(update, Update::default());
    assert_eq!(index.dump()?, before);
    Ok(())
}

#[test]
fn update_reads_only_the_given_paths() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    let update = index.update(&["things/a.md", "things/gone.md"])?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed, paths(&["things/a.md"]));
    assert_eq!(update.removed.len(), 0);
    assert_eq!(index.records()?.len(), 1);
    Ok(())
}

#[test]
fn a_deleted_file_is_removed() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    project.write("things/d.md", "no front matter\n")?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    assert_ne!(index.findings_for("things/d.md")?.len(), 0);
    project.remove("things/d.md")?;
    project.remove("things/b.md")?;
    let update = index.sync()?;
    assert_eq!(update.removed, paths(&["things/b.md", "things/d.md"]));
    assert_eq!(update.read.len(), 0);
    let dump = index.dump()?;
    assert!(!dump.contains("things/d.md"));
    assert!(!dump.contains("things/b.md"));
    assert_eq!(index.findings()?.len(), 0);
    assert_eq!(index.record("b")?.len(), 0);
    let update = index.apply(&[Change::Removed {
        path: "things/a.md".to_owned(),
    }])?;
    assert_eq!(update.removed, paths(&["things/a.md"]));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn a_non_utf8_file_name_is_a_finding() -> TestResult {
    use std::os::unix::ffi::OsStrExt;
    let project = TempProject::new()?;
    let name = std::ffi::OsStr::from_bytes(b"x\xff.md");
    let file = project.root().join(name);
    std::fs::write(&file, "synthetic\n")?;
    std::fs::File::options()
        .write(true)
        .open(&file)?
        .set_modified(SystemTime::now() - Duration::from_secs(60))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    assert_eq!(
        keys(&index.findings()?),
        vec![(
            "x\u{fffd}.md".to_owned(),
            "record/path".to_owned(),
            "error".to_owned(),
            0,
            0
        )]
    );
    assert_eq!(index.sync()?, Update::default());
    Ok(())
}

#[test]
fn findings_are_sorted_and_print_like_p05() -> TestResult {
    let project = TempProject::new()?;
    project.write("things/b.md", "---\nid: b\nkind: thing\nzzz: 1\n---\n")?;
    project.write("things/a.md", "---\nid: a\nkind: thing\nzzz: 1\n---\n")?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let findings = index.findings()?;
    let found = keys(&findings);
    let mut sorted = found.clone();
    sorted.sort_by(|a, b| (&a.0, a.3, a.4, &a.1).cmp(&(&b.0, b.3, b.4, &b.1)));
    assert_eq!(found, sorted);
    assert_eq!(found.first().map(|k| k.0.as_str()), Some("things/a.md"));
    assert_eq!(found.last().map(|k| k.0.as_str()), Some("things/b.md"));
    assert_eq!(index.findings_for("things/b.md")?.len() * 2, found.len());
    let lines: Vec<String> = findings.iter().map(ToString::to_string).collect();
    let expected = [
        "things/a.md:3:7: error schema/required: a thing needs `title`",
        "things/a.md:3:7: error schema/required: a thing needs `type`",
        "things/a.md:4:1: warning schema/unknown-field: `zzz` is not a field of a thing",
        "things/b.md:3:7: error schema/required: a thing needs `title`",
        "things/b.md:3:7: error schema/required: a thing needs `type`",
        "things/b.md:4:1: warning schema/unknown-field: `zzz` is not a field of a thing",
    ];
    assert_eq!(lines, expected);
    Ok(())
}

// AC: CE-12.2
#[test]
fn backslashes_in_given_paths_are_read_as_slashes() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    project.write("things/a.md", &thing("a", "a much longer synthetic title"))?;
    let update = index.update(&["things\\a.md"])?;
    assert_eq!(update.read, paths(&["things/a.md"]));
    assert_eq!(update.reparsed, paths(&["things/a.md"]));

    let added = Change::text("things\\d.md", thing("d", "synthetic d"));
    let preview = index.preview(std::slice::from_ref(&added))?;
    assert_eq!(preview.update.reparsed, paths(&["things/d.md"]));
    let update = index.apply(&[added])?;
    assert_eq!(update.reparsed, paths(&["things/d.md"]));
    let stored: Vec<String> = index.record("d")?.into_iter().map(|r| r.path).collect();
    assert_eq!(stored, paths(&["things/d.md"]));

    let update = index.apply(&[Change::Removed {
        path: "things\\d.md".to_owned(),
    }])?;
    assert_eq!(update.removed, paths(&["things/d.md"]));
    assert_eq!(index.update(&["C:\\things\\b.md"])?, Update::default());
    Ok(())
}

const LINKED: &str = "things/synthetic-linked.md";
const FIRST: &str = "things/synthetic-first.md";
const SECOND: &str = "things/synthetic-second.md";

/// A thing with the given body.
fn thing_with(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\n# synthetic fixture (P03), not canon\nid: {id}\nkind: thing\ntitle: {title}\ntype: synthetic\n---\n{body}\n"
    )
}

/// A project of [`LINKED`], which has a dangling link, and two things that link to it.
fn linked_project() -> Result<(TempProject, Index), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    project.write(
        LINKED,
        &thing_with(
            "synthetic-linked",
            "synthetic linked",
            "See [[synthetic-nowhere]].",
        ),
    )?;
    for (path, id) in [(FIRST, "synthetic-first"), (SECOND, "synthetic-second")] {
        project.write(
            path,
            &thing_with(id, "synthetic", "See [[synthetic-linked]]."),
        )?;
    }
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok((project, index))
}

/// The dump of a fresh in-memory index over the folder of `project`.
fn fresh_dump(project: &TempProject) -> Result<String, Box<dyn std::error::Error>> {
    let mut fresh = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    fresh.sync()?;
    Ok(fresh.dump()?)
}

fn dangles(findings: &[theos_content::Finding]) -> bool {
    findings.iter().any(|f| {
        f.path == LINKED && f.rule == "link/dangling" && f.message.contains("synthetic-nowhere")
    })
}

// AC: CE-12.2
#[test]
fn a_path_removed_and_given_again_in_one_list_is_reparsed() -> TestResult {
    let (project, mut index) = linked_project()?;
    let text = thing_with(
        "synthetic-linked",
        "synthetic linked, retitled",
        "See [[synthetic-nowhere]].",
    );
    project.write(LINKED, &text)?;
    let changes = [
        Change::Removed {
            path: LINKED.to_owned(),
        },
        Change::text(LINKED, text),
    ];
    let expected = Update {
        read: Vec::new(),
        reparsed: paths(&[LINKED]),
        removed: Vec::new(),
        relinked: paths(&[FIRST, SECOND]),
    };
    let before = index.dump()?;
    let preview = index.preview(&changes)?;
    assert_eq!(preview.update, expected);
    assert!(dangles(&preview.findings), "{:#?}", preview.findings);
    assert_eq!(index.dump()?, before);

    assert_eq!(index.apply(&changes)?, expected);
    assert!(dangles(&index.findings_for(LINKED)?));
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

// AC: CE-12.2
#[test]
fn a_path_given_and_removed_in_one_list_is_removed() -> TestResult {
    let (project, mut index) = linked_project()?;
    project.remove(LINKED)?;
    let changes = [
        Change::text(
            LINKED,
            thing_with("synthetic-linked", "synthetic linked, retitled", "x"),
        ),
        Change::Removed {
            path: LINKED.to_owned(),
        },
    ];
    let expected = Update {
        read: Vec::new(),
        reparsed: Vec::new(),
        removed: paths(&[LINKED]),
        relinked: paths(&[FIRST, SECOND]),
    };
    assert_eq!(index.preview(&changes)?.update, expected);
    assert_eq!(index.apply(&changes)?, expected);
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

#[test]
fn paths_outside_the_walk_are_ignored() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let outside = project.root().join("../outside.md");
    std::fs::write(&outside, thing("outside", "synthetic outside"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let before = index.dump()?;
    let result = index.update(&["../outside.md"]);
    std::fs::remove_file(&outside)?;
    assert_eq!(result?, Update::default());
    let hidden = Change::text(".obsidian/x.md", thing("x", "synthetic hidden"));
    assert_eq!(index.apply(&[hidden])?, Update::default());
    assert_eq!(index.dump()?, before);
    Ok(())
}

// AC: CE-12.2
#[test]
fn gitignored_paths_are_ignored_by_update_apply_and_preview() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    project.write(".gitignore", "drafts/\n")?;
    project.write("drafts/d.md", &thing("d", "synthetic draft"))?;
    project.write("sub/.gitignore", "e.md\n")?;
    project.write("sub/e.md", &thing("e", "synthetic e"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let before = index.dump()?;
    assert_eq!(before, fresh_dump(&project)?);

    assert_eq!(
        index.update(&["drafts/d.md", "sub/e.md"])?,
        Update::default()
    );
    let given = [
        Change::text("drafts/d.md", thing("d", "synthetic draft, edited")),
        Change::text("sub/e.md", thing("e", "synthetic e, edited")),
    ];
    assert_eq!(index.preview(&given)?.update, Update::default());
    assert_eq!(index.apply(&given)?, Update::default());
    assert_eq!(index.dump()?, before);

    // A record that a new `.gitignore` line hides is no record any more: a change of it
    // removes what the index holds, as the next sync would.
    project.write(".gitignore", "drafts/\nthings/c.md\n")?;
    let update = index.update(&["things/c.md"])?;
    assert_eq!(update.removed, paths(&["things/c.md"]));
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

// AC: CE-12.2
#[test]
fn a_path_the_walk_would_not_list_is_not_read() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let before = index.dump()?;
    // Another spelling of a file name, and a path through a file.
    let update = index.update(&["Things/a.md", "things/A.md", "things/a.md/x.md"])?;
    assert_eq!(update, Update::default());
    assert_eq!(index.dump()?, before);

    // A rename that changes only the case: the old spelling is gone, the new one is a record.
    std::fs::rename(
        project.root().join("things/a.md"),
        project.root().join("things/A.md"),
    )?;
    let update = index.update(&["things/a.md", "things/A.md"])?;
    assert_eq!(update.removed, paths(&["things/a.md"]));
    assert_eq!(update.reparsed, paths(&["things/A.md"]));
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

/// The walk does not follow symbolic links; `update` does not read one either.
#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_read() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let before = index.dump()?;
    std::os::unix::fs::symlink(
        project.root().join("things/a.md"),
        project.root().join("things/l.md"),
    )?;
    std::os::unix::fs::symlink(project.root().join("things"), project.root().join("linked"))?;
    assert_eq!(
        index.update(&["things/l.md", "linked/b.md"])?,
        Update::default()
    );
    assert_eq!(index.dump()?, before);
    Ok(())
}

/// The paths of the records the index holds, sorted.
fn stored(index: &Index) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(index.records()?.into_iter().map(|r| r.path).collect())
}

// AC: CE-12.2
/// The folders the spec names hold no records, at any depth and on every OS: the walk skips
/// them, and `update`, `apply` and `preview` ignore them.
#[test]
fn named_folders_are_never_records() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let named = [
        ".theos/x.md",
        ".git/x.md",
        ".obsidian/x.md",
        ".vscode/x.md",
        ".github/x.md",
        ".worktrees/w/things/x.md",
        ".trash/x.md",
        "things/.trash/x.md",
        "lore/deep/.obsidian/x.md",
    ];
    for path in named {
        project.write(path, &thing("x", "synthetic named"))?;
    }
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let all = paths(&["things/a.md", "things/b.md", "things/c.md"]);
    assert_eq!(stored(&index)?, all);
    let before = index.dump()?;
    assert_eq!(index.update(&named)?, Update::default());
    let given: Vec<Change> = named
        .iter()
        .map(|path| Change::text(*path, thing("x", "synthetic named, edited")))
        .collect();
    assert_eq!(index.preview(&given)?.update, Update::default());
    assert_eq!(index.apply(&given)?, Update::default());
    assert_eq!(index.dump()?, before);
    Ok(())
}

// AC: CE-12.2
/// On Unix a name that starts with `.` hides a file or folder: the walk skips `.drafts/a.md` and
/// `lore/.hidden.md`, and `update`, `apply` and `preview` ignore them.
#[cfg(unix)]
#[test]
fn a_dot_name_hides_on_unix() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    let hidden = [".drafts/a.md", "lore/.hidden.md"];
    for path in hidden {
        project.write(path, &thing("synthetic-hidden", "synthetic hidden"))?;
    }
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let all = paths(&["things/a.md", "things/b.md", "things/c.md"]);
    assert_eq!(stored(&index)?, all);
    let before = index.dump()?;
    assert_eq!(index.update(&hidden)?, Update::default());
    let given: Vec<Change> = hidden
        .iter()
        .map(|path| Change::text(*path, thing("synthetic-hidden", "synthetic, edited")))
        .collect();
    assert_eq!(index.preview(&given)?.update, Update::default());
    assert_eq!(index.apply(&given)?, Update::default());
    assert_eq!(index.dump()?, before);
    Ok(())
}

// AC: CE-12.2
/// On Windows the hidden attribute of a file, or of a folder above it, hides it: the walk skips
/// it, and `update`, `apply` and `preview` agree with the walk.
#[cfg(windows)]
#[test]
fn the_hidden_attribute_hides_on_windows() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    project.write("secret/e.md", &thing("e", "synthetic secret"))?;
    project.hide("things/b.md")?;
    project.hide("secret")?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    assert_eq!(stored(&index)?, paths(&["things/a.md", "things/c.md"]));
    let before = index.dump()?;
    assert_eq!(
        index.update(&["things/b.md", "secret/e.md"])?,
        Update::default()
    );
    let given: Vec<Change> = ["things/b.md", "secret/e.md", "secret/new.md"]
        .iter()
        .map(|path| Change::text(*path, thing("synthetic-given", "synthetic given")))
        .collect();
    assert_eq!(index.preview(&given)?.update, Update::default());
    assert_eq!(index.apply(&given)?, Update::default());
    assert_eq!(index.dump()?, before);

    // A record that gets the attribute is no record any more: a change of it removes what the
    // index holds, as the next sync would.
    project.hide("things/c.md")?;
    let update = index.update(&["things/c.md"])?;
    assert_eq!(update.removed, paths(&["things/c.md"]));
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

// AC: CE-12.2
/// On Windows a name that starts with `.` hides nothing: the walk lists `.drafts/d.md`, and
/// `update` and `apply` take it like any record.
#[cfg(windows)]
#[test]
fn a_dot_name_does_not_hide_on_windows() -> TestResult {
    let project = TempProject::new()?;
    three(&project)?;
    project.write(".drafts/d.md", &thing("d", "synthetic draft"))?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let all = paths(&[".drafts/d.md", "things/a.md", "things/b.md", "things/c.md"]);
    assert_eq!(stored(&index)?, all);

    project.write(".drafts/d.md", &thing("d", "synthetic draft, edited"))?;
    assert_eq!(
        index.update(&[".drafts/d.md"])?.reparsed,
        paths(&[".drafts/d.md"])
    );
    let text = thing("f", "synthetic given");
    let given = Change::text(".drafts/f.md", text.clone());
    assert_eq!(index.apply(&[given])?.reparsed, paths(&[".drafts/f.md"]));
    project.write(".drafts/f.md", &text)?;
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}
