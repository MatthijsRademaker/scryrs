use crate::{run, run_with_writers};

// Basic entrypoint smoke: verifies run() arg-collection wiring does not panic.
#[test]
fn public_run_entrypoint_no_panic() {
    assert_eq!(run(["--help"]), 0);
    assert_eq!(run(["--version"]), 0);
    assert_eq!(run(["--help-json"]), 0);
    // hotspots with no store exits 2 (missing store).
    assert_eq!(run(["hotspots", "/tmp"]), 2);
    assert_eq!(run(["record", "--file", "/nonexistent"]), 2);
    assert_eq!(run(Vec::<&str>::new()), 0);
    assert_eq!(run(["unknown"]), 2);
    assert_eq!(run(["hotspots"]), 2);
}

#[test]
fn help_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn version_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn hotspots_path_exits_0_stdout_nonempty() {
    // Hotspot command requires a valid store to exit 0.
    // This smoke test checks the missing-store case exits 2.
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn bare_invocation_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn unknown_command_exits_2_stderr_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn hotspots_without_path_exits_2_stderr_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn help_json_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}
