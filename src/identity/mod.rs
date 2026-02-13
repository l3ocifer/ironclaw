//! Agent identity management (ERC-8004).
//!
//! Provides Ethereum keypair generation, agent card construction, and
//! local identity persistence. On-chain registration is deferred to a
//! future phase — this module handles the local-first identity that
//! can later be published to the ERC-8004 Identity Registry.
//!
//! # Architecture
//!
//! Each agent has:
//! - An Ethereum keypair (stored encrypted in the secrets vault)
//! - An agent card (ERC-8004 `RegistrationFile` served as JSON)
//! - An optional on-chain registration (NFT token ID)
//!
//! The agent card is served at `/.well-known/agent-card.json` from
//! the gateway, making the agent discoverable via A2A and MCP.

pub mod agent_card;
pub mod wallet;

use serde::{Deserialize, Serialize};

/// Core agent identity, combining wallet address with agent card metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Agent display name (e.g., "Frack", "Frick").
    pub name: String,

    /// Ethereum wallet address (hex, 0x-prefixed).
    pub wallet_address: String,

    /// Secret ID referencing the encrypted keypair in the secrets store.
    /// The private key is never held in memory outside of signing operations.
    pub keypair_secret_id: String,

    /// ERC-8004 network where the agent is registered (if any).
    pub erc8004_network: Option<String>,

    /// ERC-8004 agent ID (NFT token ID) after on-chain registration.
    pub erc8004_agent_id: Option<u64>,
}

impl AgentIdentity {
    /// Create a new identity from a wallet and configuration.
    pub fn new(
        name: String,
        wallet_address: String,
        keypair_secret_id: String,
    ) -> Self {
        Self {
            name,
            wallet_address,
            keypair_secret_id,
            erc8004_network: None,
            erc8004_agent_id: None,
        }
    }

    /// Whether this identity has been registered on-chain.
    pub fn is_registered(&self) -> bool {
        self.erc8004_agent_id.is_some()
    }
}
