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
