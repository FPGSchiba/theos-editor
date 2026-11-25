//! The resolution pairs of physics test 13 (`resolution.rs`) are copies of one recipe at two
//! grid sizes. These tests keep the copies equal, so that a later retune of `hearth-valley`
//! cannot leave test 13 testing an old copy.

use std::path::Path;

use crate::support::{TestResult, recipes};

/// The recipe at `path` as TOML, without what a copy may change: `id`, `title` and `grid`
/// (`size_km` stays, so the cell size is the only difference).
fn without_grid_and_names(path: &Path) -> Result<toml::Table, Box<dyn std::error::Error>> {
    let mut table: toml::Table = std::fs::read_to_string(path)?.parse()?;
    for key in ["id", "title", "grid"] {
        table
            .remove(key)
            .ok_or_else(|| format!("{} has no `{key}`", path.display()))?;
    }
    Ok(table)
}

/// The fixture recipe `name`.
fn fixture(name: &str) -> std::path::PathBuf {
    recipes()[0].join(format!("{name}.toml"))
}

#[test]
fn the_hearth_pair_is_hearth_valley_on_two_grids() -> TestResult {
    let built_in = Path::new(env!("CARGO_MANIFEST_DIR")).join("recipes/hearth-valley.toml");
    let original = without_grid_and_names(&built_in)?;
    for name in ["hearth-192", "hearth-384"] {
        assert_eq!(
            without_grid_and_names(&fixture(name))?,
            original,
            "{name} differs from recipes/hearth-valley.toml beyond id, title and grid"
        );
    }
    Ok(())
}

#[test]
fn the_basin_pair_differs_only_in_grid() -> TestResult {
    assert_eq!(
        without_grid_and_names(&fixture("basin-192"))?,
        without_grid_and_names(&fixture("basin-384"))?,
        "basin-192 and basin-384 differ beyond id, title and grid"
    );
    Ok(())
}

#[test]
fn the_pairs_have_the_grids_test_13_names() -> TestResult {
    for (name, side) in [
        ("hearth-192", 192),
        ("hearth-384", 384),
        ("basin-192", 192),
        ("basin-384", 384),
    ] {
        let table: toml::Table = std::fs::read_to_string(fixture(name))?.parse()?;
        assert_eq!(
            table.get("grid"),
            Some(&toml::Value::Array(vec![side.into(), side.into()])),
            "{name}"
        );
    }
    Ok(())
}
