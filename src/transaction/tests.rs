//! Tests for transaction support

use super::*;
use tempfile::TempDir;

fn create_test_workspace() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
    git2::Repository::init(temp.path()).unwrap();
    let workspace_root = temp.path().to_path_buf();
    let augent_dir = workspace_root.join(".augent");
    fs::create_dir_all(&augent_dir).unwrap();

    // Create initial config files with valid bundle name (must contain '/')
    fs::write(augent_dir.join("augent.yaml"), "name: \"@test/workspace\"").unwrap();
    fs::write(
        augent_dir.join("augent.lock"),
        r#"{"name":"@test/workspace","bundles":[]}"#,
    )
    .unwrap();

    (temp, workspace_root, augent_dir)
}

#[test]
fn test_transaction_backup_configs() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs().unwrap();

    assert_eq!(transaction.config_backups.len(), 2);
}

#[test]
fn test_transaction_commit() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs().unwrap();

    // Create a file
    let test_file = workspace_root.join("test.txt");
    fs::write(&test_file, "test content").unwrap();
    transaction.track_file_created(&test_file);

    // Commit
    transaction.commit();

    // File should still exist
    assert!(test_file.exists());
}

#[test]
fn test_transaction_rollback_created_files() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

    {
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs().unwrap();

        // Create a file
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        transaction.track_file_created(&test_file);

        // Don't commit - should rollback on drop
    }

    // File should be removed
    let test_file = workspace_root.join("test.txt");
    assert!(!test_file.exists());
}

#[test]
fn test_transaction_rollback_configs() {
    let (_temp, workspace_root, augent_dir) = create_test_workspace();
    let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

    let yaml_path = augent_dir.join("augent.yaml");
    let original_content = fs::read_to_string(&yaml_path).unwrap();

    {
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs().unwrap();

        // Modify config
        fs::write(&yaml_path, "name: modified").unwrap();

        // Don't commit - should rollback on drop
    }

    // Config should be restored
    let restored_content = fs::read_to_string(&yaml_path).unwrap();
    assert_eq!(restored_content, original_content);
}

#[test]
fn test_transaction_track_dir_created() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

    {
        let mut transaction = Transaction::new(&workspace);

        // Create a directory
        let test_dir = workspace_root.join("new_dir");
        fs::create_dir(&test_dir).unwrap();
        transaction.track_dir_created(&test_dir);

        // Don't commit - should rollback on drop
    }

    // Directory should be removed
    let test_dir = workspace_root.join("new_dir");
    assert!(!test_dir.exists());
}
