use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub files: BTreeMap<String, String>,
}

impl Manifest {
    /// Returns the default config path: ~/.config/rootrat/rootrat.toml
    /// Always uses ~/.config/ regardless of OS.
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().expect("could not determine home directory");
        home.join(".config").join("rootrat").join("rootrat.toml")
    }

    /// Load from the default path, or create a new manifest if it doesn't exist yet.
    pub fn load_or_create() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::new())
        }
    }

    /// Save to the default path, creating parent dirs if needed.
    pub fn save_default(&self) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.save(&path)
    }

    /// Create a new empty manifest.
    pub fn new() -> Self {
        Self {
            repo: None,
            files: BTreeMap::new(),
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

    /// Add a file mapping. `system_path` is the absolute path on the machine.
    /// Returns the derived repo path.
    pub fn add(&mut self, system_path: &Path) -> Result<String> {
        let repo_path = Self::derive_repo_path(system_path)?;
        let display_path = Self::to_display_path(system_path)?;
        self.files.insert(repo_path.clone(), display_path);
        Ok(repo_path)
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
