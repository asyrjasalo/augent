//! Safe temporary directory base so temp dirs are never created under the current working
//! directory (e.g. when TMPDIR=tmp or TMPDIR=./tmp).

use std::env;
use std::path::PathBuf;

/// Returns a directory path suitable for creating temporary directories.
/// Never returns a relative path, so temp dirs are never created under the current working
/// directory (avoids repo/tmp when TMPDIR=tmp and cwd is the repo).
pub fn temp_dir_base() -> PathBuf {
    let t = env::temp_dir();
    if t.is_absolute() {
        t
    } else {
        #[cfg(windows)]
        {
            env::var("TEMP")
                .or_else(|_| env::var("TMP"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from("/tmp")
        }
    }
}
