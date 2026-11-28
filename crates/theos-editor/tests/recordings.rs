//! The recorded IPC answers that the UI's page tests replay (spec §5.12): a fixed list of calls
//! on copies of the fixture projects, each written to `editor-ui/e2e/ipc/<name>.json`. A
//! checked-in file that differs, a missing one and an extra one fail; `UPDATE_BINDINGS=1` writes
//! them instead.
//!
//! Project roots become `<project>` with `/` separators, fractions are rounded to six places and
//! the app's version is `<version>`, so the files are the same on every OS and across releases.
//! A variant the app cannot be brought into here is derived from a real answer and says so in its
//! `derived` key. A call on another fixture than `editor-codex` names it in its `fixture` key;
//! such a recording answers a page test only when the test names it.

mod generated;
mod support;

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use support::{TestResult, app, app_with, call, copy_fixture, default_app};
use tauri::WebviewWindow;
use tauri::test::MockRuntime;
use theos_editor::AppState;
use theos_studio::Workspace;

/// The folder of the recordings, relative to the repository root.
const FOLDER: &str = "editor-ui/e2e/ipc";

/// The `limit` of every recorded search; the find field sends the same.
const LIMIT: u32 = 20;

/// The queries the page tests type: the variant's name and the query.
const SEARCHES: [(&str, &str); 5] = [
    ("f04", "#f04"),
    ("clue", ":clue"),
    ("murmur", ":clue murmur"),
    ("mother-ilse", "Mother Ilse"),
    ("nokind", ":nokind"),
];

/// The fixture the page tests open; recordings of other fixtures name theirs.
const MAIN: &str = "editor-codex";

/// The tabs of the Codex tree.
const TABS: [&str; 4] = ["where", "who", "lore", "things"];

/// The OS values of `app_info`.
const OSES: [&str; 3] = ["windows", "macos", "linux"];

type Window = WebviewWindow<MockRuntime>;
type Error = Box<dyn std::error::Error>;

/// A copy of a fixture project in a temporary folder, and the spellings of its root.
struct Project {
    temp: tempfile::TempDir,
    roots: Vec<String>,
    fixture: &'static str,
}

impl Project {
    fn copy(fixture: &'static str) -> Result<Project, Error> {
        let temp = tempfile::tempdir()?;
        copy_fixture(fixture, temp.path())?;
        let mut roots = vec![
            temp.path().display().to_string(),
            std::path::absolute(temp.path())?.display().to_string(),
        ];
        let canonical = std::fs::canonicalize(temp.path())?.display().to_string();
        roots.push(canonical.trim_start_matches(r"\\?\").to_owned());
        roots.push(canonical);
        roots.sort_by_key(|root| std::cmp::Reverse(root.len()));
        roots.dedup();
        Ok(Project {
            temp,
            roots,
            fixture,
        })
    }

    /// The fixture's name when it is not [`MAIN`].
    fn other_fixture(&self) -> Option<&'static str> {
        (self.fixture != MAIN).then_some(self.fixture)
    }

    fn path(&self) -> &Path {
        self.temp.path()
    }

    /// `value` with the root replaced by `<project>`, `/` separators in the strings that held it,
    /// and fractions (search scores) rounded to six places, so a last-bit difference of the C
    /// library's `log` on another OS does not change the file.
    fn portable(&self, value: Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.replace(text)),
            Value::Array(items) => {
                Value::Array(items.into_iter().map(|v| self.portable(v)).collect())
            }
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(key, v)| (key, self.portable(v)))
                    .collect(),
            ),
            Value::Number(number) if !(number.is_i64() || number.is_u64()) => number
                .as_f64()
                .and_then(|fraction| serde_json::Number::from_f64((fraction * 1e6).round() / 1e6))
                .map_or(Value::Number(number), Value::Number),
            other => other,
        }
    }

    fn replace(&self, text: String) -> String {
        match self.roots.iter().find(|root| text.contains(root.as_str())) {
            Some(root) => text.replace(root.as_str(), "<project>").replace('\\', "/"),
            None => text,
        }
    }

    /// The app with this project open.
    fn open(&self) -> Result<(tauri::App<MockRuntime>, Window, Value), Error> {
        let (app, window) = app()?;
        let view = ok(&window, "project_open", self.args())?;
        Ok((app, window, view))
    }

    fn args(&self) -> Value {
        json!({ "path": self.path() })
    }
}

/// The answer of a call that must succeed.
fn ok(window: &Window, cmd: &str, args: Value) -> Result<Value, Error> {
    call(window, cmd, args).map_err(|error| format!("{cmd} failed: {error}").into())
}

/// A recording: `cmd` with `args` and its `answer`, how a derived variant was made, and the
/// fixture of a call on another one than [`MAIN`].
fn entry(
    cmd: &str,
    args: Value,
    answer: Result<Value, Value>,
    derived: Option<&str>,
    fixture: Option<&str>,
) -> serde_json::Map<String, Value> {
    let (result, value) = match answer {
        Ok(value) => ("ok", value),
        Err(value) => ("error", value),
    };
    let mut entry = serde_json::Map::new();
    entry.insert("cmd".to_owned(), Value::String(cmd.to_owned()));
    entry.insert("args".to_owned(), args);
    entry.insert("result".to_owned(), Value::String(result.to_owned()));
    entry.insert("value".to_owned(), value);
    if let Some(how) = derived {
        entry.insert("derived".to_owned(), Value::String(how.to_owned()));
    }
    if let Some(name) = fixture {
        entry.insert("fixture".to_owned(), Value::String(name.to_owned()));
    }
    entry
}

/// The recordings of one run, by file name.
#[derive(Default)]
struct Recordings(BTreeMap<String, String>);

impl Recordings {
    /// Records `cmd` with `args` and its `answer` as `<name>.json`; `derived` says how a
    /// derived variant was made.
    fn add(
        &mut self,
        name: &str,
        cmd: &str,
        args: Value,
        answer: Result<Value, Value>,
        derived: Option<&str>,
    ) -> Result<(), Error> {
        self.insert(name, &entry(cmd, args, answer, derived, None))
    }

    /// Keeps `entry` as `<name>.json`.
    fn insert(&mut self, name: &str, entry: &serde_json::Map<String, Value>) -> Result<(), Error> {
        let text = serde_json::to_string_pretty(entry)? + "\n";
        if self.0.insert(format!("{name}.json"), text).is_some() {
            return Err(format!("two recordings are named {name}").into());
        }
        Ok(())
    }

    /// Calls `cmd` on `project`'s app and records it, naming the fixture when it is not [`MAIN`].
    fn call(
        &mut self,
        project: &Project,
        window: &Window,
        name: &str,
        cmd: &str,
        args: Value,
    ) -> Result<(), Error> {
        let answer = ok(window, cmd, args.clone())?;
        let entry = entry(
            cmd,
            project.portable(args),
            Ok(project.portable(answer)),
            None,
            project.other_fixture(),
        );
        self.insert(name, &entry)
    }

    /// Compares every recording with its checked-in file, and fails on a file no call writes;
    /// when updating, writes them and removes the others.
    fn check(&self) -> Result<(), Error> {
        let folder = generated::repo().join(FOLDER);
        let mut problems = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries {
                let name = entry?.file_name().to_string_lossy().into_owned();
                if !self.0.contains_key(&name) {
                    if generated::updating() {
                        std::fs::remove_file(folder.join(&name))?;
                    } else {
                        problems.push(format!(
                            "{FOLDER}/{name} is written by no call: remove it or run \
                             `UPDATE_BINDINGS=1 cargo test -p theos-editor recordings`"
                        ));
                    }
                }
            }
        }
        for (name, text) in &self.0 {
            if let Err(problem) = generated::check(&format!("{FOLDER}/{name}"), text, "recordings")
            {
                problems.push(problem);
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n").into())
        }
    }
}

/// `app_info` once per OS. The OS, the version and the `engine` flag are set, so the files are
/// the same on every machine, release and feature set.
fn app_info(recordings: &mut Recordings) -> Result<(), Error> {
    let (_app, window) = default_app()?;
    let info = ok(&window, "app_info", json!({}))?;
    for os in OSES {
        let mut value = info.clone();
        value["os"] = json!(os);
        value["version"] = json!("<version>");
        value["engine"] = json!(false);
        let how = format!(
            "app_info with os `{os}`, the version as `<version>` and engine off, as the app \
             answers on that OS without the engine feature"
        );
        recordings.add(
            &format!("app_info-{os}"),
            "app_info",
            json!({}),
            Ok(value),
            Some(&how),
        )?;
    }
    Ok(())
}

/// The project, its tree tabs, every record and the page tests' searches.
fn codex(recordings: &mut Recordings) -> Result<(), Error> {
    let project = Project::copy(MAIN)?;
    let ids: Vec<String> = {
        let workspace = Workspace::open(project.path())?;
        let records = workspace.index().records()?;
        records.into_iter().map(|record| record.id).collect()
    };
    let (_app, window, view) = project.open()?;
    let args = project.portable(project.args());
    let view = project.portable(view);
    recordings.add(
        "project_open",
        "project_open",
        args.clone(),
        Ok(view.clone()),
        None,
    )?;
    let mut held = view;
    held["in_memory"] = json!(true);
    recordings.add(
        "project_open-in-memory",
        "project_open",
        args,
        Ok(held),
        Some("project_open with in_memory set, as when another program holds the cache"),
    )?;
    recordings.call(&project, &window, "codex_tabs", "codex_tabs", json!({}))?;
    for tab in TABS {
        let name = format!("codex_tree-{tab}");
        recordings.call(
            &project,
            &window,
            &name,
            "codex_tree",
            json!({ "tab": tab }),
        )?;
    }
    for id in ids {
        let name = format!("record_view-{id}");
        recordings.call(&project, &window, &name, "record_view", json!({ "id": id }))?;
    }
    for (slug, query) in SEARCHES {
        let name = format!("codex_search-{slug}");
        let args = json!({ "query": query, "limit": LIMIT });
        recordings.call(&project, &window, &name, "codex_search", args)?;
    }
    Ok(())
}

/// The edge states: the last project and none, no command-line argument, a folder that is not a
/// project, a focus sync after a change and after a removal, a Reload after a change, an empty tab.
fn edges(recordings: &mut Recordings) -> Result<(), Error> {
    let (_app, window) = app()?;
    let none = ok(&window, "project_last", json!({}))?;
    recordings.add(
        "project_last-none",
        "project_last",
        json!({}),
        Ok(none),
        None,
    )?;

    let data = tempfile::tempdir()?;
    let project = Project::copy(MAIN)?;
    let (_app, window) = app_with(AppState::new(Some(data.path().to_path_buf())))?;
    ok(&window, "project_open", project.args())?;
    recordings.call(&project, &window, "project_last", "project_last", json!({}))?;

    recordings.call(
        &project,
        &window,
        "project_argument-none",
        "project_argument",
        json!({}),
    )?;
    let world = json!({ "path": project.path().join("world") });
    let refused = call(&window, "project_open", world.clone());
    let refused = refused.err().ok_or("a folder without theos.toml opened")?;
    recordings.add(
        "project_open-not-a-project",
        "project_open",
        project.portable(world),
        Err(project.portable(refused)),
        None,
    )?;

    let project = Project::copy(MAIN)?;
    let (_app, window, _) = project.open()?;
    let tower = project.path().join("world/tower.md");
    let text = std::fs::read_to_string(&tower)? + "\nA synthetic line added by the recorder.\n";
    std::fs::write(&tower, text)?;
    recordings.call(
        &project,
        &window,
        "project_sync-changed",
        "project_sync",
        json!({ "reason": "focus" }),
    )?;

    let project = Project::copy(MAIN)?;
    let (_app, window, _) = project.open()?;
    let tower = project.path().join("world/tower.md");
    let text = std::fs::read_to_string(&tower)?
        + "
A synthetic line added by the recorder.
";
    std::fs::write(&tower, text)?;
    recordings.call(
        &project,
        &window,
        "project_sync-reload",
        "project_sync",
        json!({ "reason": "reload" }),
    )?;

    let project = Project::copy(MAIN)?;
    let (_app, window, _) = project.open()?;
    std::fs::remove_file(project.path().join("lore/clues/C-032.md"))?;
    recordings.call(
        &project,
        &window,
        "project_sync-removed",
        "project_sync",
        json!({ "reason": "focus" }),
    )?;

    let project = Project::copy("empty")?;
    let (_app, window, _) = project.open()?;
    let args = json!({ "tab": "who" });
    recordings.call(
        &project,
        &window,
        "codex_tree-who-empty",
        "codex_tree",
        args,
    )?;
    Ok(())
}

/// The Where tab and two pages of a project whose two files have one id: the id's, and the
/// tower's, which both files link to.
fn duplicate_id(recordings: &mut Recordings) -> Result<(), Error> {
    let project = Project::copy("duplicate-id")?;
    let (_app, window, _) = project.open()?;
    recordings.call(
        &project,
        &window,
        "codex_tree-where-duplicate-id",
        "codex_tree",
        json!({ "tab": "where" }),
    )?;
    recordings.call(
        &project,
        &window,
        "record_view-synthetic-twin",
        "record_view",
        json!({ "id": "synthetic-twin" }),
    )?;
    recordings.call(
        &project,
        &window,
        "record_view-tower-duplicate-id",
        "record_view",
        json!({ "id": "tower" }),
    )
}

#[test]
fn ipc_recordings_are_current() -> TestResult {
    let mut recordings = Recordings::default();
    app_info(&mut recordings)?;
    codex(&mut recordings)?;
    edges(&mut recordings)?;
    duplicate_id(&mut recordings)?;
    recordings.check()
}
