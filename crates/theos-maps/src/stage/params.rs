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

    use super::parse;
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

    #[test]
    fn a_missing_field_names_its_table() {
        let err = parse::<Params>("t", &table("inner = [{}]")).unwrap_err();
        assert_eq!(err, error("t", Some("inner[0]"), "missing field `x`"));
        let err = parse::<Inner>("t", &table("")).unwrap_err();
        assert_eq!(err, error("t", None, "missing field `x`"));
    }
}
