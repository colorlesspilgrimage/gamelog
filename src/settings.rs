use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::gamelog_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverSource {
    SteamGridDb,
    Rawg,
    Manual,
}

impl Default for CoverSource {
    fn default() -> Self {
        CoverSource::Manual
    }
}

impl CoverSource {
    pub fn label(self) -> &'static str {
        match self {
            CoverSource::SteamGridDb => "SteamGridDB",
            CoverSource::Rawg => "RAWG.io",
            CoverSource::Manual => "Manual",
        }
    }

    pub fn needs_api_key(self) -> bool {
        matches!(self, CoverSource::SteamGridDb | CoverSource::Rawg)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub default_cover_source: CoverSource,
    pub steamgriddb_api_key: Option<String>,
    pub rawg_api_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_cover_source: CoverSource::Manual,
            steamgriddb_api_key: None,
            rawg_api_key: None,
        }
    }
}

impl Settings {
    fn path() -> Result<PathBuf> {
        Ok(gamelog_dir()?.join("settings.toml"))
    }

    /// Loads settings from disk. Returns `Ok(None)` if no settings file
    /// exists yet, so the caller can run the first-run setup wizard.
    pub fn load() -> Result<Option<Self>> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(None);
        }
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let settings: Self =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(settings))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating config directory at {}", parent.display()))?;
        }
        let toml = toml::to_string_pretty(self)?;
        fs::write(&path, toml).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn api_key_for(&self, source: CoverSource) -> Option<&str> {
        match source {
            CoverSource::SteamGridDb => self.steamgriddb_api_key.as_deref(),
            CoverSource::Rawg => self.rawg_api_key.as_deref(),
            CoverSource::Manual => None,
        }
    }

    pub fn set_api_key(&mut self, source: CoverSource, key: String) {
        match source {
            CoverSource::SteamGridDb => self.steamgriddb_api_key = Some(key),
            CoverSource::Rawg => self.rawg_api_key = Some(key),
            CoverSource::Manual => {}
        }
    }
}
