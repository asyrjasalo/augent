//! Progress bar display for installations

use indicatif::{ProgressBar, ProgressStyle};

/// Simple progress display for installations
pub struct ProgressDisplay {
    /// Main progress bar for bundle installation
    main_pb: ProgressBar,
}

impl ProgressDisplay {
    /// Create a new progress display with total bundle count
    pub fn new(total_bundles: u64) -> Self {
        let style = ProgressStyle::default_bar()
            .template("{prefix}: [{bar:40.cyan/blue}] {pos}/{len} ({msg})")
            .unwrap()
            .progress_chars("#>-");

        let main_pb = ProgressBar::new(total_bundles);
        main_pb.set_style(style.clone());
        main_pb.set_prefix("Installing");
        main_pb.set_message("initializing...");

        Self { main_pb }
    }

    /// Update to show current bundle being installed
    pub fn update_bundle(&self, bundle_name: String, current: usize, total: usize) {
        let msg = format!("Bundle {}/{}: {}", current, total, bundle_name);
        self.main_pb.set_message(msg);
    }

    /// Increment bundle progress
    pub fn inc_bundle(&self) {
        self.main_pb.inc(1);
    }

    /// Show file being copied
    pub fn show_file(&self, file_path: &str) {
        self.main_pb.set_prefix(file_path.to_string());
    }

    /// Finish with success message
    pub fn finish(&self, message: String) {
        self.main_pb.finish_with_message(message);
    }

    /// Abandon on error
    pub fn abandon(&self) {
        self.main_pb.abandon();
    }
}
