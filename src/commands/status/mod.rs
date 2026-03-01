use anyhow::Result;
use std::fs;
use std::path::Path;

use super::collect_files;
use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq)]
pub enum FileState {
    Unchanged,
    Modified,
    MissingFromSystem,
    MissingFromRepo,
}

#[derive(Debug)]
pub struct StatusEntry {
    pub system_path: String,
    pub state: FileState,
}

/// Check the status of all tracked files and directories.
/// Returns a list of status entries for each file in the manifest.
pub fn execute(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<StatusEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        let state = compare_files(&repo_file, &system_file)?;

        entries.push(StatusEntry {
            system_path: system_path.clone(),
            state,
        });
    }

    for (repo_path, system_path) in &manifest.directories {
        let repo_base = repo_dir.join(repo_path);
        let system_base = Manifest::expand_tilde(system_path);

        let repo_files = collect_files(&repo_base)?;
        let system_files = collect_files(&system_base)?;
        let all_files: std::collections::BTreeSet<_> =
            repo_files.union(&system_files).collect();

        for relative in all_files {
            let repo_file = repo_base.join(relative);
            let system_file = system_base.join(relative);

            let state = compare_files(&repo_file, &system_file)?;

            let display = format!("{}/{}", system_path, relative.display());
            entries.push(StatusEntry {
                system_path: display,
                state,
            });
        }
    }

    Ok(entries)
}

fn compare_files(repo_file: &Path, system_file: &Path) -> Result<FileState> {
    Ok(match (repo_file.exists(), system_file.exists()) {
        (false, _) => FileState::MissingFromRepo,
        (_, false) => FileState::MissingFromSystem,
        (true, true) => {
            let repo_content = fs::read(repo_file)?;
            let system_content = fs::read(system_file)?;
            if repo_content == system_content {
                FileState::Unchanged
            } else {
                FileState::Modified
            }
        }
    })
}
