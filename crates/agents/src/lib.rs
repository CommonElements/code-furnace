use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub mod memory;
pub mod specialized;

pub use memory::*;
pub use specialized::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: Uuid,
    pub agent_type: String,
    pub prompt: String,
    pub context: HashMap<String, serde_json::Value>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub request_id: Uuid,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}

#[async_trait::async_trait]
pub trait AgentProvider: Send + Sync {
    async fn process_request(&self, request: &AgentRequest) -> Result<AgentResponse>;
    fn provider_name(&self) -> &str;
    fn supports_streaming(&self) -> bool { false }
}

pub struct ClaudeProvider {
    api_key: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

pub struct OpenAIProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AgentProvider for ClaudeProvider {
    async fn process_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let mut prompt = request.prompt.clone();
        
        // Add file context if provided
        if !request.files.is_empty() {
            prompt.push_str("\n\nFile context:\n");
            for file_path in &request.files {
                if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                    prompt.push_str(&format!("File: {}\n```\n{}\n```\n\n", file_path, content));
                }
            }
        }
        
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000
        });
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            let claude_response: serde_json::Value = response.json().await?;
            let content = claude_response["content"][0]["text"]
                .as_str()
                .unwrap_or("No response")
                .to_string();
            
            Ok(AgentResponse {
                request_id: request.id,
                content,
                metadata: HashMap::new(),
                error: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(AgentResponse {
                request_id: request.id,
                content: String::new(),
                metadata: HashMap::new(),
                error: Some(format!("API Error: {}", error_text)),
            })
        }
    }
    
    fn provider_name(&self) -> &str {
        "claude"
    }
}

#[async_trait::async_trait]
impl AgentProvider for OpenAIProvider {
    async fn process_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let mut messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a helpful AI assistant integrated into Code Furnace, a powerful development environment. Help users with coding, debugging, and development tasks."
            }),
        ];
        
        // Add file context if provided
        if !request.files.is_empty() {
            let mut context_content = request.prompt.clone();
            context_content.push_str("\n\nFile context:\n");
            for file_path in &request.files {
                if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                    context_content.push_str(&format!("File: {}\n```\n{}\n```\n\n", file_path, content));
                }
            }
            messages.push(serde_json::json!({
                "role": "user",
                "content": context_content
            }));
        } else {
            messages.push(serde_json::json!({
                "role": "user",
                "content": request.prompt
            }));
        }
        
        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": messages,
            "max_tokens": 4000,
            "temperature": 0.7
        });
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            let openai_response: serde_json::Value = response.json().await?;
            let content = openai_response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("No response")
                .to_string();
            
            Ok(AgentResponse {
                request_id: request.id,
                content,
                metadata: HashMap::new(),
                error: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(AgentResponse {
                request_id: request.id,
                content: String::new(),
                metadata: HashMap::new(),
                error: Some(format!("OpenAI API Error: {}", error_text)),
            })
        }
    }
    
    fn provider_name(&self) -> &str {
        "openai"
    }
}

pub struct AgentBridge {
    providers: HashMap<String, Box<dyn AgentProvider>>,
    default_provider: String,
    memory: AgentMemory,
    router: AgentRouter,
}

impl AgentBridge {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: "claude".to_string(),
            memory: AgentMemory::new(),
            router: AgentRouter::new(),
        }
    }
    
    pub fn register_provider(&mut self, name: String, provider: Box<dyn AgentProvider>) {
        self.providers.insert(name, provider);
    }
    
    pub fn set_default_provider(&mut self, name: String) {
        self.default_provider = name;
    }
    
    pub async fn process_request(&mut self, request: AgentRequest) -> Result<AgentResponse> {
        // Add user message to memory with context
        let context = memory::MessageContext {
            files: request.files.clone(),
            project_path: request.context.get("project_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            terminal_session: request.context.get("terminal_session")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            canvas_state: request.context.get("canvas_state")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            metadata: request.context.clone(),
        };
        
        // Create or use active conversation
        if self.memory.get_active_conversation().is_none() {
            self.memory.create_conversation("Default Conversation".to_string());
        }
        
        self.memory.add_message_to_active(
            memory::MessageRole::User,
            request.prompt.clone(),
            context.clone(),
        )?;
        
        // Build conversation context for the request
        let conversation_context = self.memory.build_conversation_context(None, 10);
        
        // Enhanced request with conversation history
        let mut enhanced_request = request.clone();
        if !conversation_context.is_empty() {
            enhanced_request.prompt = format!(
                "Conversation History:\n{}\n\n---\n\nCurrent Request:\n{}",
                conversation_context,
                request.prompt
            );
        }
        
        // Use router to determine the best agent, fallback to specified or default
        let provider = if !request.agent_type.is_empty() {
            // Use specified agent type
            self.providers.get(&request.agent_type)
        } else {
            // Use router to auto-select agent
            self.router.route_request(&request)
                .ok()
                .and_then(|_| {
                    let agent_name = self.router.determine_agent_for_request(&request);
                    self.providers.get(&agent_name)
                })
                .or_else(|| self.providers.get(&self.default_provider))
        };
        
        if let Some(provider) = provider {
            let response = provider.process_request(&enhanced_request).await?;
            
            // Add assistant response to memory
            if response.error.is_none() {
                self.memory.add_message_to_active(
                    memory::MessageRole::Assistant,
                    response.content.clone(),
                    context,
                )?;
            }
            
            Ok(response)
        } else {
            let error_msg = format!("No suitable agent provider found for request");
            Ok(AgentResponse {
                request_id: request.id,
                content: String::new(),
                metadata: HashMap::new(),
                error: Some(error_msg),
            })
        }
    }
    
    // Memory management methods
    pub fn create_conversation(&mut self, name: String) -> Uuid {
        self.memory.create_conversation(name)
    }
    
    pub fn get_conversation(&self, id: Uuid) -> Option<&ConversationThread> {
        self.memory.get_conversation(id)
    }
    
    pub fn get_active_conversation(&self) -> Option<&ConversationThread> {
        self.memory.get_active_conversation()
    }
    
    pub fn set_active_conversation(&mut self, id: Uuid) -> Result<()> {
        self.memory.set_active_conversation(id)
    }
    
    pub fn list_conversations(&self) -> Vec<&ConversationThread> {
        self.memory.list_conversations()
    }
    
    pub fn search_conversations(&self, query: &str) -> Vec<&ConversationThread> {
        self.memory.search_conversations(query)
    }
    
    // Router management
    pub fn register_specialized_agent(&mut self, agent_type: AgentType, base_provider: Box<dyn AgentProvider>) {
        let specialized = SpecializedAgent::new(agent_type.clone(), base_provider);
        let name = specialized.provider_name().to_string();
        self.providers.insert(name.clone(), Box::new(specialized));
        // Note: Router will determine agent by name during routing
    }
    
    pub fn list_available_agents(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for AgentBridge {
    fn default() -> Self {
        Self::new()
    }
}