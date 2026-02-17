use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// OpenAI API key
    pub openai_key: Option<String>,

    /// Anthropic API key
    pub anthropic_key: Option<String>,

    /// Google AI (Gemini) API key
    pub google_key: Option<String>,

    /// Active cloud provider: "openai", "anthropic", "google"
    pub active_provider: String,

    /// Active model name
    pub active_model: String,

    /// Directories to index
    pub index_roots: Vec<String>,

    /// Max search results to display
    pub max_results: usize,
}

impl Default for Settings {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        Self {
            openai_key: None,
            anthropic_key: None,
            google_key: None,
            active_provider: "openai".to_string(),
            active_model: "gpt-4o-mini".to_string(),
            index_roots: vec![home.to_string_lossy().to_string()],
            max_results: 20,
        }
    }
}

impl Settings {
    /// Path to settings file
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("trace")
            .join("settings.json")
    }

    /// Load settings from disk, or return defaults.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Persist settings to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;

        Ok(())
    }

    /// Get the active API key for the current provider.
    pub fn active_key(&self) -> Option<&String> {
        match self.active_provider.as_str() {
            "openai" => self.openai_key.as_ref(),
            "anthropic" => self.anthropic_key.as_ref(),
            "google" => self.google_key.as_ref(),
            _ => None,
        }
    }
}
