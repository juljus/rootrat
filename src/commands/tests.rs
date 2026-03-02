use super::*;
use std::process::Command;
use tempfile::TempDir;

fn init_repo(dir: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    // Set identity for the test repo so initial commits work
    Command::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn git_current_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_log_oneline(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["log", "--oneline", "--format=%s"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn git_log_author(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%an <%ae>"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// -- git_commit tests --

#[test]
fn git_commit_creates_commit() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("file.txt"), "hello").unwrap();

    git_commit(dir.path(), "add file").unwrap();

    let log = git_log_oneline(dir.path());
    assert!(log.contains("add file"));
}

#[test]
fn git_commit_uses_rootrat_identity() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("file.txt"), "hello").unwrap();

    git_commit(dir.path(), "test").unwrap();

    let author = git_log_author(dir.path());
    assert_eq!(author, "rootrat <>");
}

#[test]
fn git_commit_nothing_to_commit_is_ok() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    // No changes -- should not error
    let result = git_commit(dir.path(), "empty");
    assert!(result.is_ok());
}

#[test]
fn git_commit_multiple_files() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();

    git_commit(dir.path(), "add two files").unwrap();

    let log = git_log_oneline(dir.path());
    assert!(log.contains("add two files"));

    // Both files should be tracked
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&output.stdout);
    assert!(files.contains("a.txt"));
    assert!(files.contains("b.txt"));
}

#[test]
fn git_commit_stages_deletions() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    git_commit(dir.path(), "add").unwrap();

    fs::remove_file(dir.path().join("file.txt")).unwrap();
    git_commit(dir.path(), "remove").unwrap();

    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&output.stdout);
    assert!(!files.contains("file.txt"));
}

// -- git_init tests --

#[test]
fn git_init_creates_repo() {
    let dir = TempDir::new().unwrap();

    git_init(dir.path()).unwrap();

    assert!(dir.path().join(".git").exists());
}

#[test]
fn git_init_skips_existing_repo() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    // Should not error on an existing repo
    let result = git_init(dir.path());
    assert!(result.is_ok());
}

#[test]
fn git_init_then_commit_works() {
    let dir = TempDir::new().unwrap();

    git_init(dir.path()).unwrap();
    fs::write(dir.path().join("rootrat.toml"), "ignore = []").unwrap();
    git_commit(dir.path(), "init").unwrap();

    let log = git_log_oneline(dir.path());
    assert!(log.contains("init"));
}

// -- git_pull tests --

#[test]
fn git_pull_no_remote_fails() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    let result = git_pull(dir.path());
    assert!(result.is_err());
}

#[test]
fn git_pull_from_remote() {
    // Set up a bare remote
    let remote_dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_dir.path())
        .output()
        .unwrap();

    // Create a repo and push to the remote
    let origin_dir = TempDir::new().unwrap();
    init_repo(origin_dir.path());
    let branch = git_current_branch(origin_dir.path());
    Command::new("git")
        .args(["remote", "add", "origin", &remote_dir.path().to_string_lossy()])
        .current_dir(origin_dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(origin_dir.path())
        .output()
        .unwrap();

    // Make a new commit in origin
    fs::write(origin_dir.path().join("new.txt"), "new content").unwrap();
    git_commit(origin_dir.path(), "new file").unwrap();
    Command::new("git")
        .args(["push"])
        .current_dir(origin_dir.path())
        .output()
        .unwrap();

    // Clone from the remote
    let clone_dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["clone", &remote_dir.path().to_string_lossy(), "."])
        .current_dir(clone_dir.path())
        .output()
        .unwrap();

    // Pull should succeed and bring in the new file
    git_pull(clone_dir.path()).unwrap();
    assert!(clone_dir.path().join("new.txt").exists());
}

// -- git_push tests --

#[test]
fn git_push_no_remote_fails() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    let result = git_push(dir.path());
    assert!(result.is_err());
}

#[test]
fn git_push_to_remote() {
    // Set up a bare remote
    let remote_dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_dir.path())
        .output()
        .unwrap();

    // Create a repo with a remote
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());
    let branch = git_current_branch(dir.path());
    Command::new("git")
        .args(["remote", "add", "origin", &remote_dir.path().to_string_lossy()])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Initial push to set up tracking
    Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Make a new commit and push with our helper
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    git_commit(dir.path(), "add file").unwrap();
    git_push(dir.path()).unwrap();

    // Verify the commit made it to the remote by cloning
    let verify_dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["clone", &remote_dir.path().to_string_lossy(), "."])
        .current_dir(verify_dir.path())
        .output()
        .unwrap();
    assert!(verify_dir.path().join("file.txt").exists());
}

// -- collect_files gitignore tests --

#[test]
fn collect_files_respects_gitignore() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(dir.path().join("a.txt"), "hello").unwrap();
    fs::write(dir.path().join("b.log"), "debug stuff").unwrap();

    let files = collect_files(dir.path(), &[]).unwrap();
    let names: Vec<&str> = files.iter().filter_map(|p| p.to_str()).collect();

    assert!(names.contains(&"a.txt"));
    assert!(!names.contains(&"b.log"));
    // .gitignore itself should be collected (it's not ignored)
    assert!(names.contains(&".gitignore"));
}

#[test]
fn collect_files_nested_gitignore() {
    let dir = TempDir::new().unwrap();

    // Parent ignores *.tmp
    fs::write(dir.path().join(".gitignore"), "*.tmp\n").unwrap();
    fs::write(dir.path().join("keep.txt"), "keep").unwrap();
    fs::write(dir.path().join("drop.tmp"), "drop").unwrap();

    // Child dir ignores secret.txt
    let child = dir.path().join("sub");
    fs::create_dir(&child).unwrap();
    fs::write(child.join(".gitignore"), "secret.txt\n").unwrap();
    fs::write(child.join("visible.txt"), "visible").unwrap();
    fs::write(child.join("secret.txt"), "secret").unwrap();
    fs::write(child.join("also_drop.tmp"), "also drop").unwrap();

    let files = collect_files(dir.path(), &[]).unwrap();
    let names: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    assert!(names.contains(&"keep.txt".to_string()));
    assert!(!names.contains(&"drop.tmp".to_string()));
    assert!(names.contains(&"sub/visible.txt".to_string()));
    assert!(!names.contains(&"sub/secret.txt".to_string()));
    // Parent *.tmp rule still applies in child
    assert!(!names.contains(&"sub/also_drop.tmp".to_string()));
}

#[test]
fn collect_files_gitignore_negation() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".gitignore"), "*.log\n!important.log\n").unwrap();
    fs::write(dir.path().join("debug.log"), "debug").unwrap();
    fs::write(dir.path().join("important.log"), "important").unwrap();
    fs::write(dir.path().join("readme.txt"), "readme").unwrap();

    let files = collect_files(dir.path(), &[]).unwrap();
    let names: Vec<&str> = files.iter().filter_map(|p| p.to_str()).collect();

    assert!(!names.contains(&"debug.log"));
    assert!(names.contains(&"important.log"));
    assert!(names.contains(&"readme.txt"));
}

#[test]
fn collect_files_gitignore_directory_pattern() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".gitignore"), "build/\n").unwrap();
    fs::write(dir.path().join("src.txt"), "source").unwrap();

    let build_dir = dir.path().join("build");
    fs::create_dir(&build_dir).unwrap();
    fs::write(build_dir.join("output.bin"), "binary").unwrap();
    fs::write(build_dir.join("manifest.json"), "{}").unwrap();

    let files = collect_files(dir.path(), &[]).unwrap();
    let names: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    assert!(names.contains(&"src.txt".to_string()));
    assert!(!names.contains(&"build/output.bin".to_string()));
    assert!(!names.contains(&"build/manifest.json".to_string()));
}
