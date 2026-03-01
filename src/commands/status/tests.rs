use super::*;
use crate::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

/// Helper: create a file with given content, creating parent dirs as needed.
fn create_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Helper: set up a repo dir, system dir, and manifest with one file tracked.
/// Returns (repo_dir, system_file_path, manifest).
fn setup_tracked_file(
    repo_dir: &Path,
    system_dir: &Path,
    filename: &str,
    repo_content: &str,
    system_content: &str,
) -> Manifest {
    let system_path = system_dir.join(filename);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, system_content);
    create_file(&repo_dir.join(&repo_path), repo_content);

    let mut manifest = Manifest::new();
    manifest.add(&system_path).unwrap();
    manifest
}

#[test]
fn unchanged_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "same content",
        "same content",
    );

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, FileState::Unchanged);
}

#[test]
fn modified_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "repo version",
        "system version",
    );

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, FileState::Modified);
}

#[test]
fn missing_from_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    // Only create in repo, not on system
    create_file(&repo_dir.path().join(&repo_path), "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, FileState::MissingFromSystem);
}

#[test]
fn missing_from_repo() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    // Only create on system, not in repo
    create_file(&system_path, "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, FileState::MissingFromRepo);
}

#[test]
fn multiple_files_mixed_states() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();

    let mut manifest = Manifest::new();

    // File 1: unchanged
    let path1 = system_dir.path().join("unchanged.conf");
    let rp1 = Manifest::derive_repo_path(&path1).unwrap();
    create_file(&path1, "same");
    create_file(&repo_dir.path().join(&rp1), "same");
    manifest.add(&path1).unwrap();

    // File 2: modified
    let path2 = system_dir.path().join("modified.conf");
    let rp2 = Manifest::derive_repo_path(&path2).unwrap();
    create_file(&path2, "new");
    create_file(&repo_dir.path().join(&rp2), "old");
    manifest.add(&path2).unwrap();

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 2);

    let unchanged = entries.iter().find(|e| e.state == FileState::Unchanged);
    let modified = entries.iter().find(|e| e.state == FileState::Modified);
    assert!(unchanged.is_some());
    assert!(modified.is_some());
}

#[test]
fn empty_manifest_returns_empty() {
    let repo_dir = TempDir::new().unwrap();
    let manifest = Manifest::new();

    let entries = execute(repo_dir.path(), &manifest).unwrap();

    assert!(entries.is_empty());
}
