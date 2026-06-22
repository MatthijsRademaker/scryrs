//! Local dashboard server for scryrs trace artifacts.

use std::net::IpAddr;
use std::path::PathBuf;

pub mod server;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    pub bind_address: IpAddr,
    pub no_open: bool,
    pub dev_mode: bool,
    pub repo_root: PathBuf,
}

impl Config {
    pub fn default_for_repo(repo_root: PathBuf) -> Self {
        Self {
            port: 8080,
            bind_address: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            no_open: false,
            dev_mode: false,
            repo_root,
        }
    }

    pub fn try_new(
        port: u16,
        bind_address: IpAddr,
        no_open: bool,
        dev_mode: bool,
        repo_root: PathBuf,
    ) -> Result<Self, DashboardError> {
        if port == 0 {
            return Err(DashboardError::InvalidConfig(
                "port must be between 1 and 65535".into(),
            ));
        }
        if repo_root.as_os_str().is_empty() {
            return Err(DashboardError::InvalidConfig(
                "repo_root must not be empty".into(),
            ));
        }
        Ok(Self {
            port,
            bind_address,
            no_open,
            dev_mode,
            repo_root,
        })
    }

    #[must_use]
    pub fn frontend_dist_dir(&self) -> PathBuf {
        self.repo_root
            .join("crates")
            .join("scryrs-dashboard")
            .join("frontend")
            .join("dist")
    }
}

#[derive(Debug)]
pub enum DashboardError {
    InvalidConfig(String),
    Io(std::io::Error),
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid dashboard config: {msg}"),
            Self::Io(err) => write!(f, "dashboard I/O error: {err}"),
        }
    }
}

impl std::error::Error for DashboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidConfig(_) => None,
        }
    }
}

impl From<std::io::Error> for DashboardError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn run(config: Config) -> Result<(), DashboardError> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(server::serve(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_localhost_8080_and_browser_open() {
        let cfg = Config::default_for_repo(PathBuf::from("/repo"));

        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.bind_address.to_string(), "127.0.0.1");
        assert!(!cfg.no_open);
        assert!(!cfg.dev_mode);
        assert_eq!(cfg.repo_root, PathBuf::from("/repo"));
    }

    #[test]
    fn try_new_rejects_zero_port() {
        let err = Config::try_new(
            0,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            true,
            false,
            PathBuf::from("/repo"),
        )
        .err()
        .unwrap_or_else(|| panic!("zero port must fail"));

        assert!(err.to_string().contains("port must be between"));
    }
}
