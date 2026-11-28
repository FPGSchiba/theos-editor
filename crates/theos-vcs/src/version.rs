//! The version of git: reading the answer of `git --version`.

use std::fmt;

/// A git version: the first three numbers of `git --version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitVersion {
    /// The major number.
    pub major: u32,
    /// The minor number.
    pub minor: u32,
    /// The patch number; 0 when git's answer has none.
    pub patch: u32,
}

impl GitVersion {
    /// The oldest git theos runs with.
    pub const MINIMUM: GitVersion = GitVersion {
        major: 2,
        minor: 38,
        patch: 0,
    };

    /// Reads the answer of `git --version`.
    ///
    /// Takes the text after `git version `, then at most three dot-separated runs of digits from
    /// its start; the first other character ends the version. A missing patch is 0; a missing
    /// major or minor, a different program name or a number above `u32::MAX` gives `None`.
    #[must_use]
    pub fn parse(answer: &str) -> Option<GitVersion> {
        let mut rest = answer.strip_prefix("git version ")?;
        let mut numbers = [0u32; 3];
        for (index, slot) in numbers.iter_mut().enumerate() {
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            if digits == 0 {
                // A missing patch is 0; a missing major or minor is no version.
                return (index >= 2).then_some(GitVersion::from(numbers));
            }
            *slot = rest[..digits].parse().ok()?;
            rest = &rest[digits..];
            match rest.strip_prefix('.') {
                Some(after) => rest = after,
                // The version ends here: a missing minor is refused, a missing patch is 0.
                None if index == 0 => return None,
                None => break,
            }
        }
        Some(GitVersion::from(numbers))
    }
}

impl From<[u32; 3]> for GitVersion {
    fn from([major, minor, patch]: [u32; 3]) -> Self {
        GitVersion {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for GitVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::GitVersion;

    fn v(major: u32, minor: u32, patch: u32) -> GitVersion {
        GitVersion {
            major,
            minor,
            patch,
        }
    }

    // AC: CE-21.3
    #[test]
    fn parses_the_shapes_git_prints() {
        assert_eq!(
            GitVersion::parse("git version 2.39.3 (Apple Git-146)\n"),
            Some(v(2, 39, 3))
        );
        assert_eq!(
            GitVersion::parse("git version 2.56.1.windows.1"),
            Some(v(2, 56, 1))
        );
        assert_eq!(
            GitVersion::parse("git version 2.38.0.rc1"),
            Some(v(2, 38, 0))
        );
        assert_eq!(
            GitVersion::parse("git version 2.44.0.123.gabcdef"),
            Some(v(2, 44, 0))
        );
        assert_eq!(GitVersion::parse("git version 2.40"), Some(v(2, 40, 0)));
    }

    // AC: CE-21.3
    #[test]
    fn refuses_answers_that_are_not_a_version() {
        for answer in [
            "",
            "git version",
            "git version abc",
            "hg version 2.40.0",
            "git version .2.3",
            "git version 2",
            "git version 4294967296.1.0",
        ] {
            assert_eq!(GitVersion::parse(answer), None, "{answer:?}");
        }
    }

    // AC: CE-21.3
    #[test]
    fn orders_against_the_minimum() {
        assert!(
            GitVersion {
                major: 2,
                minor: 37,
                patch: 99
            } < GitVersion::MINIMUM
        );
        assert_eq!(
            GitVersion {
                major: 2,
                minor: 38,
                patch: 0
            },
            GitVersion::MINIMUM
        );
        let rc = GitVersion::parse("git version 2.38.0.rc1");
        assert!(rc.is_some_and(|version| version >= GitVersion::MINIMUM));
    }

    // AC: CE-21.3
    #[test]
    fn displays_three_numbers() {
        assert_eq!(GitVersion::MINIMUM.to_string(), "2.38.0");
    }
}
