//! The kind registry: kinds and lore rules are registered by parts, never listed in one place.

mod fingerprint;
mod spec;
mod types;
mod value;

use std::collections::BTreeSet;

use regex::Regex;

use crate::lore::LoreRule;

pub use spec::{
    BeatAttribute, BodyGrammar, CheckInput, FieldSpec, IdRule, KindCheck, KindSpec, Placement,
    Problem,
};
pub use types::{
    FieldType, LENS_ENTRIES, LensEntry, Narrow, TIERS, Target, VOICE_BELIEF, VOICE_SPEAKER,
};
pub use value::ValueRef;

/// Why a kind could not be registered.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Two parts registered the same kind.
    #[error("kind `{kind}` is registered by part `{first}` and again by part `{second}`")]
    DuplicateKind {
        /// The kind name.
        kind: String,
        /// The part that registered it first.
        first: &'static str,
        /// The part that tried again.
        second: &'static str,
    },
    /// The id pattern of a kind is not a regular expression.
    #[error("the id pattern of kind `{kind}` is not a valid regular expression")]
    IdPattern {
        /// The kind name.
        kind: String,
        /// What the regex crate says.
        #[source]
        source: regex::Error,
    },
    /// Two parts registered one lore rule id with another severity or class.
    #[error(
        "rule `{rule}` is registered by part `{first}` and again by part `{second}` with another \
         severity or class"
    )]
    DuplicateRule {
        /// The rule id.
        rule: String,
        /// The part that registered it first.
        first: &'static str,
        /// The part that tried again.
        second: &'static str,
    },
    /// Two registrations of one lore rule id check the same subjects: a kind in both lists,
    /// `AnyRecord` next to any record rule, or one input path twice. The rule would run twice on
    /// one subject and report every finding twice.
    #[error(
        "rule `{rule}` is registered by part `{first}` and again by part `{second}` for          subjects that overlap"
    )]
    OverlappingRule {
        /// The rule id.
        rule: String,
        /// The part that registered it first.
        first: &'static str,
        /// The part that tried again.
        second: &'static str,
    },
}

#[derive(Clone, Debug)]
struct Registered {
    part: &'static str,
    spec: KindSpec,
    id: Regex,
}

/// A lore rule and the part that registered it.
#[derive(Clone, Debug)]
struct RegisteredRule {
    part: &'static str,
    rule: LoreRule,
}

/// All registered kinds and lore rules.
#[derive(Clone, Debug, Default)]
pub struct KindRegistry {
    kinds: Vec<Registered>,
    rules: Vec<RegisteredRule>,
}

impl KindRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> KindRegistry {
        KindRegistry::default()
    }

    /// Registers `spec` for `part` and compiles its id pattern.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateKind`] when the kind exists already, and
    /// [`RegistryError::IdPattern`] when its id pattern does not compile.
    pub fn register(&mut self, part: &'static str, spec: KindSpec) -> Result<(), RegistryError> {
        if let Some(existing) = self.kinds.iter().find(|r| r.spec.name == spec.name) {
            return Err(RegistryError::DuplicateKind {
                kind: spec.name.to_owned(),
                first: existing.part,
                second: part,
            });
        }
        let id = Regex::new(spec.id.pattern).map_err(|source| RegistryError::IdPattern {
            kind: spec.name.to_owned(),
            source,
        })?;
        self.kinds.push(Registered { part, spec, id });
        Ok(())
    }

    /// Registers the lore rule `rule` for `part` (spec §5.2). One rule id may be registered by
    /// several parts, each for its own kinds, when severity and class agree and the subjects do
    /// not overlap.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateRule`] when the id is registered already with another severity
    /// or class, and [`RegistryError::OverlappingRule`] when it is registered already for a
    /// subject that `rule` checks too.
    pub fn register_rule(
        &mut self,
        part: &'static str,
        rule: LoreRule,
    ) -> Result<(), RegistryError> {
        if let Some(existing) = self.rules.iter().find(|registered| {
            registered.rule.id == rule.id
                && (registered.rule.severity, registered.rule.class) != (rule.severity, rule.class)
        }) {
            return Err(RegistryError::DuplicateRule {
                rule: rule.id.to_owned(),
                first: existing.part,
                second: part,
            });
        }
        if let Some(existing) = self.rules.iter().find(|registered| {
            registered.rule.id == rule.id && registered.rule.on.overlaps(rule.on)
        }) {
            return Err(RegistryError::OverlappingRule {
                rule: rule.id.to_owned(),
                first: existing.part,
                second: part,
            });
        }
        self.rules.push(RegisteredRule { part, rule });
        Ok(())
    }

    /// Every registered lore rule, in registration order; an id registered by several parts
    /// comes once per registration.
    pub fn lore_rules(&self) -> impl Iterator<Item = &LoreRule> {
        self.rules.iter().map(|registered| &registered.rule)
    }

    fn find(&self, name: &str) -> Option<&Registered> {
        self.kinds.iter().find(|r| r.spec.name == name)
    }

    /// The spec of the kind `name`.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<&KindSpec> {
        self.find(name).map(|r| &r.spec)
    }

    /// All specs, in registration order.
    pub fn kinds(&self) -> impl Iterator<Item = &KindSpec> {
        self.kinds.iter().map(|r| &r.spec)
    }

    /// The part that registered the kind `name`.
    #[must_use]
    pub fn part_of(&self, name: &str) -> Option<&'static str> {
        self.find(name).map(|r| r.part)
    }

    /// Whether `id` matches the id pattern of `kind`; false for an unknown kind.
    #[must_use]
    pub fn id_fits(&self, kind: &str, id: &str) -> bool {
        self.find(kind).is_some_and(|r| r.id.is_match(id))
    }

    /// Every field some target narrows by, in any slot: the fields whose value another record's
    /// links depend on (`track`, for one). A change of such a value is a link fact (spec §5.9).
    #[must_use]
    pub fn narrow_fields(&self) -> BTreeSet<&'static str> {
        let mut fields = BTreeSet::new();
        let mut take = |targets: &[Target]| {
            fields.extend(targets.iter().filter_map(|t| t.narrow).map(|n| n.field));
        };
        take(VOICE_SPEAKER);
        take(VOICE_BELIEF);
        for entry in LENS_ENTRIES {
            if let FieldType::Link(targets) | FieldType::Links(targets) = entry.ty {
                take(targets);
            }
        }
        for spec in self.kinds() {
            for field in &spec.fields {
                if let FieldType::Link(targets) | FieldType::Links(targets) = field.ty {
                    take(targets);
                }
            }
            for attribute in spec.body.beats.unwrap_or(&[]) {
                take(attribute.targets);
            }
            take(spec.body.notes.unwrap_or(&[]));
        }
        fields
    }

    /// A 64-digit lower-case hex fingerprint of every kind and every lore rule with its part,
    /// independent of the registration order.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let mut sorted: Vec<&Registered> = self.kinds.iter().collect();
        sorted.sort_by_key(|r| r.spec.name);
        let mut rules: Vec<(&'static str, String)> = self
            .rules
            .iter()
            .map(|r| (r.part, format!("{:?}", r.rule)))
            .collect();
        rules.sort();
        fingerprint::digest(
            sorted
                .into_iter()
                .map(|r| (r.part, format!("{:?}", r.spec)))
                .chain(rules),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml::Order;

    const SLUG: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";

    fn field(key: &'static str, ty: FieldType, required: bool) -> FieldSpec {
        FieldSpec { key, ty, required }
    }

    fn spec(name: &'static str, fields: Vec<FieldSpec>) -> KindSpec {
        KindSpec {
            name,
            id: IdRule {
                pattern: SLUG,
                says: "a slug",
            },
            placement: Placement::Flat("things"),
            fields,
            body: BodyGrammar::PROSE,
            checks: Vec::new(),
        }
    }

    fn no_problems(_: &CheckInput<'_>) -> Vec<Problem> {
        Vec::new()
    }

    // AC: CE-11.5
    #[test]
    fn an_empty_registry_knows_no_kind() {
        let registry = KindRegistry::new();
        assert_eq!(registry.kinds().count(), 0);
        assert!(registry.kind("npc").is_none());
        assert!(registry.part_of("npc").is_none());
        assert!(!registry.id_fits("npc", "a"));
    }

    // AC: CE-11.5
    #[test]
    fn registering_a_kind_twice_fails() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = KindRegistry::new();
        registry.register("a", spec("npc", Vec::new()))?;
        let err = registry.register("b", spec("npc", Vec::new()));
        assert!(matches!(
            err,
            Err(RegistryError::DuplicateKind { ref kind, first: "a", second: "b" }) if kind == "npc"
        ));
        assert_eq!(registry.part_of("npc"), Some("a"));
        assert_eq!(registry.kinds().count(), 1);
        Ok(())
    }

    // AC: CE-11.5
    #[test]
    fn order_follows_the_spec() {
        let with_lens = spec(
            "npc",
            vec![
                field("id", FieldType::Id, true),
                field("kind", FieldType::KindName, true),
                field("title", FieldType::Text, true),
                field("status", FieldType::Text, false),
                field("tags", FieldType::TextList, false),
                field("lens", FieldType::Lens, false),
            ],
        );
        let expected = Order::new(["id", "kind", "title", "status", "tags", "lens"]).with_entries(
            "lens",
            [
                "thinks_from",
                "stance",
                "tier",
                "unaware_of",
                "rejects",
                "speaks_to",
            ],
        );
        assert_eq!(with_lens.order(), expected);

        let plain = spec("topic", vec![field("id", FieldType::Id, true)]);
        assert_eq!(plain.order(), Order::new(["id"]));
        assert_eq!(plain.order().entries_of("lens"), [] as [String; 0]);
        assert_eq!(plain.field("id").map(|f| f.required), Some(true));
        assert!(plain.field("nope").is_none());
    }

    #[test]
    fn fingerprint_is_stable_and_follows_the_specs() -> Result<(), Box<dyn std::error::Error>> {
        let build = |first: &'static str, second: &'static str, required: bool, rule| {
            let mut registry = KindRegistry::new();
            for name in [first, second] {
                let mut s = spec(name, vec![field("id", FieldType::Id, required)]);
                s.checks.push(KindCheck {
                    rule,
                    run: no_problems,
                });
                registry.register("part", s)?;
            }
            Ok::<_, RegistryError>(registry.fingerprint())
        };
        let base = build("npc", "band", true, "x/y")?;
        assert_eq!(base.len(), 64);
        assert!(
            base.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert_eq!(base, build("band", "npc", true, "x/y")?);
        assert_ne!(base, build("npc", "band", false, "x/y")?);
        assert_ne!(base, build("npc", "band", true, "x/z")?);
        Ok(())
    }

    fn lore_rule(
        severity: crate::finding::Severity,
        class: crate::rules::RuleClass,
        kinds: &'static [&'static str],
    ) -> LoreRule {
        LoreRule {
            id: "test/rule",
            severity,
            class,
            on: crate::lore::On::Kinds(kinds),
            run: |_, _| Ok(Vec::new()),
        }
    }

    #[test]
    fn a_rule_id_twice_with_another_class_is_refused() -> Result<(), Box<dyn std::error::Error>> {
        use crate::finding::Severity::{Error, Warning};
        use crate::rules::RuleClass::{Configurable, Fixed};
        let mut registry = KindRegistry::new();
        registry.register_rule("concepts", lore_rule(Error, Fixed, &["concept"]))?;
        for (severity, class) in [(Error, Configurable), (Warning, Fixed)] {
            let err = registry.register_rule("maps", lore_rule(severity, class, &["map"]));
            assert!(matches!(
                &err,
                Err(RegistryError::DuplicateRule { rule, first: "concepts", second: "maps" })
                    if rule == "test/rule"
            ));
            assert_eq!(
                err.map_err(|e| e.to_string()).err().as_deref(),
                Some(
                    "rule `test/rule` is registered by part `concepts` and again by part `maps` \
                     with another severity or class"
                )
            );
        }
        assert_eq!(registry.lore_rules().count(), 1);
        Ok(())
    }

    #[test]
    fn a_rule_id_twice_for_overlapping_subjects_is_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::finding::Severity::Error;
        use crate::lore::On;
        use crate::rules::RuleClass::Fixed;
        let on = |on| LoreRule {
            on,
            ..lore_rule(Error, Fixed, &[])
        };
        let mut registry = KindRegistry::new();
        registry.register_rule("a", on(On::Kinds(&["concept", "map"])))?;
        registry.register_rule("a", on(On::Input(".theos/settings.toml")))?;
        for again in [
            On::Kinds(&["thing", "map"]),
            On::AnyRecord,
            On::Input(".theos/settings.toml"),
        ] {
            let err = registry.register_rule("b", on(again));
            assert!(
                matches!(
                    &err,
                    Err(RegistryError::OverlappingRule { second: "b", .. })
                ),
                "{again:?}: {err:?}"
            );
        }
        registry.register_rule("b", on(On::Kinds(&["thing"])))?;
        registry.register_rule("b", on(On::Input("canon/formalism/_source.json")))?;
        assert_eq!(registry.lore_rules().count(), 4);
        let mut registry = KindRegistry::new();
        registry.register_rule("a", on(On::AnyRecord))?;
        let err = registry.register_rule("b", on(On::Kinds(&["thing"])));
        assert!(matches!(err, Err(RegistryError::OverlappingRule { .. })));
        Ok(())
    }

    #[test]
    fn one_rule_id_may_come_from_several_parts() -> Result<(), Box<dyn std::error::Error>> {
        use crate::finding::Severity::Error;
        use crate::rules::RuleClass::Fixed;
        let mut registry = KindRegistry::new();
        registry.register_rule("concepts", lore_rule(Error, Fixed, &["concept"]))?;
        registry.register_rule("maps", lore_rule(Error, Fixed, &["map"]))?;
        let on: Vec<crate::lore::On> = registry.lore_rules().map(|rule| rule.on).collect();
        assert_eq!(
            on,
            [
                crate::lore::On::Kinds(&["concept"]),
                crate::lore::On::Kinds(&["map"])
            ]
        );
        Ok(())
    }

    #[test]
    fn the_fingerprint_follows_the_rules() -> Result<(), Box<dyn std::error::Error>> {
        use crate::finding::Severity::{Error, Warning};
        use crate::rules::RuleClass::Fixed;
        let build = |rules: &[(&'static str, LoreRule)]| {
            let mut registry = KindRegistry::new();
            for (part, rule) in rules {
                registry.register_rule(part, *rule)?;
            }
            Ok::<_, RegistryError>(registry.fingerprint())
        };
        let concept = lore_rule(Error, Fixed, &["concept"]);
        let map = lore_rule(Error, Fixed, &["map"]);
        let base = build(&[("concepts", concept), ("maps", map)])?;
        assert_eq!(base, build(&[("maps", map), ("concepts", concept)])?);
        assert_ne!(base, build(&[("concepts", concept)])?);
        assert_ne!(base, KindRegistry::new().fingerprint());
        assert_ne!(
            build(&[("concepts", concept)])?,
            build(&[("concepts", lore_rule(Warning, Fixed, &["concept"]))])?
        );
        assert_ne!(
            build(&[("concepts", concept)])?,
            build(&[("maps", concept)])?
        );
        Ok(())
    }

    #[test]
    fn id_patterns_are_compiled_at_registration() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = KindRegistry::new();
        registry.register("a", spec("npc", Vec::new()))?;
        assert!(registry.id_fits("npc", "a-b"));
        assert!(!registry.id_fits("npc", "A"));
        assert!(registry.kind("npc").is_some());

        let mut bad = spec("band", Vec::new());
        bad.id = IdRule {
            pattern: "(",
            says: "broken",
        };
        let err = registry.register("a", bad);
        assert!(matches!(err, Err(RegistryError::IdPattern { ref kind, .. }) if kind == "band"));
        assert!(registry.kind("band").is_none());
        Ok(())
    }
}
