//! What a cache was built from; when it changes, the cache is rebuilt.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::finding::Severity;
use crate::kinds::KindRegistry;
use crate::project::SCHEMA_VERSION;

/// The layout of the cache tables; bump it when `schema.sql` changes.
pub(crate) const FORMAT: i64 = 1;

/// The fingerprint of this crate's sources, computed by `build.rs`.
pub(crate) const SOURCE: &str = env!("THEOS_CONTENT_SOURCE_HASH");

/// The six facts a cache is valid for.
#[derive(Clone)]
pub(crate) struct Identity {
    pub format: i64,
    pub crate_version: &'static str,
    pub schema_version: i64,
    pub registry: String,
    pub rules: String,
    pub source: &'static str,
}

impl Identity {
    /// The identity of this build with `registry`.
    pub(crate) fn current(registry: &KindRegistry) -> Identity {
        Identity {
            format: FORMAT,
            crate_version: env!("CARGO_PKG_VERSION"),
            schema_version: SCHEMA_VERSION,
            registry: registry.fingerprint(),
            rules: rules_fingerprint(&rule_set()),
            source: SOURCE,
        }
    }

    /// The `meta` rows of this identity.
    pub(crate) fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("format", self.format.to_string()),
            ("crate_version", self.crate_version.to_owned()),
            ("schema_version", self.schema_version.to_string()),
            ("registry", self.registry.clone()),
            ("rules", self.rules.clone()),
            ("source", self.source.to_owned()),
        ]
    }

    /// The `meta` table of this identity, by key.
    pub(crate) fn meta(&self) -> BTreeMap<String, String> {
        self.pairs()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }
}

/// Every rule with its default severity that the index reports.
pub(crate) fn rule_set() -> Vec<(&'static str, Severity)> {
    let mut rules = crate::check::rules();
    rules.extend(crate::index::resolve::rule_set());
    rules
}

/// The SHA-256 of the sorted lines `rule<TAB>severity`, as lower-case hex.
pub(crate) fn rules_fingerprint(rules: &[(&str, Severity)]) -> String {
    let mut lines: Vec<String> = rules
        .iter()
        .map(|(rule, severity)| format!("{rule}\t{}\n", severity.as_str()))
        .collect();
    lines.sort();
    let mut hasher = Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
    }
    let mut hex = String::new();
    for byte in hasher.finalize() {
        // Writing to a String cannot fail.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_fingerprint_is_built_in() {
        assert_eq!(SOURCE.len(), 64);
        assert!(
            SOURCE
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        );
    }

    #[test]
    fn the_current_identity_is_made_of_the_build() {
        let current = Identity::current(&KindRegistry::new());
        assert_eq!(current.format, FORMAT);
        assert_eq!(current.source, SOURCE);
        assert_eq!(current.registry.len(), 64);
        assert_eq!(current.rules, rules_fingerprint(&rule_set()));
    }

    #[test]
    fn rules_fingerprint_is_order_free() {
        let a = [("a/one", Severity::Error), ("b/two", Severity::Warning)];
        let b = [("b/two", Severity::Warning), ("a/one", Severity::Error)];
        let c = [("a/one", Severity::Warning), ("b/two", Severity::Warning)];
        assert_eq!(rules_fingerprint(&a), rules_fingerprint(&b));
        assert_ne!(rules_fingerprint(&a), rules_fingerprint(&c));
    }
}
