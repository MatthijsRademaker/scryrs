use std::cell::RefCell;

std::thread_local! {
    static PATH: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set an override store path for the current thread (test-only).
/// Subsequent calls to `execute_record` on this thread will use this
/// path instead of `.scryrs/scryrs.db`.
#[allow(dead_code)]
pub(crate) fn set(path: String) {
    PATH.with(|p| *p.borrow_mut() = Some(path));
}

/// Get the override path, if set.
pub(crate) fn get() -> Option<String> {
    PATH.with(|p| p.borrow().clone())
}
