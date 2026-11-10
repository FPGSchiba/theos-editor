//! The fingerprint of a registry: part of the cache identity.

use std::fmt::Write;

use sha2::{Digest, Sha256};

/// SHA-256 of the lines `"{part}\t{spec}\n"`, as lower-case hex. The caller sorts the lines.
pub(super) fn digest<'a>(lines: impl IntoIterator<Item = (&'a str, String)>) -> String {
    let mut hasher = Sha256::new();
    for (part, spec) in lines {
        hasher.update(format!("{part}\t{spec}\n").as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            // Writing to a `String` cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}
