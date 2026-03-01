use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::collect_files;
use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq)]
pub enum CollectState {
    Created,
    Updated,
    Unchanged,
    Deleted,
    MissingFromSystem,
}

#[derive(Debug)]
pub struct CollectEntry {
    pub system_path: String,
    pub state: CollectState,
    pub repo_file: PathBuf,
    pub system_file: PathBuf,
}

/// Compute what would be collected from system to repo, without making changes.
pub fn plan(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<CollectEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        entries.push(plan_file(&repo_file, &system_file, system_path)?);
    }

    for (repo_path, system_path) in &manifest.directories {
        let repo_base = repo_dir.join(repo_path);
        let system_base = Manifest::expand_tilde(system_path);

        let repo_files = collect_files(&repo_base, &manifest.ignore)?;
        let system_files = collect_files(&system_base, &manifest.ignore)?;

        // Files on system -> Created, Updated, or Unchanged
        for relative in &system_files {
            let repo_file = repo_base.join(relative);
            let system_file = system_base.join(relative);
            let display = format!("{}/{}", system_path, relative.display());

            entries.push(plan_file(&repo_file, &system_file, &display)?);
        }

        // Files only in repo (not on system) -> Deleted
        for relative in repo_files.difference(&system_files) {
            let repo_file = repo_base.join(relative);
            let system_file = system_base.join(relative);
            let display = format!("{}/{}", system_path, relative.display());

            entries.push(CollectEntry {
                system_path: display,
                state: CollectState::Deleted,
                repo_file,
                system_file,
            });
        }
    }

    Ok(entries)
}

/// Execute a collect plan: copy system files to repo, delete removed files from repo.
pub fn collect_entries(entries: &[CollectEntry]) -> Result<()> {
    for entry in entries {
        match entry.state {
            CollectState::Created => {
                if let Some(parent) = entry.repo_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&entry.system_file, &entry.repo_file)?;
            }
            CollectState::Updated => {
                fs::copy(&entry.system_file, &entry.repo_file)?;
            }
            CollectState::Deleted => {
                fs::remove_file(&entry.repo_file)?;
            }
            CollectState::Unchanged | CollectState::MissingFromSystem => {}
        }
    }
    Ok(())
}

fn plan_file(repo_file: &Path, system_file: &Path, display: &str) -> Result<CollectEntry> {
    let state = if !system_file.exists() {
        CollectState::MissingFromSystem
    } else if !repo_file.exists() {
        CollectState::Created
    } else {
        let repo_content = fs::read(repo_file)?;
        let system_content = fs::read(system_file)?;
        if repo_content == system_content {
            CollectState::Unchanged
        } else {
            CollectState::Updated
        }
    };

    Ok(CollectEntry {
        system_path: display.to_string(),
        state,
        repo_file: repo_file.to_path_buf(),
        system_file: system_file.to_path_buf(),
    })
}
