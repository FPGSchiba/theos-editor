//! The rule table: every rule the validators report, with its default severity and class.

use crate::finding::Severity;
use crate::kinds::KindRegistry;
use crate::lore::LoreRule;

/// How a project may change a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleClass {
    /// Always an error; nothing lowers it (G8).
    Guardrail,
    /// The project's settings may set its severity.
    Configurable,
    /// Keeps its default severity; settings cannot change it.
    Fixed,
}

impl RuleClass {
    /// The class as written in settings messages and in the rule fingerprint.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RuleClass::Guardrail => "guardrail",
            RuleClass::Configurable => "configurable",
            RuleClass::Fixed => "fixed",
        }
    }
}

/// One rule: its id, default severity and class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleInfo {
    /// The rule id, `group/name`.
    pub id: &'static str,
    /// The severity when nothing configures it.
    pub severity: Severity,
    /// Whether and how settings may change it.
    pub class: RuleClass,
}

impl RuleInfo {
    /// The rule's group: the text before the first `/`.
    #[must_use]
    pub fn group(&self) -> &'static str {
        self.id.split_once('/').map_or(self.id, |(group, _)| group)
    }
}

/// The rules that are always blocking (G8).
pub const GUARDRAILS: [&str; 3] = [
    "lore/never-confirm",
    "lore/no-overshoot",
    "lore/inworld-vocab",
];

/// Wraps `(id, severity)` pairs as fixed rules.
fn fixed(list: Vec<(&'static str, Severity)>) -> impl Iterator<Item = RuleInfo> {
    list.into_iter().map(|(id, severity)| RuleInfo {
        id,
        severity,
        class: RuleClass::Fixed,
    })
}

/// Every lore rule the index runs: those the parts of `registry` registered, then the
/// `settings/*` rules, which belong to no part.
pub(crate) fn lore_rules(registry: &KindRegistry) -> impl Iterator<Item = &LoreRule> {
    registry.lore_rules().chain(crate::settings::RULES)
}

/// Every rule, sorted by id, one entry per id: P02's and P03's, and the lore rules the parts of
/// `registry` registered.
///
/// Adding a rule list is one more `extend` line below.
#[must_use]
pub fn all(registry: &KindRegistry) -> Vec<RuleInfo> {
    let mut rules: Vec<RuleInfo> = Vec::new();
    rules.extend(fixed(crate::check::rules()));
    rules.extend(fixed(crate::index::resolve::rule_set()));
    rules.extend(lore_rules(registry).map(|rule| RuleInfo {
        id: rule.id,
        severity: rule.severity,
        class: rule.class,
    }));
    rules.sort_by_key(|rule| rule.id);
    rules.dedup_by_key(|rule| rule.id);
    rules
}

/// The severity a rule reports with, given what the settings configure for it.
///
/// A guardrail is always an error; a configurable rule takes its setting; a fixed rule keeps its
/// default.
#[must_use]
pub fn effective(info: &RuleInfo, configured: Option<Severity>) -> Severity {
    match info.class {
        RuleClass::Guardrail => Severity::Error,
        RuleClass::Configurable => configured.unwrap_or(info.severity),
        RuleClass::Fixed => info.severity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(severity: Severity, class: RuleClass) -> RuleInfo {
        RuleInfo {
            id: "x/y",
            severity,
            class,
        }
    }

    // AC: CE-14.1
    #[test]
    fn guardrails_are_errors_whatever_they_are_given() {
        for default in [Severity::Error, Severity::Warning] {
            let rule = info(default, RuleClass::Guardrail);
            for given in [None, Some(Severity::Error), Some(Severity::Warning)] {
                assert_eq!(effective(&rule, given), Severity::Error);
            }
        }
    }

    #[test]
    fn configurable_rules_take_their_setting() {
        let rule = info(Severity::Error, RuleClass::Configurable);
        assert_eq!(effective(&rule, Some(Severity::Warning)), Severity::Warning);
        assert_eq!(effective(&rule, None), Severity::Error);
    }

    #[test]
    fn fixed_rules_keep_their_default() {
        let rule = info(Severity::Error, RuleClass::Fixed);
        assert_eq!(effective(&rule, Some(Severity::Warning)), Severity::Error);
    }

    #[test]
    fn group_and_class_names_are_spelled_out() {
        assert_eq!(info(Severity::Error, RuleClass::Fixed).group(), "x");
        assert_eq!(RuleClass::Configurable.as_str(), "configurable");
        assert_eq!(RuleClass::Guardrail.as_str(), "guardrail");
        assert_eq!(RuleClass::Fixed.as_str(), "fixed");
    }

    #[test]
    fn p02_and_p03_rules_are_fixed_and_listed_once() -> Result<(), Box<dyn std::error::Error>> {
        let rules = all(&crate::parts::builtin()?);
        let find = |id: &str| rules.iter().find(|r| r.id == id);
        let dangling = find("link/dangling").ok_or("no link/dangling")?;
        assert_eq!(dangling.class, RuleClass::Fixed);
        assert_eq!(dangling.severity, Severity::Error);
        let unknown = find("schema/unknown-field").ok_or("no schema/unknown-field")?;
        assert_eq!(unknown.class, RuleClass::Fixed);
        assert_eq!(unknown.severity, Severity::Warning);
        assert!(rules.windows(2).all(|pair| pair[0].id < pair[1].id));
        Ok(())
    }

    #[test]
    fn settings_rules_are_listed_once_and_fixed() -> Result<(), Box<dyn std::error::Error>> {
        let rules = all(&crate::parts::builtin()?);
        let settings: Vec<&RuleInfo> = rules
            .iter()
            .filter(|rule| rule.group() == "settings")
            .collect();
        assert_eq!(settings.len(), 8);
        assert!(settings.iter().all(|rule| rule.class == RuleClass::Fixed));
        let warning: Vec<&str> = settings
            .iter()
            .filter(|rule| rule.severity == Severity::Warning)
            .map(|rule| rule.id)
            .collect();
        assert_eq!(warning, ["settings/unknown-rule"]);
        Ok(())
    }

    #[test]
    fn registered_lore_rules_are_listed_once() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = crate::parts::builtin()?;
        let before = all(&registry).len();
        for (part, kinds) in [("concepts", &["concept"]), ("maps", &["map"])] {
            registry.register_rule(
                part,
                crate::lore::LoreRule {
                    id: "test/order",
                    severity: Severity::Warning,
                    class: RuleClass::Configurable,
                    on: crate::lore::On::Kinds(kinds),
                    run: |_, _| Ok(Vec::new()),
                },
            )?;
        }
        let rules = all(&registry);
        assert_eq!(rules.len(), before + 1);
        let found: Vec<RuleInfo> = rules
            .iter()
            .filter(|r| r.id == "test/order")
            .copied()
            .collect();
        assert_eq!(
            found,
            [RuleInfo {
                id: "test/order",
                severity: Severity::Warning,
                class: RuleClass::Configurable,
            }]
        );
        Ok(())
    }
}
