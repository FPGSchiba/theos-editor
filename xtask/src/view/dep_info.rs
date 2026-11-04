//! Dep-info files: the list of input files rustc writes next to every artifact.
//!
//! Cargo runs rustc from the workspace root, so the paths in a dep-info file are relative to it
//! (or absolute, for files outside it). [`parse_dep_info`] reads the file's list, [`repo_relative`]
//! turns each path into the form the rest of the gate uses, and [`dep_info_of`] says where the
//! dep-info of an artifact is.

use std::path::{Component, Path, PathBuf};

/// The files a dep-info text lists: the lines that end with `:` (rustc writes one such target
/// per input file, with no dependencies after it; the output lines have dependencies after the
/// colon), with `\ ` read as a space, other backslashes as `/`, and comment lines (`# env-dep:`
/// and the like) skipped. Sorted and without duplicates.
#[must_use]
pub fn parse_dep_info(text: &str) -> Vec<String> {
    let mut files: Vec<String> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_suffix(':'))
        .filter(|path| !path.is_empty())
        .map(unescape)
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Reads rustc's escapes (`\ ` and `\#` are the character itself), and takes every other
/// backslash for a Windows path separator.
fn unescape(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
        } else if let Some(escaped) = chars.next_if(|next| matches!(next, ' ' | '#')) {
            out.push(escaped);
        } else {
            out.push('/');
        }
    }
    out
}

/// Where a path lexically is: its prefix (a drive, empty when there is none), whether it starts
/// at a root, and its segments with `.` and `..` resolved.
#[derive(Debug)]
struct Normal {
    prefix: String,
    rooted: bool,
    segments: Vec<String>,
}

impl Normal {
    fn of(path: &str) -> Self {
        let mut normal = Self {
            prefix: String::new(),
            rooted: false,
            segments: Vec::new(),
        };
        for component in Path::new(path).components() {
            match component {
                Component::Prefix(prefix) => {
                    prefix
                        .as_os_str()
                        .to_string_lossy()
                        .trim_start_matches(r"\\?\")
                        .clone_into(&mut normal.prefix);
                }
                Component::RootDir => normal.rooted = true,
                Component::CurDir => {}
                Component::ParentDir => {
                    normal.segments.pop();
                }
                Component::Normal(segment) => {
                    normal.segments.push(segment.to_string_lossy().into_owned());
                }
            }
        }
        normal
    }

    fn absolute(&self) -> bool {
        self.rooted || !self.prefix.is_empty()
    }

    /// The path as text, with `/`.
    fn text(&self) -> String {
        let root = if self.rooted { "/" } else { "" };
        format!("{}{root}{}", self.prefix, self.segments.join("/"))
    }
}

/// Whether two names are the same; Windows file names differ only in case are one file.
fn same(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// `file` as a path relative to `root`, with `/` as separator, resolved lexically: a relative
/// path is taken from `root`, a `\\?\` prefix is dropped, `.` and `..` are resolved by
/// components, case is ignored on Windows, and nothing is canonicalised. A file outside `root`
/// keeps its absolute normal form.
#[must_use]
pub fn repo_relative(root: &Path, file: &str) -> String {
    let root = Normal::of(&root.to_string_lossy());
    let mut normal = Normal::of(file);
    if !normal.absolute() {
        normal = Normal::of(&format!("{}/{file}", root.text()));
    }
    let file = normal;
    let inside = same(&file.prefix, &root.prefix)
        && file.rooted == root.rooted
        && file.segments.len() >= root.segments.len()
        && root
            .segments
            .iter()
            .zip(&file.segments)
            .all(|(a, b)| same(a, b));
    if inside {
        file.segments[root.segments.len()..].join("/")
    } else {
        file.text()
    }
}

/// The dep-info file of the artifact whose first file is `artifact_file`: the file beside it
/// named like it, without a leading `lib` and with the extension `.d` (`libxtask-<hash>.rmeta`
/// gives `xtask-<hash>.d`, `build_script_build-<hash>.exe` gives `build_script_build-<hash>.d`).
#[must_use]
pub fn dep_info_of(artifact_file: &Path) -> PathBuf {
    let stem = artifact_file
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = stem.strip_prefix("lib").unwrap_or(&stem);
    artifact_file.with_file_name(format!("{stem}.d"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_the_targets_without_dependencies() {
        let text = "target\\debug\\deps\\xtask-0a1b.d: xtask\\src\\main.rs xtask\\src\\lib.rs\n\
                    \n\
                    target\\debug\\deps\\libxtask-0a1b.rmeta: xtask\\src\\main.rs\n\
                    \n\
                    xtask\\src\\main.rs:\n\
                    xtask\\src\\main.rs:\n\
                    c\\ d.rs:\n\
                    # env-dep:CARGO_PKG_NAME=xtask\n";
        assert_eq!(parse_dep_info(text), ["c d.rs", "xtask/src/main.rs"]);
    }

    #[test]
    fn repo_relative_resolves_dots_and_strips_the_verbatim_prefix() {
        let root = std::env::temp_dir().join("repo");
        assert_eq!(
            repo_relative(&root, "crates/a/src/../x.rs"),
            "crates/a/x.rs"
        );
        assert_eq!(repo_relative(&root, "./crates/a/x.rs"), "crates/a/x.rs");
        assert_eq!(repo_relative(&root, "/elsewhere/x.rs"), "/elsewhere/x.rs");
        let inside = root.join("clippy.toml");
        assert_eq!(
            repo_relative(&root, &inside.to_string_lossy()),
            "clippy.toml"
        );
        if cfg!(windows) {
            let verbatim = format!(r"\\?\{}", inside.display());
            assert_eq!(repo_relative(&root, &verbatim), "clippy.toml");
            let shouting = inside.to_string_lossy().to_uppercase();
            assert_eq!(repo_relative(&root, &shouting), "CLIPPY.TOML");
        }
    }

    #[test]
    fn dep_info_path_strips_lib_and_the_extension() {
        let dir = Path::new("target").join("debug").join("deps");
        assert_eq!(
            dep_info_of(&dir.join("libxtask-0a1b.rmeta")),
            dir.join("xtask-0a1b.d")
        );
        assert_eq!(
            dep_info_of(&dir.join("build_script_build-0a1b.exe")),
            dir.join("build_script_build-0a1b.d")
        );
        assert_eq!(
            dep_info_of(&dir.join("liblibfoo-0a1b.rmeta")),
            dir.join("libfoo-0a1b.d")
        );
    }
}
