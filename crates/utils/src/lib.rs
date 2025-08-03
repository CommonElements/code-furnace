use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent_api_key: Option<String>,
    pub agent_provider: AgentProvider,
    pub log_level: String,
    pub workspace_path: Option<PathBuf>,
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub gitea_token: Option<String>,
    pub auto_save: bool,
    pub theme: String,
    pub font_size: u32,
    pub enable_lsp: bool,
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
            github_token: None,
            gitlab_token: None,
            gitea_token: None,
            auto_save: true,
            theme: "dark".to_string(),
            font_size: 14,
            enable_lsp: true,
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
    
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate agent configuration
        if let Some(ref api_key) = self.agent_api_key {
            if api_key.is_empty() {
                return Err(anyhow::anyhow!("Agent API key cannot be empty"));
            }
            
            match self.agent_provider {
                AgentProvider::Claude => {
                    if !api_key.starts_with("sk-ant-") {
                        return Err(anyhow::anyhow!("Invalid Claude API key format"));
                    }
                }
                AgentProvider::OpenAI => {
                    if !api_key.starts_with("sk-") {
                        return Err(anyhow::anyhow!("Invalid OpenAI API key format"));
                    }
                }
                AgentProvider::Ollama { .. } => {
                    // Ollama typically doesn't require API keys for local instances
                }
            }
        }
        
        // Validate font size
        if self.font_size < 8 || self.font_size > 72 {
            return Err(anyhow::anyhow!("Font size must be between 8 and 72"));
        }
        
        // Validate theme
        if !["dark", "light", "auto"].contains(&self.theme.as_str()) {
            return Err(anyhow::anyhow!("Theme must be 'dark', 'light', or 'auto'"));
        }
        
        Ok(())
    }
    
    pub fn has_agent_configured(&self) -> bool {
        self.agent_api_key.is_some() && !self.agent_api_key.as_ref().unwrap().is_empty()
    }
    
    pub fn has_git_tokens(&self) -> bool {
        self.github_token.is_some() || self.gitlab_token.is_some() || self.gitea_token.is_some()
    }
    
    pub fn update_agent_config(&mut self, provider: AgentProvider, api_key: Option<String>) -> anyhow::Result<()> {
        self.agent_provider = provider;
        self.agent_api_key = api_key;
        self.validate()?;
        self.save()?;
        Ok(())
    }
    
    pub fn update_git_tokens(&mut self, github: Option<String>, gitlab: Option<String>, gitea: Option<String>) -> anyhow::Result<()> {
        self.github_token = github;
        self.gitlab_token = gitlab;
        self.gitea_token = gitea;
        self.save()?;
        Ok(())
    }
    
    pub fn update_ui_preferences(&mut self, theme: String, font_size: u32, auto_save: bool, enable_lsp: bool) -> anyhow::Result<()> {
        self.theme = theme;
        self.font_size = font_size;
        self.auto_save = auto_save;
        self.enable_lsp = enable_lsp;
        self.validate()?;
        self.save()?;
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