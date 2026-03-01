use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

/// Add a file or directory to the rootrat repo and update the manifest.
pub fn execute(system_path: &Path, repo_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    if !system_path.exists() {
        bail!("path does not exist: {}", system_path.display());
    }

    let repo_path = manifest.add(system_path)?;
    let dest = repo_dir.join(&repo_path);

    if system_path.is_dir() {
        copy_dir_recursive(system_path, &dest)?;
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(system_path, &dest)?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
