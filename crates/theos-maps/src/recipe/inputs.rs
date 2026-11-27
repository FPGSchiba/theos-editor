//! Declared recipe inputs (spec §5.2 item 2): what a recipe lets a request set, and the checks
//! of a request's values against the declarations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The declaration of one recipe input (`[inputs.<name>]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputDecl {
    /// A feature of the parent map, named by its id; only a child map has one.
    Feature {
        /// The types of feature the parent may be.
        parent_types: Vec<String>,
    },
    /// A number.
    Number {
        /// The value when the request gives none.
        default: Option<f64>,
        /// The smallest value.
        min: Option<f64>,
        /// The largest value.
        max: Option<f64>,
        /// Whether the value must be a whole number.
        #[serde(default)]
        integer: bool,
    },
    /// One of a list of words.
    Enum {
        /// The values.
        values: Vec<String>,
        /// The value when the request gives none.
        default: Option<String>,
    },
}

/// Why an input of a request, or the declaration of one, is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputProblem {
    /// The input's name.
    pub input: String,
    /// What is wrong.
    pub message: String,
}

impl InputProblem {
    fn new(input: &str, message: impl Into<String>) -> Self {
        Self {
            input: input.to_owned(),
            message: message.into(),
        }
    }
}

/// The inputs a recipe declares, by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Inputs(pub BTreeMap<String, InputDecl>);

impl Inputs {
    /// The one input of type `feature` and its parent types, `None` when there is none.
    #[must_use]
    pub fn feature(&self) -> Option<(&str, &[String])> {
        self.0.iter().find_map(|(name, decl)| match decl {
            InputDecl::Feature { parent_types } => Some((name.as_str(), parent_types.as_slice())),
            _ => None,
        })
    }

    /// Checks the declarations: at most one `feature` input, which names parent types; ordered
    /// number bounds; an `enum` with values and a default among them.
    ///
    /// # Errors
    /// The first [`InputProblem`] in name order.
    pub fn validate(&self) -> Result<(), InputProblem> {
        let mut feature: Option<&str> = None;
        for (name, decl) in &self.0 {
            match decl {
                InputDecl::Feature { parent_types } => {
                    if let Some(first) = feature {
                        return Err(InputProblem::new(
                            name,
                            format!("only one `feature` input is allowed, `{first}` is another"),
                        ));
                    }
                    feature = Some(name);
                    if parent_types.is_empty() {
                        return Err(InputProblem::new(
                            name,
                            "a `feature` input needs `parent_types`",
                        ));
                    }
                }
                InputDecl::Number { min, max, .. } => {
                    if let (Some(min), Some(max)) = (min, max)
                        && min > max
                    {
                        return Err(InputProblem::new(
                            name,
                            format!("`min` {min} is above `max` {max}"),
                        ));
                    }
                }
                InputDecl::Enum { values, default } => {
                    if values.is_empty() {
                        return Err(InputProblem::new(name, "an `enum` input needs `values`"));
                    }
                    if let Some(default) = default
                        && !values.contains(default)
                    {
                        return Err(InputProblem::new(
                            name,
                            format!("`default` `{default}` is not one of the values"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Checks request values: declared names only, the declared type, range, integer, enum value.
    ///
    /// # Errors
    /// The first [`InputProblem`] in name order of the request.
    pub fn check(&self, given: &Map<String, Value>) -> Result<(), InputProblem> {
        for (name, value) in given {
            let Some(decl) = self.0.get(name) else {
                return Err(InputProblem::new(
                    name,
                    format!("unknown recipe input `{name}`"),
                ));
            };
            check_value(name, decl, value)?;
        }
        Ok(())
    }

    /// The declarations as `describe` reports them, e.g.
    /// `{"feature": {"type": "feature", "parent_types": ["town"]}}`.
    #[must_use]
    pub fn describe(&self) -> Map<String, Value> {
        self.0
            .iter()
            .filter_map(|(name, decl)| {
                serde_json::to_value(decl)
                    .ok()
                    .map(|value| (name.clone(), value))
            })
            .collect()
    }
}

fn check_value(name: &str, decl: &InputDecl, value: &Value) -> Result<(), InputProblem> {
    match decl {
        InputDecl::Feature { .. } => {
            if value.is_string() {
                Ok(())
            } else {
                Err(InputProblem::new(
                    name,
                    format!("input `{name}` must be a feature id (a string)"),
                ))
            }
        }
        InputDecl::Number {
            min, max, integer, ..
        } => {
            let Some(n) = value.as_f64().filter(|n| n.is_finite()) else {
                return Err(InputProblem::new(
                    name,
                    format!("input `{name}` must be a number"),
                ));
            };
            if *integer && libm::fmod(n, 1.0).abs() > 0.0 {
                return Err(InputProblem::new(
                    name,
                    format!("input `{name}` must be a whole number, got {n}"),
                ));
            }
            if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                return Err(InputProblem::new(
                    name,
                    format!("input `{name}` is {n}, outside its range"),
                ));
            }
            Ok(())
        }
        InputDecl::Enum { values, .. } => match value.as_str() {
            Some(s) if values.iter().any(|v| v == s) => Ok(()),
            _ => Err(InputProblem::new(
                name,
                format!("input `{name}` must be one of: {}", values.join(", ")),
            )),
        },
    }
}

#[cfg(test)]
mod tests;
