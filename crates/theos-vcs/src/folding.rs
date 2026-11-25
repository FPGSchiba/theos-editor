//! How git compares the paths of a repository: case-folded under `core.ignorecase`, in Unicode
//! NFC under `core.precomposeunicode`, byte for byte otherwise.
//!
//! git's index is the authority for how a path is spelled. A file system that ignores case (NTFS,
//! APFS) or stores another normalization (NFD on macOS) can hold the same entry under another
//! spelling; under the repository's folding both name one entry.

use std::borrow::Cow;
use std::path::Path;

use unicode_normalization::UnicodeNormalization;

use crate::git::{Git, GitError};
use crate::repo;

/// How the repository folds paths before it compares them. Both off compares bytes, which is
/// git's default on Linux.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Folding {
    /// `core.ignorecase`: names that differ only in case are the same.
    pub ignore_case: bool,
    /// `core.precomposeunicode`: names are compared in Unicode NFC.
    pub precompose: bool,
}

impl Folding {
    /// Reads `core.ignorecase` and `core.precomposeunicode` of the repository at `top`. A setting
    /// that is not set is false.
    ///
    /// # Errors
    ///
    /// The errors of [`repo::flag`].
    pub fn read(git: &Git, top: &Path) -> Result<Folding, GitError> {
        Ok(Folding {
            ignore_case: repo::flag(git, top, "core.ignorecase")?,
            precompose: repo::flag(git, top, "core.precomposeunicode")?,
        })
    }

    /// `name` as this folding compares it: in NFC when `precompose`, then lower case when
    /// `ignore_case`. A path folds to its folded components, joined with `/` as before.
    #[must_use]
    pub fn fold<'a>(&self, name: &'a str) -> Cow<'a, str> {
        let mut text = Cow::Borrowed(name);
        if self.precompose {
            text = Cow::Owned(text.nfc().collect());
        }
        if self.ignore_case {
            text = Cow::Owned(text.to_lowercase());
        }
        text
    }

    /// Whether the path components `a` and `b` name the same entry under this folding.
    #[must_use]
    pub fn same(&self, a: &str, b: &str) -> bool {
        a == b || self.fold(a) == self.fold(b)
    }

    /// The rest of the repository path `path` below the folder `place` (components joined with
    /// `/`, empty for the top), when the first components of `path` are those of `place` under
    /// this folding. The rest keeps the spelling of `path`. A component that is not UTF-8 never
    /// matches a component of `place`.
    #[must_use]
    pub fn below<'a>(&self, place: &str, path: &'a [u8]) -> Option<&'a [u8]> {
        if place.is_empty() {
            return Some(path);
        }
        let mut rest = path;
        for part in place.split('/') {
            let slash = rest.iter().position(|byte| *byte == b'/')?;
            let head = std::str::from_utf8(&rest[..slash]).ok()?;
            if !self.same(part, head) {
                return None;
            }
            rest = &rest[slash + 1..];
        }
        Some(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXACT: Folding = Folding {
        ignore_case: false,
        precompose: false,
    };
    const CASE: Folding = Folding {
        ignore_case: true,
        precompose: false,
    };
    const NFC: Folding = Folding {
        ignore_case: false,
        precompose: true,
    };
    const BOTH: Folding = Folding {
        ignore_case: true,
        precompose: true,
    };

    /// `é` composed (NFC) and as `e` plus a combining acute accent (NFD).
    const COMPOSED: &str = "Caf\u{e9}";
    const DECOMPOSED: &str = "Cafe\u{301}";

    #[test]
    fn case_is_folded_only_under_ignore_case() {
        assert!(CASE.same("C-900.md", "c-900.md"));
        assert!(CASE.same("Ärger", "ärger"));
        assert!(!EXACT.same("C-900.md", "c-900.md"));
        assert!(!NFC.same("C-900.md", "c-900.md"));
        assert!(!CASE.same("C-900.md", "C-901.md"));
    }

    #[test]
    fn normalization_is_folded_only_under_precompose() {
        assert!(NFC.same(COMPOSED, DECOMPOSED));
        assert!(!EXACT.same(COMPOSED, DECOMPOSED));
        assert!(!CASE.same(COMPOSED, DECOMPOSED));
        assert!(BOTH.same("caf\u{e9}", DECOMPOSED));
    }

    #[test]
    fn both_off_is_byte_exact() {
        assert!(EXACT.same("a.md", "a.md"));
        assert_eq!(EXACT.fold(DECOMPOSED), DECOMPOSED);
        assert!(matches!(EXACT.fold("A.md"), Cow::Borrowed("A.md")));
    }

    #[test]
    fn below_strips_the_place_by_components() {
        assert_eq!(CASE.below("", b"a/b.md"), Some(b"a/b.md".as_slice()));
        assert_eq!(
            CASE.below("Content", b"content/lore/A.md"),
            Some(b"lore/A.md".as_slice())
        );
        assert_eq!(EXACT.below("Content", b"content/lore/A.md"), None);
        assert_eq!(
            NFC.below(
                &format!("x/{DECOMPOSED}"),
                format!("x/{COMPOSED}/a.md").as_bytes()
            ),
            Some(b"a.md".as_slice())
        );
        assert_eq!(CASE.below("content", b"content2/a.md"), None);
        assert_eq!(CASE.below("content", b"content"), None);
        assert_eq!(CASE.below("a/b", b"a/c/b/x.md"), None);
        assert_eq!(CASE.below("a", b"\xff/x.md"), None);
        assert_eq!(CASE.below("a", b"a/\xff.md"), Some(b"\xff.md".as_slice()));
    }
}
