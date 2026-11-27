//! The settings that keep git from running the repository's hooks during a read that writes an
//! index (status's `add --intent-to-add` on a temporary copy), so the read stays a pure read.
//!
//! git runs two kinds of hooks: hook files in the hooks folder, and (git 2.54 and later) hooks
//! named in the configuration (`hook.<name>.event` and `hook.<name>.command`). `core.hooksPath`
//! pointed at a folder without hooks turns off the files; `hook.<name>.enabled=false` turns off
//! each configured hook. Older gits ignore the `hook.*` keys.

use std::ffi::OsStr;
use std::path::Path;

use crate::Cancel;
use crate::git::{Git, GitError, args_text};
use crate::read_args::read_args;
use crate::staged::parse::fields;

/// The `-c` settings, without the `-c`, that turn off every hook of the repository in `top`:
/// `core.hooksPath=<empty>` (a folder without hooks) and `hook.<name>.enabled=false` for each
/// hook the configuration names. Reads the configuration once.
///
/// # Errors
///
/// The errors of [`Git::run_cancellable`], and [`GitError::Output`] when the answer cannot be
/// read or names a hook that cannot be turned off by `-c` (a name that is not UTF-8 or holds `=`).
pub(crate) fn no_hooks(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    empty: &Path,
) -> Result<Vec<String>, GitError> {
    let args = read_args(
        "config",
        &["--name-only", "-z", "--get-regexp", r"^hook\..*\.event$"],
    );
    let out = match git.run_cancellable(&args, top, None, &[], cancel) {
        Ok(out) => out,
        // `config --get-regexp` exits with 1 when no key matches.
        Err(GitError::Failed { code: Some(1), .. }) => Vec::new(),
        Err(error) => return Err(error),
    };
    let names = hook_names(&out).map_err(|message| GitError::Output {
        args: args_text(&args),
        message,
    })?;
    Ok(
        std::iter::once(format!("core.hooksPath={}", empty.display()))
            .chain(
                names
                    .iter()
                    .map(|name| format!("hook.{name}.enabled=false")),
            )
            .collect(),
    )
}

/// The `-c` arguments for `settings`: `-c` before each.
pub(crate) fn config_args(settings: &[String]) -> Vec<&OsStr> {
    settings
        .iter()
        .flat_map(|setting| [OsStr::new("-c"), OsStr::new(setting.as_str())])
        .collect()
}

/// The hook names in the NUL-separated keys `out` (`hook.<name>.event`), each once, in order. A
/// name may hold dots: it is everything between the first `hook.` and the last `.event`.
fn hook_names(out: &[u8]) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::new();
    for key in fields(out)? {
        let key = std::str::from_utf8(key).map_err(|_| "a hook's key is not UTF-8")?;
        let name = key
            .strip_prefix("hook.")
            .and_then(|rest| rest.strip_suffix(".event"))
            .ok_or_else(|| format!("`{key}` is not a hook's event"))?;
        if name.contains('=') {
            return Err(format!(
                "the hook `{name}` holds `=`, which `-c` cannot name"
            ));
        }
        if !names.iter().any(|seen| seen == name) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_answer_names_no_hook() {
        assert_eq!(hook_names(b""), Ok(Vec::new()));
    }

    #[test]
    fn every_hook_is_named_once_and_in_order() {
        let out = b"hook.probe.event\0hook.lint.event\0hook.probe.event\0";
        assert_eq!(
            hook_names(out),
            Ok(vec!["probe".to_owned(), "lint".to_owned()])
        );
    }

    #[test]
    fn a_name_with_dots_is_everything_between_hook_and_event() {
        let out = b"hook.My.Dot.event.event\0";
        assert_eq!(hook_names(out), Ok(vec!["My.Dot.event".to_owned()]));
    }

    #[test]
    fn a_name_that_is_not_utf_8_or_holds_an_equals_sign_is_an_error() {
        assert!(hook_names(b"hook.\xff.event\0").is_err());
        assert!(hook_names(b"hook.a=b.event\0").is_err());
        assert!(hook_names(b"other.key\0").is_err());
        assert!(hook_names(b"hook.probe.event").is_err());
    }

    #[test]
    fn the_settings_get_a_c_each() {
        let settings = [
            "core.hooksPath=x".to_owned(),
            "hook.a.enabled=false".to_owned(),
        ];
        let args: Vec<&str> = config_args(&settings)
            .iter()
            .filter_map(|arg| arg.to_str())
            .collect();
        assert_eq!(
            args,
            ["-c", "core.hooksPath=x", "-c", "hook.a.enabled=false"]
        );
    }
}
