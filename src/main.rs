mod commands;
mod manifest;

use anyhow::Result;
use clap::{Parser, Subcommand};
use manifest::{LocalConfig, Manifest};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rootrat", about = "A dotfiles manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a file or directory to rootrat tracking
    Add {
        /// Path to the file or directory to track
        path: String,
    },
    /// Apply tracked files from repo to system
    Apply,
    /// Collect system changes into the repo
    Collect,
    /// Show differences between repo and system files
    Diff {
        /// Only show diff for this file
        path: Option<String>,
    },
    /// Remove a file or directory from rootrat tracking
    Rm {
        /// Path to the file or directory to stop tracking
        path: String,
    },
    /// Initialize rootrat (optionally clone from a git URL)
    Init {
        /// Git URL to clone from (e.g. github.com/user/dotfiles)
        url: Option<String>,
    },
    /// Pull changes from remote
    Pull,
    /// Push changes to remote
    Push,
    /// Show status of tracked files
    Status,
    /// Sync: collect, pull, push, apply
    Sync,
}

/// Expand ~ and resolve to an absolute path. Fails if the path doesn't exist.
fn resolve_path(path: &str) -> Result<PathBuf> {
    Ok(std::fs::canonicalize(Manifest::expand_tilde(path))?)
}

/// Prompt the user with a [y/N] question, returns true if they answer yes.
fn prompt_yn(message: &str) -> Result<bool> {
    print!("{} [y/N] ", message);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

/// Load local config and manifest from the repo.
fn load_config_and_manifest() -> Result<(PathBuf, Manifest)> {
    let config = LocalConfig::load_default()?;
    let repo = config.repo_dir();
    let manifest = Manifest::load_from_repo(&repo)?;
    Ok((repo, manifest))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { path } => {
            let system_path = resolve_path(&path)?;
            let (repo, mut manifest) = load_config_and_manifest()?;

            commands::add::execute(&system_path, &repo, &mut manifest)?;
            manifest.save_to_repo(&repo)?;
            commands::git_commit(&repo, &format!("add: {}", system_path.display()))?;

            println!("added: {}", system_path.display());
        }
        Commands::Apply => {
            let (repo, manifest) = load_config_and_manifest()?;

            use commands::apply::ApplyState;
            let entries = commands::apply::plan(&repo, &manifest)?;

            if entries.is_empty() {
                println!("no files tracked");
                return Ok(());
            }

            let created: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Created).collect();
            let updated: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Updated).collect();
            let deleted: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Deleted).collect();
            let missing: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::MissingFromRepo).collect();
            let unchanged_count = entries.iter().filter(|e| e.state == ApplyState::Unchanged).count();

            if created.is_empty() && updated.is_empty() && deleted.is_empty() && missing.is_empty() {
                println!("all {} files up to date", unchanged_count);
                return Ok(());
            }

            if !created.is_empty() {
                println!("  create:");
                for entry in &created {
                    println!("    {}", entry.system_path);
                }
            }
            if !updated.is_empty() {
                println!("  modify:");
                for entry in &updated {
                    println!("    {}", entry.system_path);
                }
            }
            if !deleted.is_empty() {
                println!("  delete:");
                for entry in &deleted {
                    println!("    {}", entry.system_path);
                }
            }
            if !missing.is_empty() {
                println!("  missing (repo):");
                for entry in &missing {
                    println!("    {}", entry.system_path);
                }
            }
            if unchanged_count > 0 {
                println!("  unchanged: {}", unchanged_count);
            }

            println!();
            if !prompt_yn("proceed?")? {
                println!("aborted");
                return Ok(());
            }

            commands::apply::apply_entries(&entries)?;
            println!("done");
        }
        Commands::Collect => {
            let (repo, manifest) = load_config_and_manifest()?;

            use commands::collect::CollectState;
            let entries = commands::collect::plan(&repo, &manifest)?;

            if entries.is_empty() {
                println!("no files tracked");
                return Ok(());
            }

            let created: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Created).collect();
            let updated: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Updated).collect();
            let deleted: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Deleted).collect();
            let missing: Vec<_> = entries.iter().filter(|e| e.state == CollectState::MissingFromSystem).collect();
            let unchanged_count = entries.iter().filter(|e| e.state == CollectState::Unchanged).count();

            if created.is_empty() && updated.is_empty() && deleted.is_empty() && missing.is_empty() {
                println!("all {} files up to date", unchanged_count);
                return Ok(());
            }

            if !created.is_empty() {
                println!("  create:");
                for entry in &created {
                    println!("    {}", entry.system_path);
                }
            }
            if !updated.is_empty() {
                println!("  update:");
                for entry in &updated {
                    println!("    {}", entry.system_path);
                }
            }
            if !deleted.is_empty() {
                println!("  delete:");
                for entry in &deleted {
                    println!("    {}", entry.system_path);
                }
            }
            if !missing.is_empty() {
                println!("  missing (system):");
                for entry in &missing {
                    println!("    {}", entry.system_path);
                }
            }
            if unchanged_count > 0 {
                println!("  unchanged: {}", unchanged_count);
            }

            println!();
            if !prompt_yn("proceed?")? {
                println!("aborted");
                return Ok(());
            }

            commands::collect::collect_entries(&entries)?;
            commands::git_commit(&repo, "collect")?;
            println!("done");
        }
        Commands::Diff { path } => {
            let (repo, manifest) = load_config_and_manifest()?;

            let entries = commands::diff::execute(&repo, &manifest, path.as_deref())?;

            if entries.is_empty() {
                println!("no differences");
            } else {
                for entry in &entries {
                    println!("\x1b[1m--- {}\x1b[0m", entry.system_path);
                    print!("{}", entry.diff);
                    println!();
                }
            }
        }
        Commands::Rm { path } => {
            let expanded = Manifest::expand_tilde(&path);
            let system_path = std::fs::canonicalize(&expanded).unwrap_or(expanded);
            let (repo, mut manifest) = load_config_and_manifest()?;

            commands::rm::execute(&system_path, &repo, &mut manifest)?;
            manifest.save_to_repo(&repo)?;
            commands::git_commit(&repo, &format!("rm: {}", system_path.display()))?;

            println!("removed: {}", system_path.display());
        }
        Commands::Init { url: None } => {
            let dir = std::env::current_dir()?;
            let config = commands::init::execute(&dir)?;
            config.save_default()?;

            // Create empty manifest in repo if it doesn't exist yet
            let manifest_path = dir.join("rootrat.toml");
            if !manifest_path.exists() {
                Manifest::new().save(&manifest_path)?;
            }

            commands::git_init(&dir)?;
            commands::git_commit(&dir, "init")?;

            println!("initialized rootrat repo at: {}", dir.display());
        }
        Commands::Init { url: Some(url) } => {
            let dir = std::env::current_dir()?;
            let result = commands::init::clone_and_init(&url, &dir)?;
            result.config.save_default()?;

            println!("cloned to: {}", result.repo_dir.display());
            println!("run `rootrat apply` to apply tracked files");
        }
        Commands::Pull => {
            let config = LocalConfig::load_default()?;
            let repo = config.repo_dir();

            let count = match commands::git_pull(&repo) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("pull failed: {}", e);
                    if prompt_yn("rebase?")? {
                        commands::git_pull_rebase(&repo)?
                    } else {
                        anyhow::bail!("pull failed");
                    }
                }
            };

            if count == 0 {
                println!("already up to date");
            } else {
                println!("pulled {} commit{}", count, if count == 1 { "" } else { "s" });
            }
        }
        Commands::Push => {
            let config = LocalConfig::load_default()?;
            let repo = config.repo_dir();

            let count = commands::git_push(&repo)?;
            if count == 0 {
                println!("already up to date");
            } else {
                println!("pushed {} commit{}", count, if count == 1 { "" } else { "s" });
            }
        }
        Commands::Status => {
            let (repo, manifest) = load_config_and_manifest()?;

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
        Commands::Sync => {
            let (repo, manifest) = load_config_and_manifest()?;

            // -- Phase 1: collect (system -> repo) --
            {
                use commands::collect::CollectState;
                let entries = commands::collect::plan(&repo, &manifest)?;
                let has_changes = !entries.is_empty()
                    && entries.iter().any(|e| {
                        !matches!(e.state, CollectState::Unchanged)
                    });

                if !has_changes {
                    println!("collect: nothing to do");
                } else {
                    println!("collect:");
                    let created: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Created).collect();
                    let updated: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Updated).collect();
                    let deleted: Vec<_> = entries.iter().filter(|e| e.state == CollectState::Deleted).collect();
                    let missing: Vec<_> = entries.iter().filter(|e| e.state == CollectState::MissingFromSystem).collect();

                    if !created.is_empty() {
                        println!("  create:");
                        for entry in &created {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !updated.is_empty() {
                        println!("  update:");
                        for entry in &updated {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !deleted.is_empty() {
                        println!("  delete:");
                        for entry in &deleted {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !missing.is_empty() {
                        println!("  missing (system):");
                        for entry in &missing {
                            println!("    {}", entry.system_path);
                        }
                    }

                    println!();
                    if prompt_yn("proceed with collect?")? {
                        commands::collect::collect_entries(&entries)?;
                        commands::git_commit(&repo, "collect")?;
                        println!("collected");
                    } else if !prompt_yn("skip collect and continue sync?")? {
                        println!("aborted");
                        return Ok(());
                    }
                }
            }

            // -- Phase 2: git pull --
            println!();
            let pull_count = match commands::git_pull(&repo) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("pull failed: {}", e);
                    if prompt_yn("rebase?")? {
                        commands::git_pull_rebase(&repo)?
                    } else {
                        anyhow::bail!("pull failed, aborting sync");
                    }
                }
            };
            if pull_count == 0 {
                println!("pull: already up to date");
            } else {
                println!("pulled {} commit{}", pull_count, if pull_count == 1 { "" } else { "s" });
            }

            // -- Phase 3: git push --
            println!();
            let push_count = commands::git_push(&repo)?;
            if push_count == 0 {
                println!("push: already up to date");
            } else {
                println!("pushed {} commit{}", push_count, if push_count == 1 { "" } else { "s" });
            }

            // -- Phase 4: apply (repo -> system) --
            // Reload manifest since remote may have changed it
            let manifest = Manifest::load_from_repo(&repo)?;
            {
                use commands::apply::ApplyState;
                let entries = commands::apply::plan(&repo, &manifest)?;
                let has_changes = !entries.is_empty()
                    && entries.iter().any(|e| {
                        !matches!(e.state, ApplyState::Unchanged)
                    });

                if !has_changes {
                    println!();
                    println!("apply: nothing to do");
                } else {
                    println!();
                    println!("apply:");
                    let created: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Created).collect();
                    let updated: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Updated).collect();
                    let deleted: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::Deleted).collect();
                    let missing: Vec<_> = entries.iter().filter(|e| e.state == ApplyState::MissingFromRepo).collect();

                    if !created.is_empty() {
                        println!("  create:");
                        for entry in &created {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !updated.is_empty() {
                        println!("  modify:");
                        for entry in &updated {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !deleted.is_empty() {
                        println!("  delete:");
                        for entry in &deleted {
                            println!("    {}", entry.system_path);
                        }
                    }
                    if !missing.is_empty() {
                        println!("  missing (repo):");
                        for entry in &missing {
                            println!("    {}", entry.system_path);
                        }
                    }

                    println!();
                    if prompt_yn("proceed with apply?")? {
                        commands::apply::apply_entries(&entries)?;
                        println!("applied");
                    } else {
                        println!("skipped apply (remote is synced)");
                    }
                }
            }

            println!();
            println!("sync complete");
        }
    }

    Ok(())
}
