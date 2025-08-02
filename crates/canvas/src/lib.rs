use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: Uuid,
    pub element_type: ElementType,
    pub position: Point,
    pub size: Size,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Rectangle,
    Circle,
    Text,
    Component,
    Wireframe,
    FlowchartNode,
    FlowchartEdge,
    StickyNote,
    Image,
}

impl CanvasElement {
    pub fn new(element_type: ElementType, position: Point, size: Size) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            element_type,
            position,
            size,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }
    
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub id: Uuid,
    pub name: String,
    pub mode: CanvasMode,
    pub elements: HashMap<Uuid, CanvasElement>,
    pub viewport: Viewport,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CanvasMode {
    Freeform,
    Wireframe,
    Flowchart,
    SystemDesign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Canvas {
    pub fn new(name: String, mode: CanvasMode) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            mode,
            elements: HashMap::new(),
            viewport: Viewport::default(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn add_element(&mut self, element: CanvasElement) {
        self.elements.insert(element.id, element);
        self.updated_at = chrono::Utc::now();
    }
    
    pub fn remove_element(&mut self, element_id: Uuid) -> Option<CanvasElement> {
        let element = self.elements.remove(&element_id);
        if element.is_some() {
            self.updated_at = chrono::Utc::now();
        }
        element
    }
    
    pub fn update_element(&mut self, element_id: Uuid, updater: impl FnOnce(&mut CanvasElement)) {
        if let Some(element) = self.elements.get_mut(&element_id) {
            updater(element);
            self.updated_at = chrono::Utc::now();
        }
    }
    
    pub fn export_to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    
    pub fn import_from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    
    pub fn export_to_mermaid(&self) -> Result<String> {
        match self.mode {
            CanvasMode::Flowchart => self.export_flowchart_to_mermaid(),
            CanvasMode::SystemDesign => self.export_system_design_to_mermaid(),
            _ => Err(anyhow::anyhow!("Mermaid export not supported for this canvas mode")),
        }
    }
    
    fn export_flowchart_to_mermaid(&self) -> Result<String> {
        let mut mermaid = String::from("flowchart TD\n");
        
        for element in self.elements.values() {
            match element.element_type {
                ElementType::FlowchartNode => {
                    let label = element
                        .get_property("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Node");
                    mermaid.push_str(&format!("    {}[\"{}\"]\n", element.id, label));
                }
                ElementType::FlowchartEdge => {
                    let from = element
                        .get_property("from")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let to = element
                        .get_property("to")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !from.is_empty() && !to.is_empty() {
                        mermaid.push_str(&format!("    {} --> {}\n", from, to));
                    }
                }
                _ => {}
            }
        }
        
        Ok(mermaid)
    }
    
    fn export_system_design_to_mermaid(&self) -> Result<String> {
        let mut mermaid = String::from("graph TB\n");
        
        for element in self.elements.values() {
            if let Some(label) = element.get_property("label").and_then(|v| v.as_str()) {
                mermaid.push_str(&format!("    {}[{}]\n", element.id, label));
            }
        }
        
        Ok(mermaid)
    }
}