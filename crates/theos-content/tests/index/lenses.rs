//! Effective lenses over the `lenses/` fixture project: the chain of layers, the layer each
//! value comes from, overrides, ignored entries and held beliefs.

use theos_content::{
    EffectiveLens, HeldBelief, Index, Layer, LensData, LensValue, Origin, Project, parts,
};

use crate::common::{expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FOLK: &str = "people/synthetic-folk.md";
const GUILD: &str = "people/synthetic-folk/synthetic-guild.md";
const ELDER: &str = "people/synthetic-folk/synthetic-guild/synthetic-elder.md";
const ODD: &str = "people/synthetic-folk/synthetic-guild/synthetic-odd.md";

fn indexed() -> Result<Index, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("lenses"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn lens_of(index: &Index, id: &str) -> Result<EffectiveLens, Box<dyn std::error::Error>> {
    index
        .lens(id)?
        .ok_or_else(|| format!("no lens for {id}").into())
}

fn layer(id: &str, kind: &str, path: &str) -> Layer {
    Layer {
        id: id.to_owned(),
        kind: kind.to_owned(),
        path: path.to_owned(),
    }
}

fn folk() -> Layer {
    layer("synthetic-folk", "people", FOLK)
}

fn guild() -> Layer {
    layer("synthetic-guild", "faction", GUILD)
}

fn elder() -> Layer {
    layer("synthetic-elder", "npc", ELDER)
}

fn text(value: &str) -> LensData {
    LensData::Text(value.to_owned())
}

fn links(ids: &[&str]) -> LensData {
    LensData::Links(ids.iter().map(|id| (*id).to_owned()).collect())
}

/// Checks an origin: the layer, the text the span covers in the fixture file, and the line and
/// column.
fn origin(layer: &Layer, covers: &str, line: usize, column: usize, actual: &Origin) -> TestResult {
    let file = std::fs::read_to_string(fixture("lenses").join(&layer.path))?;
    assert_eq!(&actual.layer, layer);
    assert_eq!(
        file.get(actual.span.start..actual.span.end),
        Some(covers),
        "span of {}",
        layer.id
    );
    assert_eq!(
        (actual.position.line, actual.position.column),
        (line, column)
    );
    Ok(())
}

fn field<'a>(lens: &'a EffectiveLens, name: &str) -> Option<&'a LensValue> {
    lens.fields.iter().find(|value| value.field == name)
}

fn need<'a>(lens: &'a EffectiveLens, name: &str) -> Result<&'a LensValue, String> {
    field(lens, name).ok_or_else(|| format!("no value for {name}"))
}

// AC: CE-13.2
#[test]
fn an_npc_lens_names_the_layer_of_each_value() -> TestResult {
    let lens = lens_of(&indexed()?, "synthetic-elder")?;
    assert_eq!(lens.chain, [elder(), guild(), folk()]);
    let names: Vec<&str> = lens.fields.iter().map(|v| v.field.as_str()).collect();
    assert_eq!(
        names,
        ["thinks_from", "stance", "tier", "unaware_of", "speaks_to"]
    );

    let thinks = need(&lens, "thinks_from")?;
    assert_eq!(thinks.value, text("synthetic frame A"));
    origin(&folk(), "synthetic frame A", 7, 16, &thinks.origin)?;
    let speaks = need(&lens, "speaks_to")?;
    assert_eq!(speaks.value, text("synthetic audience A"));
    origin(&folk(), "synthetic audience A", 11, 14, &speaks.origin)?;

    let stance = need(&lens, "stance")?;
    assert_eq!(stance.value, text("synthetic stance C"));
    origin(&elder(), "synthetic stance C", 8, 11, &stance.origin)?;
    let tier = need(&lens, "tier")?;
    assert_eq!(tier.value, text("T2"));
    origin(&elder(), "T2", 9, 9, &tier.origin)?;
    let unaware = need(&lens, "unaware_of")?;
    assert_eq!(unaware.value, links(&["T-962"]));
    origin(&elder(), "[T-962]", 10, 15, &unaware.origin)?;

    assert!(field(&lens, "rejects").is_none());
    Ok(())
}

// AC: CE-13.2
#[test]
fn an_npc_under_its_people() -> TestResult {
    let lens = lens_of(&indexed()?, "synthetic-loner")?;
    assert_eq!(
        lens.chain,
        [
            layer(
                "synthetic-loner",
                "npc",
                "people/synthetic-folk/synthetic-loner.md"
            ),
            folk()
        ]
    );
    assert_eq!(lens.fields.len(), 5);
    assert!(lens.fields.iter().all(|v| v.origin.layer == folk()));
    assert!(lens.fields.iter().all(|v| v.overrides.is_empty()));
    assert_eq!(need(&lens, "unaware_of")?.value, links(&["T-960", "T-961"]));
    Ok(())
}

// AC: CE-13.2
#[test]
fn beliefs_come_through_each_layer() -> TestResult {
    let lens = lens_of(&indexed()?, "synthetic-elder")?;
    let held = |belief: &str, through: Layer| HeldBelief {
        belief: belief.to_owned(),
        path: format!("lore/beliefs/{belief}.md"),
        through,
    };
    assert_eq!(
        lens.beliefs,
        [
            held("B-962", elder()),
            held("B-961", guild()),
            held("B-960", folk())
        ]
    );
    Ok(())
}

// AC: CE-13.3
#[test]
fn a_child_value_overrides_its_parent() -> TestResult {
    let lens = lens_of(&indexed()?, "synthetic-elder")?;

    let stance = need(&lens, "stance")?;
    assert_eq!(stance.overrides.len(), 2);
    assert_eq!(stance.overrides[0].value, text("synthetic stance B"));
    origin(
        &guild(),
        "synthetic stance B",
        8,
        11,
        &stance.overrides[0].origin,
    )?;
    assert_eq!(stance.overrides[1].value, text("synthetic stance A"));
    origin(
        &folk(),
        "synthetic stance A",
        8,
        11,
        &stance.overrides[1].origin,
    )?;

    let unaware = need(&lens, "unaware_of")?;
    assert_eq!(unaware.overrides.len(), 1);
    assert_eq!(unaware.overrides[0].value, links(&["T-960", "T-961"]));
    origin(
        &folk(),
        "[T-960, T-961]",
        10,
        15,
        &unaware.overrides[0].origin,
    )?;

    let tier = need(&lens, "tier")?;
    assert_eq!(tier.overrides.len(), 2);
    assert_eq!(tier.overrides[0].value, text("T2"));
    origin(&guild(), "T2", 9, 9, &tier.overrides[0].origin)?;
    assert_eq!(tier.overrides[1].value, text("T1"));
    origin(&folk(), "T1", 9, 9, &tier.overrides[1].origin)?;
    Ok(())
}

#[test]
fn invalid_entries_do_not_override() -> TestResult {
    let index = indexed()?;
    let odd = lens_of(&index, "synthetic-odd")?;
    let odd_layer = layer("synthetic-odd", "npc", ODD);
    assert_eq!(odd.chain, [odd_layer.clone(), guild(), folk()]);

    let tier = need(&odd, "tier")?;
    assert_eq!(tier.value, text("T2"));
    assert_eq!(tier.origin.layer, guild());
    assert_eq!(tier.overrides.len(), 1);
    let unaware = need(&odd, "unaware_of")?;
    assert_eq!(unaware.value, links(&["T-960", "T-961"]));
    assert_eq!(unaware.origin.layer, folk());
    assert_eq!(unaware.overrides.len(), 0);

    let ignored: Vec<&str> = odd.ignored.iter().map(|i| i.field.as_str()).collect();
    assert_eq!(ignored, ["tier", "unaware_of"]);
    origin(&odd_layer, "T4", 8, 9, &odd.ignored[0].origin)?;
    origin(&odd_layer, "T-960", 9, 15, &odd.ignored[1].origin)?;
    for entry in &odd.ignored {
        assert_eq!(entry.findings.len(), 1);
        assert_eq!(entry.findings[0].rule, "schema/value");
        assert_eq!(entry.findings[0].path, ODD);
    }
    assert!(field(&odd, "mood").is_none());

    let flat = lens_of(&index, "synthetic-flat")?;
    let flat_layer = layer(
        "synthetic-flat",
        "npc",
        "people/synthetic-folk/synthetic-flat.md",
    );
    assert_eq!(flat.ignored.len(), 1);
    assert_eq!(flat.ignored[0].field, "lens");
    origin(&flat_layer, "synthetic", 7, 7, &flat.ignored[0].origin)?;
    assert_eq!(flat.ignored[0].findings.len(), 1);
    assert_eq!(flat.fields.len(), 5);
    assert!(flat.fields.iter().all(|v| v.origin.layer == folk()));
    Ok(())
}

#[test]
fn bad_parents_end_the_chain_and_non_holders_have_none() -> TestResult {
    let index = indexed()?;
    let stray = lens_of(&index, "synthetic-stray")?;
    assert_eq!(
        stray.chain,
        [layer("synthetic-stray", "npc", "people/synthetic-stray.md")]
    );
    assert_eq!(stray.fields.len(), 0);
    assert!(index.lens("T-960")?.is_none());
    assert!(index.lens("nobody")?.is_none());
    Ok(())
}

#[test]
fn the_fixture_findings_are_the_expected_ones() -> TestResult {
    let findings = indexed()?.findings()?;
    assert_eq!(
        keys(&findings),
        expected(&fixture("lenses").join("expected.toml"))?
    );
    Ok(())
}
