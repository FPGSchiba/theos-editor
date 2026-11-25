//! Findings as the verdict and the counts take them.

use theos_content::{Finding, Severity};

/// A finding of `rule` on `path` with `severity` and `message`, without an id or a position.
fn finding(path: &str, rule: &str, severity: Severity, message: &str) -> Finding {
    Finding {
        rule: rule.to_owned(),
        severity,
        path: path.to_owned(),
        id: None,
        span: None,
        position: None,
        message: message.to_owned(),
    }
}

/// An error of `rule` on `path`.
pub(crate) fn error(path: &str, rule: &str, message: &str) -> Finding {
    finding(path, rule, Severity::Error, message)
}

/// A warning of `rule` on `path`.
pub(crate) fn warning(path: &str, rule: &str, message: &str) -> Finding {
    finding(path, rule, Severity::Warning, message)
}
