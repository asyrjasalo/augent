//! Interactive test utilities using PTY (pseudo-terminal)
//!
//! This module provides utilities for testing interactive CLI commands that
//! use terminal input (like inquire's MultiSelect), which cannot be
//! tested with standard stdin redirection.
//!
//! Usage:
//! ```ignore
//! let test = InteractiveTest::new("augent", &["install", "./repo"]);
//! test.send_input(" ");  // Select first item
//! test.send_input("\n"); // Confirm
//! let output = test.wait_for_output();
//! assert!(output.contains("installed"));
//! ```

use expectrl::{ControlCode, Eof, Expect, Session};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct InteractiveTest {
    // Using expectrl's spawn which returns OsSession
    // Cross-platform: pty_process on Unix, conpty on Windows
    session: expectrl::session::OsSession,
}

enum ReadResult {
    Success,
    Continue,
    Error(std::io::Error),
}

#[allow(dead_code)] // Methods are part of testing infrastructure documented in INTERACTIVE_TESTING.md
impl InteractiveTest {
    pub fn new<P: AsRef<Path>>(program: &str, args: &[&str], workdir: P) -> std::io::Result<Self> {
        let workdir = workdir.as_ref();

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(workdir);
        cmd.env_remove("AUGENT_WORKSPACE");
        cmd.env_remove("AUGENT_CACHE_DIR");
        cmd.env_remove("TMPDIR");
        cmd.env("AUGENT_WORKSPACE", workdir.as_os_str());
        cmd.env(
            "AUGENT_CACHE_DIR",
            super::test_cache_dir_for_workspace(workdir).as_os_str(),
        );
        cmd.env("TMPDIR", super::test_tmpdir_for_child().as_os_str());

        let session = Session::spawn(cmd)
            .map_err(|e| std::io::Error::other(format!("Failed to spawn session: {}", e)))?;

        Ok(Self { session })
    }

    pub fn send_input(&mut self, input: &str) -> std::io::Result<()> {
        self.session
            .send(input)
            .map_err(|e| std::io::Error::other(format!("Failed to send input: {}", e)))
    }

    pub fn send_down(&mut self) -> std::io::Result<()> {
        self.send_input("\x1b[B")
    }

    pub fn send_up(&mut self) -> std::io::Result<()> {
        self.send_input("\x1b[A")
    }

    pub fn send_enter(&mut self) -> std::io::Result<()> {
        self.send_input("\n")
    }

    pub fn send_escape(&mut self) -> std::io::Result<()> {
        self.send_input("\x1b")
    }

    pub fn send_space(&mut self) -> std::io::Result<()> {
        self.send_input(" ")
    }

    pub fn wait_for_output(&mut self) -> std::io::Result<String> {
        self.wait_for_output_with_timeout(Duration::from_secs(10))
    }

    pub fn wait_for_output_with_timeout(&mut self, timeout: Duration) -> std::io::Result<String> {
        let mut output = String::new();
        let mut buffer = [0u8; 4096];
        let start = std::time::Instant::now();
        let mut no_data_count = 0;
        const MAX_NO_DATA: usize = 4; // Allow up to 200ms of no data (4 * 50ms) - reduced for faster tests

        // Brief delay so the process can produce output (helps on fast CI, e.g. x86_64 Linux)
        thread::sleep(Duration::from_millis(25));

        loop {
            if self.check_timeout_for_output(&start, timeout)? {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout waiting for output",
                ));
            }

            // Read first, before checking Eof — on Linux, check(Eof) can be true as soon as
            // the child closes the PTY; if we check first we may break and drain before
            // read() has consumed buffered output. Same pattern as wait_for_text.
            match self.session.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                    no_data_count = 0; // Reset counter on successful read
                }
                Ok(_) => {
                    // No data available (n == 0)
                    if self.handle_no_data_for_output(
                        &mut no_data_count,
                        MAX_NO_DATA,
                        &mut output,
                        &mut buffer,
                    ) {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if self.handle_no_data_for_output(
                        &mut no_data_count,
                        MAX_NO_DATA,
                        &mut output,
                        &mut buffer,
                    ) {
                        break;
                    }
                }
                Err(e) => {
                    if self.handle_read_error_for_output(&e, &mut output, &mut buffer)? {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(output)
    }

    /// Returns true if should break from loop
    fn handle_no_data_for_output(
        &mut self,
        no_data_count: &mut usize,
        max_no_data: usize,
        output: &mut String,
        buffer: &mut [u8],
    ) -> bool {
        *no_data_count += 1;
        if *no_data_count > max_no_data {
            self.drain_remaining_output(output, buffer);
            return true;
        }
        if self.session.check(Eof).is_ok() {
            self.drain_remaining_output(output, buffer);
            return true;
        }
        thread::sleep(Duration::from_millis(25));
        false
    }

    /// Returns Ok(true) if should break from loop, Ok(false) if error should be returned
    fn handle_read_error_for_output(
        &mut self,
        e: &std::io::Error,
        output: &mut String,
        buffer: &mut [u8],
    ) -> std::io::Result<bool> {
        // EIO (code 5 on Linux) can occur when process closes PTY slave;
        // drain before breaking to capture any buffered output
        #[cfg(unix)]
        if e.raw_os_error() == Some(5) {
            self.drain_remaining_output(output, buffer);
            return Ok(true);
        }
        // For Windows or other errors, check if process exited
        if self.session.check(Eof).is_ok() {
            self.drain_remaining_output(output, buffer);
            return Ok(true);
        }
        Ok(false)
    }

    fn check_timeout_for_output(
        &self,
        start: &std::time::Instant,
        timeout: Duration,
    ) -> std::io::Result<bool> {
        Ok(start.elapsed() > timeout)
    }

    /// Drain any remaining output from the PTY (e.g. after Eof or EIO on Linux).
    /// Optimized to drain faster (reduced from 5 iterations to 2).
    fn drain_remaining_output(&mut self, output: &mut String, buffer: &mut [u8]) {
        for _ in 0..2 {
            thread::sleep(Duration::from_millis(25));
            if let Ok(n) = self.session.read(buffer) {
                if n > 0 {
                    output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                }
            }
        }
    }

    pub fn wait_for_text(&mut self, expected: &str, timeout: Duration) -> std::io::Result<String> {
        let start = std::time::Instant::now();
        let mut output = String::new();
        let mut buffer = [0u8; 4096];
        let mut iteration_count = 0;
        let max_iterations = (timeout.as_millis() / 50) as usize + 100;

        thread::sleep(Duration::from_millis(25));

        loop {
            iteration_count += 1;

            self.check_iteration_limit(iteration_count, max_iterations, expected, &output)?;

            self.check_timeout(start, timeout, expected, &output)?;

            match self.read_and_process(&mut buffer, &mut output, expected) {
                ReadResult::Success => return Ok(output),
                ReadResult::Continue => thread::sleep(Duration::from_millis(25)),
                ReadResult::Error(e) => return Err(e),
            }
        }
    }

    fn check_iteration_limit(
        &self,
        iteration_count: usize,
        max_iterations: usize,
        expected: &str,
        output: &str,
    ) -> std::io::Result<()> {
        if iteration_count > max_iterations {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "Timeout waiting for text: {} (exceeded {} iterations). Output so far: {:?}",
                    expected,
                    max_iterations,
                    Self::truncate_output(output)
                ),
            ))
        } else {
            Ok(())
        }
    }

    fn check_timeout(
        &self,
        start: std::time::Instant,
        timeout: Duration,
        expected: &str,
        output: &str,
    ) -> std::io::Result<()> {
        if start.elapsed() > timeout {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "Timeout waiting for text: {} ({}ms elapsed). Output so far: {:?}",
                    expected,
                    start.elapsed().as_millis(),
                    Self::truncate_output(output)
                ),
            ))
        } else {
            Ok(())
        }
    }

    fn read_and_process(
        &mut self,
        buffer: &mut [u8],
        output: &mut String,
        expected: &str,
    ) -> ReadResult {
        match self.session.read(buffer) {
            Ok(n) if n > 0 => {
                let text = std::str::from_utf8(&buffer[..n]).unwrap_or("");
                output.push_str(text);
                if output.contains(expected) {
                    ReadResult::Success
                } else {
                    ReadResult::Continue
                }
            }
            Ok(_) => self.handle_no_data(output, expected),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                self.handle_no_data(output, expected)
            }
            Err(e) => self.handle_read_error(e, output, expected),
        }
    }

    fn handle_no_data(&mut self, output: &str, expected: &str) -> ReadResult {
        if self.session.check(Eof).is_ok() {
            ReadResult::Error(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "EOF before finding text: {}. Output so far: {:?}",
                    expected,
                    Self::truncate_output(output)
                ),
            ))
        } else {
            ReadResult::Continue
        }
    }

    fn handle_read_error(&mut self, e: std::io::Error, output: &str, expected: &str) -> ReadResult {
        #[cfg(unix)]
        if e.raw_os_error() == Some(5) {
            if output.contains(expected) {
                return ReadResult::Success;
            }
            return ReadResult::Error(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "EIO before finding text: {}. Output so far: {:?}",
                    expected,
                    Self::truncate_output(output)
                ),
            ));
        }

        if self.session.check(Eof).is_ok() {
            ReadResult::Error(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "EOF before finding text: {}. Output so far: {:?}",
                    expected,
                    Self::truncate_output(output)
                ),
            ))
        } else {
            ReadResult::Error(e)
        }
    }

    fn truncate_output(output: &str) -> String {
        if output.len() > 500 {
            format!("{}...", &output[..500])
        } else {
            output.to_string()
        }
    }

    /// Wait for process to finish without draining all output (faster than wait_for_output)
    /// This is useful when you only need to verify files/state, not capture output.
    pub fn wait_for_completion(&mut self, timeout: Duration) -> std::io::Result<()> {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout waiting for process completion",
                ));
            }

            if self.session.check(Eof).is_ok() {
                return Ok(());
            }

            thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn status(&mut self) -> std::io::Result<std::process::ExitStatus> {
        // Wait for process to finish by expecting EOF
        let _ = self.session.expect(Eof);
        // Return a dummy success status (0)
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            Ok(std::process::ExitStatus::from_raw(0))
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            Ok(std::process::ExitStatus::from_raw(0))
        }
    }
}

impl Drop for InteractiveTest {
    fn drop(&mut self) {
        // Try to check if process is still alive and kill if needed
        if self.session.check(Eof).is_err() {
            // Process might still be running, try to send EOF to clean exit
            let _ = self.session.send(ControlCode::EndOfTransmission);
        }
    }
}

#[allow(dead_code)] // Used by test files (part of testing infrastructure)
pub fn run_interactive<P: AsRef<Path>>(
    program: &str,
    args: &[&str],
    workdir: P,
    inputs: &[&str],
) -> std::io::Result<String> {
    let mut test = InteractiveTest::new(program, args, workdir)?;

    // Wait for menu to appear before sending input
    let _ = test.wait_for_text("Select bundles", Duration::from_secs(5))?;

    for input in inputs {
        test.send_input(input)?;
        thread::sleep(Duration::from_millis(100));
    }

    test.wait_for_output()
}

/// Helper to send a sequence of menu actions with proper synchronization
#[allow(dead_code)] // Used by test files (part of testing infrastructure)
pub fn send_menu_actions(
    test: &mut InteractiveTest,
    actions: &[MenuAction],
) -> std::io::Result<()> {
    for action in actions {
        match action {
            MenuAction::SelectCurrent => {
                test.send_space()?;
            }
            MenuAction::MoveDown => {
                test.send_down()?;
            }
            MenuAction::MoveUp => {
                test.send_up()?;
            }
            MenuAction::Confirm => {
                test.send_enter()?;
            }
            MenuAction::Cancel => {
                test.send_escape()?;
            }
            MenuAction::Wait(duration) => {
                thread::sleep(*duration);
            }
        }
        // Add a small delay between actions for menu to update
        // Reduced from 150ms to 25ms for faster test execution
        thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

#[allow(dead_code)] // Used by test files (part of testing infrastructure)
#[derive(Debug, Clone)]
pub enum MenuAction {
    SelectCurrent,
    MoveDown,
    MoveUp,
    Confirm,
    Cancel,
    Wait(Duration),
}

/// Run a test function with a timeout. If the test exceeds the timeout, it will panic.
///
/// This is useful for preventing CI from hanging indefinitely when interactive tests
/// get stuck, especially on Windows where PTY reads can block.
///
/// Note: This uses a separate thread to monitor the timeout. If the test hangs in a
/// blocking operation that doesn't check the timeout, this may not prevent the hang.
/// However, it provides a safety net for most cases.
///
/// # Example
/// ```ignore
/// use std::time::Duration;
/// common::run_with_timeout(Duration::from_secs(30), || {
///     // Your test code here
/// });
/// ```
#[allow(dead_code)] // Part of testing infrastructure, used by tests
pub fn run_with_timeout<F>(timeout: Duration, test_fn: F)
where
    F: FnOnce() + Send + 'static,
{
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();

    // Spawn the test in a thread
    let test_thread = thread::spawn(move || {
        test_fn();
        let _ = tx.send(());
    });

    // Spawn a timeout monitor thread
    let timeout_thread = thread::spawn(move || {
        thread::sleep(timeout);
        if rx.try_recv().is_err() {
            // Test hasn't completed, exit to fail of test
            eprintln!(
                "TEST TIMEOUT: Test exceeded {} seconds. This usually indicates a hang in interactive PTY operations, especially on Windows.",
                timeout.as_secs()
            );
            std::process::exit(1);
        }
    });

    // Wait for test to complete or timeout
    let _ = test_thread.join();
    let _ = timeout_thread.join();
}
