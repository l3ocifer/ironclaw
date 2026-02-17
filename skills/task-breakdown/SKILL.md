---
name: task-breakdown
description: Convert PRDs into 8-15 granular, machine-verifiable tasks for autonomous execution. Separates investigation from implementation.
homepage: https://github.com/snarktank/compound-product
metadata: { "openclaw": { "emoji": "🔨", "requires": { "bins": [] } } }
---

# Task Breakdown

Convert PRDs into granular, agent-executable tasks. Each task must be completable in one context window with boolean pass/fail criteria.

## The Job

1. Read the PRD file specified by the user
2. Extract high-level work items (user stories, requirements)
3. Explode into 8-15 granular tasks
4. Order by dependencies
5. Output as structured task list

## Core Rules

### Target: 8-15 Tasks Per PRD

If you have fewer than 6, split further. If more than 20, group related items.

### One Concern Per Task

Each task does ONE thing:

| Concern | Separate Task |
|---------|---------------|
| Navigate to page | T-001 |
| Check for errors | T-002 |
| Test input validation | T-003 |
| Implement fix | T-004 |
| Verify fix | T-005 |

### Investigation vs Implementation

**Never combine "find the problem" with "fix the problem"** in one task.

Investigation tasks (priority 1-3):
- Check configuration files for specific values
- Log current state to notes
- Capture screenshots or error messages

Implementation tasks (priority 4-7):
- Make targeted code changes
- Run quality checks (typecheck, tests)

Verification tasks (priority 8+):
- Verify the change works as expected
- Test edge cases

### Boolean Acceptance Criteria

Every criterion must be machine-verifiable pass/fail:

**Bad:** "Review the signup flow", "Verify it works", "Check for issues"

**Good:**
- "Navigate to /signup - page loads (status 200)"
- "Run `cargo test` - exits with code 0"
- "File `src/config.rs` contains `pub struct GeminiConfig`"
- "Email input accepts 'test@example.com' without error"

### Priority Ordering (Dependencies First)

1. Investigation/understanding tasks
2. Schema/database changes
3. Backend logic changes
4. UI component changes
5. Integration/verification tasks

## Task Sizing

**Right-sized:**
- Check one configuration file
- Test one user interaction
- Change one function or method
- Add one database migration
- Verify one specific behavior

**Too big (split these):**
- "Test the entire flow"
- "Fix the bug"
- "Add authentication"
- "Refactor the module"

## Output Format

For each task, provide:

```
### T-001: [Specific action verb] [specific target]
**Priority:** 1
**Description:** [1-2 sentences: what to do and why]
**Acceptance Criteria:**
- [ ] Specific machine-verifiable criterion
- [ ] Another criterion with expected outcome
- [ ] Quality check passes (typecheck/test)
```

If the user wants JSON output (e.g., for prd.json):

```json
{
  "id": "T-001",
  "title": "[Specific action verb] [specific target]",
  "description": "[1-2 sentences]",
  "acceptanceCriteria": ["Criterion 1", "Criterion 2"],
  "priority": 1,
  "passes": false,
  "notes": ""
}
```

## Integration with IronClaw Task Tools

After breakdown, tasks can be created using IronClaw's task tools:

```
task_create: title, description, priority, depends_on (for dependency ordering)
```

## Checklist

Before finalizing:
- [ ] 8-15 tasks generated
- [ ] Each task does ONE thing
- [ ] Investigation separated from implementation
- [ ] Every criterion is boolean pass/fail
- [ ] No vague words: "review", "identify", "verify it works"
- [ ] Priority order reflects dependencies
