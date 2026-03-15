//! Build output capture and logging.
//!
//! Captures stdout/stderr from build phases and optionally writes to a
//! log file at `{build_root}/build.log`.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::XpkgResult;

use super::types::BuildPhase;

/// Writer that captures build output to a log file and/or memory.
pub struct LogWriter {
    file: Option<File>,
    path: Option<PathBuf>,
    start: Instant,
}

impl LogWriter {
    /// Create a new log writer that writes to `{build_root}/build.log`.
    pub fn new(build_root: &Path) -> XpkgResult<Self> {
        let path = build_root.join("build.log");
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                crate::error::XpkgError::BuildFailed(format!(
                    "failed to create build log {}: {e}",
                    path.display()
                ))
            })?;

        tracing::debug!(path = %path.display(), "build log created");

        Ok(Self {
            file: Some(file),
            path: Some(path),
            start: Instant::now(),
        })
    }

    /// Create a null log writer (no file output). Used in tests.
    #[allow(dead_code)] // Used by tests in other modules.
    pub fn new_null() -> Self {
        Self {
            file: None,
            path: None,
            start: Instant::now(),
        }
    }

    /// Write a phase header to the log.
    pub fn write_phase_header(&mut self, phase: BuildPhase) {
        let elapsed = self.start.elapsed().as_secs_f64();
        let header = format!(
            "\n==> [{elapsed:>8.2}s] Running {phase}()...\n{}\n",
            "=".repeat(60)
        );
        self.write_raw(&header);
    }

    /// Write a single line to the log.
    pub fn write_line(&mut self, line: &str) {
        let elapsed = self.start.elapsed().as_secs_f64();
        let stamped = format!("[{elapsed:>8.2}s] {line}\n");
        self.write_raw(&stamped);
    }

    /// Write raw bytes to the log file.
    fn write_raw(&mut self, s: &str) {
        if let Some(ref mut file) = self.file {
            let _ = file.write_all(s.as_bytes());
        }
    }

    /// Return the path to the log file, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_writer_does_not_panic() {
        let mut writer = LogWriter::new_null();
        writer.write_phase_header(BuildPhase::Build);
        writer.write_line("test output");
        assert!(writer.path().is_none());
    }

    #[test]
    fn test_log_writer_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let writer = LogWriter::new(tmp.path()).unwrap();
        assert!(writer.path().unwrap().exists());
    }

    #[test]
    fn test_log_writer_writes_content() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let mut writer = LogWriter::new(tmp.path()).unwrap();
            writer.write_phase_header(BuildPhase::Build);
            writer.write_line("hello world");
        }
        let content = std::fs::read_to_string(tmp.path().join("build.log")).unwrap();
        assert!(content.contains("Running build()"));
        assert!(content.contains("hello world"));
    }
}
