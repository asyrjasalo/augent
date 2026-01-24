mod common;

use pty_process::blocking::{CommandExt, open_pty};

#[test]
fn test_install_with_menu_selects_all_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.path.join("bundles/bundle-a");
    std::fs::create_dir_all(&bundle_a).expect("Failed to create bundle-a");
    std::fs::write(
        bundle_a.join("augent.yaml"),
        "name: \"@test/bundle-a\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# Bundle A\n")
        .expect("Failed to write command");

    let bundle_b = workspace.path.join("bundles/bundle-b");
    std::fs::create_dir_all(&bundle_b).expect("Failed to create bundle-b");
    std::fs::write(
        bundle_b.join("augent.yaml"),
        "name: \"@test/bundle-b\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle_b.join("commands")).unwrap();
    std::fs::write(bundle_b.join("commands").join("b.md"), "# Bundle B\n")
        .expect("Failed to write command");

    let augent_bin =
        std::env::var("CARGO_BIN_EXE_augent").unwrap_or_else(|_| "target/debug/augent".to_string());

    let mut cmd = std::process::Command::new(&augent_bin);
    cmd.arg("install").arg("./bundles");
    cmd.current_dir(&workspace.path);

    let (mut pty, pts) = open_pty(None, None).expect("Failed to open PTY");

    pty.resize(pty_process::Size::new(24, 80))
        .expect("Failed to resize PTY");

    let _child = cmd.spawn(&mut pty).expect("Failed to spawn augent process");

    let mut reader = pty.clone().into_reader();

    let mut menu_prompt_found = false;

    let timeout = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        let mut buf = [0u8; 1024];
        match reader.read(&mut buf) {
            Ok(n) if n > 0 => {
                let text = String::from_utf8_lossy(&buf[..n]);

                if text.contains("Select bundles") {
                    menu_prompt_found = true;

                    std::thread::sleep(std::time::Duration::from_millis(100));

                    pts.write_all(b"\x1b[A").expect("Failed to write move up");
                    pts.write_all(b" ").expect("Failed to write space");
                    pts.write_all(b"\x1b[B").expect("Failed to write move down");
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    pts.write_all(b" ").expect("Failed to write space");
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    pts.write_all(b" ").expect("Failed to write space");
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    pts.write_all(b"\n").expect("Failed to write enter");
                    break;
                }
            }
            Ok(_) => {
                continue;
            }
            Err(e) => {
                break;
            }
        }
    }

    let _ = _child.wait().expect("Failed to wait for child");

    assert!(menu_prompt_found, "Menu prompt should have appeared");

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/commands/b.md"));

    let list_output = std::process::Command::new(&augent_bin)
        .arg("list")
        .current_dir(&workspace.path)
        .output()
        .expect("Failed to run list");

    let list_str = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_str.contains("@test/bundle-a"));
    assert!(list_str.contains("@test/bundle-b"));
}
