//! User settings persistence.
//!
//! Stores user preferences in ~/.ironclaw/settings.json.
//! Settings are loaded with env var > settings.json > default priority.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// User settings persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Whether onboarding wizard has been completed.
    #[serde(default, alias = "setup_completed")]
    pub onboard_completed: bool,

    // === Step 1: Database ===
    /// Database backend: "postgres" or "libsql".
    #[serde(default)]
    pub database_backend: Option<String>,

    /// Database connection URL (postgres://...).
    #[serde(default)]
    pub database_url: Option<String>,

    /// Database pool size.
    #[serde(default)]
    pub database_pool_size: Option<usize>,

    /// Path to local libSQL database file.
    #[serde(default)]
    pub libsql_path: Option<String>,

    /// Turso cloud URL for remote replica sync.
    #[serde(default)]
    pub libsql_url: Option<String>,

    // === Step 2: Security ===
    /// Source for the secrets master key.
    #[serde(default)]
    pub secrets_master_key_source: KeySource,

    // === Step 3: Inference Provider ===
    /// LLM backend: "nearai", "anthropic", "openai", "ollama", "openai_compatible".
    #[serde(default)]
    pub llm_backend: Option<String>,

    /// Ollama base URL (when llm_backend = "ollama").
    #[serde(default)]
    pub ollama_base_url: Option<String>,

    /// OpenAI-compatible endpoint base URL (when llm_backend = "openai_compatible").
    #[serde(default)]
    pub openai_compatible_base_url: Option<String>,

    // === Step 3b: Intelligent Routing ===
    /// Routing profile: "auto", "eco", "premium", "free".
    /// When set, enables intelligent request-based model selection.
    #[serde(default)]
    pub routing_profile: Option<String>,

    /// Force agentic routing mode (auto-detects tool-heavy requests).
    #[serde(default)]
    pub routing_force_agentic: Option<bool>,

    /// Enable session pinning (reuse selected model within a session).
    #[serde(default)]
    pub routing_session_pinning: Option<bool>,

    // === Step 4: Model Selection ===
    /// Currently selected model.
    #[serde(default)]
    pub selected_model: Option<String>,

    // === Step 5: Embeddings ===
    /// Embeddings configuration.
    #[serde(default)]
    pub embeddings: EmbeddingsSettings,

    // === Step 6: Channels ===
    /// Tunnel configuration for public webhook endpoints.
    #[serde(default)]
    pub tunnel: TunnelSettings,

    /// Channel configuration.
    #[serde(default)]
    pub channels: ChannelSettings,

    // === Step 7: Heartbeat ===
    /// Heartbeat configuration.
    #[serde(default)]
    pub heartbeat: HeartbeatSettings,

    // === Advanced Settings (not asked during setup, editable via CLI) ===
    /// Agent behavior configuration.
    #[serde(default)]
    pub agent: AgentSettings,

    /// WASM sandbox configuration.
    #[serde(default)]
    pub wasm: WasmSettings,

    /// Docker sandbox configuration.
    #[serde(default)]
    pub sandbox: SandboxSettings,

    /// Safety configuration.
    #[serde(default)]
    pub safety: SafetySettings,

    /// Builder configuration.
    #[serde(default)]
    pub builder: BuilderSettings,

    /// Memory and Logseq integration.
    #[serde(default)]
    pub memory: MemorySettings,

    /// Agent identity (ERC-8004, wallet, agent card).
    #[serde(default)]
    pub identity: IdentitySettings,

    /// Skills configuration (discovery, per-skill enable/disable, compatibility).
    #[serde(default)]
    pub skills: SkillsSettings,
}

/// Agent Skills configuration.
///
/// Controls which skill directories are scanned, per-skill overrides,
/// and compatibility with Claude/Cursor skill ecosystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsSettings {
    /// Additional skill directories to scan (beyond bundled, managed, workspace).
    #[serde(default)]
    pub extra_dirs: Vec<String>,

    /// Allowlist for bundled skills. If non-empty, only listed bundled skills are loaded.
    /// Empty = all bundled skills allowed.
    #[serde(default)]
    pub allow_bundled: Vec<String>,

    /// Whether to also scan `~/.claude/skills/` for Anthropic ecosystem compatibility.
    #[serde(default = "default_true")]
    pub include_claude_skills: bool,

    /// Whether to also scan `~/.cursor/skills/` for Cursor IDE compatibility.
    #[serde(default = "default_true")]
    pub include_cursor_skills: bool,

    /// Per-skill configuration overrides.
    #[serde(default)]
    pub entries: std::collections::HashMap<String, SkillEntrySettings>,
}

impl Default for SkillsSettings {
    fn default() -> Self {
        Self {
            extra_dirs: Vec::new(),
            allow_bundled: Vec::new(),
            include_claude_skills: true,
            include_cursor_skills: true,
            entries: std::collections::HashMap::new(),
        }
    }
}

/// Per-skill configuration override.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillEntrySettings {
    /// Explicitly enable or disable this skill.
    #[serde(default)]
    pub enabled: Option<bool>,

    /// API key for the skill's primary environment variable.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Additional environment variable overrides for this skill.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// Memory flush (pre-compaction) and Logseq bootstrap.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySettings {
    /// Pre-compaction memory flush. When enabled, run a silent turn before compaction
    /// to remind the model to write durable notes to memory.
    #[serde(default)]
    pub compaction_memory_flush: Option<MemoryFlushSettings>,

    /// Logseq graph integration. When graph_path is set, inject relevant Logseq notes into MEMORY context at bootstrap.
    #[serde(default)]
    pub logseq: Option<LogseqSettings>,
}

/// Pre-compaction memory flush configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFlushSettings {
    /// Enable the pre-compaction memory flush (default: true when section present).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Soft token threshold before compaction; flush runs when context is within this many tokens of the limit.
    #[serde(default = "default_memory_flush_soft_threshold")]
    pub soft_threshold_tokens: usize,

    /// System prompt for the silent flush turn.
    #[serde(default = "default_memory_flush_system_prompt")]
    pub system_prompt: String,

    /// User prompt for the silent flush turn (e.g. "Write any lasting notes... reply with NO_REPLY if nothing to store").
    #[serde(default = "default_memory_flush_prompt")]
    pub prompt: String,
}

fn default_memory_flush_soft_threshold() -> usize {
    4000
}

fn default_memory_flush_system_prompt() -> String {
    "Session nearing compaction. Store durable memories now.".to_string()
}

fn default_memory_flush_prompt() -> String {
    "Write any lasting notes to memory (e.g. daily log or MEMORY.md); reply with NO_REPLY if nothing to store.".to_string()
}

impl Default for MemoryFlushSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            soft_threshold_tokens: default_memory_flush_soft_threshold(),
            system_prompt: default_memory_flush_system_prompt(),
            prompt: default_memory_flush_prompt(),
        }
    }
}

/// Logseq graph integration. Reads from graph path and injects into MEMORY context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogseqSettings {
    /// Path to Logseq graph (e.g. ~/Logseq/notes-sync). Resolved with tilde expansion.
    #[serde(default)]
    pub graph_path: Option<String>,

    /// AI memory namespace under graph pages (default: ai-memory).
    #[serde(default = "default_logseq_ai_namespace")]
    pub ai_namespace: String,

    /// Max characters to inject (approx 4 chars per token; default ~2000 tokens).
    #[serde(default = "default_logseq_max_tokens")]
    pub max_tokens: usize,

    /// Include shared user profile from ai_namespace/shared/ (default: true).
    #[serde(default = "default_true")]
    pub include_user_profile: bool,

    /// Include agent preferences from ai_namespace/{agent}/preferences.md (default: true).
    #[serde(default = "default_true")]
    pub include_preferences: bool,

    /// Include recent decisions from ai_namespace/{agent}/decisions.md (default: true).
    #[serde(default = "default_true")]
    pub include_decisions: bool,

    /// Include shared voice/craft directives from ai_namespace/shared/voice.md (default: true).
    #[serde(default = "default_true")]
    pub include_voice: bool,
}

fn default_logseq_ai_namespace() -> String {
    "ai-memory".to_string()
}

fn default_logseq_max_tokens() -> usize {
    2000
}

impl Default for LogseqSettings {
    fn default() -> Self {
        Self {
            graph_path: None,
            ai_namespace: default_logseq_ai_namespace(),
            max_tokens: default_logseq_max_tokens(),
            include_user_profile: true,
            include_preferences: true,
            include_decisions: true,
            include_voice: true,
        }
    }
}

/// Source for the secrets master key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    /// Auto-generated key stored in OS keychain.
    Keychain,
    /// User provides via SECRETS_MASTER_KEY env var.
    Env,
    /// Not configured (secrets features disabled).
    #[default]
    None,
}

/// Embeddings configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsSettings {
    /// Whether embeddings are enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Provider to use: "openai" or "nearai".
    #[serde(default = "default_embeddings_provider")]
    pub provider: String,

    /// Model to use for embeddings.
    #[serde(default = "default_embeddings_model")]
    pub model: String,
}

fn default_embeddings_provider() -> String {
    "nearai".to_string()
}

fn default_embeddings_model() -> String {
    "text-embedding-3-small".to_string()
}

impl Default for EmbeddingsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_embeddings_provider(),
            model: default_embeddings_model(),
        }
    }
}

/// Tunnel settings for public webhook endpoints.
///
/// The tunnel URL is shared across all channels that need webhooks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelSettings {
    /// Public URL from tunnel provider (e.g., "https://abc123.ngrok.io").
    #[serde(default)]
    pub public_url: Option<String>,
}

/// Channel-specific settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelSettings {
    /// Whether HTTP webhook channel is enabled.
    #[serde(default)]
    pub http_enabled: bool,

    /// HTTP webhook port (if enabled).
    #[serde(default)]
    pub http_port: Option<u16>,

    /// HTTP webhook host.
    #[serde(default)]
    pub http_host: Option<String>,

    /// Telegram owner user ID. When set, the bot only responds to this user.
    /// Captured during setup by having the user message the bot.
    #[serde(default)]
    pub telegram_owner_id: Option<i64>,

    /// Enabled WASM channels by name.
    /// Channels not in this list but present in the channels directory will still load.
    /// This is primarily used by the setup wizard to track which channels were configured.
    #[serde(default)]
    pub wasm_channels: Vec<String>,

    /// Whether WASM channels are enabled.
    #[serde(default = "default_true")]
    pub wasm_channels_enabled: bool,

    /// Directory containing WASM channel modules.
    #[serde(default)]
    pub wasm_channels_dir: Option<PathBuf>,
}

/// Heartbeat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatSettings {
    /// Whether heartbeat is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Interval between heartbeat checks in seconds.
    #[serde(default = "default_heartbeat_interval")]
    pub interval_secs: u64,

    /// Channel to notify on heartbeat findings.
    #[serde(default)]
    pub notify_channel: Option<String>,

    /// User ID to notify on heartbeat findings.
    #[serde(default)]
    pub notify_user: Option<String>,
}

fn default_heartbeat_interval() -> u64 {
    1800 // 30 minutes
}

impl Default for HeartbeatSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: default_heartbeat_interval(),
            notify_channel: None,
            notify_user: None,
        }
    }
}

/// Agent behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    /// Agent name.
    #[serde(default = "default_agent_name")]
    pub name: String,

    /// Maximum parallel jobs.
    #[serde(default = "default_max_parallel_jobs")]
    pub max_parallel_jobs: u32,

    /// Job timeout in seconds.
    #[serde(default = "default_job_timeout")]
    pub job_timeout_secs: u64,

    /// Stuck job threshold in seconds.
    #[serde(default = "default_stuck_threshold")]
    pub stuck_threshold_secs: u64,

    /// Whether to use planning before tool execution.
    #[serde(default = "default_true")]
    pub use_planning: bool,

    /// Self-repair check interval in seconds.
    #[serde(default = "default_repair_interval")]
    pub repair_check_interval_secs: u64,

    /// Maximum repair attempts.
    #[serde(default = "default_max_repair_attempts")]
    pub max_repair_attempts: u32,

    /// Session idle timeout in seconds (default: 7 days). Sessions inactive
    /// longer than this are pruned from memory.
    #[serde(default = "default_session_idle_timeout")]
    pub session_idle_timeout_secs: u64,

    /// Daily session reset hour (0-23, local time). Sessions older than this
    /// hour boundary auto-reset. None = disabled.
    #[serde(default)]
    pub daily_reset_hour: Option<u8>,

    /// Reserve tokens floor for compaction. When set, compaction triggers when
    /// remaining tokens drop below this floor (instead of the default ratio).
    #[serde(default)]
    pub compaction_reserve_tokens_floor: Option<usize>,
}

fn default_agent_name() -> String {
    "ironclaw".to_string()
}

fn default_max_parallel_jobs() -> u32 {
    5
}

fn default_job_timeout() -> u64 {
    3600 // 1 hour
}

fn default_stuck_threshold() -> u64 {
    300 // 5 minutes
}

fn default_repair_interval() -> u64 {
    60 // 1 minute
}

fn default_session_idle_timeout() -> u64 {
    7 * 24 * 3600 // 7 days
}

fn default_max_repair_attempts() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            name: default_agent_name(),
            max_parallel_jobs: default_max_parallel_jobs(),
            job_timeout_secs: default_job_timeout(),
            stuck_threshold_secs: default_stuck_threshold(),
            use_planning: true,
            repair_check_interval_secs: default_repair_interval(),
            max_repair_attempts: default_max_repair_attempts(),
            session_idle_timeout_secs: default_session_idle_timeout(),
            daily_reset_hour: None,
            compaction_reserve_tokens_floor: None,
        }
    }
}

/// WASM sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSettings {
    /// Whether WASM tool execution is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Directory containing installed WASM tools.
    #[serde(default)]
    pub tools_dir: Option<PathBuf>,

    /// Default memory limit in bytes.
    #[serde(default = "default_wasm_memory_limit")]
    pub default_memory_limit: u64,

    /// Default execution timeout in seconds.
    #[serde(default = "default_wasm_timeout")]
    pub default_timeout_secs: u64,

    /// Default fuel limit for CPU metering.
    #[serde(default = "default_wasm_fuel_limit")]
    pub default_fuel_limit: u64,

    /// Whether to cache compiled modules.
    #[serde(default = "default_true")]
    pub cache_compiled: bool,

    /// Directory for compiled module cache.
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
}

fn default_wasm_memory_limit() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

fn default_wasm_timeout() -> u64 {
    60
}

fn default_wasm_fuel_limit() -> u64 {
    10_000_000
}

impl Default for WasmSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tools_dir: None,
            default_memory_limit: default_wasm_memory_limit(),
            default_timeout_secs: default_wasm_timeout(),
            default_fuel_limit: default_wasm_fuel_limit(),
            cache_compiled: true,
            cache_dir: None,
        }
    }
}

/// Docker sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSettings {
    /// Whether the Docker sandbox is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Sandbox policy: "readonly", "workspace_write", or "full_access".
    #[serde(default = "default_sandbox_policy")]
    pub policy: String,

    /// Command timeout in seconds.
    #[serde(default = "default_sandbox_timeout")]
    pub timeout_secs: u64,

    /// Memory limit in megabytes.
    #[serde(default = "default_sandbox_memory")]
    pub memory_limit_mb: u64,

    /// CPU shares (relative weight).
    #[serde(default = "default_sandbox_cpu_shares")]
    pub cpu_shares: u32,

    /// Docker image for the sandbox.
    #[serde(default = "default_sandbox_image")]
    pub image: String,

    /// Whether to auto-pull the image if not found.
    #[serde(default = "default_true")]
    pub auto_pull_image: bool,

    /// Additional domains to allow through the network proxy.
    #[serde(default)]
    pub extra_allowed_domains: Vec<String>,
}

fn default_sandbox_policy() -> String {
    "readonly".to_string()
}

fn default_sandbox_timeout() -> u64 {
    120
}

fn default_sandbox_memory() -> u64 {
    2048
}

fn default_sandbox_cpu_shares() -> u32 {
    1024
}

fn default_sandbox_image() -> String {
    "ghcr.io/nearai/sandbox:latest".to_string()
}

impl Default for SandboxSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: default_sandbox_policy(),
            timeout_secs: default_sandbox_timeout(),
            memory_limit_mb: default_sandbox_memory(),
            cpu_shares: default_sandbox_cpu_shares(),
            image: default_sandbox_image(),
            auto_pull_image: true,
            extra_allowed_domains: Vec::new(),
        }
    }
}

/// Safety configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySettings {
    /// Maximum output length in bytes.
    #[serde(default = "default_max_output_length")]
    pub max_output_length: usize,

    /// Whether injection check is enabled.
    #[serde(default = "default_true")]
    pub injection_check_enabled: bool,

    /// Command guard configuration.
    #[serde(default)]
    pub command_guard: CommandGuardSettings,

    /// Workspace integrity monitoring.
    #[serde(default)]
    pub integrity: IntegritySettings,
}

/// Command guard (destructive command blocking) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandGuardSettings {
    /// Whether the command guard is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Fail mode: "open" (allow on error) or "closed" (block on error).
    #[serde(default = "default_fail_mode")]
    pub fail_mode: String,
}

fn default_fail_mode() -> String {
    "open".to_string()
}

impl Default for CommandGuardSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_mode: default_fail_mode(),
        }
    }
}

/// Workspace integrity monitoring settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegritySettings {
    /// Whether integrity monitoring is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether to auto-restore files in Restore mode (default: true).
    #[serde(default = "default_true")]
    pub auto_restore: bool,

    /// Check interval in heartbeat cycles (default: 1 — every heartbeat).
    #[serde(default = "default_integrity_interval")]
    pub check_interval: u64,
}

fn default_integrity_interval() -> u64 {
    1
}

impl Default for IntegritySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_restore: true,
            check_interval: default_integrity_interval(),
        }
    }
}

/// Agent identity configuration (ERC-8004).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySettings {
    /// Agent display name (used in agent card). Falls back to agent.name if not set.
    #[serde(default)]
    pub agent_name: Option<String>,

    /// Source for the Ethereum keypair used for on-chain identity.
    #[serde(default)]
    pub ethereum_key_source: KeySource,

    /// ERC-8004 network for on-chain registration (e.g., "ethereum_mainnet", "base", "sepolia").
    /// None = local identity only, no on-chain registration.
    #[serde(default)]
    pub erc8004_network: Option<String>,

    /// ERC-8004 agent ID (token ID) after on-chain registration. None = not registered.
    #[serde(default)]
    pub erc8004_agent_id: Option<u64>,

    /// Service endpoints advertised in the agent card.
    #[serde(default)]
    pub services: Vec<ServiceEndpointSettings>,

    /// Agent description for the registration file.
    #[serde(default)]
    pub description: Option<String>,

    /// Agent image URL for the registration file.
    #[serde(default)]
    pub image_url: Option<String>,

    /// Whether to serve /.well-known/agent-card.json from the gateway.
    #[serde(default = "default_true")]
    pub serve_agent_card: bool,
}

/// A service endpoint in the agent card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpointSettings {
    /// Service name (e.g., "MCP", "A2A", "web").
    pub name: String,
    /// Endpoint URL.
    pub endpoint: String,
    /// Protocol version (optional).
    #[serde(default)]
    pub version: Option<String>,
}

impl Default for IdentitySettings {
    fn default() -> Self {
        Self {
            agent_name: None,
            ethereum_key_source: KeySource::None,
            erc8004_network: None,
            erc8004_agent_id: None,
            services: Vec::new(),
            description: None,
            image_url: None,
            serve_agent_card: true,
        }
    }
}

fn default_max_output_length() -> usize {
    100_000
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            max_output_length: default_max_output_length(),
            injection_check_enabled: true,
            command_guard: CommandGuardSettings::default(),
            integrity: IntegritySettings::default(),
        }
    }
}

/// Builder configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderSettings {
    /// Whether the software builder tool is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Directory for build artifacts.
    #[serde(default)]
    pub build_dir: Option<PathBuf>,

    /// Maximum iterations for the build loop.
    #[serde(default = "default_builder_max_iterations")]
    pub max_iterations: u32,

    /// Build timeout in seconds.
    #[serde(default = "default_builder_timeout")]
    pub timeout_secs: u64,

    /// Whether to automatically register built WASM tools.
    #[serde(default = "default_true")]
    pub auto_register: bool,
}

fn default_builder_max_iterations() -> u32 {
    20
}

fn default_builder_timeout() -> u64 {
    600
}

impl Default for BuilderSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            build_dir: None,
            max_iterations: default_builder_max_iterations(),
            timeout_secs: default_builder_timeout(),
            auto_register: true,
        }
    }
}

impl Settings {
    /// Reconstruct Settings from a flat key-value map (as stored in the DB).
    ///
    /// Each key is a dotted path (e.g., "agent.name"), value is a JSONB value.
    /// Missing keys get their default value.
    pub fn from_db_map(map: &std::collections::HashMap<String, serde_json::Value>) -> Self {
        // Start with defaults, then overlay each DB setting.
        //
        // The settings table stores both Settings struct fields and app-specific
        // data (e.g. nearai.session_token). Skip keys that don't correspond to
        // a known Settings path.
        let mut settings = Self::default();

        for (key, value) in map {
            // Convert the JSONB value to a string for the existing set() method
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Null => continue, // null means default, skip
                other => other.to_string(),
            };

            match settings.set(key, &value_str) {
                Ok(()) => {}
                // The settings table stores both Settings fields and app-specific
                // data (e.g. nearai.session_token). Silently skip unknown paths.
                Err(e) if e.starts_with("Path not found") => {}
                Err(e) => {
                    tracing::warn!(
                        "Failed to apply DB setting '{}' = '{}': {}",
                        key,
                        value_str,
                        e
                    );
                }
            }
        }

        settings
    }

    /// Flatten Settings into a key-value map suitable for DB storage.
    ///
    /// Each entry is a (dotted_path, JSONB value) pair.
    pub fn to_db_map(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let json = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return std::collections::HashMap::new(),
        };

        let mut map = std::collections::HashMap::new();
        collect_settings_json(&json, String::new(), &mut map);
        map
    }

    /// Get the default settings file path (~/.ironclaw/settings.json).
    pub fn default_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".ironclaw")
            .join("settings.json")
    }

    /// Load settings from disk, returning default if not found.
    pub fn load() -> Self {
        Self::load_from(&Self::default_path())
    }

    /// Load settings from a specific path (used by bootstrap legacy migration).
    pub fn load_from(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Get a setting value by dotted path (e.g., "agent.max_parallel_jobs").
    pub fn get(&self, path: &str) -> Option<String> {
        let json = serde_json::to_value(self).ok()?;
        let mut current = &json;

        for part in path.split('.') {
            current = current.get(part)?;
        }

        match current {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            serde_json::Value::Null => Some("null".to_string()),
            serde_json::Value::Array(arr) => Some(serde_json::to_string(arr).unwrap_or_default()),
            serde_json::Value::Object(obj) => Some(serde_json::to_string(obj).unwrap_or_default()),
        }
    }

    /// Set a setting value by dotted path.
    ///
    /// Returns error if path is invalid or value cannot be parsed.
    pub fn set(&mut self, path: &str, value: &str) -> Result<(), String> {
        let mut json = serde_json::to_value(&self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }

        // Navigate to parent and set the final key
        let mut current = &mut json;
        for part in &parts[..parts.len() - 1] {
            current = current
                .get_mut(*part)
                .ok_or_else(|| format!("Path not found: {}", path))?;
        }

        let final_key = parts.last().unwrap();
        let obj = current
            .as_object_mut()
            .ok_or_else(|| format!("Parent is not an object: {}", path))?;

        // Try to infer the type from the existing value
        let new_value = if let Some(existing) = obj.get(*final_key) {
            match existing {
                serde_json::Value::Bool(_) => {
                    let b = value
                        .parse::<bool>()
                        .map_err(|_| format!("Expected boolean for {}, got '{}'", path, value))?;
                    serde_json::Value::Bool(b)
                }
                serde_json::Value::Number(n) => {
                    if n.is_u64() {
                        let n = value.parse::<u64>().map_err(|_| {
                            format!("Expected integer for {}, got '{}'", path, value)
                        })?;
                        serde_json::Value::Number(n.into())
                    } else if n.is_i64() {
                        let n = value.parse::<i64>().map_err(|_| {
                            format!("Expected integer for {}, got '{}'", path, value)
                        })?;
                        serde_json::Value::Number(n.into())
                    } else {
                        let n = value.parse::<f64>().map_err(|_| {
                            format!("Expected number for {}, got '{}'", path, value)
                        })?;
                        serde_json::Number::from_f64(n)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::String(value.to_string()))
                    }
                }
                serde_json::Value::Null => {
                    // Could be Option<T>, try to parse as JSON or use string
                    serde_json::from_str(value)
                        .unwrap_or(serde_json::Value::String(value.to_string()))
                }
                serde_json::Value::Array(_) => serde_json::from_str(value)
                    .map_err(|e| format!("Invalid JSON array for {}: {}", path, e))?,
                serde_json::Value::Object(_) => serde_json::from_str(value)
                    .map_err(|e| format!("Invalid JSON object for {}: {}", path, e))?,
                serde_json::Value::String(_) => serde_json::Value::String(value.to_string()),
            }
        } else {
            // Key doesn't exist, try to parse as JSON or use string
            serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.to_string()))
        };

        obj.insert((*final_key).to_string(), new_value);

        // Deserialize back to Settings
        *self =
            serde_json::from_value(json).map_err(|e| format!("Failed to apply setting: {}", e))?;

        Ok(())
    }

    /// Reset a setting to its default value.
    pub fn reset(&mut self, path: &str) -> Result<(), String> {
        let default = Self::default();
        let default_value = default
            .get(path)
            .ok_or_else(|| format!("Unknown setting: {}", path))?;

        self.set(path, &default_value)
    }

    /// List all settings as (path, value) pairs.
    pub fn list(&self) -> Vec<(String, String)> {
        let json = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        collect_settings(&json, String::new(), &mut results);
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }
}

/// Recursively collect settings paths with their JSON values (for DB storage).
fn collect_settings_json(
    value: &serde_json::Value,
    prefix: String,
    results: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                collect_settings_json(val, path, results);
            }
        }
        other => {
            results.insert(prefix, other.clone());
        }
    }
}

/// Recursively collect settings paths and values.
fn collect_settings(
    value: &serde_json::Value,
    prefix: String,
    results: &mut Vec<(String, String)>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                collect_settings(val, path, results);
            }
        }
        serde_json::Value::Array(arr) => {
            let display = serde_json::to_string(arr).unwrap_or_default();
            results.push((prefix, display));
        }
        serde_json::Value::String(s) => {
            results.push((prefix, s.clone()));
        }
        serde_json::Value::Number(n) => {
            results.push((prefix, n.to_string()));
        }
        serde_json::Value::Bool(b) => {
            results.push((prefix, b.to_string()));
        }
        serde_json::Value::Null => {
            results.push((prefix, "null".to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_map_round_trip() {
        let settings = Settings {
            selected_model: Some("claude-3-5-sonnet-20241022".to_string()),
            ..Default::default()
        };

        let map = settings.to_db_map();
        let restored = Settings::from_db_map(&map);
        assert_eq!(
            restored.selected_model,
            Some("claude-3-5-sonnet-20241022".to_string())
        );
    }

    #[test]
    fn test_get_setting() {
        let settings = Settings::default();

        assert_eq!(settings.get("agent.name"), Some("ironclaw".to_string()));
        assert_eq!(
            settings.get("agent.max_parallel_jobs"),
            Some("5".to_string())
        );
        assert_eq!(settings.get("heartbeat.enabled"), Some("false".to_string()));
        assert_eq!(settings.get("nonexistent"), None);
    }

    #[test]
    fn test_set_setting() {
        let mut settings = Settings::default();

        settings.set("agent.name", "mybot").unwrap();
        assert_eq!(settings.agent.name, "mybot");

        settings.set("agent.max_parallel_jobs", "10").unwrap();
        assert_eq!(settings.agent.max_parallel_jobs, 10);

        settings.set("heartbeat.enabled", "true").unwrap();
        assert!(settings.heartbeat.enabled);
    }

    #[test]
    fn test_reset_setting() {
        let mut settings = Settings::default();

        settings.agent.name = "custom".to_string();
        settings.reset("agent.name").unwrap();
        assert_eq!(settings.agent.name, "ironclaw");
    }

    #[test]
    fn test_list_settings() {
        let settings = Settings::default();
        let list = settings.list();

        // Check some expected entries
        assert!(list.iter().any(|(k, _)| k == "agent.name"));
        assert!(list.iter().any(|(k, _)| k == "heartbeat.enabled"));
        assert!(list.iter().any(|(k, _)| k == "onboard_completed"));
    }

    #[test]
    fn test_key_source_serialization() {
        let settings = Settings {
            secrets_master_key_source: KeySource::Keychain,
            ..Default::default()
        };

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"keychain\""));

        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.secrets_master_key_source, KeySource::Keychain);
    }

    #[test]
    fn test_embeddings_defaults() {
        let settings = Settings::default();
        assert!(!settings.embeddings.enabled);
        assert_eq!(settings.embeddings.provider, "nearai");
        assert_eq!(settings.embeddings.model, "text-embedding-3-small");
    }

    #[test]
    fn test_telegram_owner_id_db_round_trip() {
        let mut settings = Settings::default();
        settings.channels.telegram_owner_id = Some(123456789);

        let map = settings.to_db_map();
        let restored = Settings::from_db_map(&map);
        assert_eq!(restored.channels.telegram_owner_id, Some(123456789));
    }

    #[test]
    fn test_telegram_owner_id_default_none() {
        let settings = Settings::default();
        assert_eq!(settings.channels.telegram_owner_id, None);
    }

    #[test]
    fn test_telegram_owner_id_via_set() {
        let mut settings = Settings::default();
        settings
            .set("channels.telegram_owner_id", "987654321")
            .unwrap();
        assert_eq!(settings.channels.telegram_owner_id, Some(987654321));
    }

    #[test]
    fn test_llm_backend_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let settings = Settings {
            llm_backend: Some("anthropic".to_string()),
            ollama_base_url: Some("http://localhost:11434".to_string()),
            openai_compatible_base_url: Some("http://my-vllm:8000/v1".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        std::fs::write(&path, json).unwrap();

        let loaded = Settings::load_from(&path);
        assert_eq!(loaded.llm_backend, Some("anthropic".to_string()));
        assert_eq!(
            loaded.ollama_base_url,
            Some("http://localhost:11434".to_string())
        );
        assert_eq!(
            loaded.openai_compatible_base_url,
            Some("http://my-vllm:8000/v1".to_string())
        );
    }
}
