//! Task management tools for multi-agent coordination.
//!
//! Provides LLM-facing tools for the agent task graph:
//! - `task_create`: Create a new task with optional dependencies
//! - `task_list`: List tasks with filters
//! - `task_update`: Update task status
//! - `task_ready`: List tasks ready to work on
//! - `task_export`: Export tasks as JSONL

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::context::JobContext;
use crate::tools::tool::{Tool, ToolError, ToolOutput};
use crate::workspace::tasks::{
    CreateTaskParams, ListTasksParams, TaskPriority, TaskRepository, TaskStatus,
};

// ---------------------------------------------------------------------------
// task_create
// ---------------------------------------------------------------------------

/// Tool for creating tasks in the multi-agent task graph.
pub struct TaskCreateTool {
    repo: Arc<TaskRepository>,
    agent_id: String,
    user_id: String,
}

impl TaskCreateTool {
    pub fn new(repo: Arc<TaskRepository>, agent_id: String, user_id: String) -> Self {
        Self {
            repo,
            agent_id,
            user_id,
        }
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "task_create"
    }

    fn description(&self) -> &str {
        "Create a new task in the multi-agent task graph. Tasks can have dependencies (blocks), \
         priority levels, labels, and be assigned to specific agents. Tasks with unmet \
         dependencies start as 'pending'; tasks with no dependencies start as 'ready'."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "priority": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "description": "Task priority. Default: medium"
                },
                "labels": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Labels for categorization (e.g., 'security', 'infra', 'feature')"
                },
                "assigned_to": {
                    "type": "string",
                    "description": "Agent ID to assign this task to (e.g., 'frack', 'frick'). Null = unassigned."
                },
                "depends_on": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "UUIDs of tasks that must complete before this one can start"
                },
                "due_at": {
                    "type": "string",
                    "description": "ISO 8601 due date (e.g., '2026-02-15T00:00:00Z')"
                }
            },
            "required": ["title"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'title'".into()))?
            .to_string();

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let priority = params
            .get("priority")
            .and_then(|v| v.as_str())
            .and_then(TaskPriority::from_sql)
            .unwrap_or(TaskPriority::Medium);

        let labels: Vec<String> = params
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let assigned_to = params
            .get("assigned_to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let depends_on: Vec<Uuid> = params
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                    .collect()
            })
            .unwrap_or_default();

        let due_at = params
            .get("due_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let task = self
            .repo
            .create(CreateTaskParams {
                user_id: self.user_id.clone(),
                agent_id: self.agent_id.clone(),
                title: title.clone(),
                description,
                priority,
                labels,
                assigned_to,
                depends_on,
                due_at,
                metadata: serde_json::json!({}),
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create task: {e}")))?;

        let output = format!(
            "Created task: {} (id: {}, status: {}, priority: {})",
            title, task.id, task.status, task.priority
        );
        Ok(ToolOutput::text(&output, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// task_list
// ---------------------------------------------------------------------------

/// Tool for listing tasks with filters.
pub struct TaskListTool {
    repo: Arc<TaskRepository>,
    user_id: String,
}

impl TaskListTool {
    pub fn new(repo: Arc<TaskRepository>, user_id: String) -> Self {
        Self { repo, user_id }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "task_list"
    }

    fn description(&self) -> &str {
        "List tasks from the multi-agent task graph. Filter by status, assignee, priority, \
         or labels. By default shows only active (non-completed) tasks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["pending", "ready", "in_progress", "blocked", "completed", "failed", "cancelled"],
                    "description": "Filter by status"
                },
                "assigned_to": {
                    "type": "string",
                    "description": "Filter by assigned agent ID"
                },
                "priority": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "description": "Filter by priority"
                },
                "labels": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by labels (ANY match)"
                },
                "include_completed": {
                    "type": "boolean",
                    "description": "Include completed/failed/cancelled tasks. Default: false"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max tasks to return. Default: 50"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let list_params = ListTasksParams {
            user_id: self.user_id.clone(),
            status: params
                .get("status")
                .and_then(|v| v.as_str())
                .and_then(TaskStatus::from_sql),
            assigned_to: params
                .get("assigned_to")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            priority: params
                .get("priority")
                .and_then(|v| v.as_str())
                .and_then(TaskPriority::from_sql),
            labels: params
                .get("labels")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            include_completed: params
                .get("include_completed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            limit: params.get("limit").and_then(|v| v.as_i64()),
        };

        let tasks = self
            .repo
            .list(list_params)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list tasks: {e}")))?;

        if tasks.is_empty() {
            return Ok(ToolOutput::text("No tasks found.", start.elapsed()));
        }

        let mut output = format!("Found {} tasks:\n\n", tasks.len());
        for task in &tasks {
            output.push_str(&format!(
                "- [{}] **{}** (id: {})\n  Status: {} | Priority: {} | Assigned: {}\n",
                task.priority,
                task.title,
                task.id,
                task.status,
                task.priority,
                task.assigned_to.as_deref().unwrap_or("unassigned"),
            ));
            if let Some(desc) = &task.description {
                output.push_str(&format!("  {desc}\n"));
            }
        }

        Ok(ToolOutput::text(&output, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// task_update
// ---------------------------------------------------------------------------

/// Tool for updating task status.
pub struct TaskUpdateTool {
    repo: Arc<TaskRepository>,
    agent_id: String,
    user_id: String,
}

impl TaskUpdateTool {
    pub fn new(repo: Arc<TaskRepository>, agent_id: String, user_id: String) -> Self {
        Self {
            repo,
            agent_id,
            user_id,
        }
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "task_update"
    }

    fn description(&self) -> &str {
        "Update a task's status. When a task is completed, any dependent tasks that are now \
         unblocked will automatically be promoted to 'ready'. Include a result message when \
         completing or failing a task."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "UUID of the task to update"
                },
                "status": {
                    "type": "string",
                    "enum": ["ready", "in_progress", "blocked", "completed", "failed", "cancelled"],
                    "description": "New status"
                },
                "result": {
                    "type": "string",
                    "description": "Completion notes or error message"
                }
            },
            "required": ["task_id", "status"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| ToolError::InvalidParameters("invalid or missing 'task_id'".into()))?;

        let status = params
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(TaskStatus::from_sql)
            .ok_or_else(|| ToolError::InvalidParameters("invalid or missing 'status'".into()))?;

        let result = params.get("result").and_then(|v| v.as_str());

        self.repo
            .update_status(&self.user_id, task_id, &self.agent_id, status, result)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update task: {e}")))?;

        let output = format!("Updated task {task_id} → {status}");
        Ok(ToolOutput::text(&output, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// task_ready
// ---------------------------------------------------------------------------

/// Tool for listing tasks ready to work on.
pub struct TaskReadyTool {
    repo: Arc<TaskRepository>,
    agent_id: String,
    user_id: String,
}

impl TaskReadyTool {
    pub fn new(repo: Arc<TaskRepository>, agent_id: String, user_id: String) -> Self {
        Self {
            repo,
            agent_id,
            user_id,
        }
    }
}

#[async_trait]
impl Tool for TaskReadyTool {
    fn name(&self) -> &str {
        "task_ready"
    }

    fn description(&self) -> &str {
        "List tasks that are ready to work on — all dependencies met, not yet started. \
         By default shows tasks assigned to this agent or unassigned."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "all_agents": {
                    "type": "boolean",
                    "description": "If true, show ready tasks for all agents. Default: false (show only mine + unassigned)"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let all_agents = params
            .get("all_agents")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let agent_filter = if all_agents {
            None
        } else {
            Some(self.agent_id.as_str())
        };

        let tasks = self
            .repo
            .list_ready(&self.user_id, agent_filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list ready tasks: {e}")))?;

        if tasks.is_empty() {
            return Ok(ToolOutput::text(
                "No tasks ready to work on.",
                start.elapsed(),
            ));
        }

        let mut output = format!("{} tasks ready:\n\n", tasks.len());
        for task in &tasks {
            output.push_str(&format!(
                "- [{}] **{}** (id: {})\n  Assigned: {} | Created by: {}\n",
                task.priority,
                task.title,
                task.id,
                task.assigned_to.as_deref().unwrap_or("unassigned"),
                task.created_by,
            ));
            if let Some(desc) = &task.description {
                output.push_str(&format!("  {desc}\n"));
            }
        }

        Ok(ToolOutput::text(&output, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// task_export
// ---------------------------------------------------------------------------

/// Tool for exporting the task graph as JSONL.
pub struct TaskExportTool {
    repo: Arc<TaskRepository>,
    user_id: String,
}

impl TaskExportTool {
    pub fn new(repo: Arc<TaskRepository>, user_id: String) -> Self {
        Self { repo, user_id }
    }
}

#[async_trait]
impl Tool for TaskExportTool {
    fn name(&self) -> &str {
        "task_export"
    }

    fn description(&self) -> &str {
        "Export all tasks as JSONL (one JSON object per line). Compatible with beads \
         format for external tooling and backup."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let jsonl = self
            .repo
            .export_jsonl(&self.user_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to export tasks: {e}")))?;

        if jsonl.is_empty() {
            return Ok(ToolOutput::text("No tasks to export.", start.elapsed()));
        }

        Ok(ToolOutput::text(&jsonl, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// task_archive
// ---------------------------------------------------------------------------

/// Tool for archiving old completed/failed/cancelled tasks (memory decay).
pub struct TaskArchiveTool {
    repo: Arc<TaskRepository>,
    user_id: String,
}

impl TaskArchiveTool {
    pub fn new(repo: Arc<TaskRepository>, user_id: String) -> Self {
        Self { repo, user_id }
    }
}

#[async_trait]
impl Tool for TaskArchiveTool {
    fn name(&self) -> &str {
        "task_archive"
    }

    fn description(&self) -> &str {
        "Archive completed, failed, and cancelled tasks older than a retention period. \
         Returns a compact summary of archived tasks. This frees context window space \
         by removing old terminal tasks from the active task graph."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "retention_days": {
                    "type": "integer",
                    "description": "Archive tasks older than this many days (default: 7)",
                    "default": 7
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let retention_days = params
            .get("retention_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(7) as i32;

        let (summary, count) = self
            .repo
            .archive_completed_tasks(&self.user_id, retention_days)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to archive tasks: {e}")))?;

        if count == 0 {
            return Ok(ToolOutput::text(
                &format!("No tasks older than {retention_days} days to archive."),
                start.elapsed(),
            ));
        }

        Ok(ToolOutput::text(
            &format!("Archived {count} tasks.\n\n{summary}"),
            start.elapsed(),
        ))
    }

    fn requires_sanitization(&self) -> bool {
        false
    }
}
