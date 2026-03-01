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
    name: &str,
    repo_content: &str,
    system_content: &str,
) -> Manifest {
    let system_path = system_dir.join(name);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, system_content);
    create_file(&repo_dir.join(&repo_path), repo_content);

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());
    manifest
}

fn setup_tracked_dir(
    repo_dir: &Path,
    system_dir: &Path,
    dir_name: &str,
    repo_files: &[(&str, &str)],
    system_files: &[(&str, &str)],
) -> Manifest {
    let system_path = system_dir.join(dir_name);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    for (name, content) in repo_files {
        create_file(&repo_dir.join(&repo_path).join(name), content);
    }
    for (name, content) in system_files {
        create_file(&system_path.join(name), content);
    }
    fs::create_dir_all(&system_path).unwrap();

    let mut manifest = Manifest::new();
    manifest.add(&system_path).unwrap();
    manifest
}

// -- Individual file tests --

#[test]
fn unchanged_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "same",
        "same",
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, CollectState::Unchanged);
}

#[test]
fn modified_file_detected() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "old repo version",
        "new system version",
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, CollectState::Updated);
}

#[test]
fn missing_system_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    // Only in repo, not on system
    create_file(&repo_dir.path().join(&repo_path), "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, CollectState::MissingFromSystem);
}

#[test]
fn missing_repo_file_creates() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    // Only on system, not in repo
    create_file(&system_path, "new content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, CollectState::Created);
}

#[test]
fn collect_copies_system_to_repo() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(),
        system_dir.path(),
        "config.toml",
        "old",
        "updated on system",
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    collect_entries(&entries).unwrap();

    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();
    assert_eq!(
        fs::read_to_string(repo_dir.path().join(repo_path)).unwrap(),
        "updated on system"
    );
}

#[test]
fn empty_manifest_returns_empty() {
    let repo_dir = TempDir::new().unwrap();
    let manifest = Manifest::new();

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert!(entries.is_empty());
}

// -- Directory tests --

#[test]
fn directory_all_unchanged() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let files = &[("init.lua", "config"), ("lua/plugins.lua", "plugins")];
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        files,
        files,
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.state == CollectState::Unchanged));
}

#[test]
fn directory_modified_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "old")],
        &[("init.lua", "new")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, CollectState::Updated);
}

#[test]
fn directory_new_file_on_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config")],
        &[("init.lua", "config"), ("lua/new-plugin.lua", "brand new")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    let created: Vec<_> = entries
        .iter()
        .filter(|e| e.state == CollectState::Created)
        .collect();
    assert_eq!(created.len(), 1);
    assert!(created[0].system_path.contains("new-plugin.lua"));
}

#[test]
fn directory_file_only_in_repo() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config"), ("old.lua", "removed from system")],
        &[("init.lua", "config")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    let deleted: Vec<_> = entries
        .iter()
        .filter(|e| e.state == CollectState::Deleted)
        .collect();
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].system_path.contains("old.lua"));
}

#[test]
fn collect_full_directory_flow() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "old"), ("removed.lua", "gone from system")],
        &[("init.lua", "updated"), ("brand-new.lua", "just added")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    let updated: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Updated).collect();
    let created: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Created).collect();
    let deleted: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Deleted).collect();
    assert_eq!(updated.len(), 1);
    assert_eq!(created.len(), 1);
    assert_eq!(deleted.len(), 1);

    collect_entries(&entries).unwrap();

    let repo_path = Manifest::derive_repo_path(&system_dir.path().join("nvim")).unwrap();
    let repo_base = repo_dir.path().join(&repo_path);
    assert_eq!(fs::read_to_string(repo_base.join("init.lua")).unwrap(), "updated");
    assert_eq!(fs::read_to_string(repo_base.join("brand-new.lua")).unwrap(), "just added");
    assert!(!repo_base.join("removed.lua").exists());
}
