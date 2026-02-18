//! MCP DevOps Tools Server
//!
//! High-performance MCP server for DevOps operations

use axum::{Router, routing::{get, post}, extract::Json, response::Json as ResponseJson};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::env;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod tools;
mod transport;
mod homelab;
mod memory;
mod smart_home;
mod finance;
mod research;
mod office;
mod infrastructure;
mod think;

use transport::{JsonRpcRequest, JsonRpcResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("mcp_devops_tools=info"))
        )
        .init();

    tracing::info!("Starting MCP DevOps Tools server...");

    // Get configuration
    let host = env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("MCP_HTTP_PORT")
        .unwrap_or_else(|_| "8890".to_string())
        .parse()
        .unwrap_or(8890);

    // Create router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", post(mcp_handler))
        .route("/", get(root_handler));

    // Bind and serve
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("MCP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn root_handler() -> &'static str {
    "MCP DevOps Tools Server - Use POST for JSON-RPC requests"
}

async fn mcp_handler(Json(request): Json<JsonRpcRequest>) -> ResponseJson<JsonRpcResponse> {
    tracing::info!("Received MCP request: method={}", request.method);

    let response = match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(request.id, request.params).await,
        _ => JsonRpcResponse::error(request.id, -32601, format!("Method not found: {}", request.method)),
    };

    ResponseJson(response)
}

fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {"tools": {}},
        "serverInfo": {
            "name": "mcp-devops-tools",
            "version": "1.0.0"
        }
    }))
}

fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tool_manager = tools::ToolManager::new();
    let tools: Vec<Value> = tool_manager.list_tools()
        .iter()
        .map(|t| json!({
            "name": t.name,
            "description": t.description,
            "inputSchema": t.parameters.clone().unwrap_or(json!({"type": "object", "properties": {}}))
        }))
        .collect();

    JsonRpcResponse::success(id, json!({"tools": tools}))
}

async fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    if let Some(params) = params {
        if let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) {
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let tool_manager = tools::ToolManager::new();

            match tool_manager.execute_tool(tool_name, arguments).await {
                Ok(result) => JsonRpcResponse::success(id, result),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
            }
        } else {
            JsonRpcResponse::error(id, -32602, "Missing tool name".to_string())
        }
    } else {
        JsonRpcResponse::error(id, -32602, "Missing params".to_string())
    }
}

