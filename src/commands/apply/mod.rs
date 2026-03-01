use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::collect_files;
use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq)]
pub enum ApplyState {
    Created,
    Updated,
    Unchanged,
    Deleted,
    MissingFromRepo,
}

#[derive(Debug)]
pub struct ApplyEntry {
    pub system_path: String,
    pub state: ApplyState,
    pub repo_file: PathBuf,
    pub system_file: PathBuf,
}

/// Compute what changes would be applied, without making any changes.
/// Includes Deleted entries for files on system but not in repo (within tracked directories).
pub fn plan(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<ApplyEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        entries.push(plan_file(&repo_file, &system_file, system_path)?);
    }

    for (repo_path, system_path) in &manifest.directories {
        let repo_base = repo_dir.join(repo_path);
        let system_base = Manifest::expand_tilde(system_path);

        let repo_files = collect_files(&repo_base)?;
        let system_files = collect_files(&system_base)?;

        // Files in repo -> Created, Updated, or Unchanged
        for relative in &repo_files {
            let repo_file = repo_base.join(relative);
            let system_file = system_base.join(relative);
            let display = format!("{}/{}", system_path, relative.display());

            entries.push(plan_file(&repo_file, &system_file, &display)?);
        }

        // Files only on system (not in repo) -> Deleted
        for relative in system_files.difference(&repo_files) {
            let system_file = system_base.join(relative);
            let display = format!("{}/{}", system_path, relative.display());

            entries.push(ApplyEntry {
                system_path: display,
                state: ApplyState::Deleted,
                repo_file: repo_base.join(relative),
                system_file,
            });
        }
    }

    Ok(entries)
}

/// Execute a list of planned changes.
pub fn apply_entries(entries: &[ApplyEntry]) -> Result<()> {
    for entry in entries {
        match entry.state {
            ApplyState::Created => {
                if let Some(parent) = entry.system_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&entry.repo_file, &entry.system_file)?;
            }
            ApplyState::Updated => {
                fs::copy(&entry.repo_file, &entry.system_file)?;
            }
            ApplyState::Deleted => {
                fs::remove_file(&entry.system_file)?;
            }
            ApplyState::Unchanged | ApplyState::MissingFromRepo => {}
        }
    }
    Ok(())
}

/// Non-interactive apply: plan + apply, skipping deletes for safety.
/// Used by init clone where auto-deleting system files would be dangerous.
pub fn execute(repo_dir: &Path, manifest: &Manifest) -> Result<Vec<ApplyEntry>> {
    let entries = plan(repo_dir, manifest)?;
    // Only apply non-destructive changes
    let safe: Vec<&ApplyEntry> = entries
        .iter()
        .filter(|e| e.state != ApplyState::Deleted)
        .collect();
    for entry in safe {
        match entry.state {
            ApplyState::Created => {
                if let Some(parent) = entry.system_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&entry.repo_file, &entry.system_file)?;
            }
            ApplyState::Updated => {
                fs::copy(&entry.repo_file, &entry.system_file)?;
            }
            _ => {}
        }
    }
    Ok(entries)
}

fn plan_file(repo_file: &Path, system_file: &Path, display: &str) -> Result<ApplyEntry> {
    let state = if !repo_file.exists() {
        ApplyState::MissingFromRepo
    } else if system_file.exists() {
        let repo_content = fs::read(repo_file)?;
        let system_content = fs::read(system_file)?;
        if repo_content == system_content {
            ApplyState::Unchanged
        } else {
            ApplyState::Updated
        }
    } else {
        ApplyState::Created
    };

    Ok(ApplyEntry {
        system_path: display.to_string(),
        state,
        repo_file: repo_file.to_path_buf(),
        system_file: system_file.to_path_buf(),
    })
}
