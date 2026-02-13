//! Agent card builder for ERC-8004 registration files.
//!
//! Constructs the JSON agent card that is served at
//! `/.well-known/agent-card.json` and can later be published on-chain
//! via the ERC-8004 Identity Registry.

use serde::{Deserialize, Serialize};

use crate::config::IdentityConfig;

/// ERC-8004 agent registration file.
///
/// This matches the schema defined in the ERC-8004 specification:
/// <https://eips.ethereum.org/EIPS/eip-8004>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationFile {
    /// Schema type identifier.
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Agent display name.
    pub name: String,

    /// Natural language description of the agent.
    pub description: String,

    /// Agent image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Service endpoints (MCP, A2A, web, etc.).
    pub services: Vec<ServiceEntry>,

    /// Whether the agent is currently active.
    pub active: bool,

    /// On-chain registrations (if any).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registrations: Vec<Registration>,

    /// Supported trust models.
    #[serde(
        default,
        rename = "supportedTrust",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub supported_trust: Vec<String>,
}

/// A service endpoint in the registration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    /// Service name (e.g., "MCP", "A2A", "web", "ENS", "DID", "email").
    pub name: String,

    /// Endpoint URL.
    pub endpoint: String,

    /// Protocol version (optional but recommended).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// On-chain registration reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    /// ERC-721 token ID.
    #[serde(rename = "agentId")]
    pub agent_id: u64,

    /// Registry identifier: `{namespace}:{chainId}:{identityRegistry}`.
    #[serde(rename = "agentRegistry")]
    pub agent_registry: String,
}

/// Schema type constant for ERC-8004 v1 registration files.
pub const REGISTRATION_V1_TYPE: &str =
    "https://eips.ethereum.org/EIPS/eip-8004#registration-v1";

/// Build an agent card from the resolved identity configuration.
pub fn build_agent_card(config: &IdentityConfig) -> RegistrationFile {
    let services: Vec<ServiceEntry> = config
        .services
        .iter()
        .map(|s| ServiceEntry {
            name: s.name.clone(),
            endpoint: s.endpoint.clone(),
            version: s.version.clone(),
        })
        .collect();

    let registrations = match (config.erc8004_agent_id, &config.erc8004_network) {
        (Some(agent_id), Some(network)) => {
            vec![Registration {
                agent_id,
                agent_registry: network.clone(),
            }]
        }
        _ => Vec::new(),
    };

    RegistrationFile {
        schema_type: REGISTRATION_V1_TYPE.to_string(),
        name: config.agent_name.clone(),
        description: config
            .description
            .clone()
            .unwrap_or_else(|| format!("IronClaw agent: {}", config.agent_name)),
        image: config.image_url.clone(),
        services,
        active: true,
        registrations,
        supported_trust: vec!["reputation".to_string()],
    }
}

/// Serialize the agent card to a pretty-printed JSON string.
pub fn agent_card_json(config: &IdentityConfig) -> String {
    let card = build_agent_card(config);
    serde_json::to_string_pretty(&card).expect("agent card serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceEndpoint as ConfigServiceEndpoint;

    #[test]
    fn build_minimal_agent_card() {
        let config = IdentityConfig {
            agent_name: "Frack".to_string(),
            serve_agent_card: true,
            services: vec![ConfigServiceEndpoint {
                name: "web".to_string(),
                endpoint: "https://frack.example.com/".to_string(),
                version: None,
            }],
            description: Some("Personal AI assistant on MacBook".to_string()),
            image_url: None,
            erc8004_network: None,
            erc8004_agent_id: None,
            ethereum_key_source: crate::settings::KeySource::None,
        };

        let card = build_agent_card(&config);
        assert_eq!(card.name, "Frack");
        assert_eq!(card.schema_type, REGISTRATION_V1_TYPE);
        assert_eq!(card.services.len(), 1);
        assert!(card.active);
        assert!(card.registrations.is_empty());
    }

    #[test]
    fn build_registered_agent_card() {
        let config = IdentityConfig {
            agent_name: "Frick".to_string(),
            serve_agent_card: true,
            services: vec![],
            description: None,
            image_url: None,
            erc8004_network: Some("eip155:1:0x742d35Cc".to_string()),
            erc8004_agent_id: Some(42),
            ethereum_key_source: crate::settings::KeySource::None,
        };

        let card = build_agent_card(&config);
        assert_eq!(card.registrations.len(), 1);
        assert_eq!(card.registrations[0].agent_id, 42);
    }

    #[test]
    fn agent_card_json_is_valid() {
        let config = IdentityConfig {
            agent_name: "Test".to_string(),
            serve_agent_card: true,
            services: vec![],
            description: None,
            image_url: None,
            erc8004_network: None,
            erc8004_agent_id: None,
            ethereum_key_source: crate::settings::KeySource::None,
        };

        let json = agent_card_json(&config);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["name"], "Test");
        assert_eq!(parsed["active"], true);
    }
}
