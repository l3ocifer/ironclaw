//! Error handling for MCP DevOps Tools

use std::fmt;

/// Result type alias for MCP operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for MCP operations
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    NotFound,
    Validation,
    Network,
    Internal,
    Authentication,
    RateLimit,
}

impl Error {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            message: message.into(),
            source: None,
        }
    }

    pub fn not_found_with_resource(message: &str, resource_type: &str, resource_id: &str) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            message: format!("{}: {} '{}'", message, resource_type, resource_id),
            source: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Validation,
            message: message.into(),
            source: None,
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Network,
            message: message.into(),
            source: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: message.into(),
            source: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}
