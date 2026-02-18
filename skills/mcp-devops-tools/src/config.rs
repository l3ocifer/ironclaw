//! Configuration for MCP DevOps Tools

use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Service endpoints
    pub services: ServiceEndpoints,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
}

/// Service endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoints {
    pub traefik: String,
    pub prometheus: String,
    pub grafana: String,
    pub coolify: String,
    pub n8n: String,
    pub uptime_kuma: String,
    pub authelia: String,
    pub vector: String,
    pub home_assistant: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8890,
            },
            services: ServiceEndpoints {
                traefik: "http://localhost:8080".to_string(),
                prometheus: "http://localhost:9090".to_string(),
                grafana: "http://localhost:3000".to_string(),
                coolify: "http://localhost:8000".to_string(),
                n8n: "http://localhost:5678".to_string(),
                uptime_kuma: "http://localhost:3001".to_string(),
                authelia: "http://localhost:9091".to_string(),
                vector: "http://localhost:8686".to_string(),
                home_assistant: "http://localhost:8123".to_string(),
            },
        }
    }
}
