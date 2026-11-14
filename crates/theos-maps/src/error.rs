//! The error of a run: everything that can stop it, from the plan to the package.

use std::path::PathBuf;

use crate::recipe::RecipeError;
use crate::stage::StageError;

/// Why a run stopped.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The recipe was refused before any stage ran.
    #[error(transparent)]
    Recipe(#[from] RecipeError),
    /// The estimate of the layers' memory is above the budget; nothing ran.
    #[error(
        "the run needs about {needed} bytes but the budget is {budget} bytes; largest layers: {largest}"
    )]
    OverBudget {
        /// The estimate in bytes.
        needed: u64,
        /// The budget in bytes.
        budget: u64,
        /// The largest layers with their sizes, as text.
        largest: String,
    },
    /// The cancel token was set.
    #[error("the run was cancelled")]
    Cancelled,
    /// A stage returned an error.
    #[error("{source}")]
    StageFailed {
        /// The stage id.
        stage: String,
        /// What the stage said.
        source: StageError,
    },
    /// A stage panicked.
    #[error("stage `{stage}` panicked: {message}")]
    StagePanicked {
        /// The stage id.
        stage: String,
        /// The panic message.
        message: String,
    },
    /// The checks of a stage found an error.
    #[error("stage `{stage}`: {message}")]
    FindingFailed {
        /// The stage id.
        stage: String,
        /// The finding.
        message: String,
    },
    /// A stage wrote a NaN or an infinity into a float layer.
    #[error("stage `{stage}` wrote a non-finite value into layer `{layer}` at cell {cell}")]
    NonFinite {
        /// The stage id.
        stage: String,
        /// The layer.
        layer: String,
        /// The flat index of the first offending cell.
        cell: usize,
    },
    /// The output folder already holds a file with the name of one of the run's outputs (or of
    /// its temporary `.part` file); the run wrote nothing (plan ruling L10).
    #[error("{} already exists", path.display())]
    OutputExists {
        /// The file.
        path: PathBuf,
    },
    /// A file or folder could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The path.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The map package format refused something.
    #[error(transparent)]
    Format(#[from] theos_map_format::FormatError),
    /// A render failed.
    #[error("render `{render}`: {message}")]
    Render {
        /// The render id.
        render: String,
        /// What went wrong.
        message: String,
    },
    /// The `features` or `package` output could not be built from the finished world.
    #[error("output `{role}`: {message}")]
    Output {
        /// The output's role.
        role: String,
        /// What went wrong.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_what_failed() {
        let over = RunError::OverBudget {
            needed: 9,
            budget: 3,
            largest: "x".to_owned(),
        };
        assert!(over.to_string().contains("9 bytes") && over.to_string().contains("budget is 3"));
        let nan = RunError::NonFinite {
            stage: "s".to_owned(),
            layer: "h".to_owned(),
            cell: 7,
        };
        assert!(nan.to_string().contains("`s`") && nan.to_string().contains("cell 7"));
        let exists = RunError::OutputExists {
            path: PathBuf::from("out"),
        };
        assert_eq!(exists.to_string(), "out already exists");
        let io = RunError::Io {
            path: PathBuf::from("p"),
            source: std::io::Error::other("bad"),
        };
        assert_eq!(io.to_string(), "p: bad");
        let render = RunError::Render {
            render: "base".to_owned(),
            message: "no".to_owned(),
        };
        assert_eq!(render.to_string(), "render `base`: no");
        let output = RunError::Output {
            role: "features".to_owned(),
            message: "no".to_owned(),
        };
        assert_eq!(output.to_string(), "output `features`: no");
        assert_eq!(RunError::Cancelled.to_string(), "the run was cancelled");
        let panicked = RunError::StagePanicked {
            stage: "s".to_owned(),
            message: "m".to_owned(),
        };
        assert_eq!(panicked.to_string(), "stage `s` panicked: m");
        let finding = RunError::FindingFailed {
            stage: "s".to_owned(),
            message: "m".to_owned(),
        };
        assert_eq!(finding.to_string(), "stage `s`: m");
        let from: RunError = RecipeError::UnknownRecipe("r".to_owned()).into();
        assert!(from.to_string().contains("unknown recipe"));
    }
}
