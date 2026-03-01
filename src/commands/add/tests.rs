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

// -- Directory support tests --

fn create_dir_with_files(base: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir_path = base.join(name);
    for (file_name, content) in files {
        let file_path = dir_path.join(file_name);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, content).unwrap();
    }
    dir_path
}

#[test]
fn copies_directory_to_repo() {
    let (repo_dir, source_dir) = setup();
    let dir = create_dir_with_files(
        source_dir.path(),
        "myconfig",
        &[("init.lua", "vim config"), ("lua/plugins.lua", "plugins")],
    );
    let mut manifest = Manifest::new();

    execute(&dir, repo_dir.path(), &mut manifest).unwrap();

    let repo_path = Manifest::derive_repo_path(&dir).unwrap();
    let repo_dest = repo_dir.path().join(&repo_path);
    assert_eq!(
        fs::read_to_string(repo_dest.join("init.lua")).unwrap(),
        "vim config"
    );
    assert_eq!(
        fs::read_to_string(repo_dest.join("lua/plugins.lua")).unwrap(),
        "plugins"
    );
}

#[test]
fn adds_directory_entry_to_manifest() {
    let (repo_dir, source_dir) = setup();
    let dir = create_dir_with_files(source_dir.path(), "myconfig", &[("a.txt", "a")]);
    let mut manifest = Manifest::new();

    execute(&dir, repo_dir.path(), &mut manifest).unwrap();

    assert_eq!(manifest.directories.len(), 1);
    assert!(manifest.files.is_empty());
}

#[test]
fn duplicate_directory_add_updates_content() {
    let (repo_dir, source_dir) = setup();
    let dir = create_dir_with_files(source_dir.path(), "myconfig", &[("a.txt", "v1")]);
    let mut manifest = Manifest::new();

    execute(&dir, repo_dir.path(), &mut manifest).unwrap();

    // Update a file
    fs::write(dir.join("a.txt"), "v2").unwrap();
    execute(&dir, repo_dir.path(), &mut manifest).unwrap();

    let repo_path = Manifest::derive_repo_path(&dir).unwrap();
    let copied = repo_dir.path().join(&repo_path).join("a.txt");
    assert_eq!(fs::read_to_string(copied).unwrap(), "v2");
    assert_eq!(manifest.directories.len(), 1);
}
