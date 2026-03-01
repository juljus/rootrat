use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn new_manifest_has_empty_files_and_no_repo() {
    let manifest = Manifest::new();
    assert!(manifest.files.is_empty());
    assert!(manifest.repo.is_none());
}

#[test]
fn manifest_with_repo() {
    let mut manifest = Manifest::new();
    manifest.repo = Some("/Users/juljus/dotfiles".to_string());
    assert_eq!(manifest.repo.unwrap(), "/Users/juljus/dotfiles");
}

#[test]
fn derive_repo_path_from_home_dir() {
    let home = dirs::home_dir().unwrap();
    let system_path = home.join(".claude/CLAUDE.md");
    let repo_path = Manifest::derive_repo_path(&system_path).unwrap();
    assert_eq!(repo_path, "home/.claude/CLAUDE.md");
}

#[test]
fn derive_repo_path_from_system_dir() {
    let system_path = Path::new("/etc/some-config");
    let repo_path = Manifest::derive_repo_path(system_path).unwrap();
    assert_eq!(repo_path, "system/etc/some-config");
}

#[test]
fn derive_repo_path_rejects_relative_path() {
    let system_path = Path::new("relative/path.txt");
    let result = Manifest::derive_repo_path(system_path);
    assert!(result.is_err());
}

#[test]
fn add_creates_mapping() {
    let home = dirs::home_dir().unwrap();
    let system_path = home.join(".config/ghostty/config");
    let mut manifest = Manifest::new();

    let repo_path = manifest.add(&system_path).unwrap();

    assert_eq!(repo_path, "home/.config/ghostty/config");
    assert_eq!(
        manifest.files.get("home/.config/ghostty/config").unwrap(),
        "~/.config/ghostty/config"
    );
}

#[test]
fn add_system_path_creates_mapping() {
    let system_path = Path::new("/etc/nginx/nginx.conf");
    let mut manifest = Manifest::new();

    let repo_path = manifest.add(system_path).unwrap();

    assert_eq!(repo_path, "system/etc/nginx/nginx.conf");
    assert_eq!(
        manifest.files.get("system/etc/nginx/nginx.conf").unwrap(),
        "/etc/nginx/nginx.conf"
    );
}

#[test]
fn add_duplicate_is_idempotent() {
    let home = dirs::home_dir().unwrap();
    let system_path = home.join(".claude/CLAUDE.md");
    let mut manifest = Manifest::new();

    manifest.add(&system_path).unwrap();
    manifest.add(&system_path).unwrap();

    assert_eq!(manifest.files.len(), 1);
}

#[test]
fn save_and_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.repo = Some("/Users/juljus/dotfiles".to_string());
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );
    manifest.files.insert(
        "system/etc/some-config".to_string(),
        "/etc/some-config".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert_eq!(manifest, loaded);
}

#[test]
fn save_and_load_roundtrip_without_repo() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert_eq!(manifest, loaded);
}

#[test]
fn saved_toml_is_readable() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.repo = Some("/Users/juljus/dotfiles".to_string());
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let content = fs::read_to_string(&manifest_path).unwrap();
    assert!(content.contains("repo"));
    assert!(content.contains("/Users/juljus/dotfiles"));
    assert!(content.contains("[files]"));
    assert!(content.contains("~/.claude/CLAUDE.md"));
}

#[test]
fn load_nonexistent_file_returns_error() {
    let result = Manifest::load(Path::new("/nonexistent/rootrat.toml"));
    assert!(result.is_err());
}

#[test]
fn default_path_is_in_home_config() {
    let path = Manifest::default_path();
    let home = dirs::home_dir().unwrap();
    assert_eq!(path, home.join(".config/rootrat/rootrat.toml"));
}
