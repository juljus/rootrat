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

#[test]
fn applies_file_to_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&repo_dir.path().join(&repo_path), "repo content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::Created);
    assert_eq!(fs::read_to_string(&system_path).unwrap(), "repo content");
}

#[test]
fn overwrites_existing_file() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, "old system content");
    create_file(&repo_dir.path().join(&repo_path), "new repo content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::Updated);
    assert_eq!(
        fs::read_to_string(&system_path).unwrap(),
        "new repo content"
    );
}

#[test]
fn skips_unchanged_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&system_path, "same content");
    create_file(&repo_dir.path().join(&repo_path), "same content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::Unchanged);
}

#[test]
fn creates_parent_dirs_on_system() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("deep/nested/dir/config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&repo_dir.path().join(&repo_path), "nested content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(
        fs::read_to_string(&system_path).unwrap(),
        "nested content"
    );
}

#[test]
fn missing_repo_file_reports_error_state() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    // Don't create the repo file
    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::MissingFromRepo);
    assert!(!system_path.exists());
}

#[test]
fn applies_multiple_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();

    let mut manifest = Manifest::new();

    let path1 = system_dir.path().join("a.conf");
    let rp1 = Manifest::derive_repo_path(&path1).unwrap();
    create_file(&repo_dir.path().join(&rp1), "content a");
    manifest
        .files
        .insert(rp1, path1.to_string_lossy().to_string());

    let path2 = system_dir.path().join("b.conf");
    let rp2 = Manifest::derive_repo_path(&path2).unwrap();
    create_file(&repo_dir.path().join(&rp2), "content b");
    manifest
        .files
        .insert(rp2, path2.to_string_lossy().to_string());

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(fs::read_to_string(&path1).unwrap(), "content a");
    assert_eq!(fs::read_to_string(&path2).unwrap(), "content b");
}

#[test]
fn empty_manifest_returns_empty() {
    let repo_dir = TempDir::new().unwrap();
    let manifest = Manifest::new();

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert!(results.is_empty());
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

    let results = execute(repo_dir.path(), &manifest).unwrap();

    let system_path = system_dir.path().join("nvim");
    assert_eq!(
        fs::read_to_string(system_path.join("init.lua")).unwrap(),
        "config"
    );
    assert_eq!(
        fs::read_to_string(system_path.join("lua/plugins.lua")).unwrap(),
        "plugins"
    );
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|e| e.state == ApplyState::Created));
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

    let results = execute(repo_dir.path(), &manifest).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::Unchanged);
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

    let results = execute(repo_dir.path(), &manifest).unwrap();

    let system_path = system_dir.path().join("nvim/init.lua");
    assert_eq!(fs::read_to_string(system_path).unwrap(), "new version");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].state, ApplyState::Updated);
}

// -- Plan / apply split tests --

#[test]
fn plan_detects_deleted_directory_files() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    // Repo has init.lua, system has init.lua + old-plugin.lua
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
    // Deleted state only applies to directory tracking, not individual files
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    let system_path = system_dir.path().join("config.toml");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();

    create_file(&repo_dir.path().join(&repo_path), "content");
    create_file(&system_path, "content");

    let mut manifest = Manifest::new();
    manifest
        .files
        .insert(repo_path, system_path.to_string_lossy().to_string());

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
fn execute_skips_deletes() {
    let repo_dir = TempDir::new().unwrap();
    let system_dir = TempDir::new().unwrap();
    // System has an extra file that repo doesn't
    let manifest = setup_tracked_dir(
        repo_dir.path(),
        system_dir.path(),
        "nvim",
        &[("init.lua", "config")],
        &[("init.lua", "config"), ("local-only.lua", "keep me")],
    );

    // execute is the non-interactive path -- should NOT delete
    execute(repo_dir.path(), &manifest).unwrap();

    let local_file = system_dir.path().join("nvim/local-only.lua");
    assert!(local_file.exists());
    assert_eq!(fs::read_to_string(local_file).unwrap(), "keep me");
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

    // Should have: init.lua=Updated, new.lua=Created, old.lua=Deleted
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
