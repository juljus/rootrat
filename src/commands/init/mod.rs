use anyhow::{bail, Result};
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

/// Initialize rootrat by setting the repo directory.
pub fn execute(repo_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    if !repo_dir.exists() {
        bail!("directory does not exist: {}", repo_dir.display());
    }

    manifest.repo = Some(Manifest::to_display_path(repo_dir)?);

    Ok(())
}
