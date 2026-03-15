//! Lint report formatting.
//!
//! Produces human-readable or machine-parseable (JSON) output from lint results.

use super::rules::{LintResult, Severity};

/// Output format for lint reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Human-readable colored text.
    Human,
    /// Machine-parseable JSON.
    Json,
}

/// Format a lint result into a string.
pub fn format_report(result: &LintResult, format: ReportFormat) -> String {
    match format {
        ReportFormat::Human => format_human(result),
        ReportFormat::Json => format_json(result),
    }
}

fn format_human(result: &LintResult) -> String {
    let mut out = String::new();

    if result.diagnostics.is_empty() {
        out.push_str("  No issues found.\n");
        return out;
    }

    // Group by severity (errors first, then warnings, then info).
    let mut sorted = result.diagnostics.clone();
    sorted.sort_by(|a, b| b.severity.cmp(&a.severity));

    for diag in &sorted {
        let icon = match diag.severity {
            Severity::Error => "✗",
            Severity::Warning => "⚠",
            Severity::Info => "ℹ",
        };

        let path_str = match &diag.path {
            Some(p) => format!(" ({p})"),
            None => String::new(),
        };

        out.push_str(&format!(
            "  {icon} [{severity}] {rule}: {msg}{path}\n",
            severity = diag.severity,
            rule = diag.rule,
            msg = diag.message,
            path = path_str,
        ));
    }

    // Summary line.
    let errors = result.count(Severity::Error);
    let warnings = result.count(Severity::Warning);
    let infos = result.count(Severity::Info);
    out.push_str(&format!(
        "\n  Summary: {errors} error(s), {warnings} warning(s), {infos} info(s)\n"
    ));

    out
}

fn format_json(result: &LintResult) -> String {
    let mut entries = Vec::new();

    for diag in &result.diagnostics {
        let path = match &diag.path {
            Some(p) => format!("\"{}\"", escape_json(p)),
            None => "null".to_string(),
        };

        entries.push(format!(
            "    {{\"severity\":\"{}\",\"rule\":\"{}\",\"message\":\"{}\",\"path\":{}}}",
            diag.severity,
            escape_json(&diag.rule),
            escape_json(&diag.message),
            path,
        ));
    }

    format!(
        "{{\n  \"total\":{},\n  \"errors\":{},\n  \"warnings\":{},\n  \"diagnostics\":[\n{}\n  ]\n}}",
        result.total(),
        result.count(Severity::Error),
        result.count(Severity::Warning),
        entries.join(",\n"),
    )
}

/// Minimal JSON string escaping.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_format_empty() {
        let result = LintResult::new();
        let report = format_report(&result, ReportFormat::Human);
        assert!(report.contains("No issues found"));
    }

    #[test]
    fn test_human_format_with_diagnostics() {
        let mut result = LintResult::new();
        result.add(
            Severity::Error,
            "test-rule",
            "bad thing",
            Some("/usr/bin/x"),
        );
        result.add(Severity::Warning, "test-warn", "watch out", None);

        let report = format_report(&result, ReportFormat::Human);
        assert!(report.contains("✗"));
        assert!(report.contains("⚠"));
        assert!(report.contains("1 error(s)"));
        assert!(report.contains("1 warning(s)"));
    }

    #[test]
    fn test_json_format() {
        let mut result = LintResult::new();
        result.add(Severity::Error, "test-rule", "broke", None);

        let report = format_report(&result, ReportFormat::Json);
        assert!(report.contains("\"total\":1"));
        assert!(report.contains("\"errors\":1"));
        assert!(report.contains("\"severity\":\"error\""));
        assert!(report.contains("\"rule\":\"test-rule\""));
    }

    #[test]
    fn test_json_escape() {
        assert_eq!(escape_json("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_json("line\nnewline"), "line\\nnewline");
    }
}
