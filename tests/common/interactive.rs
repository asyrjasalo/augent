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

const TEST_PTY_ROWS: u16 = 24;
const TEST_PTY_COLS: u16 = 80;

pub struct InteractiveTest {
    pty: pty_process::blocking::Pty,
    child: std::process::Child,
}

impl InteractiveTest {
    pub fn new<P: AsRef<Path>>(program: &str, args: &[&str], workdir: P) -> std::io::Result<Self> {
        let workdir = workdir.as_ref();

        let (pty, pts) = blocking::open()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))?;
        pty.resize(pty_process::Size::new(TEST_PTY_ROWS, TEST_PTY_COLS))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))?;

        let child = blocking::Command::new(program)
            .args(args)
            .current_dir(workdir)
            .spawn(pts)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))?;

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
        thread::sleep(Duration::from_millis(200));

        let mut output = String::new();
        let mut buffer = [0u8; 4096];

        loop {
            match self.pty.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    output.push_str(std::str::from_utf8(&buffer[..n]).unwrap_or(""));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if let Some(status) = self.child.try_wait().unwrap() {
                        if status.success() {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e),
            }
        }

        let _ = self.child.wait();

        Ok(output)
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

pub fn run_interactive<P: AsRef<Path>>(
    program: &str,
    args: &[&str],
    workdir: P,
    inputs: &[&str],
) -> std::io::Result<String> {
    let mut test = InteractiveTest::new(program, args, workdir)?;

    for input in inputs {
        test.send_input(input)?;
        thread::sleep(Duration::from_millis(50));
    }

    test.wait_for_output()
}
