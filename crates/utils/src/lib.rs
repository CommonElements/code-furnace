use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent_api_key: Option<String>,
    pub agent_provider: AgentProvider,
    pub log_level: String,
    pub workspace_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentProvider {
    Claude,
    OpenAI,
    Ollama { endpoint: String },
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_api_key: None,
            agent_provider: AgentProvider::Claude,
            log_level: "info".to_string(),
            workspace_path: None,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("code-furnace");
        
        let config_file = config_dir.join("config.json");
        
        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("code-furnace");
        
        std::fs::create_dir_all(&config_dir)?;
        
        let config_file = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_file, content)?;
        
        Ok(())
    }
}

pub mod paths {
    use std::path::PathBuf;
    
    pub fn get_app_data_dir() -> anyhow::Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
            .join("code-furnace");
        
        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir)
    }
    
    pub fn get_cache_dir() -> anyhow::Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
            .join("code-furnace");
        
        std::fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir)
    }
}