//! UI/Progress presentation layer
//!
//! This module handles:
//! - Progress reporting for installations and other long-running operations
//! - Interactive progress bars using indicatif
//! - Silent progress for dry-run mode
//!
//! All progress reporting goes through the ProgressReporter trait, allowing
//! different implementations based on command-line flags (e.g., --quiet, --verbose).

use indicatif::{ProgressBar, ProgressStyle};

/// Progress reporter trait for long-running operations
///
/// This trait allows different progress reporting strategies:
/// - Interactive progress bars (default)
/// - Silent/no-op progress for dry-run or quiet mode
/// - Verbose logging with detailed output
pub trait ProgressReporter: Send + Sync {
    /// Initialize file progress with total file count
    #[allow(dead_code)]
    fn init_file_progress(&mut self, total_files: u64);

    /// Update to show current bundle being installed
    #[allow(dead_code)]
    fn update_bundle(&mut self, bundle_name: &str, current: usize, total: usize);

    /// Increment bundle progress
    #[allow(dead_code)]
    fn inc_bundle(&mut self);

    /// Update file progress
    #[allow(dead_code)]
    fn update_file(&mut self, file_path: &str);

    /// Finish file progress
    fn finish_files(&mut self);

    /// Abandon on error
    fn abandon(&mut self);
}

/// Interactive progress reporter with visual progress bars
///
/// Uses indicatif ProgressBar for visual progress display during installations.
pub struct InteractiveProgressReporter {
    /// Main progress bar for bundle installation
    bundle_pb: ProgressBar,
    /// Optional file progress bar (shown when installing files)
    file_pb: Option<ProgressBar>,
}

impl InteractiveProgressReporter {
    /// Create a new interactive progress reporter with total bundle count
    pub fn new(total_bundles: u64) -> Self {
        let bundle_style = ProgressStyle::default_bar()
            .template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-");

        let bundle_pb = ProgressBar::new(total_bundles);
        bundle_pb.set_style(bundle_style);

        Self {
            bundle_pb,
            file_pb: None,
        }
    }
}

impl ProgressReporter for InteractiveProgressReporter {
    fn init_file_progress(&mut self, total_files: u64) {
        let file_style = ProgressStyle::default_bar()
            .template("  [{bar:40.green/yellow}] {pos}/{len} files {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ");

        let file_pb = ProgressBar::new(total_files);
        file_pb.set_style(file_style);
        self.file_pb = Some(file_pb);
    }

    fn update_bundle(&mut self, bundle_name: &str, current: usize, total: usize) {
        let msg = format!("({}/{}) {}", current, total, bundle_name);
        self.bundle_pb.set_message(msg);
    }

    fn inc_bundle(&mut self) {
        self.bundle_pb.inc(1);
    }

    fn update_file(&mut self, file_path: &str) {
        if let Some(ref file_pb) = self.file_pb {
            // Truncate long paths for display
            let display_path = if file_path.len() > 50 {
                format!("...{}", &file_path[file_path.len() - 47..])
            } else {
                file_path.to_string()
            };
            file_pb.set_message(display_path);
            file_pb.inc(1);
        }
    }

    fn finish_files(&mut self) {
        if let Some(ref file_pb) = self.file_pb {
            file_pb.finish();
        }
        self.bundle_pb.finish();
    }

    fn abandon(&mut self) {
        if let Some(ref file_pb) = self.file_pb {
            file_pb.abandon();
        }
        self.bundle_pb.abandon();
    }
}

/// Silent progress reporter for dry-run mode
///
/// No-op implementation that does not display anything.
/// Used when --dry-run flag is specified or in quiet mode.
#[allow(dead_code)]
#[derive(Default)]
pub struct SilentProgressReporter;

impl ProgressReporter for SilentProgressReporter {
    fn init_file_progress(&mut self, _total_files: u64) {
        // No-op for silent mode
    }

    fn update_bundle(&mut self, _bundle_name: &str, _current: usize, _total: usize) {
        // No-op for silent mode
    }

    fn inc_bundle(&mut self) {
        // No-op for silent mode
    }

    fn update_file(&mut self, _file_path: &str) {
        // No-op for silent mode
    }

    fn finish_files(&mut self) {
        // No-op for silent mode
    }

    fn abandon(&mut self) {
        // No-op for silent mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silent_progress_reporter_no_ops() {
        let mut reporter = SilentProgressReporter;

        // All methods should do nothing and not panic
        reporter.init_file_progress(10);
        reporter.update_bundle("test-bundle", 1, 5);
        reporter.inc_bundle();
        reporter.update_file("test/file.md");
        reporter.finish_files();
        reporter.abandon();
    }

    #[test]
    fn test_interactive_progress_reporter_creation() {
        let reporter = InteractiveProgressReporter::new(5);
        assert!(reporter.file_pb.is_none());
    }

    #[test]
    fn test_interactive_progress_reporter_file_init() {
        let mut reporter = InteractiveProgressReporter::new(5);
        reporter.init_file_progress(10);
        assert!(reporter.file_pb.is_some());
    }

    #[test]
    fn test_interactive_progress_reporter_bundle_update() {
        let mut reporter = InteractiveProgressReporter::new(5);
        reporter.update_bundle("test-bundle", 2, 5);
        // Should not panic - just verifying it compiles
    }

    #[test]
    fn test_interactive_progress_reporter_inc() {
        let mut reporter = InteractiveProgressReporter::new(5);
        reporter.inc_bundle();
        reporter.inc_bundle();
        assert_eq!(reporter.bundle_pb.position(), 2);
    }
}
