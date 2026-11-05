//! The twenty kinds of the data model and the four parts that register them.

use serde::Deserialize;
use theos_content::kinds::BeatAttribute;
use theos_content::parts::{self, codex, concepts, maps, scripts};
use theos_content::{KindRegistry, Placement, RegistryError};
use theos_content::{Order, Target};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Transcription {
    kind: Vec<Transcribed>,
}

#[derive(Deserialize)]
struct Transcribed {
    name: String,
    part: String,
    fields: Vec<String>,
}

fn transcription() -> Result<Vec<Transcribed>, Box<dyn std::error::Error>> {
    let text = include_str!("../fixtures/data-model-kinds.toml");
    Ok(toml::from_str::<Transcription>(text)?.kind)
}

fn sorted_names(registry: &KindRegistry) -> Vec<&'static str> {
    let mut names: Vec<_> = registry.kinds().map(|k| k.name).collect();
    names.sort_unstable();
    names
}

// AC: CE-11.1
#[test]
fn every_kind_of_the_data_model_has_its_schema() -> TestResult {
    let registry = parts::builtin()?;
    let expected = transcription()?;
    assert_eq!(registry.kinds().count(), 20);
    let mut want: Vec<&str> = expected.iter().map(|k| k.name.as_str()).collect();
    want.sort_unstable();
    assert_eq!(sorted_names(&registry), want);
    for kind in &expected {
        let spec = registry.kind(&kind.name).ok_or("kind is registered")?;
        assert_eq!(registry.part_of(&kind.name), Some(kind.part.as_str()));
        let fields: Vec<String> = spec.fields.iter().map(ToString::to_string).collect();
        assert_eq!(fields, kind.fields, "fields of {}", kind.name);
    }
    Ok(())
}

// AC: CE-11.5
#[test]
fn each_part_registers_its_kinds() -> TestResult {
    type Register = fn(&mut KindRegistry) -> Result<(), RegistryError>;
    let cases: [(Register, &[&str]); 4] = [
        (
            codex::register,
            &[
                "band", "belief", "clue", "equation", "faction", "floor", "location", "npc",
                "people", "thing", "topic", "tower", "truth",
            ],
        ),
        (scripts::register, &["dialogue", "script", "template"]),
        (concepts::register, &["concept", "concept-version"]),
        (maps::register, &["map", "map-version"]),
    ];
    for (register, names) in cases {
        let mut registry = KindRegistry::new();
        register(&mut registry)?;
        assert_eq!(sorted_names(&registry), names);
    }
    let mut registry = KindRegistry::new();
    codex::register(&mut registry)?;
    assert!(matches!(
        codex::register(&mut registry),
        Err(RegistryError::DuplicateKind { .. })
    ));
    Ok(())
}

#[test]
fn placements_and_grammars_follow_the_data_model() -> TestResult {
    use Placement::{Flat, UnderParent};
    let registry = parts::builtin()?;
    let placements = [
        ("tower", Flat("world")),
        ("band", Flat("world")),
        ("floor", UnderParent("parent")),
        ("location", UnderParent("parent")),
        ("people", Flat("people")),
        ("faction", UnderParent("parent")),
        ("npc", UnderParent("parent")),
        ("thing", Flat("things")),
        ("topic", Flat("lore/topics")),
        ("truth", Flat("lore/truths")),
        ("belief", Flat("lore/beliefs")),
        ("clue", Flat("lore/clues")),
        ("equation", Flat("canon/formalism")),
        ("script", Flat("story/scripts")),
        ("dialogue", Flat("dialogue")),
        ("template", Flat("templates")),
        ("concept", Flat("concepts")),
        ("concept-version", UnderParent("concept")),
        ("map", Flat("maps")),
        ("map-version", UnderParent("map")),
    ];
    for (name, placement) in placements {
        let spec = registry.kind(name).ok_or("kind is registered")?;
        assert_eq!(spec.placement, placement, "placement of {name}");
    }
    for spec in registry.kinds() {
        assert_eq!(
            spec.body.beats.is_some(),
            spec.name == "script",
            "{}",
            spec.name
        );
        assert_eq!(
            spec.body.notes.is_some(),
            spec.name == "concept" || spec.name == "map",
            "{}",
            spec.name
        );
    }
    let script = registry.kind("script").ok_or("script")?;
    let beats: Vec<(&str, Vec<String>)> = script
        .body
        .beats
        .unwrap_or_default()
        .iter()
        .map(|BeatAttribute { key, targets }| {
            (*key, targets.iter().map(ToString::to_string).collect())
        })
        .collect();
    let want: [(&str, &[&str]); 4] = [
        ("at", &["location", "floor"]),
        ("with", &["npc", "faction", "people"]),
        ("says", &["belief"]),
        ("reveals", &["clue"]),
    ];
    assert_eq!(beats.len(), 4);
    for ((key, targets), (wkey, wtargets)) in beats.iter().zip(want) {
        assert_eq!(*key, wkey);
        assert_eq!(targets, wtargets);
    }
    let notes = |name: &str| -> Option<&'static [Target]> {
        registry.kind(name).and_then(|s| s.body.notes)
    };
    assert_eq!(
        notes("concept"),
        Some(&[Target::kind("concept-version")][..])
    );
    assert_eq!(notes("map"), Some(&[Target::kind("map-version")][..]));
    Ok(())
}

// AC: CE-11.5
#[test]
fn orders_follow_the_data_model() -> TestResult {
    let registry = parts::builtin()?;
    let npc = registry.kind("npc").ok_or("npc")?;
    let expected = Order::new([
        "id", "kind", "title", "status", "tags", "parent", "home", "role", "lens",
    ])
    .with_entries(
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
    assert_eq!(npc.order(), expected);
    let clue = registry.kind("clue").ok_or("clue")?;
    assert_eq!(
        clue.order(),
        Order::new([
            "id", "kind", "title", "status", "tags", "reveals", "moves_to", "where", "gate",
            "after", "vector"
        ])
    );
    Ok(())
}
