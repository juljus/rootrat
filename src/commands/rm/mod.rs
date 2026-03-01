use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

/// Remove a file or directory from rootrat tracking.
/// Deletes the repo copy but leaves the system file untouched.
pub fn execute(system_path: &Path, repo_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    let repo_path = manifest.remove(system_path)?;
    let repo_dest = repo_dir.join(&repo_path);

    if repo_dest.is_dir() {
        fs::remove_dir_all(&repo_dest)?;
    } else if repo_dest.exists() {
        fs::remove_file(&repo_dest)?;
    }

    Ok(())
}
