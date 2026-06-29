//! Dashboard server for scryrs local artifacts and live server data.

use std::net::IpAddr;
use std::path::PathBuf;

pub mod server;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveSourceConfig {
    pub server_url: String,
    pub repository_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceMode {
    Local,
    Live(LiveSourceConfig),
}

impl SourceMode {
    pub fn from_dashboard_args(
        server_url: Option<&str>,
        repository_id: Option<&str>,
    ) -> Result<Self, DashboardError> {
        match (server_url, repository_id) {
            (None, None) => Ok(Self::Local),
            (Some(_), None) | (None, Some(_)) => Err(DashboardError::InvalidConfig(
                "both --server-url and --repository-id are required for live mode".into(),
            )),
            (Some(server_url), Some(repository_id)) => Self::live(server_url, repository_id),
        }
    }

    pub fn live(server_url: &str, repository_id: &str) -> Result<Self, DashboardError> {
        let server_url = server_url.trim();
        let repository_id = repository_id.trim();
        if server_url.is_empty() {
            return Err(DashboardError::InvalidConfig(
                "--server-url must not be empty".into(),
            ));
        }
        if repository_id.is_empty() {
            return Err(DashboardError::InvalidConfig(
                "--repository-id must not be empty".into(),
            ));
        }
        Ok(Self::Live(LiveSourceConfig {
            server_url: server_url.to_string(),
            repository_id: repository_id.to_string(),
        }))
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Live(_) => "live",
        }
    }

    #[must_use]
    pub fn live_config(&self) -> Option<&LiveSourceConfig> {
        match self {
            Self::Local => None,
            Self::Live(config) => Some(config),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    pub bind_address: IpAddr,
    pub no_open: bool,
    pub dev_mode: bool,
    pub repo_root: PathBuf,
    pub source_mode: SourceMode,
}

impl Config {
    pub fn default_for_repo(repo_root: PathBuf) -> Self {
        Self {
            port: 8080,
            bind_address: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            no_open: false,
            dev_mode: false,
            repo_root,
            source_mode: SourceMode::Local,
        }
    }

    pub fn try_new(
        port: u16,
        bind_address: IpAddr,
        no_open: bool,
        dev_mode: bool,
        repo_root: PathBuf,
        source_mode: SourceMode,
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
            source_mode,
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
        assert_eq!(cfg.source_mode, SourceMode::Local);
    }

    #[test]
    fn try_new_rejects_zero_port() {
        let err = Config::try_new(
            0,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            true,
            false,
            PathBuf::from("/repo"),
            SourceMode::Local,
        )
        .err()
        .unwrap_or_else(|| panic!("zero port must fail"));

        assert!(err.to_string().contains("port must be between"));
    }

    #[test]
    fn source_mode_requires_both_live_flags() {
        let err = SourceMode::from_dashboard_args(Some("http://localhost:8081"), None)
            .err()
            .unwrap_or_else(|| panic!("partial live mode must fail"));

        assert!(
            err.to_string()
                .contains("both --server-url and --repository-id are required for live mode")
        );
    }

    #[test]
    fn source_mode_accepts_full_live_configuration() {
        let mode = SourceMode::from_dashboard_args(Some("http://localhost:8081"), Some("repo-a"))
            .unwrap_or_else(|err| panic!("full live mode: {err}"));

        assert_eq!(
            mode,
            SourceMode::Live(LiveSourceConfig {
                server_url: "http://localhost:8081".into(),
                repository_id: "repo-a".into(),
            })
        );
    }
}
