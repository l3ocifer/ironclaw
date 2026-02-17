//! Agent learnings: evidence-backed rules derived from session history.
//!
//! Inspired by contrail's learnings system. Each learning is an actionable
//! rule with confidence scoring, evidence tracking, and lifecycle management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::WorkspaceError;

/// An actionable rule derived from agent experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: Uuid,
    pub user_id: String,
    pub agent_id: String,
    /// One imperative sentence describing the rule.
    pub rule: String,
    pub scope: LearningScope,
    /// Additional context for scoped learnings (repo path, tool name).
    pub scope_context: Option<String>,
    pub status: LearningStatus,
    /// Confidence score in 0.0–1.0.
    pub confidence: f32,
    /// How many times this pattern has been observed.
    pub observation_count: i32,
    pub tags: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Where the learning applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningScope {
    Global,
    Repo,
    Tool,
}

impl LearningScope {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Global => "global",
            Self::Repo => "repo",
            Self::Tool => "tool",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "repo" => Self::Repo,
            "tool" => Self::Tool,
            _ => Self::Global,
        }
    }
}

/// Lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningStatus {
    Candidate,
    Active,
    Deprecated,
}

impl LearningStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Candidate => "candidate",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "deprecated" => Self::Deprecated,
            _ => Self::Candidate,
        }
    }
}

/// Evidence linking a learning to its source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: Uuid,
    pub kind: String,
    pub reference: String,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Normalize rule text for dedup: lowercase, collapse whitespace.
pub fn normalize_rule(rule: &str) -> String {
    rule.split_whitespace()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute a stable hash for dedup.
pub fn rule_hash(rule: &str) -> String {
    let normalized = normalize_rule(rule);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// PostgreSQL-backed learnings repository.
pub struct LearningRepository {
    pool: deadpool_postgres::Pool,
}

impl LearningRepository {
    pub fn new(pool: deadpool_postgres::Pool) -> Self {
        Self { pool }
    }

    /// Create or merge a learning. If a learning with the same rule_hash exists
    /// for this user+agent, merge it (increment count, update confidence, add evidence).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_or_merge(
        &self,
        user_id: &str,
        agent_id: &str,
        rule: &str,
        scope: &LearningScope,
        scope_context: Option<&str>,
        confidence: f32,
        tags: &[String],
        evidence_kind: Option<&str>,
        evidence_ref: Option<&str>,
        evidence_context: Option<&str>,
    ) -> Result<Learning, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        let hash = rule_hash(rule);

        // Try to find existing learning with same hash
        let existing = client
            .query_opt(
                "SELECT id, observation_count, confidence FROM agent_learnings
                 WHERE user_id = $1 AND agent_id = $2 AND rule_hash = $3",
                &[&user_id, &agent_id, &hash],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        let learning_id = if let Some(row) = existing {
            let id: Uuid = row.get(0);
            let count: i32 = row.get(1);
            let old_conf: f32 = row.get(2);
            let new_conf = old_conf.max(confidence);
            let new_count = count + 1;

            client
                .execute(
                    "UPDATE agent_learnings SET observation_count = $1, confidence = $2,
                     last_seen = now(), updated_at = now() WHERE id = $3",
                    &[&new_count, &new_conf, &id],
                )
                .await
                .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

            id
        } else {
            let scope_str = scope.as_str();
            let id = Uuid::new_v4();
            let tag_arr: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();

            client
                .execute(
                    "INSERT INTO agent_learnings (id, user_id, agent_id, rule, scope, scope_context,
                     status, confidence, observation_count, tags, rule_hash)
                     VALUES ($1, $2, $3, $4, $5::learning_scope, $6, 'candidate', $7, 1, $8, $9)",
                    &[&id, &user_id, &agent_id, &rule, &scope_str, &scope_context,
                      &confidence, &tag_arr, &hash],
                )
                .await
                .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

            id
        };

        // Add evidence if provided
        if let (Some(kind), Some(reference)) = (evidence_kind, evidence_ref) {
            let eid = Uuid::new_v4();
            client
                .execute(
                    "INSERT INTO agent_learning_evidence (id, learning_id, kind, reference, context)
                     VALUES ($1, $2, $3, $4, $5)",
                    &[&eid, &learning_id, &kind, &reference, &evidence_context],
                )
                .await
                .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;
        }

        self.get_by_id(learning_id).await
    }

    /// Get a learning by ID with evidence.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Learning, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        let row = client
            .query_one(
                "SELECT id, user_id, agent_id, rule, scope::text, scope_context,
                        status::text, confidence, observation_count, tags,
                        first_seen, last_seen, created_at, updated_at
                 FROM agent_learnings WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        let evidence = self.get_evidence(id).await?;
        Ok(row_to_learning(&row, evidence))
    }

    /// List active learnings for a user+agent, ordered by confidence desc.
    pub async fn list_active(
        &self,
        user_id: &str,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<Learning>, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        let rows = client
            .query(
                "SELECT id, user_id, agent_id, rule, scope::text, scope_context,
                        status::text, confidence, observation_count, tags,
                        first_seen, last_seen, created_at, updated_at
                 FROM agent_learnings
                 WHERE user_id = $1 AND agent_id = $2 AND status = 'active'
                 ORDER BY confidence DESC, observation_count DESC
                 LIMIT $3",
                &[&user_id, &agent_id, &limit],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        let mut learnings = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: Uuid = row.get(0);
            let evidence = self.get_evidence(id).await?;
            learnings.push(row_to_learning(row, evidence));
        }
        Ok(learnings)
    }

    /// Search learnings by text query.
    pub async fn search(
        &self,
        user_id: &str,
        agent_id: &str,
        query: &str,
        limit: i64,
    ) -> Result<Vec<Learning>, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        let pattern = format!("%{}%", query.to_lowercase());
        let rows = client
            .query(
                "SELECT id, user_id, agent_id, rule, scope::text, scope_context,
                        status::text, confidence, observation_count, tags,
                        first_seen, last_seen, created_at, updated_at
                 FROM agent_learnings
                 WHERE user_id = $1 AND agent_id = $2 AND LOWER(rule) LIKE $3
                 ORDER BY confidence DESC
                 LIMIT $4",
                &[&user_id, &agent_id, &pattern, &limit],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        let mut learnings = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: Uuid = row.get(0);
            let evidence = self.get_evidence(id).await?;
            learnings.push(row_to_learning(row, evidence));
        }
        Ok(learnings)
    }

    /// Promote a candidate learning to active.
    pub async fn promote(&self, id: Uuid) -> Result<Learning, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        client
            .execute(
                "UPDATE agent_learnings SET status = 'active', updated_at = now() WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        self.get_by_id(id).await
    }

    /// Deprecate a learning.
    pub async fn deprecate(&self, id: Uuid) -> Result<Learning, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        client
            .execute(
                "UPDATE agent_learnings SET status = 'deprecated', updated_at = now() WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        self.get_by_id(id).await
    }

    /// Format top active learnings as a prompt section for system prompt injection.
    pub async fn format_for_prompt(
        &self,
        user_id: &str,
        agent_id: &str,
        max_learnings: i64,
    ) -> Result<String, WorkspaceError> {
        let learnings = self.list_active(user_id, agent_id, max_learnings).await?;
        if learnings.is_empty() {
            return Ok(String::new());
        }

        let mut out = String::from("## Active Learnings\n\n");
        out.push_str("Rules derived from experience (confidence-ranked):\n\n");
        for (i, l) in learnings.iter().enumerate() {
            out.push_str(&format!(
                "{}. [conf={:.0}%, seen={}x] {}\n",
                i + 1,
                l.confidence * 100.0,
                l.observation_count,
                l.rule,
            ));
        }
        Ok(out)
    }

    async fn get_evidence(&self, learning_id: Uuid) -> Result<Vec<Evidence>, WorkspaceError> {
        let client = self.pool.get().await.map_err(|e| {
            WorkspaceError::StorageError { reason: format!("pool error: {}", e) }
        })?;

        let rows = client
            .query(
                "SELECT id, kind, reference, context, created_at
                 FROM agent_learning_evidence WHERE learning_id = $1
                 ORDER BY created_at",
                &[&learning_id],
            )
            .await
            .map_err(|e| WorkspaceError::StorageError { reason: e.to_string() })?;

        Ok(rows
            .iter()
            .map(|r: &tokio_postgres::Row| Evidence {
                id: r.get(0),
                kind: r.get(1),
                reference: r.get(2),
                context: r.get(3),
                created_at: r.get(4),
            })
            .collect())
    }
}

fn row_to_learning(row: &tokio_postgres::Row, evidence: Vec<Evidence>) -> Learning {
    Learning {
        id: row.get(0),
        user_id: row.get(1),
        agent_id: row.get(2),
        rule: row.get(3),
        scope: LearningScope::parse(row.get::<_, &str>(4)),
        scope_context: row.get(5),
        status: LearningStatus::parse(row.get::<_, &str>(6)),
        confidence: row.get(7),
        observation_count: row.get(8),
        tags: row.get(9),
        first_seen: row.get(10),
        last_seen: row.get(11),
        created_at: row.get(12),
        updated_at: row.get(13),
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rule() {
        assert_eq!(normalize_rule("Never  use  unwrap"), "never use unwrap");
        assert_eq!(normalize_rule("ALWAYS run clippy"), "always run clippy");
    }

    #[test]
    fn test_rule_hash_deterministic() {
        let h1 = rule_hash("Never use unwrap");
        let h2 = rule_hash("never  use  unwrap");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_rule_hash_distinct() {
        let h1 = rule_hash("Never use unwrap");
        let h2 = rule_hash("Always use Result");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_scope_roundtrip() {
        assert_eq!(LearningScope::parse("global"), LearningScope::Global);
        assert_eq!(LearningScope::parse("repo"), LearningScope::Repo);
        assert_eq!(LearningScope::parse("tool"), LearningScope::Tool);
    }
}
