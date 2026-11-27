//! Final review round 5, finding 1: an unknown key reads one way in every stage and process, at
//! the top of its table and in each table nested in it: "unknown parameter", named by its path
//! (spec §5.5, plan ruling L3).
//!
//! The tests are built from the registry and `PROCESSES`, and the tables from what each stage
//! parses (its defaults and every recipe's values, as
//! [`StageParams::to_toml`](crate::stage::StageParams::to_toml) gives them), so a new stage,
//! process or nested table is covered without a new line.

use std::path::Path;

use crate::recipe;
use crate::stage::{ParamError, Registry, Stage};
use crate::stages::evolution::{Evolution, PROCESSES};

/// The key no stage knows.
const UNKNOWN: &str = "zz_unknown";

/// The tables, by stage and parameter, whose unknown keys serde reports itself: an enum tagged
/// by a field hides its keys from `serde_ignored`, so it carries `deny_unknown_fields` (Task 8
/// ruling). Serde names the key in its message, at the path of the table.
const TAGGED: &[(&str, &str)] = &[("tectonics", "source")];

/// One step of a path into a TOML value.
#[derive(Debug, Clone)]
enum Step {
    /// A key of a table.
    Key(String),
    /// An index of an array.
    Index(usize),
}

/// `at` in the style of [`ParamError`]'s parameter: `counts`, `source[0]`, `""` for the top.
fn text(at: &[Step]) -> String {
    at.iter()
        .enumerate()
        .map(|(i, step)| match step {
            Step::Key(key) if i == 0 => key.clone(),
            Step::Key(key) => format!(".{key}"),
            Step::Index(n) => format!("[{n}]"),
        })
        .collect()
}

/// The paths of every table in `value`, `value` itself included, depth first.
fn tables(value: &toml::Value, at: &mut Vec<Step>, out: &mut Vec<Vec<Step>>) {
    match value {
        toml::Value::Table(table) => {
            out.push(at.clone());
            for (key, item) in table {
                at.push(Step::Key(key.clone()));
                tables(item, at, out);
                at.pop();
            }
        }
        toml::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                at.push(Step::Index(i));
                tables(item, at, out);
                at.pop();
            }
        }
        _ => {}
    }
}

/// `value` with [`UNKNOWN`] `= 1` added to the table at `at`.
fn with_unknown(value: &toml::Value, at: &[Step]) -> toml::Table {
    let mut value = value.clone();
    let mut here = &mut value;
    for step in at {
        here = match (step, here) {
            (Step::Key(key), toml::Value::Table(table)) => table.get_mut(key).unwrap(),
            (Step::Index(i), toml::Value::Array(items)) => &mut items[*i],
            (step, other) => panic!("{step:?} does not lead into {other}"),
        };
    }
    here.as_table_mut()
        .unwrap()
        .insert(UNKNOWN.to_owned(), toml::Value::Integer(1));
    value.as_table().unwrap().clone()
}

/// The error a stage should give.
#[derive(Debug, Clone)]
struct Expected {
    /// The parameter it names.
    parameter: String,
    /// Its message, or the start of it when not `exact`.
    message: String,
    /// Whether the message is the whole message.
    exact: bool,
}

impl Expected {
    /// The error an unknown key in the table at `at` of stage `id` gives: "unknown parameter" at
    /// `<at>.zz_unknown`, or for a table of [`TAGGED`] serde's own message naming the key, at
    /// `at`.
    fn of(id: &str, at: &[Step]) -> Self {
        let tagged = matches!(at, [Step::Key(key), Step::Index(_)]
            if TAGGED.contains(&(id, key.as_str())));
        if tagged {
            return Self {
                parameter: text(at),
                message: format!("unknown field `{UNKNOWN}`"),
                exact: false,
            };
        }
        let parameter = if at.is_empty() {
            UNKNOWN.to_owned()
        } else {
            format!("{}.{UNKNOWN}", text(at))
        };
        Self {
            parameter,
            message: "unknown parameter".to_owned(),
            exact: true,
        }
    }

    /// Whether `err` is this error of `stage`.
    fn is(&self, err: &ParamError, stage: &str) -> bool {
        let message = if self.exact {
            err.message == self.message
        } else {
            err.message.starts_with(&self.message)
        };
        err.stage == stage && err.parameter.as_deref() == Some(&*self.parameter) && message
    }
}

/// Every stage table of the built-in and the fixture recipes, as `(stage id, table)`, and every
/// `[[stage.process]]` table inside them, as `(process id, table)`; without `id`, and `every`
/// for a process.
fn recipe_tables() -> Vec<(String, toml::Table)> {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes");
    let catalog = recipe::catalog(&[fixtures]).unwrap();
    let mut out = Vec::new();
    let mut add = |table: &toml::Table| {
        let id = table.get("id").and_then(toml::Value::as_str).unwrap();
        let mut table = table.clone();
        table.remove("id");
        table.remove("every");
        out.push((id.to_owned(), table));
    };
    for loaded in catalog.recipes.values() {
        for stage in &loaded.recipe.stages {
            add(stage);
            let processes = stage.get("process").and_then(toml::Value::as_array);
            for process in processes.into_iter().flatten() {
                add(process.as_table().unwrap());
            }
        }
    }
    out
}

/// The parameters `stage` parses, as TOML: its defaults when the empty table parses, and every
/// recipe table of its id.
fn samples(stage: &dyn Stage, recipes: &[(String, toml::Table)]) -> Vec<toml::Value> {
    let tables = std::iter::once(toml::Table::new()).chain(
        recipes
            .iter()
            .filter(|(id, _)| id == stage.id())
            .map(|(_, table)| table.clone()),
    );
    let samples: Vec<toml::Value> = tables
        .filter_map(|table| stage.params(&table).ok())
        .map(|params| params.to_toml().unwrap())
        .collect();
    assert!(
        !samples.is_empty(),
        "stage {}: no recipe names it and the empty table does not parse",
        stage.id()
    );
    samples
}

/// How a stage's table reaches the stage that parses it.
struct Owner<'a> {
    /// The stage that parses the table.
    stage: &'a dyn Stage,
    /// The table that stage parses, from the table of the stage checked.
    wrap: &'a dyn Fn(toml::Table) -> toml::Table,
    /// The error that stage gives, from the error of the stage checked.
    expect: &'a dyn Fn(Expected) -> Expected,
}

/// Checks every table of every sample of `stage`: as given it parses, and with an unknown key
/// it gives the [`Expected`] error. Counts the tables in `checked` and adds every miss to
/// `misses`.
fn check_every_table(
    stage: &dyn Stage,
    owner: &Owner<'_>,
    recipes: &[(String, toml::Table)],
    misses: &mut Vec<String>,
) -> usize {
    let mut checked = 0;
    for sample in samples(stage, recipes) {
        let given = (owner.wrap)(sample.as_table().unwrap().clone());
        let parsed = owner.stage.params(&given);
        assert!(
            parsed.is_ok(),
            "{}: {given}: {:?}",
            stage.id(),
            parsed.err()
        );
        let mut paths = Vec::new();
        tables(&sample, &mut Vec::new(), &mut paths);
        for at in paths {
            let table = (owner.wrap)(with_unknown(&sample, &at));
            let expected = (owner.expect)(Expected::of(stage.id(), &at));
            match owner.stage.params(&table) {
                Err(err) if expected.is(&err, owner.stage.id()) => {}
                other => misses.push(format!(
                    "{} at `{}`: expected {expected:?}, got {:?}",
                    stage.id(),
                    text(&at),
                    other.map(|_| "accepted")
                )),
            }
            checked += 1;
        }
    }
    checked
}

/// Fails listing every miss.
fn assert_no_misses(misses: &[String]) {
    let list = misses.join("\n");
    assert!(misses.is_empty(), "{} misses:\n{list}", misses.len());
}

#[test]
fn an_unknown_key_is_an_unknown_parameter_in_every_table_of_every_stage() {
    let registry = Registry::builtin();
    let recipes = recipe_tables();
    let mut misses = Vec::new();
    for id in registry.ids() {
        let stage = registry.get(id).unwrap();
        let owner = Owner {
            stage,
            wrap: &|table| table,
            expect: &|expected| expected,
        };
        let checked = check_every_table(stage, &owner, &recipes, &mut misses);
        assert!(checked > 0, "stage {id}: no table checked");
    }
    assert_no_misses(&misses);
}

#[test]
fn an_unknown_key_is_an_unknown_parameter_in_every_table_of_every_process() {
    let recipes = recipe_tables();
    let mut misses = Vec::new();
    for make in PROCESSES {
        let process = make();
        let owner = Owner {
            stage: process.as_ref(),
            wrap: &|table| table,
            expect: &|expected| expected,
        };
        let checked = check_every_table(process.as_ref(), &owner, &recipes, &mut misses);
        assert!(checked > 0, "process {}: no table checked", process.id());
    }
    assert_no_misses(&misses);
}

// The same through `landscape_evolution`, which names the process table and the process.
#[test]
fn an_unknown_key_of_a_process_is_named_inside_its_process_table() {
    let recipes = recipe_tables();
    let mut misses = Vec::new();
    let evolution = Evolution::new();
    for make in PROCESSES {
        let process = make();
        let id = process.id();
        let wrap = |mut table: toml::Table| {
            table.insert("id".to_owned(), toml::Value::String(id.to_owned()));
            let mut outer = toml::Table::new();
            outer.insert(
                "process".to_owned(),
                toml::Value::Array(vec![toml::Value::Table(table)]),
            );
            outer
        };
        let expect = |inner: Expected| Expected {
            parameter: format!("process[0].{}", inner.parameter),
            message: format!("process `{id}`: {}", inner.message),
            exact: inner.exact,
        };
        let owner = Owner {
            stage: &evolution,
            wrap: &wrap,
            expect: &expect,
        };
        let checked = check_every_table(process.as_ref(), &owner, &recipes, &mut misses);
        assert!(checked > 0, "process {id}: no table checked");
    }
    assert_no_misses(&misses);
}
