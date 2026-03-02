pub mod add;
pub mod apply;
pub mod collect;
pub mod diff;
pub mod init;
pub mod rm;
pub mod status;

use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
mod tests;

/// Stage all changes and commit in the given repo directory with the rootrat identity.
/// Does nothing if there are no changes to commit.
pub fn git_commit(repo_dir: &Path, message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git add failed: {}", stderr.trim());
    }

    let output = Command::new("git")
        .args([
            "-c", "user.name=rootrat",
            "-c", "user.email=",
            "commit", "-m", message,
        ])
        .current_dir(repo_dir)
        .output()?;

    // Exit code 1 with "nothing to commit" is not an error
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("nothing to commit") {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git commit failed: {}", stderr.trim());
    }

    Ok(())
}

/// Initialize a git repo in the given directory if it isn't one already.
pub fn git_init(repo_dir: &Path) -> Result<()> {
    // Check if already a git repo
    let status = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if status.success() {
        return Ok(());
    }

    let output = Command::new("git")
        .args(["init"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git init failed: {}", stderr.trim());
    }

    Ok(())
}

/// Get the current HEAD commit hash.
fn git_head(repo_dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-parse HEAD failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Count commits between two revisions.
fn git_count_commits(repo_dir: &Path, from: &str, to: &str) -> Result<usize> {
    let range = format!("{}..{}", from, to);
    let output = Command::new("git")
        .args(["rev-list", "--count", &range])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        return Ok(0);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or(0))
}

/// Run `git pull` in the given repo directory. Returns the number of commits pulled.
pub fn git_pull(repo_dir: &Path) -> Result<usize> {
    let before = git_head(repo_dir)?;
    let output = Command::new("git")
        .args(["pull"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git pull failed: {}", stderr.trim());
    }
    let after = git_head(repo_dir)?;
    git_count_commits(repo_dir, &before, &after)
}

/// Run `git pull --rebase` in the given repo directory. Returns the number of commits pulled.
pub fn git_pull_rebase(repo_dir: &Path) -> Result<usize> {
    let before = git_head(repo_dir)?;
    let output = Command::new("git")
        .args(["pull", "--rebase"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git pull --rebase failed: {}", stderr.trim());
    }
    let after = git_head(repo_dir)?;
    git_count_commits(repo_dir, &before, &after)
}

/// Count the number of commits ahead of the remote tracking branch.
pub fn git_unpushed_count(repo_dir: &Path) -> Result<usize> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "@{u}..HEAD"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        return Ok(0);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or(0))
}

/// Run `git push` in the given repo directory. Returns the number of commits pushed.
pub fn git_push(repo_dir: &Path) -> Result<usize> {
    let count = git_unpushed_count(repo_dir)?;
    let output = Command::new("git")
        .args(["push"])
        .current_dir(repo_dir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git push failed: {}", stderr.trim());
    }
    Ok(count)
}

/// Recursively collect all file paths within a directory, relative to `base`.
/// Skips files and directories whose name matches any entry in `ignore`.
/// Returns a sorted set for consistent ordering and easy set operations.
pub fn collect_files(base: &Path, ignore: &[String]) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    if base.exists() {
        collect_files_recursive(base, base, ignore, &mut files)?;
    }
    Ok(files)
}

fn collect_files_recursive(
    base: &Path,
    dir: &Path,
    ignore: &[String],
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if ignore.iter().any(|i| i.as_str() == name) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(base, &path, ignore, files)?;
        } else {
            files.insert(path.strip_prefix(base)?.to_path_buf());
        }
    }
    Ok(())
}
