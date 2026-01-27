//! Progress bar display for installations

use indicatif::{ProgressBar, ProgressStyle};

/// Progress display for installations
pub struct ProgressDisplay {
    /// Main progress bar for bundle installation
    bundle_pb: ProgressBar,
    /// Optional file progress bar (shown when installing files)
    file_pb: Option<ProgressBar>,
}

impl ProgressDisplay {
    /// Create a new progress display with total bundle count
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

    /// Initialize file progress bar with total file count
    pub fn init_file_progress(&mut self, total_files: u64) {
        let file_style = ProgressStyle::default_bar()
            .template("  [{bar:40.green/yellow}] {pos}/{len} files {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ");

        let file_pb = ProgressBar::new(total_files);
        file_pb.set_style(file_style);
        self.file_pb = Some(file_pb);
    }

    /// Update to show current bundle being installed
    pub fn update_bundle(&self, bundle_name: &str, current: usize, total: usize) {
        let msg = format!("({}/{}) {}", current, total, bundle_name);
        self.bundle_pb.set_message(msg);
    }

    /// Increment bundle progress
    pub fn inc_bundle(&self) {
        self.bundle_pb.inc(1);
    }

    /// Update file progress
    pub fn update_file(&self, file_path: &str) {
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

    /// Finish file progress
    pub fn finish_files(&self) {
        if let Some(ref file_pb) = self.file_pb {
            file_pb.finish();
        }
        self.bundle_pb.finish();
    }

    /// Abandon on error
    pub fn abandon(&self) {
        if let Some(ref file_pb) = self.file_pb {
            file_pb.abandon();
        }
        self.bundle_pb.abandon();
    }
}
