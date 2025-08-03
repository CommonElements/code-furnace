use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasmtime::{Caller, Linker};

/// Plugin API surface that provides access to Code Furnace functionality
#[derive(Debug, Clone)]
pub struct PluginAPI {
    pub terminal_api: TerminalAPI,
    pub editor_api: EditorAPI,
    pub canvas_api: CanvasAPI,
    pub filesystem_api: FilesystemAPI,
    pub network_api: NetworkAPI,
}

impl PluginAPI {
    pub fn new() -> Self {
        Self {
            terminal_api: TerminalAPI::new(),
            editor_api: EditorAPI::new(),
            canvas_api: CanvasAPI::new(),
            filesystem_api: FilesystemAPI::new(),
            network_api: NetworkAPI::new(),
        }
    }
}

/// Terminal operations available to plugins
#[derive(Debug, Clone)]
pub struct TerminalAPI {
    active_sessions: HashMap<String, String>,
}

impl TerminalAPI {
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
        }
    }
    
    pub fn execute_command(&self, command: &str) -> Result<String> {
        // Placeholder - would integrate with actual terminal system
        tracing::info!("Plugin executing command: {}", command);
        Ok(format!("Executed: {}", command))
    }
    
    pub fn get_active_session(&self) -> Option<&String> {
        self.active_sessions.get("current")
    }
    
    pub fn create_session(&mut self, name: String) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.active_sessions.insert(name, session_id.clone());
        Ok(session_id)
    }
}

/// Editor operations available to plugins
#[derive(Debug, Clone)]
pub struct EditorAPI {
    open_files: HashMap<String, String>,
}

impl EditorAPI {
    pub fn new() -> Self {
        Self {
            open_files: HashMap::new(),
        }
    }
    
    pub fn get_current_file(&self) -> Option<&String> {
        self.open_files.get("current")
    }
    
    pub fn get_file_content(&self, path: &str) -> Result<String> {
        // Placeholder - would integrate with actual editor system
        std::fs::read_to_string(path).map_err(Into::into)
    }
    
    pub fn set_file_content(&mut self, path: &str, content: &str) -> Result<()> {
        // Placeholder - would integrate with actual editor system
        std::fs::write(path, content).map_err(Into::into)
    }
    
    pub fn get_cursor_position(&self) -> (u32, u32) {
        // Placeholder - would get from Monaco editor
        (0, 0)
    }
    
    pub fn set_cursor_position(&self, line: u32, column: u32) -> Result<()> {
        // Placeholder - would set in Monaco editor
        tracing::info!("Setting cursor to {}:{}", line, column);
        Ok(())
    }
}

/// Canvas operations available to plugins
#[derive(Debug, Clone)]
pub struct CanvasAPI {
    current_mode: String,
}

impl CanvasAPI {
    pub fn new() -> Self {
        Self {
            current_mode: "freeform".to_string(),
        }
    }
    
    pub fn get_current_mode(&self) -> &str {
        &self.current_mode
    }
    
    pub fn set_mode(&mut self, mode: &str) -> Result<()> {
        match mode {
            "freeform" | "wireframe" | "flowchart" | "system-design" => {
                self.current_mode = mode.to_string();
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Invalid canvas mode: {}", mode)),
        }
    }
    
    pub fn add_element(&self, element_type: &str, x: f64, y: f64) -> Result<String> {
        // Placeholder - would integrate with actual canvas system
        let element_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Adding {} element at ({}, {}) with id {}", element_type, x, y, element_id);
        Ok(element_id)
    }
    
    pub fn export_canvas(&self, format: &str) -> Result<Vec<u8>> {
        // Placeholder - would export actual canvas
        match format {
            "png" | "svg" | "pdf" => {
                Ok(b"placeholder_export_data".to_vec())
            }
            _ => Err(anyhow::anyhow!("Unsupported export format: {}", format)),
        }
    }
}

/// Filesystem operations available to plugins (with permission checks)
#[derive(Debug, Clone)]
pub struct FilesystemAPI {
    allowed_paths: Vec<String>,
}

impl FilesystemAPI {
    pub fn new() -> Self {
        Self {
            allowed_paths: Vec::new(),
        }
    }
    
    pub fn set_allowed_paths(&mut self, paths: Vec<String>) {
        self.allowed_paths = paths;
    }
    
    pub fn read_file(&self, path: &str) -> Result<String> {
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access denied to path: {}", path));
        }
        std::fs::read_to_string(path).map_err(Into::into)
    }
    
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access denied to path: {}", path));
        }
        std::fs::write(path, content).map_err(Into::into)
    }
    
    pub fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access denied to path: {}", path));
        }
        
        let entries = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        
        Ok(entries)
    }
    
    fn is_path_allowed(&self, path: &str) -> bool {
        if self.allowed_paths.is_empty() {
            return false; // Default deny
        }
        
        self.allowed_paths.iter().any(|allowed| {
            path.starts_with(allowed)
        })
    }
}

/// Network operations available to plugins (with domain restrictions)
#[derive(Debug, Clone)]
pub struct NetworkAPI {
    allowed_domains: Vec<String>,
}

impl NetworkAPI {
    pub fn new() -> Self {
        Self {
            allowed_domains: Vec::new(),
        }
    }
    
    pub fn set_allowed_domains(&mut self, domains: Vec<String>) {
        self.allowed_domains = domains;
    }
    
    pub async fn http_get(&self, url: &str) -> Result<String> {
        if !self.is_domain_allowed(url) {
            return Err(anyhow::anyhow!("Access denied to domain: {}", url));
        }
        
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let body = response.text().await?;
        Ok(body)
    }
    
    pub async fn http_post(&self, url: &str, body: &str) -> Result<String> {
        if !self.is_domain_allowed(url) {
            return Err(anyhow::anyhow!("Access denied to domain: {}", url));
        }
        
        let client = reqwest::Client::new();
        let response = client.post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;
        
        let response_body = response.text().await?;
        Ok(response_body)
    }
    
    fn is_domain_allowed(&self, url: &str) -> bool {
        if self.allowed_domains.is_empty() {
            return false; // Default deny
        }
        
        if let Ok(parsed_url) = url::Url::parse(url) {
            if let Some(domain) = parsed_url.domain() {
                return self.allowed_domains.iter().any(|allowed| {
                    domain == allowed || domain.ends_with(&format!(".{}", allowed))
                });
            }
        }
        false
    }
}

/// Memory management for WASM plugins
#[derive(Debug)]
pub struct WasmMemory {
    data: Vec<u8>,
}

impl WasmMemory {
    pub fn new(initial_size: usize) -> Self {
        Self {
            data: vec![0; initial_size],
        }
    }
    
    pub fn read_string(&self, ptr: i32, len: i32) -> Result<String> {
        let start = ptr as usize;
        let end = start + len as usize;
        
        if end > self.data.len() {
            return Err(anyhow::anyhow!("Memory access out of bounds"));
        }
        
        let bytes = &self.data[start..end];
        String::from_utf8(bytes.to_vec()).map_err(Into::into)
    }
    
    pub fn write_string(&mut self, s: &str) -> Result<(i32, i32)> {
        let bytes = s.as_bytes();
        let ptr = self.data.len() as i32;
        self.data.extend_from_slice(bytes);
        Ok((ptr, bytes.len() as i32))
    }
    
    pub fn allocate(&mut self, size: usize) -> i32 {
        let ptr = self.data.len() as i32;
        self.data.resize(self.data.len() + size, 0);
        ptr
    }
}

/// Host function bindings for WASM plugins
pub struct HostFunctions {
    pub api: PluginAPI,
    pub memory: WasmMemory,
}

impl HostFunctions {
    pub fn new() -> Self {
        Self {
            api: PluginAPI::new(),
            memory: WasmMemory::new(1024 * 1024), // 1MB initial
        }
    }
    
    pub fn add_to_linker(&mut self, linker: &mut Linker<()>) -> Result<()> {
        // Terminal functions
        linker.func_wrap("env", "terminal_execute", |_caller: Caller<'_, ()>, cmd_ptr: i32, cmd_len: i32| -> i32 {
            // In real implementation, we'd read from WASM memory and execute
            tracing::info!("Plugin terminal execute: {} bytes at {}", cmd_len, cmd_ptr);
            0 // Success
        })?;
        
        // Editor functions
        linker.func_wrap("env", "editor_get_content", |_caller: Caller<'_, ()>, path_ptr: i32, path_len: i32| -> i32 {
            tracing::info!("Plugin editor get content: {} bytes at {}", path_len, path_ptr);
            0
        })?;
        
        linker.func_wrap("env", "editor_set_content", |_caller: Caller<'_, ()>, path_ptr: i32, path_len: i32, content_ptr: i32, content_len: i32| -> i32 {
            tracing::info!("Plugin editor set content: path={} bytes at {}, content={} bytes at {}", 
                path_len, path_ptr, content_len, content_ptr);
            0
        })?;
        
        // Canvas functions
        linker.func_wrap("env", "canvas_add_element", |_caller: Caller<'_, ()>, type_ptr: i32, type_len: i32, x: f64, y: f64| -> i32 {
            tracing::info!("Plugin canvas add element: type={} bytes at {}, pos=({}, {})", 
                type_len, type_ptr, x, y);
            123 // Mock element ID
        })?;
        
        linker.func_wrap("env", "canvas_set_mode", |_caller: Caller<'_, ()>, mode_ptr: i32, mode_len: i32| -> i32 {
            tracing::info!("Plugin canvas set mode: {} bytes at {}", mode_len, mode_ptr);
            0
        })?;
        
        // Filesystem functions
        linker.func_wrap("env", "fs_read_file", |_caller: Caller<'_, ()>, path_ptr: i32, path_len: i32| -> i32 {
            tracing::info!("Plugin fs read file: {} bytes at {}", path_len, path_ptr);
            0
        })?;
        
        linker.func_wrap("env", "fs_write_file", |_caller: Caller<'_, ()>, path_ptr: i32, path_len: i32, content_ptr: i32, content_len: i32| -> i32 {
            tracing::info!("Plugin fs write file: path={} bytes at {}, content={} bytes at {}", 
                path_len, path_ptr, content_len, content_ptr);
            0
        })?;
        
        // Network functions
        linker.func_wrap("env", "net_http_get", |_caller: Caller<'_, ()>, url_ptr: i32, url_len: i32| -> i32 {
            tracing::info!("Plugin http get: {} bytes at {}", url_len, url_ptr);
            0
        })?;
        
        // Memory management
        linker.func_wrap("env", "allocate", |_caller: Caller<'_, ()>, size: i32| -> i32 {
            tracing::debug!("Plugin allocate: {} bytes", size);
            size // Mock allocation - return same as size for now
        })?;
        
        linker.func_wrap("env", "deallocate", |_caller: Caller<'_, ()>, ptr: i32, size: i32| {
            tracing::debug!("Plugin deallocate: {} bytes at {}", size, ptr);
        })?;
        
        // Logging functions
        linker.func_wrap("env", "log_info", |_caller: Caller<'_, ()>, msg_ptr: i32, msg_len: i32| {
            tracing::info!("Plugin log (info): {} bytes at {}", msg_len, msg_ptr);
        })?;
        
        linker.func_wrap("env", "log_warn", |_caller: Caller<'_, ()>, msg_ptr: i32, msg_len: i32| {
            tracing::warn!("Plugin log (warn): {} bytes at {}", msg_len, msg_ptr);
        })?;
        
        linker.func_wrap("env", "log_error", |_caller: Caller<'_, ()>, msg_ptr: i32, msg_len: i32| {
            tracing::error!("Plugin log (error): {} bytes at {}", msg_len, msg_ptr);
        })?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestV2 {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub entry_point: String,
    pub permissions: Vec<super::Permission>,
    pub api_version: String,
    pub supported_platforms: Vec<String>,
    pub dependencies: HashMap<String, String>,
    pub exports: Vec<PluginExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExport {
    pub name: String,
    pub function_type: String, // "command", "provider", "hook"
    pub description: String,
    pub parameters: Vec<PluginParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub name: String,
    pub param_type: String, // "string", "number", "boolean", "object"
    pub required: bool,
    pub description: String,
    pub default_value: Option<serde_json::Value>,
}