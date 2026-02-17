-- Agent learnings: evidence-backed rules derived from session history.
--
-- Inspired by contrail's learnings system. Each learning is an actionable
-- rule with confidence scoring, evidence tracking, and lifecycle management.
-- Scoped by (user_id, agent_id) for multi-tenant isolation.

CREATE TYPE learning_status AS ENUM (
    'candidate',    -- Newly observed pattern, not yet validated
    'active',       -- Validated and in active use
    'deprecated'    -- No longer relevant
);

CREATE TYPE learning_scope AS ENUM (
    'global',       -- Applies everywhere
    'repo',         -- Applies to a specific repository
    'tool'          -- Applies to a specific tool
);

CREATE TABLE agent_learnings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         TEXT NOT NULL,
    agent_id        TEXT NOT NULL,
    -- Content
    rule            TEXT NOT NULL,           -- Imperative sentence describing the rule
    scope           learning_scope NOT NULL DEFAULT 'global',
    scope_context   TEXT,                    -- e.g. repo path or tool name when scope != global
    -- Classification
    status          learning_status NOT NULL DEFAULT 'candidate',
    confidence      REAL NOT NULL DEFAULT 0.5 CHECK (confidence >= 0.0 AND confidence <= 1.0),
    observation_count INTEGER NOT NULL DEFAULT 1,
    -- Metadata
    tags            TEXT[] NOT NULL DEFAULT '{}',
    metadata        JSONB NOT NULL DEFAULT '{}',
    -- Timestamps
    first_seen      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Dedup: normalized rule text hash
    rule_hash       TEXT NOT NULL
);

-- Evidence linking learnings to their source data
CREATE TABLE agent_learning_evidence (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    learning_id     UUID NOT NULL REFERENCES agent_learnings(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,           -- 'session_file', 'commit', 'event_id', 'conversation'
    reference       TEXT NOT NULL,           -- Path, SHA, event ID, etc.
    context         TEXT,                    -- Human-readable snippet
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX idx_learnings_user_agent ON agent_learnings(user_id, agent_id, status);
CREATE INDEX idx_learnings_confidence ON agent_learnings(user_id, agent_id, confidence DESC);
CREATE INDEX idx_learnings_rule_hash ON agent_learnings(user_id, agent_id, rule_hash);
CREATE INDEX idx_learnings_tags ON agent_learnings USING GIN(tags);
CREATE INDEX idx_learning_evidence_learning ON agent_learning_evidence(learning_id);

-- Auto-update timestamp trigger
CREATE TRIGGER trg_learnings_updated
    BEFORE UPDATE ON agent_learnings
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_task_timestamp();

-- Content hash for cross-machine session dedup (Phase 6b)
-- Add to memory_documents if not already present
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'memory_documents' AND column_name = 'content_hash'
    ) THEN
        ALTER TABLE memory_documents ADD COLUMN content_hash TEXT;
        CREATE UNIQUE INDEX idx_memory_documents_content_hash
            ON memory_documents(user_id, content_hash)
            WHERE content_hash IS NOT NULL;
    END IF;
END $$;
