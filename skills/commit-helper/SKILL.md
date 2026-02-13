---
name: commit-helper
description: Generates clear, conventional commit messages from git diffs. Use when writing commit messages or reviewing staged changes.
---

# Commit Message Generator

Automatically generates commit messages following Conventional Commits format from staged changes.

## Activation

Use when:
- User says "write a commit message"
- User asks "what should I commit with?"
- User requests "generate commit message"
- User runs `git diff --staged` and needs a message

## Instructions

1. Run `git diff --staged` to see all staged changes
2. Analyze the changes to understand:
   - Type of change (feat, fix, docs, chore, refactor, test, style, perf)
   - Scope of change (component, file, or module affected)
   - Breaking changes if any
3. Generate a commit message with:
   - **Summary line** under 50 characters
   - **Body** with detailed description (72 char lines)
   - **Breaking changes** section if applicable
   - **Affected components** list

## Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, missing semicolons)
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Build process, dependencies

### Examples

```
feat(api): add user authentication endpoint

- Implement JWT token generation
- Add bcrypt password hashing
- Create login/register routes
- Add rate limiting middleware

Closes #123
```

```
fix(ui): resolve React hydration mismatch

The server-rendered HTML didn't match client due to
Date.now() being called during SSR. Fixed by moving
timestamp generation to useEffect.

Fixes #456
```

## Best Practices

- Use present tense ("add" not "added")
- Don't capitalize first letter after colon
- No period at end of subject line
- Explain WHAT and WHY, not HOW
- Reference issue numbers when applicable
- Include breaking changes with BREAKING CHANGE: prefix

