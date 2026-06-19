//! Capability-scoped sandbox and tool policy foundation.

use std::path::{Path, PathBuf};

use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "sandbox",
        title: "scryrs-sandbox",
        summary: "capability-scoped tool and filesystem policy foundation",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolPolicy {
    pub allow_read_fs: Vec<PathBuf>,
    pub allow_write_fs: Vec<PathBuf>,
    pub allow_exec: Vec<String>,
    pub allow_net_hosts: Vec<String>,
    pub confirm_before_write: bool,
    pub confirm_before_exec: bool,
}

impl ToolPolicy {
    pub fn read_only(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            allow_read_fs: paths.into_iter().collect(),
            allow_write_fs: Vec::new(),
            allow_exec: Vec::new(),
            allow_net_hosts: Vec::new(),
            confirm_before_write: true,
            confirm_before_exec: true,
        }
    }

    pub fn can_write(&self, path: &Path) -> bool {
        self.allow_write_fs
            .iter()
            .any(|allowed| path.starts_with(allowed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_policy_denies_writes() {
        let policy = ToolPolicy::read_only([PathBuf::from("/repo")]);

        assert!(!policy.can_write(Path::new("/repo/file.rs")));
    }
}
