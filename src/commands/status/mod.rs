use anyhow::Result;
use std::fs;
use std::path::Path;

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
    pub repo_path: String,
    pub system_path: String,
    pub state: FileState,
}

/// Check the status of all tracked files.
/// Returns a list of status entries for each file in the manifest.
pub fn execute(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<StatusEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        let state = match (repo_file.exists(), system_file.exists()) {
            (false, _) => FileState::MissingFromRepo,
            (_, false) => FileState::MissingFromSystem,
            (true, true) => {
                let repo_content = fs::read(&repo_file)?;
                let system_content = fs::read(&system_file)?;
                if repo_content == system_content {
                    FileState::Unchanged
                } else {
                    FileState::Modified
                }
            }
        };

        entries.push(StatusEntry {
            repo_path: repo_path.clone(),
            system_path: system_path.clone(),
            state,
        });
    }

    Ok(entries)
}
