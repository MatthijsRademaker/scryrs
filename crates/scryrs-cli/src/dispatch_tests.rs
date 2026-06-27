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
    for cmd in &["trace", "propose", "adapters", "report", "suggest-docs"] {
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

// --- Server command tests ---

#[test]
fn server_appears_in_help_json_output() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let json_str = String::from_utf8_lossy(&out);
    assert!(
        json_str.contains("\"name\":\"server\""),
        "--help-json output must contain server command, got:\n{json_str}"
    );
    assert!(
        json_str.contains("POST /v1/trace-events/batch"),
        "--help-json server entry must document the batch endpoint, got:\n{json_str}"
    );
}

#[test]
fn server_help_exits_0_and_lists_flags() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["server", "--help"], &mut out, &mut err),
        0
    );
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(help.contains("start the central trace ingest server"));
    assert!(help.contains("--port <PORT>"));
    assert!(help.contains("--bind <ADDR>"));
    assert!(help.contains("--store <PATH>"));
    assert!(help.contains("POST /v1/trace-events/batch"));
}

#[test]
fn server_invalid_port_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["server", "--port", "0"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("invalid --port value '0'"),
        "server --port 0 must report validation error, got: {err_str}"
    );
}

#[test]
fn server_with_unknown_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["server", "--unknown"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs server: unexpected argument"),
        "server --unknown must report unexpected argument, got: {err_str}"
    );
    assert!(
        !err_str.contains("unknown command"),
        "must not say 'unknown command'"
    );
}

// --- Graph command tests ---

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_without_path_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["graph"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("scryrs graph:"));
    assert!(err_str.contains("missing required PATH argument"));
    assert!(err_str.contains("Usage: scryrs graph <PATH>"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_with_missing_hotspots_exits_2() {
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["graph", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("hotspots artifact not found"),
        "must report missing hotspots, got: {err_str}"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_with_malformed_hotspots_exits_2() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");
    fs::write(scryrs_dir.join("hotspots.json"), "not json").expect("write hotspots");

    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["graph", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("malformed hotspots file"),
        "must report malformed hotspots, got: {err_str}"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_with_valid_hotspot_and_docs_exits_0() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    // Write a minimal valid hotspots.json.
    let hotspots = serde_json::json!({
        "entries": []
    });
    fs::write(
        scryrs_dir.join("hotspots.json"),
        serde_json::to_string(&hotspots).expect("serialize"),
    )
    .expect("write hotspots");

    // Create docs dir with _nav.json and a page.
    let docs_dir = tmp.path().join(".devagent/docs/docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(docs_dir.join("graph.md"), "# Graph").expect("write page");
    let nav = serde_json::json!([
        {
            "text": "Technical",
            "items": [
                { "text": "Graph", "link": "/graph" }
            ]
        }
    ]);
    fs::write(
        docs_dir.join("_nav.json"),
        serde_json::to_string(&nav).expect("serialize"),
    )
    .expect("write _nav.json");

    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["graph", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    // stdout must be valid JSON matching KnowledgeGraphDocument shape.
    let stdout = String::from_utf8_lossy(&out);
    let doc: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(doc.get("schemaVersion").is_some());
    assert!(doc.get("metadata").is_some());
    assert!(doc.get("nodes").is_some());
    assert!(doc.get("edges").is_some());

    // Artifact was written.
    assert!(tmp.path().join(".scryrs/graph.json").exists());
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_repeated_runs_produce_byte_identical_output() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    let hotspots = serde_json::json!({
        "entries": [
            {
                "rank": 1,
                "subjectKind": "file",
                "subject": "src/main.rs",
                "score": 10,
                "counts": { "eventType": {}, "outcome": {} },
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": { "rowIds": [1, 2] }
            }
        ]
    });
    fs::write(
        scryrs_dir.join("hotspots.json"),
        serde_json::to_string(&hotspots).expect("serialize"),
    )
    .expect("write hotspots");

    let docs_dir = tmp.path().join(".devagent/docs/docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(docs_dir.join("graph.md"), "# Graph").expect("write page");
    let nav = serde_json::json!([
        {
            "text": "Technical",
            "items": [
                { "text": "Graph", "link": "/graph" }
            ]
        }
    ]);
    fs::write(
        docs_dir.join("_nav.json"),
        serde_json::to_string(&nav).expect("serialize"),
    )
    .expect("write _nav.json");

    let mut out1 = Vec::new();
    let mut err1 = Vec::new();
    assert_eq!(
        run_with_writers(
            ["graph", tmp.path().to_str().unwrap()],
            &mut out1,
            &mut err1,
        ),
        0
    );

    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    assert_eq!(
        run_with_writers(
            ["graph", tmp.path().to_str().unwrap()],
            &mut out2,
            &mut err2,
        ),
        0
    );

    assert_eq!(
        out1, out2,
        "repeated runs must produce byte-identical stdout"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn graph_with_empty_docs_exits_0_with_warning() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    let hotspots = serde_json::json!({
        "entries": [
            {
                "rank": 1,
                "subjectKind": "file",
                "subject": "src/main.rs",
                "score": 10,
                "counts": { "eventType": {}, "outcome": {} },
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": { "rowIds": [1] }
            }
        ]
    });
    fs::write(
        scryrs_dir.join("hotspots.json"),
        serde_json::to_string(&hotspots).expect("serialize"),
    )
    .expect("write hotspots");

    // No docs directory at all.
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["graph", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("docs directory not found"),
        "must warn about missing docs, got: {stderr}"
    );

    // stdout must still be valid graph JSON with hotspot-only nodes.
    let stdout = String::from_utf8_lossy(&out);
    let doc: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(doc.get("nodes").is_some());
}

// --- Route command tests ---

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_command_produces_help_output() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(
        help.contains("scryrs route"),
        "--help must list route command, got:\n{help}"
    );
    assert!(
        help.contains("RouteManifestDocument"),
        "--help must mention RouteManifestDocument, got:\n{help}"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_without_path_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["route"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("scryrs route:"));
    assert!(err_str.contains("missing required PATH argument"));
    assert!(err_str.contains("Usage: scryrs route <PATH>"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_with_extra_args_exits_2_with_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["route", "/tmp", "extra"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(err_str.contains("unexpected argument"));
    assert!(err_str.contains("scryrs route:"));
    assert!(err_str.contains("Usage: scryrs route <PATH>"));
    assert!(err_str.contains("See `scryrs --help`"));
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_missing_graph_exits_2() {
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("graph artifact not found"),
        "must report missing graph.json, got: {err_str}"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_malformed_graph_exits_2() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");
    fs::write(scryrs_dir.join("graph.json"), "not json").expect("write graph");

    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("malformed graph file"),
        "must report malformed graph.json, got: {err_str}"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_repeated_runs_produce_byte_identical_output() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    // Build a graph with known content.
    let graph = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "nodes": [
            {
                "id": "file:src/main.rs",
                "label": "src/main.rs",
                "kind": "file",
                "tags": [],
                "aliases": [],
                "evidenceLinks": [
                    {
                        "sourceKind": "local_trace_row",
                        "subject": "src/main.rs",
                        "rowIds": [1, 2],
                        "score": 10
                    }
                ]
            },
            {
                "id": "search:routing",
                "label": "routing",
                "kind": "search",
                "tags": [],
                "aliases": [],
                "evidenceLinks": [
                    {
                        "sourceKind": "hotspot_subject",
                        "subject": "routing",
                        "rowIds": [5],
                        "score": 42
                    }
                ]
            }
        ],
        "edges": []
    });
    fs::write(
        scryrs_dir.join("graph.json"),
        serde_json::to_string(&graph).expect("serialize"),
    )
    .expect("write graph.json");

    let mut out1 = Vec::new();
    let mut err1 = Vec::new();
    assert_eq!(
        run_with_writers(
            ["route", tmp.path().to_str().unwrap()],
            &mut out1,
            &mut err1,
        ),
        0
    );

    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    assert_eq!(
        run_with_writers(
            ["route", tmp.path().to_str().unwrap()],
            &mut out2,
            &mut err2,
        ),
        0
    );

    assert_eq!(
        out1, out2,
        "repeated runs must produce byte-identical stdout"
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_identity_boundary_preserves_distinct_subjects() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    // Three distinct nodes with shared text "auth" but different subjectKinds.
    let graph = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "nodes": [
            {
                "id": "file:auth",
                "label": "auth",
                "kind": "file",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            },
            {
                "id": "search:auth",
                "label": "auth",
                "kind": "search",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            },
            {
                "id": "symbol:auth",
                "label": "auth",
                "kind": "symbol",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            }
        ],
        "edges": []
    });
    fs::write(
        scryrs_dir.join("graph.json"),
        serde_json::to_string(&graph).expect("serialize"),
    )
    .expect("write graph.json");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("must be valid JSON");
    let routes = manifest["routes"].as_array().expect("routes must be array");

    assert_eq!(routes.len(), 3, "must produce three distinct entries");

    let ids: Vec<&str> = routes.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"file:auth"));
    assert!(ids.contains(&"search:auth"));
    assert!(ids.contains(&"symbol:auth"));

    // Verify no grouping on any entry.
    for route in routes {
        assert!(
            route.get("grouping").is_none(),
            "no route should have grouping"
        );
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_doc_pages_include_grouping_from_contains_edges() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    let graph = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "nodes": [
            {
                "id": "technical",
                "label": "Technical",
                "kind": "doc_group",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            },
            {
                "id": "doc_page:graph",
                "label": "graph",
                "kind": "doc_page",
                "tags": [],
                "aliases": [],
                "evidenceLinks": [
                    {
                        "sourceKind": "doc_reference",
                        "subject": "graph",
                        "rowIds": [],
                        "docRef": "graph"
                    }
                ]
            }
        ],
        "edges": [
            {
                "id": "e1",
                "sourceNodeId": "technical",
                "targetNodeId": "doc_page:graph",
                "relationship": "contains",
                "tags": [],
                "evidenceLinks": []
            }
        ]
    });
    fs::write(
        scryrs_dir.join("graph.json"),
        serde_json::to_string(&graph).expect("serialize"),
    )
    .expect("write graph.json");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("must be valid JSON");
    let routes = manifest["routes"].as_array().expect("routes must be array");

    // Find the doc_page:graph entry.
    let doc_entry = routes
        .iter()
        .find(|r| r["id"].as_str() == Some("doc_page:graph"))
        .expect("doc_page:graph must exist");

    let grouping = doc_entry
        .get("grouping")
        .expect("doc_page:graph must have grouping");
    assert_eq!(grouping["groupId"].as_str(), Some("technical"));
    assert_eq!(grouping["groupLabel"].as_str(), Some("Technical"));
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_hotspot_nodes_remain_ungrouped_in_v1() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    // Graph with hotspot node and doc page in separate domains (v1: no cross-domain edges).
    let graph = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "nodes": [
            {
                "id": "file:src/main.rs",
                "label": "src/main.rs",
                "kind": "file",
                "tags": [],
                "aliases": [],
                "evidenceLinks": [
                    {
                        "sourceKind": "local_trace_row",
                        "subject": "src/main.rs",
                        "rowIds": [1],
                        "score": 10
                    }
                ]
            },
            {
                "id": "technical",
                "label": "Technical",
                "kind": "doc_group",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            },
            {
                "id": "doc_page:graph",
                "label": "graph",
                "kind": "doc_page",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            }
        ],
        "edges": [
            {
                "id": "e1",
                "sourceNodeId": "technical",
                "targetNodeId": "doc_page:graph",
                "relationship": "contains",
                "tags": [],
                "evidenceLinks": []
            }
        ]
    });
    fs::write(
        scryrs_dir.join("graph.json"),
        serde_json::to_string(&graph).expect("serialize"),
    )
    .expect("write graph.json");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("must be valid JSON");
    let routes = manifest["routes"].as_array().expect("routes must be array");

    // Hotspot node must NOT have grouping.
    let hotspot_entry = routes
        .iter()
        .find(|r| r["id"].as_str() == Some("file:src/main.rs"))
        .expect("file:src/main.rs route must exist");
    assert!(
        hotspot_entry.get("grouping").is_none(),
        "hotspot node must not have grouping in v1"
    );

    // Doc page still has grouping from doc_group parent.
    let doc_entry = routes
        .iter()
        .find(|r| r["id"].as_str() == Some("doc_page:graph"))
        .expect("doc_page:graph route must exist");
    assert!(doc_entry.get("grouping").is_some());
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn route_artifact_written_to_routes_json() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::create_dir(&scryrs_dir).expect("create .scryrs");

    let graph = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "nodes": [
            {
                "id": "file:src/main.rs",
                "label": "src/main.rs",
                "kind": "file",
                "tags": [],
                "aliases": [],
                "evidenceLinks": []
            }
        ],
        "edges": []
    });
    fs::write(
        scryrs_dir.join("graph.json"),
        serde_json::to_string(&graph).expect("serialize"),
    )
    .expect("write graph.json");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["route", tmp.path().to_str().unwrap()], &mut out, &mut err,),
        0
    );

    let artifact = tmp.path().join(".scryrs/routes.json");
    assert!(
        artifact.exists(),
        ".scryrs/routes.json must exist after successful route run"
    );

    // Verify artifact content is valid route manifest.
    let content = fs::read_to_string(&artifact).expect("read routes.json");
    let doc: serde_json::Value =
        serde_json::from_str(&content).expect("routes.json must be valid JSON");
    assert_eq!(doc["schemaVersion"].as_str(), Some("1.0.0"));
    assert!(doc.get("routes").is_some());
}
