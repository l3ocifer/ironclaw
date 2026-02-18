//! MCP DevOps Tools - High-Performance Rust Implementation
//!
//! Comprehensive Model Context Protocol (MCP) implementation with extensive
//! performance optimizations including zero-copy operations, efficient memory
//! management, and async-first design patterns.

// Core modules
pub mod config;
pub mod error;
pub mod tools;
pub mod transport;

// Infrastructure modules
pub mod homelab;
pub mod infrastructure;

// Specialized modules
pub mod memory;
pub mod office;
pub mod smart_home;
pub mod finance;
pub mod research;

// Extended thinking for complex problems
pub mod think;

// Re-export core types
pub use config::Config;
pub use error::{Error, Result};
pub use tools::{ToolDefinition, ToolExecutionResult};

/// High-performance MCP client creation with optimized defaults
pub fn new(config: Config) -> Result<McpClient> {
    McpClient::new(config)
}

/// MCP Client for tool execution
pub struct McpClient {
    config: Config,
    tools: tools::ToolManager,
}

impl McpClient {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            tools: tools::ToolManager::new(),
        })
    }

    pub async fn execute_tool(&self, name: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        self.tools.execute_tool(name, params).await
    }

    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.list_tools()
    }
}

