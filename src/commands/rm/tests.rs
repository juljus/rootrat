use super::*;
use crate::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

fn create_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Set up a tracked file: create both the system file and repo copy, add to manifest.
fn setup_tracked_file(
    repo_dir: &Path,
    system_dir: &Path,
    name: &str,
    content: &str,
) -> (std::path::PathBuf, Manifest) {
    let system_path = system_dir.join(name);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, content);
    create_file(&repo_dir.join(&repo_path), content);

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    (system_path, manifest)
}

/// Set up a tracked directory: create system and repo copies, add to manifest.
fn setup_tracked_dir(
    repo_dir: &Path,
    system_dir: &Path,
    dir_name: &str,
    files: &[(&str, &str)],
) -> (std::path::PathBuf, Manifest) {
    let system_path = system_dir.join(dir_name);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    for (name, content) in files {
        create_file(&system_path.join(name), content);
        create_file(&repo_dir.join(&repo_path).join(name), content);
    }

    let mut manifest = Manifest::new();
    manifest
        .directories
        .insert(repo_path, system_path.to_string_lossy().to_string());

    (system_path, manifest)
}

#[test]
fn removes_file_from_repo() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) =
        setup_tracked_file(repo_dir.path(), system_dir.path(), "test.conf", "content");

    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();
    assert!(repo_dir.path().join(&repo_path).exists());

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(!repo_dir.path().join(&repo_path).exists());
}

#[test]
fn removes_entry_from_manifest() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) =
        setup_tracked_file(repo_dir.path(), system_dir.path(), "test.conf", "content");

    assert_eq!(manifest.files.len(), 1);

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(manifest.files.is_empty());
}

#[test]
fn leaves_system_file_untouched() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "test.conf",
        "precious content",
    );

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(system_path.exists());
    assert_eq!(fs::read_to_string(&system_path).unwrap(), "precious content");
}

#[test]
fn fails_if_not_tracked() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let file = system_dir.path().join("untracked.conf");
    create_file(&file, "content");

    let mut manifest = Manifest::new();
    let result = execute(&file, repo_dir.path(), &mut manifest);

    assert!(result.is_err());
}

#[test]
fn removes_directory_from_repo() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config"), ("lua/plugins.lua", "plugins")],
    );

    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();
    assert!(repo_dir.path().join(&repo_path).exists());

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(!repo_dir.path().join(&repo_path).exists());
}

#[test]
fn removes_directory_entry_from_manifest() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config")],
    );

    assert_eq!(manifest.directories.len(), 1);

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(manifest.directories.is_empty());
}

#[test]
fn leaves_system_directory_untouched() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let (system_path, mut manifest) = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config"), ("lua/plugins.lua", "plugins")],
    );

    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(system_path.exists());
    assert_eq!(
        fs::read_to_string(system_path.join("init.lua")).unwrap(),
        "config"
    );
    assert_eq!(
        fs::read_to_string(system_path.join("lua/plugins.lua")).unwrap(),
        "plugins"
    );
}

#[test]
fn repo_copy_already_missing_still_succeeds() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("test.conf");
    create_file(&system_path, "content");

    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();
    // Don't create repo copy -- it's already gone

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path.clone(), system_path.to_string_lossy().to_string());

    // Should still succeed (just removes manifest entry)
    execute(&system_path, repo_dir.path(), &mut manifest).unwrap();

    assert!(manifest.files.is_empty());
}
