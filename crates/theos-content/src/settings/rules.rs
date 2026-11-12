//! The `settings/*` rules (spec §5.12): each reports the problems of its own id that
//! [`parse`](super::parse) found in `.theos/settings.toml`. They belong to no part, so they are
//! a list of their own that the lore step runs next to the registered rules.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, TextIndex};

use super::SETTINGS;

/// Defines a rule `$rule` with the function `$run` that reports the problems of its id.
macro_rules! settings_rule {
    ($rule:ident, $run:ident, $id:literal, $severity:expr) => {
        const $rule: LoreRule = LoreRule {
            id: $id,
            severity: $severity,
            class: RuleClass::Fixed,
            on: On::Input(SETTINGS),
            run: $run,
        };

        fn $run(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
            report(ctx, subject, &$rule)
        }
    };
}

settings_rule!(TOML, toml, "settings/toml", Severity::Error);
settings_rule!(
    UNKNOWN_KEY,
    unknown_key,
    "settings/unknown-key",
    Severity::Error
);
settings_rule!(GUARDRAIL, guardrail, "settings/guardrail", Severity::Error);
settings_rule!(FIXED, fixed, "settings/fixed", Severity::Error);
settings_rule!(
    UNKNOWN_RULE,
    unknown_rule,
    "settings/unknown-rule",
    Severity::Warning
);
settings_rule!(SEVERITY, severity, "settings/severity", Severity::Error);
settings_rule!(
    ALLOW_REASON,
    allow_reason,
    "settings/allow-reason",
    Severity::Error
);
settings_rule!(WORD, word, "settings/word", Severity::Error);

/// Every settings rule.
pub(crate) const RULES: &[LoreRule] = &[
    TOML,
    UNKNOWN_KEY,
    GUARDRAIL,
    FIXED,
    UNKNOWN_RULE,
    SEVERITY,
    ALLOW_REASON,
    WORD,
];

/// The findings of `rule` on the settings file `subject`: one per problem with its id.
fn report(
    ctx: &mut Context<'_>,
    subject: &Subject,
    rule: &LoreRule,
) -> Result<Vec<Finding>, FactError> {
    let Subject::Input(input) = subject else {
        return Ok(Vec::new());
    };
    let parsed = ctx.settings()?;
    let lines = TextIndex::new(input.text.clone().unwrap_or_default());
    Ok(parsed
        .problems
        .iter()
        .filter(|problem| problem.rule == rule.id)
        .map(|problem| {
            let position = lines
                .position(problem.span.start)
                .unwrap_or(Position { line: 1, column: 1 });
            subject.finding(rule, problem.span, position, problem.message.clone())
        })
        .collect())
}
