use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tokio::process::{Child, Command};
use tokio::io::{BufReader, AsyncBufReadExt};
use std::process::Stdio;

pub mod git;
pub use git::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub project_type: ProjectType,
    pub config: ProjectConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_opened: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    RustCargo,
    NodeJs,
    Python,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub dev_command: Option<String>,
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub ports: Vec<u16>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            dev_command: None,
            build_command: None,
            test_command: None,
            env_vars: HashMap::new(),
            ports: Vec::new(),
        }
    }
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        let project_type = Self::detect_project_type(&path);
        let config = Self::generate_default_config(&project_type, &path);
        
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            path,
            project_type,
            config,
            created_at: now,
            last_opened: now,
        }
    }
    
    fn detect_project_type(path: &PathBuf) -> ProjectType {
        if path.join("Cargo.toml").exists() {
            ProjectType::RustCargo
        } else if path.join("package.json").exists() {
            ProjectType::NodeJs
        } else if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
            ProjectType::Python
        } else {
            ProjectType::Generic
        }
    }
    
    fn generate_default_config(project_type: &ProjectType, path: &PathBuf) -> ProjectConfig {
        let mut config = ProjectConfig::default();
        
        match project_type {
            ProjectType::RustCargo => {
                config.dev_command = Some("cargo run".to_string());
                config.build_command = Some("cargo build".to_string());
                config.test_command = Some("cargo test".to_string());
            }
            ProjectType::NodeJs => {
                // Try to read package.json for scripts
                if let Ok(package_content) = std::fs::read_to_string(path.join("package.json")) {
                    if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&package_content) {
                        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
                            if scripts.contains_key("dev") {
                                config.dev_command = Some("npm run dev".to_string());
                            } else if scripts.contains_key("start") {
                                config.dev_command = Some("npm start".to_string());
                            }
                            
                            if scripts.contains_key("build") {
                                config.build_command = Some("npm run build".to_string());
                            }
                            
                            if scripts.contains_key("test") {
                                config.test_command = Some("npm test".to_string());
                            }
                        }
                    }
                }
            }
            ProjectType::Python => {
                config.dev_command = Some("python main.py".to_string());
                config.test_command = Some("pytest".to_string());
            }
            ProjectType::Generic => {
                // No default commands for generic projects
            }
        }
        
        config
    }
    
    pub fn update_last_opened(&mut self) {
        self.last_opened = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProcess {
    pub id: Uuid,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub env_vars: HashMap<String, String>,
    pub logs: Vec<LogEntry>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub auto_restart: bool,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

pub struct WorkspaceManager {
    projects: Arc<RwLock<HashMap<Uuid, Project>>>,
    active_project: Arc<RwLock<Option<Uuid>>>,
    background_processes: Arc<RwLock<HashMap<Uuid, BackgroundProcess>>>,
    running_processes: Arc<RwLock<HashMap<Uuid, Child>>>,
    git_manager: Arc<RwLock<GitManager>>,
    event_bus: code_furnace_events::EventBus,
}

impl WorkspaceManager {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
            active_project: Arc::new(RwLock::new(None)),
            background_processes: Arc::new(RwLock::new(HashMap::new())),
            running_processes: Arc::new(RwLock::new(HashMap::new())),
            git_manager: Arc::new(RwLock::new(GitManager::new())),
            event_bus,
        }
    }
    
    pub async fn create_project(&self, name: String, path: PathBuf) -> Result<Uuid> {
        let project = Project::new(name, path);
        let project_id = project.id;
        
        let mut projects = self.projects.write().await;
        projects.insert(project_id, project);
        
        let event = code_furnace_events::Event::new(
            "workspace.project.created",
            "workspace-manager",
            serde_json::to_value(&project_id)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(project_id)
    }
    
    pub async fn open_project(&self, project_id: Uuid) -> Result<()> {
        {
            let mut projects = self.projects.write().await;
            if let Some(project) = projects.get_mut(&project_id) {
                project.update_last_opened();
            } else {
                return Err(anyhow::anyhow!("Project not found: {}", project_id));
            }
        }
        
        let mut active_project = self.active_project.write().await;
        *active_project = Some(project_id);
        
        let event = code_furnace_events::Event::new(
            "workspace.project.opened",
            "workspace-manager",
            serde_json::to_value(&project_id)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(())
    }
    
    pub async fn get_project(&self, project_id: Uuid) -> Option<Project> {
        let projects = self.projects.read().await;
        projects.get(&project_id).cloned()
    }
    
    pub async fn list_projects(&self) -> Vec<Project> {
        let projects = self.projects.read().await;
        let mut project_list: Vec<Project> = projects.values().cloned().collect();
        project_list.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        project_list
    }
    
    pub async fn get_active_project(&self) -> Option<Project> {
        let active_project_id = {
            let active_project = self.active_project.read().await;
            *active_project
        };
        
        if let Some(project_id) = active_project_id {
            self.get_project(project_id).await
        } else {
            None
        }
    }
    
    pub async fn start_background_process(
        &self,
        name: String,
        command: String,
        args: Vec<String>,
        working_directory: PathBuf,
        port: Option<u16>,
        env_vars: HashMap<String, String>,
        auto_restart: bool,
    ) -> Result<Uuid> {
        let process_id = Uuid::new_v4();
        
        // Parse command and arguments
        let mut cmd = Command::new(&command);
        cmd.args(&args)
           .current_dir(&working_directory)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .stdin(Stdio::null());
        
        // Set environment variables
        for (key, value) in &env_vars {
            cmd.env(key, value);
        }
        
        // Start the process
        let child = cmd.spawn()?;
        let pid = child.id();
        
        let process = BackgroundProcess {
            id: process_id,
            name: name.clone(),
            command: command.clone(),
            args: args.clone(),
            working_directory: working_directory.clone(),
            status: ProcessStatus::Running,
            pid,
            port,
            env_vars,
            logs: Vec::new(),
            started_at: chrono::Utc::now(),
            stopped_at: None,
            auto_restart,
            restart_count: 0,
        };
        
        // Store the process
        {
            let mut background_processes = self.background_processes.write().await;
            background_processes.insert(process_id, process);
        }
        
        // Store the running child process
        {
            let mut running_processes = self.running_processes.write().await;
            running_processes.insert(process_id, child);
        }
        
        // Start monitoring the process output
        self.monitor_process_output(process_id).await;
        
        let event = code_furnace_events::Event::new(
            "workspace.process.started",
            "workspace-manager",
            serde_json::json!({
                "process_id": process_id,
                "name": name,
                "command": command,
                "args": args,
                "pid": pid
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(process_id)
    }
    
    async fn monitor_process_output(&self, process_id: Uuid) {
        let (stdout, stderr) = {
            let mut running_processes = self.running_processes.write().await;
            if let Some(child) = running_processes.get_mut(&process_id) {
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                (stdout, stderr)
            } else {
                return;
            }
        };
        
        let background_processes = self.background_processes.clone();
        let event_bus = self.event_bus.clone();
        
        // Monitor stdout
        if let Some(stdout) = stdout {
            let background_processes_clone = background_processes.clone();
            let event_bus_clone = event_bus.clone();
            
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                
                while let Ok(bytes_read) = reader.read_line(&mut line).await {
                    if bytes_read == 0 {
                        break;
                    }
                    
                    let log_entry = LogEntry {
                        timestamp: chrono::Utc::now(),
                        level: LogLevel::Info,
                        message: line.trim().to_string(),
                    };
                    
                    // Add to process logs
                    {
                        let mut processes = background_processes_clone.write().await;
                        if let Some(process) = processes.get_mut(&process_id) {
                            process.logs.push(log_entry.clone());
                            
                            // Keep only last 1000 log entries
                            if process.logs.len() > 1000 {
                                process.logs.drain(0..process.logs.len() - 1000);
                            }
                        }
                    }
                    
                    // Publish log event
                    let event = code_furnace_events::Event::new(
                        "workspace.process.log",
                        "workspace-manager",
                        serde_json::json!({
                            "process_id": process_id,
                            "log": log_entry
                        }),
                    );
                    let _ = event_bus_clone.publish(event);
                    
                    line.clear();
                }
            });
        }
        
        // Monitor stderr
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                
                while let Ok(bytes_read) = reader.read_line(&mut line).await {
                    if bytes_read == 0 {
                        break;
                    }
                    
                    let log_entry = LogEntry {
                        timestamp: chrono::Utc::now(),
                        level: LogLevel::Error,
                        message: line.trim().to_string(),
                    };
                    
                    // Add to process logs
                    {
                        let mut processes = background_processes.write().await;
                        if let Some(process) = processes.get_mut(&process_id) {
                            process.logs.push(log_entry.clone());
                            
                            // Keep only last 1000 log entries
                            if process.logs.len() > 1000 {
                                process.logs.drain(0..process.logs.len() - 1000);
                            }
                        }
                    }
                    
                    // Publish log event
                    let event = code_furnace_events::Event::new(
                        "workspace.process.log",
                        "workspace-manager",
                        serde_json::json!({
                            "process_id": process_id,
                            "log": log_entry
                        }),
                    );
                    let _ = event_bus.publish(event);
                    
                    line.clear();
                }
            });
        }
    }
    
    pub async fn stop_background_process(&self, process_id: Uuid) -> Result<()> {
        // Kill the actual process
        {
            let mut running_processes = self.running_processes.write().await;
            if let Some(mut child) = running_processes.remove(&process_id) {
                let _ = child.kill().await;
            }
        }
        
        // Update process status
        {
            let mut background_processes = self.background_processes.write().await;
            if let Some(process) = background_processes.get_mut(&process_id) {
                process.status = ProcessStatus::Stopped;
                process.stopped_at = Some(chrono::Utc::now());
            }
        }
        
        let event = code_furnace_events::Event::new(
            "workspace.process.stopped",
            "workspace-manager",
            serde_json::to_value(&process_id)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(())
    }
    
    pub async fn restart_background_process(&self, process_id: Uuid) -> Result<()> {
        let (name, command, args, working_directory, port, env_vars, auto_restart) = {
            let background_processes = self.background_processes.read().await;
            if let Some(process) = background_processes.get(&process_id) {
                (
                    process.name.clone(),
                    process.command.clone(), 
                    process.args.clone(),
                    process.working_directory.clone(),
                    process.port,
                    process.env_vars.clone(),
                    process.auto_restart,
                )
            } else {
                return Err(anyhow::anyhow!("Process not found: {}", process_id));
            }
        };
        
        // Stop the current process
        self.stop_background_process(process_id).await?;
        
        // Increment restart count
        {
            let mut background_processes = self.background_processes.write().await;
            if let Some(process) = background_processes.get_mut(&process_id) {
                process.restart_count += 1;
            }
        }
        
        // Start a new process with the same configuration
        self.start_background_process(
            name,
            command,
            args,
            working_directory,
            port,
            env_vars,
            auto_restart,
        ).await?;
        
        Ok(())
    }
    
    pub async fn get_process_logs(&self, process_id: Uuid, limit: Option<usize>) -> Vec<LogEntry> {
        let background_processes = self.background_processes.read().await;
        if let Some(process) = background_processes.get(&process_id) {
            let logs = &process.logs;
            match limit {
                Some(n) => {
                    let start = if logs.len() > n { logs.len() - n } else { 0 };
                    logs[start..].to_vec()
                }
                None => logs.clone(),
            }
        } else {
            Vec::new()
        }
    }
    
    pub async fn get_project_processes(&self, project_id: Uuid) -> Vec<BackgroundProcess> {
        let project_path = {
            let projects = self.projects.read().await;
            if let Some(project) = projects.get(&project_id) {
                project.path.clone()
            } else {
                return Vec::new();
            }
        };
        
        let background_processes = self.background_processes.read().await;
        background_processes
            .values()
            .filter(|process| process.working_directory.starts_with(&project_path))
            .cloned()
            .collect()
    }
    
    pub async fn list_background_processes(&self) -> Vec<BackgroundProcess> {
        let background_processes = self.background_processes.read().await;
        background_processes.values().cloned().collect()
    }
    
    // Git Integration Methods
    pub async fn open_git_repository(&self, path: PathBuf) -> Result<GitRepository> {
        let mut git_manager = self.git_manager.write().await;
        git_manager.open_repository(path)
    }
    
    pub async fn get_git_status(&self, repo_path: &PathBuf) -> Result<GitStatus> {
        let git_manager = self.git_manager.read().await;
        let repo = git2::Repository::open(repo_path)?;
        git_manager.get_status(&repo)
    }
    
    pub async fn git_stage_file(&self, repo_path: &PathBuf, file_path: &str) -> Result<()> {
        let mut git_manager = self.git_manager.write().await;
        git_manager.stage_file(repo_path, file_path)
    }
    
    pub async fn git_unstage_file(&self, repo_path: &PathBuf, file_path: &str) -> Result<()> {
        let mut git_manager = self.git_manager.write().await;
        git_manager.unstage_file(repo_path, file_path)
    }
    
    pub async fn git_commit(&self, repo_path: &PathBuf, message: &str, author_name: &str, author_email: &str) -> Result<String> {
        let mut git_manager = self.git_manager.write().await;
        let commit_id = git_manager.commit(repo_path, message, author_name, author_email)?;
        
        // Publish commit event
        let event = code_furnace_events::Event::new(
            "workspace.git.commit",
            "workspace-manager",
            serde_json::json!({
                "repo_path": repo_path,
                "commit_id": commit_id,
                "message": message
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(commit_id)
    }
    
    pub async fn git_get_commit_history(&self, repo_path: &PathBuf, limit: Option<usize>) -> Result<Vec<GitCommit>> {
        let git_manager = self.git_manager.read().await;
        git_manager.get_commit_history(repo_path, limit)
    }
    
    pub async fn git_get_branches(&self, repo_path: &PathBuf) -> Result<Vec<GitBranch>> {
        let git_manager = self.git_manager.read().await;
        git_manager.get_branches(repo_path)
    }
    
    pub async fn git_create_branch(&self, repo_path: &PathBuf, branch_name: &str, from_head: bool) -> Result<()> {
        let mut git_manager = self.git_manager.write().await;
        git_manager.create_branch(repo_path, branch_name, from_head)
    }
    
    pub async fn git_switch_branch(&self, repo_path: &PathBuf, branch_name: &str) -> Result<()> {
        let mut git_manager = self.git_manager.write().await;
        git_manager.switch_branch(repo_path, branch_name)?;
        
        // Publish branch switch event
        let event = code_furnace_events::Event::new(
            "workspace.git.branch_switched",
            "workspace-manager",
            serde_json::json!({
                "repo_path": repo_path,
                "branch_name": branch_name
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(())
    }
    
    pub async fn git_get_diff(&self, repo_path: &PathBuf, staged: bool) -> Result<Vec<GitDiff>> {
        let git_manager = self.git_manager.read().await;
        git_manager.get_diff(repo_path, staged)
    }
    
    pub async fn git_push(&self, repo_path: &PathBuf, remote: &str, branch: &str) -> Result<()> {
        let git_manager = self.git_manager.read().await;
        git_manager.push(repo_path, remote, branch)
    }
    
    pub async fn git_pull(&self, repo_path: &PathBuf, remote: &str, branch: &str) -> Result<()> {
        let git_manager = self.git_manager.read().await;
        git_manager.pull(repo_path, remote, branch)
    }
    
    pub async fn generate_ai_commit_message(&self, repo_path: &PathBuf, staged_files: &[String]) -> Result<String> {
        // Use the AI-powered commit message generation in GitManager
        let git_manager = self.git_manager.read().await;
        git_manager.generate_ai_commit_message(repo_path, staged_files).await
    }
    
    // Quick action to start common project processes
    pub async fn start_project_dev_server(&self, project_id: Uuid) -> Result<Uuid> {
        let project = self.get_project(project_id).await
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;
        
        if let Some(dev_command) = &project.config.dev_command {
            let parts: Vec<&str> = dev_command.split_whitespace().collect();
            if let Some((command, args)) = parts.split_first() {
                self.start_background_process(
                    format!("{} Dev Server", project.name),
                    command.to_string(),
                    args.iter().map(|s| s.to_string()).collect(),
                    project.path.clone(),
                    project.config.ports.first().copied(),
                    project.config.env_vars.clone(),
                    true, // auto-restart
                ).await
            } else {
                Err(anyhow::anyhow!("Invalid dev command"))
            }
        } else {
            Err(anyhow::anyhow!("No dev command configured for project"))
        }
    }
}