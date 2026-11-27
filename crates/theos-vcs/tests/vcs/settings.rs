//! A user's git settings do not change what the reads answer.

use std::io::Write;
use std::path::Path;

use theos_vcs::Cancel;
use theos_vcs::history::{self, Page};
use theos_vcs::read::{self, Version};
use theos_vcs::status;

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Every setting of P46 §5.10 that could change an answer, plus the ones a later read could meet.
const HOSTILE: &str = "[color]\n\tui = always\n\tstatus = always\n\
    [log]\n\tshowRoot = false\n\tshowSignature = true\n\tfollow = false\n\tdecorate = full\n\
    [core]\n\tquotePath = true\n\tprecomposeUnicode = false\n\
    [status]\n\trenames = false\n\tshowUntrackedFiles = no\n\tbranch = false\n\
    [diff]\n\trenames = false\n\texternal = false-diff-tool\n\
    [format]\n\tpretty = oneline\n\
    [i18n]\n\tlogOutputEncoding = ISO-8859-1\n";

/// What the three reads answer, as text.
fn answers(sandbox: &Sandbox, repo: &Path) -> Result<String> {
    let git = sandbox.git()?;
    let cancel = Cancel::new();
    let status = status::status(&git, repo, &cancel)?;
    let all = Page { skip: 0, max: 100 };
    let history = history::history(&git, repo, b"a.md", all, &cancel)?;
    // `c.md` was renamed from `b.md` in a commit: `log.follow` and `diff.renames` must not stop
    // its history at the rename.
    let renamed = history::history(&git, repo, b"c.md", all, &cancel)?;
    let head = read::read(&git, repo, Version::Head, b"a.md")?;
    let index = read::read(&git, repo, Version::Index, b"c.md")?;
    Ok(format!(
        "{status:?}\n{history:?}\n{renamed:?}\n{head:?}\n{index:?}"
    ))
}

#[test]
fn user_settings_do_not_change_the_reads() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let body = "A synthetic record for the settings test.\nIt has a few lines of body text,\n\
                so that git finds a renamed copy similar.\nOne more line.\n";
    Sandbox::write(&repo.join("a.md"), &format!("first\n{body}"))?;
    Sandbox::write(&repo.join("b.md"), body)?;
    sandbox.commit(&repo, "first")?;
    Sandbox::write(&repo.join("a.md"), &format!("second\n{body}"))?;
    sandbox.commit(&repo, "second \u{fc}ber\n\nApproved-by: Jann")?;
    sandbox.run(&repo, &["mv", "b.md", "c.md"])?;
    sandbox.commit(&repo, "rename b to c")?;
    // An unstaged rename, which `status.renames=false` must not split.
    std::fs::rename(repo.join("c.md"), repo.join("d.md"))?;
    Sandbox::write(&repo.join("a.md"), &format!("third\n{body}"))?;
    Sandbox::write(&repo.join("\u{fc}ntracked.md"), "new\n")?;

    let before = answers(&sandbox, &repo)?;
    let followed = history::history(
        &sandbox.git()?,
        &repo,
        b"c.md",
        Page { skip: 0, max: 100 },
        &Cancel::new(),
    )?;
    assert_eq!(followed.len(), 2, "{followed:?}");
    let unstaged = status::status(&sandbox.git()?, &repo, &Cancel::new())?;
    assert!(
        unstaged
            .entries
            .iter()
            .any(|entry| matches!(entry, status::Entry::Renamed { .. })),
        "{unstaged:?}"
    );
    let mut config = std::fs::OpenOptions::new()
        .append(true)
        .open(repo.join(".git").join("config"))?;
    config.write_all(HOSTILE.as_bytes())?;
    drop(config);
    let after = answers(&sandbox, &repo)?;
    assert_eq!(before, after);
    Ok(())
}
