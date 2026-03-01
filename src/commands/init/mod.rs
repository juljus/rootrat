use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

pub struct CloneResult {
    pub repo_dir: PathBuf,
    pub manifest: Manifest,
}

/// Initialize rootrat by setting the repo directory.
pub fn execute(repo_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    if !repo_dir.exists() {
        bail!("directory does not exist: {}", repo_dir.display());
    }

    manifest.repo = Some(Manifest::to_display_path(repo_dir)?);

    Ok(())
}

/// Normalize a git URL. Adds https:// if no protocol is specified.
/// Local paths (starting with / or .) are left as-is.
pub fn normalize_url(url: &str) -> String {
    if url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("git@")
        || url.starts_with('/')
        || url.starts_with('.')
    {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// Clone a repo from a URL into `target_dir`, load the manifest, and set the repo path.
pub fn clone_and_init(url: &str, target_dir: &Path) -> Result<CloneResult> {
    let normalized = normalize_url(url);

    let output = Command::new("git")
        .args(["clone", &normalized])
        .current_dir(target_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed: {}", stderr.trim());
    }

    // git clone creates a subdirectory named after the repo
    let repo_name = normalized
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("dotfiles");

    let repo_dir = target_dir.join(repo_name);
    let manifest_path = repo_dir.join("rootrat.toml");

    if !manifest_path.exists() {
        bail!(
            "cloned repo does not contain a rootrat.toml at: {}",
            manifest_path.display()
        );
    }

    let mut manifest = Manifest::load(&manifest_path)?;
    manifest.repo = Some(Manifest::to_display_path(&repo_dir)?);

    Ok(CloneResult { repo_dir, manifest })
}
