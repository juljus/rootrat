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
fn identical_files_returns_empty() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "same content",
        "same content",
    );

    let entries = execute(repo_dir.path(), &manifest, None).unwrap();

    assert!(entries.is_empty());
}

#[test]
fn modified_file_shows_diff() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "line one\nline two\n",
        "line one\nline changed\n",
    );

    let entries = execute(repo_dir.path(), &manifest, None).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].diff.contains("line two"));
    assert!(entries[0].diff.contains("line changed"));
}

#[test]
fn missing_system_file_shows_error_state() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&repo_dir.path().join(&repo_path), "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = execute(repo_dir.path(), &manifest, None).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].diff.contains("missing from system"));
}

#[test]
fn missing_repo_file_shows_error_state() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = execute(repo_dir.path(), &manifest, None).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].diff.contains("missing from repo"));
}

#[test]
fn filter_by_path() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();

    let mut manifest = Manifest::new();

    // File 1
    let path1 = system_dir.path().join("a.conf");
    let rp1 = Manifest::derive_repo_path(&path1).unwrap();
    create_file(&path1, "old");
    create_file(&repo_dir.path().join(&rp1), "new");
    manifest.add(&path1).unwrap();

    // File 2
    let path2 = system_dir.path().join("b.conf");
    let rp2 = Manifest::derive_repo_path(&path2).unwrap();
    create_file(&path2, "old");
    create_file(&repo_dir.path().join(&rp2), "new");
    manifest.add(&path2).unwrap();

    let system_path_str = path1.to_string_lossy().to_string();
    let entries = execute(repo_dir.path(), &manifest, Some(&system_path_str)).unwrap();

    assert_eq!(entries.len(), 1);
}
