use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Global mutex to serialize CWD changes across tests.
/// `std::env::set_current_dir` is process-global; parallel test
/// threads would race on it without this guard.
pub static CWD_GUARD: Mutex<()> = Mutex::new(());

/// Change CWD to `dir`, run `f`, then restore original CWD.
pub fn with_cwd(dir: &std::path::Path, f: impl FnOnce()) {
    let _lock = CWD_GUARD
        .lock()
        .unwrap_or_else(|e| panic!("CWD guard poisoned: {e}"));
    let original = std::env::current_dir().unwrap_or_else(|e| panic!("current_dir: {e}"));
    std::env::set_current_dir(dir).unwrap_or_else(|e| panic!("set_current_dir: {e}"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(&original).unwrap_or_else(|e| panic!("restore cwd: {e}"));
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Returns relative file paths mapped to SHA-256 hex digests of file contents.
pub fn compute_file_inventory(root: &Path) -> HashMap<PathBuf, String> {
    let mut inventory = HashMap::new();
    walk_dir(root, root, &mut inventory);
    inventory
}

fn walk_dir(root: &Path, dir: &Path, inventory: &mut HashMap<PathBuf, String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(root, &path, inventory);
        } else if path.is_file() {
            let contents =
                std::fs::read(&path).unwrap_or_else(|e| panic!("read file for inventory: {e}"));
            let hash = format!("{:x}", Sha256::digest(&contents));
            let relative = path
                .strip_prefix(root)
                .unwrap_or_else(|e| panic!("relative path: {e}"));
            inventory.insert(relative.to_path_buf(), hash);
        }
    }
}

/// Recursively read a file or directory into a single byte buffer for
/// deterministic comparison. Directory snapshots concatenate relative
/// file paths and their contents in sorted order.
pub fn snapshot_dir_or_file(path: &Path) -> Vec<u8> {
    if path.is_file() {
        return std::fs::read(path).unwrap_or_else(|e| panic!("read protected file: {e}"));
    }
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
            .unwrap_or_else(|e| panic!("read protected dir: {e}"))
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();
        let mut buf = Vec::new();
        for entry in entries {
            let relative = entry
                .strip_prefix(path)
                .unwrap_or_else(|e| panic!("relative path: {e}"));
            buf.extend_from_slice(relative.to_string_lossy().as_bytes());
            buf.push(b'\n');
            let contents = snapshot_dir_or_file(&entry);
            buf.extend_from_slice(&contents);
        }
        return buf;
    }
    Vec::new()
}

/// Runs an action and asserts that all changes are confined to the allowed
/// write prefixes while protected paths remain byte-for-byte unchanged.
pub fn verify_writes_confined(
    root: &Path,
    protected_paths: &[&str],
    allowed_write_prefixes: &[&str],
    action: impl FnOnce() -> i32,
) {
    let mut protected_snapshots: HashMap<&str, Vec<u8>> = HashMap::new();
    for protected_path in protected_paths {
        protected_snapshots.insert(
            protected_path,
            snapshot_dir_or_file(&root.join(protected_path)),
        );
    }

    let inventory_before = compute_file_inventory(root);
    assert_eq!(action(), 0);
    let inventory_after = compute_file_inventory(root);

    for protected_path in protected_paths {
        let before = protected_snapshots
            .get(protected_path)
            .unwrap_or_else(|| panic!("missing protected snapshot for {protected_path}"));
        let after = snapshot_dir_or_file(&root.join(protected_path));
        assert_eq!(
            before, &after,
            "protected path must not be modified: {protected_path}"
        );
    }

    for (path, hash_after) in &inventory_after {
        let path_str = path.to_string_lossy();
        let allowed = allowed_write_prefixes
            .iter()
            .any(|prefix| path_str.starts_with(prefix));
        if allowed {
            continue;
        }
        match inventory_before.get(path) {
            Some(hash_before) => {
                assert_eq!(
                    hash_before, hash_after,
                    "file outside allowed prefixes must not be modified: {path_str}"
                );
            }
            None => panic!("new file created outside allowed prefixes: {path_str}"),
        }
    }

    for path in inventory_before.keys() {
        assert!(
            inventory_after.contains_key(path),
            "file must not be deleted: {}",
            path.display()
        );
    }
}
