# Remaining Integration Work: Reference Repositories → IronClaw

> Status: **All items implemented** (see commit history for implementation details)

This document tracks every remaining gap from the reference repositories cloned
into `examples/reference-repos/`. Each section describes **what was already ported**,
**what was missing**, and **what was done** to close the gap.

---

## Table of Contents

1. [claw-compactor: Observation Extraction + Pipeline Wiring](#1-claw-compactor-observation-extraction--pipeline-wiring)
2. [dcg: Additional Security Packs](#2-dcg-additional-security-packs)
3. [beads: Task Memory Decay](#3-beads-task-memory-decay)
4. [monty: External Function Bridge](#4-monty-external-function-bridge)
5. [clawsec: Heartbeat Integrity Wiring](#5-clawsec-heartbeat-integrity-wiring)
6. [weave: Auto-Merge in Workspace Write Path](#6-weave-auto-merge-in-workspace-write-path)
7. [Phase 3: OpenClaw Gaps](#7-phase-3-openclaw-gaps)
8. [Phase 6b: Session Intelligence (contrail)](#8-phase-6b-session-intelligence-contrail)

---

## 1. claw-compactor: Observation Extraction + Pipeline Wiring

### Already Ported
- 4-stage `CompressorPipeline` in `src/agent/compressor/`: dedup, dictionary, patterns, text_optimizer
- CJK-aware token estimation
- All stages have unit tests

### Gap A: Observation Extraction (`observations.rs`)
**Problem**: The observation extraction layer from `observation_compressor.py` was not
ported. This is the highest-savings layer (~97% on session transcripts), converting raw
message sequences into structured observation summaries.

**Solution**: New module `src/agent/compressor/observations.rs` that:
- Extracts structured observations from conversation messages (decisions, actions, facts, errors)
- Groups observations by category
- Produces a compact markdown summary
- Integrated as Stage 0 (before dedup) in the pipeline

### Gap B: Pipeline Wiring into Compaction
**Problem**: The `CompressorPipeline` existed but was not called from `compaction.rs`.
The compactor would summarize raw messages without first compressing them.

**Solution**: Modified `src/agent/compaction.rs` to run `CompressorPipeline::compress()`
on messages before generating the LLM summary. This reduces the token count sent to the
summarization LLM call, saving both tokens and cost.

---

## 2. dcg: Additional Security Packs

### Already Ported
- 9 packs: `core.git`, `core.filesystem`, `database`, `containers`, `cloud`, `system`,
  `piped_exec`, `inline_scripts`, `sensitive_paths`
- Two-phase evaluation (keyword quick-reject + regex pattern match)
- Safe patterns, fail mode, suggestions, severity levels
- 30+ tests

### Gap: Missing Security Packs
**Problem**: The reference `dcg` has 49+ security packs. IronClaw only had 9, missing
coverage for storage, secrets, remote access, CI/CD, networking, DNS, backup, and
platform-specific commands.

**Solution**: Added 11 new packs to `src/safety/command_guard.rs`:
- `storage` — S3, GCS, MinIO, Azure Blob destructive operations
- `secrets` — Vault, 1Password CLI, AWS Secrets Manager, Doppler
- `remote` — rsync, scp, ssh destructive patterns
- `ci_cd` — Jenkins, GitHub Actions, GitLab CI destructive ops
- `networking` — iptables replacement (nftables), ufw, firewalld
- `dns` — DNS record deletion, zone transfers
- `backup` — restic, borg, velero destructive operations
- `messaging` — Kafka, RabbitMQ, NATS destructive operations
- `search` — Elasticsearch, OpenSearch index deletion
- `package_managers` — npm/pip/cargo global and dangerous installs
- `env_vars` — Dangerous environment variable modifications

Total: **20 packs** (9 original + 11 new)

---

## 3. beads: Task Memory Decay

### Already Ported
- Full task DAG in `src/workspace/tasks.rs` with PostgreSQL backend
- 5 LLM tools: `task_create`, `task_list`, `task_update`, `task_ready`, `task_export`
- Dependency tracking, cycle detection, JSONL export/import

### Gap: No Memory Decay
**Problem**: Long-running task lists accumulate completed/cancelled tasks that consume
context window space. The beads reference summarizes old closed tasks to save tokens.

**Solution**: Added `archive_completed_tasks()` method to `TaskRepository` that:
- Finds tasks in terminal states (completed, failed, cancelled) older than a configurable
  retention period (default: 7 days)
- Generates a compact summary of archived tasks
- Deletes the archived tasks from the active table
- Returns the summary for optional workspace storage
- Added `task_archive` tool for LLM-initiated cleanup

---

## 4. monty: External Function Bridge

### Already Ported
- Full `PythonTool` with `MontyRun` execution, `ResourceLimits`, output formatting
- 10-second timeout, 16MB memory limit
- Safe sandboxed execution

### Gap: No External Functions
**Problem**: The `PythonTool` passes empty `vec![]` for external functions, so Python
code cannot call any host functions. This limits usefulness for data processing.

**Solution**: Added sync utility functions bridged into the Python sandbox:
- `json_parse(s)` → parse JSON string into Python dict/list
- `json_dump(obj)` → serialize Python object to JSON string
- `base64_encode(s)` → base64-encode a string
- `base64_decode(s)` → base64-decode a string
- `hash_sha256(s)` → compute SHA-256 hex digest

Note: Async workspace functions (`memory_read`, `memory_write`) are not bridged because
monty runs synchronously. This is documented as a future enhancement if monty adds async
support.

---

## 5. clawsec: Heartbeat Integrity Wiring

### Already Ported
- `IntegrityMonitor` in `src/safety/integrity.rs` with SHA-256 baselines
- Per-file protection modes: Restore, Alert, Ignore
- Hash-chained audit log
- WASM tool checksum verification

### Gap: Not Wired into Heartbeat
**Problem**: The integrity monitor exists but `check_heartbeat()` in `heartbeat.rs`
does not call it. Identity file drift would go undetected between manual checks.

**Solution**: Modified `HeartbeatRunner` to:
- Accept an optional `IntegrityMonitor` at construction
- Run `integrity.check()` on every heartbeat tick, before the LLM call
- Report violations as `NeedsAttention` with details of drifted files
- Auto-restore files with `Restore` protection mode

---

## 6. weave: Auto-Merge in Workspace Write Path

### Already Ported
- Vendored `weave-core` with 3 merge functions: `semantic_merge`, `merge_prefer_ours`,
  `merge_with_markers`
- Unit tests for clean merge and conflict scenarios

### Gap: Not Wired into Workspace Writes
**Problem**: The merge functions exist but `Workspace::write()` does a simple overwrite.
When two agents edit the same file concurrently, the last write wins.

**Solution**: Added `write_with_merge()` method to `Workspace` that:
- Reads the current content before writing
- If current content differs from what the caller expected (stale base), invokes
  `merge_prefer_ours()` for automatic conflict resolution
- Falls back to overwrite if merge fails or file is new
- Logs merge events for auditability

---

## Summary of Changes

| File | Change |
|------|--------|
| `src/agent/compressor/observations.rs` | **New** — observation extraction layer |
| `src/agent/compressor/mod.rs` | Added observation stage to pipeline |
| `src/agent/compaction.rs` | Wired CompressorPipeline before summarization |
| `src/safety/command_guard.rs` | Added 11 new security packs |
| `src/workspace/tasks.rs` | Added `archive_completed_tasks()` + summary |
| `src/tools/builtin/task.rs` | Added `task_archive` tool |
| `src/tools/builtin/python.rs` | Added external function bridge |
| `src/tools/registry.rs` | Added `task_archive` to PROTECTED_TOOL_NAMES |
| `src/agent/heartbeat.rs` | Wired IntegrityMonitor into heartbeat tick |
| `src/workspace/mod.rs` | Added `write_with_merge()` method |
| `CLAUDE.md` | Updated integration status |
| `FEATURE_PARITY.md` | Updated feature statuses |
| `docs/INTEGRATION_PLAN.md` | Marked completed items |

---

## 7. Phase 3: OpenClaw Gaps

### 7a. BOOT.md on Startup

**Problem**: `run_boot_if_present()` method existed but was never called from
the agent startup path.

**Solution**: Added call to `run_boot_if_present("system")` in `Agent::run()`
right before entering the main message loop. BOOT.md is read from workspace and
executed as a single agent turn with full tool access.

### 7b. Memory Flush with Tool Execution

**Problem**: `run_memory_flush_turn()` provided memory tool definitions to the
LLM but did not execute tool calls in the response. If the model decided to call
`memory_write`, the call was silently ignored.

**Solution**: Replaced the single-shot LLM call with an iteration loop (up to
`MEMORY_FLUSH_MAX_ITERATIONS = 3`). Each iteration:
1. Calls the LLM with available memory tools
2. If `RespondResult::Text` → done (either `NO_REPLY` or text logged)
3. If `RespondResult::ToolCalls` → executes each tool via `execute_chat_tool()`,
   appends tool results to message history, and loops

### 7c. Daily Session Reset

**Problem**: Daily reset code existed at line 569-610 of `agent_loop.rs` and
config fields existed in `settings.rs` and `config.rs`, but was listed as
unimplemented in the roadmap. It was actually already functional — just
needed to be enabled via `daily_reset_hour` config (0-23, default: None/disabled).

**Status**: Already implemented and wired. Documented as ✅ Done.

---

## 8. Phase 6b: Session Intelligence (contrail)

### 8a. Learnings System

**Problem**: No mechanism for agents to learn from experience across sessions.
Patterns and rules were lost between compaction/session boundaries.

**Solution**: New module `src/workspace/learnings.rs` with:
- `Learning` struct: rule, scope (global/repo/tool), status (candidate/active/deprecated),
  confidence (0.0-1.0), observation count, evidence chain
- `LearningRepository`: PostgreSQL-backed CRUD with auto-dedup via rule text hash
- `Evidence` linking: each learning can have multiple evidence references
- Lifecycle: candidate → active → deprecated
- 3 LLM tools: `learning_create` (auto-dedup), `learning_search`, `learning_promote`
- Prompt injection: top 15 active learnings (by confidence) injected into main session
  system prompts via `system_prompt_with_learnings()`
- Database migration: `V10__learnings.sql` (agent_learnings + agent_learning_evidence tables)

### 8b. Salience Scoring

**Problem**: Compaction treated all turns equally, potentially summarizing
high-importance turns (errors, decisions, questions) while preserving trivial ones.

**Solution**: New module `src/agent/compressor/salience.rs` with:
- `score_turn(content, role)` → SalienceResult (score + cues)
- Signal detection: questions (+0.4), errors/failures (+0.6), decisions (+0.4),
  file effects (+0.5), memory ops (+0.3), user role (+0.3), long messages (+0.2)
- `partition_by_salience(turns, threshold)` → (keep_indices, summarize_indices)
- `rank_turns(turns, max_count)` → top-N indices by descending score
- `recency_boost(ended_at, now)` → multiplicative decay factor for sessions
- Wired into `compact_with_summary()`: high-salience turns preserved verbatim as
  "Key Moments" section, only low-salience turns are fed to LLM for summarization

### 8c. Cross-Machine Session Merge

**Problem**: When Frack and Frick sync their PostgreSQL databases, duplicate session
files could be created if both agents save the same conversation to workspace.

**Solution**: Content-hash dedup on write:
- `Workspace::write_dedup(path, content)` computes SHA-256 of content
- Checks `memory_documents.content_hash` for existing match
- If match found: skip write (idempotent), return `Ok(false)`
- If no match: write normally, set `content_hash`, return `Ok(true)`
- Session save (`save_thread_to_workspace_before_new`) now uses `write_dedup`
  with graceful fallback to `write()` on error
- Database migration: adds `content_hash TEXT` column + unique index on
  `(user_id, content_hash)` to `memory_documents`

---

## Summary of All Changes (Phase 3 + Phase 6b)

| File | Change |
|------|--------|
| `src/agent/agent_loop.rs` | BOOT.md startup call, memory flush tool loop, session save dedup, learning_repo in AgentDeps |
| `src/workspace/learnings.rs` | **New** — learnings system with PostgreSQL storage |
| `src/agent/compressor/salience.rs` | **New** — turn/session salience scoring |
| `src/tools/builtin/learning.rs` | **New** — learning_create, learning_search, learning_promote |
| `src/agent/compressor/mod.rs` | Added `pub mod salience;` |
| `src/workspace/mod.rs` | Added `pub mod learnings;`, `write_dedup()`, `system_prompt_with_learnings()`, content hash storage methods |
| `src/workspace/repository.rs` | Added `has_content_hash()`, `set_content_hash()` |
| `src/tools/builtin/mod.rs` | Added `pub mod learning;` + re-exports |
| `src/tools/registry.rs` | Added `register_learning_tools()`, learning tools to PROTECTED_TOOL_NAMES |
| `src/main.rs` | Wired `learning_repo` into AgentDeps, registered learning tools |
| `src/error.rs` | Added `WorkspaceError::StorageError` variant |
| `src/agent/compaction.rs` | Salience-based partition in `compact_with_summary()` |
| `migrations/V10__learnings.sql` | **New** — agent_learnings, agent_learning_evidence tables, content_hash column |
| `CLAUDE.md` | Updated roadmap (Phase 3 + Phase 6b ✅ Done) |
| `FEATURE_PARITY.md` | Updated feature parity with 6 new deviations |
| `docs/REMAINING_INTEGRATION_WORK.md` | Added Phase 3 and Phase 6b sections |
