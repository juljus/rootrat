use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

// -- LocalConfig: repo pointer at ~/.config/rootrat/rootrat.toml --

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LocalConfig {
    pub repo: String,
}

impl LocalConfig {
    /// Returns the default config path: ~/.config/rootrat/rootrat.toml
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().expect("could not determine home directory");
        home.join(".config").join("rootrat").join("rootrat.toml")
    }

    /// Load from a specific path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from the default path, or bail if not initialized.
    pub fn load_default() -> Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            bail!("not initialized -- run `rootrat init` first");
        }
        Self::load(&path)
    }

    /// Save to a specific path, creating parent dirs if needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Save to the default path.
    pub fn save_default(&self) -> Result<()> {
        self.save(&Self::default_path())
    }

    /// Resolve the repo path to an absolute PathBuf.
    pub fn repo_dir(&self) -> PathBuf {
        Manifest::expand_tilde(&self.repo)
    }
}

// -- Manifest: file mappings at <repo>/rootrat.toml --

fn default_ignore() -> Vec<String> {
    vec![
        ".DS_Store".to_string(),
        "Thumbs.db".to_string(),
        ".git".to_string(),
    ]
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directories: BTreeMap<String, String>,
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
}

impl Manifest {
    /// Create a new manifest with default ignore list.
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            directories: BTreeMap::new(),
            ignore: default_ignore(),
        }
    }

    /// Load a manifest from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let manifest: Self = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Save the manifest to a TOML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load the manifest from `<repo_dir>/rootrat.toml`, or return empty if not found.
    pub fn load_from_repo(repo_dir: &Path) -> Result<Self> {
        let path = repo_dir.join("rootrat.toml");
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::new())
        }
    }

    /// Save the manifest to `<repo_dir>/rootrat.toml`.
    pub fn save_to_repo(&self, repo_dir: &Path) -> Result<()> {
        self.save(&repo_dir.join("rootrat.toml"))
    }

    /// Add a file or directory mapping. `system_path` is the absolute path on the machine.
    /// Automatically detects whether the path is a directory or file.
    /// Returns the derived repo path.
    pub fn add(&mut self, system_path: &Path) -> Result<String> {
        let repo_path = Self::derive_repo_path(system_path)?;
        let display_path = Self::to_display_path(system_path)?;
        if system_path.is_dir() {
            self.directories.insert(repo_path.clone(), display_path);
        } else {
            self.files.insert(repo_path.clone(), display_path);
        }
        Ok(repo_path)
    }

    /// Remove a file or directory mapping by system path.
    /// Returns the repo path that was removed.
    pub fn remove(&mut self, system_path: &Path) -> Result<String> {
        let repo_path = Self::derive_repo_path(system_path)?;

        if self.files.remove(&repo_path).is_some() {
            return Ok(repo_path);
        }
        if self.directories.remove(&repo_path).is_some() {
            return Ok(repo_path);
        }

        bail!("not tracked: {}", system_path.display())
    }

    /// Derive the repo path from a system path.
    /// ~/.config/ghostty/config -> home/.config/ghostty/config
    /// /etc/some-config         -> system/etc/some-config
    pub fn derive_repo_path(system_path: &Path) -> Result<String> {
        if !system_path.is_absolute() {
            bail!("path must be absolute: {}", system_path.display());
        }

        let home = dirs::home_dir().expect("could not determine home directory");

        if let Ok(relative) = system_path.strip_prefix(&home) {
            Ok(format!("home/{}", relative.display()))
        } else {
            // Strip the leading "/" for system paths
            let without_root = system_path
                .strip_prefix("/")
                .expect("absolute path should start with /");
            Ok(format!("system/{}", without_root.display()))
        }
    }

    /// Expand ~ to the home directory in a path string.
    pub fn expand_tilde(path: &str) -> PathBuf {
        if path.starts_with('~') {
            let home = dirs::home_dir().expect("could not determine home directory");
            home.join(path.strip_prefix("~/").unwrap_or(&path[1..]))
        } else {
            PathBuf::from(path)
        }
    }

    /// Convert a system path to its display form.
    /// Home paths become ~/... , system paths stay absolute.
    pub fn to_display_path(system_path: &Path) -> Result<String> {
        let home = dirs::home_dir().expect("could not determine home directory");

        if let Ok(relative) = system_path.strip_prefix(&home) {
            Ok(format!("~/{}", relative.display()))
        } else {
            Ok(system_path.display().to_string())
        }
    }
}

