//! The base of a command: what the caller saw of a file.

use std::fmt;

use sha2::{Digest, Sha256};

/// The SHA-256 of a file's bytes, as lower-case hex. A caller keeps the base of what it showed
/// the user and hands it back with the command that changes the file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Base(String);

impl Base {
    /// The base of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Base {
        let digest = Sha256::digest(bytes);
        let hex = digest
            .iter()
            .flat_map(|byte| [byte >> 4, byte & 0xf])
            .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
            .collect();
        Base(hex)
    }

    /// The hex digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_is_the_sha256_hex() {
        let base = Base::of(b"");
        assert_eq!(
            base.as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(base.to_string(), base.as_str());
        assert_ne!(Base::of(b"a"), base);
    }
}
