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
    repo_content: Option<&str>,
    system_content: Option<&str>,
) -> Manifest {
    let system_path = system_dir.join(name);
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    if let Some(content) = repo_content {
        create_file(&repo_dir.join(&repo_path), content);
    }
    if let Some(content) = system_content {
        create_file(&system_path, content);
    }

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());
    manifest
}

// -- Individual file tests --

#[test]
fn applies_file_to_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "config.toml",
        Some("repo content"), None,
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ApplyState::Created);

    apply_entries(&entries).unwrap();
    assert_eq!(fs::read_to_string(system_dir.path().join("config.toml")).unwrap(), "repo content");
}

#[test]
fn overwrites_existing_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "config.toml",
        Some("new repo content"), Some("old system content"),
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert_eq!(entries[0].state, ApplyState::Updated);

    apply_entries(&entries).unwrap();
    assert_eq!(fs::read_to_string(system_dir.path().join("config.toml")).unwrap(), "new repo content");
}

#[test]
fn skips_unchanged_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "config.toml",
        Some("same content"), Some("same content"),
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ApplyState::Unchanged);
}

#[test]
fn creates_parent_dirs_on_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "deep/nested/dir/config.toml",
        Some("nested content"), None,
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    apply_entries(&entries).unwrap();

    assert_eq!(
        fs::read_to_string(system_dir.path().join("deep/nested/dir/config.toml")).unwrap(),
        "nested content"
    );
}

#[test]
fn missing_repo_file_reports_error_state() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "config.toml",
        None, None,
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ApplyState::MissingFromRepo);
}

#[test]
fn applies_multiple_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();

    let path1 = system_dir.path().join("a.conf");
    let path2 = system_dir.path().join("b.conf");
    let rp1 = Manifest::derive_repo_path(&path1).unwrap();
    let rp2 = Manifest::derive_repo_path(&path2).unwrap();

    create_file(&repo_dir.path().join(&rp1), "content a");
    create_file(&repo_dir.path().join(&rp2), "content b");

    let mut manifest = Manifest::new();
    manifest.files.insert(rp1, path1.to_string_lossy().to_string());
    manifest.files.insert(rp2, path2.to_string_lossy().to_string());

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    apply_entries(&entries).unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(fs::read_to_string(&path1).unwrap(), "content a");
    assert_eq!(fs::read_to_string(&path2).unwrap(), "content b");
}

#[test]
fn empty_manifest_returns_empty() {
    let repo_dir = TempDir::new().unwrap();
    let manifest = Manifest::new();

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert!(entries.is_empty());
}

// -- Directory support tests --

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

#[test]
fn applies_directory_to_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config"), ("lua/plugins.lua", "plugins")],
        &[],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    apply_entries(&entries).unwrap();

    let system_path = system_dir.path().join("nvim");
    assert_eq!(fs::read_to_string(system_path.join("init.lua")).unwrap(), "config");
    assert_eq!(fs::read_to_string(system_path.join("lua/plugins.lua")).unwrap(), "plugins");
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.state == ApplyState::Created));
}

#[test]
fn directory_skips_unchanged_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let files = &[("init.lua", "same")];
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        files,
        files,
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ApplyState::Unchanged);
}

#[test]
fn directory_updates_modified_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "new version")],
        &[("init.lua", "old version")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    apply_entries(&entries).unwrap();

    let system_path = system_dir.path().join("nvim/init.lua");
    assert_eq!(fs::read_to_string(system_path).unwrap(), "new version");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ApplyState::Updated);
}

// -- Plan / apply split tests --

#[test]
fn plan_detects_deleted_directory_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config")],
        &[("init.lua", "config"), ("old-plugin.lua", "removed")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    let deleted: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Deleted).collect();
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].system_path.contains("old-plugin.lua"));
}

#[test]
fn plan_does_not_delete_individual_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_file(
        repo_dir.path(), system_dir.path(), "config.toml",
        Some("content"), Some("content"),
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();
    assert!(entries.iter().all(|e| e.state != ApplyState::Deleted));
}

#[test]
fn apply_entries_deletes_files() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("to-delete.txt");
    create_file(&file_path, "goodbye");

    let entries = vec![ApplyEntry {
        system_path: file_path.to_string_lossy().to_string(),
        state: ApplyState::Deleted,
        repo_file: tmp.path().join("nonexistent"),
        system_file: file_path.clone(),
    }];

    apply_entries(&entries).unwrap();
    assert!(!file_path.exists());
}

#[test]
fn apply_entries_creates_files() {
    let tmp = TempDir::new().unwrap();
    let repo_file = tmp.path().join("source.txt");
    let system_file = tmp.path().join("dest.txt");
    create_file(&repo_file, "new content");

    let entries = vec![ApplyEntry {
        system_path: "test".to_string(),
        state: ApplyState::Created,
        repo_file: repo_file.clone(),
        system_file: system_file.clone(),
    }];

    apply_entries(&entries).unwrap();
    assert_eq!(fs::read_to_string(&system_file).unwrap(), "new content");
}

#[test]
fn plan_and_apply_full_directory_flow() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "updated"), ("new.lua", "brand new")],
        &[("init.lua", "original"), ("old.lua", "to remove")],
    );

    let entries = plan(repo_dir.path(), &manifest).unwrap();

    let updated: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Updated).collect();
    let created: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Created).collect();
    let deleted: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Deleted).collect();
    assert_eq!(updated.len(), 1);
    assert_eq!(created.len(), 1);
    assert_eq!(deleted.len(), 1);

    apply_entries(&entries).unwrap();

    let nvim = system_dir.path().join("nvim");
    assert_eq!(fs::read_to_string(nvim.join("init.lua")).unwrap(), "updated");
    assert_eq!(fs::read_to_string(nvim.join("new.lua")).unwrap(), "brand new");
    assert!(!nvim.join("old.lua").exists());
}
