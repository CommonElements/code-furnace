use crate::{AgentProvider, AgentRequest, AgentResponse};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    CodeExplainer,
    CodeReviewer,
    TestGenerator,
    GitAssistant,
    UIDesigner,
    SystemArchitect,
    DocumentationWriter,
    Debugger,
}

impl AgentType {
    pub fn get_system_prompt(&self) -> &'static str {
        match self {
            AgentType::CodeExplainer => {
                "You are a code explanation specialist. Your role is to analyze code and provide clear, comprehensive explanations of how it works. Focus on:\n\
                - Breaking down complex logic into simple terms\n\
                - Explaining the purpose and functionality of each component\n\
                - Identifying patterns, algorithms, and design principles\n\
                - Highlighting potential issues or improvements\n\
                - Providing context about how the code fits into the larger system"
            }
            AgentType::CodeReviewer => {
                "You are a senior code reviewer. Your role is to provide thorough, constructive code reviews. Focus on:\n\
                - Code quality, readability, and maintainability\n\
                - Performance implications and optimizations\n\
                - Security vulnerabilities and best practices\n\
                - Architectural concerns and design patterns\n\
                - Testing coverage and edge cases\n\
                - Documentation and naming conventions"
            }
            AgentType::TestGenerator => {
                "You are a test automation specialist. Your role is to generate comprehensive test suites. Focus on:\n\
                - Unit tests for individual functions and methods\n\
                - Integration tests for component interactions\n\
                - Edge cases and error handling scenarios\n\
                - Mock data and test fixtures\n\
                - Test organization and maintainability\n\
                - Performance and load testing considerations"
            }
            AgentType::GitAssistant => {
                "You are a Git workflow specialist. Your role is to help with version control tasks. Focus on:\n\
                - Analyzing code changes and generating meaningful commit messages\n\
                - Suggesting branch strategies and merge approaches\n\
                - Identifying potential conflicts and resolution strategies\n\
                - Code history analysis and change impact assessment\n\
                - Release notes and changelog generation"
            }
            AgentType::UIDesigner => {
                "You are a UI/UX design specialist. Your role is to help with interface design and user experience. Focus on:\n\
                - Component design and layout suggestions\n\
                - Accessibility and usability best practices\n\
                - Design system consistency and patterns\n\
                - Responsive design and cross-platform compatibility\n\
                - User flow optimization and interaction design"
            }
            AgentType::SystemArchitect => {
                "You are a system architecture specialist. Your role is to design and analyze system architectures. Focus on:\n\
                - High-level system design and component relationships\n\
                - Scalability, performance, and reliability considerations\n\
                - Technology stack recommendations and trade-offs\n\
                - Integration patterns and API design\n\
                - Security architecture and data flow analysis"
            }
            AgentType::DocumentationWriter => {
                "You are a technical documentation specialist. Your role is to create clear, comprehensive documentation. Focus on:\n\
                - API documentation and usage examples\n\
                - User guides and tutorials\n\
                - Code comments and inline documentation\n\
                - Architecture decision records and design docs\n\
                - README files and project setup instructions"
            }
            AgentType::Debugger => {
                "You are a debugging and troubleshooting specialist. Your role is to help identify and resolve issues. Focus on:\n\
                - Error analysis and root cause identification\n\
                - Step-by-step debugging strategies\n\
                - Log analysis and diagnostic techniques\n\
                - Performance bottleneck identification\n\
                - Monitoring and observability recommendations"
            }
        }
    }
    
    pub fn get_capabilities(&self) -> Vec<&'static str> {
        match self {
            AgentType::CodeExplainer => vec![
                "code_analysis", "explain_algorithms", "identify_patterns", "suggest_improvements"
            ],
            AgentType::CodeReviewer => vec![
                "quality_assessment", "security_review", "performance_analysis", "best_practices"
            ],
            AgentType::TestGenerator => vec![
                "unit_tests", "integration_tests", "mock_generation", "test_strategies"
            ],
            AgentType::GitAssistant => vec![
                "commit_messages", "diff_analysis", "merge_strategies", "release_notes"
            ],
            AgentType::UIDesigner => vec![
                "component_design", "accessibility", "responsive_design", "user_experience"
            ],
            AgentType::SystemArchitect => vec![
                "architecture_design", "scalability_planning", "integration_patterns", "tech_stack"
            ],
            AgentType::DocumentationWriter => vec![
                "api_docs", "user_guides", "tutorials", "technical_writing"
            ],
            AgentType::Debugger => vec![
                "error_analysis", "troubleshooting", "performance_debugging", "log_analysis"
            ],
        }
    }
}

pub struct SpecializedAgent {
    agent_type: AgentType,
    base_provider: Box<dyn AgentProvider>,
}

impl SpecializedAgent {
    pub fn new(agent_type: AgentType, base_provider: Box<dyn AgentProvider>) -> Self {
        Self {
            agent_type,
            base_provider,
        }
    }
    
    fn enhance_prompt(&self, request: &AgentRequest) -> String {
        let mut enhanced_prompt = String::new();
        
        // Add system prompt
        enhanced_prompt.push_str(&format!("{}\n\n", self.agent_type.get_system_prompt()));
        
        // Add context about the current task
        enhanced_prompt.push_str("Current task context:\n");
        
        // Add file context if provided
        if !request.files.is_empty() {
            enhanced_prompt.push_str(&format!("Files being analyzed: {}\n", request.files.join(", ")));
        }
        
        // Add any additional context from the request
        if !request.context.is_empty() {
            enhanced_prompt.push_str("Additional context:\n");
            for (key, value) in &request.context {
                enhanced_prompt.push_str(&format!("- {}: {}\n", key, value));
            }
        }
        
        enhanced_prompt.push_str("\n---\n\n");
        enhanced_prompt.push_str(&request.prompt);
        
        enhanced_prompt
    }
}

#[async_trait]
impl AgentProvider for SpecializedAgent {
    async fn process_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        // Create an enhanced request with specialized prompting
        let mut enhanced_request = request.clone();
        enhanced_request.prompt = self.enhance_prompt(request);
        
        // Add agent type to metadata
        let mut response = self.base_provider.process_request(&enhanced_request).await?;
        response.metadata.insert(
            "agent_type".to_string(),
            serde_json::to_value(&self.agent_type)?,
        );
        response.metadata.insert(
            "capabilities".to_string(),
            serde_json::to_value(self.agent_type.get_capabilities())?,
        );
        
        Ok(response)
    }
    
    fn provider_name(&self) -> &str {
        match self.agent_type {
            AgentType::CodeExplainer => "code-explainer",
            AgentType::CodeReviewer => "code-reviewer",
            AgentType::TestGenerator => "test-generator",
            AgentType::GitAssistant => "git-assistant",
            AgentType::UIDesigner => "ui-designer",
            AgentType::SystemArchitect => "system-architect",
            AgentType::DocumentationWriter => "doc-writer",
            AgentType::Debugger => "debugger",
        }
    }
    
    fn supports_streaming(&self) -> bool {
        self.base_provider.supports_streaming()
    }
}

pub struct AgentRouter {
    agents: HashMap<String, Box<dyn AgentProvider>>,
    default_agent: String,
}

impl AgentRouter {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            default_agent: "general".to_string(),
        }
    }
    
    pub fn register_agent(&mut self, name: String, agent: Box<dyn AgentProvider>) {
        self.agents.insert(name, agent);
    }
    
    pub fn set_default_agent(&mut self, name: String) {
        self.default_agent = name;
    }
    
    pub fn route_request(&self, request: &AgentRequest) -> Result<&dyn AgentProvider> {
        // Determine the best agent based on the request
        let agent_name = self.determine_agent_for_request(request);
        
        self.agents
            .get(&agent_name)
            .map(|agent| agent.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_name))
    }
    
    pub fn determine_agent_for_request(&self, request: &AgentRequest) -> String {
        let prompt_lower = request.prompt.to_lowercase();
        let files = &request.files;
        
        // Check for specific keywords and context to route to appropriate agent
        if prompt_lower.contains("explain") || prompt_lower.contains("how does") || prompt_lower.contains("what is") {
            if !files.is_empty() {
                return "code-explainer".to_string();
            }
        }
        
        if prompt_lower.contains("review") || prompt_lower.contains("check") || prompt_lower.contains("improve") {
            return "code-reviewer".to_string();
        }
        
        if prompt_lower.contains("test") || prompt_lower.contains("unit test") || prompt_lower.contains("integration test") {
            return "test-generator".to_string();
        }
        
        if prompt_lower.contains("commit") || prompt_lower.contains("git") || prompt_lower.contains("merge") {
            return "git-assistant".to_string();
        }
        
        if prompt_lower.contains("ui") || prompt_lower.contains("design") || prompt_lower.contains("component") {
            return "ui-designer".to_string();
        }
        
        if prompt_lower.contains("architecture") || prompt_lower.contains("system") || prompt_lower.contains("scalability") {
            return "system-architect".to_string();
        }
        
        if prompt_lower.contains("document") || prompt_lower.contains("readme") || prompt_lower.contains("api doc") {
            return "doc-writer".to_string();
        }
        
        if prompt_lower.contains("debug") || prompt_lower.contains("error") || prompt_lower.contains("bug") {
            return "debugger".to_string();
        }
        
        // Check file types for additional context
        if !files.is_empty() {
            let has_test_files = files.iter().any(|f| f.contains("test") || f.contains("spec"));
            if has_test_files {
                return "test-generator".to_string();
            }
            
            let has_ui_files = files.iter().any(|f| 
                f.ends_with(".tsx") || f.ends_with(".jsx") || f.ends_with(".vue") || f.ends_with(".svelte")
            );
            if has_ui_files {
                return "ui-designer".to_string();
            }
        }
        
        // Default to code explainer for file-based queries, otherwise general
        if !files.is_empty() {
            "code-explainer".to_string()
        } else {
            self.default_agent.clone()
        }
    }
    
    pub fn list_available_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}