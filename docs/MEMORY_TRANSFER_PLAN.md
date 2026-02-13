# Memory Transfer Plan: OpenClaw → IronClaw

**Status:** Plan only. No code changes to IronClaw until this plan is approved.  
**Scope:** Memory-oriented and agent persistent-session behavior only. Security-conscious; minimal changes.

---

## 1. Source and Target

| Item | Location |
|------|----------|
| **IronClaw** | `/Users/leo/git/ai/ironclaw` (cloned from https://github.com/nearai/ironclaw) |
| **OpenClaw** | `/Users/leo/git/ai/openclaw` (existing repo with custom memory/hooks) |
| **Unified agent instructions** | `/Users/leo/git/homelab/unified-ai-configs/instructions/AGENTS.md` (reference for “Morning Ritual”, memory rules, safety) |

---

## 2. OpenClaw Memory/Session Features (Summary)

From git history, docs, and code:

| Feature | OpenClaw behavior | Security note |
|--------|--------------------|---------------|
| **Session memory hook** | On `/new`: read current session transcript, last N user/assistant messages → write `memory/YYYY-MM-DD-<slug>.md` with LLM-generated slug (or timestamp). Config: `messages` (default 15). | Reads only gateway-owned session file; writes only under workspace. LLM slug is optional (can use timestamp-only). |
| **Logseq-prime hook** | On agent bootstrap: read `graphPath/pages/aiNamespace/shared/Leo.md`, `.../Agent/preferences.md`, `.../Agent/decisions.md` → inject into MEMORY.md bootstrap. Config: graphPath, aiNamespace, maxTokens, include* flags. | Reads user-configured path; must restrict to graphPath, .md only, no symlink follow. |
| **Pre-compaction memory flush** | Before auto-compaction: one silent agentic turn (“write durable memory now”, NO_REPLY). Tracked per session (memoryFlushAt, memoryFlushCompactionCount). Only when workspace writable. | Same trust as normal turn; no new capabilities. |
| **AGENTS.md / templates** | Rich templates: read SOUL → USER → daily notes → MEMORY (main session only); “write it down”; safety; heartbeats; group chat behavior. | Content only; no security impact. |
| **MEMORY.md only in main** | MEMORY.md loaded only for main/direct session, not in group/channel contexts. | Privacy: avoid leaking long-term memory into group chats. |
| **Boot-md hook** | On gateway startup: run BOOT.md from workspace if present. | Workspace path only. |
| **Daily logs** | OpenClaw: `memory/YYYY-MM-DD.md`. IronClaw already: `daily/YYYY-MM-DD.md`. | Already aligned conceptually. |

---

## 3. IronClaw Current State (Relevant Parts)

- **Workspace:** `MEMORY.md`, `AGENTS.md`, `SOUL.md`, `USER.md`, `IDENTITY.md`, `HEARTBEAT.md`, `daily/YYYY-MM-DD.md`. System prompt built from identity files + today/yesterday daily in `workspace/mod.rs` (`system_prompt()`). No “main vs group” distinction yet.
- **Compaction:** `context_monitor` + `ContextCompactor` in `agent_loop.rs`; no pre-compaction memory flush.
- **/new:** `Submission::NewThread` in `agent/submission.rs`; creates new thread; no export of previous thread to workspace.
- **Hooks:** Not implemented (FEATURE_PARITY: “Bundled hooks ❌”, “Hooks system ❌”). Equivalent behavior must be implemented inline in Rust where desired.

---

## 4. What to Transfer (Memory-Only, Security-Safe)

### 4.1 Content / Templates (no security risk)

- **Action:** Add optional “recommended” memory/agent instructions that mirror OpenClaw + unified-ai-configs:
  - Read SOUL, USER, daily (today + yesterday), and MEMORY only in “main” session.
  - “Write it down” — no “mental notes”; persist to files.
  - Short safety rules (no exfil, no destructive commands without asking, trash over rm).
- **Implementation:**  
  - Keep IronClaw’s default seeded `AGENTS.md` short.  
  - Add a **recommended template** in `docs/` (e.g. `docs/reference/AGENTS.recommended.md`) that users can copy into their workspace.  
  - Optionally: document how to point workspace seed at a custom template (e.g. env or config path) for advanced users.  
- **Do not:** Hardcode user-specific content (e.g. “Leo”); keep templates generic or clearly user-editable.

### 4.2 Session save on /new (session-memory equivalent)

- **Behavior:** When the user sends `/new`, before or when creating the new thread, persist the **current thread’s last N user/assistant messages** into the workspace.
- **Options:**
  - **A (simplest, no LLM):** Append to today’s daily log, or write a single file e.g. `daily/YYYY-MM-DD-session-end-HHMMSS.md` with a fixed header + message excerpts.
  - **B (closer to OpenClaw):** Same as A for v1; later, optional LLM-generated slug for filename (same security model as other agent turns).
- **Security:** Use only in-memory thread history (no arbitrary file read). Write only under workspace paths. No execution of user-controlled paths.
- **Where in IronClaw:** In the code path that handles `Submission::NewThread` (e.g. in `agent_loop.rs` or `session_manager.rs`): get current thread messages, format, then call `workspace.append_daily_log(...)` or `workspace.write(daily_path, content)` (with path from workspace API only).

### 4.3 Pre-compaction memory flush

- **Behavior:** Immediately before running auto-compaction (in `agent_loop.rs` where `context_monitor.suggest_compaction` is used), if config enables it and workspace is writable:
  - Run one **silent** agentic turn: system prompt + user prompt asking the model to write durable notes to memory and reply with NO_REPLY if nothing to store.
  - Store in thread/session metadata: e.g. `last_memory_flush_compaction_count` (or equivalent) so we only run flush once per compaction cycle.
- **Config:** Add something like `compaction.memory_flush` with: `enabled`, `soft_threshold_tokens`, `system_prompt`, `prompt` (defaults matching OpenClaw semantics).
- **Security:** Same as a normal turn; no new capabilities or file access.

### 4.4 MEMORY.md only in main session

- **Behavior:** Load `MEMORY.md` into the system prompt only when the conversation is a “main” (direct) session, not in group/channel contexts.
- **Implementation:**  
  - IronClaw currently has `channel` and optional `external_thread_id`; it does not yet have an explicit “main vs group” flag.  
  - Add a clear notion where needed (e.g. from channel type or a “main” channel name) and pass it into workspace/system prompt building.  
  - In `Workspace::system_prompt()` (or caller): if “not main”, omit the `MEMORY.md` read (and optionally omit or shorten other private context).  
- **Security:** Reduces risk of leaking long-term personal memory into group chats.

### 4.5 Logseq-prime equivalent (optional, Phase 2)

- **Behavior:** At bootstrap (or when building system prompt), if config has a “Logseq path” (e.g. `memory.logseq_graph_path`), read from that path under strict rules and prepend to MEMORY content for that request.
- **Rules:** Allowlist: only under the configured path, only `.md` files, no symlink following, max size/token limit. No execution.
- **Security:** User-configured path; strict path normalization and allowlisting to avoid LFI.

### 4.6 BOOT.md on startup (optional, Phase 2)

- **Behavior:** On gateway/process startup, if workspace has `BOOT.md`, run it once (e.g. one agentic turn or inject into first run). Lower priority than 4.1–4.4.

---

## 5. Explicitly Out of Scope (or Defer)

- **OpenClaw-specific hooks infrastructure** (TypeScript events, plugin hooks). We implement equivalent behavior in Rust, not a full hook subsystem in v1.
- **QMD / other memory backends** (OpenClaw has QMD, SQLite-vec, etc.). IronClaw uses PostgreSQL + workspace; no change for this transfer.
- **Session management details** (daily reset, idle reset, per-channel reset). Only memory-related behavior is in scope.
- **Skills, plugins, multi-agent** from OpenClaw. Out of scope for this plan.

---

## 6. Implementation Order (Recommended)

1. **4.1** – Add `docs/reference/AGENTS.recommended.md` (and optionally IDENTITY/USER recommendations) so users can copy memory/session instructions without changing default seed.
2. **4.4** – Add “main session” notion and omit MEMORY.md from system prompt for non-main sessions.
3. **4.2** – On `/new`, write current thread summary to workspace (daily or dated file).
4. **4.3** – Pre-compaction memory flush (config + one silent turn + session metadata).
5. **4.5** – Logseq-prime-style bootstrap injector (optional, Phase 2).
6. **4.6** – BOOT.md on startup (optional, Phase 2).

---

## 7. Files to Touch (High Level)

| Area | Files (IronClaw) |
|------|------------------|
| Templates | New: `docs/reference/AGENTS.recommended.md`. Possibly `docs/reference/IDENTITY.recommended.md`. |
| Main vs group | `src/workspace/mod.rs` (system_prompt), call sites that pass channel/session type; possibly `src/agent/agent_loop.rs`, `src/channels/*`. |
| Session save on /new | `src/agent/agent_loop.rs` (where `Submission::NewThread` is handled), `src/agent/session_manager.rs` (thread access). |
| Pre-compaction flush | `src/agent/agent_loop.rs`, `src/agent/context_monitor.rs` or compaction module; config schema (e.g. `settings.rs` or config types). |
| Config | Settings/config for `compaction.memory_flush`, and optionally `memory.logseq_graph_path` later. |

---

## 8. References

- OpenClaw: `docs/concepts/memory.md`, `docs/concepts/session.md`, `docs/reference/session-management-compaction.md`, `docs/reference/templates/AGENTS.md`, `docs/reference/templates/IDENTITY.md`.
- OpenClaw hooks: `src/hooks/bundled/session-memory/handler.ts`, `src/hooks/bundled/logseq-prime/handler.ts`, `src/hooks/bundled/boot-md/HOOK.md`.
- OpenClaw memory flush: `src/auto-reply/reply/memory-flush.ts`, `src/auto-reply/reply/agent-runner-memory.ts`, `src/config/types.agent-defaults.ts` (memoryFlush).
- IronClaw: `README.md`, `FEATURE_PARITY.md`, `src/workspace/mod.rs`, `src/agent/agent_loop.rs`, `src/agent/compaction.rs`, `src/agent/submission.rs`.

---

---

## 9. Gap Analysis (Post-Implementation Audit, 2026-02-13)

After implementing §4.1–§4.5, a comprehensive audit of all 9,160 OpenClaw commits and source code revealed the following remaining gaps:

| # | Feature | Status | Priority | Notes |
|---|---------|--------|----------|-------|
| G1 | BOOT.md on startup | **Not implemented** | Medium | Plan §4.6 — deferred |
| G2 | LLM slug for session save filenames | **Not implemented** | Low | Timestamp works; LLM slug adds latency for marginal benefit |
| G3 | Memory flush with full tool access | **Partial** | Medium | v1 is silent (no tools); OpenClaw gives full tool access |
| G4 | Command logger (JSONL audit) | **Not implemented** | Low | Use tracing instead |
| G5 | Compaction safeguard (adaptive chunks, staged pruning, tool failures, file ops) | **Not implemented** | High | See INTEGRATION_PLAN.md §2.2 |
| G6 | Session transcript indexing | **Not implemented** | Low | Experimental in OpenClaw |
| G7 | Daily session reset (4 AM) | **Not implemented** | Medium | Session lifecycle feature |
| G8 | QMD memory backend | **Not transferring** | N/A | IronClaw uses PostgreSQL |
| G9 | reserveTokensFloor config | **Not implemented** | Medium | Hardcoded threshold ratio instead |
| G10 | MEMORY.md code-enforced group filtering | **IronClaw is better** | N/A | OpenClaw is instruction-only for groups; IronClaw enforces via `is_main_session()` |

### What Was Successfully Transferred

| Feature | Plan Section | Status |
|---------|-------------|--------|
| AGENTS.md recommended template | §4.1 | ✅ Complete |
| Session save on `/new` | §4.2 | ✅ Complete |
| Pre-compaction memory flush | §4.3 | ✅ Complete (v1 — no tools) |
| MEMORY.md main-session-only | §4.4 | ✅ Complete |
| Logseq integration | §4.5 | ✅ Complete |

Full integration plan for remaining gaps and reference repos: [`docs/INTEGRATION_PLAN.md`](INTEGRATION_PLAN.md).

---

## 10. Deployment Topology

### Agent: Frack (MacBook)
- Primary interactive agent on Leo's development machine
- Channels: CLI/TUI, web gateway (localhost)
- Local filesystem access, Logseq graph

### Agent: Frick (Homelab Server — `alef`)
- Production/infrastructure agent on homelab server (`ssh alef`)
- Shared services: PostgreSQL, Redis, Prometheus, Grafana, K3s, Ollama, ComfyUI
- Internet access via Traefik + Cloudflare tunnel

### Shared Infrastructure
- PostgreSQL on `alef` for workspace/memory persistence
- Logseq graph (synced natively)
- Same user identity (Leo), different agent identities (SOUL.md)

---

*Plan completed. Implementation of §4.1–§4.5 done. Remaining gaps tracked in [INTEGRATION_PLAN.md](INTEGRATION_PLAN.md). Keep all changes minimal, Rust-only, and security-first.*
