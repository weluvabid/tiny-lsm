use std::{io, path::PathBuf};

/// Errors exposed by TinyLSM's public API.
///
/// Variants are introduced as their corresponding capabilities are
/// implemented; Phase 0 establishes the common error boundary.
#[derive(Debug)]
pub enum TinyLsmError {
    /// An I/O operation failed.
    Io {
        /// The operation being performed.
        operation: &'static str,
        /// Path involved, if the operation was path-specific.
        path: Option<PathBuf>,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// Persisted bytes did not conform to the supported file format.
    CorruptData {
        /// Path of the corrupt file, if known.
        path: Option<PathBuf>,
        /// Human-readable explanation.
        reason: String,
    },
    /// The requested operation conflicts with the store's current state.
    InvalidState(String),
    /// A numeric identifier or generation cannot be advanced without overflow.
    IdentifierExhausted(&'static str),
}

impl std::fmt::Display for TinyLsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => {
                write!(f, "I/O error while {operation}")?;
                if let Some(path) = path {
                    write!(f, " ({})", path.display())?;
                }
                write!(f, ": {source}")
            }
            Self::CorruptData { path, reason } => {
                write!(f, "corrupt persisted data")?;
                if let Some(path) = path {
                    write!(f, " ({})", path.display())?;
                }
                write!(f, ": {reason}")
            }
            Self::InvalidState(reason) => write!(f, "invalid engine state: {reason}"),
            Self::IdentifierExhausted(name) => {
                write!(f, "{name} identifier space exhausted")
            }
        }
    }
}

impl std::error::Error for TinyLsmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::CorruptData { .. } | Self::InvalidState(_) | Self::IdentifierExhausted(_) => None,
        }
    }
}
