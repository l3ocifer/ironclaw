//! Sandboxed Python execution tool.
//!
//! Uses [`monty`] — a secure Python interpreter written in Rust — to execute
//! Python code in a fully sandboxed environment with configurable resource
//! limits (CPU time, memory). No filesystem access, no network, no subprocess
//! spawning. Safe for untrusted or LLM-generated code.
//!
//! # Security Model
//!
//! - **No I/O**: No filesystem, network, or subprocess access.
//! - **Resource limits**: Configurable max execution time and memory.
//! - **Deterministic**: Same inputs → same outputs (no random, no time).
//! - **External functions**: Bridged explicitly — the agent can register
//!   specific functions (e.g., `memory_read`) that Python code can call,
//!   with each call mediated by IronClaw's safety layer.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use monty::{CollectStringPrint, LimitedTracker, MontyObject, MontyRun, ResourceLimits};
use tracing::debug;

use crate::context::JobContext;
use crate::tools::tool::{Tool, ToolError, ToolOutput};

/// Default max execution time in seconds.
const DEFAULT_MAX_DURATION_SECS: f64 = 10.0;

/// Default max memory in bytes (16 MB).
const DEFAULT_MAX_MEMORY: usize = 16 * 1024 * 1024;

/// Tool for executing sandboxed Python code.
///
/// Useful for data transformation, calculations, text processing, and
/// other tasks where the LLM would benefit from running actual code
/// rather than reasoning through the answer.
pub struct PythonTool {
    max_duration_secs: f64,
    max_memory: usize,
}

impl PythonTool {
    /// Create a new Python tool with default resource limits.
    pub fn new() -> Self {
        Self {
            max_duration_secs: DEFAULT_MAX_DURATION_SECS,
            max_memory: DEFAULT_MAX_MEMORY,
        }
    }

    /// Create with custom resource limits.
    pub fn with_limits(max_duration_secs: f64, max_memory: usize) -> Self {
        Self {
            max_duration_secs,
            max_memory,
        }
    }
}

impl Default for PythonTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn name(&self) -> &str {
        "python"
    }

    fn description(&self) -> &str {
        "Execute Python code in a secure sandbox. No filesystem, network, or subprocess access. \
         Useful for calculations, data transformation, text processing, JSON manipulation, \
         and algorithmic tasks. Returns stdout output and the final expression value."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Python code to execute. Use print() for output. The value of the last expression is returned as 'result'."
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'code' parameter".to_string()))?;

        debug!(code_len = code.len(), "Executing sandboxed Python");

        // Parse the code
        let runner = MontyRun::new(code.to_string(), "<sandbox>", vec![], vec![]).map_err(|e| {
            ToolError::ExecutionFailed(format!("Python parse error: {e}"))
        })?;

        // Configure resource limits
        let limits = ResourceLimits {
            max_duration: Some(Duration::from_secs_f64(self.max_duration_secs)),
            max_memory: Some(self.max_memory),
            ..Default::default()
        };
        let tracker = LimitedTracker::new(limits);
        let mut printer = CollectStringPrint::new();

        // Execute with limits
        let result = runner.run(vec![], tracker, &mut printer).map_err(|e| {
            let collected = printer.output();
            let mut msg = format!("Python execution error: {e}");
            if !collected.is_empty() {
                msg.push_str(&format!("\n\nOutput before error:\n{collected}"));
            }
            ToolError::ExecutionFailed(msg)
        })?;

        let collected = printer.output();

        // Format output
        let result_str = format_monty_object(&result);
        let mut output = String::new();

        if !collected.is_empty() {
            output.push_str(collected);
        }

        // Append result if it's not None (i.e., the code produced a value)
        if result != MontyObject::None {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("→ ");
            output.push_str(&result_str);
        }

        if output.is_empty() {
            output = "(no output)".to_string();
        }

        debug!(
            duration_ms = start.elapsed().as_millis(),
            output_len = output.len(),
            "Python execution complete"
        );

        Ok(ToolOutput::text(&output, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        true // Output could contain anything
    }
}

/// Format a MontyObject for human-readable display.
fn format_monty_object(obj: &MontyObject) -> String {
    match obj {
        MontyObject::None => "None".to_string(),
        MontyObject::Bool(b) => b.to_string(),
        MontyObject::Int(i) => i.to_string(),
        MontyObject::BigInt(b) => b.to_string(),
        MontyObject::Float(f) => format!("{f}"),
        MontyObject::String(s) => format!("{s:?}"),
        MontyObject::Bytes(b) => format!("b{b:?}"),
        MontyObject::List(items) => {
            let inner: Vec<String> = items.iter().map(format_monty_object).collect();
            format!("[{}]", inner.join(", "))
        }
        MontyObject::Tuple(items) => {
            let inner: Vec<String> = items.iter().map(format_monty_object).collect();
            if items.len() == 1 {
                format!("({},)", inner[0])
            } else {
                format!("({})", inner.join(", "))
            }
        }
        MontyObject::Dict(_) => {
            // DictPairs internals are private; use Debug formatting
            format!("{obj:?}")
        }
        MontyObject::Set(items) => {
            let inner: Vec<String> = items.iter().map(format_monty_object).collect();
            format!("{{{}}}", inner.join(", "))
        }
        MontyObject::Ellipsis => "...".to_string(),
        _ => format!("{obj:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expression() {
        let runner = MontyRun::new("2 + 2".to_string(), "<test>", vec![], vec![]).unwrap();
        let result = runner.run_no_limits(vec![]).unwrap();
        assert_eq!(result, MontyObject::Int(4));
    }

    #[test]
    fn test_print_output() {
        let runner =
            MontyRun::new("print('hello')".to_string(), "<test>", vec![], vec![]).unwrap();
        let mut printer = CollectStringPrint::new();
        let tracker = monty::NoLimitTracker;
        let _ = runner.run(vec![], tracker, &mut printer).unwrap();
        assert!(printer.output().contains("hello"));
    }

    #[test]
    fn test_format_monty_objects() {
        assert_eq!(format_monty_object(&MontyObject::Int(42)), "42");
        assert_eq!(format_monty_object(&MontyObject::None), "None");
        assert_eq!(
            format_monty_object(&MontyObject::String("hi".to_string())),
            "\"hi\""
        );
        assert_eq!(
            format_monty_object(&MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::Int(2)
            ])),
            "[1, 2]"
        );
    }
}
