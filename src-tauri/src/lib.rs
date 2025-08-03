use std::sync::Arc;
use tauri::{Manager, State, Emitter};
use tracing::{info, error};
use tokio::sync::broadcast;

// Re-export our crates for easier access
pub use code_furnace_agents as agents;
pub use code_furnace_terminal as terminal;
pub use code_furnace_editor as editor;
pub use code_furnace_canvas as canvas;
pub use code_furnace_events as events;
pub use code_furnace_workspace as workspace;
pub use code_furnace_plugins as plugins;
pub use code_furnace_utils as utils;

// Application state that will be shared across all managers
#[derive(Clone)]
pub struct AppState {
    pub event_bus: events::EventBus,
    pub agent_bridge: Arc<tokio::sync::RwLock<agents::AgentBridge>>,
    pub terminal_manager: Arc<terminal::TerminalManager>,
    pub editor_manager: Arc<editor::EditorManager>,
    pub workspace_manager: Arc<workspace::WorkspaceManager>,
    pub plugin_runtime: Arc<tokio::sync::RwLock<plugins::PluginRuntime>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let event_bus = events::EventBus::new();
        
        // Initialize managers with shared event bus
        let mut agent_bridge = agents::AgentBridge::new();
        let terminal_manager = terminal::TerminalManager::new(event_bus.clone());
        let editor_manager = editor::EditorManager::new(event_bus.clone());
        let workspace_manager = workspace::WorkspaceManager::new(event_bus.clone());
        let plugin_runtime = plugins::PluginRuntime::new(event_bus.clone())?;
        
        // Load configuration and set up agent providers
        if let Ok(config) = utils::Config::load() {
            if let Some(api_key) = config.agent_api_key {
                match config.agent_provider {
                    utils::AgentProvider::Claude => {
                        let base_claude = agents::ClaudeProvider::new(api_key.clone());
                        agent_bridge.register_provider("claude".to_string(), Box::new(base_claude));
                        agent_bridge.set_default_provider("claude".to_string());
                        
                        // Register specialized agents
                        let agent_types = vec![
                            agents::AgentType::CodeExplainer,
                            agents::AgentType::CodeReviewer,
                            agents::AgentType::TestGenerator,
                            agents::AgentType::GitAssistant,
                            agents::AgentType::UIDesigner,
                            agents::AgentType::SystemArchitect,
                            agents::AgentType::DocumentationWriter,
                            agents::AgentType::Debugger,
                        ];
                        
                        for agent_type in agent_types {
                            let claude_provider = agents::ClaudeProvider::new(api_key.clone());
                            agent_bridge.register_specialized_agent(agent_type, Box::new(claude_provider));
                        }
                    }
                    utils::AgentProvider::OpenAI => {
                        let base_openai = agents::OpenAIProvider::new(api_key.clone());
                        agent_bridge.register_provider("openai".to_string(), Box::new(base_openai));
                        agent_bridge.set_default_provider("openai".to_string());
                        
                        // Register specialized agents
                        let agent_types = vec![
                            agents::AgentType::CodeExplainer,
                            agents::AgentType::CodeReviewer,
                            agents::AgentType::TestGenerator,
                            agents::AgentType::GitAssistant,
                            agents::AgentType::UIDesigner,
                            agents::AgentType::SystemArchitect,
                            agents::AgentType::DocumentationWriter,
                            agents::AgentType::Debugger,
                        ];
                        
                        for agent_type in agent_types {
                            let openai_provider = agents::OpenAIProvider::new(api_key.clone());
                            agent_bridge.register_specialized_agent(agent_type, Box::new(openai_provider));
                        }
                    }
                    _ => {
                        info!("Ollama and other agent providers not yet implemented");
                    }
                }
            }
        }
        
        Ok(Self {
            event_bus,
            agent_bridge: Arc::new(tokio::sync::RwLock::new(agent_bridge)),
            terminal_manager: Arc::new(terminal_manager),
            editor_manager: Arc::new(editor_manager),
            workspace_manager: Arc::new(workspace_manager),
            plugin_runtime: Arc::new(tokio::sync::RwLock::new(plugin_runtime)),
        })
    }
}

// Tauri command handlers
#[tauri::command]
async fn create_terminal_session(
    state: State<'_, AppState>,
    name: String,
    working_directory: String,
) -> Result<String, String> {
    let working_dir = std::path::PathBuf::from(working_directory);
    match state.terminal_manager.create_session(name, working_dir).await {
        Ok(session_id) => Ok(session_id.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn execute_terminal_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<String, String> {
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    match state.terminal_manager.execute_command(session_uuid, command).await {
        Ok(block_id) => Ok(block_id.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn get_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<terminal::TerminalSession>, String> {
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    Ok(state.terminal_manager.get_session(session_uuid).await)
}

#[tauri::command]
async fn open_file(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(file_path);
    match state.editor_manager.open_file(path).await {
        Ok(buffer_id) => Ok(buffer_id.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn get_file_buffer(
    state: State<'_, AppState>,
    buffer_id: String,
) -> Result<Option<editor::FileBuffer>, String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    Ok(state.editor_manager.get_buffer(buffer_uuid).await)
}

#[tauri::command]
async fn update_file_buffer(
    state: State<'_, AppState>,
    buffer_id: String,
    content: String,
) -> Result<(), String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    state.editor_manager.update_buffer(buffer_uuid, content).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_file_buffer(
    state: State<'_, AppState>,
    buffer_id: String,
) -> Result<(), String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    state.editor_manager.save_buffer(buffer_uuid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_file_tree(
    state: State<'_, AppState>,
) -> Result<Option<editor::FileTreeNode>, String> {
    state.editor_manager.get_file_tree().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn expand_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<editor::FileTreeNode>, String> {
    let path_buf = std::path::PathBuf::from(path);
    state.editor_manager.expand_directory(path_buf).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_file(
    state: State<'_, AppState>,
    path: String,
    content: Option<String>,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);
    state.editor_manager.create_file(path_buf, content).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);
    state.editor_manager.create_directory(path_buf).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);
    state.editor_manager.delete_file(path_buf).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_file(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let old_path_buf = std::path::PathBuf::from(old_path);
    let new_path_buf = std::path::PathBuf::from(new_path);
    state.editor_manager.rename_file(old_path_buf, new_path_buf).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_workspace_root(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(root_path);
    state.editor_manager.set_workspace_root(path.clone()).await.map_err(|e| e.to_string())?;
    // Also create/open project in workspace manager
    let project_name = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let project_id = state.workspace_manager.create_project(project_name, path).await.map_err(|e| e.to_string())?;
    state.workspace_manager.open_project(project_id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn ask_agent(
    state: State<'_, AppState>,
    prompt: String,
    context_files: Vec<String>,
    agent_type: Option<String>,
) -> Result<agents::AgentResponse, String> {
    let mut agent_bridge = state.agent_bridge.write().await;
    
    let request = agents::AgentRequest {
        id: uuid::Uuid::new_v4(),
        agent_type: agent_type.unwrap_or_default(),
        prompt,
        context: std::collections::HashMap::new(),
        files: context_files,
    };
    
    agent_bridge.process_request(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_terminal_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<terminal::TerminalSession>, String> {
    Ok(state.terminal_manager.list_sessions().await)
}

#[tauri::command]
async fn send_terminal_input(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.terminal_manager.send_input(session_uuid, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn resize_terminal(
    state: State<'_, AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.terminal_manager.resize_terminal(session_uuid, cols, rows).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn close_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.terminal_manager.close_session(session_uuid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_file_buffers(
    state: State<'_, AppState>,
) -> Result<Vec<editor::FileBuffer>, String> {
    Ok(state.editor_manager.list_buffers().await)
}

#[tauri::command]
async fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<workspace::Project>, String> {
    Ok(state.workspace_manager.list_projects().await)
}

#[tauri::command]
async fn create_project(
    state: State<'_, AppState>,
    name: String,
    path: String,
) -> Result<String, String> {
    let project_id = state.workspace_manager.create_project(name, std::path::PathBuf::from(path))
        .await.map_err(|e| e.to_string())?;
    Ok(project_id.to_string())
}

#[tauri::command]
async fn open_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    state.workspace_manager.open_project(uuid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_active_project(
    state: State<'_, AppState>,
) -> Result<Option<workspace::Project>, String> {
    Ok(state.workspace_manager.get_active_project().await)
}

// Background Process Commands
#[tauri::command]
async fn start_background_process(
    state: State<'_, AppState>,
    name: String,
    command: String,
    args: Vec<String>,
    working_directory: String,
    port: Option<u16>,
    env_vars: std::collections::HashMap<String, String>,
    auto_restart: bool,
) -> Result<String, String> {
    let process_id = state.workspace_manager.start_background_process(
        name, command, args, std::path::PathBuf::from(working_directory), 
        port, env_vars, auto_restart
    ).await.map_err(|e| e.to_string())?;
    Ok(process_id.to_string())
}

#[tauri::command]
async fn stop_background_process(
    state: State<'_, AppState>,
    process_id: String,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&process_id).map_err(|e| e.to_string())?;
    state.workspace_manager.stop_background_process(uuid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn restart_background_process(
    state: State<'_, AppState>,
    process_id: String,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&process_id).map_err(|e| e.to_string())?;
    state.workspace_manager.restart_background_process(uuid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_background_processes(
    state: State<'_, AppState>,
) -> Result<Vec<workspace::BackgroundProcess>, String> {
    Ok(state.workspace_manager.list_background_processes().await)
}

#[tauri::command]
async fn get_process_logs(
    state: State<'_, AppState>,
    process_id: String,
    limit: Option<usize>,
) -> Result<Vec<workspace::LogEntry>, String> {
    let uuid = uuid::Uuid::parse_str(&process_id).map_err(|e| e.to_string())?;
    Ok(state.workspace_manager.get_process_logs(uuid, limit).await)
}

#[tauri::command]
async fn start_project_dev_server(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let process_id = state.workspace_manager.start_project_dev_server(uuid).await.map_err(|e| e.to_string())?;
    Ok(process_id.to_string())
}

// Git Commands
#[tauri::command]
async fn open_git_repository(
    state: State<'_, AppState>,
    path: String,
) -> Result<workspace::GitRepository, String> {
    state.workspace_manager.open_git_repository(std::path::PathBuf::from(path))
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_git_status(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<workspace::GitStatus, String> {
    state.workspace_manager.get_git_status(&std::path::PathBuf::from(repo_path))
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_stage_file(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
) -> Result<(), String> {
    state.workspace_manager.git_stage_file(&std::path::PathBuf::from(repo_path), &file_path)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_unstage_file(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
) -> Result<(), String> {
    state.workspace_manager.git_unstage_file(&std::path::PathBuf::from(repo_path), &file_path)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_commit(
    state: State<'_, AppState>,
    repo_path: String,
    message: String,
    author_name: String,
    author_email: String,
) -> Result<String, String> {
    state.workspace_manager.git_commit(&std::path::PathBuf::from(repo_path), &message, &author_name, &author_email)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_get_commit_history(
    state: State<'_, AppState>,
    repo_path: String,
    limit: Option<usize>,
) -> Result<Vec<workspace::GitCommit>, String> {
    state.workspace_manager.git_get_commit_history(&std::path::PathBuf::from(repo_path), limit)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_get_branches(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<Vec<workspace::GitBranch>, String> {
    state.workspace_manager.git_get_branches(&std::path::PathBuf::from(repo_path))
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_create_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    from_head: bool,
) -> Result<(), String> {
    state.workspace_manager.git_create_branch(&std::path::PathBuf::from(repo_path), &branch_name, from_head)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_switch_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
) -> Result<(), String> {
    state.workspace_manager.git_switch_branch(&std::path::PathBuf::from(repo_path), &branch_name)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_get_diff(
    state: State<'_, AppState>,
    repo_path: String,
    staged: bool,
) -> Result<Vec<workspace::GitDiff>, String> {
    state.workspace_manager.git_get_diff(&std::path::PathBuf::from(repo_path), staged)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_push(
    state: State<'_, AppState>,
    repo_path: String,
    remote: String,
    branch: String,
) -> Result<(), String> {
    state.workspace_manager.git_push(&std::path::PathBuf::from(repo_path), &remote, &branch)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn git_pull(
    state: State<'_, AppState>,
    repo_path: String,
    remote: String,
    branch: String,
) -> Result<(), String> {
    state.workspace_manager.git_pull(&std::path::PathBuf::from(repo_path), &remote, &branch)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn generate_ai_commit_message(
    state: State<'_, AppState>,
    repo_path: String,
    staged_files: Vec<String>,
) -> Result<String, String> {
    state.workspace_manager.generate_ai_commit_message(&std::path::PathBuf::from(repo_path), &staged_files)
        .await.map_err(|e| e.to_string())
}

// LSP Commands
#[tauri::command]
async fn get_completion(
    state: State<'_, AppState>,
    buffer_id: String,
    line: u32,
    character: u32,
) -> Result<Vec<editor::LSPCompletionItem>, String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    state.editor_manager.get_completion(buffer_uuid, line, character).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_hover_info(
    state: State<'_, AppState>,
    buffer_id: String,
    line: u32,
    character: u32,
) -> Result<Option<editor::LSPHover>, String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    state.editor_manager.get_hover(buffer_uuid, line, character).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_diagnostics(
    state: State<'_, AppState>,
    buffer_id: String,
) -> Result<Option<Vec<editor::LSPDiagnostic>>, String> {
    let buffer_uuid = uuid::Uuid::parse_str(&buffer_id).map_err(|e| e.to_string())?;
    Ok(state.editor_manager.get_diagnostics(buffer_uuid).await)
}

// Agent conversation management commands
#[tauri::command]
async fn create_conversation(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let mut agent_bridge = state.agent_bridge.write().await;
    let conversation_id = agent_bridge.create_conversation(name);
    Ok(conversation_id.to_string())
}

#[tauri::command]
async fn list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<agents::ConversationThread>, String> {
    let agent_bridge = state.agent_bridge.read().await;
    let conversations = agent_bridge.list_conversations();
    Ok(conversations.into_iter().cloned().collect())
}

#[tauri::command]
async fn get_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<agents::ConversationThread>, String> {
    let agent_bridge = state.agent_bridge.read().await;
    let id = uuid::Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    Ok(agent_bridge.get_conversation(id).cloned())
}

#[tauri::command]
async fn set_active_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let mut agent_bridge = state.agent_bridge.write().await;
    let id = uuid::Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    agent_bridge.set_active_conversation(id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_conversations(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<agents::ConversationThread>, String> {
    let agent_bridge = state.agent_bridge.read().await;
    let conversations = agent_bridge.search_conversations(&query);
    Ok(conversations.into_iter().cloned().collect())
}

#[tauri::command]
async fn list_available_agents(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let agent_bridge = state.agent_bridge.read().await;
    Ok(agent_bridge.list_available_agents())
}

// Configuration Commands
#[tauri::command]
async fn get_config() -> Result<utils::Config, String> {
    utils::Config::load().map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_agent_config(
    provider: String,
    api_key: Option<String>,
) -> Result<(), String> {
    let mut config = utils::Config::load().map_err(|e| e.to_string())?;
    
    let agent_provider = match provider.as_str() {
        "claude" => utils::AgentProvider::Claude,
        "openai" => utils::AgentProvider::OpenAI,
        "ollama" => utils::AgentProvider::Ollama { 
            endpoint: "http://localhost:11434".to_string() 
        },
        _ => return Err("Invalid agent provider".to_string()),
    };
    
    config.update_agent_config(agent_provider, api_key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_git_tokens(
    github_token: Option<String>,
    gitlab_token: Option<String>,
    gitea_token: Option<String>,
) -> Result<(), String> {
    let mut config = utils::Config::load().map_err(|e| e.to_string())?;
    config.update_git_tokens(github_token, gitlab_token, gitea_token).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_ui_preferences(
    theme: String,
    font_size: u32,
    auto_save: bool,
    enable_lsp: bool,
) -> Result<(), String> {
    let mut config = utils::Config::load().map_err(|e| e.to_string())?;
    config.update_ui_preferences(theme, font_size, auto_save, enable_lsp).map_err(|e| e.to_string())
}

#[tauri::command]
async fn validate_config() -> Result<(), String> {
    let config = utils::Config::load().map_err(|e| e.to_string())?;
    config.validate().map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_agent_connection(
    provider: String,
    api_key: String,
) -> Result<bool, String> {
    // Create a test agent provider and attempt a simple request
    let test_provider: Box<dyn agents::AgentProvider> = match provider.as_str() {
        "claude" => Box::new(agents::ClaudeProvider::new(api_key)),
        "openai" => Box::new(agents::OpenAIProvider::new(api_key)),
        _ => return Err("Unsupported provider for testing".to_string()),
    };
    
    let test_request = agents::AgentRequest {
        id: uuid::Uuid::new_v4(),
        agent_type: "test".to_string(),
        prompt: "Say 'OK' if you can read this".to_string(),
        context: std::collections::HashMap::new(),
        files: Vec::new(),
    };
    
    match test_provider.process_request(&test_request).await {
        Ok(response) => Ok(response.error.is_none()),
        Err(_) => Ok(false),
    }
}

// Canvas Commands
#[tauri::command]
async fn create_canvas(
    state: State<'_, AppState>,
    name: String,
    mode: String,
) -> Result<String, String> {
    let canvas_mode = match mode.as_str() {
        "freeform" => canvas::CanvasMode::Freeform,
        "wireframe" => canvas::CanvasMode::Wireframe,
        "flowchart" => canvas::CanvasMode::Flowchart,
        "system-design" => canvas::CanvasMode::SystemDesign,
        _ => canvas::CanvasMode::Freeform,
    };
    
    let canvas = canvas::Canvas::new(name, canvas_mode);
    let canvas_id = canvas.id;
    
    // Store canvas in workspace manager (canvases are project-specific)
    // For now, we'll just return the ID - in a full implementation this would be persisted
    Ok(canvas_id.to_string())
}

#[tauri::command]
async fn get_canvas(
    state: State<'_, AppState>,
    canvas_id: String,
) -> Result<Option<canvas::Canvas>, String> {
    // In a full implementation, this would retrieve from storage
    // For now, return a placeholder
    Ok(None)
}

#[tauri::command]
async fn update_canvas(
    state: State<'_, AppState>,
    canvas_id: String,
    canvas_data: serde_json::Value,
) -> Result<(), String> {
    // In a full implementation, this would update the stored canvas
    // Publish canvas update event
    let event = events::Event::new(
        "canvas.updated",
        "canvas-manager",
        serde_json::json!({
            "canvas_id": canvas_id,
            "data": canvas_data
        }),
    );
    state.event_bus.publish(event).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn save_canvas(
    state: State<'_, AppState>,
    canvas_id: String,
) -> Result<(), String> {
    // In a full implementation, this would persist the canvas to disk
    let event = events::Event::new(
        "canvas.saved",
        "canvas-manager",
        serde_json::json!({
            "canvas_id": canvas_id
        }),
    );
    state.event_bus.publish(event).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn list_canvases(
    state: State<'_, AppState>,
) -> Result<Vec<canvas::Canvas>, String> {
    // In a full implementation, this would return all stored canvases
    Ok(Vec::new())
}

#[tauri::command]
async fn delete_canvas(
    state: State<'_, AppState>,
    canvas_id: String,
) -> Result<(), String> {
    // In a full implementation, this would delete the canvas
    let event = events::Event::new(
        "canvas.deleted",
        "canvas-manager",
        serde_json::json!({
            "canvas_id": canvas_id
        }),
    );
    state.event_bus.publish(event).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn export_canvas(
    state: State<'_, AppState>,
    canvas_id: String,
    format: String,
) -> Result<String, String> {
    // In a full implementation, this would export the canvas in the specified format
    match format.as_str() {
        "json" => Ok(serde_json::json!({"canvas_id": canvas_id, "format": "json"}).to_string()),
        "mermaid" => Ok(format!("graph TD\n  A[Canvas {}] --> B[Exported]", canvas_id)),
        "svg" => Ok(format!("<svg><text>Canvas {}</text></svg>", canvas_id)),
        _ => Err("Unsupported export format".to_string()),
    }
}

// Event System Commands - for real-time frontend updates
#[tauri::command]
async fn subscribe_to_events(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    event_types: Vec<String>,
) -> Result<(), String> {
    let event_bus = state.event_bus.clone();
    
    // Create receivers for each event type
    for event_type in event_types {
        let receiver = if event_type == "*" {
            event_bus.subscribe(None)
        } else {
            event_bus.subscribe(Some(&event_type))
        };
        
        let app_handle = app.clone();
        let event_type_clone = event_type.clone();
        
        // Spawn a task to listen for events and emit them to frontend
        tokio::spawn(async move {
            let mut rx = receiver;
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        // Emit event to frontend
                        if let Err(e) = app_handle.emit(&format!("event:{}", event_type_clone), &event) {
                            tracing::error!("Failed to emit event to frontend: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Event channel closed for type: {}", event_type_clone);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        tracing::warn!("Event listener lagged, missed {} events for type: {}", missed, event_type_clone);
                        // Continue listening
                    }
                }
            }
        });
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state asynchronously
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match AppState::new().await {
                    Ok(state) => {
                        handle.manage(state);
                        log::info!("Code Furnace application state initialized successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize application state: {}", e);
                        std::process::exit(1);
                    }
                }
            });
            
            log::info!("Starting Code Furnace application");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_terminal_session,
            execute_terminal_command,
            get_terminal_session,
            list_terminal_sessions,
            send_terminal_input,
            resize_terminal,
            close_terminal_session,
            open_file,
            get_file_buffer,
            update_file_buffer,
            save_file_buffer,
            get_file_tree,
            expand_directory,
            create_file,
            create_directory,
            delete_file,
            rename_file,
            set_workspace_root,
            list_file_buffers,
            ask_agent,
            list_projects,
            create_project,
            open_project,
            get_active_project,
            start_background_process,
            stop_background_process,
            restart_background_process,
            list_background_processes,
            get_process_logs,
            start_project_dev_server,
            open_git_repository,
            get_git_status,
            git_stage_file,
            git_unstage_file,
            git_commit,
            git_get_commit_history,
            git_get_branches,
            git_create_branch,
            git_switch_branch,
            git_get_diff,
            git_push,
            git_pull,
            generate_ai_commit_message,
            get_completion,
            get_hover_info,
            get_diagnostics,
            create_conversation,
            list_conversations,
            get_conversation,
            set_active_conversation,
            search_conversations,
            list_available_agents,
            get_config,
            update_agent_config,
            update_git_tokens,
            update_ui_preferences,
            validate_config,
            test_agent_connection,
            subscribe_to_events,
            create_canvas,
            get_canvas,
            update_canvas,
            save_canvas,
            list_canvases,
            delete_canvas,
            export_canvas,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}