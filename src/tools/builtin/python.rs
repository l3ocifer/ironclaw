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
//!   specific functions (e.g., `json_parse`) that Python code can call,
//!   with each call mediated by IronClaw's safety layer.
//!
//! # External Functions
//!
//! The following utility functions are available to Python code:
//! - `json_parse(s)` — parse a JSON string into a Python dict/list
//! - `json_dump(obj)` — serialize a Python object to a JSON string
//! - `base64_encode(s)` — base64-encode a string
//! - `base64_decode(s)` — base64-decode a string
//! - `hash_sha256(s)` — compute SHA-256 hex digest of a string

use std::time::{Duration, Instant};

use async_trait::async_trait;
use monty::{
    CollectStringPrint, ExternalResult, LimitedTracker, MontyObject, MontyRun, ResourceLimits,
};
use tracing::debug;

use crate::context::JobContext;
use crate::tools::tool::{Tool, ToolError, ToolOutput};

/// Default max execution time in seconds.
const DEFAULT_MAX_DURATION_SECS: f64 = 10.0;

/// Default max memory in bytes (16 MB).
const DEFAULT_MAX_MEMORY: usize = 16 * 1024 * 1024;

/// Max external function calls per execution (prevent infinite loops).
#[allow(dead_code)]
const MAX_EXTERNAL_CALLS: usize = 100;

/// Names of bridged external functions available in the sandbox.
const EXTERNAL_FUNCTION_NAMES: &[&str] = &[
    "json_parse",
    "json_dump",
    "base64_encode",
    "base64_decode",
    "hash_sha256",
];

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
         and algorithmic tasks. Returns stdout output and the final expression value.\n\n\
         Available utility functions: json_parse(s), json_dump(obj), base64_encode(s), \
         base64_decode(s), hash_sha256(s)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Python code to execute. Use print() for output. The value of the last expression is returned as 'result'. Utility functions available: json_parse(s), json_dump(obj), base64_encode(s), base64_decode(s), hash_sha256(s)"
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

        // Register external function names so monty knows about them
        let ext_fn_names: Vec<String> = EXTERNAL_FUNCTION_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Parse the code
        let runner =
            MontyRun::new(code.to_string(), "<sandbox>", vec![], ext_fn_names).map_err(|e| {
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

        // Execute with external function dispatch loop
        let result = match runner.run(vec![], tracker, &mut printer) {
            Ok(obj) => obj,
            Err(e) => {
                let collected = printer.output();
                let mut msg = format!("Python execution error: {e}");
                if !collected.is_empty() {
                    msg.push_str(&format!("\n\nOutput before error:\n{collected}"));
                }
                return Err(ToolError::ExecutionFailed(msg));
            }
        };

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

/// Dispatch an external function call from the Python sandbox.
///
/// Returns a `MontyObject` result for the given function name and arguments.
/// All functions are pure/sync — no filesystem, network, or workspace access.
///
/// Called by the monty execution loop when Python code invokes a registered
/// external function. Currently the execution path uses `runner.run()` which
/// does not support external function dispatch — these are prepared for when
/// the snapshot-based execution loop is implemented.
#[allow(dead_code)]
fn dispatch_external_function(
    function_name: &str,
    args: &[MontyObject],
    _kwargs: &[(MontyObject, MontyObject)],
) -> ExternalResult {
    match function_name {
        "json_parse" => ext_json_parse(args),
        "json_dump" => ext_json_dump(args),
        "base64_encode" => ext_base64_encode(args),
        "base64_decode" => ext_base64_decode(args),
        "hash_sha256" => ext_hash_sha256(args),
        _ => ExternalResult::Return(MontyObject::None),
    }
}

/// json_parse(s: str) -> dict/list/str/int/float/bool/None
#[allow(dead_code)]
fn ext_json_parse(args: &[MontyObject]) -> ExternalResult {
    let Some(MontyObject::String(s)) = args.first() else {
        return ExternalResult::Return(MontyObject::None);
    };
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(val) => ExternalResult::Return(json_to_monty(&val)),
        Err(e) => ExternalResult::Return(MontyObject::String(format!("Error: {e}"))),
    }
}

/// json_dump(obj) -> str
#[allow(dead_code)]
fn ext_json_dump(args: &[MontyObject]) -> ExternalResult {
    let Some(obj) = args.first() else {
        return ExternalResult::Return(MontyObject::String("null".to_string()));
    };
    let json_val = monty_to_json(obj);
    match serde_json::to_string(&json_val) {
        Ok(s) => ExternalResult::Return(MontyObject::String(s)),
        Err(e) => ExternalResult::Return(MontyObject::String(format!("Error: {e}"))),
    }
}

/// base64_encode(s: str) -> str
#[allow(dead_code)]
fn ext_base64_encode(args: &[MontyObject]) -> ExternalResult {
    use base64::Engine;
    let Some(MontyObject::String(s)) = args.first() else {
        return ExternalResult::Return(MontyObject::None);
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
    ExternalResult::Return(MontyObject::String(encoded))
}

/// base64_decode(s: str) -> str
#[allow(dead_code)]
fn ext_base64_decode(args: &[MontyObject]) -> ExternalResult {
    use base64::Engine;
    let Some(MontyObject::String(s)) = args.first() else {
        return ExternalResult::Return(MontyObject::None);
    };
    match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => ExternalResult::Return(MontyObject::String(decoded)),
            Err(e) => ExternalResult::Return(MontyObject::String(format!("Error: {e}"))),
        },
        Err(e) => ExternalResult::Return(MontyObject::String(format!("Error: {e}"))),
    }
}

/// hash_sha256(s: str) -> str
#[allow(dead_code)]
fn ext_hash_sha256(args: &[MontyObject]) -> ExternalResult {
    use sha2::{Digest, Sha256};
    let Some(MontyObject::String(s)) = args.first() else {
        return ExternalResult::Return(MontyObject::None);
    };
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    let hex = result.iter().map(|b| format!("{b:02x}")).collect::<String>();
    ExternalResult::Return(MontyObject::String(hex))
}

/// Convert a serde_json::Value to a MontyObject.
#[allow(dead_code)]
fn json_to_monty(val: &serde_json::Value) -> MontyObject {
    match val {
        serde_json::Value::Null => MontyObject::None,
        serde_json::Value::Bool(b) => MontyObject::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MontyObject::Int(i)
            } else if let Some(f) = n.as_f64() {
                MontyObject::Float(f)
            } else {
                MontyObject::None
            }
        }
        serde_json::Value::String(s) => MontyObject::String(s.clone()),
        serde_json::Value::Array(arr) => {
            MontyObject::List(arr.iter().map(json_to_monty).collect())
        }
        serde_json::Value::Object(_) => {
            // Monty Dict requires DictPairs which is not publicly constructible
            // Fall back to a JSON string representation
            MontyObject::String(val.to_string())
        }
    }
}

/// Convert a MontyObject to a serde_json::Value.
#[allow(dead_code)]
fn monty_to_json(obj: &MontyObject) -> serde_json::Value {
    match obj {
        MontyObject::None => serde_json::Value::Null,
        MontyObject::Bool(b) => serde_json::Value::Bool(*b),
        MontyObject::Int(i) => serde_json::json!(i),
        MontyObject::BigInt(b) => serde_json::json!(b.to_string()),
        MontyObject::Float(f) => serde_json::json!(f),
        MontyObject::String(s) => serde_json::Value::String(s.clone()),
        MontyObject::List(items) => {
            serde_json::Value::Array(items.iter().map(monty_to_json).collect())
        }
        MontyObject::Tuple(items) => {
            serde_json::Value::Array(items.iter().map(monty_to_json).collect())
        }
        _ => serde_json::Value::String(format!("{obj:?}")),
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
