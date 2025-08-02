use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store, TypedFunc};

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
}

impl PluginRuntime {
    pub fn new(event_bus: code_furnace_events::EventBus) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_simd(true);
        config.wasm_multi_value(true);
        config.wasm_bulk_memory(true);
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            engine,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
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
        args: serde_json::Value,
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
        // Add host functions based on plugin permissions
        for permission in &plugin.manifest.permissions {
            match permission {
                Permission::FileSystem { .. } => {
                    // Add file system functions
                    linker.func_wrap("env", "read_file", |_caller: wasmtime::Caller<'_, ()>, _path_ptr: i32, _path_len: i32| -> i32 {
                        // Placeholder implementation
                        0
                    })?;
                }
                Permission::Terminal => {
                    // Add terminal functions
                    linker.func_wrap("env", "execute_command", |_caller: wasmtime::Caller<'_, ()>, _cmd_ptr: i32, _cmd_len: i32| -> i32 {
                        // Placeholder implementation
                        0
                    })?;
                }
                _ => {
                    // Add other permission-based functions
                }
            }
        }
        
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
}