---
name: prd-generator
description: Generate structured Product Requirements Documents (PRDs) from feature descriptions. Use when planning features, starting projects, or asked to create a PRD.
homepage: https://github.com/snarktank/ralph
metadata: { "openclaw": { "emoji": "📋", "requires": { "bins": [] } } }
---

# PRD Generator

Create detailed, actionable PRDs suitable for autonomous agent execution.

## The Job

1. Receive a feature description
2. Ask 3-5 clarifying questions (with lettered options for quick "1A, 2C, 3B" responses)
3. Generate a structured PRD
4. Save to `tasks/prd-[feature-name].md`

**Do NOT implement.** Just create the PRD.

## Clarifying Questions

Ask only where the prompt is ambiguous. Focus on:

- **Problem/Goal:** What problem does this solve?
- **Core Functionality:** What are the key actions?
- **Scope/Boundaries:** What should it NOT do?
- **Success Criteria:** How do we know it's done?

Format with lettered options:

```
1. What is the primary goal?
   A. Improve onboarding
   B. Increase retention
   C. Reduce support burden
   D. Other: [please specify]
```

## PRD Structure

### 1. Introduction/Overview
Brief description of the feature and the problem it solves.

### 2. Goals
Specific, measurable objectives (bullet list).

### 3. User Stories
Each story needs:
- **Title:** Short descriptive name
- **Description:** "As a [user], I want [feature] so that [benefit]"
- **Acceptance Criteria:** Verifiable boolean checklist

Each story must be completable in one focused session (one context window).

**Critical:** Acceptance criteria must be verifiable, not vague. "Works correctly" is bad. "Button shows confirmation dialog before deleting" is good.

### 4. Functional Requirements
Numbered: "FR-1: The system must allow users to..."

### 5. Non-Goals (Out of Scope)
What this feature will NOT include.

### 6. Technical Considerations (Optional)
Known constraints, dependencies, integration points.

### 7. Success Metrics
How will success be measured?

### 8. Open Questions
Remaining areas needing clarification.

## Output

- **Format:** Markdown
- **Location:** `tasks/`
- **Filename:** `prd-[feature-name].md` (kebab-case)

## Story Sizing Rule

If you cannot describe the change in 2-3 sentences, it is too big. Split it.

**Right-sized:** Add a database column, add a UI component, update a server action, add a filter dropdown.

**Too big:** "Build the entire dashboard", "Add authentication", "Refactor the API".

## Checklist

Before saving:
- [ ] Asked clarifying questions with lettered options
- [ ] User stories are small and specific
- [ ] Functional requirements are numbered and unambiguous
- [ ] Non-goals section defines clear boundaries
- [ ] Saved to `tasks/prd-[feature-name].md`
