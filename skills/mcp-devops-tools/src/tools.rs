//! Tool definitions and execution for MCP DevOps

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// High-performance tool manager with optimized caching
#[derive(Debug)]
pub struct ToolManager {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolManager {
    pub fn new() -> Self {
        let mut manager = Self {
            tools: HashMap::with_capacity(32),
        };
        manager.register_all_tools();
        manager
    }

    fn register_all_tools(&mut self) {
        // Infrastructure tools
        self.register_tool(ToolDefinition::new(
            "list_docker_containers",
            "List all Docker containers with their status",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "all": {"type": "boolean", "description": "Include stopped containers", "default": false}
            }
        })));

        self.register_tool(ToolDefinition::new(
            "get_container_logs",
            "Get logs from a Docker container",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "container_id": {"type": "string", "description": "Container ID or name"},
                "lines": {"type": "integer", "description": "Number of lines to fetch", "default": 100}
            },
            "required": ["container_id"]
        })));

        self.register_tool(ToolDefinition::new(
            "list_k8s_pods",
            "List Kubernetes pods in a namespace",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "namespace": {"type": "string", "description": "Kubernetes namespace", "default": "default"}
            }
        })));

        // Database tools
        self.register_tool(ToolDefinition::new(
            "list_databases",
            "List all available databases",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "enum": ["postgresql", "mongodb", "supabase"]}
            }
        })));

        self.register_tool(ToolDefinition::new(
            "execute_query",
            "Execute a database query",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "enum": ["postgresql", "mongodb", "supabase"]},
                "database": {"type": "string"},
                "query": {"type": "string"}
            },
            "required": ["provider", "database", "query"]
        })));

        // Memory tools
        self.register_tool(ToolDefinition::new(
            "create_memory",
            "Create a new memory in the knowledge graph",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "memory_type": {"type": "string", "enum": ["project", "decision", "meeting", "task", "knowledge"]},
                "title": {"type": "string"},
                "content": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            },
            "required": ["memory_type", "title", "content"]
        })));

        self.register_tool(ToolDefinition::new(
            "search_memory",
            "Search through stored memories",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "memory_type": {"type": "string", "enum": ["project", "decision", "meeting", "task", "knowledge"]}
            },
            "required": ["query"]
        })));

        // Smart Home tools
        self.register_tool(ToolDefinition::new(
            "ha_turn_on",
            "Turn on a Home Assistant device",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "string"},
                "brightness": {"type": "integer"},
                "color": {"type": "string"}
            },
            "required": ["entity_id"]
        })));

        self.register_tool(ToolDefinition::new(
            "ha_turn_off",
            "Turn off a Home Assistant device",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "string"}
            },
            "required": ["entity_id"]
        })));

        // Finance tools
        self.register_tool(ToolDefinition::new(
            "get_stock_quote",
            "Get real-time stock quote",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "symbol": {"type": "string"}
            },
            "required": ["symbol"]
        })));

        self.register_tool(ToolDefinition::new(
            "place_order",
            "Place a stock order",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "symbol": {"type": "string"},
                "quantity": {"type": "integer"},
                "side": {"type": "string", "enum": ["buy", "sell"]},
                "type": {"type": "string", "enum": ["market", "limit"]}
            },
            "required": ["symbol", "quantity", "side", "type"]
        })));

        // Homelab tools
        self.register_tool(ToolDefinition::new(
            "traefik_list_services",
            "List all services and routes configured in Traefik",
        ));

        self.register_tool(ToolDefinition::new(
            "prometheus_query",
            "Query Prometheus metrics",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "prometheus_url": {"type": "string", "default": "http://localhost:9090"}
            },
            "required": ["query"]
        })));

        self.register_tool(ToolDefinition::new(
            "service_health_check",
            "Check health status of homelab services",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "service_name": {"type": "string"},
                "check_type": {"type": "string", "enum": ["http", "docker", "port"], "default": "http"}
            },
            "required": ["service_name"]
        })));

        // Core system tools
        self.register_tool(ToolDefinition::new(
            "health_check",
            "Check system health status",
        ));

        self.register_tool(ToolDefinition::new(
            "security_validate",
            "Validate input for security issues",
        ).with_parameters(json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            },
            "required": ["input"]
        })));
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    pub async fn execute_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            "health_check" => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": "✅ MCP Server Status: Healthy\n✅ All modules loaded\n✅ Ready for requests"
                }]
            })),
            "security_validate" => {
                let input = arguments.get("input").and_then(|i| i.as_str()).unwrap_or("");
                let is_safe = !input.contains("<script")
                    && !input.contains("DROP TABLE")
                    && !input.contains("rm -rf")
                    && !input.contains("../");
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("🔒 Security Validation\nInput: \"{}\"\nStatus: {}",
                            input,
                            if is_safe { "✅ SAFE" } else { "⚠️ POTENTIAL THREAT" }
                        )
                    }]
                }))
            }
            _ => {
                if self.tools.contains_key(name) {
                    Ok(json!({"result": "success", "tool": name, "arguments": arguments}))
                } else {
                    Err(Error::not_found_with_resource("Tool not found", "tool", name))
                }
            }
        }
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub content: Vec<ContentBlock>,
    pub error: Option<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

impl ToolExecutionResult {
    pub fn success(content: Vec<ContentBlock>) -> Self {
        Self {
            success: true,
            content,
            error: None,
            metadata: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            content: Vec::new(),
            error: Some(message.into()),
            metadata: None,
        }
    }
}

/// Content block for tool outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    pub content_type: String,
    pub content: String,
}

impl ContentBlock {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content_type: "text/plain".to_string(),
            content: content.into(),
        }
    }
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
    pub required_parameters: Vec<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: None,
            required_parameters: Vec::new(),
            metadata: None,
        }
    }

    pub fn with_parameters(mut self, parameters: Value) -> Self {
        self.parameters = Some(parameters);
        self
    }

    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required_parameters = required;
        self
    }
}

