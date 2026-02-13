//! Task graph for multi-agent coordination.
//!
//! Inspired by [beads](https://github.com/steveyegge/beads) — provides a
//! DAG-based task management system stored in PostgreSQL. Agents can create
//! tasks with dependencies, assign them to other agents, and track status
//! through a well-defined lifecycle.
//!
//! # Data Model
//!
//! ```text
//! Task {id, title, description, status, priority, assigned_to, labels, ...}
//!   ├── depends_on → [Task, Task, ...]  (DAG edges)
//!   └── events → [TaskEvent, ...]       (audit trail)
//! ```
//!
//! # JSONL Interop
//!
//! Tasks can be exported/imported as JSONL for compatibility with beads
//! tooling and for portable backup/sync.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status lifecycle.
///
/// ```text
/// pending → ready → in_progress → completed
///                  ↘ blocked ↗   ↘ failed
///                                 ↘ cancelled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Created, waiting for dependencies to clear.
    Pending,
    /// All dependencies met, can be picked up.
    Ready,
    /// Currently being worked on.
    InProgress,
    /// Manually blocked (waiting for external input).
    Blocked,
    /// Successfully completed.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled by user or agent.
    Cancelled,
}

impl TaskStatus {
    /// SQL enum string for this status.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from SQL enum string.
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "ready" => Some(Self::Ready),
            "in_progress" => Some(Self::InProgress),
            "blocked" => Some(Self::Blocked),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether this status represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_sql())
    }
}

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    /// P0: drop everything.
    Critical,
    /// P1: do next.
    High,
    /// P2: normal queue.
    Medium,
    /// P3: backlog.
    Low,
}

impl TaskPriority {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "critical" => Some(Self::Critical),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            _ => None,
        }
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_sql())
    }
}

/// A task in the task graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub user_id: String,
    pub created_by: String,
    pub assigned_to: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub labels: Vec<String>,
    pub metadata: serde_json::Value,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
}

/// A dependency edge in the task DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub task_id: Uuid,
    pub depends_on: Uuid,
    pub dep_type: String,
    pub created_at: DateTime<Utc>,
}

/// An event in the task audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_id: String,
    pub event_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating a new task.
#[derive(Debug, Clone)]
pub struct CreateTaskParams {
    pub user_id: String,
    pub agent_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub labels: Vec<String>,
    pub assigned_to: Option<String>,
    pub depends_on: Vec<Uuid>,
    pub due_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

/// Parameters for listing tasks.
#[derive(Debug, Clone, Default)]
pub struct ListTasksParams {
    pub user_id: String,
    pub status: Option<TaskStatus>,
    pub assigned_to: Option<String>,
    pub priority: Option<TaskPriority>,
    pub labels: Vec<String>,
    pub limit: Option<i64>,
    pub include_completed: bool,
}

/// Task repository — PostgreSQL operations for the task graph.
pub struct TaskRepository {
    pool: deadpool_postgres::Pool,
}

impl TaskRepository {
    pub fn new(pool: deadpool_postgres::Pool) -> Self {
        Self { pool }
    }

    /// Create a new task.
    pub async fn create(&self, params: CreateTaskParams) -> Result<Task, TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;

        // Determine initial status: 'ready' if no deps, 'pending' if deps exist
        let initial_status = if params.depends_on.is_empty() {
            TaskStatus::Ready
        } else {
            TaskStatus::Pending
        };

        let row = client
            .query_one(
                "INSERT INTO agent_tasks (user_id, created_by, assigned_to, title, description, \
                 status, priority, labels, metadata, due_at) \
                 VALUES ($1, $2, $3, $4, $5, $6::task_status, $7::task_priority, $8, $9, $10) \
                 RETURNING id, created_at, updated_at",
                &[
                    &params.user_id,
                    &params.agent_id,
                    &params.assigned_to,
                    &params.title,
                    &params.description,
                    &initial_status.as_sql(),
                    &params.priority.as_sql(),
                    &params.labels,
                    &params.metadata,
                    &params.due_at,
                ],
            )
            .await
            .map_err(TaskError::Db)?;

        let task_id: Uuid = row.get(0);
        let created_at: DateTime<Utc> = row.get(1);
        let updated_at: DateTime<Utc> = row.get(2);

        // Add dependency edges
        for dep_id in &params.depends_on {
            client
                .execute(
                    "INSERT INTO agent_task_deps (task_id, depends_on) VALUES ($1, $2) \
                     ON CONFLICT DO NOTHING",
                    &[&task_id, dep_id],
                )
                .await
                .map_err(TaskError::Db)?;
        }

        // Record creation event
        client
            .execute(
                "INSERT INTO agent_task_events (task_id, agent_id, event_type, new_value) \
                 VALUES ($1, $2, 'created', $3)",
                &[&task_id, &params.agent_id, &params.title],
            )
            .await
            .map_err(TaskError::Db)?;

        Ok(Task {
            id: task_id,
            user_id: params.user_id,
            created_by: params.agent_id,
            assigned_to: params.assigned_to,
            title: params.title,
            description: params.description,
            status: initial_status,
            priority: params.priority,
            labels: params.labels,
            metadata: params.metadata,
            result: None,
            created_at,
            updated_at,
            started_at: None,
            completed_at: None,
            due_at: params.due_at,
            content_hash: None,
        })
    }

    /// List tasks with filters.
    pub async fn list(&self, params: ListTasksParams) -> Result<Vec<Task>, TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;

        let mut conditions = vec!["user_id = $1".to_string()];
        let mut bind_idx = 2u32;
        let mut query_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            vec![Box::new(params.user_id.clone())];

        if let Some(status) = &params.status {
            conditions.push(format!("status = ${bind_idx}::task_status"));
            query_params.push(Box::new(status.as_sql().to_string()));
            bind_idx += 1;
        } else if !params.include_completed {
            conditions.push("status NOT IN ('completed', 'failed', 'cancelled')".to_string());
        }

        if let Some(assigned) = &params.assigned_to {
            conditions.push(format!("assigned_to = ${bind_idx}"));
            query_params.push(Box::new(assigned.clone()));
            bind_idx += 1;
        }

        if let Some(priority) = &params.priority {
            conditions.push(format!("priority = ${bind_idx}::task_priority"));
            query_params.push(Box::new(priority.as_sql().to_string()));
            bind_idx += 1;
        }

        if !params.labels.is_empty() {
            conditions.push(format!("labels && ${bind_idx}"));
            query_params.push(Box::new(params.labels.clone()));
            bind_idx += 1;
        }

        let limit = params.limit.unwrap_or(50);
        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT id, user_id, created_by, assigned_to, title, description, \
             status::text, priority::text, labels, metadata, result, \
             created_at, updated_at, started_at, completed_at, due_at, content_hash \
             FROM agent_tasks WHERE {where_clause} \
             ORDER BY \
               CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
               WHEN 'medium' THEN 2 WHEN 'low' THEN 3 END, \
               created_at DESC \
             LIMIT ${bind_idx}"
        );
        query_params.push(Box::new(limit));

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = query_params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client
            .query(&sql, &param_refs)
            .await
            .map_err(TaskError::Db)?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    /// Get a single task by ID.
    pub async fn get(&self, user_id: &str, task_id: Uuid) -> Result<Option<Task>, TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;
        let row = client
            .query_opt(
                "SELECT id, user_id, created_by, assigned_to, title, description, \
                 status::text, priority::text, labels, metadata, result, \
                 created_at, updated_at, started_at, completed_at, due_at, content_hash \
                 FROM agent_tasks WHERE id = $1 AND user_id = $2",
                &[&task_id, &user_id],
            )
            .await
            .map_err(TaskError::Db)?;
        Ok(row.as_ref().map(row_to_task))
    }

    /// Update task status with event logging.
    ///
    /// Requires `user_id` for authorization — only tasks owned by the same user
    /// can be updated.
    pub async fn update_status(
        &self,
        user_id: &str,
        task_id: Uuid,
        agent_id: &str,
        new_status: TaskStatus,
        result: Option<&str>,
    ) -> Result<(), TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;

        // Get current status (scoped to user_id for authorization)
        let row = client
            .query_opt(
                "SELECT status::text FROM agent_tasks WHERE id = $1 AND user_id = $2",
                &[&task_id, &user_id],
            )
            .await
            .map_err(TaskError::Db)?
            .ok_or(TaskError::NotFound(task_id))?;

        let old_status: String = row.get(0);

        // Update status and timestamps
        let (started_at_clause, completed_at_clause) = match new_status {
            TaskStatus::InProgress => ("started_at = COALESCE(started_at, now()),", ""),
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                ("", "completed_at = now(),")
            }
            _ => ("", ""),
        };

        let sql = format!(
            "UPDATE agent_tasks SET status = $1::task_status, {started_at_clause} \
             {completed_at_clause} result = COALESCE($2, result) WHERE id = $3 AND user_id = $4"
        );

        client
            .execute(
                &sql,
                &[&new_status.as_sql(), &result, &task_id, &user_id],
            )
            .await
            .map_err(TaskError::Db)?;

        // Log event
        client
            .execute(
                "INSERT INTO agent_task_events (task_id, agent_id, event_type, old_value, new_value, comment) \
                 VALUES ($1, $2, 'status_change', $3, $4, $5)",
                &[
                    &task_id,
                    &agent_id,
                    &old_status,
                    &new_status.as_sql(),
                    &result,
                ],
            )
            .await
            .map_err(TaskError::Db)?;

        // Auto-promote dependents if this task just completed
        if new_status == TaskStatus::Completed {
            self.promote_dependents(&client, task_id).await?;
        }

        Ok(())
    }

    /// List tasks that are ready to be worked on (all hard deps completed).
    pub async fn list_ready(&self, user_id: &str, agent_id: Option<&str>) -> Result<Vec<Task>, TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;

        let sql = if agent_id.is_some() {
            "SELECT id, user_id, created_by, assigned_to, title, description, \
             status::text, priority::text, labels, metadata, result, \
             created_at, updated_at, started_at, completed_at, due_at, content_hash \
             FROM agent_tasks_ready WHERE user_id = $1 AND (assigned_to IS NULL OR assigned_to = $2) \
             ORDER BY \
               CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
               WHEN 'medium' THEN 2 WHEN 'low' THEN 3 END, \
               created_at \
             LIMIT 50"
        } else {
            "SELECT id, user_id, created_by, assigned_to, title, description, \
             status::text, priority::text, labels, metadata, result, \
             created_at, updated_at, started_at, completed_at, due_at, content_hash \
             FROM agent_tasks_ready WHERE user_id = $1 \
             ORDER BY \
               CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
               WHEN 'medium' THEN 2 WHEN 'low' THEN 3 END, \
               created_at \
             LIMIT 50"
        };

        let rows = if let Some(agent) = agent_id {
            client.query(sql, &[&user_id, &agent]).await
        } else {
            client.query(sql, &[&user_id]).await
        }
        .map_err(TaskError::Db)?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    /// Add a dependency between tasks.
    pub async fn add_dependency(
        &self,
        task_id: Uuid,
        depends_on: Uuid,
        agent_id: &str,
    ) -> Result<(), TaskError> {
        if task_id == depends_on {
            return Err(TaskError::InvalidDependency(
                "Task cannot depend on itself".to_string(),
            ));
        }

        let client = self.pool.get().await.map_err(TaskError::Pool)?;

        // Check for cycles (simple: does depends_on transitively depend on task_id?)
        if self
            .has_path(&client, depends_on, task_id)
            .await?
        {
            return Err(TaskError::InvalidDependency(
                "Adding this dependency would create a cycle".to_string(),
            ));
        }

        client
            .execute(
                "INSERT INTO agent_task_deps (task_id, depends_on) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
                &[&task_id, &depends_on],
            )
            .await
            .map_err(TaskError::Db)?;

        // Log event
        client
            .execute(
                "INSERT INTO agent_task_events (task_id, agent_id, event_type, new_value) \
                 VALUES ($1, $2, 'dep_added', $3)",
                &[&task_id, &agent_id, &depends_on.to_string()],
            )
            .await
            .map_err(TaskError::Db)?;

        // If the task was 'ready', demote to 'pending' since it now has an unmet dep
        client
            .execute(
                "UPDATE agent_tasks SET status = 'pending'::task_status \
                 WHERE id = $1 AND status = 'ready'::task_status",
                &[&task_id],
            )
            .await
            .map_err(TaskError::Db)?;

        Ok(())
    }

    /// Export tasks as JSONL (beads-compatible format).
    pub async fn export_jsonl(&self, user_id: &str) -> Result<String, TaskError> {
        let tasks = self
            .list(ListTasksParams {
                user_id: user_id.to_string(),
                include_completed: true,
                limit: Some(10000),
                ..Default::default()
            })
            .await?;

        let mut lines = Vec::new();
        for task in &tasks {
            let line = serde_json::to_string(task).map_err(|e| {
                TaskError::Serialization(format!("Failed to serialize task: {e}"))
            })?;
            lines.push(line);
        }
        Ok(lines.join("\n"))
    }

    /// Import tasks from JSONL.
    pub async fn import_jsonl(&self, jsonl: &str) -> Result<usize, TaskError> {
        let client = self.pool.get().await.map_err(TaskError::Pool)?;
        let mut count = 0;
        for line in jsonl.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let task: Task = serde_json::from_str(line).map_err(|e| {
                TaskError::Serialization(format!("Failed to deserialize task: {e}"))
            })?;

            // Upsert by content_hash if available, otherwise insert
            client
                .execute(
                    "INSERT INTO agent_tasks (id, user_id, created_by, assigned_to, title, \
                     description, status, priority, labels, metadata, result, \
                     created_at, updated_at, started_at, completed_at, due_at, content_hash) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7::task_status, $8::task_priority, \
                     $9, $10, $11, $12, $13, $14, $15, $16, $17) \
                     ON CONFLICT (id) DO UPDATE SET \
                     status = EXCLUDED.status, updated_at = now(), result = EXCLUDED.result",
                    &[
                        &task.id,
                        &task.user_id,
                        &task.created_by,
                        &task.assigned_to,
                        &task.title,
                        &task.description,
                        &task.status.as_sql(),
                        &task.priority.as_sql(),
                        &task.labels,
                        &task.metadata,
                        &task.result,
                        &task.created_at,
                        &task.updated_at,
                        &task.started_at,
                        &task.completed_at,
                        &task.due_at,
                        &task.content_hash,
                    ],
                )
                .await
                .map_err(TaskError::Db)?;

            count += 1;
        }
        Ok(count)
    }

    // --- Internal helpers ---

    /// Check if there's a path from `from` to `to` in the dependency graph (cycle detection).
    async fn has_path(
        &self,
        client: &deadpool_postgres::Client,
        from: Uuid,
        to: Uuid,
    ) -> Result<bool, TaskError> {
        let row = client
            .query_one(
                "WITH RECURSIVE dep_chain AS ( \
                     SELECT depends_on FROM agent_task_deps WHERE task_id = $1 \
                     UNION \
                     SELECT d.depends_on FROM agent_task_deps d \
                     JOIN dep_chain c ON d.task_id = c.depends_on \
                 ) \
                 SELECT EXISTS(SELECT 1 FROM dep_chain WHERE depends_on = $2)",
                &[&from, &to],
            )
            .await
            .map_err(TaskError::Db)?;
        Ok(row.get(0))
    }

    /// After a task completes, check if any dependents are now unblocked.
    async fn promote_dependents(
        &self,
        client: &deadpool_postgres::Client,
        completed_task_id: Uuid,
    ) -> Result<(), TaskError> {
        // Find tasks that depend on the completed task and are still pending
        let rows = client
            .query(
                "SELECT DISTINCT d.task_id FROM agent_task_deps d \
                 JOIN agent_tasks t ON t.id = d.task_id \
                 WHERE d.depends_on = $1 AND t.status = 'pending'::task_status",
                &[&completed_task_id],
            )
            .await
            .map_err(TaskError::Db)?;

        for row in rows {
            let candidate_id: Uuid = row.get(0);

            // Check if ALL blocking deps are now completed
            let still_blocked: bool = client
                .query_one(
                    "SELECT EXISTS( \
                         SELECT 1 FROM agent_task_deps d \
                         JOIN agent_tasks dep ON dep.id = d.depends_on \
                         WHERE d.task_id = $1 AND d.dep_type = 'blocks' \
                         AND dep.status NOT IN ('completed', 'cancelled') \
                     )",
                    &[&candidate_id],
                )
                .await
                .map_err(TaskError::Db)?
                .get(0);

            if !still_blocked {
                client
                    .execute(
                        "UPDATE agent_tasks SET status = 'ready'::task_status WHERE id = $1",
                        &[&candidate_id],
                    )
                    .await
                    .map_err(TaskError::Db)?;
            }
        }

        Ok(())
    }
}

/// Convert a database row to a Task.
fn row_to_task(row: &tokio_postgres::Row) -> Task {
    Task {
        id: row.get(0),
        user_id: row.get(1),
        created_by: row.get(2),
        assigned_to: row.get(3),
        title: row.get(4),
        description: row.get(5),
        status: TaskStatus::from_sql(row.get::<_, &str>(6)).unwrap_or(TaskStatus::Pending),
        priority: TaskPriority::from_sql(row.get::<_, &str>(7)).unwrap_or(TaskPriority::Medium),
        labels: row.get(8),
        metadata: row.get(9),
        result: row.get(10),
        created_at: row.get(11),
        updated_at: row.get(12),
        started_at: row.get(13),
        completed_at: row.get(14),
        due_at: row.get(15),
        content_hash: row.get(16),
    }
}

/// Task-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Database error: {0}")]
    Db(#[from] tokio_postgres::Error),

    #[error("Connection pool error: {0}")]
    Pool(#[source] deadpool_postgres::PoolError),

    #[error("Task not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
