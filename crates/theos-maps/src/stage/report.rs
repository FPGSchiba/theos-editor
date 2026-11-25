//! What a stage hands back: its report, the findings of its checks and its metrics.

use std::collections::BTreeMap;

/// What a run of a stage returns besides its layers: values that are not layers (for example a
/// walking time between two Chimes) and findings made while running.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StageReport {
    /// Named numbers, ordered by name.
    pub values: BTreeMap<String, f64>,
    /// Findings made during the run.
    pub findings: Vec<Finding>,
}

impl StageReport {
    /// Sets the value `name` to `value`.
    pub fn put(&mut self, name: &str, value: f64) {
        self.values.insert(name.to_owned(), value);
    }
}

/// How bad a finding is: a warning goes into the summary, an error fails the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Reported in the summary.
    Warning,
    /// Fails the run, naming the stage.
    Error,
}

/// One result of a stage's checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// How bad it is.
    pub severity: Severity,
    /// What was found.
    pub message: String,
    /// The layer it concerns, if one.
    pub layer: Option<String>,
}

impl Finding {
    /// A warning.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            layer: None,
        }
    }

    /// An error.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            layer: None,
        }
    }

    /// The same finding, about `layer`.
    #[must_use]
    pub fn on_layer(self, layer: &str) -> Self {
        Self {
            layer: Some(layer.to_owned()),
            ..self
        }
    }
}

/// The numbers a stage reports into the summary, under the names of its `metric_names`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metrics(pub BTreeMap<String, f64>);

#[cfg(test)]
mod tests {
    use super::{Finding, Metrics, Severity, StageReport};

    #[test]
    fn findings_carry_severity_message_and_layer() {
        let warning = Finding::warning("slow");
        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.message, "slow");
        assert_eq!(warning.layer, None);
        let error = Finding::error("not finite").on_layer("height");
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.layer.as_deref(), Some("height"));
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn reports_and_metrics_start_empty() {
        let report = StageReport::default();
        assert!(report.values.is_empty() && report.findings.is_empty());
        assert!(Metrics::default().0.is_empty());
    }
}
