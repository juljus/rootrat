use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn returns_config_with_path_outside_home() {
    let dir = TempDir::new().unwrap();

    let config = execute(dir.path()).unwrap();

    // Outside home dir, should store absolute path
    assert_eq!(config.repo, dir.path().to_str().unwrap());
}

#[test]
fn returns_config_with_tilde_for_home_path() {
    let home = dirs::home_dir().unwrap();
    let dir = home.join(".rootrat_test_init");
    fs::create_dir_all(&dir).unwrap();

    let config = execute(&dir).unwrap();

    assert_eq!(config.repo, "~/.rootrat_test_init");

    fs::remove_dir(&dir).unwrap();
}

#[test]
fn repo_dir_resolves_correctly() {
    let dir = TempDir::new().unwrap();

    let config = execute(dir.path()).unwrap();

    assert_eq!(config.repo_dir(), dir.path());
}

#[test]
fn fails_if_dir_does_not_exist() {
    let result = execute(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

// -- URL normalization tests --

#[test]
fn normalize_url_adds_https_for_bare_domain() {
    let url = normalize_url("github.com/juljus/dotfiles");
    assert_eq!(url, "https://github.com/juljus/dotfiles");
}

#[test]
fn normalize_url_keeps_https() {
    let url = normalize_url("https://github.com/juljus/dotfiles");
    assert_eq!(url, "https://github.com/juljus/dotfiles");
}

#[test]
fn normalize_url_keeps_ssh() {
    let url = normalize_url("git@github.com:juljus/dotfiles.git");
    assert_eq!(url, "git@github.com:juljus/dotfiles.git");
}

// -- Clone + init tests (using local bare repos) --

/// Create a bare git repo with a rootrat.toml in it.
fn create_test_remote(dir: &Path) {
    Command::new("git")
        .args(["init", "--bare"])
        .arg(dir)
        .output()
        .unwrap();

    // We need a working copy to commit into the bare repo
    let work_dir = dir.parent().unwrap().join("work");
    Command::new("git")
        .args(["clone", dir.to_str().unwrap(), work_dir.to_str().unwrap()])
        .output()
        .unwrap();

    // Create a rootrat.toml with a test mapping (no repo field -- that's in LocalConfig now)
    let manifest_content = "[files]\n\"home/.testrc\" = \"~/.testrc\"\n";
    fs::write(work_dir.join("rootrat.toml"), manifest_content).unwrap();
    fs::create_dir_all(work_dir.join("home")).unwrap();
    fs::write(work_dir.join("home/.testrc"), "test content").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(&work_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(&work_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["push"])
        .current_dir(&work_dir)
        .output()
        .unwrap();

    fs::remove_dir_all(&work_dir).unwrap();
}

#[test]
fn clone_from_url_creates_repo() {
    let tmp = TempDir::new().unwrap();
    let remote = tmp.path().join("remote.git");
    create_test_remote(&remote);

    let clone_dir = tmp.path().join("clone_target");
    fs::create_dir_all(&clone_dir).unwrap();

    let result = clone_and_init(remote.to_str().unwrap(), &clone_dir).unwrap();

    assert!(result.repo_dir.join("rootrat.toml").exists());
    assert!(result.repo_dir.join("home/.testrc").exists());
}

#[test]
fn clone_from_url_returns_config() {
    let tmp = TempDir::new().unwrap();
    let remote = tmp.path().join("remote.git");
    create_test_remote(&remote);

    let clone_dir = tmp.path().join("clone_target");
    fs::create_dir_all(&clone_dir).unwrap();

    let result = clone_and_init(remote.to_str().unwrap(), &clone_dir).unwrap();

    assert!(!result.config.repo.is_empty());
    assert!(result.repo_dir.join("rootrat.toml").exists());
}
