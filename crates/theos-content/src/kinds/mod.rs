//! The kind registry: kinds are registered by parts, never listed in one place.

mod fingerprint;
mod spec;
mod types;

use regex::Regex;

pub use spec::{
    BeatAttribute, BodyGrammar, CheckInput, FieldSpec, IdRule, KindCheck, KindSpec, Placement,
    Problem,
};
pub use types::{
    FieldType, LENS_ENTRIES, LensEntry, Narrow, TIERS, Target, VOICE_BELIEF, VOICE_SPEAKER,
};

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
}

#[derive(Clone, Debug)]
struct Registered {
    part: &'static str,
    spec: KindSpec,
    id: Regex,
}

/// All registered kinds.
#[derive(Clone, Debug, Default)]
pub struct KindRegistry {
    kinds: Vec<Registered>,
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

    /// A 64-digit lower-case hex fingerprint of every kind and its part, independent of the
    /// registration order.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let mut sorted: Vec<&Registered> = self.kinds.iter().collect();
        sorted.sort_by_key(|r| r.spec.name);
        fingerprint::digest(
            sorted
                .into_iter()
                .map(|r| (r.part, format!("{:?}", r.spec))),
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
