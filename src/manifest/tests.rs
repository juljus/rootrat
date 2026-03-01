use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// -- LocalConfig tests --

#[test]
fn local_config_default_path_is_in_home_config() {
    let path = LocalConfig::default_path();
    let home = dirs::home_dir().unwrap();
    assert_eq!(path, home.join(".config/rootrat/rootrat.toml"));
}

#[test]
fn local_config_save_and_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("rootrat.toml");

    let config = LocalConfig {
        repo: "~/Projects/dotfiles".to_string(),
    };
    config.save(&config_path).unwrap();

    let loaded = LocalConfig::load(&config_path).unwrap();
    assert_eq!(config, loaded);
}

#[test]
fn local_config_saved_toml_is_readable() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("rootrat.toml");

    let config = LocalConfig {
        repo: "~/Projects/dotfiles".to_string(),
    };
    config.save(&config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("repo"));
    assert!(content.contains("~/Projects/dotfiles"));
    assert!(!content.contains("[files]"));
}

#[test]
fn local_config_repo_dir_expands_tilde() {
    let config = LocalConfig {
        repo: "~/Projects/dotfiles".to_string(),
    };
    let home = dirs::home_dir().unwrap();
    assert_eq!(config.repo_dir(), home.join("Projects/dotfiles"));
}

#[test]
fn local_config_load_nonexistent_returns_error() {
    let result = LocalConfig::load(Path::new("/nonexistent/rootrat.toml"));
    assert!(result.is_err());
}

// -- Manifest tests --

#[test]
fn new_manifest_has_empty_maps() {
    let manifest = Manifest::new();
    assert!(manifest.files.is_empty());
    assert!(manifest.directories.is_empty());
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
fn saved_toml_is_readable() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let content = fs::read_to_string(&manifest_path).unwrap();
    assert!(content.contains("[files]"));
    assert!(content.contains("~/.claude/CLAUDE.md"));
}

#[test]
fn load_nonexistent_file_returns_error() {
    let result = Manifest::load(Path::new("/nonexistent/rootrat.toml"));
    assert!(result.is_err());
}

#[test]
fn load_from_repo_returns_empty_when_missing() {
    let dir = TempDir::new().unwrap();
    let manifest = Manifest::load_from_repo(dir.path()).unwrap();
    assert!(manifest.files.is_empty());
    assert!(manifest.directories.is_empty());
}

#[test]
fn save_to_repo_and_load_from_repo() {
    let dir = TempDir::new().unwrap();
    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.testrc".to_string(),
        "~/.testrc".to_string(),
    );

    manifest.save_to_repo(dir.path()).unwrap();

    let loaded = Manifest::load_from_repo(dir.path()).unwrap();
    assert_eq!(manifest, loaded);
}

// -- Directory support tests --

#[test]
fn add_directory_creates_mapping() {
    let home = dirs::home_dir().unwrap();
    let nvim_dir = home.join(".config/nvim");
    fs::create_dir_all(&nvim_dir).unwrap();

    let mut manifest = Manifest::new();
    let repo_path = manifest.add(&nvim_dir).unwrap();

    assert_eq!(repo_path, "home/.config/nvim");
    assert_eq!(
        manifest.directories.get("home/.config/nvim").unwrap(),
        "~/.config/nvim"
    );
    assert!(manifest.files.is_empty());

    // Cleanup
    fs::remove_dir(&nvim_dir).ok();
}

#[test]
fn add_directory_system_path_creates_mapping() {
    let tmp = TempDir::new().unwrap();
    let system_dir = tmp.path().join("etc/nginx");
    fs::create_dir_all(&system_dir).unwrap();

    let mut manifest = Manifest::new();
    let repo_path = manifest.add(&system_dir).unwrap();

    // Should go into directories, not files
    assert!(manifest.directories.contains_key(&repo_path));
    assert!(manifest.files.is_empty());
}

#[test]
fn add_file_still_goes_to_files() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.conf");
    fs::write(&file_path, "content").unwrap();

    let mut manifest = Manifest::new();
    manifest.add(&file_path).unwrap();

    assert!(manifest.directories.is_empty());
    assert_eq!(manifest.files.len(), 1);
}

#[test]
fn add_directory_duplicate_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let dir_path = tmp.path().join("mydir");
    fs::create_dir(&dir_path).unwrap();

    let mut manifest = Manifest::new();
    manifest.add(&dir_path).unwrap();
    manifest.add(&dir_path).unwrap();

    assert_eq!(manifest.directories.len(), 1);
}

#[test]
fn save_and_load_roundtrip_with_directories() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.claude/CLAUDE.md".to_string(),
        "~/.claude/CLAUDE.md".to_string(),
    );
    manifest.directories.insert(
        "home/.config/nvim".to_string(),
        "~/.config/nvim".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert_eq!(manifest, loaded);
}

#[test]
fn saved_toml_with_directories_is_readable() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.directories.insert(
        "home/.config/nvim".to_string(),
        "~/.config/nvim".to_string(),
    );

    manifest.save(&manifest_path).unwrap();

    let content = fs::read_to_string(&manifest_path).unwrap();
    assert!(content.contains("[directories]"));
    assert!(content.contains("~/.config/nvim"));
}

#[test]
fn load_manifest_without_directories_section() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    // Write a manifest without [directories] -- backward compat
    let content = "[files]\n\"home/.claude/CLAUDE.md\" = \"~/.claude/CLAUDE.md\"\n";
    fs::write(&manifest_path, content).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert!(loaded.directories.is_empty());
    assert_eq!(loaded.files.len(), 1);
}

#[test]
fn load_empty_manifest() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    fs::write(&manifest_path, "").unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert!(loaded.files.is_empty());
    assert!(loaded.directories.is_empty());
}

// -- Ignore tests --

#[test]
fn new_manifest_has_default_ignore() {
    let manifest = Manifest::new();
    assert!(manifest.ignore.contains(&".DS_Store".to_string()));
    assert!(manifest.ignore.contains(&"Thumbs.db".to_string()));
    assert!(manifest.ignore.contains(&".git".to_string()));
}

#[test]
fn save_and_load_roundtrip_with_ignore() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let mut manifest = Manifest::new();
    manifest.ignore = vec!["custom-ignore".to_string()];

    manifest.save(&manifest_path).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert_eq!(loaded.ignore, vec!["custom-ignore".to_string()]);
}

#[test]
fn load_manifest_without_ignore_gets_defaults() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let content = "[files]\n\"home/.testrc\" = \"~/.testrc\"\n";
    fs::write(&manifest_path, content).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert!(loaded.ignore.contains(&".DS_Store".to_string()));
    assert!(loaded.ignore.contains(&".git".to_string()));
}

#[test]
fn explicit_empty_ignore_is_respected() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let content = "ignore = []\n";
    fs::write(&manifest_path, content).unwrap();

    let loaded = Manifest::load(&manifest_path).unwrap();
    assert!(loaded.ignore.is_empty());
}

#[test]
fn saved_toml_contains_ignore_list() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("rootrat.toml");

    let manifest = Manifest::new();
    manifest.save(&manifest_path).unwrap();

    let content = fs::read_to_string(&manifest_path).unwrap();
    assert!(content.contains("ignore"));
    assert!(content.contains(".DS_Store"));
    assert!(content.contains("Thumbs.db"));
    assert!(content.contains(".git"));
}

// -- Remove tests --

#[test]
fn remove_file_entry() {
    let home = dirs::home_dir().unwrap();
    let system_path = home.join(".config/ghostty/config");
    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.config/ghostty/config".to_string(),
        "~/.config/ghostty/config".to_string(),
    );

    let repo_path = manifest.remove(&system_path).unwrap();

    assert_eq!(repo_path, "home/.config/ghostty/config");
    assert!(manifest.files.is_empty());
}

#[test]
fn remove_directory_entry() {
    let home = dirs::home_dir().unwrap();
    let system_path = home.join(".config/nvim");
    let mut manifest = Manifest::new();
    manifest.directories.insert(
        "home/.config/nvim".to_string(),
        "~/.config/nvim".to_string(),
    );

    let repo_path = manifest.remove(&system_path).unwrap();

    assert_eq!(repo_path, "home/.config/nvim");
    assert!(manifest.directories.is_empty());
}

#[test]
fn remove_nonexistent_errors() {
    let mut manifest = Manifest::new();
    let result = manifest.remove(Path::new("/etc/not-tracked"));
    assert!(result.is_err());
}

#[test]
fn remove_leaves_other_entries() {
    let home = dirs::home_dir().unwrap();
    let mut manifest = Manifest::new();
    manifest.files.insert(
        "home/.config/a.conf".to_string(),
        "~/.config/a.conf".to_string(),
    );
    manifest.files.insert(
        "home/.config/b.conf".to_string(),
        "~/.config/b.conf".to_string(),
    );

    manifest.remove(&home.join(".config/a.conf")).unwrap();

    assert_eq!(manifest.files.len(), 1);
    assert!(manifest.files.contains_key("home/.config/b.conf"));
}
