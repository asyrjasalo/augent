//! Interactive test utilities using PTY (pseudo-terminal)
//!
//! This module provides utilities for testing interactive CLI commands that
//! use terminal input (like dialoguer's MultiSelect), which cannot be
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

use pty_process::blocking;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
#[allow(clippy::io_other_error)]
const TEST_PTY_ROWS: u16 = 24;
#[allow(dead_code)]
const TEST_PTY_COLS: u16 = 80;

pub struct InteractiveTest {
    pty: pty_process::blocking::Pty,
    child: std::process::Child,
}

#[allow(dead_code)] // Methods are part of testing infrastructure documented in INTERACTIVE_TESTING.md
impl InteractiveTest {
    pub fn new<P: AsRef<Path>>(program: &str, args: &[&str], workdir: P) -> std::io::Result<Self> {
        let workdir = workdir.as_ref();

        let (pty, pts) = blocking::open().map_err(|e| std::io::Error::other(format!("{}", e)))?;
        pty.resize(pty_process::Size::new(TEST_PTY_ROWS, TEST_PTY_COLS))
            .map_err(|e| std::io::Error::other(format!("{}", e)))?;

        let child = blocking::Command::new(program)
            .args(args)
            .current_dir(workdir)
            .spawn(pts)
            .map_err(|e| std::io::Error::other(format!("{}", e)))?;

        Ok(Self { pty, child })
    }

    pub fn send_input(&mut self, input: &str) -> std::io::Result<()> {
        self.pty.write_all(input.as_bytes())?;
        self.pty.flush()?;
        Ok(())
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

            match self.pty.read(&mut buffer) {
                Ok(0) => {
                    // EOF - process has closed the PTY
                    break;
                }
                Ok(n) => {
                    output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                    no_data_count = 0; // Reset counter on successful read
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Check if process has exited
                    if let Ok(Some(_status)) = self.child.try_wait() {
                        // Process exited, do multiple final read attempts to ensure we get all output
                        for _ in 0..5 {
                            thread::sleep(Duration::from_millis(50));
                            match self.pty.read(&mut buffer) {
                                Ok(n) if n > 0 => {
                                    output
                                        .push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                                }
                                _ => {}
                            }
                        }
                        break;
                    }

                    no_data_count += 1;
                    if no_data_count > MAX_NO_DATA {
                        // No data for too long, assume we're done
                        break;
                    }

                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    // EIO (code 5 on Linux) can occur when process closes PTY
                    // Treat this as EOF condition, not an error
                    #[cfg(unix)]
                    if e.raw_os_error() == Some(5) {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        let _ = self.child.wait();

        Ok(output)
    }

    pub fn wait_for_text(&mut self, expected: &str, timeout: Duration) -> std::io::Result<String> {
        let mut accumulated = String::new();
        let mut buffer = [0u8; 4096];
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Timeout waiting for text: '{}'", expected),
                ));
            }

            match self.pty.read(&mut buffer) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("EOF before finding text: '{}'", expected),
                    ));
                }
                Ok(n) => {
                    let chunk = std::str::from_utf8(&buffer[..n]).unwrap_or("");
                    accumulated.push_str(chunk);

                    if accumulated.contains(expected) {
                        return Ok(accumulated);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    // EIO (code 5 on Linux) can occur when process closes PTY
                    // Treat this as EOF condition, not an error
                    #[cfg(unix)]
                    if e.raw_os_error() == Some(5) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            format!("EOF before finding text: '{}'", expected),
                        ));
                    }
                    return Err(e);
                }
            }
        }
    }

    pub fn status(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait()
    }
}

impl Drop for InteractiveTest {
    fn drop(&mut self) {
        if let Ok(status) = self.child.try_wait() {
            if status.is_some() {
                return;
            }
        }
        let _ = self.child.kill();
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
        thread::sleep(Duration::from_millis(100));
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
