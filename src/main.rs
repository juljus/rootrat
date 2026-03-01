mod commands;
mod manifest;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use manifest::Manifest;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rootrat", about = "A dotfiles manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a file to rootrat tracking
    Add {
        /// Path to the file to track
        path: String,
    },
    /// Apply tracked files from repo to system
    Apply,
    /// Show differences between repo and system files
    Diff {
        /// Only show diff for this file
        path: Option<String>,
    },
    /// Initialize rootrat (optionally clone from a git URL)
    Init {
        /// Git URL to clone from (e.g. github.com/user/dotfiles)
        url: Option<String>,
    },
    /// Show status of tracked files
    Status,
}

/// Expand ~ and resolve to an absolute path. Fails if the path doesn't exist.
fn resolve_path(path: &str) -> Result<PathBuf> {
    Ok(std::fs::canonicalize(Manifest::expand_tilde(path))?)
}

/// Get the repo dir from the manifest, or bail.
fn repo_dir(manifest: &Manifest) -> Result<PathBuf> {
    match &manifest.repo {
        Some(r) => Ok(Manifest::expand_tilde(r)),
        None => bail!("no repo configured -- run `rootrat init` first"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { path } => {
            let system_path = resolve_path(&path)?;
            let mut manifest = Manifest::load_or_create()?;
            let repo = repo_dir(&manifest)?;

            commands::add::execute(&system_path, &repo, &mut manifest)?;
            manifest.save_default()?;

            println!("added: {}", system_path.display());
        }
        Commands::Apply => {
            let manifest = Manifest::load_or_create()?;
            let repo = repo_dir(&manifest)?;

            let entries = commands::apply::execute(&repo, &manifest)?;

            if entries.is_empty() {
                println!("no files tracked");
            } else {
                use commands::apply::ApplyState;
                for entry in &entries {
                    let marker = match entry.state {
                        ApplyState::Created => "  created",
                        ApplyState::Updated => "  updated",
                        ApplyState::Unchanged => "  unchanged",
                        ApplyState::MissingFromRepo => "  missing (repo)",
                    };
                    println!("{:>20}  {}", marker, entry.system_path);
                }
            }
        }
        Commands::Diff { path } => {
            let manifest = Manifest::load_or_create()?;
            let repo = repo_dir(&manifest)?;

            let entries = commands::diff::execute(&repo, &manifest, path.as_deref())?;

            if entries.is_empty() {
                println!("no differences");
            } else {
                for entry in &entries {
                    println!("--- {}", entry.system_path);
                    print!("{}", entry.diff);
                    println!();
                }
            }
        }
        Commands::Init { url: None } => {
            let dir = std::env::current_dir()?;
            let mut manifest = Manifest::load_or_create()?;

            commands::init::execute(&dir, &mut manifest)?;
            manifest.save_default()?;

            println!("initialized rootrat repo at: {}", dir.display());
        }
        Commands::Init { url: Some(url) } => {
            let dir = std::env::current_dir()?;
            let result = commands::init::clone_and_init(&url, &dir)?;

            println!("cloned to: {}", result.repo_dir.display());

            // Apply all files from the cloned repo
            let entries = commands::apply::execute(&result.repo_dir, &result.manifest)?;

            use commands::apply::ApplyState;
            for entry in &entries {
                let marker = match entry.state {
                    ApplyState::Created => "  created",
                    ApplyState::Updated => "  updated",
                    ApplyState::Unchanged => "  unchanged",
                    ApplyState::MissingFromRepo => "  missing (repo)",
                };
                println!("{:>20}  {}", marker, entry.system_path);
            }

            // Save manifest after successful apply
            result.manifest.save_default()?;
        }
        Commands::Status => {
            let manifest = Manifest::load_or_create()?;
            let repo = repo_dir(&manifest)?;

            let entries = commands::status::execute(&repo, &manifest)?;

            if entries.is_empty() {
                println!("no files tracked");
            } else {
                use commands::status::FileState;
                for entry in &entries {
                    let marker = match entry.state {
                        FileState::Unchanged => "  ok",
                        FileState::Modified => "  modified",
                        FileState::MissingFromSystem => "  missing (system)",
                        FileState::MissingFromRepo => "  missing (repo)",
                    };
                    println!("{:>20}  {}", marker, entry.system_path);
                }
            }
        }
    }

    Ok(())
}
