use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: MessageContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContext {
    pub files: Vec<String>,
    pub project_path: Option<String>,
    pub terminal_session: Option<String>,
    pub canvas_state: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for MessageContext {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            project_path: None,
            terminal_session: None,
            canvas_state: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    pub id: Uuid,
    pub name: String,
    pub messages: Vec<ConversationMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
}

impl ConversationThread {
    pub fn new(name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            archived: false,
        }
    }
    
    pub fn add_message(&mut self, role: MessageRole, content: String, context: MessageContext) {
        let message = ConversationMessage {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: chrono::Utc::now(),
            context,
        };
        
        self.messages.push(message);
        self.updated_at = chrono::Utc::now();
    }
    
    pub fn get_context_summary(&self) -> String {
        let mut context_parts = Vec::new();
        
        // Collect unique files mentioned across all messages
        let mut files = std::collections::HashSet::new();
        let mut projects = std::collections::HashSet::new();
        
        for message in &self.messages {
            files.extend(message.context.files.iter().cloned());
            if let Some(project) = &message.context.project_path {
                projects.insert(project.clone());
            }
        }
        
        if !files.is_empty() {
            context_parts.push(format!("Files discussed: {}", files.iter().take(5).map(|f| {
                std::path::Path::new(f).file_name().unwrap_or_default().to_string_lossy()
            }).collect::<Vec<_>>().join(", ")));
        }
        
        if !projects.is_empty() {
            let project_names: Vec<String> = projects.iter().take(3).map(|s| s.to_string()).collect();
            context_parts.push(format!("Projects: {}", project_names.join(", ")));
        }
        
        context_parts.join(" | ")
    }
    
    pub fn get_recent_messages(&self, limit: usize) -> Vec<&ConversationMessage> {
        self.messages.iter().rev().take(limit).collect()
    }
}

#[derive(Debug, Clone)]
pub struct AgentMemory {
    conversations: HashMap<Uuid, ConversationThread>,
    active_conversation: Option<Uuid>,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
            active_conversation: None,
        }
    }
    
    pub fn create_conversation(&mut self, name: String) -> Uuid {
        let conversation = ConversationThread::new(name);
        let id = conversation.id;
        self.conversations.insert(id, conversation);
        self.active_conversation = Some(id);
        id
    }
    
    pub fn get_conversation(&self, id: Uuid) -> Option<&ConversationThread> {
        self.conversations.get(&id)
    }
    
    pub fn get_conversation_mut(&mut self, id: Uuid) -> Option<&mut ConversationThread> {
        self.conversations.get_mut(&id)
    }
    
    pub fn get_active_conversation(&self) -> Option<&ConversationThread> {
        self.active_conversation.and_then(|id| self.conversations.get(&id))
    }
    
    pub fn get_active_conversation_mut(&mut self) -> Option<&mut ConversationThread> {
        self.active_conversation.and_then(|id| self.conversations.get_mut(&id))
    }
    
    pub fn set_active_conversation(&mut self, id: Uuid) -> Result<()> {
        if self.conversations.contains_key(&id) {
            self.active_conversation = Some(id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Conversation not found: {}", id))
        }
    }
    
    pub fn list_conversations(&self) -> Vec<&ConversationThread> {
        let mut conversations: Vec<_> = self.conversations.values().collect();
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        conversations
    }
    
    pub fn add_message_to_active(&mut self, role: MessageRole, content: String, context: MessageContext) -> Result<()> {
        if let Some(conversation) = self.get_active_conversation_mut() {
            conversation.add_message(role, content, context);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active conversation"))
        }
    }
    
    pub fn build_conversation_context(&self, conversation_id: Option<Uuid>, max_messages: usize) -> String {
        let conversation = if let Some(id) = conversation_id {
            self.conversations.get(&id)
        } else {
            self.get_active_conversation()
        };
        
        if let Some(conv) = conversation {
            let recent_messages = conv.get_recent_messages(max_messages);
            let mut context = format!("=== Conversation: {} ===\n", conv.name);
            
            for message in recent_messages.iter().rev() {
                let role_str = match message.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant", 
                    MessageRole::System => "System",
                };
                
                context.push_str(&format!("\n{}: {}\n", role_str, message.content));
                
                // Add file context if present
                if !message.context.files.is_empty() {
                    context.push_str(&format!("Files: {}\n", message.context.files.join(", ")));
                }
            }
            
            context
        } else {
            "No conversation history available.".to_string()
        }
    }
    
    pub fn search_conversations(&self, query: &str) -> Vec<&ConversationThread> {
        self.conversations
            .values()
            .filter(|conv| {
                conv.name.to_lowercase().contains(&query.to_lowercase()) ||
                conv.messages.iter().any(|msg| 
                    msg.content.to_lowercase().contains(&query.to_lowercase())
                )
            })
            .collect()
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}