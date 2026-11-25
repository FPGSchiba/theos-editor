//! Glob lists of the `gate/*.toml` files, read by `globset` the same way everywhere.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::{Error, Result};

/// The globs of one list. `*` stays inside a folder, `**` crosses folders. A bad pattern is named
/// by its place in the list, not quoted.
///
/// The error is an [`Error::Config`] without a file name; the caller names its file with
/// [`in_file`].
///
/// # Errors
///
/// Fails when a pattern is not a valid glob.
pub fn glob_set(list: &str, patterns: &[String], ignore_case: bool) -> Result<GlobSet> {
    let mut set = GlobSetBuilder::new();
    for (index, pattern) in patterns.iter().enumerate() {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .backslash_escape(true)
            .case_insensitive(ignore_case)
            .build()
            .map_err(|_| {
                config_error(format!(
                    "`{list}` pattern {} is not a valid glob",
                    index + 1
                ))
            })?;
        set.add(glob);
    }
    set.build()
        .map_err(|_| config_error(format!("`{list}` cannot be read as globs")))
}

/// Names `file` in a [`glob_set`] error; any other error is returned as it is.
#[must_use]
pub fn in_file(error: Error, file: &str) -> Error {
    match error {
        Error::Config { message, .. } => Error::Config {
            file: file.to_owned(),
            message,
        },
        other => other,
    }
}

fn config_error(message: impl Into<String>) -> Error {
    Error::Config {
        file: String::new(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_name_the_list_and_the_caller_names_the_file() {
        let patterns = ["ok".to_owned(), "SECRET[".to_owned()];
        let error = in_file(
            glob_set("list", &patterns, false).expect_err("bad glob"),
            "gate/x.toml",
        );
        assert_eq!(
            error.to_string(),
            "gate/x.toml: `list` pattern 2 is not a valid glob"
        );
        let parse = Error::Parse {
            what: "a".to_owned(),
            message: "b".to_owned(),
        };
        let other = in_file(parse, "gate/x.toml");
        assert!(matches!(other, Error::Parse { .. }));
    }
}
