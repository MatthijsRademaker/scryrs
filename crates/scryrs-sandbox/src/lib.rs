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
    fn read_only_constructor_sets_correct_defaults() {
        let policy = ToolPolicy::read_only([PathBuf::from("/repo")]);

        assert_eq!(policy.allow_read_fs, vec![PathBuf::from("/repo")]);
        assert!(
            policy.allow_write_fs.is_empty(),
            "read_only policy should have empty allow_write_fs"
        );
        assert!(
            policy.allow_exec.is_empty(),
            "read_only policy should have empty allow_exec"
        );
        assert!(
            policy.confirm_before_write,
            "read_only policy should set confirm_before_write to true"
        );
        assert!(
            policy.confirm_before_exec,
            "read_only policy should set confirm_before_exec to true"
        );
    }

    #[test]
    fn read_only_policy_denies_writes() {
        let policy = ToolPolicy::read_only([PathBuf::from("/repo")]);

        assert!(!policy.can_write(Path::new("/repo/file.rs")));
    }

    #[test]
    fn can_write_permits_paths_under_allowed_prefix() {
        let policy = ToolPolicy {
            allow_write_fs: vec![PathBuf::from("/tmp/project")],
            ..ToolPolicy::default()
        };

        assert!(policy.can_write(Path::new("/tmp/project/src/main.rs")));
        assert!(policy.can_write(Path::new("/tmp/project")));
        assert!(policy.can_write(Path::new("/tmp/project/deeply/nested/file.txt")));
    }

    #[test]
    fn can_write_rejects_paths_outside_allowed_prefixes() {
        let policy = ToolPolicy {
            allow_write_fs: vec![PathBuf::from("/tmp/project")],
            ..ToolPolicy::default()
        };

        assert!(!policy.can_write(Path::new("/etc/passwd")));
        assert!(!policy.can_write(Path::new("/tmp/other/file.rs")));
        assert!(!policy.can_write(Path::new("/home/user/file.txt")));
    }

    #[test]
    fn can_write_rejects_all_paths_when_allowlist_is_empty() {
        let policy = ToolPolicy::default();
        assert!(policy.allow_write_fs.is_empty());

        assert!(!policy.can_write(Path::new("/any/path")));
        assert!(!policy.can_write(Path::new("/tmp/test")));
        assert!(!policy.can_write(Path::new("/")));
    }
}
