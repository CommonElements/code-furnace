use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod lsp;
pub use lsp::{LSPManager, LSPDiagnostic, LSPCompletionItem, LSPHover};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBuffer {
    pub id: Uuid,
    pub path: PathBuf,
    pub content: String,
    pub language: String,
    pub modified: bool,
    pub cursor_position: CursorPosition,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl FileBuffer {
    pub fn new(path: PathBuf, content: String) -> Self {
        let language = Self::detect_language(&path);
        
        Self {
            id: Uuid::new_v4(),
            path,
            content,
            language,
            modified: false,
            cursor_position: CursorPosition::default(),
            last_modified: chrono::Utc::now(),
        }
    }
    
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self::new(path, content))
    }
    
    fn detect_language(path: &PathBuf) -> String {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("py") => "python".to_string(),
            Some("json") => "json".to_string(),
            Some("toml") => "toml".to_string(),
            Some("md") => "markdown".to_string(),
            Some("html") => "html".to_string(),
            Some("css") => "css".to_string(),
            _ => "plaintext".to_string(),
        }
    }
    
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.modified = true;
        self.last_modified = chrono::Utc::now();
    }
    
    pub fn save(&mut self) -> Result<()> {
        std::fs::write(&self.path, &self.content)?;
        self.modified = false;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: Vec<FileTreeNode>,
    pub expanded: bool,
}

impl FileTreeNode {
    pub fn build_tree(root_path: PathBuf) -> Result<Self> {
        let metadata = std::fs::metadata(&root_path)?;
        let name = root_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let mut node = Self {
            name,
            path: root_path.clone(),
            is_directory: metadata.is_dir(),
            children: Vec::new(),
            expanded: false,
        };
        
        if metadata.is_dir() {
            let entries = std::fs::read_dir(&root_path)?;
            for entry in entries {
                let entry = entry?;
                let child_path = entry.path();
                
                // Skip hidden files and common ignore patterns
                if let Some(name) = child_path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || 
                       name_str == "node_modules" || 
                       name_str == "target" ||
                       name_str == "dist" {
                        continue;
                    }
                }
                
                if let Ok(child_node) = Self::build_tree(child_path) {
                    node.children.push(child_node);
                }
            }
            
            // Sort children: directories first, then files, both alphabetically
            node.children.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });
        }
        
        Ok(node)
    }
}

pub struct EditorManager {
    buffers: Arc<RwLock<HashMap<Uuid, FileBuffer>>>,
    active_buffer: Arc<RwLock<Option<Uuid>>>,
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    event_bus: code_furnace_events::EventBus,
    lsp_manager: Arc<LSPManager>,
    document_versions: Arc<RwLock<HashMap<String, u64>>>,
}

impl EditorManager {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            active_buffer: Arc::new(RwLock::new(None)),
            workspace_root: Arc::new(RwLock::new(None)),
            event_bus,
            lsp_manager: Arc::new(LSPManager::new()),
            document_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn set_workspace_root(&self, root: PathBuf) -> Result<()> {
        let mut workspace_root = self.workspace_root.write().await;
        *workspace_root = Some(root.clone());
        
        let event = code_furnace_events::Event::new(
            "editor.workspace.changed",
            "editor-manager",
            serde_json::to_value(&root)?,
        );
        self.event_bus.publish(event)?;
        
        Ok(())
    }
    
    pub async fn open_file(&self, path: PathBuf) -> Result<Uuid> {
        let buffer = FileBuffer::from_file(path.clone())?;
        let buffer_id = buffer.id;
        
        // Get workspace root for LSP
        let workspace_root = {
            let root = self.workspace_root.read().await;
            root.clone().unwrap_or_else(|| {
                path.parent().unwrap_or(&path).to_path_buf()
            })
        };
        
        // Start LSP server if needed
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            if let Some(language) = self.lsp_manager.get_language_for_extension(extension) {
                if let Err(e) = self.lsp_manager.ensure_server(&language, workspace_root).await {
                    tracing::warn!("Failed to start LSP server for {}: {}", language, e);
                }
                
                // Notify LSP of opened document
                let uri = format!("file://{}", path.to_string_lossy());
                if let Err(e) = self.lsp_manager.did_open_document(
                    uri.clone(),
                    language.clone(),
                    buffer.content.clone()
                ).await {
                    tracing::warn!("Failed to notify LSP of opened document: {}", e);
                }
                
                // Initialize document version
                let mut versions = self.document_versions.write().await;
                versions.insert(uri, 1);
            }
        }
        
        let mut buffers = self.buffers.write().await;
        buffers.insert(buffer_id, buffer);
        
        let mut active_buffer = self.active_buffer.write().await;
        *active_buffer = Some(buffer_id);
        
        let event = code_furnace_events::Event::new(
            "editor.file.opened",
            "editor-manager",
            serde_json::json!({
                "buffer_id": buffer_id,
                "path": path
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(buffer_id)
    }
    
    pub async fn get_buffer(&self, buffer_id: Uuid) -> Option<FileBuffer> {
        let buffers = self.buffers.read().await;
        buffers.get(&buffer_id).cloned()
    }
    
    pub async fn update_buffer(&self, buffer_id: Uuid, content: String) -> Result<()> {
        let mut buffers = self.buffers.write().await;
        
        if let Some(buffer) = buffers.get_mut(&buffer_id) {
            buffer.update_content(content.clone());
            
            // Notify LSP of document change
            if let Some(extension) = buffer.path.extension().and_then(|ext| ext.to_str()) {
                if let Some(language) = self.lsp_manager.get_language_for_extension(extension) {
                    let uri = format!("file://{}", buffer.path.to_string_lossy());
                    
                    // Get and increment document version
                    let version = {
                        let mut versions = self.document_versions.write().await;
                        let version = versions.entry(uri.clone()).or_insert(1);
                        *version += 1;
                        *version
                    };
                    
                    if let Err(e) = self.lsp_manager.did_change_document(
                        uri,
                        language,
                        content,
                        version
                    ).await {
                        tracing::warn!("Failed to notify LSP of document change: {}", e);
                    }
                }
            }
            
            let event = code_furnace_events::Event::new(
                "editor.buffer.modified",
                "editor-manager",
                serde_json::json!({
                    "buffer_id": buffer_id,
                    "path": buffer.path
                }),
            );
            self.event_bus.publish(event)?;
        }
        
        Ok(())
    }
    
    pub async fn save_buffer(&self, buffer_id: Uuid) -> Result<()> {
        let mut buffers = self.buffers.write().await;
        
        if let Some(buffer) = buffers.get_mut(&buffer_id) {
            buffer.save()?;
            
            let event = code_furnace_events::Event::new(
                "editor.file.saved",
                "editor-manager",
                serde_json::json!({
                    "buffer_id": buffer_id,
                    "path": buffer.path
                }),
            );
            self.event_bus.publish(event)?;
        }
        
        Ok(())
    }
    
    pub async fn get_file_tree(&self) -> Result<Option<FileTreeNode>> {
        let workspace_root = self.workspace_root.read().await;
        
        if let Some(root) = workspace_root.as_ref() {
            Ok(Some(FileTreeNode::build_tree(root.clone())?))
        } else {
            Ok(None)
        }
    }
    
    pub async fn list_buffers(&self) -> Vec<FileBuffer> {
        let buffers = self.buffers.read().await;
        buffers.values().cloned().collect()
    }
    
    pub async fn get_completion(&self, buffer_id: Uuid, line: u32, character: u32) -> Result<Vec<LSPCompletionItem>> {
        let buffers = self.buffers.read().await;
        
        if let Some(buffer) = buffers.get(&buffer_id) {
            if let Some(extension) = buffer.path.extension().and_then(|ext| ext.to_str()) {
                if let Some(language) = self.lsp_manager.get_language_for_extension(extension) {
                    let uri = format!("file://{}", buffer.path.to_string_lossy());
                    return self.lsp_manager.get_completion(language, uri, line, character).await;
                }
            }
        }
        
        Ok(Vec::new())
    }
    
    pub async fn get_hover(&self, buffer_id: Uuid, line: u32, character: u32) -> Result<Option<LSPHover>> {
        let buffers = self.buffers.read().await;
        
        if let Some(buffer) = buffers.get(&buffer_id) {
            if let Some(extension) = buffer.path.extension().and_then(|ext| ext.to_str()) {
                if let Some(language) = self.lsp_manager.get_language_for_extension(extension) {
                    let uri = format!("file://{}", buffer.path.to_string_lossy());
                    return self.lsp_manager.get_hover(language, uri, line, character).await;
                }
            }
        }
        
        Ok(None)
    }
    
    pub async fn get_diagnostics(&self, buffer_id: Uuid) -> Option<Vec<LSPDiagnostic>> {
        let buffers = self.buffers.read().await;
        
        if let Some(buffer) = buffers.get(&buffer_id) {
            if let Some(extension) = buffer.path.extension().and_then(|ext| ext.to_str()) {
                if let Some(language) = self.lsp_manager.get_language_for_extension(extension) {
                    let uri = format!("file://{}", buffer.path.to_string_lossy());
                    return self.lsp_manager.get_diagnostics(&language, &uri).await;
                }
            }
        }
        
        None
    }
}

impl Drop for EditorManager {
    fn drop(&mut self) {
        // Shutdown LSP servers when the editor manager is dropped
        let lsp_manager = self.lsp_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = lsp_manager.shutdown_all().await {
                tracing::error!("Failed to shutdown LSP servers: {}", e);
            }
        });
    }
}