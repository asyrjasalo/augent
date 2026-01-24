# Interactive Testing Guide

This guide explains how to write automated tests for CLI commands that require user interaction (menus, confirmations, etc.).

## Overview

Augent uses `dialoguer` for interactive UI elements, which reads directly from the terminal (not stdin). This means we cannot use standard stdin redirection in tests. Instead, we use **PTY (pseudo-terminal)** to simulate real user interaction.

## Infrastructure

The interactive testing infrastructure is located in `tests/common/interactive.rs`:

### `InteractiveTest` Struct

Provides methods to:

- Create a PTY-based test session
- Send keystrokes (arrow keys, space, enter, escape)
- Wait for command output

### Key Methods

```rust
use common::InteractiveTest;

// Create new interactive test
let mut test = InteractiveTest::new("augent", &["install", "./repo"], &workspace.path)?;

// Send keystrokes
test.send_space()?;      // Select/deselect
test.send_down()?;       // Navigate down
test.send_up()?;         // Navigate up
test.send_enter()?;      // Confirm
test.send_escape()?;     // Cancel

// Wait for command to complete
let output = test.wait_for_output()?;

// Check exit status
let status = test.status()?;
```

### Convenience Function

```rust
use common::run_interactive;

let output = run_interactive(
    "augent",
    &["install", "./repo"],
    &workspace.path,
    &[" ", "\n"], // Space then enter
)?;
```

## Test Patterns

### Pattern 1: Menu Selection Tests

Test that the interactive menu works correctly:

```rust
#[test]
fn test_menu_selects_all() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create bundles
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )?;

    // Wait for menu to render
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Select bundles using space
    test.send_space()?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B")?; // Down arrow
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space()?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Confirm with enter
    test.send_input("\n")?;

    let output = test.wait_for_output()?;
    assert!(output.contains("installed"));
}
```

### Pattern 2: Confirmation Prompt Tests

Test yes/no confirmations:

```rust
#[test]
fn test_confirmation_user_accepts() {
    let workspace = common::TestWorkspace::new();

    // Install bundle first
    augent_cmd()
        .args(["install", "./bundle"])
        .assert()
        .success();

    // Uninstall with confirmation
    augent_cmd()
        .args(["uninstall", "bundle"])
        .write_stdin("y\n") // Send "y" and newline
        .assert()
        .success()
        .stdout(predicate::str::contains("Are you sure"))
        .stdout(predicate::str::contains("uninstalled"));
}
```

### Pattern 3: Menu Navigation Tests

Test arrow key navigation:

```rust
#[test]
fn test_menu_navigation() {
    let mut test = InteractiveTest::new(...)?;

    // Navigate down to third item
    for _ in 0..2 {
        test.send_input("\x1b[B")?; // Down arrow
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Select and confirm
    test.send_space()?;
    test.send_input("\n")?;

    let output = test.wait_for_output()?;
}
```

### Pattern 4: Selection Toggle Tests

Test toggling selections on/off:

```rust
#[test]
fn test_selection_toggle() {
    let mut test = InteractiveTest::new(...)?;

    // Select first item
    test.send_space()?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Deselect by pressing space again
    test.send_space()?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    test.send_input("\n")?;
    let output = test.wait_for_output()?;

    // Verify first item was NOT installed
    assert!(!workspace.file_exists(".claude/item1.md"));
}
```

## Key Sequences

### Escape Sequences

| Action | Escape Sequence | Method |
|---------|-----------------|---------|
| Up | `\x1b[A` | `test.send_up()?` |
| Down | `\x1b[B` | `test.send_down()?` |
| Right | `\x1b[C` | - |
| Left | `\x1b[D` | - |
| Escape | `\x1b` | `test.send_escape()?` |

### Special Keys

| Action | Key | Method |
|---------|------|---------|
| Select/Deselect | Space | `test.send_space()?` |
| Confirm | Enter | `test.send_enter()?` |
| Cancel | Escape | `test.send_escape()?` |

## Timing Considerations

### Why Delays Are Necessary

Interactive UI elements need time to:

1. Render the menu
2. Process keystrokes
3. Update the display

**Always add delays between keystrokes:**

```rust
test.send_space()?;
std::thread::sleep(std::time::Duration::from_millis(50));
test.send_input("\x1b[B")?;
std::thread::sleep(std::time::Duration::from_millis(50));
```

### Recommended Delays

- After creating test: 200ms (let menu render)
- Between keystrokes: 20-50ms
- After final keystroke: 50ms (before confirming)

## Common Pitfalls

### 1. No Initial Delay

**Problem:** Test tries to send input before menu is ready

**Solution:** Always wait after creating test:

```rust
let mut test = InteractiveTest::new(...)?;
std::thread::sleep(std::time::Duration::from_millis(200)); // Wait for menu
```

### 2. Forgetting to Flush

**Problem:** Input not sent immediately

**Solution:** `InteractiveTest::send_input()` handles flushing automatically

### 3. Wrong Escape Sequence

**Problem:** Using literal arrow keys

**Solution:** Use ANSI escape sequences:

```rust
// Wrong
test.send_input("down")?;

// Correct
test.send_input("\x1b[B")?;
```

### 4. Testing Without Waiting

**Problem:** Test completes before output is ready

**Solution:** Always call `wait_for_output()` before assertions

```rust
let output = test.wait_for_output()?;
assert!(output.contains("expected"));
```

## Testing Strategy

### What to Test

1. **Happy Paths**: Normal user flows
   - Selecting items
   - Confirming prompts
   - Completing installation/uninstallation

2. **Error Paths**: Edge cases and error handling
   - Canceling selections
   - Rejecting confirmations
   - Empty selections

3. **UI Behavior**: Display and formatting
   - Menu shows correct items
   - Navigation instructions are clear
   - Prompts are understandable

4. **Large Lists**: Performance with many items
   - Scrolling through long lists
   - Selecting from large sets

### Test Organization

Place tests in appropriate files:

- `tests/interactive_menu_tests.rs` - Menu interaction tests
- `tests/install_interactive_tests.rs` - Install command tests
- `tests/uninstall_interactive_tests.rs` - Uninstall command tests

## Example: Complete Test

```rust
#[test]
fn test_complete_install_workflow() {
    // Setup
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo/bundle-a");
    workspace.write_file("repo/bundle-a/augent.yaml", "name: \"@test/bundle-a\"\n");
    workspace.write_file("repo/bundle-a/commands/test.md", "# Test\n");

    // Execute
    let bin_path = PathBuf::from(env!("CARGO_BIN_EXE_augent"));
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    ).expect("Failed to create test");

    std::thread::sleep(std::time::Duration::from_millis(200));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    // Verify
    let output = test.wait_for_output().expect("Failed to wait");
    assert!(output.contains("installed"));
    assert!(workspace.file_exists(".claude/commands/test.md"));
}
```

## Running Interactive Tests

Run all tests:

```bash
cargo test --test interactive_menu_tests
cargo test --test uninstall_interactive_tests
```

Run specific test:

```bash
cargo test --test interactive_menu_tests test_menu_selects_all
```

## Additional Resources

- `tests/common/interactive.rs` - Interactive test infrastructure
- `tests/common/mod.rs` - Common test utilities
- `src/commands/menu.rs` - Menu implementation
- `src/commands/uninstall.rs` - Confirmation prompt implementation
