//! The paths of git's index: git's spelling, literal paths, a conflicted path once, a cancel.

use theos_vcs::{Cancel, Folding, GitError, index};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn the_index_lists_tracked_paths_in_gits_spelling() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("Lore/A [x].md"), "a\n")?;
    Sandbox::write(&repo.join("Lore/b c.md"), "b\n")?;
    sandbox.commit(&repo, "first")?;
    Sandbox::write(&repo.join("Lore/staged.md"), "s\n")?;
    sandbox.run(&repo, &["add", "Lore/staged.md"])?;
    Sandbox::write(&repo.join("Lore/untracked.md"), "u\n")?;

    let paths = index::paths(&sandbox.git()?, &repo, &Cancel::new())?;
    assert_eq!(
        paths,
        [
            b"Lore/A [x].md".to_vec(),
            b"Lore/b c.md".to_vec(),
            b"Lore/staged.md".to_vec()
        ]
    );
    // A folder spelled otherwise finds the entries under a case-folding repository, in git's
    // spelling.
    let folding = Folding {
        ignore_case: true,
        precompose: false,
    };
    let below: Vec<&[u8]> = paths
        .iter()
        .filter_map(|path| folding.below("lore", path))
        .collect();
    assert_eq!(below, [b"A [x].md".as_slice(), b"b c.md", b"staged.md"]);
    Ok(())
}

#[test]
fn a_conflicted_path_is_listed_once() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "base\n")?;
    sandbox.commit(&repo, "base")?;
    sandbox.run(&repo, &["switch", "-q", "-c", "side"])?;
    Sandbox::write(&repo.join("a.md"), "side\n")?;
    sandbox.commit(&repo, "side")?;
    sandbox.run(&repo, &["switch", "-q", "main"])?;
    Sandbox::write(&repo.join("a.md"), "main\n")?;
    sandbox.commit(&repo, "main")?;
    let merge = [
        "-c",
        "user.name=Synthetic",
        "-c",
        "user.email=synthetic@example.invalid",
        "merge",
        "-q",
        "side",
    ];
    assert!(sandbox.run(&repo, &merge).is_err());
    let stages = sandbox.run(&repo, &["ls-files", "-s"])?;
    assert_eq!(stages.lines().count(), 3, "{stages}");

    let paths = index::paths(&sandbox.git()?, &repo, &Cancel::new())?;
    assert_eq!(paths, [b"a.md".to_vec()]);
    Ok(())
}

#[test]
fn a_set_cancel_lists_nothing() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let cancel = Cancel::new();
    cancel.cancel();
    let result = index::paths(&sandbox.git()?, &repo, &cancel);
    assert!(
        matches!(result, Err(GitError::Cancelled { .. })),
        "{result:?}"
    );
    Ok(())
}
