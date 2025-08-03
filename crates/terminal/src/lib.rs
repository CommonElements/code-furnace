use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use uuid::Uuid;
use std::process::Stdio;

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
    
    pub fn get_latest_block_mut(&mut self) -> Option<&mut TerminalBlock> {
        self.blocks.last_mut()
    }
}

#[derive(Debug)]
pub struct ActiveTerminal {
    pub child: Arc<Mutex<Option<Child>>>,
    pub stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pub output_tx: mpsc::UnboundedSender<String>,
    pub input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
}

pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, TerminalSession>>>,
    active_terminals: Arc<RwLock<HashMap<Uuid, ActiveTerminal>>>,
    event_bus: code_furnace_events::EventBus,
}

impl TerminalManager {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_terminals: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        }
    }
    
    pub async fn create_session(&self, name: String, working_directory: std::path::PathBuf) -> Result<Uuid> {
        let session = TerminalSession::new(name, working_directory.clone());
        let session_id = session.id;
        
        // Set up communication channels
        let (output_tx, _output_rx) = mpsc::unbounded_channel::<String>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        
        let active_terminal = ActiveTerminal {
            child: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            output_tx: output_tx.clone(),
            input_rx: Arc::new(Mutex::new(input_rx)),
        };
        
        // Store session and active terminal
        let mut sessions = self.sessions.write().await;
        let mut active_terminals = self.active_terminals.write().await;
        
        sessions.insert(session_id, session);
        active_terminals.insert(session_id, active_terminal);
        
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
            let start_time = chrono::Utc::now();
            
            // Execute command using tokio process with better output handling
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            
            let mut child = Command::new(&shell)
                .arg("-c")
                .arg(&command)
                .current_dir(&session.working_directory)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            
            let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;
            let stderr = child.stderr.take().ok_or_else(|| anyhow::anyhow!("Failed to get stderr"))?;
            
            // Read stdout and stderr concurrently
            let stdout_task = async {
                let mut stdout_reader = BufReader::new(stdout);
                let mut output = String::new();
                let mut line = String::new();
                while stdout_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    output.push_str(&line);
                    line.clear();
                }
                output
            };
            
            let stderr_task = async {
                let mut stderr_reader = BufReader::new(stderr);
                let mut output = String::new();
                let mut line = String::new();
                while stderr_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    output.push_str(&line);
                    line.clear();
                }
                output
            };
            
            // Wait for both output streams and process completion
            let (stdout_output, stderr_output) = tokio::join!(stdout_task, stderr_task);
            let exit_status = child.wait().await?;
            
            // Combine outputs
            block.output = stdout_output;
            if !stderr_output.is_empty() {
                block.output.push_str(&format!("\nSTDERR:\n{}", stderr_output));
            }
            
            block.exit_code = exit_status.code();
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
                    "exit_code": exit_status.code()
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
    
    pub async fn send_input(&self, session_id: Uuid, input: String) -> Result<()> {
        let active_terminals = self.active_terminals.read().await;
        
        if let Some(active_terminal) = active_terminals.get(&session_id) {
            // For now, just store the input - in a full PTY implementation this would send to stdin
            let _ = active_terminal.output_tx.send(format!("Input received: {}", input));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Active terminal not found: {}", session_id))
        }
    }
    
    pub async fn resize_terminal(&self, session_id: Uuid, cols: u16, rows: u16) -> Result<()> {
        // For now, just acknowledge the resize - in a full PTY implementation this would resize the terminal
        let event = code_furnace_events::Event::new(
            "terminal.resized",
            "terminal-manager",
            serde_json::json!({
                "session_id": session_id,
                "cols": cols,
                "rows": rows
            }),
        );
        self.event_bus.publish(event)?;
        Ok(())
    }
    
    pub async fn close_session(&self, session_id: Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let mut active_terminals = self.active_terminals.write().await;
        
        // Mark session as inactive
        if let Some(session) = sessions.get_mut(&session_id) {
            session.active = false;
        }
        
        // Clean up active terminal
        if let Some(_active_terminal) = active_terminals.remove(&session_id) {
            // Terminal resources will be dropped and cleaned up automatically
        }
        
        // Publish session closed event
        let event = code_furnace_events::Event::new(
            "terminal.session.closed",
            "terminal-manager",
            serde_json::to_value(&session_id)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(())
    }
}