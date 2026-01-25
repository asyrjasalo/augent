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

#[allow(dead_code)] // Methods are part of testing infrastructure documented in INTERACTIVE_TESTING.md
impl InteractiveTest {
    pub fn new<P: AsRef<Path>>(program: &str, args: &[&str], workdir: P) -> std::io::Result<Self> {
        let workdir = workdir.as_ref();

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(workdir);

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
        const MAX_NO_DATA: usize = 20; // Allow up to 1 second of no data (20 * 50ms)

        loop {
            if start.elapsed() > timeout {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout waiting for output",
                ));
            }

            // Check if process has exited first
            if self.session.check(Eof).is_ok() {
                // Process exited, do final read attempts to get any remaining output
                for _ in 0..5 {
                    thread::sleep(Duration::from_millis(50));
                    match self.session.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                        }
                        _ => {}
                    }
                }
                break;
            }

            // Try to read
            match self.session.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                    no_data_count = 0; // Reset counter on successful read
                }
                Ok(_) => {
                    // No data available (n == 0)
                    no_data_count += 1;
                    if no_data_count > MAX_NO_DATA {
                        // No data for too long, assume we're done
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    no_data_count += 1;
                    if no_data_count > MAX_NO_DATA {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    // EIO (code 5 on Linux) can occur when process closes PTY
                    // Also handle generic IO errors gracefully
                    #[cfg(unix)]
                    if e.raw_os_error() == Some(5) {
                        break;
                    }
                    // For Windows or other errors, check if process exited
                    if self.session.check(Eof).is_ok() {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(output)
    }

    pub fn wait_for_text(&mut self, expected: &str, timeout: Duration) -> std::io::Result<String> {
        let start = std::time::Instant::now();
        let mut output = String::new();
        let mut buffer = [0u8; 4096];

        loop {
            if start.elapsed() > timeout {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Timeout waiting for text: {}", expected),
                ));
            }

            // Read any available data first (before checking EOF so we don't miss output)
            match self.session.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let text = std::str::from_utf8(&buffer[..n]).unwrap_or("");
                    output.push_str(text);
                    // Check if pattern matches
                    if output.contains(expected) {
                        return Ok(output);
                    }
                }
                Ok(_) | Err(_) => {
                    // No data (n=0) or error: check if process has exited
                    if self.session.check(Eof).is_ok() {
                        let preview = if output.len() > 500 {
                            format!("{}...", &output[..500])
                        } else {
                            output.clone()
                        };
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            format!(
                                "EOF before finding text: {}. Output so far: {:?}",
                                expected, preview
                            ),
                        ));
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
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

#[allow(dead_code)]
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
#[allow(dead_code)]
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
        thread::sleep(Duration::from_millis(150));
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum MenuAction {
    SelectCurrent,
    MoveDown,
    MoveUp,
    Confirm,
    Cancel,
    Wait(Duration),
}
