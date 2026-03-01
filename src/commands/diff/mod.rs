use anyhow::Result;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

use super::collect_files;
use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct DiffEntry {
    pub system_path: String,
    pub diff: String,
}

/// Show diffs for tracked files and directories. If `filter` is provided, only show that path.
pub fn execute(
    repo_dir: &Path,
    manifest: &Manifest,
    filter: Option<&str>,
) -> Result<Vec<DiffEntry>> {
    let mut entries = Vec::new();

    for (repo_path, system_path) in &manifest.files {
        if let Some(f) = filter {
            if system_path != f {
                continue;
            }
        }

        let repo_file = repo_dir.join(repo_path);
        let system_file = Manifest::expand_tilde(system_path);

        if let Some(entry) = diff_file(&repo_file, &system_file, system_path, repo_path)? {
            entries.push(entry);
        }
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
            let display = format!("{}/{}", system_path, relative.display());
            let repo_display = format!("{}/{}", repo_path, relative.display());

            if let Some(f) = filter {
                if display != f {
                    continue;
                }
            }

            if let Some(entry) = diff_file(&repo_file, &system_file, &display, &repo_display)? {
                entries.push(entry);
            }
        }
    }

    Ok(entries)
}

/// Diff a single file pair. Returns None if the files are identical.
fn diff_file(
    repo_file: &Path,
    system_file: &Path,
    system_display: &str,
    repo_display: &str,
) -> Result<Option<DiffEntry>> {
    let diff = match (repo_file.exists(), system_file.exists()) {
        (false, _) => format!("  [missing from repo: {}]", repo_display),
        (_, false) => format!("  [missing from system: {}]", system_display),
        (true, true) => {
            let repo_content = fs::read_to_string(repo_file)?;
            let system_content = fs::read_to_string(system_file)?;

            if repo_content == system_content {
                return Ok(None);
            }

            format_diff(&repo_content, &system_content)
        }
    };

    Ok(Some(DiffEntry {
        system_path: system_display.to_string(),
        diff,
    }))
}

fn format_diff(repo_content: &str, system_content: &str) -> String {
    let text_diff = TextDiff::from_lines(repo_content, system_content);
    let mut output = String::new();

    for change in text_diff.iter_all_changes() {
        let marker = match change.tag() {
            ChangeTag::Delete => "- ",
            ChangeTag::Insert => "+ ",
            ChangeTag::Equal => "  ",
        };
        output.push_str(marker);
        output.push_str(change.value());
    }

    output
}
