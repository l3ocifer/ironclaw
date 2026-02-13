-- Agent task graph (inspired by beads/JSONL task model).
--
-- Supports multi-agent task coordination with:
-- - Dependency tracking (DAG)
-- - Per-agent task ownership and assignment
-- - Status lifecycle with event history
-- - JSONL-compatible export for interop with beads tooling
-- - Scoped by (user_id, agent_id) for multi-tenant isolation

-- Task status enum
CREATE TYPE task_status AS ENUM (
    'pending',      -- Created, not yet actionable (blocked by dependencies)
    'ready',        -- All dependencies met, can be picked up
    'in_progress',  -- Currently being worked on by an agent
    'blocked',      -- Manually blocked (e.g., waiting for external input)
    'completed',    -- Successfully finished
    'failed',       -- Failed with error
    'cancelled'     -- Cancelled by user or agent
);

-- Task priority enum
CREATE TYPE task_priority AS ENUM (
    'critical',     -- P0: drop everything
    'high',         -- P1: do next
    'medium',       -- P2: normal queue
    'low'           -- P3: backlog
);

-- Main tasks table
CREATE TABLE agent_tasks (
    -- Primary key: content-addressable hash of (user_id, title, created_at)
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Ownership
    user_id         TEXT NOT NULL,
    created_by      TEXT NOT NULL,       -- agent_id that created this task
    assigned_to     TEXT,                -- agent_id currently responsible (NULL = unassigned)
    -- Content
    title           TEXT NOT NULL,
    description     TEXT,
    -- Classification
    status          task_status NOT NULL DEFAULT 'pending',
    priority        task_priority NOT NULL DEFAULT 'medium',
    -- Metadata
    labels          TEXT[] NOT NULL DEFAULT '{}',
    metadata        JSONB NOT NULL DEFAULT '{}',
    -- Result
    result          TEXT,                -- completion notes or error message
    -- Timestamps
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    due_at          TIMESTAMPTZ,
    -- JSONL interop: beads-compatible hash for dedup and sync
    content_hash    TEXT
);

-- Task dependency edges (DAG)
CREATE TABLE agent_task_deps (
    task_id         UUID NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    depends_on      UUID NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    -- Dependency type: 'blocks' (hard), 'relates' (soft/informational)
    dep_type        TEXT NOT NULL DEFAULT 'blocks',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, depends_on),
    -- Prevent self-references
    CHECK (task_id != depends_on)
);

-- Task event log (append-only audit trail)
CREATE TABLE agent_task_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id         UUID NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    agent_id        TEXT NOT NULL,       -- which agent made this change
    event_type      TEXT NOT NULL,       -- 'created', 'status_change', 'assigned', 'comment', 'dep_added', 'dep_removed'
    old_value       TEXT,
    new_value       TEXT,
    comment         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes for common queries
CREATE INDEX idx_agent_tasks_user_status ON agent_tasks(user_id, status);
CREATE INDEX idx_agent_tasks_assigned ON agent_tasks(assigned_to, status);
CREATE INDEX idx_agent_tasks_priority ON agent_tasks(user_id, priority, status);
CREATE INDEX idx_agent_tasks_labels ON agent_tasks USING GIN(labels);
CREATE INDEX idx_agent_tasks_created ON agent_tasks(user_id, created_at DESC);
CREATE INDEX idx_agent_task_events_task ON agent_task_events(task_id, created_at);
CREATE INDEX idx_agent_task_deps_depends ON agent_task_deps(depends_on);

-- Auto-update updated_at on tasks
CREATE OR REPLACE FUNCTION update_agent_task_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_agent_tasks_updated
    BEFORE UPDATE ON agent_tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_task_timestamp();

-- View: tasks that are ready to work on (all hard deps completed)
CREATE VIEW agent_tasks_ready AS
SELECT t.*
FROM agent_tasks t
WHERE t.status IN ('pending', 'ready')
  AND NOT EXISTS (
    SELECT 1
    FROM agent_task_deps d
    JOIN agent_tasks dep ON dep.id = d.depends_on
    WHERE d.task_id = t.id
      AND d.dep_type = 'blocks'
      AND dep.status NOT IN ('completed', 'cancelled')
  );
