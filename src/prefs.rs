use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::converter::UnitSystem;

const APP_QUALIFIER: &str = "com";
const APP_ORG: &str = "quickconvert";
const APP_NAME: &str = "QuickConvert";
const PREFS_FILE: &str = "preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultSystem {
    Imperial,
    Metric,
}

impl Default for DefaultSystem {
    fn default() -> Self {
        Self::Metric
    }
}

impl From<DefaultSystem> for UnitSystem {
    fn from(value: DefaultSystem) -> Self {
        match value {
            DefaultSystem::Imperial => UnitSystem::Imperial,
            DefaultSystem::Metric => UnitSystem::Metric,
        }
    }
}

impl From<UnitSystem> for DefaultSystem {
    fn from(value: UnitSystem) -> Self {
        match value {
            UnitSystem::Imperial => Self::Imperial,
            UnitSystem::Metric => Self::Metric,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_decimal_places")]
    pub decimal_places: u32,
    #[serde(default)]
    pub default_system: DefaultSystem,
    #[serde(default)]
    pub default_imperial: bool,
    #[serde(default = "default_metric_flag")]
    pub default_metric: bool,
}

fn default_decimal_places() -> u32 {
    2
}

fn default_metric_flag() -> bool {
    true
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            decimal_places: 2,
            default_system: DefaultSystem::Metric,
            default_imperial: false,
            default_metric: true,
        }
    }
}

impl Preferences {
    pub fn clamp_decimal_places(&mut self) {
        self.decimal_places = self.decimal_places.clamp(2, 6);
    }

    pub fn config_dir() -> Result<PathBuf> {
        ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
            .map(|dirs| dirs.config_dir().to_path_buf())
            .context("Could not resolve config directory")
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(PREFS_FILE))
    }

    pub fn bundled_default() -> Result<Self> {
        let raw = include_str!("../data/preferences.json");
        serde_json::from_str(raw).context("Invalid bundled preferences.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let mut prefs: Self = serde_json::from_str(&raw)
                .with_context(|| format!("Failed to parse {}", path.display()))?;
            prefs.clamp_decimal_places();
            prefs.sync_default_flags();
            return Ok(prefs);
        }

        let mut prefs = Self::bundled_default().unwrap_or_default();
        prefs.clamp_decimal_places();
        prefs.save()?;
        Ok(prefs)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let path = dir.join(PREFS_FILE);
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(&path, raw)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn sync_default_flags(&mut self) {
        match self.default_system {
            DefaultSystem::Imperial => {
                self.default_imperial = true;
                self.default_metric = false;
            }
            DefaultSystem::Metric => {
                self.default_imperial = false;
                self.default_metric = true;
            }
        }
    }

    pub fn set_default_system(&mut self, system: DefaultSystem) {
        self.default_system = system;
        self.sync_default_flags();
    }
}
