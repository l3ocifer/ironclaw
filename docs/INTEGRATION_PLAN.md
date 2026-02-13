# Integration Plan: Reference Repos + OpenClaw Gaps → IronClaw

**Status:** Partially implemented. Phase 1 (security hardening), Phase 2 (compressor — partial), and Phase 4 (weave, monty, task graph) are done. Remaining items proceed section by section.  
**Principle:** Everything in Rust. Security is the highest priority. IronClaw's design philosophy leads.  
**Date:** 2026-02-13

---

## Table of Contents

- [1. Gap Analysis: Remaining OpenClaw Features](#1-gap-analysis-remaining-openclaw-features)
- [2. Reference Repository Integration](#2-reference-repository-integration)
- [3. Implementation Roadmap](#3-implementation-roadmap)
- [4. Detailed Designs](#4-detailed-designs)
- [5. Security Review Checklist](#5-security-review-checklist)

---

## 1. Gap Analysis: Remaining OpenClaw Features

These are OpenClaw memory/session features that have NOT yet been transferred to IronClaw. Each is assessed for value, risk, and priority.

### 1.1 BOOT.md on Startup (G1) — Priority: Medium

**OpenClaw behavior:** On gateway startup, if `BOOT.md` exists in workspace, run it as a single agent turn in the main session. Agent can use tools (message, memory read/write). Output is suppressed (`NO_REPLY`).

**IronClaw plan:**
- In `main.rs` or the gateway startup path, after workspace is initialized:
  1. Check if `BOOT.md` exists in workspace
  2. Read its content
  3. Run one agent turn with boot content as user message + system prompt including "run these startup instructions, reply NO_REPLY when done"
  4. Allow full tool access (same as a normal turn)
- **Security:** Boot instructions come from workspace (user-controlled, trusted). Same trust model as AGENTS.md. No new attack surface.
- **Files to modify:** `src/main.rs` or `src/agent/agent_loop.rs` (add `run_boot_if_present()`)

### 1.2 LLM Slug for Session Save (G2) — Priority: Low

**OpenClaw behavior:** When saving session on `/new`, use an LLM to generate a descriptive filename slug (e.g., `api-design`, `vendor-pitch`). Falls back to timestamp.

**IronClaw plan:**
- Current: `daily/YYYY-MM-DD-session-HHMMSS.md` (timestamp only) — this works fine.
- **Defer:** Add optional LLM slug generation in a future pass. The timestamp format is reliable and debuggable. LLM slugs add latency and an extra API call for marginal naming benefit.

### 1.3 Memory Flush with Tool Access (G3) — Priority: Medium

**OpenClaw behavior:** The pre-compaction memory flush turn has full tool access — the model can call `memory_write`, `memory_read`, `memory_search`, `file_write`, etc. to actually persist notes.

**IronClaw current:** v1 sends an empty tool list. The model can only produce text (which is logged but not acted upon).

**IronClaw plan:**
- Expand `run_memory_flush_turn()` to include memory tools (`memory_write`, `memory_read`, `memory_search`) in the tool list.
- Process tool calls in a simple loop (max 3 iterations to prevent runaway).
- Do NOT include destructive tools (shell, file write outside workspace, HTTP).
- **Security:** Only memory tools — same write scope as the agent's normal workspace. Iteration cap prevents infinite loops.
- **Files to modify:** `src/agent/agent_loop.rs` (`run_memory_flush_turn`)

### 1.4 Command Logger (G4) — Priority: Low

**OpenClaw behavior:** Logs all slash commands (`/new`, `/reset`, `/stop`) to a JSONL audit file at `~/.openclaw/logs/commands.log`.

**IronClaw plan:**
- Add structured logging (via `tracing`) for all `Submission` variants when parsed in `agent_loop.rs`.
- Use `tracing::info!(target: "audit", command = "new", session = %key, channel = %channel)`.
- IronClaw already has a tracing infrastructure; this is a log line, not a separate file.
- **Optional:** Add a dedicated JSONL audit appender in a future phase if needed.
- **Files to modify:** `src/agent/agent_loop.rs` (add trace events in submission handling)

### 1.5 Compaction Safeguard — Adaptive Chunking + Staged Pruning (G5) — Priority: High

**OpenClaw behavior:** The `compaction-safeguard` extension provides:
1. **Adaptive chunk sizing** — computes chunk ratio based on actual message sizes vs context window
2. **Staged pruning** — when new content exceeds history budget, drops oldest chunks first, summarizes them separately, feeds that as `previousSummary` to main summarization
3. **Tool failure tracking** — collects tool errors from compacted messages, appends as "Tool Failures" section
4. **File operation tracking** — records read/modified files during compacted turns, appends to summary
5. **Split-turn handling** — if a turn was split across compaction boundary, summarizes prefix separately

**IronClaw current:** Basic three-strategy compaction (summarize with keep_recent, truncate, move-to-workspace). No adaptive sizing, no staged pruning, no failure/file tracking.

**IronClaw plan:**
- Enhance `src/agent/compaction.rs` and `src/agent/context_monitor.rs`:
  1. Add `adaptive_chunk_ratio()` — compute based on message token distribution
  2. Add `prune_history_for_context_share()` — staged pruning when new content is too large
  3. Add `collect_tool_failures()` — extract tool errors from messages being compacted
  4. Add `format_file_operations()` — track files read/modified in compacted context
  5. Feed dropped-chunk summary as `previous_summary` to main summarization call
- **Security:** No new capabilities. Compaction only reads existing messages and calls LLM for summarization.
- **Files to modify:** `src/agent/compaction.rs`, `src/agent/context_monitor.rs`

### 1.6 Daily Session Reset (G7) — Priority: Medium

**OpenClaw behavior:** Sessions auto-reset at a configurable hour (default 4 AM local) or after idle timeout. On reset, compaction state is cleared.

**IronClaw plan:**
- Add optional `session.daily_reset_hour` (default: disabled) to settings.
- In the agent loop, before processing a submission, check if the current thread's last activity is before the day boundary.
- If stale, automatically trigger the equivalent of `/new` (save session, create new thread).
- **Security:** No new capabilities. Just lifecycle management.
- **Files to modify:** `src/settings.rs`, `src/agent/agent_loop.rs`

### 1.7 reserveTokensFloor Config (G9) — Priority: Medium

**OpenClaw behavior:** `agents.defaults.compaction.reserveTokensFloor` (default 20000) sets a floor for token reservation during compaction and memory flush threshold calculation.

**IronClaw current:** Uses hardcoded `COMPACTION_THRESHOLD = 0.8` ratio.

**IronClaw plan:**
- Add `compaction_reserve_tokens_floor: Option<usize>` to `settings.rs` agent config.
- Use in `ContextMonitor` and memory flush threshold calculation.
- **Files to modify:** `src/settings.rs`, `src/agent/context_monitor.rs`, `src/agent/agent_loop.rs`

### 1.8 Features NOT Being Transferred

These OpenClaw features are intentionally excluded:

| Feature | Reason |
|---------|--------|
| QMD memory backend | IronClaw uses PostgreSQL + pgvector; no need for SQLite-vec alternative |
| Session transcript indexing | Experimental in OpenClaw; IronClaw's hybrid search covers this use case |
| OpenClaw hooks infrastructure | TypeScript event system; IronClaw implements behavior inline in Rust |
| Skills/plugins/multi-agent | Out of scope for memory transfer |
| Soul-evil hook | Removed from OpenClaw; never relevant |

---

## 2. Reference Repository Integration

Each reference repo is analyzed for integration into IronClaw. All implementations will be in Rust. Security is the primary lens.

### 2.1 dcg — Destructive Command Guard (P0 — Critical)

**Repo:** `examples/reference-repos/dcg/` | **Language:** Rust | **Effort:** Easy

**What it provides:**
- 49+ security packs covering git, filesystem, databases, Kubernetes, Docker, cloud CLIs
- Sub-millisecond pattern matching with SIMD, lazy regex, quick-reject filters
- Heredoc/inline-script scanning (detects `python -c "os.remove()"`)
- Context classification (distinguishes executed commands from data in grep patterns)
- Fail-open design (timeouts/errors allow commands through — safety for availability)
- Allow-once mechanism with short codes for temporary exceptions

**Integration approach:**
- **Option A (preferred):** Use `dcg` as a Rust library dependency (add to `Cargo.toml`). Call its check function before every shell command execution in `src/tools/builtin/shell.rs`.
- **Option B:** Use as a subprocess. Shell tool spawns `dcg check` before execution. Simpler but adds process overhead.
- **Scope:** All shell commands from:
  - Built-in shell tool (`src/tools/builtin/shell.rs`)
  - Worker runtime (`src/worker/runtime.rs`)
  - Claude bridge (`src/worker/claude_bridge.rs`)
  - Docker sandbox commands

**Implementation:**
1. Add `dcg` as a dependency or vendor the pattern engine
2. Create `src/safety/command_guard.rs` wrapping dcg's API
3. Hook into `ShellTool::execute()` — call guard before `tokio::process::Command`
4. On block: return `ToolError::Blocked` with explanation; log to audit
5. On allow-once: require user approval via existing approval overlay
6. Configuration: `settings.safety.command_guard.enabled` (default: true), `settings.safety.command_guard.packs` (default: all)

**Security considerations:**
- dcg is fail-open by design; IronClaw should make this configurable (fail-open vs fail-closed)
- Agent-specific trust levels could map to dcg's per-agent config
- Must scan heredocs and inline scripts, not just the top-level command string

### 2.2 claw-compactor — Token Compression (P1 — High)

**Repo:** `examples/reference-repos/claw-compactor/` | **Language:** Python | **Effort:** Medium

**What it provides:**
- 5 compression layers: rule engine, dictionary, observation extraction, RLE, CCP
- ~97% savings on session transcripts via observation extraction
- Tiered summaries (L0 raw, L1 compressed, L2 ultra-compressed) for progressive context loading
- Near-duplicate detection via shingle hashing + Jaccard similarity
- CJK-aware processing
- Zero LLM cost — all deterministic

**Integration approach:**
Port algorithms to Rust as a new module `src/agent/compressor.rs`. This enhances IronClaw's existing compaction, not replaces it.

**Components to port:**
1. **Deduplication** (`dedup.py`) → `src/agent/compressor/dedup.rs`
   - Shingle hashing (n-gram character shingles)
   - MinHash approximation for Jaccard similarity
   - Near-duplicate message detection and removal
   
2. **Dictionary compression** (`dictionary.py`) → `src/agent/compressor/dictionary.rs`
   - Auto-learned codebook from repeated patterns
   - `$XX` substitution for common strings
   - Frequency-based selection
   
3. **RLE/pattern compression** (`rle.py`) → `src/agent/compressor/patterns.rs`
   - Path shorthand (common path prefixes → variables)
   - IP prefix compression
   - Enum/repeated-value compaction
   
4. **Observation extraction** (`observation_compressor.py`) → `src/agent/compressor/observations.rs`
   - Session JSONL → structured observations (what happened, what changed, what failed)
   - Remove raw tool output, keep structured facts
   
5. **Tiered summaries** (`generate_summary_tiers.py`) → integrate into `src/agent/compaction.rs`
   - L0: recent raw messages
   - L1: compressed older messages (dictionary + dedup)
   - L2: summarized ancient context

**Security considerations:**
- Compression is read-only transformation of existing messages — no new capabilities
- Dictionary codebook must not leak across sessions/users (per-session codebook)
- Dedup thresholds must be tuned to avoid false positives (losing distinct messages)

### 2.3 clawsec — Security Skill Suite (P1 — High)

**Repo:** `examples/reference-repos/clawsec/` | **Language:** TS/Python | **Effort:** Medium

**What it provides:**
- **soul-guardian:** Drift detection and auto-restore for SOUL.md, AGENTS.md, IDENTITY.md, USER.md
- **clawsec-feed:** NVD CVE polling for OpenClaw-related vulnerabilities
- **openclaw-audit-watchdog:** Daily security audits with email reporting
- **Checksum verification:** SHA256 for skill/tool artifacts
- **Heartbeat integration:** Security checks as heartbeat tasks

**Integration approach:**
Implement as native Rust modules integrated into IronClaw's heartbeat and safety systems.

**Components to implement:**

1. **Workspace integrity monitor** → `src/safety/integrity.rs`
   - On startup: compute SHA-256 of identity files (SOUL.md, AGENTS.md, IDENTITY.md, USER.md)
   - Store baselines in `~/.ironclaw/integrity.json`
   - On heartbeat: recompute and compare
   - On drift: warn user, optionally auto-restore from baseline
   - **Security:** Detects unauthorized modification of agent identity (prompt injection via file tampering)

2. **WASM tool checksum verification** → `src/tools/wasm/verification.rs`
   - On install: compute and store SHA-256 of WASM binary
   - On load: verify checksum before execution
   - **Security:** Detects tampered tools

3. **Heartbeat security tasks** → add to `src/agent/heartbeat.rs`
   - Check workspace integrity
   - Verify tool checksums
   - Check for stale/expired session tokens
   - Log security summary to daily log

4. **CVE/advisory feed** → `src/safety/advisory.rs` (future phase)
   - Optional NVD polling for IronClaw-related CVEs
   - Structured advisory format for heartbeat reporting

**Security considerations:**
- Integrity baselines must be stored securely (ideally encrypted or in DB, not plaintext JSON)
- Auto-restore should require user confirmation for AGENTS.md (user may have intentionally edited)
- CVE feed is read-only HTTP; rate-limit to avoid API abuse

### 2.4 weave — Semantic Merge Driver (P2 — Medium)

**Repo:** `examples/reference-repos/weave/` | **Language:** Rust | **Effort:** Easy

**What it provides:**
- Entity-level 3-way merge using tree-sitter parsers
- Resolves false merge conflicts when multiple agents edit the same file
- Supports: TypeScript, JavaScript, Python, Go, Rust, JSON, YAML, TOML, Markdown

**Integration approach:**
Use `weave-core` as a Rust dependency.

**Use cases in IronClaw:**
1. **Multi-agent memory merge** — when Frack and Frick both modify MEMORY.md or daily logs, use weave for conflict-free merging
2. **Workspace sync** — if workspace is git-backed, configure weave as the merge driver
3. **Tool-generated code** — when agent generates/edits code files, use weave to merge concurrent edits

**Implementation:**
1. Add `weave-core` to `Cargo.toml`
2. Create `src/workspace/merge.rs` — wrapper for entity-level merge
3. Integrate into workspace write operations: if concurrent write detected, attempt weave merge before overwriting
4. Configure as git merge driver for workspace repos

**Security considerations:**
- weave is a pure data transformation library — no network, no filesystem side effects
- Tree-sitter parsing is sandboxed within the library
- No new attack surface

### 2.5 monty — Secure Python Interpreter (P2 — Medium)

**Repo:** `examples/reference-repos/monty/` | **Language:** Rust | **Effort:** Medium

**What it provides:**
- Minimal Python interpreter in Rust
- No host filesystem/network/env access by default
- External function interface (expose only what you want)
- ~0.06ms startup (vs seconds for Docker)
- Snapshotting (`dump()`/`load()`) for pause/resume
- Resource limits: memory, stack depth, execution time

**Integration approach:**
Add as an alternative sandbox alongside WASM for executing Python code.

**Use cases:**
1. **Code execution tool** — when agent generates Python code to run (data analysis, calculations)
2. **Memory processing** — run user-defined Python transforms on workspace data
3. **Lightweight scripting** — faster than Docker sandbox for simple scripts

**Implementation:**
1. Add `monty` to `Cargo.toml`
2. Create `src/tools/builtin/python.rs` — `PythonTool` that uses monty
3. Expose only approved host functions:
   - `print()` — captured as tool output
   - `read_file()` — proxied through workspace (path-restricted)
   - `write_file()` — proxied through workspace
   - `json_parse()`, `json_dump()` — data manipulation
4. NO: `os`, `subprocess`, `socket`, `ctypes`, file I/O outside sandbox
5. Resource limits: 10s execution, 64MB memory, 1000 recursion depth

**Security considerations:**
- Default deny for all host access — capability-based, matches IronClaw philosophy
- Snapshotting must be stored securely (serialized state could contain sensitive data)
- Type checker (`ty`) provides additional safety layer
- Must audit monty's Rust implementation for unsafe blocks

### 2.6 beads — Task Dependency Graph (P3 — Future)

**Repo:** `examples/reference-repos/beads/` | **Language:** Go | **Effort:** Hard

**What it provides:**
- Dependency-aware task graph for long-horizon planning
- Hash-based IDs (collision-free in multi-agent)
- Semantic memory decay (summarize old closed tasks)
- Git-backed persistence

**Integration approach (future):**
- Extract the task dependency model and implement in Rust
- Use for IronClaw's job scheduling and routine planning
- Hash-based IDs for multi-agent task coordination between Frack and Frick
- NOT a priority until multi-agent coordination is needed

### 2.7 aline — Git-Backed Memory (P3 — Future)

**Repo:** `examples/reference-repos/aline/` | **Language:** Python | **Effort:** Design only

**Relevant concepts:**
- Auto-commit agent trajectory as git history → versioned, auditable memory
- Push to share context across sessions
- Per-project memory isolation

**IronClaw relevance:** PostgreSQL already provides versioned, queryable memory. Git-backing could be layered on top for workspace files. Useful design pattern but not urgent.

### 2.8 OneContext — Unified Agent Context (P3 — Future)

**Repo:** `examples/reference-repos/OneContext/` | **Language:** Node/Python | **Effort:** Design only

**Relevant concepts:**
- Shareable context links
- Session archiving and resume
- Multi-agent context handoff

**IronClaw relevance:** Session archiving maps to workspace session files. Multi-agent handoff is relevant for Frack↔Frick but needs custom design around shared PostgreSQL.

---

## 3. Implementation Roadmap

### Phase 1: Security Hardening (P0) — Estimated: 1-2 days

| # | Task | Files | Depends On |
|---|------|-------|------------|
| 1.1 | Integrate dcg as command guard for shell tool | `Cargo.toml`, `src/safety/command_guard.rs`, `src/tools/builtin/shell.rs` | — |
| 1.2 | Add workspace integrity monitoring | `src/safety/integrity.rs`, `src/agent/heartbeat.rs` | — |
| 1.3 | Add WASM tool checksum verification | `src/tools/wasm/verification.rs`, `src/tools/wasm/loader.rs` | — |

### Phase 2: Compaction Enhancement (P1) — Estimated: 2-3 days

| # | Task | Files | Depends On |
|---|------|-------|------------|
| 2.1 | Port dedup (shingle hashing + MinHash) | `src/agent/compressor/dedup.rs` | — |
| 2.2 | Port dictionary compression | `src/agent/compressor/dictionary.rs` | — |
| 2.3 | Port pattern/RLE compression | `src/agent/compressor/patterns.rs` | — |
| 2.4 | Port observation extraction | `src/agent/compressor/observations.rs` | — |
| 2.5 | Implement adaptive chunk sizing | `src/agent/compaction.rs`, `src/agent/context_monitor.rs` | — |
| 2.6 | Implement staged pruning | `src/agent/compaction.rs` | 2.5 |
| 2.7 | Add tool failure + file operation tracking to summaries | `src/agent/compaction.rs` | 2.5 |
| 2.8 | Implement tiered summaries (L0/L1/L2) | `src/agent/compaction.rs` | 2.1-2.4 |
| 2.9 | Add reserveTokensFloor config | `src/settings.rs`, `src/agent/context_monitor.rs` | — |

### Phase 3: Remaining OpenClaw Gaps (P1) — Estimated: 1-2 days

| # | Task | Files | Depends On |
|---|------|-------|------------|
| 3.1 | BOOT.md on startup | `src/agent/agent_loop.rs` or `src/main.rs` | — |
| 3.2 | Memory flush with tool access | `src/agent/agent_loop.rs` | — |
| 3.3 | Daily session reset | `src/settings.rs`, `src/agent/agent_loop.rs` | — |
| 3.4 | Command audit logging | `src/agent/agent_loop.rs` | — |

### Phase 4: Advanced Integration (P2) — Estimated: 2-3 days

| # | Task | Files | Depends On |
|---|------|-------|------------|
| 4.1 | Add weave-core dependency for semantic merge | `Cargo.toml`, `src/workspace/merge.rs` | — |
| 4.2 | Add monty for sandboxed Python execution | `Cargo.toml`, `src/tools/builtin/python.rs` | — |
| 4.3 | Heartbeat security tasks | `src/agent/heartbeat.rs` | 1.2, 1.3 |
| 4.4 | CVE/advisory feed (optional) | `src/safety/advisory.rs` | — |

### Phase 5: Future (P3) — No timeline

- Task dependency graph (beads concepts)
- Git-backed workspace versioning (aline concepts)
- Multi-agent context handoff (OneContext concepts)
- LLM slug for session save filenames

---

## 4. Detailed Designs

### 4.1 Command Guard Integration (dcg)

```
User sends shell command
        │
        ▼
┌──────────────────────┐
│  ShellTool::execute() │
│                       │
│  1. Parse command     │
│  2. Check approval    │
│  3. ──► CommandGuard  │──── BLOCKED ──► ToolError::Blocked
│         .check(cmd)   │                 + audit log
│  4. Execute command   │
│  5. Return output     │
└──────────────────────┘
```

```rust
// src/safety/command_guard.rs
pub struct CommandGuard {
    enabled: bool,
    fail_mode: FailMode, // Open or Closed
}

pub enum GuardResult {
    Allow,
    Block { reason: String, pack: String },
    AllowOnce { code: String },
}

impl CommandGuard {
    pub fn check(&self, command: &str) -> GuardResult { /* ... */ }
}
```

### 4.2 Workspace Integrity Monitor

```rust
// src/safety/integrity.rs
use sha2::{Sha256, Digest};

pub struct IntegrityMonitor {
    baselines: HashMap<String, String>, // path → SHA-256 hex
}

const MONITORED_FILES: &[&str] = &[
    "SOUL.md", "AGENTS.md", "IDENTITY.md", "USER.md", "HEARTBEAT.md",
];

impl IntegrityMonitor {
    pub fn compute_baseline(workspace: &Workspace) -> Self { /* ... */ }
    pub fn check(&self, workspace: &Workspace) -> Vec<IntegrityViolation> { /* ... */ }
    pub fn store(&self, path: &Path) -> Result<()> { /* ... */ }
    pub fn load(path: &Path) -> Result<Self> { /* ... */ }
}

pub struct IntegrityViolation {
    pub file: String,
    pub expected_hash: String,
    pub actual_hash: String,
    pub action: ViolationAction, // Warn, AutoRestore, Block
}
```

### 4.3 Tiered Compaction

```
Context Window
┌────────────────────────────────────────────────┐
│ System Prompt + Identity Files                  │
├────────────────────────────────────────────────┤
│ L2 Summary (ancient context, ultra-compressed) │
├────────────────────────────────────────────────┤
│ L1 Messages (older, dictionary+dedup applied)  │
├────────────────────────────────────────────────┤
│ L0 Messages (recent, raw)                      │
├────────────────────────────────────────────────┤
│ Tool Failures + File Operations (from compact) │
└────────────────────────────────────────────────┘
```

---

## 5. Security Review Checklist

Every integration must pass this checklist before merging:

- [ ] **No new filesystem access** beyond existing workspace paths
- [ ] **No new network access** unless explicitly allowlisted
- [ ] **No new shell execution** without command guard
- [ ] **No secret exposure** — credentials never passed to new components
- [ ] **Capability-based** — new features opt-in, not default-enabled
- [ ] **Fail-safe defaults** — new features fail closed unless explicitly configured otherwise
- [ ] **Input validation** — all external input sanitized before processing
- [ ] **Resource limits** — all new execution paths have timeout/memory limits
- [ ] **Audit trail** — all security-relevant actions logged via tracing
- [ ] **No unsafe Rust** — unless absolutely necessary and documented
- [ ] **Clippy clean** — no warnings
- [ ] **Tests** — at least unit tests for security-critical paths

---

*Plan completed 2026-02-13. Proceed with Phase 1 (security hardening) first.*
