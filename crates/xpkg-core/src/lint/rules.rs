//! Core lint types: severity, diagnostics, and results.

use std::fmt;

/// Severity level of a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational note — does not cause lint failure.
    Info,
    /// Warning — causes failure only in strict mode.
    Warning,
    /// Error — always causes lint failure.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A single lint diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Rule identifier (e.g. "permissions-world-writable").
    pub rule: String,
    /// Human-readable message.
    pub message: String,
    /// Optional file path related to the diagnostic.
    pub path: Option<String>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref path) = self.path {
            write!(
                f,
                "[{}] {}: {} ({})",
                self.severity, self.rule, self.message, path
            )
        } else {
            write!(f, "[{}] {}: {}", self.severity, self.rule, self.message)
        }
    }
}

/// Aggregated lint results.
#[derive(Debug)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl LintResult {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Add a diagnostic to the results.
    pub fn add(&mut self, severity: Severity, rule: &str, message: &str, path: Option<&str>) {
        self.diagnostics.push(Diagnostic {
            severity,
            rule: rule.to_string(),
            message: message.to_string(),
            path: path.map(String::from),
        });
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Returns true if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }

    /// Count of diagnostics by severity.
    pub fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }

    /// In strict mode, elevate all warnings to errors.
    pub fn apply_strict(&mut self) {
        for diag in &mut self.diagnostics {
            if diag.severity == Severity::Warning {
                diag.severity = Severity::Error;
            }
        }
    }

    /// Total number of diagnostics.
    pub fn total(&self) -> usize {
        self.diagnostics.len()
    }
}

impl Default for LintResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_result_empty() {
        let result = LintResult::new();
        assert!(!result.has_errors());
        assert!(!result.has_warnings());
        assert_eq!(result.total(), 0);
    }

    #[test]
    fn test_lint_result_add_and_count() {
        let mut result = LintResult::new();
        result.add(Severity::Error, "test-rule", "something broke", None);
        result.add(
            Severity::Warning,
            "test-warn",
            "careful",
            Some("/usr/bin/x"),
        );
        result.add(Severity::Info, "test-info", "note", None);

        assert!(result.has_errors());
        assert!(result.has_warnings());
        assert_eq!(result.count(Severity::Error), 1);
        assert_eq!(result.count(Severity::Warning), 1);
        assert_eq!(result.count(Severity::Info), 1);
        assert_eq!(result.total(), 3);
    }

    #[test]
    fn test_apply_strict_elevates_warnings() {
        let mut result = LintResult::new();
        result.add(Severity::Warning, "test", "warn", None);
        result.add(Severity::Info, "test2", "info", None);

        result.apply_strict();

        assert_eq!(result.count(Severity::Error), 1);
        assert_eq!(result.count(Severity::Warning), 0);
        assert_eq!(result.count(Severity::Info), 1);
    }
}
