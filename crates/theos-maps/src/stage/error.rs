//! The errors of the Stage contract: a stage breaking its declaration, and bad parameters.

use std::fmt;

use crate::world::{LayerKind, WorldError};

/// How a stage used a layer it had not declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Read (or taken) without a declared read.
    Read,
    /// Written (or taken) without a declared write.
    Write,
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
        })
    }
}

/// Why a stage's run failed. Every variant but `Cancelled` names the stage, and every
/// contract breach names the layer (spec §5.3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StageError {
    /// The stage used a layer it did not declare.
    #[error("stage `{stage}` did not declare layer `{layer}` as a {access}")]
    Undeclared {
        /// The stage (`<parent>/<process>` in a child context).
        stage: String,
        /// The layer.
        layer: String,
        /// The use that was not declared.
        access: Access,
    },
    /// A declared layer the stage needs is not in the world.
    #[error("stage `{stage}` needs layer `{layer}`, which the world does not hold")]
    Missing {
        /// The stage.
        stage: String,
        /// The layer.
        layer: String,
    },
    /// A layer's kind differs from the declared one.
    #[error("stage `{stage}`: layer `{layer}` must be {expected}, found {found}")]
    WrongKind {
        /// The stage.
        stage: String,
        /// The layer.
        layer: String,
        /// The declared kind.
        expected: LayerKind,
        /// The kind written, asked for, or held by the world.
        found: LayerKind,
    },
    /// The stage ended without writing a declared, non-optional layer.
    #[error("stage `{stage}` ended without writing its declared layer `{layer}`")]
    NotWritten {
        /// The stage.
        stage: String,
        /// The layer.
        layer: String,
    },
    /// The run's cancel token was set.
    #[error("the run was cancelled")]
    Cancelled,
    /// The stage could not do its work.
    #[error("stage `{stage}` failed: {message}")]
    Failed {
        /// The stage.
        stage: String,
        /// What went wrong.
        message: String,
    },
    /// The world refused a written layer; the source names the layer.
    #[error("stage `{stage}` wrote a layer the world refused: {source}")]
    World {
        /// The stage.
        stage: String,
        /// The world's reason.
        source: WorldError,
    },
}

/// Parameters of a stage that a recipe got wrong.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ParamError {
    /// The stage id.
    pub stage: String,
    /// The parameter's path in the stage table (`rate`, `source[0].rate`); `None` when the
    /// problem concerns the whole table.
    pub parameter: Option<String>,
    /// What is wrong.
    pub message: String,
}

impl ParamError {
    /// The error of `stage` about its `parameter` (a path such as `source[0].r`).
    #[must_use]
    pub fn at(stage: &str, parameter: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.to_owned(),
            parameter: Some(parameter.into()),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stage `{}`", self.stage)?;
        if let Some(parameter) = &self.parameter {
            write!(f, ", parameter `{parameter}`")?;
        }
        write!(f, ": {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, ParamError, StageError};
    use crate::world::{LayerKind, WorldError};

    fn text(err: &StageError) -> String {
        err.to_string()
    }

    #[test]
    fn stage_errors_name_the_stage_and_the_layer() {
        let undeclared = StageError::Undeclared {
            stage: "hydrology".into(),
            layer: "kd".into(),
            access: Access::Read,
        };
        assert_eq!(
            text(&undeclared),
            "stage `hydrology` did not declare layer `kd` as a read"
        );
        let undeclared = StageError::Undeclared {
            stage: "hydrology".into(),
            layer: "lakes".into(),
            access: Access::Write,
        };
        assert_eq!(
            text(&undeclared),
            "stage `hydrology` did not declare layer `lakes` as a write"
        );
        let missing = StageError::Missing {
            stage: "hydrology".into(),
            layer: "height".into(),
        };
        assert_eq!(
            text(&missing),
            "stage `hydrology` needs layer `height`, which the world does not hold"
        );
        let wrong = StageError::WrongKind {
            stage: "hydrology".into(),
            layer: "lakes".into(),
            expected: LayerKind::Mask,
            found: LayerKind::Value("climate"),
        };
        assert_eq!(
            text(&wrong),
            "stage `hydrology`: layer `lakes` must be a mask, found a `climate` value"
        );
        let not_written = StageError::NotWritten {
            stage: "hydrology".into(),
            layer: "discharge".into(),
        };
        assert_eq!(
            text(&not_written),
            "stage `hydrology` ended without writing its declared layer `discharge`"
        );
        assert_eq!(text(&StageError::Cancelled), "the run was cancelled");
        let world = StageError::World {
            stage: "hydrology".into(),
            source: WorldError::Shape {
                name: "lakes".into(),
                expected: (2, 2),
                actual: (3, 3),
            },
        };
        let world_text = text(&world);
        assert!(
            world_text.starts_with("stage `hydrology` wrote a layer the world refused: ")
                && world_text.contains("lakes"),
            "{world_text}"
        );
    }

    #[test]
    fn param_errors_name_the_stage_and_the_parameter() {
        let at = ParamError {
            stage: "tectonics".into(),
            parameter: Some("source[0].rate".into()),
            message: "invalid type".into(),
        };
        assert_eq!(
            at.to_string(),
            "stage `tectonics`, parameter `source[0].rate`: invalid type"
        );
        let whole = ParamError {
            stage: "tectonics".into(),
            parameter: None,
            message: "no sources".into(),
        };
        assert_eq!(whole.to_string(), "stage `tectonics`: no sources");
    }
}
