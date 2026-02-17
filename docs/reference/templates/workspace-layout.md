# Recommended Workspace Layout for Agent-Managed Projects

Give the agent a map, not a thousand-page manual. AGENTS.md is the table of contents.
Everything else lives in structured directories the agent navigates on demand.

## Layout

```
project/
├── AGENTS.md              ← Operating instructions (~100 lines, pointers to deeper docs)
├── SOUL.md                ← Agent identity and personality
├── USER.md                ← User context and preferences
├── IDENTITY.md            ← Display name, avatar, presentation
├── MEMORY.md              ← Long-term curated index (lean, <20K chars)
├── BOOT.md                ← Startup checklist (run on gateway restart)
├── HEARTBEAT.md           ← Periodic checklist (<20 lines)
├── active-tasks.md        ← Crash recovery: in-progress, blocked, queued
├── lessons.md             ← Patterns and corrections learned over time
│
├── daily/                 ← Daily logs (append-only, one per day)
│   ├── 2026-02-13.md
│   └── 2026-02-14.md
│
├── docs/                  ← Knowledge store (system of record)
│   ├── ARCHITECTURE.md    ← Top-level system map, domains, layering
│   ├── DESIGN.md          ← Design principles, core beliefs
│   ├── QUALITY.md         ← Quality grades per domain/layer
│   │
│   ├── design-docs/       ← Design decisions (versioned, indexed)
│   │   ├── index.md       ← Catalogue with verification status
│   │   └── *.md
│   │
│   ├── exec-plans/        ← Execution plans (first-class artifacts)
│   │   ├── active/        ← Currently in-flight
│   │   ├── completed/     ← Done (kept for reference)
│   │   └── tech-debt.md   ← Known debt, tracked and graded
│   │
│   ├── product-specs/     ← Product requirements and specs
│   │   ├── index.md
│   │   └── *.md
│   │
│   └── references/        ← External docs, LLM-friendly formats
│       └── *.md
│
├── projects/              ← Project-specific context
│   └── {project-name}/
│       ├── README.md
│       ├── status.md
│       └── notes.md
│
└── goals/                 ← Goal tracking
    └── *.md
```

## Principles

### AGENTS.md is a map, not a manual
Keep it under 150 lines. It tells the agent *where to look*, not *everything it needs to know*.
Deep context lives in docs/ and the agent reads it on demand via the workspace search and read tools.

### Repository knowledge is the system of record
If it's not in the repo, it doesn't exist to the agent. Slack discussions, Google Docs, verbal
decisions — encode them as versioned markdown or they're invisible.

### Progressive disclosure
The agent starts with AGENTS.md (injected into context), follows pointers to docs/ as needed,
and drills into specific files for detail. This avoids crowding out the actual task with
irrelevant context.

### Enforce mechanically, not just instructionally
When a rule matters, encode it in tooling (linters, CI, tests) not just documentation.
Custom lint error messages should include remediation instructions — the agent reads them.

### Capture taste as artifacts
Review comments, refactoring patterns, and bug post-mortems should feed back into docs/ as
updates to design docs, quality grades, or lessons.md. Taste captured once applies everywhere.

### Doc gardening
Schedule a recurring routine (weekly cron) to scan for stale docs that don't reflect the
current codebase. Flag them for update or archive. Documentation rots faster than code.
