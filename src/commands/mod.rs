pub mod add;
pub mod apply;
pub mod collect;
pub mod diff;
pub mod init;
pub mod rm;
pub mod status;

use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively collect all file paths within a directory, relative to `base`.
/// Returns a sorted set for consistent ordering and easy set operations.
pub fn collect_files(base: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    if base.exists() {
        collect_files_recursive(base, base, &mut files)?;
    }
    Ok(files)
}

fn collect_files_recursive(
    base: &Path,
    dir: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(base, &path, files)?;
        } else {
            files.insert(path.strip_prefix(base)?.to_path_buf());
        }
    }
    Ok(())
}
