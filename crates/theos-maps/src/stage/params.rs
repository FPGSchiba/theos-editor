//! Typed stage parameters and their parsing from a recipe's stage table.

use std::any::Any;
use std::fmt::Debug;

use serde::de::DeserializeOwned;

use super::ParamError;

/// The parsed parameters of one stage, held by the runner as a trait object.
///
/// Every `DeserializeOwned + Debug + Send + Sync + 'static` type is one, so a stage's own
/// `Params` type needs no impl. `Box<dyn StageParams>` is not one, so `&boxed` dereferences to
/// the parameters inside instead of wrapping the box.
pub trait StageParams: Any + Debug + Send + Sync {
    /// The parameters as [`Any`], for the downcast to the stage's own type.
    fn as_any(&self) -> &dyn Any;
}

impl<T: DeserializeOwned + Debug + Send + Sync + 'static> StageParams for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Parses a recipe's stage table into `P`; the `id` key names the stage and is not a parameter.
///
/// Unknown keys are found with `serde_ignored` around the TOML deserialiser (ruling L3) and
/// named with their path. The path of a value of the wrong type comes from
/// `serde_path_to_error`, because `toml`'s error keeps its key path private.
///
/// `serde_ignored` cannot see inside internally tagged or untagged enums or `#[serde(flatten)]`
/// fields, because serde buffers their content first; such types carry
/// `#[serde(deny_unknown_fields)]`.
///
/// # Errors
/// [`ParamError`] naming `stage` and the unknown or invalid parameter. Unknown keys win over
/// other errors, so a misspelt required key reads as unknown rather than missing.
pub(crate) fn parse<P: DeserializeOwned>(
    stage: &str,
    table: &toml::Table,
) -> Result<P, ParamError> {
    let mut table = table.clone();
    table.remove("id");
    let mut unknown = Vec::new();
    let parsed = serde_path_to_error::deserialize(serde_ignored::Deserializer::new(
        toml::Value::Table(table),
        &mut |path: serde_ignored::Path<'_>| unknown.push(path_text(&path)),
    ));
    if let Some((first, rest)) = unknown.split_first() {
        return Err(unknown_parameter(stage, first, rest));
    }
    parsed.map_err(|err| ParamError {
        stage: stage.to_owned(),
        parameter: err
            .path()
            .iter()
            .next()
            .is_some()
            .then(|| err.path().to_string()),
        message: err.inner().message().to_owned(),
    })
}

/// The error for the unknown keys `first` and `rest`.
fn unknown_parameter(stage: &str, first: &str, rest: &[String]) -> ParamError {
    let mut message = "unknown parameter".to_owned();
    if !rest.is_empty() {
        let others: Vec<String> = rest.iter().map(|path| format!("`{path}`")).collect();
        message.push_str("; also unknown: ");
        message.push_str(&others.join(", "));
    }
    ParamError {
        stage: stage.to_owned(),
        parameter: Some(first.to_owned()),
        message,
    }
}

/// The domain checks every stage's `validate` shares, each rule with one fixed wording, so the
/// same rule reads the same in every stage (final review, round 2).
///
/// Every float check refuses NaN and both infinities, so each wording says "finite" or names a
/// bounded interval. A stage's own rules (a relation between two parameters, a set of values)
/// keep their own words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Check<'a> {
    /// The stage whose parameters are checked.
    stage: &'a str,
}

impl<'a> Check<'a> {
    /// The checks of the parameters of `stage`.
    pub(crate) const fn of(stage: &'a str) -> Self {
        Self { stage }
    }

    /// The error of `parameter` for a rule of the stage's own, in its own words.
    #[must_use]
    pub(crate) fn refuse(self, parameter: &str, message: impl Into<String>) -> ParamError {
        ParamError::at(self.stage, parameter, message)
    }

    /// `Ok` when `holds`, else the error of `parameter` with `message`.
    fn require(self, holds: bool, parameter: &str, message: String) -> Result<(), ParamError> {
        if holds {
            Ok(())
        } else {
            Err(self.refuse(parameter, message))
        }
    }

    /// `value` is neither NaN nor an infinity: "must be finite".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn finite(self, parameter: &str, value: f64) -> Result<(), ParamError> {
        self.require(value.is_finite(), parameter, "must be finite".to_owned())
    }

    /// `value` is finite and above 0: "must be positive and finite".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn positive(self, parameter: &str, value: f64) -> Result<(), ParamError> {
        let holds = value.is_finite() && value > 0.0;
        self.require(holds, parameter, "must be positive and finite".to_owned())
    }

    /// `value` is finite and at least `min`: "must be at least `min` and finite".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn at_least(self, parameter: &str, value: f64, min: f64) -> Result<(), ParamError> {
        let holds = value.is_finite() && value >= min;
        self.require(
            holds,
            parameter,
            format!("must be at least {min} and finite"),
        )
    }

    /// `value` lies in the closed interval `[lo, hi]`: "must be in [`lo`, `hi`]".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn in_closed(
        self,
        parameter: &str,
        value: f64,
        lo: f64,
        hi: f64,
    ) -> Result<(), ParamError> {
        let holds = (lo..=hi).contains(&value);
        self.require(holds, parameter, format!("must be in [{lo}, {hi}]"))
    }

    /// `value` lies in the half-open interval `[lo, hi)`: "must be in [`lo`, `hi`)".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn in_closed_open(
        self,
        parameter: &str,
        value: f64,
        lo: f64,
        hi: f64,
    ) -> Result<(), ParamError> {
        let holds = (lo..hi).contains(&value);
        self.require(holds, parameter, format!("must be in [{lo}, {hi})"))
    }

    /// `value` lies in the half-open interval `(lo, hi]`: "must be in (`lo`, `hi`]".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn in_open_closed(
        self,
        parameter: &str,
        value: f64,
        lo: f64,
        hi: f64,
    ) -> Result<(), ParamError> {
        let holds = value > lo && value <= hi;
        self.require(holds, parameter, format!("must be in ({lo}, {hi}]"))
    }

    /// A whole number of at least `min`: "must be at least `min`".
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and `parameter`.
    pub(crate) fn count_at_least<T: Copy + PartialOrd + std::fmt::Display>(
        self,
        parameter: &str,
        value: T,
        min: T,
    ) -> Result<(), ParamError> {
        let holds = value >= min;
        self.require(holds, parameter, format!("must be at least {min}"))
    }
}

/// A `serde_ignored` path in the style of `serde_path_to_error`: `source[0].rate`.
fn path_text(path: &serde_ignored::Path<'_>) -> String {
    use serde_ignored::Path;
    match path {
        Path::Root => String::new(),
        Path::Seq { parent, index } => format!("{}[{index}]", path_text(parent)),
        Path::Map { parent, key } => match path_text(parent) {
            parent if parent.is_empty() => key.clone(),
            parent => format!("{parent}.{key}"),
        },
        Path::Some { parent }
        | Path::NewtypeStruct { parent }
        | Path::NewtypeVariant { parent } => path_text(parent),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{Check, parse};
    use crate::stage::ParamError;

    #[derive(Debug, Deserialize)]
    #[serde(default)]
    struct Params {
        rate: f64,
        inner: Vec<Inner>,
        opt: Option<Inner>,
    }

    impl Default for Params {
        fn default() -> Self {
            Self {
                rate: 2.5,
                inner: Vec::new(),
                opt: None,
            }
        }
    }

    #[derive(Debug, Deserialize)]
    struct Inner {
        x: f64,
    }

    fn table(text: &str) -> toml::Table {
        text.parse().unwrap()
    }

    fn error(stage: &str, parameter: Option<&str>, message: &str) -> ParamError {
        ParamError {
            stage: stage.into(),
            parameter: parameter.map(Into::into),
            message: message.into(),
        }
    }

    #[test]
    fn the_id_key_is_not_a_parameter() {
        let p: Params = parse("t", &table(r#"id = "t""#)).unwrap();
        assert!((p.rate - 2.5).abs() < 1e-12);
        assert!(p.inner.is_empty() && p.opt.is_none());
        let p: Params = parse("t", &table("rate = 1\ninner = [{x = 3}]\nopt = {x = 4}")).unwrap();
        assert!(
            (p.rate - 1.0).abs() < 1e-12,
            "integers are accepted as floats"
        );
        assert!((p.inner[0].x - 3.0).abs() < 1e-12);
        assert!(p.opt.is_some_and(|o| (o.x - 4.0).abs() < 1e-12));
    }

    #[test]
    fn unknown_keys_are_named_with_their_path() {
        let err = parse::<Params>("t", &table("bogus = 1")).unwrap_err();
        assert_eq!(err, error("t", Some("bogus"), "unknown parameter"));
        let err = parse::<Params>("t", &table("inner = [{x = 1, y = 2}]")).unwrap_err();
        assert_eq!(err, error("t", Some("inner[0].y"), "unknown parameter"));
        let err = parse::<Params>("t", &table("opt = {x = 1, y = 2}")).unwrap_err();
        assert_eq!(err, error("t", Some("opt.y"), "unknown parameter"));
    }

    #[test]
    fn several_unknown_keys_are_all_named() {
        let err = parse::<Params>("t", &table("a = 1\nb = 2\nc = 3")).unwrap_err();
        assert_eq!(
            err,
            error("t", Some("a"), "unknown parameter; also unknown: `b`, `c`")
        );
    }

    #[test]
    fn a_misspelt_required_key_is_reported_as_unknown() {
        let err = parse::<Inner>("t", &table("y = 1")).unwrap_err();
        assert_eq!(err, error("t", Some("y"), "unknown parameter"));
    }

    #[test]
    fn a_wrong_type_names_the_parameter() {
        let err = parse::<Params>("t", &table(r#"rate = "x""#)).unwrap_err();
        assert_eq!(
            err,
            error(
                "t",
                Some("rate"),
                r#"invalid type: string "x", expected f64"#
            )
        );
        let err = parse::<Params>("t", &table(r#"inner = [{x = 1}, {x = "a"}]"#)).unwrap_err();
        assert_eq!(
            err,
            error(
                "t",
                Some("inner[1].x"),
                r#"invalid type: string "a", expected f64"#
            )
        );
    }

    /// The message of `result`, or `"ok"`.
    fn said(result: Result<(), ParamError>) -> String {
        match result {
            Ok(()) => "ok".to_owned(),
            Err(e) => {
                assert_eq!((e.stage.as_str(), e.parameter.as_deref()), ("t", Some("p")));
                e.message
            }
        }
    }

    // Final review round 2: one wording per rule, NaN and the infinities refused by every
    // bounded rule.
    #[test]
    fn each_check_has_one_wording_and_refuses_nan_and_infinities() {
        let check = Check::of("t");
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        for value in [0.0, -2.5] {
            assert_eq!(said(check.finite("p", value)), "ok");
        }
        for value in [nan, inf, -inf] {
            assert_eq!(said(check.finite("p", value)), "must be finite");
        }
        for value in [0.0, -1.0, nan, inf] {
            assert_eq!(
                said(check.positive("p", value)),
                "must be positive and finite"
            );
        }
        assert_eq!(said(check.positive("p", 1e-300)), "ok");
        for value in [0.99, nan, inf] {
            assert_eq!(
                said(check.at_least("p", value, 1.0)),
                "must be at least 1 and finite"
            );
        }
        assert_eq!(said(check.at_least("p", 1.0, 1.0)), "ok");
        assert_eq!(said(check.at_least("p", -0.0, 0.0)), "ok");
        assert_eq!(
            said(check.at_least("p", -0.1, 0.0)),
            "must be at least 0 and finite"
        );
        for value in [-0.1, 1.1, nan, inf, -inf] {
            assert_eq!(
                said(check.in_closed("p", value, 0.0, 1.0)),
                "must be in [0, 1]"
            );
        }
        for value in [0.0, 1.0] {
            assert_eq!(said(check.in_closed("p", value, 0.0, 1.0)), "ok");
        }
        for value in [-0.1, 0.5, nan, inf] {
            assert_eq!(
                said(check.in_closed_open("p", value, 0.0, 0.5)),
                "must be in [0, 0.5)"
            );
        }
        assert_eq!(said(check.in_closed_open("p", 0.0, 0.0, 0.5)), "ok");
        for value in [0.0, 1.1, nan, inf] {
            assert_eq!(
                said(check.in_open_closed("p", value, 0.0, 1.0)),
                "must be in (0, 1]"
            );
        }
        assert_eq!(said(check.in_open_closed("p", 1.0, 0.0, 1.0)), "ok");
        assert_eq!(
            said(check.count_at_least("p", 0_u32, 1)),
            "must be at least 1"
        );
        assert_eq!(said(check.count_at_least("p", 1_u32, 1)), "ok");
    }

    #[test]
    fn a_missing_field_names_its_table() {
        let err = parse::<Params>("t", &table("inner = [{}]")).unwrap_err();
        assert_eq!(err, error("t", Some("inner[0]"), "missing field `x`"));
        let err = parse::<Inner>("t", &table("")).unwrap_err();
        assert_eq!(err, error("t", None, "missing field `x`"));
    }
}
