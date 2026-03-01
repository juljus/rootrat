use super::*;
use crate::manifest::Manifest;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup() -> (TempDir, TempDir) {
    let repo_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();
    (repo_dir, source_dir)
}

fn create_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let file_path = dir.join(name);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&file_path, content).unwrap();
    file_path
}

#[test]
fn copies_file_to_repo() {
    let (repo_dir, source_dir) = setup();
    let file = create_file(source_dir.path(), "test.conf", "some config");
    let mut manifest = Manifest::new();

    execute(&file, repo_dir.path(), &mut manifest).unwrap();

    let repo_path = Manifest::derive_repo_path(&file).unwrap();
    let copied = repo_dir.path().join(&repo_path);
    assert!(copied.exists());
    assert_eq!(fs::read_to_string(copied).unwrap(), "some config");
}

#[test]
fn adds_entry_to_manifest() {
    let (repo_dir, source_dir) = setup();
    let file = create_file(source_dir.path(), "test.conf", "content");
    let mut manifest = Manifest::new();

    execute(&file, repo_dir.path(), &mut manifest).unwrap();

    assert_eq!(manifest.files.len(), 1);
}

#[test]
fn creates_parent_dirs_in_repo() {
    let (repo_dir, source_dir) = setup();
    let file = create_file(source_dir.path(), "deep/nested/config.toml", "nested");
    let mut manifest = Manifest::new();

    execute(&file, repo_dir.path(), &mut manifest).unwrap();

    let repo_path = Manifest::derive_repo_path(&file).unwrap();
    let copied = repo_dir.path().join(&repo_path);
    assert!(copied.exists());
    assert_eq!(fs::read_to_string(copied).unwrap(), "nested");
}

#[test]
fn fails_if_file_does_not_exist() {
    let (repo_dir, _source_dir) = setup();
    let fake_path = Path::new("/tmp/nonexistent_rootrat_test_file");
    let mut manifest = Manifest::new();

    let result = execute(fake_path, repo_dir.path(), &mut manifest);

    assert!(result.is_err());
    assert!(manifest.files.is_empty());
}

#[test]
fn duplicate_add_updates_file_content() {
    let (repo_dir, source_dir) = setup();
    let file = create_file(source_dir.path(), "test.conf", "version 1");
    let mut manifest = Manifest::new();

    execute(&file, repo_dir.path(), &mut manifest).unwrap();

    // Update the source file
    fs::write(&file, "version 2").unwrap();
    execute(&file, repo_dir.path(), &mut manifest).unwrap();

    let repo_path = Manifest::derive_repo_path(&file).unwrap();
    let copied = repo_dir.path().join(&repo_path);
    assert_eq!(fs::read_to_string(copied).unwrap(), "version 2");
    assert_eq!(manifest.files.len(), 1);
}
