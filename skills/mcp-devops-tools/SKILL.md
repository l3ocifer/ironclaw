---
name: mcp-devops-tools
description: Comprehensive DevOps toolkit for infrastructure management, database operations, homelab services, smart home control, and office automation. Provides 25+ tools for Docker, Kubernetes, PostgreSQL, MongoDB, Home Assistant, Alpaca trading, and more.
---

# MCP DevOps Tools

High-performance Model Context Protocol (MCP) implementation providing extensive DevOps capabilities for AI assistants.

## Activation

Use when:
- User asks to "manage Docker containers" or "check container status"
- User requests "Kubernetes pod management" or "get pod logs"
- User says "query database" or "list tables"
- User asks about "homelab services" or "check service health"
- User requests "create presentation/document/spreadsheet"
- User asks to "control smart home" or "turn on/off lights"
- User says "check stock prices" or "place trade order"
- User requests "deep research on [topic]"
- User asks to "store memory" or "search knowledge graph"

## Tool Categories

### Infrastructure Tools
```
list_docker_containers  - List Docker containers with status
get_container_logs      - Fetch container logs
list_k8s_pods           - List Kubernetes pods
get_pod_logs            - Get pod logs
```

### Database Tools
```
list_databases          - List available databases (PostgreSQL/MongoDB/Supabase)
execute_query           - Execute database queries
list_tables             - List tables in a database
```

### Office Automation
```
create_presentation     - Create PowerPoint presentations
create_document         - Create Word documents
create_workbook         - Create Excel workbooks
```

### Memory & Knowledge Graph
```
create_memory           - Store memories (project/decision/meeting/task/knowledge)
search_memory           - Search stored memories
store_llm_response      - Store LLM responses for reference
```

### Smart Home (Home Assistant)
```
ha_turn_on              - Turn on devices (with brightness/color)
ha_turn_off             - Turn off devices
ha_set_temperature      - Set climate control temperature
```

### Finance (Alpaca Trading)
```
get_account_info        - Get trading account information
get_stock_quote         - Get real-time stock quotes
place_order             - Place stock orders
```

### Homelab Services
```
traefik_list_services   - List Traefik routes and services
traefik_service_health  - Check Traefik service health
prometheus_query        - Query Prometheus metrics
grafana_dashboards      - List Grafana dashboards
service_health_check    - Check homelab service health
coolify_deployments     - List Coolify deployments
n8n_workflows           - List N8N workflows
uptime_monitors         - Check Uptime Kuma monitors
authelia_users          - Manage Authelia authentication
vaultwarden_status      - Check Vaultwarden status
vector_logs             - Query Vector log pipeline
```

### Research & Maps
```
deep_research           - Conduct deep research on topics
query_overpass          - Query OpenStreetMap data
find_places             - Find places near location
search_grants           - Search government grants
```

### Core System
```
health_check            - Check system health
security_validate       - Validate input security
```

## Instructions

1. **Identify Tool Category**: Determine which category matches the user's request
2. **Check Required Parameters**: Each tool has specific required parameters
3. **Execute Tool**: Call the appropriate MCP tool with parameters
4. **Handle Response**: Format and present results to user

## Tool Schemas

### Infrastructure Example

```json
{
  "name": "list_docker_containers",
  "description": "List all Docker containers with their status",
  "inputSchema": {
    "type": "object",
    "properties": {
      "all": {
        "type": "boolean",
        "description": "Include stopped containers",
        "default": false
      }
    }
  }
}
```

### Database Example

```json
{
  "name": "execute_query",
  "description": "Execute a database query",
  "inputSchema": {
    "type": "object",
    "properties": {
      "provider": {
        "type": "string",
        "enum": ["postgresql", "mongodb", "supabase"],
        "description": "Database provider"
      },
      "database": {
        "type": "string",
        "description": "Database name"
      },
      "query": {
        "type": "string",
        "description": "Query to execute"
      }
    },
    "required": ["provider", "database", "query"]
  }
}
```

### Smart Home Example

```json
{
  "name": "ha_turn_on",
  "description": "Turn on a Home Assistant device",
  "inputSchema": {
    "type": "object",
    "properties": {
      "entity_id": {
        "type": "string",
        "description": "Entity ID of the device"
      },
      "brightness": {
        "type": "integer",
        "description": "Brightness level (0-255)"
      },
      "color": {
        "type": "string",
        "description": "Color name or hex code"
      }
    },
    "required": ["entity_id"]
  }
}
```

### Memory Example

```json
{
  "name": "create_memory",
  "description": "Create a new memory in the knowledge graph",
  "inputSchema": {
    "type": "object",
    "properties": {
      "memory_type": {
        "type": "string",
        "enum": ["project", "decision", "meeting", "task", "knowledge"],
        "description": "Type of memory to store"
      },
      "title": {
        "type": "string",
        "description": "Memory title"
      },
      "content": {
        "type": "string",
        "description": "Memory content"
      },
      "tags": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Tags for categorization"
      }
    },
    "required": ["memory_type", "title", "content"]
  }
}
```

## Usage Examples

### Check Docker Containers

```bash
# Request
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "list_docker_containers", "arguments": {"all": true}}}

# Response
🐳 Docker Containers (all containers)

📋 Found containers from your homelab:
• neon-postgres-leopaska (running)
• redis-nd-leopaska (running)
• homeassistant-leopaska (running)
• grafana-leopaska (running)
• n8n-leopaska (running)

✅ Total containers: 20+
```

### Query Prometheus Metrics

```bash
# Request
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "prometheus_query", "arguments": {"query": "up", "prometheus_url": "http://localhost:9090"}}}

# Response
📊 Prometheus Query

Server: http://localhost:9090
Query: up

📈 Results:
• homeassistant-leopaska: up=1 (healthy)
• grafana-leopaska: up=1 (healthy)
• prometheus-leopaska: up=1 (healthy)
• traefik: up=1 (healthy)

📋 Metrics Summary:
• Total Targets: 6
• Up: 6
• Down: 0
```

### Create Memory

```bash
# Request
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "create_memory", "arguments": {"memory_type": "decision", "title": "Use Rust for MCP", "content": "Decided to use Rust for MCP implementation due to performance requirements", "tags": ["rust", "mcp", "architecture"]}}}

# Response
🧠 Memory Created

Type: decision
Title: "Use Rust for MCP"
Content: Decided to use Rust for MCP implementation...
Timestamp: 1702732800

✅ Memory stored in knowledge graph
```

### Control Smart Home

```bash
# Request
{"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "ha_turn_on", "arguments": {"entity_id": "light.living_room", "brightness": 200, "color": "warm_white"}}}

# Response
🏠 Home Assistant: Turn On

Entity: light.living_room
Brightness: 200
Color: warm_white

✅ Command sent to Home Assistant
```

## Architecture

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│   Client    │───▶│ Stdio Bridge │───▶│ MCP Server  │
│  (Claude)   │    │   (Node.js)  │    │   (Rust)    │
└─────────────┘    └──────────────┘    └─────────────┘
                           │                    │
                           │                    ▼
                           │            ┌─────────────┐
                           │            │   Modules   │
                           │            │ • DevOps    │
                           │            │ • Cloud     │
                           │            │ • Security  │
                           │            │ • Office    │
                           │            │ • Homelab   │
                           │            │ • Finance   │
                           │            │ • AI/Memory │
                           │            └─────────────┘
```

## Configuration

### Environment Variables

```bash
MCP_HTTP_HOST=127.0.0.1
MCP_HTTP_PORT=8890
```

### Service Endpoints

| Service | Default URL |
|---------|-------------|
| Traefik | http://localhost:8080 |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:3000 |
| Coolify | http://localhost:8000 |
| N8N | http://localhost:5678 |
| Uptime Kuma | http://localhost:3001 |
| Authelia | http://localhost:9091 |
| Vector | http://localhost:8686 |

## Performance

- **Sub-millisecond response times** for most operations
- **Zero-copy optimizations** for data handling
- **Async/await** throughout for maximum concurrency
- **Resource pooling** for database connections
- **Pre-allocated data structures** for efficiency

## Security

- **Rate limiting** prevents abuse
- **Input validation** sanitizes all inputs
- **Memory protection** with secure zeroization
- **XSS/SQL injection detection** built-in
- **Path traversal prevention** enforced

## Extended Thinking Integration

For complex problems, use structured thinking:

```rust
pub enum ThinkStrategy {
    Deep,        // Deep, thorough analysis
    Exploratory, // Explore multiple approaches
    Analytical,  // Structured analytical thinking
}
```

The thinking module improves performance by up to 54% on difficult tasks.

## Best Practices

1. **Check service health** before executing operations
2. **Use appropriate tool** for each task category
3. **Validate inputs** using security_validate when needed
4. **Store important decisions** in the memory system
5. **Monitor with Prometheus** for operational visibility
6. **Use homelab tools** for infrastructure management

