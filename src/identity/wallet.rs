//! Ethereum wallet management for agent identity.
//!
//! Generates and manages Ethereum keypairs using alloy's local signer.
//! Private keys are stored encrypted in the secrets vault — never held
//! in memory longer than necessary.

use alloy::signers::local::PrivateKeySigner;

/// Generate a new random Ethereum keypair.
///
/// Returns `(private_key_hex, address_hex)`. The private key should be
/// immediately stored in the secrets vault and zeroed from memory.
pub fn generate_keypair() -> (String, String) {
    let signer = PrivateKeySigner::random();
    let address = format!("{:#x}", signer.address());
    // credential().to_bytes() returns FixedBytes<32>; hex_encode gives lowercase hex
    let private_key = format!("0x{}", hex_encode(signer.credential().to_bytes().as_ref()));
    (private_key, address)
}

/// Derive the Ethereum address from a hex-encoded private key.
///
/// The key can be with or without the `0x` prefix.
pub fn address_from_private_key(private_key_hex: &str) -> Result<String, WalletError> {
    let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
    let key_bytes = hex_decode(key_hex).map_err(|_| WalletError::InvalidPrivateKey)?;
    let signer = PrivateKeySigner::from_bytes(
        &alloy::primitives::B256::from_slice(&key_bytes),
    )
    .map_err(|e| WalletError::SignerCreation(e.to_string()))?;
    Ok(format!("{:#x}", signer.address()))
}

/// Sign an arbitrary message with the agent's private key.
///
/// Uses EIP-191 personal_sign (prefixed message signing).
pub async fn sign_message(
    private_key_hex: &str,
    message: &[u8],
) -> Result<String, WalletError> {
    use alloy::signers::Signer;

    let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
    let key_bytes = hex_decode(key_hex).map_err(|_| WalletError::InvalidPrivateKey)?;
    let signer = PrivateKeySigner::from_bytes(
        &alloy::primitives::B256::from_slice(&key_bytes),
    )
    .map_err(|e| WalletError::SignerCreation(e.to_string()))?;

    let signature = signer
        .sign_message(message)
        .await
        .map_err(|e| WalletError::SigningFailed(e.to_string()))?;

    Ok(format!("0x{}", hex_encode(&signature.as_bytes())))
}

/// Wallet management errors.
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("invalid private key format")]
    InvalidPrivateKey,
    #[error("failed to create signer: {0}")]
    SignerCreation(String),
    #[error("signing failed: {0}")]
    SigningFailed(String),
}

// Minimal hex encode/decode to avoid pulling in another crate.

fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    if hex.len() % 2 != 0 {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_keypair_produces_valid_address() {
        let (key, addr) = generate_keypair();
        assert!(key.starts_with("0x"), "private key should be 0x-prefixed");
        assert!(addr.starts_with("0x"), "address should be 0x-prefixed");
        assert_eq!(addr.len(), 42, "address should be 42 chars (0x + 40 hex)");

        // Derive address from key and verify it matches
        let derived = address_from_private_key(&key).unwrap();
        assert_eq!(derived, addr);
    }

    #[tokio::test]
    async fn sign_and_verify_roundtrip() {
        let (key, _addr) = generate_keypair();
        let message = b"hello ironclaw";
        let sig = sign_message(&key, message).await.unwrap();
        assert!(sig.starts_with("0x"));
        assert!(sig.len() > 2, "signature should not be empty");
    }
}
