use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

/// Add a file to the rootrat repo and update the manifest.
pub fn execute(system_path: &Path, repo_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    if !system_path.exists() {
        bail!("file does not exist: {}", system_path.display());
    }

    let repo_path = manifest.add(system_path)?;
    let dest = repo_dir.join(&repo_path);

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(system_path, &dest)?;

    Ok(())
}
