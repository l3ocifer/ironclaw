//! AST-aware code intelligence tools powered by tilth (subprocess).
//!
//! Provides three tools for LLM agents:
//! - `code_read`  — smart file reading (outline large files, full content for small)
//! - `code_search` — AST-aware symbol/content/caller search via tree-sitter
//! - `code_files`  — glob file finding with token estimates
//!
//! Invokes the `tilth` binary as a subprocess. tilth uses tree-sitter for AST
//! parsing and ripgrep internals for search, achieving ~18ms per call.
//! The binary must be in PATH or at `~/.cargo/bin/tilth`.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::context::JobContext;
use crate::tools::tool::{Tool, ToolDomain, ToolError, ToolOutput, require_str};

/// Shared config for tilth tools.
#[derive(Clone)]
pub struct TilthState {
    /// Path to the tilth binary.
    binary: PathBuf,
    /// Default scope directory for searches.
    default_scope: PathBuf,
}

impl TilthState {
    pub fn new(default_scope: PathBuf) -> Self {
        let binary = which_tilth().unwrap_or_else(|| PathBuf::from("tilth"));
        Self {
            binary,
            default_scope,
        }
    }

    /// Check if tilth binary is available.
    pub fn is_available(&self) -> bool {
        self.binary.exists() || which_tilth().is_some()
    }
}

/// Find the tilth binary in common locations.
fn which_tilth() -> Option<PathBuf> {
    // Check PATH first
    if let Ok(output) = std::process::Command::new("which")
        .arg("tilth")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    // Check ~/.cargo/bin/tilth
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo").join("bin").join("tilth");
        if cargo_bin.exists() {
            return Some(cargo_bin);
        }
    }
    None
}

/// Run tilth with the given arguments and return stdout.
async fn run_tilth(
    binary: &PathBuf,
    args: &[&str],
) -> Result<String, ToolError> {
    let output = tokio::process::Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ToolError::ExecutionFailed(
                    "tilth binary not found. Install with: cargo install tilth".to_string(),
                )
            } else {
                ToolError::ExecutionFailed(format!("Failed to run tilth: {e}"))
            }
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // tilth prints user-facing errors to stdout (e.g. "not found" suggestions)
        if !stdout.is_empty() {
            Ok(stdout.to_string())
        } else {
            Err(ToolError::ExecutionFailed(stderr.to_string()))
        }
    }
}

// ---------------------------------------------------------------------------
// code_read — smart file reading
// ---------------------------------------------------------------------------

pub struct CodeReadTool {
    state: TilthState,
}

impl CodeReadTool {
    pub fn new(state: TilthState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for CodeReadTool {
    fn name(&self) -> &str {
        "code_read"
    }

    fn description(&self) -> &str {
        "Read a file with AST-aware smart viewing. Small files return full content; \
         large files return a structural outline (signatures, classes, imports with line ranges). \
         Use `section` to drill into specific line ranges (e.g. \"45-89\") or markdown headings \
         (e.g. \"## Architecture\"). Use `full: true` to force full content on large files."
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to read"
                },
                "section": {
                    "type": "string",
                    "description": "Line range (e.g. \"45-89\") or markdown heading (e.g. \"## Architecture\")"
                },
                "full": {
                    "type": "boolean",
                    "description": "Force full content even for large files",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let path = require_str(&params, "path")?;
        let section = params.get("section").and_then(|v| v.as_str());
        let full = params.get("full").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut args: Vec<&str> = vec![path];
        if let Some(sec) = section {
            args.push("--section");
            args.push(sec);
        }
        if full {
            args.push("--full");
        }

        let output = run_tilth(&self.state.binary, &args).await?;
        Ok(ToolOutput::text(output, start.elapsed()))
    }
}

// ---------------------------------------------------------------------------
// code_search — AST-aware symbol/content/caller search
// ---------------------------------------------------------------------------

pub struct CodeSearchTool {
    state: TilthState,
}

impl CodeSearchTool {
    pub fn new(state: TilthState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "code_search"
    }

    fn description(&self) -> &str {
        "Search code with AST awareness via tree-sitter. Finds symbol definitions first, \
         then usages. Supports comma-separated multi-symbol lookup (max 5). \
         Expanded results include full source and a callee footer for navigating call chains. \
         Use kind=\"callers\" to find all call sites. Supports 9 languages: \
         Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, Ruby."
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Symbol name, text, /regex/, or comma-separated symbols (max 5)"
                },
                "scope": {
                    "type": "string",
                    "description": "Directory to search within (defaults to workspace root)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let query = require_str(&params, "query")?;
        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| self.state.default_scope.to_str().unwrap_or("."));

        let args = vec![query, "--scope", scope];
        let output = run_tilth(&self.state.binary, &args).await?;
        Ok(ToolOutput::text(output, start.elapsed()))
    }
}

// ---------------------------------------------------------------------------
// code_files — glob file finding with token estimates
// ---------------------------------------------------------------------------

pub struct CodeFilesTool {
    state: TilthState,
}

impl CodeFilesTool {
    pub fn new(state: TilthState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for CodeFilesTool {
    fn name(&self) -> &str {
        "code_files"
    }

    fn description(&self) -> &str {
        "Find files by glob pattern with token estimates. Respects .gitignore. \
         Returns paths and approximate token counts for each match."
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern, e.g. \"*.test.ts\", \"src/**/*.rs\""
                },
                "scope": {
                    "type": "string",
                    "description": "Directory to search within (defaults to workspace root)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let pattern = require_str(&params, "pattern")?;
        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| self.state.default_scope.to_str().unwrap_or("."));

        let args = vec![pattern, "--scope", scope];
        let output = run_tilth(&self.state.binary, &args).await?;
        Ok(ToolOutput::text(output, start.elapsed()))
    }
}
