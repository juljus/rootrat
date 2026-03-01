use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq)]
pub enum ApplyState {
    Created,
    Updated,
    Unchanged,
    MissingFromRepo,
}

#[derive(Debug)]
pub struct ApplyEntry {
    pub system_path: String,
    pub state: ApplyState,
}

/// Apply all tracked files from the repo to their system locations.
pub fn execute(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<ApplyEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        if !repo_file.exists() {
            entries.push(ApplyEntry {
                system_path: system_path.clone(),
                state: ApplyState::MissingFromRepo,
            });
            continue;
        }

        let repo_content = fs::read(&repo_file)?;

        let state = if system_file.exists() {
            let system_content = fs::read(&system_file)?;
            if repo_content == system_content {
                ApplyState::Unchanged
            } else {
                fs::write(&system_file, &repo_content)?;
                ApplyState::Updated
            }
        } else {
            if let Some(parent) = system_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&system_file, &repo_content)?;
            ApplyState::Created
        };

        entries.push(ApplyEntry {
            system_path: system_path.clone(),
            state,
        });
    }

    Ok(entries)
}
