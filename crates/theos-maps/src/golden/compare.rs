//! Comparing a run with its golden record: the first difference in pipeline order (spec §5.14).

use std::collections::BTreeSet;

use super::{GoldenRun, GoldenStage};

/// The first difference between a golden run and a fresh one, in pipeline order: the run's
/// identity, then each stage's key and its layers by name, then `features`, then `package`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GoldenMismatch {
    /// The two records belong to different runs.
    #[error(
        "the golden run is {expected_recipe} seed {expected_seed}, \
         the fresh one {actual_recipe} seed {actual_seed}"
    )]
    Run {
        /// The golden run's recipe.
        expected_recipe: String,
        /// The golden run's seed.
        expected_seed: u64,
        /// The fresh run's recipe.
        actual_recipe: String,
        /// The fresh run's seed.
        actual_seed: u64,
    },
    /// The pipeline differs at a position: another stage, or a stage only one run has.
    #[error(
        "pipeline position {position}: expected stage {}, got stage {}",
        shown(.expected.as_deref()),
        shown(.actual.as_deref())
    )]
    Stage {
        /// The 0-based position in the pipeline.
        position: usize,
        /// The golden run's stage key there, if it has one.
        expected: Option<String>,
        /// The fresh run's stage key there, if it has one.
        actual: Option<String>,
    },
    /// A layer a stage wrote has another hash, or only one run wrote it.
    #[error(
        "stage `{stage}`, layer `{layer}`: expected {}, got {}",
        shown(.expected.as_deref()),
        shown(.actual.as_deref())
    )]
    Layer {
        /// The stage key, `<id>#<occurrence>`.
        stage: String,
        /// The layer name.
        layer: String,
        /// The golden hash, if the golden run wrote the layer.
        expected: Option<String>,
        /// The fresh hash, if the fresh run wrote the layer.
        actual: Option<String>,
    },
    /// The `features` output or the package differs.
    #[error("output `{output}`: expected {expected}, got {actual}")]
    Output {
        /// `features` or `package`.
        output: &'static str,
        /// The golden hash.
        expected: String,
        /// The fresh hash.
        actual: String,
    },
}

/// `value` in backticks, or `nothing`.
fn shown(value: Option<&str>) -> String {
    value.map_or_else(|| "nothing".to_owned(), |v| format!("`{v}`"))
}

/// Compares the fresh run `actual` with the golden run `expected`.
///
/// # Errors
/// The first [`GoldenMismatch`] in pipeline order.
pub fn compare(expected: &GoldenRun, actual: &GoldenRun) -> Result<(), GoldenMismatch> {
    if (&expected.recipe, expected.seed) != (&actual.recipe, actual.seed) {
        return Err(GoldenMismatch::Run {
            expected_recipe: expected.recipe.clone(),
            expected_seed: expected.seed,
            actual_recipe: actual.recipe.clone(),
            actual_seed: actual.seed,
        });
    }
    let positions = expected.stages.len().max(actual.stages.len());
    for position in 0..positions {
        let golden = expected.stages.get(position);
        let fresh = actual.stages.get(position);
        match (golden, fresh) {
            (Some(golden), Some(fresh)) if golden.key == fresh.key => {
                compare_layers(golden, fresh)?;
            }
            _ => {
                return Err(GoldenMismatch::Stage {
                    position,
                    expected: golden.map(|s| s.key.clone()),
                    actual: fresh.map(|s| s.key.clone()),
                });
            }
        }
    }
    for (output, golden, fresh) in [
        ("features", &expected.features, &actual.features),
        ("package", &expected.package, &actual.package),
    ] {
        if golden != fresh {
            return Err(GoldenMismatch::Output {
                output,
                expected: golden.clone(),
                actual: fresh.clone(),
            });
        }
    }
    Ok(())
}

/// The first layer, by name, whose hash differs between two records of the same stage.
fn compare_layers(expected: &GoldenStage, actual: &GoldenStage) -> Result<(), GoldenMismatch> {
    let names: BTreeSet<&String> = expected.layers.keys().chain(actual.layers.keys()).collect();
    for name in names {
        let golden = expected.layers.get(name);
        let fresh = actual.layers.get(name);
        if golden != fresh {
            return Err(GoldenMismatch::Layer {
                stage: expected.key.clone(),
                layer: name.clone(),
                expected: golden.cloned(),
                actual: fresh.cloned(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
