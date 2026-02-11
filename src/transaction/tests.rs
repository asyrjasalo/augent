//! Tests for transaction support

use super::*;
use tempfile::TempDir;

fn create_test_workspace() -> (TempDir, PathBuf, PathBuf) {
    let temp =
        TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
    git2::Repository::init(temp.path()).expect("Failed to init git repository");
    let workspace_root = temp.path().to_path_buf();
    let augent_dir = workspace_root.join(".augent");
    fs::create_dir_all(&augent_dir).expect("Failed to create .augent directory");

    // Create initial config files with valid bundle name (must contain '/')
    fs::write(augent_dir.join("augent.yaml"), "name: \"@test/workspace\"")
        .expect("Failed to write augent.yaml");
    fs::write(
        augent_dir.join("augent.lock"),
        r#"{"name":"@test/workspace","bundles":[]}"#,
    )
    .expect("Failed to write augent.lock");

    (temp, workspace_root, augent_dir)
}

#[test]
fn test_transaction_backup_configs() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace =
        crate::workspace::Workspace::open(&workspace_root).expect("Failed to open workspace");

    let mut transaction = Transaction::new(&workspace);
    transaction
        .backup_configs()
        .expect("Failed to backup configs");

    assert_eq!(transaction.config_backups.len(), 2);
}

#[test]
fn test_transaction_commit() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace =
        crate::workspace::Workspace::open(&workspace_root).expect("Failed to open workspace");

    let mut transaction = Transaction::new(&workspace);
    transaction
        .backup_configs()
        .expect("Failed to backup configs");

    // Create a file
    let test_file = workspace_root.join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    transaction.track_file_created(&test_file);

    // Commit
    transaction.commit();

    // File should still exist
    assert!(test_file.exists());
}

#[test]
fn test_transaction_rollback_created_files() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace =
        crate::workspace::Workspace::open(&workspace_root).expect("Failed to open workspace");

    {
        let mut transaction = Transaction::new(&workspace);
        transaction
            .backup_configs()
            .expect("Failed to backup configs");

        // Create a file
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");
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
    let workspace =
        crate::workspace::Workspace::open(&workspace_root).expect("Failed to open workspace");

    let yaml_path = augent_dir.join("augent.yaml");
    let original_content = fs::read_to_string(&yaml_path).expect("Failed to read config file");

    {
        let mut transaction = Transaction::new(&workspace);
        transaction
            .backup_configs()
            .expect("Failed to backup configs");

        // Modify config
        fs::write(&yaml_path, "name: modified").expect("Failed to write modified config");

        // Don't commit - should rollback on drop
    }

    // Config should be restored
    let restored_content = fs::read_to_string(&yaml_path).expect("Failed to read restored config");
    assert_eq!(restored_content, original_content);
}

#[test]
fn test_transaction_track_dir_created() {
    let (_temp, workspace_root, _augent_dir) = create_test_workspace();
    let workspace =
        crate::workspace::Workspace::open(&workspace_root).expect("Failed to open workspace");

    {
        let mut transaction = Transaction::new(&workspace);

        // Create a directory
        let test_dir = workspace_root.join("new_dir");
        fs::create_dir(&test_dir).expect("Failed to create test directory");
        transaction.track_dir_created(&test_dir);

        // Don't commit - should rollback on drop
    }

    // Directory should be removed
    let test_dir = workspace_root.join("new_dir");
    assert!(!test_dir.exists());
}
