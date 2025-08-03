use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPDiagnostic {
    pub range: LSPRange,
    pub severity: u32, // 1: Error, 2: Warning, 3: Information, 4: Hint
    pub message: String,
    pub source: Option<String>,
    pub code: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPRange {
    pub start: LSPPosition,
    pub end: LSPPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPCompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<LSPDocumentation>,
    #[serde(rename = "insertText")]
    pub insert_text: Option<String>,
    #[serde(rename = "sortText")]
    pub sort_text: Option<String>,
    #[serde(rename = "filterText")]
    pub filter_text: Option<String>,
    #[serde(rename = "preselect")]
    pub preselect: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LSPDocumentation {
    String(String),
    MarkupContent {
        kind: String,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPHover {
    pub contents: Vec<String>,
    pub range: Option<LSPRange>,
}

#[derive(Debug)]
pub struct LSPServer {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
    pub workspace_folders: Vec<PathBuf>,
    process: Option<Child>,
    request_id: Arc<RwLock<u64>>,
    diagnostics: Arc<RwLock<HashMap<String, Vec<LSPDiagnostic>>>>,
    response_sender: Option<mpsc::UnboundedSender<LSPRequest>>,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
}

#[derive(Debug)]
struct LSPRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
    response_tx: oneshot::Sender<serde_json::Value>,
}

impl LSPServer {
    pub fn new(language: String, command: String, args: Vec<String>) -> Self {
        Self {
            language,
            command,
            args,
            workspace_folders: Vec::new(),
            process: None,
            request_id: Arc::new(RwLock::new(0)),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
            response_sender: None,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start(&mut self, workspace_root: PathBuf) -> Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = tokio::process::Command::from(cmd).spawn()?;
        let stdout = child.stdout.take().expect("Failed to get stdout");
        
        // Create channels for handling requests and responses
        let (request_tx, request_rx) = mpsc::unbounded_channel::<LSPRequest>();
        self.response_sender = Some(request_tx);
        
        // Spawn background task to handle LSP communication
        let pending_requests = self.pending_requests.clone();
        let diagnostics = self.diagnostics.clone();
        tokio::spawn(async move {
            Self::handle_lsp_communication(stdout, request_rx, pending_requests, diagnostics).await;
        });
        
        // Initialize the LSP server
        self.send_initialize(&mut child, workspace_root).await?;
        
        self.process = Some(child);
        Ok(())
    }
    
    async fn handle_lsp_communication(
        stdout: ChildStdout,
        mut request_rx: mpsc::UnboundedReceiver<LSPRequest>,
        pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
        diagnostics: Arc<RwLock<HashMap<String, Vec<LSPDiagnostic>>>>,
    ) {
        let mut reader = BufReader::new(stdout);
        let mut buffer = String::new();
        
        loop {
            tokio::select! {
                // Handle incoming requests (not needed for this simplified version)
                _ = request_rx.recv() => {
                    // Requests are handled directly by the methods
                }
                
                // Read LSP messages from stdout
                result = Self::read_lsp_message(&mut reader, &mut buffer) => {
                    match result {
                        Ok(Some(message)) => {
                            Self::process_lsp_message(message, &pending_requests, &diagnostics).await;
                        }
                        Ok(None) => {
                            // EOF reached
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Error reading LSP message: {}", e);
                        }
                    }
                }
            }
        }
    }
    
    async fn read_lsp_message(
        reader: &mut BufReader<ChildStdout>,
        buffer: &mut String,
    ) -> Result<Option<serde_json::Value>> {
        buffer.clear();
        
        // Read Content-Length header
        if reader.read_line(buffer).await? == 0 {
            return Ok(None);
        }
        
        let content_length = if let Some(length_str) = buffer.strip_prefix("Content-Length: ") {
            length_str.trim().parse::<usize>()?
        } else {
            return Err(anyhow::anyhow!("Invalid LSP message format"));
        };
        
        // Read additional headers until empty line
        loop {
            buffer.clear();
            if reader.read_line(buffer).await? == 0 {
                return Ok(None);
            }
            if buffer.trim().is_empty() {
                break;
            }
        }
        
        // Read the JSON content
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).await?;
        
        let json_str = String::from_utf8(content)?;
        let message: serde_json::Value = serde_json::from_str(&json_str)?;
        
        Ok(Some(message))
    }
    
    async fn process_lsp_message(
        message: serde_json::Value,
        pending_requests: &Arc<RwLock<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
        diagnostics: &Arc<RwLock<HashMap<String, Vec<LSPDiagnostic>>>>,
    ) {
        if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
            // Handle notifications
            match method {
                "textDocument/publishDiagnostics" => {
                    if let Some(params) = message.get("params") {
                        Self::handle_diagnostics(params, diagnostics).await;
                    }
                }
                _ => {
                    // Other notifications can be handled here
                }
            }
        } else if let Some(id) = message.get("id").and_then(|i| i.as_u64()) {
            // Handle responses to our requests
            let mut pending = pending_requests.write().await;
            if let Some(sender) = pending.remove(&id) {
                let _ = sender.send(message);
            }
        }
    }
    
    async fn handle_diagnostics(
        params: &serde_json::Value,
        diagnostics: &Arc<RwLock<HashMap<String, Vec<LSPDiagnostic>>>>,
    ) {
        if let (Some(uri), Some(diags_json)) = (
            params.get("uri").and_then(|u| u.as_str()),
            params.get("diagnostics").and_then(|d| d.as_array())
        ) {
            let mut parsed_diagnostics = Vec::new();
            
            for diag in diags_json {
                if let Ok(diagnostic) = serde_json::from_value::<LSPDiagnostic>(diag.clone()) {
                    parsed_diagnostics.push(diagnostic);
                }
            }
            
            let mut diag_map = diagnostics.write().await;
            diag_map.insert(uri.to_string(), parsed_diagnostics);
        }
    }
    
    async fn send_initialize(&self, child: &mut Child, workspace_root: PathBuf) -> Result<()> {
        let stdin = child.stdin.as_mut().unwrap();
        
        let initialize_params = serde_json::json!({
            "processId": std::process::id(),
            "rootPath": workspace_root.to_string_lossy(),
            "rootUri": format!("file://{}", workspace_root.to_string_lossy()),
            "capabilities": {
                "workspace": {
                    "applyEdit": true,
                    "workspaceEdit": {
                        "documentChanges": true
                    },
                    "didChangeConfiguration": {
                        "dynamicRegistration": true
                    },
                    "didChangeWatchedFiles": {
                        "dynamicRegistration": true
                    },
                    "symbol": {
                        "dynamicRegistration": true
                    },
                    "executeCommand": {
                        "dynamicRegistration": true
                    }
                },
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "versionSupport": false,
                        "tagSupport": {
                            "valueSet": [1, 2]
                        }
                    },
                    "synchronization": {
                        "dynamicRegistration": true,
                        "willSave": true,
                        "willSaveWaitUntil": true,
                        "didSave": true
                    },
                    "completion": {
                        "dynamicRegistration": true,
                        "contextSupport": true,
                        "completionItem": {
                            "snippetSupport": true,
                            "commitCharactersSupport": true,
                            "documentationFormat": ["markdown", "plaintext"],
                            "deprecatedSupport": true,
                            "preselectSupport": true
                        },
                        "completionItemKind": {
                            "valueSet": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
                        }
                    },
                    "hover": {
                        "dynamicRegistration": true,
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "signatureHelp": {
                        "dynamicRegistration": true,
                        "signatureInformation": {
                            "documentationFormat": ["markdown", "plaintext"]
                        }
                    },
                    "definition": {
                        "dynamicRegistration": true
                    },
                    "references": {
                        "dynamicRegistration": true
                    },
                    "documentHighlight": {
                        "dynamicRegistration": true
                    },
                    "documentSymbol": {
                        "dynamicRegistration": true
                    },
                    "codeAction": {
                        "dynamicRegistration": true
                    },
                    "codeLens": {
                        "dynamicRegistration": true
                    },
                    "formatting": {
                        "dynamicRegistration": true
                    },
                    "rangeFormatting": {
                        "dynamicRegistration": true
                    },
                    "onTypeFormatting": {
                        "dynamicRegistration": true
                    },
                    "rename": {
                        "dynamicRegistration": true
                    }
                }
            },
            "trace": "off",
            "workspaceFolders": [{
                "uri": format!("file://{}", workspace_root.to_string_lossy()),
                "name": workspace_root.file_name().unwrap_or_default().to_string_lossy()
            }]
        });
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": initialize_params
        });
        
        Self::send_message(stdin, &request).await?;
        
        // Send initialized notification
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        
        Self::send_message(stdin, &initialized).await?;
        
        Ok(())
    }
    
    async fn send_message(stdin: &mut ChildStdin, message: &serde_json::Value) -> Result<()> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(content.as_bytes()).await?;
        stdin.flush().await?;
        
        Ok(())
    }
    
    pub async fn did_open(&mut self, uri: String, language_id: String, content: String) -> Result<()> {
        if let Some(child) = &mut self.process {
            let stdin = child.stdin.as_mut().unwrap();
            
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": language_id,
                        "version": 1,
                        "text": content
                    }
                }
            });
            
            Self::send_message(stdin, &notification).await?;
        }
        
        Ok(())
    }
    
    pub async fn did_change(&mut self, uri: String, content: String, version: u64) -> Result<()> {
        if let Some(child) = &mut self.process {
            let stdin = child.stdin.as_mut().unwrap();
            
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": version
                    },
                    "contentChanges": [{
                        "text": content
                    }]
                }
            });
            
            Self::send_message(stdin, &notification).await?;
        }
        
        Ok(())
    }
    
    pub async fn completion(&mut self, uri: String, line: u32, character: u32) -> Result<Vec<LSPCompletionItem>> {
        if let Some(child) = &mut self.process {
            let stdin = child.stdin.as_mut().unwrap();
            let mut request_id = self.request_id.write().await;
            *request_id += 1;
            let id = *request_id;
            
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {
                        "uri": uri
                    },
                    "position": {
                        "line": line,
                        "character": character
                    },
                    "context": {
                        "triggerKind": 1
                    }
                }
            });
            
            // Create response channel
            let (response_tx, response_rx) = oneshot::channel();
            {
                let mut pending = self.pending_requests.write().await;
                pending.insert(id, response_tx);
            }
            
            Self::send_message(stdin, &request).await?;
            
            // Wait for response with timeout
            match timeout(Duration::from_secs(5), response_rx).await {
                Ok(Ok(response)) => {
                    Self::parse_completion_response(response)
                }
                Ok(Err(_)) => {
                    tracing::warn!("LSP completion response channel closed");
                    Ok(Vec::new())
                }
                Err(_) => {
                    tracing::warn!("LSP completion request timed out");
                    // Clean up pending request
                    let mut pending = self.pending_requests.write().await;
                    pending.remove(&id);
                    Ok(Vec::new())
                }
            }
        } else {
            Ok(Vec::new())
        }
    }
    
    fn parse_completion_response(response: serde_json::Value) -> Result<Vec<LSPCompletionItem>> {
        let mut items = Vec::new();
        
        if let Some(result) = response.get("result") {
            // Handle both CompletionList and CompletionItem[] formats
            let empty_vec = vec![];
            let completion_items = if let Some(list) = result.get("items") {
                // CompletionList format
                list.as_array().unwrap_or(&empty_vec)
            } else if result.is_array() {
                // Direct CompletionItem[] format
                result.as_array().unwrap_or(&empty_vec)
            } else {
                return Ok(items);
            };
            
            for item_json in completion_items {
                if let Ok(item) = serde_json::from_value::<LSPCompletionItem>(item_json.clone()) {
                    items.push(item);
                }
            }
        }
        
        Ok(items)
    }
    
    pub async fn hover(&mut self, uri: String, line: u32, character: u32) -> Result<Option<LSPHover>> {
        if let Some(child) = &mut self.process {
            let stdin = child.stdin.as_mut().unwrap();
            let mut request_id = self.request_id.write().await;
            *request_id += 1;
            let id = *request_id;
            
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": uri
                    },
                    "position": {
                        "line": line,
                        "character": character
                    }
                }
            });
            
            // Create response channel
            let (response_tx, response_rx) = oneshot::channel();
            {
                let mut pending = self.pending_requests.write().await;
                pending.insert(id, response_tx);
            }
            
            Self::send_message(stdin, &request).await?;
            
            // Wait for response with timeout
            match timeout(Duration::from_secs(5), response_rx).await {
                Ok(Ok(response)) => {
                    Ok(Self::parse_hover_response(response)?)
                }
                Ok(Err(_)) => {
                    tracing::warn!("LSP hover response channel closed");
                    Ok(None)
                }
                Err(_) => {
                    tracing::warn!("LSP hover request timed out");
                    // Clean up pending request
                    let mut pending = self.pending_requests.write().await;
                    pending.remove(&id);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
    
    fn parse_hover_response(response: serde_json::Value) -> Result<Option<LSPHover>> {
        if let Some(result) = response.get("result") {
            if result.is_null() {
                return Ok(None);
            }
            
            let contents = if let Some(contents_json) = result.get("contents") {
                let mut contents = Vec::new();
                
                // Handle different formats of hover contents
                match contents_json {
                    serde_json::Value::String(s) => {
                        contents.push(s.clone());
                    }
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            match item {
                                serde_json::Value::String(s) => contents.push(s.clone()),
                                serde_json::Value::Object(obj) => {
                                    // Handle MarkupContent format
                                    if let Some(value) = obj.get("value").and_then(|v| v.as_str()) {
                                        contents.push(value.to_string());
                                    } else if let Some(content) = obj.get("contents").and_then(|v| v.as_str()) {
                                        contents.push(content.to_string());
                                    } else if let Some(language) = obj.get("language").and_then(|v| v.as_str()) {
                                        if let Some(code_value) = obj.get("value").and_then(|v| v.as_str()) {
                                            contents.push(format!("```{}\n{}\n```", language, code_value));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    serde_json::Value::Object(obj) => {
                        // Handle single MarkupContent or MarkedString
                        if let Some(kind) = obj.get("kind").and_then(|k| k.as_str()) {
                            if let Some(value) = obj.get("value").and_then(|v| v.as_str()) {
                                if kind == "markdown" {
                                    contents.push(value.to_string());
                                } else {
                                    contents.push(value.to_string());
                                }
                            }
                        } else if let Some(language) = obj.get("language").and_then(|v| v.as_str()) {
                            if let Some(code_value) = obj.get("value").and_then(|v| v.as_str()) {
                                contents.push(format!("```{}\n{}\n```", language, code_value));
                            }
                        } else if let Some(value) = obj.get("value").and_then(|v| v.as_str()) {
                            contents.push(value.to_string());
                        }
                    }
                    _ => {}
                }
                
                contents
            } else {
                Vec::new()
            };
            
            let range = result.get("range")
                .and_then(|r| serde_json::from_value(r.clone()).ok());
            
            Ok(Some(LSPHover { contents, range }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_diagnostics(&self, uri: &str) -> Option<Vec<LSPDiagnostic>> {
        let diagnostics = self.diagnostics.read().await;
        diagnostics.get(uri).cloned()
    }
    
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(child) = &mut self.process {
            let stdin = child.stdin.as_mut().unwrap();
            
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 999,
                "method": "shutdown",
                "params": null
            });
            
            Self::send_message(stdin, &request).await?;
            
            let exit = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit"
            });
            
            Self::send_message(stdin, &exit).await?;
            
            child.kill().await?;
        }
        
        Ok(())
    }
}

pub struct LSPManager {
    servers: Arc<RwLock<HashMap<String, LSPServer>>>,
    language_configs: HashMap<String, LSPConfig>,
}

#[derive(Debug, Clone)]
pub struct LSPConfig {
    pub command: String,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
}

impl LSPManager {
    pub fn new() -> Self {
        let mut language_configs = HashMap::new();
        
        // Rust LSP (rust-analyzer)
        language_configs.insert("rust".to_string(), LSPConfig {
            command: "rust-analyzer".to_string(),
            args: vec![],
            file_extensions: vec!["rs".to_string()],
        });
        
        // TypeScript LSP
        language_configs.insert("typescript".to_string(), LSPConfig {
            command: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["ts".to_string(), "tsx".to_string()],
        });
        
        // JavaScript LSP (same as TypeScript)
        language_configs.insert("javascript".to_string(), LSPConfig {
            command: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["js".to_string(), "jsx".to_string()],
        });
        
        // Python LSP (pylsp)
        language_configs.insert("python".to_string(), LSPConfig {
            command: "pylsp".to_string(),
            args: vec![],
            file_extensions: vec!["py".to_string()],
        });
        
        // Go LSP (gopls)
        language_configs.insert("go".to_string(), LSPConfig {
            command: "gopls".to_string(),
            args: vec![],
            file_extensions: vec!["go".to_string()],
        });
        
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            language_configs,
        }
    }
    
    pub async fn ensure_server(&self, language: &str, workspace_root: PathBuf) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        if !servers.contains_key(language) {
            if let Some(config) = self.language_configs.get(language) {
                let mut server = LSPServer::new(
                    language.to_string(),
                    config.command.clone(),
                    config.args.clone(),
                );
                
                // Try to start the server
                match server.start(workspace_root).await {
                    Ok(_) => {
                        servers.insert(language.to_string(), server);
                        tracing::info!("Started LSP server for {}", language);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to start LSP server for {}: {}", language, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn did_open_document(&self, uri: String, language: String, content: String) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(&language) {
            server.did_open(uri, language, content).await?;
        }
        
        Ok(())
    }
    
    pub async fn did_change_document(&self, uri: String, language: String, content: String, version: u64) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(&language) {
            server.did_change(uri, content, version).await?;
        }
        
        Ok(())
    }
    
    pub async fn get_completion(&self, language: String, uri: String, line: u32, character: u32) -> Result<Vec<LSPCompletionItem>> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(&language) {
            server.completion(uri, line, character).await
        } else {
            Ok(Vec::new())
        }
    }
    
    pub async fn get_hover(&self, language: String, uri: String, line: u32, character: u32) -> Result<Option<LSPHover>> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(&language) {
            server.hover(uri, line, character).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_diagnostics(&self, language: &str, uri: &str) -> Option<Vec<LSPDiagnostic>> {
        let servers = self.servers.read().await;
        
        if let Some(server) = servers.get(language) {
            server.get_diagnostics(uri).await
        } else {
            None
        }
    }
    
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        for (_, server) in servers.iter_mut() {
            if let Err(e) = server.shutdown().await {
                tracing::warn!("Failed to shutdown LSP server: {}", e);
            }
        }
        
        servers.clear();
        Ok(())
    }
    
    pub fn get_language_for_extension(&self, extension: &str) -> Option<String> {
        for (language, config) in &self.language_configs {
            if config.file_extensions.contains(&extension.to_string()) {
                return Some(language.clone());
            }
        }
        None
    }
}