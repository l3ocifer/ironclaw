//! LLM tools for the learnings system.
//!
//! Provides tools for creating, searching, and promoting learnings
//! (evidence-backed rules derived from agent experience).

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;

use crate::context::JobContext;
use crate::tools::tool::{Tool, ToolError, ToolOutput};
use crate::workspace::learnings::{LearningRepository, LearningScope};

// ---------------------------------------------------------------------------
// learning_create
// ---------------------------------------------------------------------------

/// Create or reinforce a learning (auto-deduplicates by rule text).
pub struct LearningCreateTool {
    repo: Arc<LearningRepository>,
}

impl LearningCreateTool {
    pub fn new(repo: Arc<LearningRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl Tool for LearningCreateTool {
    fn name(&self) -> &str {
        "learning_create"
    }

    fn description(&self) -> &str {
        "Record a new learning (actionable rule derived from experience). \
         Auto-deduplicates: if the same rule already exists, increments its \
         observation count and confidence instead of creating a duplicate."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["rule"],
            "properties": {
                "rule": {
                    "type": "string",
                    "description": "One imperative sentence describing the rule (e.g. 'Always run clippy before committing')"
                },
                "scope": {
                    "type": "string",
                    "enum": ["global", "repo", "tool"],
                    "description": "Where this learning applies (default: global)"
                },
                "scope_context": {
                    "type": "string",
                    "description": "Additional context when scope is repo (path) or tool (name)"
                },
                "confidence": {
                    "type": "number",
                    "description": "Confidence score 0.0-1.0 (default: 0.5)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Tags for categorization"
                },
                "evidence_kind": {
                    "type": "string",
                    "description": "Type of evidence: session_file, commit, event_id, conversation"
                },
                "evidence_ref": {
                    "type": "string",
                    "description": "Reference to source evidence (path, SHA, event ID)"
                },
                "evidence_context": {
                    "type": "string",
                    "description": "Human-readable snippet from the evidence"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        let rule = params
            .get("rule")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| ToolError::InvalidParameters("'rule' is required".into()))?
            .trim();

        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .map(LearningScope::parse)
            .unwrap_or(LearningScope::Global);

        let scope_context = params.get("scope_context").and_then(|v| v.as_str());
        let confidence = params
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let tags: Vec<String> = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let evidence_kind = params.get("evidence_kind").and_then(|v| v.as_str());
        let evidence_ref = params.get("evidence_ref").and_then(|v| v.as_str());
        let evidence_context = params.get("evidence_context").and_then(|v| v.as_str());

        // agent_id is derived from user_id context; in multi-agent setup this
        // comes from the AGENT_ID env var resolved at startup.
        let user_id = &ctx.user_id;
        let agent_id = "default";

        match self
            .repo
            .create_or_merge(
                user_id, agent_id, rule, &scope, scope_context,
                confidence, &tags, evidence_kind, evidence_ref, evidence_context,
            )
            .await
        {
            Ok(learning) => {
                let msg = if learning.observation_count > 1 {
                    format!(
                        "Reinforced existing learning (seen {}x, confidence={:.0}%)\n\nID: {}\nRule: {}\nStatus: {}",
                        learning.observation_count, learning.confidence * 100.0,
                        learning.id, learning.rule, learning.status.as_str(),
                    )
                } else {
                    format!(
                        "Created new candidate learning\n\nID: {}\nRule: {}\nScope: {}",
                        learning.id, learning.rule, learning.scope.as_str(),
                    )
                };
                Ok(ToolOutput::text(msg, start.elapsed()))
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!("Failed to create learning: {}", e))),
        }
    }
}

// ---------------------------------------------------------------------------
// learning_search
// ---------------------------------------------------------------------------

/// Search learnings by text query.
pub struct LearningSearchTool {
    repo: Arc<LearningRepository>,
}

impl LearningSearchTool {
    pub fn new(repo: Arc<LearningRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl Tool for LearningSearchTool {
    fn name(&self) -> &str {
        "learning_search"
    }

    fn description(&self) -> &str {
        "Search agent learnings (actionable rules derived from experience). \
         Returns matching learnings with confidence scores and observation counts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search text to match against learning rules"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 10)"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| ToolError::InvalidParameters("'query' is required".into()))?
            .trim();

        let limit = params
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10)
            .min(50);

        let user_id = &ctx.user_id;
        let agent_id = "default";

        match self.repo.search(user_id, agent_id, query, limit).await {
            Ok(learnings) if learnings.is_empty() => {
                Ok(ToolOutput::text("No learnings found matching that query.", start.elapsed()))
            }
            Ok(learnings) => {
                let mut out = format!("Found {} learning(s):\n\n", learnings.len());
                for l in &learnings {
                    out.push_str(&format!(
                        "- [{}] (conf={:.0}%, seen={}x, {}) {}\n",
                        l.id, l.confidence * 100.0, l.observation_count,
                        l.status.as_str(), l.rule,
                    ));
                }
                Ok(ToolOutput::text(out, start.elapsed()))
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!("Search failed: {}", e))),
        }
    }
}

// ---------------------------------------------------------------------------
// learning_promote
// ---------------------------------------------------------------------------

/// Promote a candidate learning to active, or deprecate a learning.
pub struct LearningPromoteTool {
    repo: Arc<LearningRepository>,
}

impl LearningPromoteTool {
    pub fn new(repo: Arc<LearningRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl Tool for LearningPromoteTool {
    fn name(&self) -> &str {
        "learning_promote"
    }

    fn description(&self) -> &str {
        "Promote a candidate learning to active status, or deprecate a learning. \
         Active learnings are included in the system prompt for future sessions."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["id", "action"],
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Learning UUID to update"
                },
                "action": {
                    "type": "string",
                    "enum": ["promote", "deprecate"],
                    "description": "Action to perform"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        let id = params
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .ok_or_else(|| ToolError::InvalidParameters("'id' must be a valid UUID".into()))?;

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("'action' is required".into()))?;

        let result = match action {
            "promote" => self.repo.promote(id).await,
            "deprecate" => self.repo.deprecate(id).await,
            _ => return Err(ToolError::InvalidParameters("action must be 'promote' or 'deprecate'".into())),
        };

        match result {
            Ok(learning) => Ok(ToolOutput::text(
                format!("Learning {} -> {}\n\nRule: {}", learning.id, learning.status.as_str(), learning.rule),
                start.elapsed(),
            )),
            Err(e) => Err(ToolError::ExecutionFailed(format!("Failed to update learning: {}", e))),
        }
    }
}
