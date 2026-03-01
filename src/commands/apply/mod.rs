use anyhow::Result;
use std::fs;
use std::path::Path;

use super::collect_files;
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

/// Apply all tracked files and directories from the repo to their system locations.
pub fn execute(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<ApplyEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        entries.push(apply_file(&repo_file, &system_file, system_path)?);
    }

    for (repo_path, system_path) in &manifest.directories {
        let repo_base = repo_dir.join(repo_path);
        let system_base = Manifest::expand_tilde(system_path);

        let repo_files = collect_files(&repo_base)?;

        for relative in &repo_files {
            let repo_file = repo_base.join(relative);
            let system_file = system_base.join(relative);
            let display = format!("{}/{}", system_path, relative.display());

            entries.push(apply_file(&repo_file, &system_file, &display)?);
        }
    }

    Ok(entries)
}

fn apply_file(repo_file: &Path, system_file: &Path, display: &str) -> Result<ApplyEntry> {
    if !repo_file.exists() {
        return Ok(ApplyEntry {
            system_path: display.to_string(),
            state: ApplyState::MissingFromRepo,
        });
    }

    let repo_content = fs::read(repo_file)?;

    let state = if system_file.exists() {
        let system_content = fs::read(system_file)?;
        if repo_content == system_content {
            ApplyState::Unchanged
        } else {
            fs::write(system_file, &repo_content)?;
            ApplyState::Updated
        }
    } else {
        if let Some(parent) = system_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(system_file, &repo_content)?;
        ApplyState::Created
    };

    Ok(ApplyEntry {
        system_path: display.to_string(),
        state,
    })
}
