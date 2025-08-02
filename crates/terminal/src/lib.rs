use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalBlock {
    pub id: Uuid,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub working_directory: std::path::PathBuf,
    pub environment: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration: Option<chrono::Duration>,
}

impl TerminalBlock {
    pub fn new(command: String, working_directory: std::path::PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            command,
            output: String::new(),
            exit_code: None,
            working_directory,
            environment: std::env::vars().collect(),
            timestamp: chrono::Utc::now(),
            duration: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: Uuid,
    pub name: String,
    pub working_directory: std::path::PathBuf,
    pub shell: String,
    pub blocks: Vec<TerminalBlock>,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TerminalSession {
    pub fn new(name: String, working_directory: std::path::PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            working_directory,
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            blocks: Vec::new(),
            active: true,
            created_at: chrono::Utc::now(),
        }
    }
    
    pub fn add_block(&mut self, block: TerminalBlock) {
        self.blocks.push(block);
    }
    
    pub fn get_latest_block(&self) -> Option<&TerminalBlock> {
        self.blocks.last()
    }
}

pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, TerminalSession>>>,
    event_bus: code_furnace_events::EventBus,
}

impl TerminalManager {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        }
    }
    
    pub async fn create_session(&self, name: String, working_directory: std::path::PathBuf) -> Result<Uuid> {
        let session = TerminalSession::new(name, working_directory);
        let session_id = session.id;
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);
        
        // Publish session created event
        let event = code_furnace_events::Event::new(
            "terminal.session.created",
            "terminal-manager",
            serde_json::to_value(&session_id)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(session_id)
    }
    
    pub async fn execute_command(&self, session_id: Uuid, command: String) -> Result<Uuid> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            let mut block = TerminalBlock::new(command.clone(), session.working_directory.clone());
            
            // Execute command (simplified for now - in full implementation would use pty-process)
            let start_time = chrono::Utc::now();
            
            let output = tokio::process::Command::new(&session.shell)
                .arg("-c")
                .arg(&command)
                .current_dir(&session.working_directory)
                .output()
                .await?;
            
            block.output = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.stderr.is_empty() {
                block.output.push_str(&format!("\nSTDERR:\n{}", String::from_utf8_lossy(&output.stderr)));
            }
            
            block.exit_code = output.status.code();
            block.duration = Some(chrono::Utc::now() - start_time);
            
            let block_id = block.id;
            session.add_block(block);
            
            // Publish command executed event
            let event = code_furnace_events::Event::new(
                "terminal.command.executed",
                "terminal-manager",
                serde_json::json!({
                    "session_id": session_id,
                    "block_id": block_id,
                    "command": command,
                    "exit_code": output.status.code()
                }),
            );
            self.event_bus.publish(event)?;
            
            Ok(block_id)
        } else {
            Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }
    
    pub async fn get_session(&self, session_id: Uuid) -> Option<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).cloned()
    }
    
    pub async fn list_sessions(&self) -> Vec<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }
}