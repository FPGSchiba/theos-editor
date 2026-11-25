use std::fs;
use std::path::Path;

use super::*;
use crate::recipe::RecipeError;
use crate::recipe::test_stages::text;

fn recipe_text(id: &str, title: &str) -> String {
    text("", &["a"]).replace(
        "id = \"t\"\ntitle = \"T\"",
        &format!("id = \"{id}\"\ntitle = \"{title}\""),
    )
}

fn write(dir: &Path, name: &str, body: &str) {
    fs::write(dir.join(name), body).unwrap();
}

fn title<'a>(catalog: &'a Catalog, id: &str) -> &'a str {
    &catalog.recipes[id].recipe.title
}

#[test]
fn folder_recipes_win_over_builtins_and_later_folders_win() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write(first.path(), "x.toml", &recipe_text("x", "first"));
    write(first.path(), "y.toml", &recipe_text("y", "first"));
    write(second.path(), "x.toml", &recipe_text("x", "second"));
    let builtin = recipe_text("x", "builtin");
    let builtin_z = recipe_text("z", "builtin");
    let builtins = [("x.toml", builtin.as_str()), ("z.toml", builtin_z.as_str())];

    let none = catalog_with(&builtins, &[]).unwrap();
    assert_eq!(title(&none, "x"), "builtin");
    assert_eq!(none.recipes["x"].origin, RecipeOrigin::Builtin);

    let one = catalog_with(&builtins, &[first.path().to_owned()]).unwrap();
    assert_eq!(title(&one, "x"), "first");
    assert_eq!(
        one.recipes["x"].origin,
        RecipeOrigin::File(first.path().join("x.toml"))
    );

    let both = catalog_with(
        &builtins,
        &[first.path().to_owned(), second.path().to_owned()],
    )
    .unwrap();
    assert_eq!(title(&both, "x"), "second");
    assert_eq!(title(&both, "y"), "first");
    assert_eq!(title(&both, "z"), "builtin");
    assert_eq!(both.problems.len(), 0);
}

#[test]
fn files_of_a_folder_load_in_sorted_order() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "b.toml", &recipe_text("same", "from b"));
    write(dir.path(), "a.toml", &recipe_text("same", "from a"));
    let catalog = catalog_with(&[], &[dir.path().to_owned()]).unwrap();
    assert_eq!(title(&catalog, "same"), "from b");
}

#[test]
fn a_broken_file_is_a_problem_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bad.toml", "id = ");
    write(dir.path(), "worse.toml", "bogus = 1\n");
    write(dir.path(), "good.toml", &recipe_text("good", "ok"));
    let catalog = catalog_with(&[], &[dir.path().to_owned()]).unwrap();
    assert_eq!(catalog.recipes.keys().collect::<Vec<_>>(), ["good"]);
    assert_eq!(catalog.problems.len(), 2);
    assert_eq!(catalog.problems[0].file, dir.path().join("bad.toml"));
    assert_eq!(catalog.problems[0].id.as_deref(), Some("bad"));
    assert!(catalog.problems[1].message.contains("bogus"));
}

#[test]
fn a_non_utf8_file_is_a_problem() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bin.toml"), [0xff_u8, 0xfe]).unwrap();
    let catalog = catalog_with(&[], &[dir.path().to_owned()]).unwrap();
    assert_eq!(catalog.problems.len(), 1);
    assert!(catalog.problems[0].message.contains("UTF-8"));
}

#[test]
fn a_broken_override_hides_the_built_in() {
    // L16: the file names the id but does not parse, so the built-in of that id must not run.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "x.toml",
        "id = \"x\"\ntitle = \"broken\"\nbogus = 1\n",
    );
    let builtin = recipe_text("x", "builtin");
    let other = recipe_text("w", "builtin");
    let builtins = [("x.toml", builtin.as_str()), ("w.toml", other.as_str())];
    let catalog = catalog_with(&builtins, &[dir.path().to_owned()]).unwrap();
    assert!(!catalog.recipes.contains_key("x"));
    assert!(catalog.recipes.contains_key("w"));
    assert_eq!(catalog.problems.len(), 1);
    assert_eq!(catalog.problems[0].id.as_deref(), Some("x"));
}

#[test]
fn a_broken_built_in_is_a_problem_named_builtin() {
    let catalog = catalog_with(&[("bad.toml", "id = ")], &[]).unwrap();
    assert!(catalog.recipes.is_empty());
    assert_eq!(catalog.problems[0].file, Path::new("builtin:bad.toml"));
}

#[test]
fn a_later_good_file_replaces_an_earlier_broken_one() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write(first.path(), "x.toml", "id = \"x\"\nbogus = 1\n");
    write(second.path(), "x.toml", &recipe_text("x", "fixed"));
    let catalog = catalog_with(&[], &[first.path().to_owned(), second.path().to_owned()]).unwrap();
    assert_eq!(title(&catalog, "x"), "fixed");
    assert_eq!(catalog.problems.len(), 1);
}

#[test]
fn other_files_and_subfolders_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "notes.txt", "not a recipe");
    write(dir.path(), "README.md", "# nope");
    fs::create_dir(dir.path().join("sub")).unwrap();
    write(
        &dir.path().join("sub"),
        "inner.toml",
        &recipe_text("inner", "x"),
    );
    fs::create_dir(dir.path().join("dir.toml")).unwrap();
    let catalog = catalog_with(&[], &[dir.path().to_owned()]).unwrap();
    assert!(catalog.recipes.is_empty() && catalog.problems.is_empty());
}

// Final review round 5, finding 6 (a Task 9 nit): the extension matches in any case, as file
// names on Windows do, so `Ridge.TOML` is a recipe and not skipped silently.
#[test]
fn the_extension_matches_in_any_case() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Ridge.TOML", &recipe_text("ridge", "upper"));
    write(dir.path(), "dale.Toml", &recipe_text("dale", "mixed"));
    let catalog = catalog_with(&[], &[dir.path().to_owned()]).unwrap();
    assert_eq!(
        catalog.recipes.keys().collect::<Vec<_>>(),
        ["dale", "ridge"]
    );
    assert_eq!(title(&catalog, "ridge"), "upper");
    assert_eq!(title(&catalog, "dale"), "mixed");
    assert!(catalog.problems.is_empty(), "{:?}", catalog.problems);
}

#[test]
fn a_missing_folder_is_an_io_error_naming_it() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    let err = catalog(std::slice::from_ref(&missing)).unwrap_err();
    assert!(
        matches!(&err, RecipeError::Io { path, .. } if *path == missing),
        "{err}"
    );
    assert!(err.to_string().contains("nope"));
}

#[test]
fn the_public_catalog_holds_the_built_ins_and_folders() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "x.toml", &recipe_text("x", "file"));
    let catalog = catalog(&[dir.path().to_owned()]).unwrap();
    assert_eq!(title(&catalog, "x"), "file");
}
