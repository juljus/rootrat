use super::*;
use crate::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

#[test]
fn sets_repo_path_outside_home() {
    let dir = TempDir::new().unwrap();
    let mut manifest = Manifest::new();

    execute(dir.path(), &mut manifest).unwrap();

    // Outside home dir, should store absolute path
    assert_eq!(manifest.repo.unwrap(), dir.path().to_str().unwrap());
}

#[test]
fn sets_repo_path_inside_home_with_tilde() {
    let home = dirs::home_dir().unwrap();
    let dir = home.join(".rootrat_test_init");
    fs::create_dir_all(&dir).unwrap();

    let mut manifest = Manifest::new();
    execute(&dir, &mut manifest).unwrap();

    assert_eq!(manifest.repo.unwrap(), "~/.rootrat_test_init");

    fs::remove_dir(&dir).unwrap();
}

#[test]
fn overwrites_existing_repo_path() {
    let dir = TempDir::new().unwrap();
    let mut manifest = Manifest::new();
    manifest.repo = Some("/old/path".to_string());

    execute(dir.path(), &mut manifest).unwrap();

    assert!(manifest.repo.is_some());
    assert_ne!(manifest.repo.unwrap(), "/old/path");
}

#[test]
fn preserves_existing_files() {
    let dir = TempDir::new().unwrap();
    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );

    execute(dir.path(), &mut manifest).unwrap();

    assert_eq!(manifest.files.len(), 1);
}

#[test]
fn fails_if_dir_does_not_exist() {
    let mut manifest = Manifest::new();
    let result = execute(Path::new("/nonexistent/path"), &mut manifest);
    assert!(result.is_err());
}
