---
name: repo-creator
description: Creates new GitHub repositories with proper structure and initialization. Use when creating new projects or repos.
---

# Repository Creator

Automates creation of new GitHub repositories with best practices.

## Activation

Use when:
- User says "create a new repo"
- User asks "set up a GitHub repository"
- User requests "initialize a new project"
- User wants to "create a repository for this code"

## Instructions

1. Determine repository details:
   - Name (from current directory or user input)
   - Visibility (private recommended for new projects)
   - Description
   - Initialize with README, .gitignore, LICENSE
2. Create repository using GitHub CLI
3. Set up initial structure:
   - README.md with proper structure
   - .gitignore for detected language/framework
   - LICENSE (MIT default)
   - Initial commit
4. Push to GitHub
5. Set up branch protection (optional)

## Repository Creation Process

### Step 1: Gather Information

```bash
# Current directory name as default
REPO_NAME=$(basename $(pwd) | sed 's/^\.//')

# Detect project type
if [ -f "Cargo.toml" ]; then
    PROJECT_TYPE="rust"
elif [ -f "package.json" ]; then
    PROJECT_TYPE="javascript"
elif [ -f "go.mod" ]; then
    PROJECT_TYPE="go"
elif [ -f "pyproject.toml" ] || [ -f "setup.py" ]; then
    PROJECT_TYPE="python"
else
    PROJECT_TYPE="general"
fi
```

### Step 2: Create Repository

```bash
# Create repo with gh CLI
gh repo create ${REPO_NAME} \
    --private \
    --description "Generated description based on project" \
    --clone

# Or for current directory
gh repo create ${REPO_NAME} \
    --private \
    --source=. \
    --push
```

### Step 3: Initialize Structure

**README.md Template:**

```markdown
# {Project Name}

Brief description of what this project does.

## Features

- Feature 1
- Feature 2
- Feature 3

## Installation

\`\`\`bash
# Installation commands
\`\`\`

## Usage

\`\`\`{language}
// Usage examples
\`\`\`

## Configuration

Environment variables and configuration options.

## Development

\`\`\`bash
# Development setup
\`\`\`

## Contributing

Contributions are welcome! Please open an issue or PR.

## License

{LICENSE_TYPE} - see [LICENSE](LICENSE) file for details.
```

**Language-Specific .gitignore:**

```bash
# Use GitHub's gitignore templates
curl -s "https://raw.githubusercontent.com/github/gitignore/main/{Language}.gitignore" > .gitignore
```

### Step 4: Branch Protection (Optional)

```bash
# Protect main branch
gh api repos/:owner/:repo/branches/main/protection \
    --method PUT \
    --field required_pull_request_reviews[required_approving_review_count]=1 \
    --field enforce_admins=true \
    --field required_status_checks=null
```

## Best Practices

### Repository Naming
- Use kebab-case: `my-awesome-project`
- Be descriptive but concise
- Avoid generic names

### Initial Commit
```bash
git add .
git commit -m "feat: initial commit

- Project structure
- README and documentation
- Basic configuration"
```

### Repository Description
Include:
- Primary language/framework
- Main purpose
- Key features

### Topics/Tags
Add relevant topics for discoverability:
```bash
gh repo edit --add-topic "rust,cli,devops"
```

## Example Workflow

```bash
# Create new Rust project
mkdir my-rust-cli
cd my-rust-cli
cargo init

# User says: "Create a GitHub repo for this"
# Claude Code will:
# 1. Detect it's a Rust project
# 2. Suggest repo name: "my-rust-cli"
# 3. Create private repo
# 4. Add Rust .gitignore
# 5. Create comprehensive README
# 6. Add MIT license
# 7. Make initial commit
# 8. Push to GitHub
```

## Common Scenarios

### Scenario 1: New Project from Scratch
```
User: "Create a new TypeScript library project"
1. mkdir typescript-lib && cd typescript-lib
2. npm init -y
3. Create gh repo with TypeScript template
4. Add package.json, tsconfig.json
5. Push initial structure
```

### Scenario 2: Existing Code
```
User: "Turn this into a repo"
1. Check if git initialized (git init if not)
2. Create .gitignore based on detected type
3. Create README from code analysis
4. gh repo create with --source=.
5. Push existing commits
```

### Scenario 3: Monorepo
```
User: "Create a monorepo structure"
1. Create with lerna/nx/turborepo structure
2. Add workspace configuration
3. Set up CI/CD workflows
4. Create multi-package README
```

