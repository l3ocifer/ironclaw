# IronClaw Operations Guide

## Architecture

```
┌──────────────────────┐     ┌─────────────────────────────────────────────┐
│ Frack (MacBook)      │     │ Frick (Homelab — alef K3s)                  │
│                      │     │                                             │
│ ironclaw binary      │     │ ironclaw pod (K8s Deployment)               │
│ launchd auto-start   │     │  └─ restart: Always                         │
│ Ollama (bare metal)  │     │                                             │
│  - qwen3-coder:30b   │     │ Ollama (bare metal, K8s ExternalName svc)   │
│  - deepseek-r1:70b   │     │  - qwen3-coder:30b   - deepseek-r1:70b     │
│                      │     │  - gemma3:27b        - qwen2.5-coder:32b   │
│ Logseq graph (local) │────▶│ Logseq graph (Syncthing, hostPath mount)    │
│  ~/logseq/notes-sync │ sync│  /var/syncthing/data/logseq-graph           │
│                      │     │                                             │
│ MCP: mcp.leopaska.xyz│────▶│ MCP server (K8s, ai namespace)              │
│                      │     │  mcp-server.ai.svc:8890                     │
│                      │     │                                             │
│ DB: alef:5432        │────▶│ PostgreSQL (K8s, databases namespace)        │
│                      │     │  postgres.databases.svc:5432                 │
│                      │     │  DB: ironclaw (pgvector enabled)             │
│                      │     │                                             │
│                      │     │ ArgoCD auto-deploys from GitHub on push      │
│                      │     │ Image Updater tracks ghcr.io/l3ocifer/ironclaw│
│                      │     │ Cloudflare Tunnel: ironclaw.leopaska.xyz     │
└──────────────────────┘     └─────────────────────────────────────────────┘
```

## Frack (MacBook) Setup

### Prerequisites
- Rust toolchain installed (`rustup`)
- Ollama running (`brew services start ollama`)
- Models: `ollama pull qwen3-coder:30b && ollama pull deepseek-r1:70b`
- SSH access to alef (`ssh alef`)

### Install

```bash
cd ~/git/ai/ironclaw

# 1. Configure .env (already created, update DB password)
vim .env

# 2. Run setup script (builds, installs launchd plist)
./deploy/setup-frack.sh
```

### Management

```bash
# Start
launchctl load ~/Library/LaunchAgents/com.ironclaw.frack.plist

# Stop
launchctl unload ~/Library/LaunchAgents/com.ironclaw.frack.plist

# Logs
tail -f ~/.ironclaw/frack.log
tail -f ~/.ironclaw/frack.err.log

# Rebuild after code changes
cargo build --release
launchctl unload ~/Library/LaunchAgents/com.ironclaw.frack.plist
launchctl load ~/Library/LaunchAgents/com.ironclaw.frack.plist
```

### Auto-restart
The launchd plist has `KeepAlive.SuccessfulExit = false`, meaning macOS will restart Frack if it crashes. It also has `RunAtLoad = true` so it starts on login.

## Frick (Homelab K3s) Setup

### Prerequisites
- `ironclaw` database exists in PostgreSQL (verified ✅)
- `pgvector` extension installed (verified ✅)
- `ironclaw` DB user exists (verified ✅)
- ArgoCD running (verified ✅)
- GHCR image pushed (`ghcr.io/l3ocifer/ironclaw:latest`)

### Deploy

```bash
# 1. Create sealed secret (one-time, from alef)
ssh alef
kubectl create namespace ironclaw --dry-run=client -o yaml | kubectl apply -f -
kubectl create secret generic ironclaw-secrets \
  --namespace ironclaw \
  --from-literal=DATABASE_URL='postgres://ironclaw:REAL_PASSWORD@external-postgres:5432/ironclaw' \
  --from-literal=ANTHROPIC_API_KEY='sk-ant-...' \
  --from-literal=OPENAI_API_KEY='sk-...' \
  --from-literal=GEMINI_API_KEY='...'

# 2. Push code to GitHub (ArgoCD auto-syncs)
git push origin main

# 3. Verify
kubectl get pods -n ironclaw
curl https://ironclaw.leopaska.xyz/health
```

### Auto-restart
K8s `restartPolicy: Always` ensures Frick pod restarts on crash. The Deployment also uses `strategy: Recreate` for clean restarts.

### ArgoCD Pipeline
1. Push to `main` → GitHub Actions builds Docker image → pushes to GHCR
2. ArgoCD Image Updater detects new image digest
3. ArgoCD syncs, rolling out the new pod
4. Full pipeline: ~5 min from push to deploy

## MCP Integration

Both agents connect to the DevOps MCP server:

| Agent | MCP URL | Method |
|-------|---------|--------|
| Frack | `https://mcp.leopaska.xyz` | Cloudflare Tunnel (external) |
| Frick | `http://mcp-server.ai.svc.cluster.local:8890` | K8s internal service |

MCP config: `~/.ironclaw/mcp-servers.json`

Available MCP modules (25+):
- **Infrastructure**: Docker, Kubernetes, Terraform
- **Database**: PostgreSQL, MongoDB
- **Smart Home**: Home Assistant
- **AI**: Ollama management
- **Monitoring**: Prometheus, Grafana
- **Security**: Vault, secrets management
- **Cloud**: AWS, GCP, Cloudflare
- **Collaboration**: GitHub, Slack
- **Development**: CI/CD, code analysis

## Logseq Sync

Logseq graph is synced between both machines via Syncthing:

```
MacBook (~/logseq/notes-sync)
    ↕ Syncthing (native sync)
Alef K3s (syncthing-data PVC → hostPath mount)
    → ironclaw pod (/home/ironclaw/.ironclaw/logseq, read-only)
```

AI Memory namespace structure:
```
pages/ai-memory/
├── Frack/          # Frack-specific memory
│   ├── decisions.md
│   ├── notes.md
│   └── preferences.md
├── Frick/          # Frick-specific memory
│   └── infrastructure.md
└── shared/         # Shared between agents
    ├── Leo.md      # User profile
    ├── index/
    ├── projects/
    └── tech/
```

## Database

PostgreSQL on alef K3s (`databases` namespace):

```bash
# Connect to ironclaw DB
ssh alef "kubectl exec -n databases deploy/postgres -- psql -U ironclaw -d ironclaw"

# Run migrations
ssh alef "kubectl exec -n ironclaw deploy/ironclaw -- ironclaw migrate"

# Check tables
ssh alef "kubectl exec -n databases deploy/postgres -- psql -U postgres -d ironclaw -c '\\dt'"
```

## Monitoring

```bash
# Frick pod status
ssh alef "kubectl get pods -n ironclaw"
ssh alef "kubectl logs -n ironclaw deploy/ironclaw --tail=100"

# Frack status
launchctl list | grep ironclaw
tail -50 ~/.ironclaw/frack.log

# MCP server
ssh alef "kubectl logs -n ai deploy/mcp-server --tail=50"

# Ollama (alef)
ssh alef "systemctl status ollama"

# Ollama (MacBook)
brew services info ollama
```

## Troubleshooting

### Frack won't start
1. Check logs: `cat ~/.ironclaw/frack.err.log`
2. Check binary exists: `ls -la target/release/ironclaw`
3. Check Ollama: `curl http://localhost:11434/api/tags`
4. Test manually: `cd ~/git/ai/ironclaw && ./target/release/ironclaw`

### Frick pod CrashLoopBackOff
1. Check logs: `ssh alef "kubectl logs -n ironclaw deploy/ironclaw"`
2. Check secrets: `ssh alef "kubectl get secret -n ironclaw"`
3. Check DB connectivity: `ssh alef "kubectl exec -n ironclaw deploy/ironclaw -- pg_isready -h external-postgres -p 5432"`
4. Check Ollama: `ssh alef "kubectl exec -n ironclaw deploy/ironclaw -- curl http://external-ollama:11434/api/tags"`

### Logseq not syncing
1. Check Syncthing: `ssh alef "kubectl get pods -n storage"`
2. Verify data: `ssh alef "kubectl exec -n storage deploy/syncthing -- ls /var/syncthing/data/logseq-graph/pages/ai-memory/"`
3. Check MacBook Syncthing: Open Syncthing UI at http://localhost:8384
