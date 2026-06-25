//! Central trace ingest server for `scryrs server`.
//!
//! Provides the HTTP runtime for `POST /v1/trace-events/batch`, server-owned
//! SQLite persistence, idempotent first-writer-wins semantics, and
//! deterministic per-item diagnostics.

use std::net::IpAddr;
use std::path::PathBuf;

pub mod server;
pub mod store;
pub(crate) mod time;

/// Default signal threshold — a `HotspotSignal` is emitted when a
/// cumulative hotspot score crosses this value from below.
pub const DEFAULT_SIGNAL_THRESHOLD: u32 = 10;

/// Configuration for the central ingest server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// TCP port to bind (default 8081).
    pub port: u16,
    /// Bind address (default 127.0.0.1).
    pub bind_address: IpAddr,
    /// Path to the server-owned SQLite database (default `.scryrs/server.db`).
    pub store_path: PathBuf,
    /// Cumulative hotspot score threshold that triggers a `HotspotSignal`.
    /// Defaults to `DEFAULT_SIGNAL_THRESHOLD` (10).
    pub signal_threshold: u32,
}

impl Config {
    /// Default configuration: localhost:8081, `.scryrs/server.db`,
    /// signal_threshold 10.
    #[must_use]
    pub fn default_local() -> Self {
        Self {
            port: 8081,
            bind_address: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            store_path: PathBuf::from(".scryrs/server.db"),
            signal_threshold: DEFAULT_SIGNAL_THRESHOLD,
        }
    }

    /// Construct a validated `Config` with the given signal threshold.
    ///
    /// # Errors
    ///
    /// Returns `ServerError::InvalidConfig` when `port` is zero or
    /// `store_path` is empty.
    pub fn try_new(
        port: u16,
        bind_address: IpAddr,
        store_path: PathBuf,
        signal_threshold: u32,
    ) -> Result<Self, ServerError> {
        if port == 0 {
            return Err(ServerError::InvalidConfig(
                "port must be between 1 and 65535".into(),
            ));
        }
        if store_path.as_os_str().is_empty() {
            return Err(ServerError::InvalidConfig(
                "store_path must not be empty".into(),
            ));
        }
        Ok(Self {
            port,
            bind_address,
            store_path,
            signal_threshold,
        })
    }
}

/// Top-level error type for the central ingest server.
#[derive(Debug)]
pub enum ServerError {
    InvalidConfig(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid server config: {msg}"),
            Self::Io(err) => write!(f, "server I/O error: {err}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidConfig(_) => None,
        }
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Start the central ingest server on the configured address and port.
///
/// Blocks until shutdown (SIGTERM/SIGINT).
pub fn run(config: Config) -> Result<(), ServerError> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(server::serve(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_localhost_8081_and_default_store() {
        let cfg = Config::default_local();

        assert_eq!(cfg.port, 8081);
        assert_eq!(cfg.bind_address.to_string(), "127.0.0.1");
        assert_eq!(cfg.store_path, PathBuf::from(".scryrs/server.db"));
        assert_eq!(cfg.signal_threshold, DEFAULT_SIGNAL_THRESHOLD);
    }

    #[test]
    fn try_new_rejects_zero_port() {
        let err = Config::try_new(
            0,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            PathBuf::from(".scryrs/server.db"),
            10,
        )
        .err()
        .unwrap_or_else(|| panic!("zero port must fail"));

        assert!(err.to_string().contains("port must be between"));
    }

    #[test]
    fn try_new_rejects_empty_store_path() {
        let err = Config::try_new(
            8081,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            PathBuf::from(""),
            10,
        )
        .err()
        .unwrap_or_else(|| panic!("empty store path must fail"));

        assert!(err.to_string().contains("store_path must not be empty"));
    }

    #[test]
    fn try_new_accepts_valid_config() {
        let cfg = Config::try_new(
            9091,
            IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            PathBuf::from("/tmp/live.db"),
            20,
        )
        .unwrap_or_else(|e| panic!("valid config must succeed: {e}"));

        assert_eq!(cfg.port, 9091);
        assert_eq!(cfg.bind_address.to_string(), "0.0.0.0");
        assert_eq!(cfg.store_path, PathBuf::from("/tmp/live.db"));
        assert_eq!(cfg.signal_threshold, 20);
    }

    #[test]
    fn signal_threshold_default_is_10() {
        assert_eq!(DEFAULT_SIGNAL_THRESHOLD, 10);
        let cfg = Config::default_local();
        assert_eq!(cfg.signal_threshold, 10);
    }
}
