# Execution Plan Template

Plans are first-class artifacts. They live in `docs/exec-plans/active/` while in flight,
move to `docs/exec-plans/completed/` when done. Version them. Log decisions in them.

## Lightweight Plan (small changes, <1 day)

```markdown
# Plan: [Title]

**Status:** active | completed | abandoned
**Created:** YYYY-MM-DD
**Owner:** [agent or human]

## Goal
[One sentence: what and why]

## Steps
- [ ] Step 1
- [ ] Step 2
- [ ] Step 3

## Decisions
- [YYYY-MM-DD] [Decision and rationale]

## Outcome
[Filled on completion]
```

## Full Execution Plan (complex work, multi-day)

```markdown
# Plan: [Title]

**Status:** active | blocked | completed | abandoned
**Created:** YYYY-MM-DD
**Last updated:** YYYY-MM-DD
**Owner:** [agent or human]
**Estimated effort:** [hours/days]

## Goal
[What we're building and why it matters]

## Context
[Background, links to design docs, product specs]

## Scope
### In scope
- [Item 1]
- [Item 2]

### Out of scope
- [Explicitly excluded item]

## Approach
[High-level strategy, key architectural decisions]

## Phases
### Phase 1: [Name]
- [ ] Task 1.1
- [ ] Task 1.2

### Phase 2: [Name]
- [ ] Task 2.1
- [ ] Task 2.2

## Dependencies
- [External dependency or prerequisite]

## Risks
- [Risk and mitigation]

## Decision Log
| Date | Decision | Rationale |
|------|----------|-----------|
| YYYY-MM-DD | [What] | [Why] |

## Progress Log
| Date | Update |
|------|--------|
| YYYY-MM-DD | [What happened] |

## Outcome
[Filled on completion: what shipped, what was learned, what debt remains]
```

## Tech Debt Tracker

Keep a running `docs/exec-plans/tech-debt.md`:

```markdown
# Technical Debt Tracker

## Critical (blocks progress)
- [ ] [Debt item] — [Impact] — Filed: YYYY-MM-DD

## High (degrades quality)
- [ ] [Debt item] — [Impact] — Filed: YYYY-MM-DD

## Medium (cleanup when convenient)
- [ ] [Debt item] — [Impact] — Filed: YYYY-MM-DD

## Resolved
- [x] [Debt item] — Resolved: YYYY-MM-DD via [PR/commit]
```
