//! The rule that the code of a crate stays in its `src/`: `coverage` counts the lines of
//! `crates/<name>/src/**` and `xtask/src/**` and nothing else (spec §5.4), so code that a crate
//! builds from anywhere else would be outside the floor. A `#[path]` or an `include!` in a file
//! under such a `src/` must lead into that same `src/`, and so must `[lib].path` and
//! `[[bin]].path` in the manifest of such a crate. `include_str!` and `include_bytes!` read data
//! and are fine, as are `[[test]]`, `[[bench]]`, `[[example]]` and `build`, which are not
//! measured code.

use super::at_file;
use super::fixture_routes::{folder_of, normalise, string_value};
use crate::verdict::Finding;

/// The message of a `#[path]` that leaves `src/`.
pub(super) const PATH_OUTSIDE_SRC: &str = "`#[path]` leads out of the `src/` folder of its crate: \
                                           `coverage` counts only the code in `src/`, so code \
                                           elsewhere would be outside its floor; move the file \
                                           into `src/`";
/// The message of an `include!` that leaves `src/`.
pub(super) const INCLUDE_OUTSIDE_SRC: &str = "`include!` leads out of the `src/` folder of its \
                                              crate: `coverage` counts only the code in `src/`, \
                                              so code elsewhere would be outside its floor; move \
                                              the file into `src/`";
const MANIFEST_OUTSIDE_SRC: &str = "has a `[lib]` or `[[bin]]` `path` that does not lead into \
                                    the `src/` folder of its crate: `coverage` counts only the \
                                    code in `src/`, so code elsewhere would be outside its \
                                    floor; move the file into `src/` or leave the path out";

/// The folder `src/` that `file` is in, when `file` is in the `src/` of a measured crate
/// (`crates/<name>/src/` or `xtask/src/`): in lower case, with a `/` at the end.
fn src_of_file(file: &str) -> Option<String> {
    let lower = file.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('/').collect();
    match parts.as_slice() {
        ["crates", name, "src", _, ..] => Some(format!("crates/{name}/src/")),
        ["xtask", "src", _, ..] => Some("xtask/src/".to_owned()),
        _ => None,
    }
}

/// Whether the string literal `literal`, the path of a `#[path]` or an `include!` in the file
/// `file`, leads out of the `src/` folder that `file` is in. A file that is not in the `src/` of a
/// measured crate has no such bound, and a path that cannot be followed is judged elsewhere.
pub(super) fn leaves_src(file: &str, literal: &str) -> bool {
    let Some(src) = src_of_file(file) else {
        return false;
    };
    let resolved = string_value(literal)
        .and_then(|path| normalise(&path))
        .and_then(|own| folder_of(file, &own));
    resolved.is_some_and(|folder| !folder.starts_with(&src))
}

/// The findings for the manifest `manifest` of the file `file`: one when it is the manifest of a
/// measured crate (`crates/<name>/Cargo.toml` or `xtask/Cargo.toml`) and a `[lib]` or `[[bin]]`
/// `path` does not lead into the `src/` of the crate, however it is written. A path that cannot
/// be followed (absolute, out of the repository, not a string) leaves `src/` too.
pub(super) fn manifest_findings(file: &str, manifest: &toml::Table) -> Vec<Finding> {
    let lower = file.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('/').collect();
    let src = match parts.as_slice() {
        ["crates", name, "cargo.toml"] => format!("crates/{name}/src/"),
        ["xtask", "cargo.toml"] => "xtask/src/".to_owned(),
        _ => return Vec::new(),
    };
    let lib = manifest.get("lib").and_then(|lib| lib.get("path"));
    let bins = manifest
        .get("bin")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|bin| bin.get("path"));
    let leaves = lib.into_iter().chain(bins).any(|path| {
        let resolved = path
            .as_str()
            .and_then(normalise)
            .and_then(|own| folder_of(file, &own));
        !resolved.is_some_and(|folder| folder.starts_with(&src))
    });
    if leaves {
        vec![at_file(file, MANIFEST_OUTSIDE_SRC)]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::*;

    #[test]
    fn src_of_file_only_for_measured_crates() {
        assert_eq!(
            src_of_file("crates/a/src/lib.rs").as_deref(),
            Some("crates/a/src/")
        );
        assert_eq!(
            src_of_file("Crates/A/Src/x/y.rs").as_deref(),
            Some("crates/a/src/")
        );
        assert_eq!(
            src_of_file("xtask/src/main.rs").as_deref(),
            Some("xtask/src/")
        );
        for file in [
            "crates/a/tests/x.rs",
            "crates/a/build.rs",
            "crates/a/src",
            "src/lib.rs",
            "xtask/tests/x.rs",
            "other/crates/a/src/lib.rs",
        ] {
            assert_eq!(src_of_file(file), None, "{file}");
        }
    }

    #[test]
    fn leaves_src_by_where_the_path_leads() {
        let file = "crates/a/src/deep/mod.rs";
        for literal in [
            "\"x.rs\"",
            "\"../x.rs\"",
            "\"../../src/x.rs\"",
            "\"..\\\\x.rs\"",
            "r\"..\\x.rs\"",
            "\"../SRC/x.rs\"",
        ] {
            assert!(!leaves_src(file, literal), "{literal}");
        }
        for literal in [
            "\"../../x.rs\"",
            "\"../../lib/x.rs\"",
            "\"../../tests/x.rs\"",
            "\"../../../b/src/x.rs\"",
        ] {
            assert!(leaves_src(file, literal), "{literal}");
        }
        // No bound outside the `src/` of a measured crate.
        assert!(!leaves_src("crates/a/tests/t.rs", "\"../lib/x.rs\""));
    }

    #[test]
    fn manifest_paths_must_lead_into_src() -> crate::Result<()> {
        let file = "crates/a/Cargo.toml";
        for text in [
            "[lib]\npath = \"lib/lib.rs\"\n",
            "[lib]\npath = \"src/../lib.rs\"\n",
            "[[bin]]\nname = \"x\"\npath = \"main.rs\"\n",
            "[lib]\npath = \"/abs/lib.rs\"\n",
            "[lib]\npath = 3\n",
        ] {
            let found = manifest_findings(file, &parse(file, text)?);
            assert_eq!(found.len(), 1, "{text}");
            assert_eq!(found[0].file.as_deref(), Some(file));
        }
        for text in [
            "",
            "[lib]\npath = \"src/lib.rs\"\n",
            "[lib]\npath = \"Src\\\\lib.rs\"\n",
            "[[bin]]\nname = \"x\"\npath = \"src/bin/x.rs\"\n",
            "[[test]]\nname = \"x\"\npath = \"tests/x.rs\"\n",
            "[package]\nbuild = \"build.rs\"\n",
        ] {
            let found = manifest_findings(file, &parse(file, text)?);
            assert!(found.is_empty(), "{text}: {found:?}");
        }
        // Manifests of other things are not judged.
        let outside = parse("Cargo.toml", "[lib]\npath = \"lib.rs\"\n")?;
        assert!(manifest_findings("Cargo.toml", &outside).is_empty());
        assert_eq!(manifest_findings("xtask/Cargo.toml", &outside).len(), 1);
        Ok(())
    }
}
