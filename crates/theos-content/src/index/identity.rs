//! What a cache was built from; when it changes, the cache is rebuilt.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::kinds::KindRegistry;
use crate::project::SCHEMA_VERSION;
use crate::rules::RuleInfo;

/// The layout of the cache tables; bump it when `schema.sql` changes.
pub(crate) const FORMAT: i64 = 2;

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
            rules: rules_fingerprint(&rule_set(registry)),
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

/// Every rule the index reports, from the rule table.
pub(crate) fn rule_set(registry: &KindRegistry) -> Vec<RuleInfo> {
    crate::rules::all(registry)
}

/// The SHA-256 of the sorted lines `rule<TAB>severity<TAB>class`, as lower-case hex.
pub(crate) fn rules_fingerprint(rules: &[RuleInfo]) -> String {
    let mut lines: Vec<String> = rules
        .iter()
        .map(|rule| {
            format!(
                "{}	{}	{}
",
                rule.id,
                rule.severity.as_str(),
                rule.class.as_str()
            )
        })
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
    use crate::finding::Severity;
    use crate::rules::RuleClass;

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
        let registry = KindRegistry::new();
        let current = Identity::current(&registry);
        assert_eq!(current.format, FORMAT);
        assert_eq!(current.source, SOURCE);
        assert_eq!(current.registry.len(), 64);
        assert_eq!(current.rules, rules_fingerprint(&rule_set(&registry)));
    }

    fn rule(id: &'static str, severity: Severity, class: RuleClass) -> RuleInfo {
        RuleInfo {
            id,
            severity,
            class,
        }
    }

    #[test]
    fn rules_fingerprint_is_order_free() {
        let one = rule("a/one", Severity::Error, RuleClass::Fixed);
        let two = rule("b/two", Severity::Warning, RuleClass::Fixed);
        let other = rule("a/one", Severity::Warning, RuleClass::Fixed);
        assert_eq!(
            rules_fingerprint(&[one, two]),
            rules_fingerprint(&[two, one])
        );
        assert_ne!(
            rules_fingerprint(&[one, two]),
            rules_fingerprint(&[other, two])
        );
    }

    #[test]
    fn the_rules_fingerprint_changes_with_a_class() {
        let fixed = rule("a/one", Severity::Error, RuleClass::Fixed);
        let configurable = rule("a/one", Severity::Error, RuleClass::Configurable);
        assert_ne!(
            rules_fingerprint(&[fixed]),
            rules_fingerprint(&[configurable])
        );
    }
}
