//! `world/floor-number`: a floor's `number` lies inside the `floors` range of its band.

use crate::finding::{Finding, Severity};
use crate::index::Slot;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule: a floor's `number` is outside the `floors` of its band.
pub(crate) const FLOOR_NUMBER: LoreRule = LoreRule {
    id: "world/floor-number",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["floor"]),
    run: check,
};

/// Skips a floor whose `number`, parent link or the band's `floors` is invalid or does not
/// resolve to a band: the finding on that fault exists already.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(number) = record
        .text("number")
        .and_then(|value| value.text.as_deref()?.parse::<u32>().ok())
    else {
        return Ok(Vec::new());
    };
    let Some((span, position)) = record.value_at("number") else {
        return Ok(Vec::new());
    };
    let parent = Slot::Field("parent".to_owned());
    let Some(link) = record.links_in(&parent).next() else {
        return Ok(Vec::new());
    };
    let Some(band) = ctx
        .records(&link.target)?
        .into_iter()
        .find(|found| found.kind.as_deref() == Some("band"))
    else {
        return Ok(Vec::new());
    };
    let bounds: Vec<u32> = band
        .items("floors")
        .iter()
        .filter_map(|item| item.text.as_deref()?.parse().ok())
        .collect();
    let [first, last] = bounds[..] else {
        return Ok(Vec::new());
    };
    if (first..=last).contains(&number) {
        return Ok(Vec::new());
    }
    Ok(vec![subject.finding(
        &FLOOR_NUMBER,
        span,
        position,
        format!(
            "floor number {number} lies outside the floors {first} to {last} of the band `{}`",
            band.id
        ),
    )])
}
