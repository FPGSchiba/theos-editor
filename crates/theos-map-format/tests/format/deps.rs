//! The format crate stands alone: it must not depend on another crate of this workspace.

use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// AC: CE-76.4
#[test]
fn the_format_crate_depends_on_no_workspace_member() -> TestResult {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .no_deps()
        .exec()?;
    let members: BTreeSet<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    let this = metadata
        .workspace_packages()
        .into_iter()
        .find(|package| package.name.as_str() == "theos-map-format")
        .ok_or("theos-map-format is not in the workspace metadata")?;
    // Every kind of dependency counts: normal, build and dev.
    let offending: Vec<&str> = this
        .dependencies
        .iter()
        .map(|dependency| dependency.name.as_str())
        .filter(|name| members.contains(name))
        .collect();
    assert!(
        offending.is_empty(),
        "theos-map-format depends on workspace members: {offending:?}"
    );
    assert!(
        !this.dependencies.is_empty(),
        "the dependency list is empty"
    );
    Ok(())
}
