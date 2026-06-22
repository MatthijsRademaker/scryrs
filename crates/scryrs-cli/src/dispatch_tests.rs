use crate::run_with_writers;

#[test]
fn help_flag_prints_help_and_exits_0() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
}

#[test]
fn short_help_flag_produces_identical_output_to_long_help() {
    let mut out_long = Vec::new();
    let mut out_short = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out_long, &mut err), 0);
    assert!(err.is_empty());
    assert_eq!(run_with_writers(["-h"], &mut out_short, &mut err), 0);
    assert!(err.is_empty());
    assert_eq!(
        out_short, out_long,
        "-h must produce identical output to --help"
    );
}

#[test]
fn version_flag_prints_version_and_exits_0() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(String::from_utf8_lossy(&out).contains("scryrs "));
}

#[test]
fn short_version_flag_prints_version_and_exits_0() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["-V"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(String::from_utf8_lossy(&out).contains("scryrs "));
}

#[test]
fn bare_invocation_produces_identical_output_to_help() {
    let mut out_help = Vec::new();
    let mut out_bare = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out_help, &mut err), 0);
    assert!(err.is_empty());
    assert_eq!(
        run_with_writers(Vec::<&str>::new(), &mut out_bare, &mut err),
        0
    );
    assert!(err.is_empty());
    assert_eq!(
        out_bare, out_help,
        "bare invocation must produce identical output to --help"
    );
}

#[test]
fn hotspots_with_path_emits_json_and_exits_0() {
    // With no store at the path, exits 2 with error on stderr.
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("datastore not found"));
}

#[test]
fn hotspots_without_path_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("scryrs hotspots:"));
    assert!(err_str.contains("missing required PATH argument"));
    assert!(err_str.contains("Usage: scryrs hotspots <PATH>"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[test]
fn unknown_command_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("unknown command: 'unknown'"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[test]
fn components_command_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["components"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("unknown command: 'components'"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[test]
fn hotspots_with_extra_args_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["hotspots", "/tmp", "extra"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("unexpected argument after PATH"));
    assert!(err_str.contains("Usage: scryrs hotspots <PATH>"));
    assert!(err_str.contains("See `scryrs --help`"));
    assert!(!err_str.contains("unknown command"));
}

#[test]
fn record_with_help_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["record", "--help"], &mut out, &mut err),
        2,
        "record --help must exit 2"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
    assert!(
        err_str.contains("unexpected argument"),
        "must report unexpected argument, got: {err_str}"
    );
}

#[test]
fn record_with_version_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["record", "--version"], &mut out, &mut err),
        2,
        "record --version must exit 2"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
}

// --- --help-json surface tests (CLI Foundation 04) ---

#[test]
fn help_json_flag_outputs_valid_json_and_exits_0() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
}

#[test]
fn short_hj_flag_works_identically() {
    let mut out_long = Vec::new();
    let mut out_short = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["--help-json"], &mut out_long, &mut err),
        0
    );
    assert!(err.is_empty());
    assert_eq!(run_with_writers(["-hj"], &mut out_short, &mut err), 0);
    assert!(err.is_empty());
    assert_eq!(
        out_long, out_short,
        "--help-json and -hj must produce identical output"
    );
}

#[test]
fn help_json_does_not_interfere_with_existing_behavior() {
    // All existing commands and flags must still produce their expected output.
    // This test re-runs a representative subset to catch regressions.

    // --help still produces help text
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(String::from_utf8_lossy(&out).contains("COMMANDS"));

    // --version still produces version string
    out.clear();
    err.clear();
    assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
    assert!(String::from_utf8_lossy(&out).contains("scryrs "));

    // Missing store exits 2 with error on stderr (no longer placeholder).
    out.clear();
    err.clear();
    assert_eq!(
        run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("datastore not found"),
        "missing store should produce 'not found' error, got: {stderr}"
    );
    out.clear();
    err.clear();
    assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
    assert!(String::from_utf8_lossy(&err).contains("missing required PATH argument"));

    // Bare invocation still produces help
    out.clear();
    err.clear();
    assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
    assert!(String::from_utf8_lossy(&out).contains("COMMANDS"));

    // Unknown command still exits 2
    out.clear();
    err.clear();
    assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
    assert!(String::from_utf8_lossy(&err).contains("unknown command"));
}

#[test]
fn help_json_is_idempotent() {
    let mut first = Vec::new();
    let mut second = Vec::new();
    let mut err = Vec::new();

    run_with_writers(["--help-json"], &mut first, &mut err);
    assert!(err.is_empty());
    run_with_writers(["--help-json"], &mut second, &mut err);
    assert!(err.is_empty());
    assert_eq!(
        first, second,
        "--help-json must produce identical output on every invocation"
    );
}

#[test]
fn help_json_after_command_exits_2() {
    // --help-json after a command falls through to the command's argument
    // parser, which rejects flag-like positional arguments.
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["hotspots", "--help-json"], &mut out, &mut err),
        2,
        "--help-json after hotspots must exit 2 (no per-command introspection in v0)"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("unexpected argument after PATH"),
        "should report flag-like argument as invalid, got: {err_str}"
    );
}

#[test]
fn hotspots_short_hj_exits_2() {
    // -hj after a subcommand is not normalized (normalization only at root level)
    // and is rejected as an invalid argument.
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["hotspots", "-hj"], &mut out, &mut err),
        2,
        "-hj after hotspots must exit 2 (normalization only at root level)"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("unexpected argument after PATH"),
        "should report flag-like argument as invalid, got: {err_str}"
    );
}

#[test]
fn previously_stubbed_commands_exit_2() {
    for cmd in &[
        "trace",
        "propose",
        "graph",
        "route",
        "adapters",
        "report",
        "suggest-docs",
    ] {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers([*cmd], &mut out, &mut err),
            2,
            "command '{cmd}' should exit 2"
        );
        assert!(out.is_empty(), "command '{cmd}' should not produce stdout");
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("unknown command:"),
            "command '{cmd}' should produce unknown command error on stderr"
        );
        assert!(
            err_str.contains("See `scryrs --help`"),
            "command '{cmd}' should include escalation to --help on stderr"
        );
    }
}

// 3.5: Stale hotspot placeholder wording must be absent from --help output.
#[test]
fn help_output_does_not_contain_placeholder_wording() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(
        !help.contains("placeholder"),
        "--help output must not contain 'placeholder', got:\n{help}"
    );
}

// 3.5: Stale hotspot placeholder wording must be absent from --help-json output.
#[test]
fn help_json_output_does_not_contain_placeholder_wording() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let json_str = String::from_utf8_lossy(&out);
    assert!(
        !json_str.contains("placeholder"),
        "--help-json output must not contain 'placeholder', got:\n{json_str}"
    );
}

// --- Dashboard command tests ---

#[test]
fn dashboard_help_exits_0_and_lists_flags() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["dashboard", "--help"], &mut out, &mut err),
        0
    );
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(help.contains("start local dashboard server"));
    assert!(help.contains("--port <PORT>"));
    assert!(help.contains("--bind <ADDR>"));
    assert!(help.contains("--no-open"));
    assert!(help.contains("--dev"));
}

#[test]
fn dashboard_invalid_port_exits_2_without_starting_server() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["dashboard", "--port", "0"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("invalid --port value '0'"),
        "dashboard --port 0 must report validation error, got: {err_str}"
    );
}

#[test]
fn dashboard_with_unknown_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["dashboard", "--unknown"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs dashboard: unexpected argument"),
        "dashboard --unknown must report unexpected argument, got: {err_str}"
    );
    assert!(
        !err_str.contains("unknown command"),
        "must not say 'unknown command'"
    );
}
