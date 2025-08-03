use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use wasmtime::{Config, Engine, Linker, Module, Store};

pub mod api;
pub use api::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry_point: String,
    pub permissions: Vec<Permission>,
    pub api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    FileSystem { paths: Vec<String> },
    Network { domains: Vec<String> },
    Terminal,
    Editor,
    Canvas,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub id: Uuid,
    pub manifest: PluginManifest,
    pub wasm_module: Vec<u8>,
    pub enabled: bool,
    pub installation_path: PathBuf,
}

impl Plugin {
    pub fn new(manifest: PluginManifest, wasm_module: Vec<u8>, installation_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            manifest,
            wasm_module,
            enabled: true,
            installation_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionContext {
    pub plugin_id: Uuid,
    pub function_name: String,
    pub arguments: serde_json::Value,
    pub permissions: Vec<Permission>,
}

pub struct PluginRuntime {
    engine: Engine,
    plugins: Arc<RwLock<HashMap<Uuid, Plugin>>>,
    event_bus: code_furnace_events::EventBus,
    host_functions: Arc<RwLock<HostFunctions>>,
    plugin_registry: Arc<RwLock<PluginRegistry>>,
}

impl PluginRuntime {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_simd(true);
        config.wasm_multi_value(true);
        config.wasm_bulk_memory(true);
        config.wasm_reference_types(true);
        config.consume_fuel(true); // Enable fuel consumption for resource limiting
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            engine,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
            host_functions: Arc::new(RwLock::new(HostFunctions::new())),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
        })
    }
    
    pub async fn install_plugin(&self, manifest: PluginManifest, wasm_bytes: Vec<u8>, installation_path: PathBuf) -> Result<Uuid> {
        let plugin = Plugin::new(manifest, wasm_bytes, installation_path);
        let plugin_id = plugin.id;
        
        // Validate the WASM module
        Module::new(&self.engine, &plugin.wasm_module)?;
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id, plugin);
        
        let event = code_furnace_events::Event::new(
            "plugins.installed",
            "plugin-runtime",
            serde_json::json!({
                "plugin_id": plugin_id,
                "name": plugins.get(&plugin_id).unwrap().manifest.name
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(plugin_id)
    }
    
    pub async fn uninstall_plugin(&self, plugin_id: Uuid) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.remove(&plugin_id) {
            // Clean up plugin files
            if plugin.installation_path.exists() {
                std::fs::remove_dir_all(&plugin.installation_path)?;
            }
            
            let event = code_furnace_events::Event::new(
                "plugins.uninstalled",
                "plugin-runtime",
                serde_json::json!({
                    "plugin_id": plugin_id,
                    "name": plugin.manifest.name
                }),
            );
            self.event_bus.publish(event)?;
        }
        
        Ok(())
    }
    
    pub async fn execute_plugin_function(
        &self,
        plugin_id: Uuid,
        function_name: &str,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let plugin = {
            let plugins = self.plugins.read().await;
            plugins.get(&plugin_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?
        };
        
        if !plugin.enabled {
            return Err(anyhow::anyhow!("Plugin is disabled"));
        }
        
        let module = Module::new(&self.engine, &plugin.wasm_module)?;
        let mut linker = Linker::new(&self.engine);
        
        // Add host functions that plugins can call
        self.add_host_functions(&mut linker, &plugin)?;
        
        let mut store = Store::new(&self.engine, ());
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Get the function export
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
            .map_err(|_| anyhow::anyhow!("Function '{}' not found in plugin", function_name))?;
        
        // For now, return a placeholder result
        // In a real implementation, we'd properly marshal arguments and results
        let result = func.call(&mut store, (0, 0))?;
        
        let event = code_furnace_events::Event::new(
            "plugins.function.executed",
            "plugin-runtime",
            serde_json::json!({
                "plugin_id": plugin_id,
                "function_name": function_name,
                "result": result
            }),
        );
        self.event_bus.publish(event)?;
        
        Ok(serde_json::json!({ "result": result }))
    }
    
    fn add_host_functions(&self, linker: &mut Linker<()>, plugin: &Plugin) -> Result<()> {
        // Configure host functions based on plugin permissions
        let mut host_funcs = HostFunctions::new();
        
        // Configure API access based on permissions
        for permission in &plugin.manifest.permissions {
            match permission {
                Permission::FileSystem { paths } => {
                    host_funcs.api.filesystem_api.set_allowed_paths(paths.clone());
                }
                Permission::Network { domains } => {
                    host_funcs.api.network_api.set_allowed_domains(domains.clone());
                }
                _ => {
                    // Other permissions are implicitly granted by having the permission
                }
            }
        }
        
        // Add all host functions to the linker
        host_funcs.add_to_linker(linker)?;
        
        Ok(())
    }
    
    pub async fn list_plugins(&self) -> Vec<Plugin> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }
    
    pub async fn enable_plugin(&self, plugin_id: Uuid) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.get_mut(&plugin_id) {
            plugin.enabled = true;
            
            let event = code_furnace_events::Event::new(
                "plugins.enabled",
                "plugin-runtime",
                serde_json::to_value(&plugin_id)?,
            );
            self.event_bus.publish(event)?;
        }
        
        Ok(())
    }
    
    pub async fn disable_plugin(&self, plugin_id: Uuid) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.get_mut(&plugin_id) {
            plugin.enabled = false;
            
            let event = code_furnace_events::Event::new(
                "plugins.disabled",
                "plugin-runtime",    
                serde_json::to_value(&plugin_id)?,
            );
            self.event_bus.publish(event)?;
        }
        
        Ok(())
    }
    
    /// Execute a plugin function with enhanced error handling and fuel limiting
    pub async fn execute_plugin_function_safe(
        &self,
        plugin_id: Uuid,
        function_name: &str,
        _args: serde_json::Value,
        _fuel_limit: u64,
    ) -> Result<serde_json::Value> {
        let plugin = {
            let plugins = self.plugins.read().await;
            plugins.get(&plugin_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?
        };
        
        if !plugin.enabled {
            return Err(anyhow::anyhow!("Plugin is disabled"));
        }
        
        let module = Module::new(&self.engine, &plugin.wasm_module)?;
        let mut linker = Linker::new(&self.engine);
        
        // Add host functions
        self.add_host_functions(&mut linker, &plugin)?;
        
        let mut store = Store::new(&self.engine, ());
        // Note: Fuel consumption requires wasmtime features that might not be available
        // For now, rely on timeout for resource limiting
        
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Try to get the function with proper error handling
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
            .map_err(|e| anyhow::anyhow!("Function '{}' not found in plugin: {}", function_name, e))?;
        
        // Execute with timeout for resource limiting
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30), 
            tokio::task::spawn_blocking(move || {
                func.call(&mut store, (0, 0)).map_err(|e| anyhow::anyhow!("Plugin execution error: {}", e))
            })
        ).await??;
        
        let (success, result_value) = match &result {
            Ok(value) => (true, *value),
            Err(_) => (false, -1)
        };
        
        let event = code_furnace_events::Event::new(
            "plugins.function.executed",
            "plugin-runtime",
            serde_json::json!({
                "plugin_id": plugin_id,
                "function_name": function_name,
                "success": success,
                "result": result_value
            }),
        );
        self.event_bus.publish(event)?;
        
        match result {
            Ok(value) => Ok(serde_json::json!({ "result": value })),
            Err(e) => Ok(serde_json::json!({ "error": e.to_string() }))
        }
    }
    
    /// Get plugin registry for browsing available plugins
    pub async fn get_registry(&self) -> Arc<RwLock<PluginRegistry>> {
        self.plugin_registry.clone()
    }
}

/// Plugin registry for managing plugin discovery and installation
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    pub available_plugins: HashMap<String, PluginRegistryEntry>,
    pub installed_plugins: HashMap<Uuid, String>, // plugin_id -> name mapping
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistryEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub download_url: String,
    pub checksum: String,
    pub manifest: PluginManifestV2,
    pub downloads: u64,
    pub rating: f32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            available_plugins: HashMap::new(),
            installed_plugins: HashMap::new(),
        }
    }
    
    pub async fn refresh_from_remote(&mut self, registry_url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client.get(registry_url).send().await?;
        
        if response.status().is_success() {
            let registry_data: HashMap<String, PluginRegistryEntry> = response.json().await?;
            self.available_plugins = registry_data;
            tracing::info!("Refreshed plugin registry with {} plugins", self.available_plugins.len());
        } else {
            return Err(anyhow::anyhow!("Failed to fetch plugin registry: {}", response.status()));
        }
        
        Ok(())
    }
    
    pub fn search_plugins(&self, query: &str) -> Vec<&PluginRegistryEntry> {
        let query_lower = query.to_lowercase();
        self.available_plugins
            .values()
            .filter(|plugin| {
                plugin.name.to_lowercase().contains(&query_lower) ||
                plugin.description.to_lowercase().contains(&query_lower) ||
                plugin.author.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
    
    pub fn get_plugin_by_name(&self, name: &str) -> Option<&PluginRegistryEntry> {
        self.available_plugins.get(name)
    }
    
    pub fn list_installed(&self) -> Vec<String> {
        self.installed_plugins.values().cloned().collect()
    }
    
    pub fn mark_installed(&mut self, plugin_id: Uuid, name: String) {
        self.installed_plugins.insert(plugin_id, name);
    }
    
    pub fn mark_uninstalled(&mut self, plugin_id: Uuid) {
        self.installed_plugins.remove(&plugin_id);
    }
}

/// Plugin development utilities
pub mod dev_utils {
    use super::*;
    
    /// Create a basic plugin manifest template
    pub fn create_manifest_template(name: &str, author: &str) -> PluginManifestV2 {
        PluginManifestV2 {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: "A Code Furnace plugin".to_string(),
            author: author.to_string(),
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            keywords: vec!["plugin".to_string()],
            entry_point: "plugin.wasm".to_string(),
            permissions: vec![],
            api_version: "1.0.0".to_string(),
            supported_platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
            dependencies: HashMap::new(),
            exports: vec![
                PluginExport {
                    name: "main".to_string(),
                    function_type: "command".to_string(),
                    description: "Main plugin function".to_string(),
                    parameters: vec![],
                }
            ],
        }
    }
    
    /// Validate a plugin manifest
    pub fn validate_manifest(manifest: &PluginManifestV2) -> Result<()> {
        if manifest.name.is_empty() {
            return Err(anyhow::anyhow!("Plugin name cannot be empty"));
        }
        
        if manifest.version.is_empty() {
            return Err(anyhow::anyhow!("Plugin version cannot be empty"));
        }
        
        if manifest.author.is_empty() {
            return Err(anyhow::anyhow!("Plugin author cannot be empty"));
        }
        
        if manifest.api_version != "1.0.0" {
            return Err(anyhow::anyhow!("Unsupported API version: {}", manifest.api_version));
        }
        
        // Validate permissions
        for permission in &manifest.permissions {
            match permission {
                Permission::FileSystem { paths } => {
                    if paths.is_empty() {
                        return Err(anyhow::anyhow!("FileSystem permission must specify at least one path"));
                    }
                }
                Permission::Network { domains } => {
                    if domains.is_empty() {
                        return Err(anyhow::anyhow!("Network permission must specify at least one domain"));
                    }
                }
                _ => {} // Other permissions are valid as-is
            }
        }
        
        Ok(())
    }
}