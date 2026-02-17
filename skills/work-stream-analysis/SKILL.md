---
name: work-stream-analysis
description: Analyze a task to identify parallelizable work streams before execution. Inspired by ccpm's issue-analyze pattern.
homepage: https://github.com/automazeio/ccpm
metadata:
  category: planning
  requires: none
---

# Work Stream Analysis

Before starting a complex task, analyze it to identify independent work streams that can execute in parallel.

## When to Use

- Task touches multiple layers (DB, API, UI, tests)
- Task involves multiple files with no shared state
- Estimated effort > 2 hours
- Multiple sub-components have clear boundaries

## Analysis Process

1. **Identify layers**: What architectural layers does this task touch?
2. **Map file patterns**: Which files belong to each layer?
3. **Find dependencies**: Which streams must complete before others can start?
4. **Flag conflicts**: Which files might be touched by multiple streams?
5. **Define stream boundaries**: Clear inputs/outputs for each stream

## Output Format

For each work stream:
- **Stream name**: e.g. "Database migrations"
- **Files**: List of files this stream will create/modify
- **Dependencies**: Which other streams must complete first
- **Parallel**: true/false — can this run alongside other streams?
- **Estimated effort**: Small/Medium/Large

## Example

Task: "Add user authentication"

| Stream | Files | Dependencies | Parallel |
|--------|-------|-------------|----------|
| DB schema | `migrations/`, `src/models/user.rs` | None | Yes |
| Auth service | `src/services/auth.rs` | DB schema | Yes (after DB) |
| API endpoints | `src/routes/auth.rs` | Auth service | Yes (after service) |
| Middleware | `src/middleware/auth.rs` | Auth service | Yes (after service) |
| Tests | `tests/auth/` | All above | No |

## Key Principles

- **File-level isolation**: Streams that touch different files can safely run in parallel
- **Dependency ordering**: Use the dependency graph to determine execution order
- **Conflict zones**: Files touched by multiple streams need sequential handling
- **Context firewall**: Each stream should work with minimal context, returning a concise summary
