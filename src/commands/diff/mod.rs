use anyhow::Result;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct DiffEntry {
    pub system_path: String,
    pub diff: String,
}

/// Show diffs for tracked files. If `filter` is provided, only show that file.
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

        let diff = match (repo_file.exists(), system_file.exists()) {
            (false, _) => format!("  [missing from repo: {}]", repo_path),
            (_, false) => format!("  [missing from system: {}]", system_path),
            (true, true) => {
                let repo_content = fs::read_to_string(&repo_file)?;
                let system_content = fs::read_to_string(&system_file)?;

                if repo_content == system_content {
                    continue;
                }

                format_diff(&repo_content, &system_content)
            }
        };

        entries.push(DiffEntry {
            system_path: system_path.clone(),
            diff,
        });
    }

    Ok(entries)
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
