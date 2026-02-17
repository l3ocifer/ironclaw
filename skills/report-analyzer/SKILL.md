---
name: report-analyzer
description: Analyze daily reports (metrics, errors, feedback) to identify the #1 actionable priority. Use for proactive self-improvement or when asked to analyze reports.
homepage: https://github.com/snarktank/compound-product
metadata: { "openclaw": { "emoji": "📊", "requires": { "bins": [] } } }
---

# Report Analyzer

Analyze reports to identify the single highest-impact actionable item. Designed for autonomous improvement loops.

## The Job

1. Read the report file(s) specified by the user
2. Identify all actionable items
3. Rank by impact (revenue > user experience > code quality > cosmetic)
4. Select the #1 priority
5. Output a structured analysis

## Input

Reports can be any markdown file containing:
- Metrics (signups, errors, latency, conversion rates)
- Error logs or stack traces
- User feedback or support tickets
- Performance data
- CI/CD results

## Analysis Process

### Step 1: Extract Actionable Items

For each issue found, capture:
- **What:** The specific problem
- **Evidence:** Data or quotes supporting it
- **Impact:** Who/what is affected and how badly
- **Effort:** Rough estimate (trivial/small/medium/large)

### Step 2: Rank by Impact

Priority tiers:
1. **Revenue-blocking** — checkout errors, payment failures, signup broken
2. **User-facing errors** — crashes, data loss, broken features
3. **Performance** — slow pages, high latency, resource exhaustion
4. **User experience** — confusing UI, missing features, accessibility
5. **Code quality** — test failures, type errors, security warnings
6. **Cosmetic** — styling issues, copy changes

### Step 3: Apply Deduplication

Skip items that were already fixed recently (check recent commits, MEMORY.md, or daily logs for related fixes in the last 7 days).

### Step 4: Select #1 Priority

Choose the item with highest impact that is feasible to fix. Prefer items where:
- Impact is clearly measurable
- Fix is well-defined (not exploratory)
- Effort is proportional to impact

## Output Format

```markdown
## Analysis Summary

**Report:** [filename]
**Date:** [date]
**Items found:** [count]

## Priority #1: [Title]

**Category:** [Revenue/Error/Performance/UX/Quality/Cosmetic]
**Impact:** [Description of who/what is affected]
**Evidence:** [Specific data points or quotes]
**Proposed fix:** [Concrete description of what to change]
**Effort estimate:** [Trivial/Small/Medium/Large]

## Other Notable Items

1. [Item 2 — brief description + impact]
2. [Item 3 — brief description + impact]
3. [Item 4 — brief description + impact]
```

## Integration with IronClaw

After analysis, the priority item can be:

1. **Created as a task:** Use `task_create` to add to the task graph
2. **Converted to a PRD:** Use the `prd-generator` skill for complex items
3. **Broken into tasks:** Use the `task-breakdown` skill for multi-step fixes
4. **Implemented directly:** For trivial/small items, fix immediately

## Example

Given a report:
```
# Daily Report - 2026-02-13
## Errors
- 47 TypeErrors in /api/checkout (up 300% from yesterday)
- 3 timeouts in /api/search (avg 12s response time)
## Feedback
- "Can't find the settings page" (5 users)
- "Dark mode text is unreadable" (2 users)
```

Output:
```
## Priority #1: Fix checkout TypeErrors

**Category:** Revenue-blocking
**Impact:** 47 errors/day, likely blocking purchases, 300% increase suggests recent regression
**Evidence:** TypeError count in /api/checkout up 300% from yesterday
**Proposed fix:** Check recent commits to /api/checkout for breaking change, fix type mismatch
**Effort estimate:** Small (likely a recent regression with a clear fix)
```
