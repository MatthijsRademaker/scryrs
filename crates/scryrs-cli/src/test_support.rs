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
