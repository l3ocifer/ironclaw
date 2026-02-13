//! WASM tool checksum verification.
//!
//! Computes and verifies SHA-256 checksums of WASM binaries to detect
//! tampered or corrupted tools before execution.
//!
//! Inspired by [clawsec](https://github.com/prompt-security/clawsec) checksum patterns.
//!
//! # Design
//!
//! On install:
//!   1. Compute SHA-256 of the WASM binary.
//!   2. Store in `~/.ironclaw/tools/checksums.json`.
//!
//! On load (before execution):
//!   1. Recompute SHA-256 of the binary on disk.
//!   2. Compare against stored checksum.
//!   3. If mismatch: refuse to execute, log to audit, alert user.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Checksum entry for a WASM tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolChecksum {
    /// SHA-256 hex digest of the WASM binary.
    pub sha256: String,
    /// File size in bytes (for quick mismatch detection).
    pub size: u64,
    /// When the checksum was recorded.
    pub recorded_at: String,
    /// Optional source URL or path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Result of a checksum verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyResult {
    /// Checksum matches — tool is trusted.
    Valid,
    /// Checksum mismatch — tool may have been tampered with.
    Mismatch {
        expected: String,
        actual: String,
    },
    /// No checksum on file (tool was not registered).
    Unknown,
    /// File not found or unreadable.
    FileError(String),
}

impl VerifyResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    pub fn is_tampered(&self) -> bool {
        matches!(self, Self::Mismatch { .. })
    }
}

impl std::fmt::Display for VerifyResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "checksum valid"),
            Self::Mismatch { expected, actual } => {
                write!(
                    f,
                    "CHECKSUM MISMATCH: expected {}…, got {}…",
                    &expected[..12.min(expected.len())],
                    &actual[..12.min(actual.len())]
                )
            }
            Self::Unknown => write!(f, "no checksum recorded"),
            Self::FileError(e) => write!(f, "file error: {}", e),
        }
    }
}

/// WASM tool checksum store.
///
/// Manages SHA-256 checksums for all installed WASM tools.
pub struct ChecksumStore {
    /// Checksums keyed by tool name (not path — for portability).
    checksums: HashMap<String, ToolChecksum>,
    /// Path to the checksums.json file.
    store_path: PathBuf,
}

impl ChecksumStore {
    /// Create a new checksum store.
    ///
    /// `store_path` is typically `~/.ironclaw/tools/checksums.json`.
    pub fn new(store_path: PathBuf) -> Self {
        Self {
            checksums: HashMap::new(),
            store_path,
        }
    }

    /// Default store path.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ironclaw")
            .join("tools")
            .join("checksums.json")
    }

    /// Load checksums from disk.
    pub fn load(&mut self) -> Result<(), String> {
        if !self.store_path.exists() {
            return Ok(());
        }

        let data =
            std::fs::read_to_string(&self.store_path).map_err(|e| format!("Read error: {}", e))?;
        self.checksums =
            serde_json::from_str(&data).map_err(|e| format!("Parse error: {}", e))?;

        tracing::debug!(
            "Loaded {} WASM tool checksums",
            self.checksums.len()
        );
        Ok(())
    }

    /// Save checksums to disk.
    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.store_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {}", e))?;
        }

        let data = serde_json::to_string_pretty(&self.checksums)
            .map_err(|e| format!("Serialize error: {}", e))?;

        // Atomic write
        let tmp = self.store_path.with_extension("tmp");
        std::fs::write(&tmp, &data).map_err(|e| format!("Write error: {}", e))?;
        std::fs::rename(&tmp, &self.store_path).map_err(|e| format!("Rename error: {}", e))?;

        Ok(())
    }

    /// Register a WASM tool's checksum.
    ///
    /// Call this when installing or updating a tool.
    pub fn register(
        &mut self,
        tool_name: &str,
        wasm_path: &Path,
        source: Option<&str>,
    ) -> Result<ToolChecksum, String> {
        let (sha256, size) = hash_file(wasm_path)?;

        let checksum = ToolChecksum {
            sha256,
            size,
            recorded_at: chrono::Utc::now().to_rfc3339(),
            source: source.map(String::from),
        };

        self.checksums
            .insert(tool_name.to_string(), checksum.clone());
        self.save()?;

        tracing::info!(
            "Registered checksum for WASM tool '{}': {}…",
            tool_name,
            &checksum.sha256[..12]
        );

        Ok(checksum)
    }

    /// Verify a WASM tool's checksum before execution.
    ///
    /// Returns `Valid` if checksum matches, `Mismatch` if tampered,
    /// `Unknown` if no checksum recorded, `FileError` if file unreadable.
    pub fn verify(&self, tool_name: &str, wasm_path: &Path) -> VerifyResult {
        let expected = match self.checksums.get(tool_name) {
            Some(c) => c,
            None => return VerifyResult::Unknown,
        };

        // Quick check: file size first (fast rejection for different files)
        match std::fs::metadata(wasm_path) {
            Ok(meta) => {
                if meta.len() != expected.size {
                    let actual_hash = match hash_file(wasm_path) {
                        Ok((h, _)) => h,
                        Err(e) => return VerifyResult::FileError(e),
                    };
                    return VerifyResult::Mismatch {
                        expected: expected.sha256.clone(),
                        actual: actual_hash,
                    };
                }
            }
            Err(e) => return VerifyResult::FileError(format!("metadata: {}", e)),
        }

        // Full SHA-256 verification
        match hash_file(wasm_path) {
            Ok((actual_hash, _)) => {
                if actual_hash == expected.sha256 {
                    VerifyResult::Valid
                } else {
                    tracing::warn!(
                        target: "audit",
                        checksum = "mismatch",
                        tool = tool_name,
                        expected = &expected.sha256[..12],
                        actual = &actual_hash[..12],
                    );
                    VerifyResult::Mismatch {
                        expected: expected.sha256.clone(),
                        actual: actual_hash,
                    }
                }
            }
            Err(e) => VerifyResult::FileError(e),
        }
    }

    /// Remove a tool's checksum (e.g., on uninstall).
    pub fn remove(&mut self, tool_name: &str) -> bool {
        let removed = self.checksums.remove(tool_name).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// List all registered tool checksums.
    pub fn list(&self) -> &HashMap<String, ToolChecksum> {
        &self.checksums
    }
}

/// Compute SHA-256 hash of a file, reading in 8KB chunks.
///
/// Returns (hex_digest, file_size).
fn hash_file(path: &Path) -> Result<(String, u64), String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(|e| format!("open {}: {}", path.display(), e))?;
    let metadata = file.metadata().map_err(|e| format!("metadata: {}", e))?;

    // Reject symlinks
    if metadata.file_type().is_symlink() {
        return Err("refusing to hash symlink".to_string());
    }

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let result = hasher.finalize();
    let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    Ok((hex, metadata.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        tmp.flush().unwrap();

        let (hash, size) = hash_file(tmp.path()).unwrap();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(size, 11);
    }

    #[test]
    fn test_verify_valid() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"wasm binary content").unwrap();
        tmp.flush().unwrap();

        let store_path = std::env::temp_dir().join("ironclaw_test_checksums.json");
        let mut store = ChecksumStore::new(store_path.clone());

        store.register("test_tool", tmp.path(), None).unwrap();

        assert_eq!(store.verify("test_tool", tmp.path()), VerifyResult::Valid);

        // Cleanup
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn test_verify_mismatch() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"original content").unwrap();
        tmp.flush().unwrap();

        let store_path = std::env::temp_dir().join("ironclaw_test_checksums2.json");
        let mut store = ChecksumStore::new(store_path.clone());

        store.register("test_tool", tmp.path(), None).unwrap();

        // Tamper with the file
        std::fs::write(tmp.path(), b"tampered content").unwrap();

        assert!(store.verify("test_tool", tmp.path()).is_tampered());

        // Cleanup
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn test_verify_unknown() {
        let store = ChecksumStore::new(PathBuf::from("/tmp/nonexistent_checksums.json"));
        let result = store.verify("unknown_tool", Path::new("/tmp/whatever.wasm"));
        assert_eq!(result, VerifyResult::Unknown);
    }
}
